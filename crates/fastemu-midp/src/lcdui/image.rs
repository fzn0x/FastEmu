//! `javax.microedition.lcdui.Image` over `fastemu-gfx` surfaces.
//!
//! An image's pixels sit behind an `Arc<Mutex<Surface>>` in its payload,
//! so a `Graphics` drawing on a mutable image and a `Graphics` drawing the
//! image elsewhere reach the same pixels, and natives can hold the pixels
//! while they borrow other parts of the VM.

use std::cell::Cell;
use std::sync::Arc;
use std::time::Duration;

use fastemu_gfx::{GfxError, Surface, SurfaceKind};
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Throw, Value};
use parking_lot::Mutex;

use super::graphics::new_graphics;
use super::throw_gfx;
use crate::stream_bytes::CountedBytes;
use crate::suite::source::normalize;

const CLASS: &str = "javax/microedition/lcdui/Image";

/// Pixels shared between an image and the `Graphics` objects that draw on
/// it.
pub type SharedSurface = Arc<Mutex<Surface>>;

/// What a Java `Image` holds.
pub struct ImageData {
    pub surface: SharedSurface,
}

impl Payload for ImageData {
    fn heap_bytes(&self) -> usize {
        let surface = self.surface.lock();
        surface.width() as usize * surface.height() as usize * 4
    }
}

/// A new Java `Image` holding `surface`.
pub fn new_image(ctx: &mut Ctx<'_>, surface: Surface) -> JResult<ObjRef> {
    let class = ctx.find_class(CLASS)?;
    let obj = ctx.new_object(class)?;
    ctx.set_payload(
        obj,
        ImageData {
            surface: Arc::new(Mutex::new(surface)),
        },
    );
    Ok(obj)
}

/// The pixels of a Java `Image`.
pub fn surface_of(ctx: &Ctx<'_>, image: ObjRef) -> JResult<SharedSurface> {
    Ok(Arc::clone(&ctx.payload::<ImageData>(image)?.surface))
}

/// Asks the heap for the pixels of a `width` by `height` image before any
/// of them is made, so one that the heap could never hold is an
/// `OutOfMemoryError` and not a host allocation of up to 128 MiB. A size
/// that is not positive passes: the surface refuses it with the
/// `IllegalArgumentException` MIDP names.
pub(crate) fn reserve_pixels(ctx: &mut Ctx<'_>, width: i64, height: i64) -> JResult<()> {
    if width <= 0 || height <= 0 {
        return Ok(());
    }
    let bytes = (width as u64)
        .saturating_mul(height as u64)
        .saturating_mul(4);
    ctx.check_alloc(usize::try_from(bytes).unwrap_or(usize::MAX))
}

/// An immutable copy of `pixels`, made after the heap is asked for it, or
/// `None` when they are immutable already and an image can share them. The
/// copy is the size of an image the game holds, but a game that copies one
/// image again and again would hold gigabytes of copies otherwise.
pub(crate) fn snapshot_of(ctx: &mut Ctx<'_>, pixels: &SharedSurface) -> JResult<Option<Surface>> {
    let (width, height, mutable) = {
        let surface = pixels.lock();
        (surface.width(), surface.height(), surface.is_mutable())
    };
    if !mutable {
        return Ok(None);
    }
    reserve_pixels(ctx, width.into(), height.into())?;
    Ok(Some(pixels.lock().to_immutable()))
}

/// The image decodes of one game take turns: one runs at a time.
///
/// A decode holds host memory that the heap counts only once the image is
/// made: the file it read or copied, the decoder's buffers and the pixels.
/// The heap check before it reserves nothing, and the decode runs with the
/// VM lock released, so without turns every thread of a game could hold
/// that much at once, each checked against the same free room: eight
/// threads held 67 to 83 MiB where one held 17 MiB. With turns a game's
/// image calls hold one file and one decode beside the heap.
#[derive(Default)]
struct DecodeTurn(Arc<Mutex<()>>);

thread_local! {
    /// Whether this thread holds its game's decode turn now.
    static IN_TURN: Cell<bool> = const { Cell::new(false) };
}

/// Marks this thread as holding its game's decode turn while it lives,
/// unwinding included.
struct InTurn;

impl InTurn {
    fn enter() -> Self {
        IN_TURN.set(true);
        Self
    }
}

impl Drop for InTurn {
    fn drop(&mut self) {
        IN_TURN.set(false);
    }
}

/// How long a thread waits for its turn before it looks again whether its
/// game is stopping.
const TURN_SLICE: Duration = Duration::from_millis(100);

/// Runs `decode` in the game's decode turn: from before it reads or copies
/// the file to after the image is counted on the heap.
///
/// The rules that keep this free of deadlocks:
///
/// - Nothing waits for the turn while it holds the VM lock. The turn is
///   taken at once when it is free; otherwise the thread waits for it
///   with the VM lock released, which the thread in its turn needs to
///   finish. So the order is always the turn, then the VM lock, then (in
///   `Ctx::read_resource`) the source's inflating lock, which waits on
///   nothing.
/// - `decode` never calls Java: a game's code that made an image there
///   would wait for the turn its own thread holds. Reading a resource,
///   asking the heap, decoding and making the `Image` run no game code.
/// - It is not reentrant: a thread in its turn must not ask for it again.
///   Debug builds panic on that; release builds run the inner decode in
///   the turn the thread holds, rather than wait for themselves.
///
/// A thread that waited while its game was stopped returns `Throw::Stop`
/// without decoding.
fn decode_in_turn<R>(
    ctx: &mut Ctx<'_>,
    decode: impl FnOnce(&mut Ctx<'_>) -> JResult<R>,
) -> JResult<R> {
    let nested = IN_TURN.get();
    debug_assert!(!nested, "a decode asked for its game's turn again");
    if nested {
        return decode(ctx);
    }
    if ctx.extension::<DecodeTurn>().is_none() {
        ctx.set_extension(DecodeTurn::default());
    }
    let turns = Arc::clone(&ctx.extension::<DecodeTurn>().expect("set above").0);
    let _turn = match turns.try_lock() {
        Some(turn) => turn,
        None => {
            let vm = ctx.vm();
            let waited = ctx.without_lock(|| {
                loop {
                    if let Some(turn) = turns.try_lock_for(TURN_SLICE) {
                        break Some(turn);
                    }
                    if vm.is_stopping() {
                        break None;
                    }
                }
            });
            match waited {
                Some(turn) => turn,
                None => return Err(Throw::Stop),
            }
        }
    };
    // A thread that waited while its game was closed decodes nothing: the
    // game's code would not run on anyway.
    if ctx.is_stopping() {
        return Err(Throw::Stop);
    }
    let _inside = InTurn::enter();
    decode(ctx)
}

/// Checks the image `bytes` declares against the heap, then decodes it
/// with the VM lock released, as decoding is slow and touches nothing of
/// the VM. A size the heap cannot hold is `OutOfMemoryError`; a GIF whose
/// first frame has more pixels than the image and is more than the heap
/// can hold is a file that does not decode. Runs inside
/// [`decode_in_turn`] only.
fn decode_checked(ctx: &mut Ctx<'_>, bytes: Vec<u8>) -> JResult<Result<Surface, GfxError>> {
    debug_assert!(IN_TURN.get(), "an image decoded outside its game's turn");
    // A small file can declare an image of 128 MiB, so the decoder must not
    // be the first to look. A file whose size cannot be read goes to the
    // decoder, which refuses it for the same reason.
    if let Some((width, height)) = fastemu_gfx::probe_size(&bytes) {
        reserve_pixels(ctx, width.into(), height.into())?;
    }
    // The frame is decoded whole before it is cut to the image: 160 MiB
    // for a file of 34 bytes. One the heap could hold decodes as ever.
    if let Some(frame) = fastemu_gfx::probe_frame_bytes(&bytes) {
        match ctx.check_alloc(usize::try_from(frame).unwrap_or(usize::MAX)) {
            Ok(()) => {}
            Err(Throw::Stop) => return Err(Throw::Stop),
            Err(Throw::Exception(_)) => {
                return Ok(Err(GfxError::Decode(
                    "GIF: the heap cannot hold the first frame, which is larger than the image"
                        .into(),
                )));
            }
        }
    }
    Ok(ctx.without_lock(move || Surface::decode(&bytes)))
}

/// Makes the image decoded from the resource `path` in the game's decode
/// turn: `None` when there is no such resource.
pub(crate) fn image_from_resource(
    ctx: &mut Ctx<'_>,
    path: &str,
) -> JResult<Option<Result<ObjRef, GfxError>>> {
    decode_in_turn(ctx, |ctx| {
        let Some(bytes) = ctx.read_resource(path)? else {
            return Ok(None);
        };
        Ok(Some(match decode_checked(ctx, bytes)? {
            Ok(surface) => Ok(new_image(ctx, surface)?),
            Err(e) => Err(e),
        }))
    })
}

/// Makes an image of `kind` decoded from the `byte[]` `data`, or from the
/// part `offset .. offset + length` of it, in the game's decode turn. The
/// bytes are copied once the turn has come, and only the part. A part
/// outside the array is `ArrayIndexOutOfBoundsException`, before the heap
/// is asked.
pub(crate) fn image_from_array(
    ctx: &mut Ctx<'_>,
    data: ObjRef,
    range: Option<(i32, i32)>,
    kind: SurfaceKind,
) -> JResult<Result<ObjRef, GfxError>> {
    decode_in_turn(ctx, |ctx| {
        let array = ctx.array::<i8>(data)?;
        let part = match range {
            None => Some(array),
            Some((offset, length)) => usize::try_from(offset)
                .ok()
                .zip(usize::try_from(length).ok())
                .and_then(|(offset, length)| array.get(offset..offset.checked_add(length)?)),
        };
        let Some(part) = part else {
            return Ok(Err(GfxError::ArrayIndexOutOfBounds));
        };
        let bytes: Vec<u8> = part.iter().map(|&b| b as u8).collect();
        Ok(match decode_checked(ctx, bytes)? {
            Ok(surface) => Ok(new_image(ctx, surface.into_kind(kind))?),
            Err(e) => Err(e),
        })
    })
}

/// Turns a decode or size error of `createImage(String)` and
/// `createImage(InputStream)` into the `IOException` MIDP names, keeping
/// `OutOfMemoryError` as it is.
fn as_io_error(ctx: &mut Ctx<'_>, e: &GfxError) -> Throw {
    match e {
        GfxError::OutOfMemory => throw_gfx(ctx, e),
        _ => ctx.new_exception("java/io/IOException", Some(&e.to_string())),
    }
}

fn create_from_resource(ctx: &mut Ctx<'_>, name: Value) -> JResult<Value> {
    let name = ctx.non_null(name)?;
    let name = ctx.string(name)?;
    let path = normalize(&name);
    match image_from_resource(ctx, &path)? {
        Some(Ok(image)) => Ok(image.into()),
        Some(Err(e)) => Err(as_io_error(ctx, &e)),
        None => ctx.throw(
            "java/io/IOException",
            &format!("resource not found: {name}"),
        ),
    }
}

/// The most bytes `Image.createImage(InputStream)` reads. The PNG and JPEG
/// files of games are far smaller; the largest in the test corpus is
/// 144 KB.
const MAX_IMAGE_FILE: usize = 16 * 1024 * 1024;

/// How many reads in a row may return no bytes before the stream counts as
/// giving no data. `InputStream.read` blocks until it has a byte, so a read
/// that returns 0 again and again is a stream that is broken.
const MAX_EMPTY_READS: u32 = 1000;

/// Reads a Java `InputStream` to its end through its `read([BII)I`, at most
/// [`MAX_IMAGE_FILE`] bytes: an image stream that goes on is an
/// `IOException`, as a phone's would end in one, and not a Vec that grows
/// until the host is out of memory. The bytes grow in a holder the heap
/// counts, which this returns, and the heap is asked before they grow, so
/// data the heap cannot hold is an `OutOfMemoryError` at about the heap's
/// size. The game's `read` runs between the reads, and so do its other
/// threads, which would each hold a stream of their own otherwise.
fn read_stream(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<CountedBytes> {
    let buffer = ctx.new_array(fastemu_jvm::ArrayType::Byte, 4096)?;
    let bytes = CountedBytes::new(ctx)?;
    let mut chunk = [0u8; 4096];
    let mut empty_reads = 0;
    loop {
        let n = ctx
            .call_virtual(
                stream,
                "read",
                "([BII)I",
                &[buffer.into(), Value::Int(0), Value::Int(4096)],
            )?
            .int();
        if n < 0 {
            break;
        }
        let read = ctx.array::<i8>(buffer)?;
        let n = (n as usize).min(read.len());
        if n == 0 {
            empty_reads += 1;
            if empty_reads >= MAX_EMPTY_READS {
                return ctx.throw("java/io/IOException", "the stream gives no data");
            }
            continue;
        }
        empty_reads = 0;
        for (to, &from) in chunk.iter_mut().zip(&read[..n]) {
            *to = from as u8;
        }
        if bytes.len(ctx)? + n > MAX_IMAGE_FILE {
            return ctx.throw(
                "java/io/IOException",
                "the image data is larger than 16 MiB",
            );
        }
        bytes.push(ctx, &chunk[..n], MAX_IMAGE_FILE)?;
    }
    Ok(bytes)
}

fn create_from_stream(ctx: &mut Ctx<'_>, stream: Value) -> JResult<Value> {
    let stream = ctx.non_null(stream)?;
    // Read before the turn, as reading runs the game's code. The bytes stay
    // counted while the thread waits for its turn, and while they decode.
    let read = read_stream(ctx, stream)?;
    let made = decode_in_turn(ctx, |ctx| {
        let bytes = read.take(ctx)?;
        Ok(match decode_checked(ctx, bytes)? {
            Ok(surface) => Ok(new_image(ctx, surface)?),
            Err(e) => Err(e),
        })
    })?;
    match made {
        Ok(image) => Ok(image.into()),
        Err(e) => Err(as_io_error(ctx, &e)),
    }
}

fn create_from_bytes(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let data = ctx.non_null(a[0])?;
    let range = Some((a[1].int(), a[2].int()));
    match image_from_array(ctx, data, range, SurfaceKind::Immutable)? {
        Ok(image) => Ok(image.into()),
        Err(e) => Err(throw_gfx(ctx, &e)),
    }
}

fn create_copy(ctx: &mut Ctx<'_>, source: Value) -> JResult<Value> {
    let source = ctx.non_null(source)?;
    let pixels = surface_of(ctx, source)?;
    // MIDP allows handing back an immutable source itself.
    let Some(copy) = snapshot_of(ctx, &pixels)? else {
        return Ok(source.into());
    };
    Ok(new_image(ctx, copy)?.into())
}

fn create_region(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let source = ctx.non_null(a[0])?;
    let pixels = surface_of(ctx, source)?;
    let (x, y, width, height) = (a[1].int(), a[2].int(), a[3].int(), a[4].int());
    let (source_width, source_height) = {
        let surface = pixels.lock();
        (i64::from(surface.width()), i64::from(surface.height()))
    };
    // A region that is not inside the source is refused by `from_region`
    // with the exception MIDP names; one that is can be no larger than the
    // source.
    let inside = x >= 0
        && y >= 0
        && i64::from(x) + i64::from(width) <= source_width
        && i64::from(y) + i64::from(height) <= source_height;
    if inside {
        reserve_pixels(ctx, width.into(), height.into())?;
    }
    let result = Surface::from_region(
        &pixels.lock(),
        a[1].int(),
        a[2].int(),
        a[3].int(),
        a[4].int(),
        a[5].int(),
    );
    match result {
        Ok(surface) => Ok(new_image(ctx, surface)?.into()),
        Err(e) => Err(throw_gfx(ctx, &e)),
    }
}

fn create_rgb(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let rgb = ctx.non_null(a[0])?;
    // A size the array cannot fill is refused by `from_rgb` with the
    // exception MIDP names.
    let (width, height) = (i64::from(a[1].int()), i64::from(a[2].int()));
    if width * height <= ctx.array::<i32>(rgb)?.len() as i64 {
        reserve_pixels(ctx, width, height)?;
    }
    let result = Surface::from_rgb(
        ctx.array::<i32>(rgb)?,
        a[1].int(),
        a[2].int(),
        a[3].boolean(),
    );
    match result {
        Ok(surface) => Ok(new_image(ctx, surface)?.into()),
        Err(e) => Err(throw_gfx(ctx, &e)),
    }
}

fn create_mutable(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    reserve_pixels(ctx, a[0].int().into(), a[1].int().into())?;
    match Surface::new_mutable(a[0].int(), a[1].int()) {
        Ok(surface) => Ok(new_image(ctx, surface)?.into()),
        Err(e) => Err(throw_gfx(ctx, &e)),
    }
}

fn get_rgb(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let out = ctx.non_null(a[1])?;
    let pixels = surface_of(ctx, this)?;
    let surface = pixels.lock();
    let result = surface.get_rgb(
        ctx.array_mut::<i32>(out)?,
        a[2].int(),
        a[3].int(),
        a[4].int(),
        a[5].int(),
        a[6].int(),
        a[7].int(),
    );
    drop(surface);
    result.map(|()| Value::Void).map_err(|e| throw_gfx(ctx, &e))
}

fn get_graphics(ctx: &mut Ctx<'_>, this: Value) -> JResult<Value> {
    let this = ctx.non_null(this)?;
    let pixels = surface_of(ctx, this)?;
    if pixels.lock().kind() == SurfaceKind::Immutable {
        return ctx.throw(
            "java/lang/IllegalStateException",
            "an immutable image has no Graphics",
        );
    }
    Ok(new_graphics(ctx, pixels)?.into())
}

fn dimension(f: fn(&Surface) -> u32) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> {
    move |ctx, a| {
        let this = ctx.non_null(a[0])?;
        let pixels = surface_of(ctx, this)?;
        Ok(Value::Int(f(&pixels.lock()) as i32))
    }
}

/// The class.
pub fn class() -> NativeClass {
    const IMAGE: &str = "Ljavax/microedition/lcdui/Image;";
    NativeClass::new(CLASS)
        .static_method("createImage", &format!("(II){IMAGE}"), create_mutable)
        .static_method("createImage", &format!("({IMAGE}){IMAGE}"), |ctx, a| {
            create_copy(ctx, a[0])
        })
        .static_method(
            "createImage",
            &format!("(Ljava/lang/String;){IMAGE}"),
            |ctx, a| create_from_resource(ctx, a[0]),
        )
        .static_method("createImage", &format!("([BII){IMAGE}"), create_from_bytes)
        .static_method(
            "createImage",
            &format!("({IMAGE}IIIII){IMAGE}"),
            create_region,
        )
        .static_method(
            "createImage",
            &format!("(Ljava/io/InputStream;){IMAGE}"),
            |ctx, a| create_from_stream(ctx, a[0]),
        )
        .static_method("createRGBImage", &format!("([IIIZ){IMAGE}"), create_rgb)
        .method(
            "getGraphics",
            "()Ljavax/microedition/lcdui/Graphics;",
            |ctx, a| get_graphics(ctx, a[0]),
        )
        .method("getWidth", "()I", dimension(Surface::width))
        .method("getHeight", "()I", dimension(Surface::height))
        .method("isMutable", "()Z", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let pixels = surface_of(ctx, this)?;
            Ok(Value::from(pixels.lock().kind() != SurfaceKind::Immutable))
        })
        .method("getRGB", "([IIIIIII)V", get_rgb)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
    use std::sync::{Arc, mpsc};
    use std::time::{Duration, Instant};

    use fastemu_jvm::{JResult, MemorySource, Throw, Vm, VmConfig};

    use super::decode_in_turn;

    /// How long a test waits for something that comes at once when the
    /// turns work. It only matters when they deadlock: the test then fails
    /// instead of hanging.
    const PATIENCE: Duration = Duration::from_secs(30);

    fn vm() -> Vm {
        let config = VmConfig {
            log: Arc::new(|_| {}),
            ..VmConfig::default()
        };
        Vm::new(config, Arc::new(MemorySource::new()), Vec::new())
    }

    /// Spins until `done`, or until [`PATIENCE`] has passed.
    fn spin_until(done: impl Fn() -> bool) {
        let deadline = Instant::now() + PATIENCE;
        while !done() && Instant::now() < deadline {
            std::thread::yield_now();
        }
    }

    /// Runs `body` on a thread attached to `vm` as `name` and returns where
    /// its result will arrive.
    fn attached(
        vm: &Vm,
        name: &str,
        body: impl FnOnce(&mut fastemu_jvm::Ctx<'_>) -> JResult<()> + Send + 'static,
    ) -> mpsc::Receiver<JResult<()>> {
        let (vm, name) = (vm.clone(), name.to_owned());
        let (done, result) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = done.send(vm.attach(&name, body));
        });
        result
    }

    /// The result `thread` sent, or a failure when none came in time.
    fn joined(thread: &mpsc::Receiver<JResult<()>>) -> JResult<()> {
        thread
            .recv_timeout(PATIENCE)
            .expect("the decode turn deadlocked")
    }

    #[test]
    fn the_threads_of_a_game_decode_one_image_at_a_time() {
        const THREADS: usize = 4;
        let vm = vm();
        let asked = Arc::new(AtomicUsize::new(0));
        // How many are inside now, and the most that were at once.
        let inside = Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)));
        let threads: Vec<_> = (0..THREADS)
            .map(|i| {
                let (asked, inside) = (Arc::clone(&asked), Arc::clone(&inside));
                attached(&vm, &format!("decoder {i}"), move |ctx| {
                    asked.fetch_add(1, SeqCst);
                    decode_in_turn(ctx, |ctx| {
                        let now = inside.0.fetch_add(1, SeqCst) + 1;
                        inside.1.fetch_max(now, SeqCst);
                        // The first decode lasts until every thread has
                        // asked for its turn, with the VM lock released as
                        // a decode runs: without turns they would all be
                        // inside now.
                        ctx.without_lock(|| spin_until(|| asked.load(SeqCst) == THREADS));
                        inside.0.fetch_sub(1, SeqCst);
                        Ok(())
                    })
                })
            })
            .collect();
        for thread in &threads {
            assert!(joined(thread).is_ok());
        }
        assert_eq!(asked.load(SeqCst), THREADS);
        assert_eq!(inside.1.load(SeqCst), 1, "two decodes ran at once");
    }

    #[test]
    fn a_thread_that_waits_for_its_turn_while_the_game_stops_decodes_nothing() {
        let vm = vm();
        let first_inside = Arc::new(AtomicBool::new(false));
        let second_asked = Arc::new(AtomicBool::new(false));
        let second_ran = Arc::new(AtomicBool::new(false));
        let first = {
            let (stopping, first_inside, second_asked) = (
                vm.clone(),
                Arc::clone(&first_inside),
                Arc::clone(&second_asked),
            );
            attached(&vm, "first", move |ctx| {
                decode_in_turn(ctx, |ctx| {
                    first_inside.store(true, SeqCst);
                    // Holds the turn until the second thread asked for it
                    // and the game is stopping.
                    ctx.without_lock(|| {
                        spin_until(|| second_asked.load(SeqCst) && stopping.is_stopping());
                    });
                    Ok(())
                })
            })
        };
        spin_until(|| first_inside.load(SeqCst));
        assert!(first_inside.load(SeqCst), "the first thread never decoded");
        let second = {
            let (second_asked, second_ran) = (Arc::clone(&second_asked), Arc::clone(&second_ran));
            attached(&vm, "second", move |ctx| {
                second_asked.store(true, SeqCst);
                decode_in_turn(ctx, |_| {
                    second_ran.store(true, SeqCst);
                    Ok(())
                })
            })
        };
        spin_until(|| second_asked.load(SeqCst));
        vm.stop();
        assert!(joined(&first).is_ok());
        assert!(matches!(joined(&second), Err(Throw::Stop)));
        assert!(
            !second_ran.load(SeqCst),
            "a decode ran after the game stopped"
        );
    }
}
