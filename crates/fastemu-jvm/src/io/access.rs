//! Byte access to any Java stream from Rust, for the classes that wrap
//! one (the data streams, readers, writers, `PrintStream`).
//!
//! A wrapped stream may be a game's own subclass, so each operation calls
//! the stream's Java method through virtual dispatch, as Java's wrappers
//! do. A plain `ByteArrayInputStream` or `ByteArrayOutputStream`, which
//! no game code can have changed, is read or written straight through
//! its fields instead.

use super::bytes::{is_plain_output, read_plain, write_plain};
use crate::ctx::{Ctx, JResult};
use crate::value::{ObjRef, Value};

/// `stream.read()`: the next byte, or -1 at the end.
pub(crate) fn read_byte(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<i32> {
    let mut one = Vec::with_capacity(1);
    if let Some(n) = read_plain(ctx, stream, 1, &mut one)? {
        return Ok(if n > 0 { i32::from(one[0]) } else { -1 });
    }
    Ok(ctx.call_virtual(stream, "read", "()I", &[])?.int())
}

/// `stream.read(arr, off, len)`: what it read into the Java `byte[]`
/// `arr`, or -1 at the end. The caller has checked the range.
pub(crate) fn read_into(
    ctx: &mut Ctx<'_>,
    stream: ObjRef,
    arr: ObjRef,
    off: usize,
    len: usize,
) -> JResult<i32> {
    let mut bytes = Vec::new();
    if let Some(n) = read_plain(ctx, stream, len, &mut bytes)? {
        for (slot, &b) in ctx.array_mut::<i8>(arr)?[off..off + bytes.len()]
            .iter_mut()
            .zip(&bytes)
        {
            *slot = b as i8;
        }
        return Ok(n);
    }
    let args = [arr.into(), Value::Int(off as i32), Value::Int(len as i32)];
    Ok(ctx.call_virtual(stream, "read", "([BII)I", &args)?.int())
}

/// `stream.write(b)`.
pub(crate) fn write_byte(ctx: &mut Ctx<'_>, stream: ObjRef, b: i32) -> JResult<()> {
    if !write_plain(ctx, stream, &[b as u8])? {
        ctx.call_virtual(stream, "write", "(I)V", &[Value::Int(b)])?;
    }
    Ok(())
}

/// `stream.write(b)` for each of `bytes` in turn, as `writeInt` and its
/// kin write.
pub(crate) fn write_each(ctx: &mut Ctx<'_>, stream: ObjRef, bytes: &[u8]) -> JResult<()> {
    if !write_plain(ctx, stream, bytes)? {
        for &b in bytes {
            ctx.call_virtual(stream, "write", "(I)V", &[Value::Int(i32::from(b))])?;
        }
    }
    Ok(())
}

/// `stream.write(bytes, 0, bytes.length)`, with the bytes in a new Java
/// array for a stream that is not a plain `ByteArrayOutputStream`.
pub(crate) fn write_all(ctx: &mut Ctx<'_>, stream: ObjRef, bytes: &[u8]) -> JResult<()> {
    if !write_plain(ctx, stream, bytes)? {
        let arr = ctx.new_byte_array(bytes)?;
        let args = [arr.into(), Value::Int(0), Value::Int(bytes.len() as i32)];
        ctx.call_virtual(stream, "write", "([BII)V", &args)?;
    }
    Ok(())
}

/// `stream.write(arr, off, len)` with the caller's own Java array, whose
/// range the caller has checked.
pub(crate) fn write_array(
    ctx: &mut Ctx<'_>,
    stream: ObjRef,
    arr: ObjRef,
    off: usize,
    len: usize,
) -> JResult<()> {
    if is_plain_output(ctx, stream) {
        let bytes: Vec<u8> = ctx.array::<i8>(arr)?[off..off + len]
            .iter()
            .map(|&b| b as u8)
            .collect();
        write_plain(ctx, stream, &bytes)?;
    } else {
        let args = [arr.into(), Value::Int(off as i32), Value::Int(len as i32)];
        ctx.call_virtual(stream, "write", "([BII)V", &args)?;
    }
    Ok(())
}
