//! `java.io.DataInput`, `DataOutput`, `DataInputStream` and
//! `DataOutputStream`: Java's primitives in big-endian bytes, and strings
//! in modified UTF-8 behind a two-byte length, as level files and saved
//! games hold them.
//!
//! Each method does what the JDK's does, one byte read or written at a
//! time where the JDK does that, so a game's own stream underneath sees
//! the same calls (see [`super::access`] for the fast path).

use super::access::{read_byte, read_into, write_all, write_array, write_byte, write_each};
use crate::class::ArrayType;
use crate::ctx::{Ctx, JResult, Throw};
use crate::lang::{check_range, this, throw};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

const DIS: &str = "java/io/DataInputStream";
const DOS: &str = "java/io/DataOutputStream";

/// The longest string `writeUTF` writes: its length is a `u16`.
const MAX_UTF_BYTES: usize = 65535;

fn eof(ctx: &mut Ctx<'_>) -> Throw {
    ctx.new_exception("java/io/EOFException", None)
}

/// The stream a `DataInputStream` wraps: its protected `in` field.
fn input(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<ObjRef> {
    let f = ctx.field_id(DIS, "in", "Ljava/io/InputStream;")?;
    let v = ctx.get_field(obj, f);
    ctx.non_null(v)
}

/// The stream a `DataOutputStream` wraps: its protected `out` field.
fn output(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<ObjRef> {
    let f = ctx.field_id(DOS, "out", "Ljava/io/OutputStream;")?;
    let v = ctx.get_field(obj, f);
    ctx.non_null(v)
}

/// `N` bytes from `N` calls of `in.read()`, as `readShort` and `readInt`
/// take them; `EOFException` when any was past the end.
fn read_bytes<const N: usize>(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<[u8; N]> {
    let mut bytes = [0; N];
    let mut ended = false;
    for b in &mut bytes {
        let read = read_byte(ctx, stream)?;
        ended |= read < 0;
        *b = read as u8;
    }
    if ended {
        return Err(eof(ctx));
    }
    Ok(bytes)
}

/// `readFully`: `in.read(arr, off + n, len - n)` until `len` bytes are
/// in, `EOFException` if the stream ends first.
fn read_fully(
    ctx: &mut Ctx<'_>,
    stream: ObjRef,
    arr: ObjRef,
    off: usize,
    len: usize,
) -> JResult<()> {
    let mut n = 0;
    while n < len {
        let count = read_into(ctx, stream, arr, off + n, len - n)?;
        if count < 0 {
            return Err(eof(ctx));
        }
        n += count as usize;
    }
    Ok(())
}

/// `readFully` into a new array of `len` bytes, returned as Rust bytes.
fn read_fully_vec(ctx: &mut Ctx<'_>, stream: ObjRef, len: usize) -> JResult<Vec<u8>> {
    let arr = ctx.new_array(ArrayType::Byte, len as i32)?;
    read_fully(ctx, stream, arr, 0, len)?;
    Ok(ctx.array::<i8>(arr)?.iter().map(|&b| b as u8).collect())
}

fn read_unsigned_short(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<u16> {
    Ok(u16::from_be_bytes(read_bytes::<2>(ctx, stream)?))
}

fn read_int(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<i32> {
    Ok(i32::from_be_bytes(read_bytes::<4>(ctx, stream)?))
}

fn read_long(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<i64> {
    let bytes = read_fully_vec(ctx, stream, 8)?;
    let mut be = [0; 8];
    be.copy_from_slice(&bytes);
    Ok(i64::from_be_bytes(be))
}

/// Decodes modified UTF-8 as `DataInputStream.readUTF` does, with its
/// `UTFDataFormatException` messages.
fn decode_modified_utf8(bytes: &[u8]) -> Result<Vec<u16>, String> {
    let mut chars = Vec::with_capacity(bytes.len());
    let mut count = 0;
    while count < bytes.len() {
        let c = bytes[count];
        match c >> 4 {
            0..=7 => {
                chars.push(u16::from(c));
                count += 1;
            }
            12 | 13 => {
                count += 2;
                if count > bytes.len() {
                    return Err("malformed input: partial character at end".to_owned());
                }
                let c2 = bytes[count - 1];
                if c2 & 0xC0 != 0x80 {
                    return Err(format!("malformed input around byte {count}"));
                }
                chars.push((u16::from(c & 0x1F) << 6) | u16::from(c2 & 0x3F));
            }
            14 => {
                count += 3;
                if count > bytes.len() {
                    return Err("malformed input: partial character at end".to_owned());
                }
                let (c2, c3) = (bytes[count - 2], bytes[count - 1]);
                if c2 & 0xC0 != 0x80 || c3 & 0xC0 != 0x80 {
                    return Err(format!("malformed input around byte {}", count - 1));
                }
                chars.push(
                    (u16::from(c & 0x0F) << 12)
                        | (u16::from(c2 & 0x3F) << 6)
                        | u16::from(c3 & 0x3F),
                );
            }
            _ => return Err(format!("malformed input around byte {count}")),
        }
    }
    Ok(chars)
}

/// Encodes modified UTF-8 behind its two-byte length, as `writeUTF`
/// writes it: U+0000 takes two bytes, each surrogate three.
fn encode_modified_utf8(chars: &[u16]) -> Result<Vec<u8>, String> {
    let len: usize = chars
        .iter()
        .map(|&c| match c {
            0x0001..=0x007F => 1,
            0x0000 | 0x0080..=0x07FF => 2,
            _ => 3,
        })
        .sum();
    if len > MAX_UTF_BYTES {
        return Err(format!("encoded string too long: {len} bytes"));
    }
    let mut out = Vec::with_capacity(len + 2);
    out.extend_from_slice(&(len as u16).to_be_bytes());
    for &c in chars {
        match c {
            0x0001..=0x007F => out.push(c as u8),
            0x0000 | 0x0080..=0x07FF => {
                out.push(0xC0 | (c >> 6) as u8);
                out.push(0x80 | (c & 0x3F) as u8);
            }
            _ => {
                out.push(0xE0 | (c >> 12) as u8);
                out.push(0x80 | ((c >> 6) & 0x3F) as u8);
                out.push(0x80 | (c & 0x3F) as u8);
            }
        }
    }
    Ok(out)
}

fn utf_result(ctx: &mut Ctx<'_>, bytes: &[u8]) -> JResult<Value> {
    match decode_modified_utf8(bytes) {
        Ok(chars) => Ok(ctx.new_string_utf16(chars)?.into()),
        Err(message) => throw(ctx, "java/io/UTFDataFormatException", &message),
    }
}

/// `DataInputStream.readUTF(DataInput)`: through the final methods of a
/// `DataInputStream`, through the interface for any other `DataInput`.
fn read_utf(ctx: &mut Ctx<'_>, source: ObjRef) -> JResult<Value> {
    let dis = ctx.find_class(DIS)?;
    if ctx.is_instance(source, dis) {
        let stream = input(ctx, source)?;
        let len = read_unsigned_short(ctx, stream)?;
        let bytes = read_fully_vec(ctx, stream, usize::from(len))?;
        return utf_result(ctx, &bytes);
    }
    let len = ctx
        .call_interface(source, "readUnsignedShort", "()I", &[])?
        .int();
    let arr = ctx.new_array(ArrayType::Byte, len)?;
    let args = [arr.into(), Value::Int(0), Value::Int(len)];
    ctx.call_interface(source, "readFully", "([BII)V", &args)?;
    let bytes: Vec<u8> = ctx.array::<i8>(arr)?.iter().map(|&b| b as u8).collect();
    utf_result(ctx, &bytes)
}

/// The bits `Float.floatToIntBits` gives, every NaN as one.
fn float_bits(v: f32) -> i32 {
    if v.is_nan() {
        0x7FC0_0000
    } else {
        v.to_bits() as i32
    }
}

/// The bits `Double.doubleToLongBits` gives, every NaN as one.
fn double_bits(v: f64) -> i64 {
    if v.is_nan() {
        0x7FF8_0000_0000_0000
    } else {
        v.to_bits() as i64
    }
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        data_input(),
        data_output(),
        data_input_stream(),
        data_output_stream(),
    ]
}

fn data_input() -> NativeClass {
    [
        ("readFully", "([B)V"),
        ("readFully", "([BII)V"),
        ("skipBytes", "(I)I"),
        ("readBoolean", "()Z"),
        ("readByte", "()B"),
        ("readUnsignedByte", "()I"),
        ("readShort", "()S"),
        ("readUnsignedShort", "()I"),
        ("readChar", "()C"),
        ("readInt", "()I"),
        ("readLong", "()J"),
        ("readFloat", "()F"),
        ("readDouble", "()D"),
        ("readUTF", "()Ljava/lang/String;"),
    ]
    .into_iter()
    .fold(NativeClass::interface("java/io/DataInput"), |c, (n, d)| {
        c.abstract_method(n, d)
    })
}

fn data_output() -> NativeClass {
    [
        ("write", "(I)V"),
        ("write", "([B)V"),
        ("write", "([BII)V"),
        ("writeBoolean", "(Z)V"),
        ("writeByte", "(I)V"),
        ("writeShort", "(I)V"),
        ("writeChar", "(I)V"),
        ("writeInt", "(I)V"),
        ("writeLong", "(J)V"),
        ("writeFloat", "(F)V"),
        ("writeDouble", "(D)V"),
        ("writeChars", "(Ljava/lang/String;)V"),
        ("writeUTF", "(Ljava/lang/String;)V"),
    ]
    .into_iter()
    .fold(NativeClass::interface("java/io/DataOutput"), |c, (n, d)| {
        c.abstract_method(n, d)
    })
}

/// A `DataInputStream` method that passes its call on to the wrapped
/// stream unchanged.
fn delegate_in(
    name: &'static str,
    desc: &'static str,
) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> {
    move |ctx, a| {
        let stream = input(ctx, this(a))?;
        ctx.call_virtual(stream, name, desc, &a[1..])
    }
}

fn data_input_stream() -> NativeClass {
    use crate::classfile::access::{FINAL, PUBLIC, STATIC};
    let fin = PUBLIC | FINAL;
    NativeClass::new(DIS)
        .extends("java/io/InputStream")
        .implements("java/io/DataInput")
        .field_with(
            "in",
            "Ljava/io/InputStream;",
            crate::classfile::access::PROTECTED,
        )
        .method("<init>", "(Ljava/io/InputStream;)V", |ctx, a| {
            let f = ctx.field_id(DIS, "in", "Ljava/io/InputStream;")?;
            ctx.set_field(this(a), f, a[1]);
            Ok(Value::Void)
        })
        .method("read", "()I", |ctx, a| {
            let stream = input(ctx, this(a))?;
            Ok(Value::Int(read_byte(ctx, stream)?))
        })
        .method_with("read", "([B)I", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            let arr = ctx.non_null(a[1])?;
            let len = ctx.array_len(arr)?;
            Ok(Value::Int(read_into(ctx, stream, arr, 0, len)?))
        })
        .method_with("read", "([BII)I", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            let arr = ctx.non_null(a[1])?;
            let size = ctx.array_len(arr)?;
            let exception = "java/lang/IndexOutOfBoundsException";
            let (off, len) = check_range(ctx, exception, a[2].int(), a[3].int(), size)?;
            Ok(Value::Int(read_into(ctx, stream, arr, off, len)?))
        })
        .method_with("readFully", "([B)V", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            let arr = ctx.non_null(a[1])?;
            let len = ctx.array_len(arr)?;
            read_fully(ctx, stream, arr, 0, len)?;
            Ok(Value::Void)
        })
        .method_with("readFully", "([BII)V", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            let arr = ctx.non_null(a[1])?;
            let (off, len) = (a[2].int(), a[3].int());
            if len == 0 {
                return Ok(Value::Void);
            }
            let size = ctx.array_len(arr)?;
            let exception = "java/lang/IndexOutOfBoundsException";
            let (off, len) = check_range(ctx, exception, off, len, size)?;
            read_fully(ctx, stream, arr, off, len)?;
            Ok(Value::Void)
        })
        .method_with("skipBytes", "(I)I", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            let n = a[1].int();
            let mut total = 0;
            while total < n {
                let left = Value::Long(i64::from(n - total));
                let skipped = ctx.call_virtual(stream, "skip", "(J)J", &[left])?.long();
                if skipped <= 0 {
                    break;
                }
                total += skipped as i32;
            }
            Ok(Value::Int(total))
        })
        .method_with("readBoolean", "()Z", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            let [b] = read_bytes::<1>(ctx, stream)?;
            Ok(Value::from(b != 0))
        })
        .method_with("readByte", "()B", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            let [b] = read_bytes::<1>(ctx, stream)?;
            Ok(Value::Int(i32::from(b as i8)))
        })
        .method_with("readUnsignedByte", "()I", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            let [b] = read_bytes::<1>(ctx, stream)?;
            Ok(Value::Int(i32::from(b)))
        })
        .method_with("readShort", "()S", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            let v = read_unsigned_short(ctx, stream)?;
            Ok(Value::Int(i32::from(v as i16)))
        })
        .method_with("readUnsignedShort", "()I", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            Ok(Value::Int(i32::from(read_unsigned_short(ctx, stream)?)))
        })
        .method_with("readChar", "()C", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            Ok(Value::Int(i32::from(read_unsigned_short(ctx, stream)?)))
        })
        .method_with("readInt", "()I", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            Ok(Value::Int(read_int(ctx, stream)?))
        })
        .method_with("readLong", "()J", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            Ok(Value::Long(read_long(ctx, stream)?))
        })
        .method_with("readFloat", "()F", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            let bits = read_int(ctx, stream)?;
            Ok(Value::Float(f32::from_bits(bits as u32)))
        })
        .method_with("readDouble", "()D", fin, |ctx, a| {
            let stream = input(ctx, this(a))?;
            let bits = read_long(ctx, stream)?;
            Ok(Value::Double(f64::from_bits(bits as u64)))
        })
        .method_with("readUTF", "()Ljava/lang/String;", fin, |ctx, a| {
            read_utf(ctx, this(a))
        })
        .method_with(
            "readUTF",
            "(Ljava/io/DataInput;)Ljava/lang/String;",
            PUBLIC | STATIC | FINAL,
            |ctx, a| {
                let source = ctx.non_null(a[0])?;
                read_utf(ctx, source)
            },
        )
        .method("skip", "(J)J", delegate_in("skip", "(J)J"))
        .method("available", "()I", delegate_in("available", "()I"))
        .method("close", "()V", delegate_in("close", "()V"))
        .method("mark", "(I)V", delegate_in("mark", "(I)V"))
        .method("reset", "()V", delegate_in("reset", "()V"))
        .method("markSupported", "()Z", delegate_in("markSupported", "()Z"))
}

/// Writes `bytes` to the stream under the `DataOutputStream` `obj` with
/// one `write(int)` call each, as the JDK's `writeShort` and `writeInt` do.
fn put_each(ctx: &mut Ctx<'_>, obj: ObjRef, bytes: &[u8]) -> JResult<Value> {
    let stream = output(ctx, obj)?;
    write_each(ctx, stream, bytes)?;
    Ok(Value::Void)
}

/// Writes `bytes` to the stream under `obj` with one `write(byte[], int,
/// int)` call, as the JDK's `writeLong` does.
fn put_all(ctx: &mut Ctx<'_>, obj: ObjRef, bytes: &[u8]) -> JResult<Value> {
    let stream = output(ctx, obj)?;
    write_all(ctx, stream, bytes)?;
    Ok(Value::Void)
}

fn data_output_stream() -> NativeClass {
    use crate::classfile::access::{FINAL, PROTECTED, PUBLIC};
    let fin = PUBLIC | FINAL;
    NativeClass::new(DOS)
        .extends("java/io/OutputStream")
        .implements("java/io/DataOutput")
        .field_with("out", "Ljava/io/OutputStream;", PROTECTED)
        .method("<init>", "(Ljava/io/OutputStream;)V", |ctx, a| {
            let f = ctx.field_id(DOS, "out", "Ljava/io/OutputStream;")?;
            ctx.set_field(this(a), f, a[1]);
            Ok(Value::Void)
        })
        .method("write", "(I)V", |ctx, a| {
            let stream = output(ctx, this(a))?;
            write_byte(ctx, stream, a[1].int())?;
            Ok(Value::Void)
        })
        .method("write", "([BII)V", |ctx, a| {
            let stream = output(ctx, this(a))?;
            let arr = ctx.non_null(a[1])?;
            let size = ctx.array_len(arr)?;
            let exception = "java/lang/IndexOutOfBoundsException";
            let (off, len) = check_range(ctx, exception, a[2].int(), a[3].int(), size)?;
            write_array(ctx, stream, arr, off, len)?;
            Ok(Value::Void)
        })
        .method("flush", "()V", |ctx, a| {
            let stream = output(ctx, this(a))?;
            ctx.call_virtual(stream, "flush", "()V", &[])?;
            Ok(Value::Void)
        })
        .method("close", "()V", |ctx, a| {
            // As Java's: flush, then close the stream even if flush threw.
            let flushed = ctx.call_virtual(this(a), "flush", "()V", &[]);
            let stream = output(ctx, this(a))?;
            let closed = ctx.call_virtual(stream, "close", "()V", &[]);
            flushed?;
            closed?;
            Ok(Value::Void)
        })
        .method_with("writeBoolean", "(Z)V", fin, |ctx, a| {
            put_each(ctx, this(a), &[u8::from(a[1].boolean())])
        })
        .method_with("writeByte", "(I)V", fin, |ctx, a| {
            put_each(ctx, this(a), &[a[1].int() as u8])
        })
        .method_with("writeShort", "(I)V", fin, |ctx, a| {
            put_each(ctx, this(a), &(a[1].int() as u16).to_be_bytes())
        })
        .method_with("writeChar", "(I)V", fin, |ctx, a| {
            put_each(ctx, this(a), &(a[1].int() as u16).to_be_bytes())
        })
        .method_with("writeInt", "(I)V", fin, |ctx, a| {
            put_each(ctx, this(a), &a[1].int().to_be_bytes())
        })
        .method_with("writeLong", "(J)V", fin, |ctx, a| {
            put_all(ctx, this(a), &a[1].long().to_be_bytes())
        })
        .method_with("writeFloat", "(F)V", fin, |ctx, a| {
            put_each(ctx, this(a), &float_bits(a[1].float()).to_be_bytes())
        })
        .method_with("writeDouble", "(D)V", fin, |ctx, a| {
            put_all(ctx, this(a), &double_bits(a[1].double()).to_be_bytes())
        })
        .method_with("writeChars", "(Ljava/lang/String;)V", fin, |ctx, a| {
            let s = ctx.non_null(a[1])?;
            let bytes: Vec<u8> = ctx
                .string_utf16(s)?
                .iter()
                .flat_map(|c| c.to_be_bytes())
                .collect();
            put_each(ctx, this(a), &bytes)
        })
        .method_with("writeUTF", "(Ljava/lang/String;)V", fin, |ctx, a| {
            let s = ctx.non_null(a[1])?;
            let bytes = match encode_modified_utf8(ctx.string_utf16(s)?) {
                Ok(bytes) => bytes,
                Err(message) => {
                    return throw(ctx, "java/io/UTFDataFormatException", &message);
                }
            };
            // Java writes the encoded string through this stream's own
            // write(byte[], int, int), which a subclass may override.
            let obj = this(a);
            if ctx.class_name(ctx.class_of(obj)) == DOS {
                return put_all(ctx, obj, &bytes);
            }
            let arr = ctx.new_byte_array(&bytes)?;
            let args = [arr.into(), Value::Int(0), Value::Int(bytes.len() as i32)];
            ctx.call_virtual(obj, "write", "([BII)V", &args)?;
            Ok(Value::Void)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn modified_utf8_round_trips() {
        let text = utf16("a\u{0}\u{e9}\u{20ac}\u{1F600}");
        let bytes = encode_modified_utf8(&text).unwrap();
        assert_eq!(
            bytes,
            [
                0, 14, b'a', 0xC0, 0x80, 0xC3, 0xA9, 0xE2, 0x82, 0xAC, 0xED, 0xA0, 0xBD, 0xED,
                0xB8, 0x80
            ]
        );
        assert_eq!(decode_modified_utf8(&bytes[2..]).unwrap(), text);
    }

    #[test]
    fn bad_modified_utf8_names_the_byte_as_java_does() {
        assert_eq!(
            decode_modified_utf8(&[b'a', 0xC3]).unwrap_err(),
            "malformed input: partial character at end"
        );
        assert_eq!(
            decode_modified_utf8(&[b'a', 0xE2, 0x41, 0x80]).unwrap_err(),
            "malformed input around byte 3"
        );
        assert_eq!(
            decode_modified_utf8(&[b'a', 0xC3, 0x41]).unwrap_err(),
            "malformed input around byte 3"
        );
        assert_eq!(
            decode_modified_utf8(&[0x80]).unwrap_err(),
            "malformed input around byte 0"
        );
        assert_eq!(
            decode_modified_utf8(&[b'x', 0xF0, 0x9F, 0x98, 0x80]).unwrap_err(),
            "malformed input around byte 1"
        );
    }

    #[test]
    fn strings_past_65535_bytes_are_refused() {
        let long = vec![0x20ACu16; 21846];
        assert_eq!(
            encode_modified_utf8(&long).unwrap_err(),
            "encoded string too long: 65538 bytes"
        );
        assert!(encode_modified_utf8(&vec![b'a'.into(); 65535]).is_ok());
    }

    #[test]
    fn nan_bits_are_canonical() {
        assert_eq!(float_bits(f32::from_bits(0x7FC1_2345)), 0x7FC0_0000);
        assert_eq!(float_bits(-0.0), i32::MIN);
        assert_eq!(
            double_bits(f64::from_bits(0xFFF8_0000_0000_0001)),
            0x7FF8_0000_0000_0000
        );
    }
}
