//! What the host holds while a game reads a resource of its JAR. A small
//! JAR can hold a resource that inflates a thousand times, and the host
//! must not build the whole of it before the heap judges it: the read
//! stops at what the heap could hold, and the threads of a game read one
//! resource at a time. Each case measures the most the process held above
//! where it began, with an allocator that counts.

#![allow(
    unsafe_code,
    reason = "an allocator that counts has to implement GlobalAlloc, which is unsafe"
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Duration;

use fastemu_jvm::{Throw, Vm, VmConfig};
use fastemu_midp::suite::{JarSource, ZipArchive};

/// The bytes the process holds now, and the most it held since a case
/// began measuring.
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

/// The system's allocator, counting what it hands out. A zeroed block
/// counts at its size whether or not anything touches it.
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

/// The cases share the counters, so they take turns.
static TURN: Mutex<()> = Mutex::new(());

/// The heap of the machines the cases run: 2 MiB, as on the smallest
/// profiles.
const HEAP: usize = 2 << 20;

/// What a case may hold above where it began. A read that stops at the
/// heap holds about 2 MiB; one that builds the resource holds the 24 MiB
/// below, and eight of them at once 192.
const ROOM: usize = 8 << 20;

/// How long a case may take. They take well under a second.
const DEADLINE: Duration = Duration::from_secs(60);

/// A ZIP of one deflated file `name` holding `data`.
fn jar_of(name: &str, data: &[u8]) -> Vec<u8> {
    let body = miniz_oxide::deflate::compress_to_vec(data, 6);
    let crc = crc32fast::hash(data);
    let mut header = Vec::new();
    header.extend_from_slice(&20u16.to_le_bytes());
    header.extend_from_slice(&0u16.to_le_bytes());
    header.extend_from_slice(&8u16.to_le_bytes());
    header.extend_from_slice(&[0; 4]);
    header.extend_from_slice(&crc.to_le_bytes());
    header.extend_from_slice(&(body.len() as u32).to_le_bytes());
    header.extend_from_slice(&(data.len() as u32).to_le_bytes());
    header.extend_from_slice(&(name.len() as u16).to_le_bytes());
    header.extend_from_slice(&0u16.to_le_bytes());
    let mut out = Vec::new();
    out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
    out.extend_from_slice(&header);
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(&body);
    let start = out.len() as u32;
    let mut directory = Vec::new();
    directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
    directory.extend_from_slice(&20u16.to_le_bytes());
    directory.extend_from_slice(&header);
    directory.extend_from_slice(&[0; 10]);
    directory.extend_from_slice(&0u32.to_le_bytes());
    directory.extend_from_slice(name.as_bytes());
    out.extend_from_slice(&directory);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// A machine of a 2 MiB heap over a JAR of the resource `/bomb.bin`, which
/// is `len` bytes of zeros.
fn machine(len: usize) -> Vm {
    let archive = ZipArchive::new(jar_of("bomb.bin", &vec![0u8; len])).expect("a JAR");
    let source = JarSource::new(Arc::new(archive), None);
    let config = VmConfig {
        heap_bytes: HEAP,
        ..VmConfig::default()
    };
    Vm::new(config, Arc::new(source), Vec::new())
}

/// What `Class.getResourceAsStream("/bomb.bin")` gives on a thread of the
/// machine: `ok`, or the exception.
fn open_bomb(ctx: &mut fastemu_jvm::Ctx<'_>) -> String {
    let class = ctx.find_class("java/lang/Object").expect("a class");
    let mirror = ctx.class_object(class);
    let name = ctx.new_string("/bomb.bin").expect("a string");
    let opened = ctx.call_virtual(
        mirror,
        "getResourceAsStream",
        "(Ljava/lang/String;)Ljava/io/InputStream;",
        &[name.into()],
    );
    match opened {
        Ok(_) => "ok".to_owned(),
        Err(Throw::Exception(e)) => ctx.describe_exception(e),
        Err(Throw::Stop) => "stopped".to_owned(),
    }
}

/// The outcome of `threads` reads of the bomb that run at once, and the
/// most the process held above what it held when they began.
fn read_bomb(len: usize, threads: usize) -> (Vec<String>, usize) {
    let _turn = TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let vm = machine(len);
    // One read first, so the classes it needs are loaded and the heap the
    // machine made is counted before the measure begins.
    let (tx, rx) = mpsc::channel();
    let first = tx.clone();
    vm.spawn("warm", move |ctx| {
        let _ = first.send(open_bomb(ctx));
        Ok(())
    })
    .expect("a thread");
    rx.recv_timeout(DEADLINE).expect("the first read finished");
    let start = LIVE.load(Relaxed);
    PEAK.store(start, Relaxed);
    for i in 0..threads {
        let tx = tx.clone();
        vm.spawn(&format!("probe{i}"), move |ctx| {
            let _ = tx.send(open_bomb(ctx));
            Ok(())
        })
        .expect("a thread");
    }
    let outcomes: Vec<String> = (0..threads)
        .map(|_| rx.recv_timeout(DEADLINE).expect("a read finished"))
        .collect();
    let peak = PEAK.load(Relaxed).saturating_sub(start);
    vm.stop();
    (outcomes, peak)
}

#[test]
fn a_resource_that_inflates_to_24_mib_costs_the_heap_and_not_its_size() {
    let (outcomes, peak) = read_bomb(24 << 20, 1);
    assert_eq!(outcomes, ["java.lang.OutOfMemoryError: Java heap space"]);
    assert!(peak < ROOM, "the host held {peak} bytes at its peak");
}

#[test]
fn eight_threads_reading_one_bomb_do_not_hold_eight_buffers() {
    let (outcomes, peak) = read_bomb(24 << 20, 8);
    assert_eq!(outcomes.len(), 8);
    assert!(
        outcomes
            .iter()
            .all(|o| o == "java.lang.OutOfMemoryError: Java heap space"),
        "{outcomes:?}"
    );
    assert!(peak < ROOM, "the host held {peak} bytes at its peak");
}

#[test]
fn a_resource_that_fits_the_heap_is_read_whole() {
    // 600 KB of zeros in a 2 MiB heap: the byte array takes 600 KB.
    let (outcomes, peak) = read_bomb(600 << 10, 1);
    assert_eq!(outcomes, ["ok"]);
    assert!(peak < ROOM, "the host held {peak} bytes at its peak");
}
