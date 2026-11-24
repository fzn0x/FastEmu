//! The `InputStream` and `OutputStream` a connection hands out, and the
//! `DataInputStream` and `DataOutputStream` wrappers.
//!
//! The two Java classes are thin: each instance's payload holds the
//! protocol's [`Input`] or [`Output`], and closing the stream drops it.

use std::sync::Arc;

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Value};

use super::{check_range, copy_in, copy_out, this, throw_io};

const INPUT_STREAM: &str = "fastemu/io/ConnectionInputStream";
const OUTPUT_STREAM: &str = "fastemu/io/ConnectionOutputStream";

/// The most one `read` or `write` call moves between Java and the
/// connection at a time.
const CHUNK: usize = 64 * 1024;

/// What a connection's input stream reads from.
pub(crate) trait Input: Send + Sync {
    /// Reads into `buf` (never empty), blocking until at least one byte
    /// arrives. `Ok(0)` means the end of the stream.
    fn read(&self, ctx: &mut Ctx<'_>, buf: &mut [u8]) -> JResult<usize>;
    /// How many bytes `read` can return without blocking.
    fn available(&self) -> usize;
    /// The game closed the stream.
    fn close(&self, ctx: &mut Ctx<'_>) -> JResult<()>;
}

/// What a connection's output stream writes to.
pub(crate) trait Output: Send + Sync {
    /// Writes all of `bytes`, blocking as needed.
    fn write(&self, ctx: &mut Ctx<'_>, bytes: &[u8]) -> JResult<()>;
    /// `OutputStream.flush`.
    fn flush(&self, ctx: &mut Ctx<'_>) -> JResult<()>;
    /// The game closed the stream.
    fn close(&self, ctx: &mut Ctx<'_>) -> JResult<()>;
}

/// The payload of a connection input stream: its source, until closed.
#[derive(Default)]
struct InputPayload(Option<Arc<dyn Input>>);

impl Payload for InputPayload {}

/// The payload of a connection output stream: its sink, until closed.
#[derive(Default)]
struct OutputPayload(Option<Arc<dyn Output>>);

impl Payload for OutputPayload {}

/// A new `InputStream` reading from `input`.
pub(crate) fn new_input(ctx: &mut Ctx<'_>, input: Arc<dyn Input>) -> JResult<ObjRef> {
    let class = ctx.find_class(INPUT_STREAM)?;
    let stream = ctx.new_object(class)?;
    ctx.set_payload(stream, InputPayload(Some(input)));
    Ok(stream)
}

/// A new `OutputStream` writing to `output`.
pub(crate) fn new_output(ctx: &mut Ctx<'_>, output: Arc<dyn Output>) -> JResult<ObjRef> {
    let class = ctx.find_class(OUTPUT_STREAM)?;
    let stream = ctx.new_object(class)?;
    ctx.set_payload(stream, OutputPayload(Some(output)));
    Ok(stream)
}

/// `new DataInputStream(stream)`. The class comes from the CLDC library
/// and is looked up by name when it is first needed.
pub(crate) fn data_input(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<ObjRef> {
    ctx.construct(
        "java/io/DataInputStream",
        "(Ljava/io/InputStream;)V",
        &[stream.into()],
    )
}

/// `new DataOutputStream(stream)`, looked up by name like
/// [`data_input`].
pub(crate) fn data_output(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<ObjRef> {
    ctx.construct(
        "java/io/DataOutputStream",
        "(Ljava/io/OutputStream;)V",
        &[stream.into()],
    )
}

fn input(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<Arc<dyn Input>> {
    match &ctx.payload::<InputPayload>(stream)?.0 {
        Some(input) => Ok(Arc::clone(input)),
        None => throw_io(ctx, "stream closed"),
    }
}

fn output(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<Arc<dyn Output>> {
    match &ctx.payload::<OutputPayload>(stream)?.0 {
        Some(output) => Ok(Arc::clone(output)),
        None => throw_io(ctx, "stream closed"),
    }
}

/// `read(byte[], int, int)`.
fn read_into(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let arr = ctx.non_null(a[1])?;
    let size = ctx.array_len(arr)?;
    let (off, len) = check_range(ctx, a[2].int(), a[3].int(), size)?;
    if len == 0 {
        return Ok(Value::Int(0));
    }
    let source = input(ctx, this(a))?;
    let mut buf = vec![0; len.min(CHUNK)];
    let n = source.read(ctx, &mut buf)?;
    if n == 0 {
        return Ok(Value::Int(-1));
    }
    copy_in(ctx, arr, off, &buf[..n])?;
    Ok(Value::Int(n as i32))
}

/// `write(byte[], int, int)`.
fn write_from(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let arr = ctx.non_null(a[1])?;
    let size = ctx.array_len(arr)?;
    let (off, len) = check_range(ctx, a[2].int(), a[3].int(), size)?;
    let sink = output(ctx, this(a))?;
    for start in (off..off + len).step_by(CHUNK) {
        let n = CHUNK.min(off + len - start);
        let bytes = copy_out(ctx, arr, start, n)?;
        sink.write(ctx, &bytes)?;
    }
    Ok(Value::Void)
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new(INPUT_STREAM)
            .extends("java/io/InputStream")
            .payload::<InputPayload>()
            .method("read", "()I", |ctx, a| {
                let source = input(ctx, this(a))?;
                let mut byte = [0];
                Ok(Value::Int(match source.read(ctx, &mut byte)? {
                    0 => -1,
                    _ => i32::from(byte[0]),
                }))
            })
            .method("read", "([BII)I", read_into)
            .method("skip", "(J)J", |ctx, a| {
                let n = a[1].long();
                if n <= 0 {
                    return Ok(Value::Long(0));
                }
                let source = input(ctx, this(a))?;
                let mut buf = vec![0; (n as u64).min(CHUNK as u64) as usize];
                Ok(Value::Long(source.read(ctx, &mut buf)? as i64))
            })
            .method("available", "()I", |ctx, a| {
                let source = input(ctx, this(a))?;
                Ok(Value::Int(
                    i32::try_from(source.available()).unwrap_or(i32::MAX),
                ))
            })
            .method("close", "()V", |ctx, a| {
                let taken = ctx.payload_mut::<InputPayload>(this(a))?.0.take();
                if let Some(source) = taken {
                    source.close(ctx)?;
                }
                Ok(Value::Void)
            }),
        NativeClass::new(OUTPUT_STREAM)
            .extends("java/io/OutputStream")
            .payload::<OutputPayload>()
            .method("write", "(I)V", |ctx, a| {
                let sink = output(ctx, this(a))?;
                sink.write(ctx, &[a[1].int() as u8])?;
                Ok(Value::Void)
            })
            .method("write", "([BII)V", write_from)
            .method("flush", "()V", |ctx, a| {
                let sink = ctx.payload::<OutputPayload>(this(a))?.0.clone();
                if let Some(sink) = sink {
                    sink.flush(ctx)?;
                }
                Ok(Value::Void)
            })
            .method("close", "()V", |ctx, a| {
                let taken = ctx.payload_mut::<OutputPayload>(this(a))?.0.take();
                if let Some(sink) = taken {
                    sink.close(ctx)?;
                }
                Ok(Value::Void)
            }),
    ]
}
