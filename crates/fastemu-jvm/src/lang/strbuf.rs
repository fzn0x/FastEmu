//! `java.lang.StringBuffer` and `java.lang.StringBuilder`, one Rust
//! implementation for both. `javac` turns string concatenation into
//! `StringBuilder` calls (`StringBuffer` before Java 5).

use super::float_fmt::{double_to_string, float_to_string};
use super::string::JString;
use super::{check_range, this, throw};
use crate::ctx::{Ctx, JResult, Throw};
use crate::heap::Payload;
use crate::native::NativeClass;
use crate::value::Value;

/// The contents of a string buffer.
pub(crate) struct StrBuf {
    pub(crate) chars: Vec<u16>,
    /// The capacity Java would report: 16 to start, growing to twice
    /// plus two (or the length needed, if more) when the text outgrows it.
    capacity: usize,
}

impl StrBuf {
    fn ensure(&mut self, needed: usize) {
        if needed > self.capacity {
            self.capacity = (self.capacity * 2 + 2).max(needed);
        }
    }
}

impl Default for StrBuf {
    fn default() -> Self {
        Self {
            chars: Vec::with_capacity(16),
            capacity: 16,
        }
    }
}

impl Payload for StrBuf {
    fn heap_bytes(&self) -> usize {
        24 + 2 * self.capacity
    }
}

/// The text of a buffer, for reading and editing; the capacity follows
/// the length when the guard is dropped.
struct BufMut<'a>(&'a mut StrBuf);

impl std::ops::Deref for BufMut<'_> {
    type Target = Vec<u16>;

    fn deref(&self) -> &Vec<u16> {
        &self.0.chars
    }
}

impl std::ops::DerefMut for BufMut<'_> {
    fn deref_mut(&mut self) -> &mut Vec<u16> {
        &mut self.0.chars
    }
}

impl Drop for BufMut<'_> {
    fn drop(&mut self) {
        let len = self.0.chars.len();
        self.0.ensure(len);
    }
}

fn buf<'a>(ctx: &'a mut Ctx<'_>, a: &[Value]) -> JResult<BufMut<'a>> {
    Ok(BufMut(ctx.payload_mut::<StrBuf>(this(a))?))
}

fn state<'a>(ctx: &'a mut Ctx<'_>, a: &[Value]) -> JResult<&'a mut StrBuf> {
    ctx.payload_mut::<StrBuf>(this(a))
}

fn len(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<usize> {
    Ok(buf(ctx, a)?.len())
}

/// Throws `OutOfMemoryError` when a buffer of `chars` characters could not
/// fit in the heap, before the host allocates it: the heap only measures
/// a buffer at its next collection.
fn check_fits(ctx: &mut Ctx<'_>, chars: usize) -> JResult<()> {
    if chars.saturating_mul(2) > ctx.heap_usage().0 {
        return Err(ctx.out_of_memory());
    }
    Ok(())
}

fn index_error(ctx: &mut Ctx<'_>, message: String) -> Throw {
    ctx.new_exception("java/lang/StringIndexOutOfBoundsException", Some(&message))
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// The text `append(Object)` and `insert(int, Object)` add: `"null"` or
/// the object's `toString()`.
fn object_text(ctx: &mut Ctx<'_>, v: Value) -> JResult<Vec<u16>> {
    let Some(obj) = v.obj() else {
        return Ok(utf16("null"));
    };
    if let Some(s) = ctx.try_payload::<JString>(obj) {
        return Ok(s.chars.to_vec());
    }
    if let Some(b) = ctx.try_payload::<StrBuf>(obj) {
        return Ok(b.chars.clone());
    }
    let s = ctx.call_virtual(obj, "toString", "()Ljava/lang/String;", &[])?;
    match s.obj() {
        Some(s) => Ok(ctx.string_utf16(s)?.to_vec()),
        None => Ok(utf16("null")),
    }
}

/// The text of the value argument of an `append` or `insert`.
fn text_of(ctx: &mut Ctx<'_>, desc: &str, v: Value) -> JResult<Vec<u16>> {
    Ok(match desc {
        "Z" => utf16(if v.boolean() { "true" } else { "false" }),
        "C" => vec![v.char()],
        "I" => utf16(&v.int().to_string()),
        "J" => utf16(&v.long().to_string()),
        "F" => utf16(&float_to_string(v.float())),
        "D" => utf16(&double_to_string(v.double())),
        "[C" => {
            let arr = ctx.non_null(v)?;
            ctx.array::<u16>(arr)?.to_vec()
        }
        _ => object_text(ctx, v)?,
    })
}

const VALUE_DESCS: [&str; 10] = [
    "Ljava/lang/String;",
    "Ljava/lang/Object;",
    "Ljava/lang/StringBuffer;",
    "Ljava/lang/CharSequence;",
    "Z",
    "C",
    "I",
    "J",
    "F",
    "D",
];

fn reverse(chars: &mut [u16]) {
    chars.reverse();
    // Surrogate pairs were reversed too; put each back in order.
    let mut i = 0;
    while i + 1 < chars.len() {
        if (0xDC00..0xE000).contains(&chars[i]) && (0xD800..0xDC00).contains(&chars[i + 1]) {
            chars.swap(i, i + 1);
            i += 2;
        } else {
            i += 1;
        }
    }
}

fn class(name: &str) -> NativeClass {
    let me = format!("L{name};");
    let mut class = NativeClass::new(name)
        .final_class()
        .implements("java/lang/CharSequence")
        .payload::<StrBuf>()
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .method("<init>", "(I)V", |ctx, a| {
            let capacity = a[1].int();
            if capacity < 0 {
                return throw(
                    ctx,
                    "java/lang/NegativeArraySizeException",
                    &capacity.to_string(),
                );
            }
            check_fits(ctx, capacity as usize)?;
            state(ctx, a)?.capacity = capacity as usize;
            Ok(Value::Void)
        })
        .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
            let s = ctx.non_null(a[1])?;
            let chars = ctx.string_utf16(s)?.to_vec();
            let sb = state(ctx, a)?;
            sb.capacity = chars.len() + 16;
            sb.chars.extend_from_slice(&chars);
            Ok(Value::Void)
        })
        .method("<init>", "(Ljava/lang/CharSequence;)V", |ctx, a| {
            ctx.non_null(a[1])?;
            let chars = object_text(ctx, a[1])?;
            let sb = state(ctx, a)?;
            sb.capacity = chars.len() + 16;
            sb.chars.extend_from_slice(&chars);
            Ok(Value::Void)
        })
        .method("length", "()I", |ctx, a| {
            Ok(Value::Int(len(ctx, a)? as i32))
        })
        .method("capacity", "()I", |ctx, a| {
            Ok(Value::Int(state(ctx, a)?.capacity as i32))
        })
        .method("ensureCapacity", "(I)V", |ctx, a| {
            let want = a[1].int().max(0) as usize;
            check_fits(ctx, want)?;
            state(ctx, a)?.ensure(want);
            Ok(Value::Void)
        })
        .method("toString", "()Ljava/lang/String;", |ctx, a| {
            let chars = buf(ctx, a)?.clone();
            Ok(ctx.new_string_utf16(chars)?.into())
        })
        .method("charAt", "(I)C", |ctx, a| {
            let i = a[1].int();
            let c = state(ctx, a)?.chars.get(i as usize).copied();
            match c {
                Some(c) => Ok(Value::Int(i32::from(c))),
                None => {
                    let n = len(ctx, a)?;
                    Err(index_error(ctx, format!("index {i},length {n}")))
                }
            }
        })
        .method("setCharAt", "(IC)V", |ctx, a| {
            let i = a[1].int();
            let c = a[2].char();
            let n = len(ctx, a)?;
            if i < 0 || i as usize >= n {
                return Err(index_error(ctx, format!("index {i},length {n}")));
            }
            buf(ctx, a)?[i as usize] = c;
            Ok(Value::Void)
        })
        .method("setLength", "(I)V", |ctx, a| {
            let n = a[1].int();
            if n < 0 {
                return Err(index_error(ctx, format!("String index out of range: {n}")));
            }
            check_fits(ctx, n as usize)?;
            buf(ctx, a)?.resize(n as usize, 0);
            Ok(Value::Void)
        })
        .method("getChars", "(II[CI)V", |ctx, a| {
            let (begin, end) = (a[1].int(), a[2].int());
            let n = len(ctx, a)?;
            if begin < 0 || end > n as i32 || begin > end {
                return Err(index_error(
                    ctx,
                    format!("start {begin}, end {end}, length {n}"),
                ));
            }
            let part = buf(ctx, a)?[begin as usize..end as usize].to_vec();
            let dst = ctx.non_null(a[3])?;
            let dst_len = ctx.array_len(dst)?;
            let (at, k) = check_range(
                ctx,
                "java/lang/ArrayIndexOutOfBoundsException",
                a[4].int(),
                part.len() as i32,
                dst_len,
            )?;
            ctx.array_mut::<u16>(dst)?[at..at + k].copy_from_slice(&part);
            Ok(Value::Void)
        })
        .method("indexOf", "(Ljava/lang/String;)I", |ctx, a| {
            let s = ctx.non_null(a[1])?;
            let target = ctx.string_utf16(s)?.to_vec();
            Ok(Value::Int(super::string::index_of(
                &buf(ctx, a)?,
                &target,
                0,
            )))
        })
        .method("indexOf", "(Ljava/lang/String;I)I", |ctx, a| {
            let s = ctx.non_null(a[1])?;
            let target = ctx.string_utf16(s)?.to_vec();
            Ok(Value::Int(super::string::index_of(
                &buf(ctx, a)?,
                &target,
                a[2].int(),
            )))
        })
        .method("lastIndexOf", "(Ljava/lang/String;)I", |ctx, a| {
            let s = ctx.non_null(a[1])?;
            let target = ctx.string_utf16(s)?.to_vec();
            let b = buf(ctx, a)?;
            Ok(Value::Int(super::string::last_index_of(
                &b,
                &target,
                b.len() as i32,
            )))
        })
        .method("substring", "(I)Ljava/lang/String;", |ctx, a| {
            let n = len(ctx, a)? as i32;
            substring(ctx, a, a[1].int(), n)
        })
        .method("substring", "(II)Ljava/lang/String;", |ctx, a| {
            substring(ctx, a, a[1].int(), a[2].int())
        })
        .method("subSequence", "(II)Ljava/lang/CharSequence;", |ctx, a| {
            substring(ctx, a, a[1].int(), a[2].int())
        })
        .method("reverse", &format!("(){me}"), |ctx, a| {
            reverse(&mut buf(ctx, a)?);
            Ok(a[0])
        })
        .method("delete", &format!("(II){me}"), |ctx, a| {
            let n = len(ctx, a)?;
            let start = a[1].int();
            let end = a[2].int().min(n as i32);
            if start < 0 || start > end {
                return Err(index_error(
                    ctx,
                    format!("start {start}, end {end}, length {n}"),
                ));
            }
            buf(ctx, a)?.drain(start as usize..end as usize);
            Ok(a[0])
        })
        .method("deleteCharAt", &format!("(I){me}"), |ctx, a| {
            let i = a[1].int();
            let n = len(ctx, a)?;
            if i < 0 || i as usize >= n {
                return Err(index_error(ctx, format!("index {i},length {n}")));
            }
            buf(ctx, a)?.remove(i as usize);
            Ok(a[0])
        })
        .method(
            "replace",
            &format!("(IILjava/lang/String;){me}"),
            |ctx, a| {
                let s = ctx.non_null(a[3])?;
                let with = ctx.string_utf16(s)?.to_vec();
                let n = len(ctx, a)?;
                let start = a[1].int();
                let end = a[2].int().min(n as i32);
                if start < 0 || start as usize > n || start > end {
                    return Err(index_error(
                        ctx,
                        format!("start {start}, end {end}, length {n}"),
                    ));
                }
                buf(ctx, a)?.splice(start as usize..end as usize, with);
                Ok(a[0])
            },
        )
        .method("append", &format!("([CII){me}"), |ctx, a| {
            let arr = ctx.non_null(a[1])?;
            let size = ctx.array_len(arr)?;
            let (off, n) = check_range(
                ctx,
                "java/lang/IndexOutOfBoundsException",
                a[2].int(),
                a[3].int(),
                size,
            )?;
            let part = ctx.array::<u16>(arr)?[off..off + n].to_vec();
            buf(ctx, a)?.extend_from_slice(&part);
            Ok(a[0])
        })
        .method(
            "append",
            &format!("(Ljava/lang/CharSequence;II){me}"),
            |ctx, a| {
                let text = object_text(ctx, a[1])?;
                let (start, end) = (a[2].int(), a[3].int());
                if start < 0 || start > end || end as usize > text.len() {
                    let message = format!("start {start}, end {end}, length {}", text.len());
                    return throw(ctx, "java/lang/IndexOutOfBoundsException", &message);
                }
                buf(ctx, a)?.extend_from_slice(&text[start as usize..end as usize]);
                Ok(a[0])
            },
        )
        .method("insert", &format!("(I[CII){me}"), |ctx, a| {
            let arr = ctx.non_null(a[2])?;
            let size = ctx.array_len(arr)?;
            let (off, n) = check_range(
                ctx,
                "java/lang/StringIndexOutOfBoundsException",
                a[3].int(),
                a[4].int(),
                size,
            )?;
            let part = ctx.array::<u16>(arr)?[off..off + n].to_vec();
            insert_at(ctx, a, a[1].int(), part)
        });
    for desc in VALUE_DESCS.iter().copied().chain(["[C"]) {
        let append_desc = format!("({desc}){me}");
        let insert_desc = format!("(I{desc}){me}");
        let d = desc.to_owned();
        class = class.method("append", &append_desc, move |ctx, a| {
            let text = text_of(ctx, &d, a[1])?;
            buf(ctx, a)?.extend_from_slice(&text);
            Ok(a[0])
        });
        let d = desc.to_owned();
        class = class.method("insert", &insert_desc, move |ctx, a| {
            let text = text_of(ctx, &d, a[2])?;
            insert_at(ctx, a, a[1].int(), text)
        });
    }
    class
}

fn insert_at(ctx: &mut Ctx<'_>, a: &[Value], at: i32, text: Vec<u16>) -> JResult<Value> {
    let n = len(ctx, a)?;
    if at < 0 || at as usize > n {
        return Err(index_error(ctx, format!("offset {at}, length {n}")));
    }
    let at = at as usize;
    buf(ctx, a)?.splice(at..at, text);
    Ok(a[0])
}

fn substring(ctx: &mut Ctx<'_>, a: &[Value], begin: i32, end: i32) -> JResult<Value> {
    let n = len(ctx, a)? as i32;
    if begin < 0 || end > n || begin > end {
        return Err(index_error(
            ctx,
            format!("start {begin}, end {end}, length {n}"),
        ));
    }
    let part = buf(ctx, a)?[begin as usize..end as usize].to_vec();
    Ok(ctx.new_string_utf16(part)?.into())
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        class("java/lang/StringBuffer"),
        class("java/lang/StringBuilder"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_keeps_surrogate_pairs() {
        let mut s: Vec<u16> = "ab\u{1F600}c".encode_utf16().collect();
        reverse(&mut s);
        assert_eq!(String::from_utf16(&s).unwrap(), "c\u{1F600}ba");
    }

    #[test]
    fn sizes_past_the_heap_throw_out_of_memory_error() {
        let config = crate::VmConfig {
            heap_bytes: 1 << 20,
            ..crate::VmConfig::default()
        };
        let source = std::sync::Arc::new(crate::MemorySource::new());
        let vm = crate::Vm::new(config, source, Vec::new());
        let thrown = vm.attach("test", |ctx| {
            let mut thrown = Vec::new();
            for (name, size) in [("setLength", 50_000_000), ("ensureCapacity", i32::MAX)] {
                let sb = ctx.construct("java/lang/StringBuffer", "()V", &[]).unwrap();
                match ctx.call_virtual(sb, name, "(I)V", &[Value::Int(size)]) {
                    Err(Throw::Exception(e)) => thrown.push(ctx.describe_exception(e)),
                    other => thrown.push(format!("{name} returned {other:?}")),
                }
            }
            let sized = [Value::Int(i32::MAX)];
            match ctx.construct("java/lang/StringBuffer", "(I)V", &sized) {
                Err(Throw::Exception(e)) => thrown.push(ctx.describe_exception(e)),
                other => thrown.push(format!("<init> returned {other:?}")),
            }
            thrown
        });
        for t in thrown {
            assert!(t.starts_with("java.lang.OutOfMemoryError"), "{t}");
        }
    }
}
