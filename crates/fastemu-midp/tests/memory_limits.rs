//! What a game can make in host memory is counted against its heap:
//! images, tile layers, canvas back buffers and the surfaces of custom
//! items. A request the heap cannot hold is an `OutOfMemoryError` before
//! any pixels are made, and a game that keeps such objects in a list
//! reaches the limit instead of gigabytes.
//!
//! Each test runs one MIDlet of `tests/memory_java` on a phone whose heap
//! is 1 MiB, and reads what it printed. Some also measure the host: the
//! tests run one at a time under a counting allocator, which keeps the
//! most bytes the process held at once, so a request that reached the
//! host shows even when the error the game gets is the same. Without a
//! JDK the tests print why and pass, unless `FASTEMU_REQUIRE_JAVA=1` is
//! set (CI sets it).

#![allow(
    unsafe_code,
    reason = "the counting allocator forwards every call to the system's"
)]

mod support;

use std::alloc::{GlobalAlloc, Layout, System};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use fastemu_core::{Frame, Key, default_profile};
use support::{Running, rgb, scratch, zip};

/// The bytes the process holds now, and the most it held since a test
/// began.
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

/// The system's allocator, counting what it hands out. A zeroed block is
/// counted at its size whether or not anything touches it, so a buffer the
/// operating system gives out lazily shows too.
struct Counting;

fn grew(by: usize) {
    let now = LIVE.fetch_add(by, Relaxed) + by;
    PEAK.fetch_max(now, Relaxed);
}

// SAFETY: every method forwards to `System`, which upholds the contract of
// `GlobalAlloc`, with the same arguments, and only adds to atomic counters.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract is passed on unchanged.
        let block = unsafe { System.alloc(layout) };
        if !block.is_null() {
            grew(layout.size());
        }
        block
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract is passed on unchanged.
        let block = unsafe { System.alloc_zeroed(layout) };
        if !block.is_null() {
            grew(layout.size());
        }
        block
    }

    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract is passed on unchanged.
        unsafe { System.dealloc(block, layout) };
        LIVE.fetch_sub(layout.size(), Relaxed);
    }

    unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller's contract is passed on unchanged.
        let moved = unsafe { System.realloc(block, layout, new_size) };
        if !moved.is_null() {
            LIVE.fetch_sub(layout.size(), Relaxed);
            grew(new_size);
        }
        moved
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// Keeps the other tests of this file from running while one measures, and
/// remembers where its measure began.
struct Alone {
    _turn: MutexGuard<'static, ()>,
    start: usize,
}

impl Alone {
    fn new() -> Self {
        static TURNS: Mutex<()> = Mutex::new(());
        let turn = TURNS.lock().unwrap_or_else(PoisonError::into_inner);
        let start = LIVE.load(Relaxed);
        PEAK.store(start, Relaxed);
        Self { _turn: turn, start }
    }

    /// How many bytes beyond those the test began with the process held at
    /// most, in MiB.
    fn peak_mib(&self) -> usize {
        PEAK.load(Relaxed).saturating_sub(self.start) >> 20
    }
}

/// The heap of the phone the tests run on.
const HEAP: u64 = 1024 * 1024;

/// More than the process holds while a MIDlet runs on a heap of 1 MiB, and
/// less than the 64 MiB of the smallest image or grid a test refuses.
const LIMIT_MIB: usize = 16;

const MANIFEST: &str = "Manifest-Version: 1.0\r\n\
MIDlet-Name: Memory\r\n\
MIDlet-Vendor: FastEmu\r\n\
MIDlet-Version: 1.0\r\n\
MIDlet-1: BigImage, , m.BigImage\r\n\
MIDlet-2: ImageFile, , m.ImageFile\r\n\
MIDlet-3: Streams, , m.Streams\r\n\
MIDlet-4: Tiles, , m.Tiles\r\n\
MIDlet-5: Sprites, , m.Sprites\r\n\
MIDlet-6: Canvases, , m.Canvases\r\n\
MIDlet-7: GameBuffer, , m.GameBuffer\r\n\
MIDlet-8: Customs, , m.Customs\r\n\
MIDlet-9: SiemensBitmaps, , m.SiemensBitmaps\r\n\
MIDlet-10: SiemensLayers, , m.SiemensLayers\r\n\
MIDlet-11: SiemensSprites, , m.SiemensSprites\r\n\
MIDlet-12: Returns, , m.Returns\r\n";

/// Which `MIDlet-<n>` (from zero) runs each program.
mod midlet {
    pub const BIG_IMAGE: usize = 0;
    pub const IMAGE_FILE: usize = 1;
    pub const STREAMS: usize = 2;
    pub const TILES: usize = 3;
    pub const SPRITES: usize = 4;
    pub const CANVASES: usize = 5;
    pub const GAME_BUFFER: usize = 6;
    pub const CUSTOMS: usize = 7;
    pub const SIEMENS_BITMAPS: usize = 8;
    pub const SIEMENS_LAYERS: usize = 9;
    pub const SIEMENS_SPRITES: usize = 10;
    pub const RETURNS: usize = 11;
}

/// What the back buffer of a shown canvas takes of the heap on the default
/// phone, 240 by 320 pixels of 4 bytes.
const SCREEN_BYTES: i64 = 240 * 320 * 4;

/// The MIDlets, compiled once against the shared stubs and this test's
/// own, or `None` (after saying why) when there is no JDK.
fn classes() -> Option<&'static Path> {
    static CLASSES: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let compiled = CLASSES.get_or_init(|| {
        let javac = support::tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
        let stubs = std::env::join_paths([
            tests.join("memory_java").join("stubs"),
            tests.join("java").join("stubs"),
            tests.join("screens_java").join("stubs"),
            tests.join("vendor_stubs"),
        ])
        .expect("joining the stub folders");
        let out = scratch("memory-classes");
        let mut sources: Vec<PathBuf> = std::fs::read_dir(tests.join("memory_java/midlets/m"))
            .expect("the MIDlet sources")
            .map(|e| e.expect("an entry").path())
            .collect();
        sources.sort();
        // The stubs are only read: -implicit:none keeps their classes out
        // of the JAR, so the runtime's classes serve their names.
        let output = Command::new(javac)
            .args(["--release", "8", "-g", "-encoding", "UTF-8", "-nowarn"])
            .args(["-implicit:none", "-sourcepath"])
            .arg(stubs)
            .arg("-d")
            .arg(&out)
            .args(&sources)
            .output()
            .map_err(|e| format!("running javac: {e}"))?;
        assert!(
            output.status.success(),
            "javac failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(out)
    });
    match compiled {
        Ok(dir) => Some(dir),
        Err(why) => {
            assert!(
                std::env::var("FASTEMU_REQUIRE_JAVA").as_deref() != Ok("1"),
                "Java is required but unusable: {why}"
            );
            eprintln!("skipping: {why}");
            None
        }
    }
}

/// Packs the MIDlets and `resources` into a JAR in a scratch folder of
/// its own.
fn suite(name: &str, resources: &[(&str, Vec<u8>)]) -> Option<PathBuf> {
    let classes = classes()?;
    let mut entries = vec![(
        "META-INF/MANIFEST.MF".to_owned(),
        MANIFEST.as_bytes().to_vec(),
    )];
    let mut files: Vec<PathBuf> = std::fs::read_dir(classes.join("m"))
        .expect("the compiled MIDlets")
        .map(|e| e.expect("an entry").path())
        .collect();
    files.sort();
    for file in files {
        let name = format!("m/{}", file.file_name().unwrap().to_string_lossy());
        entries.push((name, std::fs::read(&file).expect("a class file")));
    }
    for (path, data) in resources {
        entries.push(((*path).to_owned(), data.clone()));
    }
    let path = scratch(name).join("memory.jar");
    std::fs::write(&path, zip(&entries, true)).expect("writing the JAR");
    Some(path)
}

/// Starts MIDlet `midlet` on the default phone with a heap of `heap`
/// bytes.
fn launch(jar: &Path, midlet: usize, heap: u64) -> Running {
    let profile = fastemu_core::DeviceProfile {
        heap_bytes: heap,
        ..default_profile()
    };
    Running::launch_on(jar, midlet, profile)
}

/// Starts MIDlet `midlet` on a Siemens phone, whose classes it uses, with a
/// heap of `heap` bytes.
fn siemens(jar: &Path, midlet: usize, heap: u64) -> Running {
    let profile = fastemu_core::DeviceProfile {
        heap_bytes: heap,
        ..fastemu_core::builtin_profiles()
            .into_iter()
            .find(|p| p.id.starts_with("siemens"))
            .expect("a built-in Siemens profile")
    };
    Running::launch_on(jar, midlet, profile)
}

/// A PNG whose header declares `width` by `height` pixels and whose data
/// is one row: a few hundred bytes for an image the decoder would give
/// 128 MiB.
fn declared_png(width: u32, height: u32) -> Vec<u8> {
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8], data: &[u8]| {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        png.extend_from_slice(&body);
        png.extend_from_slice(&crc32fast::hash(&body).to_be_bytes());
    };
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &header);
    let row = vec![0; 1 + 3 * width as usize];
    chunk(
        b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(&row, 6),
    );
    chunk(b"IEND", &[]);
    png
}

/// A BMP of 54 bytes, a header and no pixels, that declares `width` by
/// `height` pixels of 24 bits.
fn declared_bmp(width: i32, height: i32) -> Vec<u8> {
    let mut bmp = b"BM".to_vec();
    bmp.extend_from_slice(&54_u32.to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&54_u32.to_le_bytes());
    bmp.extend_from_slice(&40_u32.to_le_bytes());
    bmp.extend_from_slice(&width.to_le_bytes());
    bmp.extend_from_slice(&height.to_le_bytes());
    bmp.extend_from_slice(&1_u16.to_le_bytes());
    bmp.extend_from_slice(&24_u16.to_le_bytes());
    bmp.extend_from_slice(&[0; 24]);
    bmp
}

/// The numbers after `prefix` in `line`, which starts with it.
fn numbers(line: &str, prefix: &str) -> Vec<i64> {
    line[prefix.len()..]
        .split(|c: char| !c.is_ascii_digit() && c != '-')
        .filter_map(|n| n.parse().ok())
        .collect()
}

#[test]
fn images_are_refused_before_their_pixels_and_kept_ones_end_at_the_heap() {
    let alone = Alone::new();
    let Some(jar) = suite("big-image", &[]) else {
        return;
    };
    let mut run = launch(&jar, midlet::BIG_IMAGE, HEAP);
    run.wait_log("done ");
    let log = run.dump();
    for line in [
        "big image OutOfMemoryError",
        "huge image OutOfMemoryError",
        // A size the array cannot fill is still the exception MIDP names.
        "rgb image ArrayIndexOutOfBoundsException",
        "empty image IllegalArgumentException",
        "small image 64x64",
    ] {
        assert!(run.has(line), "no {line:?}; the log:\n{log}");
    }
    // The refused requests counted only the errors they threw: the 64 MiB
    // of the image would be far more than the heap.
    let changed = run.wait_log("free changed ");
    let used = numbers(&changed, "free changed ")[0];
    assert!((0..8192).contains(&used), "{changed}; {log}");
    // Images kept in a list end in the error at the heap: 400 by 400 pixels
    // are 640000 bytes of 1048576.
    let kept = run.wait_log("kept ");
    assert!(kept.ends_with("then OutOfMemoryError"), "{log}");
    let made = numbers(&kept, "kept ")[0];
    assert!((1..20).contains(&made), "kept {made} images; {log}");
    // And the host never held the 64 MiB of the image that was refused, nor
    // the 128 MiB that a size of 2 billion by 2 billion would saturate to.
    assert!(alone.peak_mib() < LIMIT_MIB, "{} MiB", alone.peak_mib());
}

#[test]
fn a_file_that_declares_more_pixels_than_the_heap_holds_is_refused_before_it_is_decoded() {
    let alone = Alone::new();
    let png = declared_png(16384, 2048);
    let bmp = declared_bmp(16384, 2048);
    assert_eq!(fastemu_gfx::probe_size(&png), Some((16384, 2048)));
    assert_eq!(fastemu_gfx::probe_size(&bmp), Some((16384, 2048)));
    let resources = [
        ("bomb.png", png),
        ("bomb.bmp", bmp),
        ("dot.png", support::test_png()),
    ];
    let Some(jar) = suite("image-file", &resources) else {
        return;
    };
    let mut run = launch(&jar, midlet::IMAGE_FILE, HEAP);
    run.wait_log("done");
    let log = run.dump();
    for file in ["/bomb.png", "/bomb.bmp"] {
        for how in ["by name", "by bytes", "by stream"] {
            let line = format!("{file} {how} OutOfMemoryError");
            assert!(run.has(&line), "no {line:?}; the log:\n{log}");
        }
    }
    // A real image still loads, the three ways.
    for how in ["by name", "by bytes", "by stream"] {
        let line = format!("/dot.png {how} ok 2x2");
        assert!(run.has(&line), "no {line:?}; the log:\n{log}");
    }
    // Six files that declare 128 MiB each were read, and none was decoded.
    assert!(alone.peak_mib() < LIMIT_MIB, "{} MiB", alone.peak_mib());
}

#[test]
fn an_image_stream_that_never_ends_is_an_io_exception_at_16_mib() {
    let alone = Alone::new();
    let Some(jar) = suite("streams", &[]) else {
        return;
    };
    let mut run = launch(&jar, midlet::STREAMS, HEAP);
    let line = run.wait_log("endless ");
    let log = run.dump();
    assert!(
        line.starts_with("endless IOException the image data is larger than 16 MiB calls "),
        "{log}"
    );
    // 4 KiB a read: 4096 reads fill 16 MiB and the next one is refused.
    let message = "endless IOException the image data is larger than 16 MiB";
    assert_eq!(numbers(&line, message), [4097, 4097 * 4096], "{log}");
    // A stream that gives no bytes ends after 1000 reads in a row.
    let empty = run.wait_log("empty ");
    assert_eq!(
        empty, "empty IOException the stream gives no data calls 1000",
        "{log}"
    );
    // The bytes read are held once and are 16 MiB at most; the vector they
    // grow in may hold its old and its new block at a doubling.
    assert!(alone.peak_mib() < 40, "{} MiB", alone.peak_mib());
}

#[test]
fn a_tiled_layer_the_heap_cannot_hold_is_an_out_of_memory_error_before_its_grid() {
    let alone = Alone::new();
    let Some(jar) = suite("tiles", &[]) else {
        return;
    };
    let mut run = launch(&jar, midlet::TILES, HEAP);
    run.wait_log("done ");
    let log = run.dump();
    // 4096 by 4096 cells are 64 MiB.
    assert!(run.has("big layer OutOfMemoryError"), "{log}");
    // A layer that fits works.
    assert!(run.has("small layer 20x20 cell 3"), "{log}");
    // A layer is 360000 bytes: they end in the error at the heap, and a
    // heap that is let go makes layers again.
    let kept = run.wait_log("layers ");
    assert!(kept.ends_with("then OutOfMemoryError"), "{log}");
    let count = numbers(&kept, "layers ")[0];
    assert!((1..10).contains(&count), "{log}");
    // Neither the 64 MiB grid that was refused nor a hundred of 360000
    // bytes were made.
    assert!(alone.peak_mib() < LIMIT_MIB, "{} MiB", alone.peak_mib());
}

#[test]
fn animated_tiles_stop_at_32767_with_an_out_of_memory_error() {
    let _alone = Alone::new();
    let Some(jar) = suite("animated", &[]) else {
        return;
    };
    let mut run = launch(&jar, midlet::TILES, HEAP);
    let line = run.wait_log("animated ");
    let log = run.dump();
    assert_eq!(
        line, "animated 32767 then OutOfMemoryError last -32767",
        "{log}"
    );
    // The tiles made before the error work.
    assert!(run.has("animated tile 1 shows 1"), "{log}");
}

#[test]
fn many_sprites_that_keep_one_big_frame_sequence_end_in_out_of_memory() {
    let _alone = Alone::new();
    let Some(jar) = suite("sprites", &[]) else {
        return;
    };
    let mut run = launch(&jar, midlet::SPRITES, HEAP);
    let line = run.wait_log("sprites ");
    let log = run.dump();
    // The array is 400 KB and each sprite keeps another copy.
    assert!(line.ends_with("kept then OutOfMemoryError"), "{log}");
    let count = numbers(&line, "sprites ")[0];
    assert!((1..10).contains(&count), "{log}");
    // A copy of a sprite keeps a sequence of its own, 80 KB.
    let copies = run.wait_log("copies ");
    assert!(copies.ends_with("kept then OutOfMemoryError"), "{log}");
    let count = numbers(&copies, "copies ")[0];
    assert!((1..20).contains(&count), "{log}");
    run.wait_log("done");
}

#[test]
fn a_shown_canvas_counts_its_back_buffer_and_canvases_kept_end_in_out_of_memory() {
    let _alone = Alone::new();
    let Some(jar) = suite("canvases", &[]) else {
        return;
    };
    let mut run = launch(&jar, midlet::CANVASES, HEAP);
    let first = run.wait_log("first canvas counts ");
    let log = run.dump();
    let counted = numbers(&first, "first canvas counts ")[0];
    assert!(counted >= SCREEN_BYTES * 3 / 4, "{first}; {log}");
    // 307200 bytes each: the heap of 1 MiB holds three and a bit.
    let shown = run.wait_log("canvases ");
    assert!(shown.ends_with("then OutOfMemoryError"), "{log}");
    let count = numbers(&shown, "canvases ")[0];
    assert!((1..10).contains(&count), "{log}");
    run.wait_log("done");
}

#[test]
fn a_game_canvas_buffer_the_heap_cannot_hold_is_refused_before_it_is_made() {
    let alone = Alone::new();
    let Some(jar) = suite("game-buffer", &[]) else {
        return;
    };
    // A phone with a screen of 3072 by 2048 pixels and a heap of 1 MiB:
    // the buffer of a GameCanvas is 24 MiB.
    let profile = fastemu_core::DeviceProfile {
        screen_width: 3072,
        screen_height: 2048,
        heap_bytes: HEAP,
        ..default_profile()
    };
    let mut run = Running::launch_on(&jar, midlet::GAME_BUFFER, profile);
    run.wait_log("done");
    let log = run.dump();
    assert!(run.has("game canvas OutOfMemoryError"), "{log}");
    let changed = run.wait_log("game canvas free changed ");
    let used = numbers(&changed, "game canvas free changed ")[0];
    assert!((0..8192).contains(&used), "{log}");
    assert!(alone.peak_mib() < LIMIT_MIB, "{} MiB", alone.peak_mib());
}

#[test]
fn custom_items_count_their_surfaces_and_screens_not_shown_give_their_buffers_back() {
    let _alone = Alone::new();
    let Some(jar) = suite("customs", &[]) else {
        return;
    };
    let mut run = launch(&jar, midlet::CUSTOMS, HEAP);
    run.wait_log("done");
    let log = run.dump();
    // The item of 240 by 8192 pixels, 7.8 MB, is not made.
    assert!(run.has("custom big painted false"), "{log}");
    assert!(
        run.has("a CustomItem is too large for the heap; it is left blank"),
        "{log}"
    );
    // The form's own buffer and the surface of the small item, at least 80
    // by 100 pixels, are counted.
    let form = run.wait_log("custom form counts ");
    let counted = numbers(&form, "custom form counts ")[0];
    assert!(counted >= SCREEN_BYTES + 80 * 100 * 4, "{log}");
    // Ten forms are shown in turn and kept: only the one on the screen
    // holds a buffer, so the heap of 1 MiB, which holds three, has room,
    // and the count is one buffer and the forms and their items, not ten
    // buffers.
    assert!(run.has("custom forms 10 shown"), "{log}");
    let forms = run.wait_log("custom forms count ");
    let counted = numbers(&forms, "custom forms count ")[0];
    assert!(
        (SCREEN_BYTES..2 * SCREEN_BYTES).contains(&counted),
        "{counted}; {log}"
    );
}

#[test]
fn siemens_bitmaps_and_tiled_layers_the_heap_cannot_hold_are_refused_before_they_are_made() {
    let alone = Alone::new();
    let Some(jar) = suite("siemens-bitmaps", &[]) else {
        return;
    };
    // The packed bitmap is 1 MiB, so the heap is 2 MiB.
    let mut run = siemens(&jar, midlet::SIEMENS_BITMAPS, 2 * HEAP);
    run.wait_log("done");
    let log = run.dump();
    // 32 MiB of pixels and 64 MiB of cells.
    assert!(run.has("siemens bitmap OutOfMemoryError"), "{log}");
    assert!(run.has("siemens layer OutOfMemoryError"), "{log}");
    assert!(alone.peak_mib() < LIMIT_MIB, "{} MiB", alone.peak_mib());
}

#[test]
fn siemens_layers_count_their_animated_tiles() {
    let _alone = Alone::new();
    let Some(jar) = suite("siemens-layers", &[]) else {
        return;
    };
    // Each layer's 32767 animated tiles are 131 KB the heap is told about.
    let mut run = siemens(&jar, midlet::SIEMENS_LAYERS, HEAP);
    let kept = run.wait_log("siemens layers ");
    let log = run.dump();
    assert!(
        run.has("siemens animated 32767 then IllegalArgumentException"),
        "{log}"
    );
    assert!(kept.ends_with("then OutOfMemoryError"), "{log}");
    let count = numbers(&kept, "siemens layers ")[0];
    assert!((1..12).contains(&count), "{log}");
}

#[test]
fn siemens_sprites_count_the_frame_sequences_they_keep() {
    let _alone = Alone::new();
    let Some(jar) = suite("siemens-sprites", &[]) else {
        return;
    };
    let mut run = siemens(&jar, midlet::SIEMENS_SPRITES, HEAP);
    // The array is 400 KB and each sprite keeps another copy.
    let sprites = run.wait_log("siemens sprites ");
    let log = run.dump();
    assert!(sprites.ends_with("kept then OutOfMemoryError"), "{log}");
    let count = numbers(&sprites, "siemens sprites ")[0];
    assert!((1..10).contains(&count), "{log}");
    // A copy of a sprite keeps a sequence of its own, 80 KB.
    let copies = run.wait_log("siemens copies ");
    assert!(copies.ends_with("kept then OutOfMemoryError"), "{log}");
    let count = numbers(&copies, "siemens copies ")[0];
    assert!((1..20).contains(&count), "{log}");
    run.wait_log("done");
}

/// The colour of the focused row of a list, and a column it crosses.
const HIGHLIGHT: u32 = 0x1F_5FAF;
const ROW_COLUMN: u32 = 230;

/// The first row of `frame` where the focused row of a list shows.
fn focused_row(frame: &Frame) -> Option<u32> {
    (0..frame.height()).find(|&y| rgb(frame, ROW_COLUMN, y) == HIGHLIGHT)
}

fn same_pixels(a: &Frame, b: &Frame) -> bool {
    a.width() == b.width()
        && a.height() == b.height()
        && (0..a.height()).all(|y| (0..a.width()).all(|x| rgb(a, x, y) == rgb(b, x, y)))
}

#[test]
fn a_screen_that_was_hidden_paints_all_of_its_content_when_it_is_shown_again() {
    let _alone = Alone::new();
    let Some(jar) = suite("returns", &[]) else {
        return;
    };
    let mut run = launch(&jar, midlet::RETURNS, HEAP);
    run.wait_log("menu shown");
    let first = run.wait_frame(|f| focused_row(f).is_some());
    let row = focused_row(&first).expect("a focused row");
    // The alert covers the menu, which gives its buffer back.
    run.tap(Key::Fire);
    run.wait_log("selected");
    let alert = run.wait_frame(|f| f.serial() > first.serial() && focused_row(f).is_none());
    // Showing the menu again paints it as it was.
    run.tap(Key::Fire);
    run.wait_log("dismissed");
    let again = run.wait_frame(|f| f.serial() > alert.serial() && focused_row(f).is_some());
    let log = run.dump();
    assert_eq!(focused_row(&again), Some(row), "{log}");
    assert!(
        same_pixels(&first, &again),
        "the menu looks different; {log}"
    );
}
