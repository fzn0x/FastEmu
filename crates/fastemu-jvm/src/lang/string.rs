//! `java.lang.String`: immutable UTF-16 text in a Rust payload, with a
//! cached hash. Literals and `intern()` share one table.

use super::number::{to_lower, to_upper};
use super::strbuf::StrBuf;
use super::{check_range, this, throw};
use crate::charset::Charset;
use crate::class::ArrayType;
use crate::ctx::{Ctx, JResult};
use crate::heap::Payload;
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};
use crate::vm::VmState;

/// The characters of a `String`.
#[derive(Default)]
pub(crate) struct JString {
    pub(crate) chars: Box<[u16]>,
    /// The hash, 0 until computed (as Java caches it).
    hash: i32,
}

impl Payload for JString {
    fn heap_bytes(&self) -> usize {
        24 + 2 * self.chars.len()
    }
}

/// Java's `String.hashCode`.
pub(crate) fn hash(chars: &[u16]) -> i32 {
    chars
        .iter()
        .fold(0i32, |h, &c| h.wrapping_mul(31).wrapping_add(i32::from(c)))
}

/// A new string object.
pub(crate) fn new_string(state: &mut VmState, chars: Vec<u16>) -> ObjRef {
    let obj = state.alloc_instance(state.wk.string);
    state.heap.get_mut(obj).payload = Some(Box::new(JString {
        chars: chars.into(),
        hash: 0,
    }));
    obj
}

/// The canonical string with these characters.
pub(crate) fn intern(state: &mut VmState, chars: &[u16]) -> ObjRef {
    if let Some(&s) = state.interned.get(chars) {
        return s;
    }
    let s = new_string(state, chars.to_vec());
    state.interned.insert(chars.into(), s);
    s
}

/// The characters of `obj`, if it is a string.
pub(crate) fn chars(state: &VmState, obj: ObjRef) -> Option<&[u16]> {
    state.heap.get(obj).payload::<JString>().map(|s| &*s.chars)
}

/// `hashCode()` of `obj` if it is a string, computed once and cached as
/// Java caches it.
pub(crate) fn cached_hash(state: &mut VmState, obj: ObjRef) -> Option<i32> {
    let s = state.heap.get_mut(obj).payload_mut::<JString>()?;
    if s.hash == 0 {
        s.hash = hash(&s.chars);
    }
    Some(s.hash)
}

fn units<'a>(ctx: &'a Ctx<'_>, obj: ObjRef) -> &'a [u16] {
    &ctx.state
        .heap
        .get(obj)
        .payload::<JString>()
        .expect("a String has a JString payload")
        .chars
}

fn this_units<'a>(ctx: &'a Ctx<'_>, a: &[Value]) -> &'a [u16] {
    units(ctx, this(a))
}

/// The characters of the string argument `v`, copied; throws
/// `NullPointerException` for `null`.
fn arg_units(ctx: &mut Ctx<'_>, v: Value) -> JResult<Vec<u16>> {
    let obj = ctx.non_null(v)?;
    Ok(units(ctx, obj).to_vec())
}

fn string_result(ctx: &mut Ctx<'_>, chars: Vec<u16>) -> JResult<Value> {
    Ok(ctx.new_string_utf16(chars)?.into())
}

/// The charset of conversions that name none (`new String(bytes)`,
/// `getBytes()`, readers, writers and print streams), from
/// `microedition.encoding`, the game's encoding setting.
pub(crate) fn default_charset(ctx: &Ctx<'_>) -> Charset {
    Charset::default_for(ctx.property("microedition.encoding"))
}

/// The charset the string argument `v` names; throws
/// `UnsupportedEncodingException` for one this virtual machine lacks.
pub(crate) fn named_charset(ctx: &mut Ctx<'_>, v: Value) -> JResult<Charset> {
    let name_obj = ctx.non_null(v)?;
    let name = ctx.string(name_obj)?;
    match Charset::lookup(&name) {
        Some(c) => Ok(c),
        None => throw(ctx, "java/io/UnsupportedEncodingException", &name),
    }
}

fn index_error(ctx: &mut Ctx<'_>, message: String) -> crate::ctx::Throw {
    ctx.new_exception("java/lang/StringIndexOutOfBoundsException", Some(&message))
}

/// Sets the characters of a string under construction.
#[allow(
    clippy::unnecessary_wraps,
    reason = "it has the shape of a native method body, so natives return it directly"
)]
fn init_with(ctx: &mut Ctx<'_>, a: &[Value], chars: Vec<u16>) -> JResult<Value> {
    ctx.set_payload(
        this(a),
        JString {
            chars: chars.into(),
            hash: 0,
        },
    );
    Ok(Value::Void)
}

fn bytes_of(ctx: &mut Ctx<'_>, v: Value) -> JResult<Vec<u8>> {
    let arr = ctx.non_null(v)?;
    Ok(ctx.array::<i8>(arr)?.iter().map(|&b| b as u8).collect())
}

fn decode_range(ctx: &mut Ctx<'_>, a: &[Value], charset: Option<Value>) -> JResult<Value> {
    let bytes = bytes_of(ctx, a[1])?;
    let (offset, len) = if a.len() >= 4 {
        check_range(
            ctx,
            "java/lang/StringIndexOutOfBoundsException",
            a[2].int(),
            a[3].int(),
            bytes.len(),
        )?
    } else {
        (0, bytes.len())
    };
    let charset = match charset {
        Some(v) => named_charset(ctx, v)?,
        None => default_charset(ctx),
    };
    let chars = charset.decode(&bytes[offset..offset + len]);
    init_with(ctx, a, chars)
}

/// `indexOf` for a code point, which may be a surrogate pair.
fn index_of_char(s: &[u16], ch: i32, from: i32) -> i32 {
    let from = from.max(0) as usize;
    if from >= s.len() {
        return -1;
    }
    if let Ok(c) = u16::try_from(ch) {
        return s[from..]
            .iter()
            .position(|&x| x == c)
            .map_or(-1, |i| (i + from) as i32);
    }
    let Some(c) = char::from_u32(ch as u32) else {
        return -1;
    };
    let mut pair = [0; 2];
    let pair = c.encode_utf16(&mut pair);
    index_of(s, pair, from as i32)
}

fn last_index_of_char(s: &[u16], ch: i32, from: i32) -> i32 {
    if from < 0 {
        return -1;
    }
    let end = (from as usize).min(s.len().saturating_sub(1));
    if s.is_empty() {
        return -1;
    }
    if let Ok(c) = u16::try_from(ch) {
        return s[..=end]
            .iter()
            .rposition(|&x| x == c)
            .map_or(-1, |i| i as i32);
    }
    let Some(c) = char::from_u32(ch as u32) else {
        return -1;
    };
    let mut pair = [0; 2];
    let pair = c.encode_utf16(&mut pair);
    last_index_of(s, pair, from)
}

/// `indexOf(String, int)`.
pub(crate) fn index_of(s: &[u16], target: &[u16], from: i32) -> i32 {
    let from = from.max(0) as usize;
    if from > s.len() {
        return if target.is_empty() {
            s.len() as i32
        } else {
            -1
        };
    }
    if target.is_empty() {
        return from as i32;
    }
    s[from..]
        .windows(target.len())
        .position(|w| w == target)
        .map_or(-1, |i| (i + from) as i32)
}

/// `lastIndexOf(String, int)`.
pub(crate) fn last_index_of(s: &[u16], target: &[u16], from: i32) -> i32 {
    if from < 0 || target.len() > s.len() {
        return -1;
    }
    let start = (from as usize).min(s.len() - target.len());
    (0..=start)
        .rev()
        .find(|&i| &s[i..i + target.len()] == target)
        .map_or(-1, |i| i as i32)
}

fn region_matches(
    s: &[u16],
    ignore_case: bool,
    offset: i32,
    other: &[u16],
    other_offset: i32,
    len: i32,
) -> bool {
    if offset < 0
        || other_offset < 0
        || i64::from(offset) > s.len() as i64 - i64::from(len)
        || i64::from(other_offset) > other.len() as i64 - i64::from(len)
    {
        return false;
    }
    if len <= 0 {
        return true;
    }
    let (o1, o2, n) = (offset as usize, other_offset as usize, len as usize);
    s[o1..o1 + n]
        .iter()
        .zip(&other[o2..o2 + n])
        .all(|(&a, &b)| {
            a == b
                || (ignore_case
                    && (to_upper(a) == to_upper(b)
                        || to_lower(to_upper(a)) == to_lower(to_upper(b))))
        })
}

fn compare(a: &[u16], b: &[u16]) -> i32 {
    for (&x, &y) in a.iter().zip(b) {
        if x != y {
            return i32::from(x) - i32::from(y);
        }
    }
    a.len() as i32 - b.len() as i32
}

fn is_trimmed(c: u16) -> bool {
    c <= u16::from(b' ')
}

fn value_of(ctx: &mut Ctx<'_>, text: String) -> JResult<Value> {
    super::string_value(ctx, &text)
}

pub(crate) fn classes() -> Vec<NativeClass> {
    let ret = "Ljava/lang/String;";
    vec![
        NativeClass::new("java/lang/String")
            .final_class()
            .implements("java/lang/CharSequence")
            .payload::<JString>()
            .method("<init>", "()V", |ctx, a| init_with(ctx, a, Vec::new()))
            .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
                let chars = arg_units(ctx, a[1])?;
                init_with(ctx, a, chars)
            })
            .method("<init>", "([C)V", |ctx, a| {
                let arr = ctx.non_null(a[1])?;
                let chars = ctx.array::<u16>(arr)?.to_vec();
                init_with(ctx, a, chars)
            })
            .method("<init>", "([CII)V", |ctx, a| {
                let arr = ctx.non_null(a[1])?;
                let len = ctx.array_len(arr)?;
                let (off, n) = check_range(
                    ctx,
                    "java/lang/StringIndexOutOfBoundsException",
                    a[2].int(),
                    a[3].int(),
                    len,
                )?;
                let chars = ctx.array::<u16>(arr)?[off..off + n].to_vec();
                init_with(ctx, a, chars)
            })
            .method("<init>", "([B)V", |ctx, a| decode_range(ctx, a, None))
            .method("<init>", "([BII)V", |ctx, a| decode_range(ctx, a, None))
            .method("<init>", "([BLjava/lang/String;)V", |ctx, a| {
                decode_range(ctx, a, Some(a[2]))
            })
            .method("<init>", "([BIILjava/lang/String;)V", |ctx, a| {
                decode_range(ctx, a, Some(a[4]))
            })
            .method("<init>", "(Ljava/lang/StringBuffer;)V", init_from_buffer)
            .method("<init>", "(Ljava/lang/StringBuilder;)V", init_from_buffer)
            .method("length", "()I", |ctx, a| {
                Ok(Value::Int(this_units(ctx, a).len() as i32))
            })
            .method("isEmpty", "()Z", |ctx, a| {
                Ok(Value::from(this_units(ctx, a).is_empty()))
            })
            .method("charAt", "(I)C", |ctx, a| {
                let i = a[1].int();
                match this_units(ctx, a).get(i as usize) {
                    Some(&c) => Ok(Value::Int(i32::from(c))),
                    None => Err(index_error(ctx, format!("String index out of range: {i}"))),
                }
            })
            .method("hashCode", "()I", |ctx, a| {
                let h =
                    cached_hash(&mut ctx.state, this(a)).expect("a String has a JString payload");
                Ok(Value::Int(h))
            })
            .method("equals", "(Ljava/lang/Object;)Z", |ctx, a| {
                let Some(other) = a[1].obj() else {
                    return Ok(Value::from(false));
                };
                let equal = match chars(&ctx.state, other) {
                    Some(o) => o == this_units(ctx, a),
                    None => false,
                };
                Ok(Value::from(equal))
            })
            .method("equalsIgnoreCase", "(Ljava/lang/String;)Z", |ctx, a| {
                let Some(other) = a[1].obj() else {
                    return Ok(Value::from(false));
                };
                let o = units(ctx, other);
                let s = this_units(ctx, a);
                Ok(Value::from(
                    s.len() == o.len() && region_matches(s, true, 0, o, 0, s.len() as i32),
                ))
            })
            .method("compareTo", "(Ljava/lang/String;)I", |ctx, a| {
                let o = arg_units(ctx, a[1])?;
                Ok(Value::Int(compare(this_units(ctx, a), &o)))
            })
            .method("compareTo", "(Ljava/lang/Object;)I", |ctx, a| {
                let o = arg_units(ctx, a[1])?;
                Ok(Value::Int(compare(this_units(ctx, a), &o)))
            })
            .method("toString", "()Ljava/lang/String;", |_, a| Ok(a[0]))
            .method("intern", "()Ljava/lang/String;", |ctx, a| {
                let chars = this_units(ctx, a).to_vec();
                Ok(ctx.intern(&chars).into())
            })
            .method(
                "concat",
                "(Ljava/lang/String;)Ljava/lang/String;",
                |ctx, a| {
                    let o = arg_units(ctx, a[1])?;
                    if o.is_empty() {
                        return Ok(a[0]);
                    }
                    let mut s = this_units(ctx, a).to_vec();
                    s.extend_from_slice(&o);
                    string_result(ctx, s)
                },
            )
            .method("startsWith", "(Ljava/lang/String;)Z", |ctx, a| {
                let o = arg_units(ctx, a[1])?;
                Ok(Value::from(this_units(ctx, a).starts_with(&o)))
            })
            .method("startsWith", "(Ljava/lang/String;I)Z", |ctx, a| {
                let o = arg_units(ctx, a[1])?;
                let s = this_units(ctx, a);
                let off = a[2].int();
                Ok(Value::from(
                    off >= 0 && (off as usize) <= s.len() && s[off as usize..].starts_with(&o),
                ))
            })
            .method("endsWith", "(Ljava/lang/String;)Z", |ctx, a| {
                let o = arg_units(ctx, a[1])?;
                Ok(Value::from(this_units(ctx, a).ends_with(&o)))
            })
            .method("indexOf", "(I)I", |ctx, a| {
                Ok(Value::Int(index_of_char(this_units(ctx, a), a[1].int(), 0)))
            })
            .method("indexOf", "(II)I", |ctx, a| {
                Ok(Value::Int(index_of_char(
                    this_units(ctx, a),
                    a[1].int(),
                    a[2].int(),
                )))
            })
            .method("indexOf", "(Ljava/lang/String;)I", |ctx, a| {
                let o = arg_units(ctx, a[1])?;
                Ok(Value::Int(index_of(this_units(ctx, a), &o, 0)))
            })
            .method("indexOf", "(Ljava/lang/String;I)I", |ctx, a| {
                let o = arg_units(ctx, a[1])?;
                Ok(Value::Int(index_of(this_units(ctx, a), &o, a[2].int())))
            })
            .method("lastIndexOf", "(I)I", |ctx, a| {
                let s = this_units(ctx, a);
                Ok(Value::Int(last_index_of_char(
                    s,
                    a[1].int(),
                    s.len() as i32 - 1,
                )))
            })
            .method("lastIndexOf", "(II)I", |ctx, a| {
                Ok(Value::Int(last_index_of_char(
                    this_units(ctx, a),
                    a[1].int(),
                    a[2].int(),
                )))
            })
            .method("lastIndexOf", "(Ljava/lang/String;)I", |ctx, a| {
                let o = arg_units(ctx, a[1])?;
                let s = this_units(ctx, a);
                Ok(Value::Int(last_index_of(s, &o, s.len() as i32)))
            })
            .method("lastIndexOf", "(Ljava/lang/String;I)I", |ctx, a| {
                let o = arg_units(ctx, a[1])?;
                Ok(Value::Int(last_index_of(
                    this_units(ctx, a),
                    &o,
                    a[2].int(),
                )))
            })
            .method("contains", "(Ljava/lang/CharSequence;)Z", |ctx, a| {
                let seq = ctx.non_null(a[1])?;
                let text = ctx.call_virtual(seq, "toString", "()Ljava/lang/String;", &[])?;
                let o = arg_units(ctx, text)?;
                Ok(Value::from(index_of(this_units(ctx, a), &o, 0) >= 0))
            })
            .method("regionMatches", "(ILjava/lang/String;II)Z", |ctx, a| {
                let o = arg_units(ctx, a[2])?;
                Ok(Value::from(region_matches(
                    this_units(ctx, a),
                    false,
                    a[1].int(),
                    &o,
                    a[3].int(),
                    a[4].int(),
                )))
            })
            .method("regionMatches", "(ZILjava/lang/String;II)Z", |ctx, a| {
                let o = arg_units(ctx, a[3])?;
                Ok(Value::from(region_matches(
                    this_units(ctx, a),
                    a[1].boolean(),
                    a[2].int(),
                    &o,
                    a[4].int(),
                    a[5].int(),
                )))
            })
            .method("substring", "(I)Ljava/lang/String;", |ctx, a| {
                let len = this_units(ctx, a).len() as i32;
                substring(ctx, a, a[1].int(), len)
            })
            .method("substring", "(II)Ljava/lang/String;", |ctx, a| {
                substring(ctx, a, a[1].int(), a[2].int())
            })
            .method("subSequence", "(II)Ljava/lang/CharSequence;", |ctx, a| {
                substring(ctx, a, a[1].int(), a[2].int())
            })
            .method("replace", "(CC)Ljava/lang/String;", |ctx, a| {
                let (from, to) = (a[1].char(), a[2].char());
                let s = this_units(ctx, a);
                if from == to || !s.contains(&from) {
                    return Ok(a[0]);
                }
                let replaced = s.iter().map(|&c| if c == from { to } else { c }).collect();
                string_result(ctx, replaced)
            })
            .method("toLowerCase", "()Ljava/lang/String;", |ctx, a| {
                map_case(ctx, a, to_lower, str::to_lowercase)
            })
            .method("toUpperCase", "()Ljava/lang/String;", |ctx, a| {
                map_case(ctx, a, to_upper, str::to_uppercase)
            })
            .method("trim", "()Ljava/lang/String;", |ctx, a| {
                let s = this_units(ctx, a);
                let start = s.iter().position(|&c| !is_trimmed(c)).unwrap_or(s.len());
                let end = s
                    .iter()
                    .rposition(|&c| !is_trimmed(c))
                    .map_or(start, |i| i + 1);
                if start == 0 && end == s.len() {
                    return Ok(a[0]);
                }
                let trimmed = s[start..end].to_vec();
                string_result(ctx, trimmed)
            })
            .method("toCharArray", "()[C", |ctx, a| {
                let chars = this_units(ctx, a).to_vec();
                let arr = ctx.new_array(ArrayType::Char, chars.len() as i32)?;
                ctx.array_mut::<u16>(arr)?.copy_from_slice(&chars);
                Ok(arr.into())
            })
            .method("getChars", "(II[CI)V", |ctx, a| {
                let s = this_units(ctx, a).to_vec();
                let (begin, end) = (a[1].int(), a[2].int());
                if begin < 0 || end > s.len() as i32 || begin > end {
                    return Err(index_error(
                        ctx,
                        format!("begin {begin}, end {end}, length {}", s.len()),
                    ));
                }
                let dst = ctx.non_null(a[3])?;
                let dst_len = ctx.array_len(dst)?;
                let n = end - begin;
                let (at, _) = check_range(
                    ctx,
                    "java/lang/ArrayIndexOutOfBoundsException",
                    a[4].int(),
                    n,
                    dst_len,
                )?;
                ctx.array_mut::<u16>(dst)?[at..at + n as usize]
                    .copy_from_slice(&s[begin as usize..end as usize]);
                Ok(Value::Void)
            })
            .method("getBytes", "()[B", |ctx, a| {
                let bytes = default_charset(ctx).encode(this_units(ctx, a));
                Ok(ctx.new_byte_array(&bytes)?.into())
            })
            .method("getBytes", "(Ljava/lang/String;)[B", |ctx, a| {
                let charset = named_charset(ctx, a[1])?;
                let bytes = charset.encode(this_units(ctx, a));
                Ok(ctx.new_byte_array(&bytes)?.into())
            })
            .static_method(
                "valueOf",
                &format!("(Ljava/lang/Object;){ret}"),
                // As in Java, a toString() that returns null makes
                // valueOf return null too.
                |ctx, a| match a[0].obj() {
                    None => value_of(ctx, "null".to_owned()),
                    Some(obj) => ctx.call_virtual(obj, "toString", "()Ljava/lang/String;", &[]),
                },
            )
            .static_method("valueOf", &format!("(Z){ret}"), |ctx, a| {
                value_of(
                    ctx,
                    if a[0].boolean() { "true" } else { "false" }.to_owned(),
                )
            })
            .static_method("valueOf", &format!("(C){ret}"), |ctx, a| {
                string_result(ctx, vec![a[0].char()])
            })
            .static_method("valueOf", &format!("(I){ret}"), |ctx, a| {
                value_of(ctx, a[0].int().to_string())
            })
            .static_method("valueOf", &format!("(J){ret}"), |ctx, a| {
                value_of(ctx, a[0].long().to_string())
            })
            .static_method("valueOf", &format!("(F){ret}"), |ctx, a| {
                value_of(ctx, super::float_fmt::float_to_string(a[0].float()))
            })
            .static_method("valueOf", &format!("(D){ret}"), |ctx, a| {
                value_of(ctx, super::float_fmt::double_to_string(a[0].double()))
            })
            .static_method("valueOf", &format!("([C){ret}"), |ctx, a| {
                let arr = ctx.non_null(a[0])?;
                let chars = ctx.array::<u16>(arr)?.to_vec();
                string_result(ctx, chars)
            })
            .static_method("valueOf", &format!("([CII){ret}"), chars_range)
            .static_method("copyValueOf", &format!("([C){ret}"), |ctx, a| {
                let arr = ctx.non_null(a[0])?;
                let chars = ctx.array::<u16>(arr)?.to_vec();
                string_result(ctx, chars)
            })
            .static_method("copyValueOf", &format!("([CII){ret}"), chars_range),
    ]
}

fn chars_range(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let arr = ctx.non_null(a[0])?;
    let len = ctx.array_len(arr)?;
    let (off, n) = check_range(
        ctx,
        "java/lang/StringIndexOutOfBoundsException",
        a[1].int(),
        a[2].int(),
        len,
    )?;
    let chars = ctx.array::<u16>(arr)?[off..off + n].to_vec();
    string_result(ctx, chars)
}

fn init_from_buffer(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let buf = ctx.non_null(a[1])?;
    let chars = ctx.payload::<StrBuf>(buf)?.chars.clone();
    init_with(ctx, a, chars)
}

fn substring(ctx: &mut Ctx<'_>, a: &[Value], begin: i32, end: i32) -> JResult<Value> {
    let s = this_units(ctx, a);
    let len = s.len() as i32;
    if begin < 0 || end > len || begin > end {
        return Err(index_error(
            ctx,
            format!("begin {begin}, end {end}, length {len}"),
        ));
    }
    if begin == 0 && end == len {
        return Ok(a[0]);
    }
    let part = s[begin as usize..end as usize].to_vec();
    string_result(ctx, part)
}

/// `toLowerCase` and `toUpperCase`: the full Unicode mapping (sharp s
/// upper-cases to `SS`, as in Java), or one character at a time when the
/// string holds unpaired surrogates.
fn map_case(
    ctx: &mut Ctx<'_>,
    a: &[Value],
    per_char: fn(u16) -> u16,
    full: fn(&str) -> String,
) -> JResult<Value> {
    let s = this_units(ctx, a);
    let mapped: Vec<u16> = match String::from_utf16(s) {
        Ok(text) => full(&text).encode_utf16().collect(),
        Err(_) => s.iter().map(|&c| per_char(c)).collect(),
    };
    if mapped == s {
        return Ok(a[0]);
    }
    string_result(ctx, mapped)
}

impl Ctx<'_> {
    /// The text of string `obj` without copying, for natives that read
    /// strings often.
    pub fn with_string<R>(&self, obj: ObjRef, f: impl FnOnce(&[u16]) -> R) -> JResult<R> {
        Ok(f(self.string_utf16(obj)?))
    }

    /// Whether `obj` is a `java.lang.String`.
    pub fn is_string(&self, obj: ObjRef) -> bool {
        self.state.heap.get(obj).payload::<JString>().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn hashes_match_java() {
        assert_eq!(hash(&u("")), 0);
        assert_eq!(hash(&u("hello")), 99_162_322);
        assert_eq!(hash(&u("polygenelubricants")), i32::MIN);
    }

    #[test]
    fn searches_follow_java_edge_cases() {
        let s = u("abcabc");
        assert_eq!(index_of(&s, &u("bc"), 0), 1);
        assert_eq!(index_of(&s, &u("bc"), 2), 4);
        assert_eq!(index_of(&s, &u(""), 9), 6);
        assert_eq!(index_of(&s, &u("x"), -3), -1);
        assert_eq!(last_index_of(&s, &u("bc"), 6), 4);
        assert_eq!(last_index_of(&s, &u("bc"), 3), 1);
        assert_eq!(last_index_of(&s, &u(""), 2), 2);
        assert_eq!(index_of_char(&s, 'c' as i32, -5), 2);
        assert_eq!(last_index_of_char(&s, 'a' as i32, 100), 3);
        let emoji = u("x\u{1F600}");
        assert_eq!(index_of_char(&emoji, 0x1F600, 0), 1);
    }

    #[test]
    fn comparisons_follow_java() {
        assert_eq!(compare(&u("apple"), &u("banana")), -1);
        assert_eq!(compare(&u("ab"), &u("abc")), -1);
        assert_eq!(compare(&u("b"), &u("a")), 1);
        assert!(region_matches(&u("Hello"), true, 1, &u("ELL"), 0, 3));
        assert!(!region_matches(&u("Hello"), false, 1, &u("ELL"), 0, 3));
        assert!(!region_matches(&u("Hello"), false, 4, &u("ELL"), 0, 3));
    }

    /// `new String(bytes)` and `getBytes()` with the encoding setting
    /// naming no known charset read and write ISO-8859-1.
    #[test]
    fn an_unknown_encoding_setting_means_latin_1() {
        let config = crate::VmConfig {
            properties: vec![("microedition.encoding".into(), "x-no-such".into())],
            ..crate::VmConfig::default()
        };
        let source = std::sync::Arc::new(crate::MemorySource::new());
        let vm = crate::Vm::new(config, source, Vec::new());
        let (text, bytes) = vm.attach("test", |ctx| {
            let arr = ctx.new_byte_array(&[0x41, 0xE9, 0xFF]).unwrap();
            let s = ctx
                .construct("java/lang/String", "([B)V", &[arr.into()])
                .unwrap();
            let text = ctx.string(s).unwrap();
            let back = ctx.call_virtual(s, "getBytes", "()[B", &[]).unwrap();
            let bytes = ctx.array::<i8>(back.obj().unwrap()).unwrap().to_vec();
            (text, bytes)
        });
        assert_eq!(text, "A\u{e9}\u{ff}");
        assert_eq!(bytes, [0x41, 0xE9_u8 as i8, -1]);
    }
}
