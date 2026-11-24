//! `java.io.Reader`, `Writer`, `InputStreamReader` and
//! `OutputStreamWriter`: characters over bytes, in a named charset or the
//! game's own (`microedition.encoding`).
//!
//! As on CLDC phones, the reader takes bytes from its stream only as it
//! needs them for the characters asked for, and the writer passes each
//! write on at once, so a game that mixes the reader with the stream
//! under it, or reads a `ByteArrayOutputStream` without flushing the
//! writer, sees what it saw on a phone.

use std::collections::VecDeque;

use super::access::{read_byte, write_all};
use crate::charset::{Charset, Decoder, Encoder};
use crate::class::ArrayType;
use crate::classfile::access::PROTECTED;
use crate::ctx::{Ctx, JResult};
use crate::heap::{Payload, Tracer};
use crate::lang::string::{default_charset, named_charset};
use crate::lang::{check_range, this, throw};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

/// The most `Reader.skip` reads at once, as in Java.
const SKIP_BUFFER: i64 = 8192;

/// What an `InputStreamReader` keeps.
struct ReaderState {
    /// The stream it reads; `None` once closed.
    input: Option<ObjRef>,
    /// Holds the start of a character until its other bytes come.
    decoder: Decoder,
    /// Characters decoded but not yet returned: the second half of a
    /// surrogate pair, or what one bad sequence turned into.
    decoded: VecDeque<u16>,
    /// The stream ended while decoding; cleared when the reader reports
    /// the end, so a stream that grows can be read again.
    ended: bool,
}

impl Default for ReaderState {
    fn default() -> Self {
        Self {
            input: None,
            decoder: Charset::Utf8.decoder(),
            decoded: VecDeque::new(),
            ended: false,
        }
    }
}

impl Payload for ReaderState {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.input);
    }
}

/// What an `OutputStreamWriter` keeps.
struct WriterState {
    /// The stream it writes; `None` once closed.
    output: Option<ObjRef>,
    /// Keeps UTF-16's mark and ISO-2022-JP's shift state between writes.
    encoder: Encoder,
    /// A high surrogate that ended the last write, waiting for its pair.
    high: Option<u16>,
}

impl Default for WriterState {
    fn default() -> Self {
        Self {
            output: None,
            encoder: Charset::Utf8.encoder(),
            high: None,
        }
    }
}

impl Payload for WriterState {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.output);
    }
}

fn closed<T>(ctx: &mut Ctx<'_>) -> JResult<T> {
    throw(ctx, "java/io/IOException", "Stream closed")
}

/// Sets the protected `lock` field of a `Reader` or `Writer`.
fn set_lock(ctx: &mut Ctx<'_>, class: &str, obj: ObjRef, lock: ObjRef) -> JResult<()> {
    let f = ctx.field_id(class, "lock", "Ljava/lang/Object;")?;
    ctx.set_field(obj, f, lock.into());
    Ok(())
}

/// The charset argument of a constructor: the default for `None`,
/// `UnsupportedEncodingException` for a name this VM lacks.
fn charset_arg(ctx: &mut Ctx<'_>, name: Option<Value>) -> JResult<Charset> {
    match name {
        None => Ok(default_charset(ctx)),
        Some(v) => named_charset(ctx, v),
    }
}

fn reader_init(ctx: &mut Ctx<'_>, a: &[Value], name: Option<Value>) -> JResult<Value> {
    let input = ctx.non_null(a[1])?;
    let charset = charset_arg(ctx, name)?;
    set_lock(ctx, "java/io/Reader", this(a), input)?;
    let state = ctx.payload_mut::<ReaderState>(this(a))?;
    state.input = Some(input);
    state.decoder = charset.decoder();
    Ok(Value::Void)
}

/// The next UTF-16 unit of the reader `obj`, `None` at the end of its
/// stream. Reads bytes one at a time until they make a character.
fn next_unit(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<Option<u16>> {
    loop {
        let state = ctx.payload_mut::<ReaderState>(obj)?;
        if let Some(c) = state.decoded.pop_front() {
            return Ok(Some(c));
        }
        let mut out = Vec::new();
        if state.ended {
            // A character the end cut off becomes U+FFFD first.
            state.decoder.decode(&[], true, &mut out);
            if out.is_empty() {
                state.ended = false;
                return Ok(None);
            }
            state.decoded.extend(out);
            continue;
        }
        let Some(input) = state.input else {
            return closed(ctx);
        };
        let b = read_byte(ctx, input)?;
        let state = ctx.payload_mut::<ReaderState>(obj)?;
        match u8::try_from(b) {
            Ok(byte) => {
                state.decoder.decode(&[byte], false, &mut out);
                state.decoded.extend(out);
            }
            Err(_) => state.ended = true,
        }
    }
}

/// The stream of the reader `obj`, or `IOException` once it is closed.
fn reader_input(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<ObjRef> {
    match ctx.payload::<ReaderState>(obj)?.input {
        Some(input) => Ok(input),
        None => closed(ctx),
    }
}

fn writer_init(ctx: &mut Ctx<'_>, a: &[Value], name: Option<Value>) -> JResult<Value> {
    let output = ctx.non_null(a[1])?;
    let charset = charset_arg(ctx, name)?;
    set_lock(ctx, "java/io/Writer", this(a), output)?;
    let state = ctx.payload_mut::<WriterState>(this(a))?;
    state.output = Some(output);
    state.encoder = charset.encoder();
    Ok(Value::Void)
}

/// The stream of the writer `obj`, or `IOException` once it is closed:
/// Java checks that before the arguments.
fn writer_output(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<ObjRef> {
    match ctx.payload::<WriterState>(obj)?.output {
        Some(output) => Ok(output),
        None => closed(ctx),
    }
}

/// Encodes `units` after any high surrogate the last write left, keeps a
/// high surrogate that ends them for the next write, and writes the
/// bytes to the stream of the writer `obj` at once.
fn write_units(ctx: &mut Ctx<'_>, obj: ObjRef, units: &[u16]) -> JResult<()> {
    let output = writer_output(ctx, obj)?;
    let state = ctx.payload_mut::<WriterState>(obj)?;
    let mut chars: Vec<u16> = state.high.take().into_iter().collect();
    chars.extend_from_slice(units);
    if chars.last().is_some_and(|&c| (0xD800..0xDC00).contains(&c)) {
        state.high = chars.pop();
    }
    let mut bytes = Vec::with_capacity(chars.len());
    state.encoder.encode(&chars, &mut bytes);
    if !bytes.is_empty() {
        write_all(ctx, output, &bytes)?;
    }
    Ok(())
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        reader(),
        writer(),
        input_stream_reader(),
        output_stream_writer(),
    ]
}

fn reader() -> NativeClass {
    const READER: &str = "java/io/Reader";
    NativeClass::new(READER)
        .abstract_class()
        .field_with("lock", "Ljava/lang/Object;", PROTECTED)
        .method_with("<init>", "()V", PROTECTED, |ctx, a| {
            set_lock(ctx, READER, this(a), this(a))?;
            Ok(Value::Void)
        })
        .method_with("<init>", "(Ljava/lang/Object;)V", PROTECTED, |ctx, a| {
            let lock = ctx.non_null(a[1])?;
            set_lock(ctx, READER, this(a), lock)?;
            Ok(Value::Void)
        })
        .method("read", "()I", |ctx, a| {
            let one = ctx.new_array(ArrayType::Char, 1)?;
            let args = [one.into(), Value::Int(0), Value::Int(1)];
            let n = ctx.call_virtual(this(a), "read", "([CII)I", &args)?.int();
            if n == -1 {
                return Ok(Value::Int(-1));
            }
            Ok(Value::Int(i32::from(ctx.array::<u16>(one)?[0])))
        })
        .method("read", "([C)I", |ctx, a| {
            let buf = ctx.non_null(a[1])?;
            let len = ctx.array_len(buf)? as i32;
            let args = [buf.into(), Value::Int(0), Value::Int(len)];
            ctx.call_virtual(this(a), "read", "([CII)I", &args)
        })
        .abstract_method("read", "([CII)I")
        .method("skip", "(J)J", |ctx, a| {
            let n = a[1].long();
            if n < 0 {
                return throw(
                    ctx,
                    "java/lang/IllegalArgumentException",
                    "skip value is negative",
                );
            }
            let size = n.min(SKIP_BUFFER) as i32;
            let scratch = ctx.new_array(ArrayType::Char, size)?;
            let mut remaining = n;
            while remaining > 0 {
                let chunk = Value::Int(remaining.min(i64::from(size)) as i32);
                let args = [scratch.into(), Value::Int(0), chunk];
                let read = ctx.call_virtual(this(a), "read", "([CII)I", &args)?.int();
                if read == -1 {
                    break;
                }
                remaining -= i64::from(read);
            }
            Ok(Value::Long(n - remaining))
        })
        .method("ready", "()Z", |_, _| Ok(Value::from(false)))
        .method("markSupported", "()Z", |_, _| Ok(Value::from(false)))
        .method("mark", "(I)V", |ctx, _| {
            throw(ctx, "java/io/IOException", "mark() not supported")
        })
        .method("reset", "()V", |ctx, _| {
            throw(ctx, "java/io/IOException", "reset() not supported")
        })
        .abstract_method("close", "()V")
}

fn writer() -> NativeClass {
    const WRITER: &str = "java/io/Writer";
    NativeClass::new(WRITER)
        .abstract_class()
        .field_with("lock", "Ljava/lang/Object;", PROTECTED)
        .method_with("<init>", "()V", PROTECTED, |ctx, a| {
            set_lock(ctx, WRITER, this(a), this(a))?;
            Ok(Value::Void)
        })
        .method_with("<init>", "(Ljava/lang/Object;)V", PROTECTED, |ctx, a| {
            let lock = ctx.non_null(a[1])?;
            set_lock(ctx, WRITER, this(a), lock)?;
            Ok(Value::Void)
        })
        .method("write", "(I)V", |ctx, a| {
            let one = ctx.new_array(ArrayType::Char, 1)?;
            ctx.array_mut::<u16>(one)?[0] = a[1].int() as u16;
            let args = [one.into(), Value::Int(0), Value::Int(1)];
            ctx.call_virtual(this(a), "write", "([CII)V", &args)?;
            Ok(Value::Void)
        })
        .method("write", "([C)V", |ctx, a| {
            let buf = ctx.non_null(a[1])?;
            let len = ctx.array_len(buf)? as i32;
            let args = [buf.into(), Value::Int(0), Value::Int(len)];
            ctx.call_virtual(this(a), "write", "([CII)V", &args)?;
            Ok(Value::Void)
        })
        .abstract_method("write", "([CII)V")
        .method("write", "(Ljava/lang/String;)V", |ctx, a| {
            let s = ctx.non_null(a[1])?;
            let len = ctx.string_utf16(s)?.len() as i32;
            let args = [s.into(), Value::Int(0), Value::Int(len)];
            ctx.call_virtual(this(a), "write", "(Ljava/lang/String;II)V", &args)?;
            Ok(Value::Void)
        })
        .method("write", "(Ljava/lang/String;II)V", |ctx, a| {
            let chars = string_range(ctx, a)?;
            let buf = ctx.new_array(ArrayType::Char, chars.len() as i32)?;
            ctx.array_mut::<u16>(buf)?.copy_from_slice(&chars);
            let args = [buf.into(), Value::Int(0), Value::Int(chars.len() as i32)];
            ctx.call_virtual(this(a), "write", "([CII)V", &args)?;
            Ok(Value::Void)
        })
        .abstract_method("flush", "()V")
        .abstract_method("close", "()V")
}

/// The characters `write(String, int, int)` writes, with Java's checks:
/// a negative count throws `IndexOutOfBoundsException`, a range outside
/// the string `StringIndexOutOfBoundsException`.
fn string_range(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Vec<u16>> {
    let s = ctx.non_null(a[1])?;
    let (off, len) = (a[2].int(), a[3].int());
    if len < 0 {
        return throw(ctx, "java/lang/IndexOutOfBoundsException", &len.to_string());
    }
    let size = ctx.string_utf16(s)?.len();
    let exception = "java/lang/StringIndexOutOfBoundsException";
    let (off, len) = check_range(ctx, exception, off, len, size)?;
    Ok(ctx.string_utf16(s)?[off..off + len].to_vec())
}

fn input_stream_reader() -> NativeClass {
    NativeClass::new("java/io/InputStreamReader")
        .extends("java/io/Reader")
        .payload::<ReaderState>()
        .method("<init>", "(Ljava/io/InputStream;)V", |ctx, a| {
            reader_init(ctx, a, None)
        })
        .method(
            "<init>",
            "(Ljava/io/InputStream;Ljava/lang/String;)V",
            |ctx, a| reader_init(ctx, a, Some(a[2])),
        )
        .method("read", "()I", |ctx, a| {
            Ok(Value::Int(next_unit(ctx, this(a))?.map_or(-1, i32::from)))
        })
        .method("read", "([CII)I", |ctx, a| {
            let obj = this(a);
            reader_input(ctx, obj)?;
            let buf = ctx.non_null(a[1])?;
            let size = ctx.array_len(buf)?;
            let exception = "java/lang/IndexOutOfBoundsException";
            let (off, len) = check_range(ctx, exception, a[2].int(), a[3].int(), size)?;
            let mut n = 0;
            while n < len {
                let Some(c) = next_unit(ctx, obj)? else {
                    break;
                };
                ctx.array_mut::<u16>(buf)?[off + n] = c;
                n += 1;
            }
            Ok(Value::Int(if n == 0 && len > 0 { -1 } else { n as i32 }))
        })
        .method("ready", "()Z", |ctx, a| {
            let input = reader_input(ctx, this(a))?;
            let state = ctx.payload::<ReaderState>(this(a))?;
            if !state.decoded.is_empty() || state.decoder.has_pending() {
                return Ok(Value::from(true));
            }
            let available = ctx.call_virtual(input, "available", "()I", &[])?.int();
            Ok(Value::from(available > 0))
        })
        .method("markSupported", "()Z", |ctx, a| {
            let input = reader_input(ctx, this(a))?;
            ctx.call_virtual(input, "markSupported", "()Z", &[])
        })
        .method("mark", "(I)V", |ctx, a| {
            let input = reader_input(ctx, this(a))?;
            if !ctx
                .call_virtual(input, "markSupported", "()Z", &[])?
                .boolean()
            {
                return throw(ctx, "java/io/IOException", "mark() not supported");
            }
            ctx.call_virtual(input, "mark", "(I)V", &[a[1]])?;
            Ok(Value::Void)
        })
        .method("reset", "()V", |ctx, a| {
            let input = reader_input(ctx, this(a))?;
            ctx.call_virtual(input, "reset", "()V", &[])?;
            let state = ctx.payload_mut::<ReaderState>(this(a))?;
            state.decoder.reset();
            state.decoded.clear();
            state.ended = false;
            Ok(Value::Void)
        })
        .method("close", "()V", |ctx, a| {
            let Some(input) = ctx.payload_mut::<ReaderState>(this(a))?.input.take() else {
                return Ok(Value::Void);
            };
            ctx.call_virtual(input, "close", "()V", &[])?;
            Ok(Value::Void)
        })
}

fn output_stream_writer() -> NativeClass {
    NativeClass::new("java/io/OutputStreamWriter")
        .extends("java/io/Writer")
        .payload::<WriterState>()
        .method("<init>", "(Ljava/io/OutputStream;)V", |ctx, a| {
            writer_init(ctx, a, None)
        })
        .method(
            "<init>",
            "(Ljava/io/OutputStream;Ljava/lang/String;)V",
            |ctx, a| writer_init(ctx, a, Some(a[2])),
        )
        .method("write", "(I)V", |ctx, a| {
            write_units(ctx, this(a), &[a[1].int() as u16])?;
            Ok(Value::Void)
        })
        .method("write", "([CII)V", |ctx, a| {
            writer_output(ctx, this(a))?;
            let buf = ctx.non_null(a[1])?;
            let size = ctx.array_len(buf)?;
            let exception = "java/lang/IndexOutOfBoundsException";
            let (off, len) = check_range(ctx, exception, a[2].int(), a[3].int(), size)?;
            let chars = ctx.array::<u16>(buf)?[off..off + len].to_vec();
            write_units(ctx, this(a), &chars)?;
            Ok(Value::Void)
        })
        .method("write", "(Ljava/lang/String;II)V", |ctx, a| {
            writer_output(ctx, this(a))?;
            let chars = string_range(ctx, a)?;
            write_units(ctx, this(a), &chars)?;
            Ok(Value::Void)
        })
        .method("flush", "()V", |ctx, a| {
            let output = writer_output(ctx, this(a))?;
            ctx.call_virtual(output, "flush", "()V", &[])?;
            Ok(Value::Void)
        })
        .method("close", "()V", |ctx, a| {
            let obj = this(a);
            let state = ctx.payload_mut::<WriterState>(obj)?;
            let Some(output) = state.output else {
                return Ok(Value::Void);
            };
            // A high surrogate still waiting for its pair is written as
            // the charset's replacement, and a charset with shift states
            // returns to its first one, as Java's encoder ends.
            let mut bytes = Vec::new();
            if let Some(high) = state.high.take() {
                state.encoder.encode(&[high], &mut bytes);
            }
            state.encoder.finish(&mut bytes);
            let ended = if bytes.is_empty() {
                Ok(())
            } else {
                write_all(ctx, output, &bytes)
            };
            let flushed = ended.and_then(|()| ctx.call_virtual(output, "flush", "()V", &[]));
            ctx.payload_mut::<WriterState>(obj)?.output = None;
            let closed = ctx.call_virtual(output, "close", "()V", &[]);
            flushed?;
            closed?;
            Ok(Value::Void)
        })
}
