//! Reading a game's `InputStream` (or `SourceStream`) to its end.
//!
//! A `java.io.ByteArrayInputStream` (what `getResourceAsStream` returns)
//! is copied straight out of its buffer. Any other stream, the game's own
//! classes included, is read through its `read(byte[], int, int)` method,
//! so overrides run as on a phone.

use fastemu_core::LogLevel;
use fastemu_jvm::{ArrayType, Ctx, JResult, ObjRef, Value};

const BYTE_ARRAY_STREAM: &str = "java/io/ByteArrayInputStream";

/// Bytes asked for per `read` call.
const CHUNK: usize = 16 * 1024;

/// More sound data than this from one stream is cut off: a phone would
/// have run out of memory long before.
const MAX_BYTES: usize = 32 * 1024 * 1024;

/// Reads the `java.io.InputStream` `stream` to its end. An `IOException`
/// from the stream passes through.
pub(crate) fn read_all(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<Vec<u8>> {
    if let Some(bytes) = read_byte_array_stream(ctx, stream)? {
        return Ok(bytes);
    }
    read_chunks(ctx, stream, true)
}

/// Reads a `javax.microedition.media.protocol.SourceStream` to its end,
/// or until it has nothing more to give.
pub(crate) fn read_source_stream(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<Vec<u8>> {
    read_chunks(ctx, stream, false)
}

/// Calls `read(byte[], int, int)` until the end. `byte_reads`: the stream
/// is an `InputStream`, whose `read()` gets past a `read` that returned 0;
/// otherwise 0 ends the reading.
fn read_chunks(ctx: &mut Ctx<'_>, stream: ObjRef, byte_reads: bool) -> JResult<Vec<u8>> {
    let buffer = ctx.new_array(ArrayType::Byte, CHUNK as i32)?;
    let mut out = Vec::new();
    loop {
        let args = [buffer.into(), Value::Int(0), Value::Int(CHUNK as i32)];
        let count = ctx.call_virtual(stream, "read", "([BII)I", &args)?.int();
        if count < 0 || (count == 0 && !byte_reads) {
            break;
        }
        if count == 0 {
            // Some streams return 0 rather than block: one byte at a time
            // still gets to the end.
            let byte = ctx.call_virtual(stream, "read", "()I", &[])?.int();
            if byte < 0 {
                break;
            }
            out.push(byte as u8);
            continue;
        }
        let count = (count as usize).min(CHUNK);
        out.extend(ctx.array::<i8>(buffer)?[..count].iter().map(|&b| b as u8));
        if out.len() >= MAX_BYTES {
            ctx.log(
                LogLevel::Warn,
                format!(
                    "media: a sound stream is longer than {MAX_BYTES} bytes; keeping the start"
                ),
            );
            break;
        }
    }
    Ok(out)
}

/// The rest of a plain `ByteArrayInputStream`, read from its fields, or
/// `None` for any other stream (a subclass may override `read`).
fn read_byte_array_stream(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<Option<Vec<u8>>> {
    let class = ctx.find_class(BYTE_ARRAY_STREAM)?;
    if ctx.class_of(stream) != class {
        return Ok(None);
    }
    let buf = ctx.field_id(BYTE_ARRAY_STREAM, "buf", "[B")?;
    let pos = ctx.field_id(BYTE_ARRAY_STREAM, "pos", "I")?;
    let count = ctx.field_id(BYTE_ARRAY_STREAM, "count", "I")?;
    let Some(array) = ctx.get_field(stream, buf).obj() else {
        return Ok(None);
    };
    let at = ctx.get_field(stream, pos).int();
    let end = ctx.get_field(stream, count).int();
    let len = ctx.array_len(array)?;
    let (Ok(at), Ok(end)) = (usize::try_from(at), usize::try_from(end)) else {
        return Ok(None);
    };
    // Fields a game set out of range: let the stream's own read decide.
    if end > len || at > end {
        return Ok(None);
    }
    let bytes = ctx.array::<i8>(array)?[at..end]
        .iter()
        .map(|&b| b as u8)
        .collect();
    ctx.set_field(stream, pos, Value::Int(end as i32));
    Ok(Some(bytes))
}
