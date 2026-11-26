//! The memory the host holds while a class loads, before the class memory
//! limit has judged the class. A class that does not fit must be cheap to
//! refuse, and a small class file must not make the host build gigabytes
//! on the way to a class that does. Each case measures the most heap the
//! host held above what it held before the load, with an allocator that
//! counts.

#![allow(
    unsafe_code,
    reason = "an allocator that counts has to implement GlobalAlloc, which is unsafe"
)]

mod support;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError, mpsc};
use std::time::Duration;

use fastemu_jvm::{MemorySource, Throw, Vm, VmConfig, VmLimits};
use support::{ClassBuilder, PUBLIC, STATIC};

/// The bytes the program holds now, and the most it held since the last
/// case began.
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

/// The system's allocator, with a count of the bytes it holds.
struct Counting;

// SAFETY: each call is passed on to `System` as it came, and the counters
// are atomics, so the contract of `GlobalAlloc` is `System`'s.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller keeps the contract of `GlobalAlloc::alloc`.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        // SAFETY: the caller keeps the contract of `GlobalAlloc::dealloc`.
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The cases share the counters, so they take turns. A case makes its
/// class files in its turn too: made while another case measured, they
/// would count in that case's peak.
static TURN: Mutex<()> = Mutex::new(());

/// How long a case may take. They take well under a second.
const DEADLINE: Duration = Duration::from_secs(60);

/// What a case may hold above where it began, with its class files made:
/// what the faulty code held was a few times this or more.
const ROOM: usize = 4 << 20;

/// The class files of a case, by internal name.
type Classes = Vec<(String, Vec<u8>)>;

/// Loads the class `name` of the class files `classes` makes on a machine
/// of its own and returns the most heap the host held above what it held
/// before the load, in bytes, or the error the load threw.
fn peak_of_load(classes: impl FnOnce() -> Classes, name: &'static str) -> Result<usize, String> {
    let (peak, outcome) = peak_and_outcome(VmConfig::default(), classes, name);
    outcome.map(|()| peak)
}

/// Loads the class `name` of the class files `classes` makes on a machine
/// of `config` and returns the most heap the host held above what it held
/// before the load, in bytes, whether the load threw or not, and what it
/// threw.
fn peak_and_outcome(
    config: VmConfig,
    classes: impl FnOnce() -> Classes,
    name: &'static str,
) -> (usize, Result<(), String>) {
    let _turn = TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let classes = classes();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut source = MemorySource::new();
        for (name, bytes) in classes {
            source.add_class(name, bytes);
        }
        let vm = Vm::new(config, std::sync::Arc::new(source), Vec::new());
        let (result_tx, result_rx) = mpsc::channel();
        vm.spawn("probe", move |ctx| {
            let before = LIVE.load(Ordering::Relaxed);
            PEAK.store(before, Ordering::Relaxed);
            let loaded = ctx.find_class(name);
            let peak = PEAK.load(Ordering::Relaxed).saturating_sub(before);
            let outcome = match loaded {
                Ok(_) => Ok(()),
                Err(Throw::Exception(e)) => Err(ctx.describe_exception(e)),
                Err(Throw::Stop) => Err("stopped".to_owned()),
            };
            let _ = result_tx.send((peak, outcome));
            Ok(())
        })
        .expect("a thread");
        let result = result_rx.recv_timeout(DEADLINE);
        if result.is_ok() {
            vm.stop();
        }
        let _ = tx.send(result.ok());
    });
    rx.recv_timeout(DEADLINE + Duration::from_secs(5))
        .ok()
        .flatten()
        .expect("the case finished in time")
}

#[test]
fn methods_that_share_a_long_descriptor_do_not_each_hold_a_parse_of_it() {
    // A descriptor of 8000 parameters is 8 KB of file, and parsing it makes
    // a `String` for each parameter. The methods of a class file can all
    // name the same descriptor, so a parse kept for each of them is 8000
    // strings times the methods, and a class has up to 65535 methods.
    let classes = || {
        let desc = format!("({})V", "I".repeat(8000));
        let mut c = ClassBuilder::new("Wide", "java/lang/Object", 49, 0);
        for _ in 0..300 {
            c.method(PUBLIC | STATIC, "m", &desc, 0, 1, &[0xb1], &[]);
        }
        vec![("Wide".to_owned(), c.build())]
    };
    let peak = peak_of_load(classes, "Wide").expect("the class loads");
    assert!(peak < ROOM, "the load held {peak} bytes at its peak");
}

/// The interface `name`, which extends all of `parents`.
fn interface(name: &str, parents: &[String]) -> Vec<u8> {
    let mut c = ClassBuilder::new(name, "java/lang/Object", 49, 0);
    c.access(0x0601);
    let parents: Vec<[u8; 2]> = parents.iter().map(|p| c.class(p).to_be_bytes()).collect();
    let mut bytes = c.build();
    // With no fields, methods or attributes, the class file ends in four
    // counts of 2 bytes: the interfaces, then the three others.
    let at = bytes.len() - 8;
    let list = (parents.len() as u16).to_be_bytes().into_iter();
    bytes.splice(at..at + 2, list.chain(parents.into_iter().flatten()));
    bytes
}

#[test]
fn a_class_that_names_an_interface_over_and_over_does_not_copy_its_ancestors_each_time() {
    // `J` has 1500 ancestors. A class that names it 5000 times would add
    // the 1500 to its own list 5000 times, 30 MB, before the list was
    // cleaned: with 65535 mentions and 10000 ancestors it is 2.6 GB.
    let classes = || {
        let ancestors: Vec<String> = (0..1500).map(|i| format!("I{i}")).collect();
        let mut classes: Classes = ancestors
            .iter()
            .map(|name| (name.clone(), interface(name, &[])))
            .collect();
        classes.push(("J".to_owned(), interface("J", &ancestors)));
        classes.push(("C".to_owned(), {
            let mut c = ClassBuilder::new("C", "java/lang/Object", 49, 0);
            let j = c.class("J").to_be_bytes();
            let mut bytes = c.build();
            let at = bytes.len() - 8;
            let list = 5000u16.to_be_bytes().into_iter();
            bytes.splice(at..at + 2, list.chain(j.into_iter().cycle().take(2 * 5000)));
            bytes
        }));
        classes
    };
    let peak = peak_of_load(classes, "C").expect("the class loads");
    assert!(peak < ROOM, "the load held {peak} bytes at its peak");
}

/// A machine that holds `max_class_bytes` of class data.
fn limited(max_class_bytes: usize) -> VmConfig {
    VmConfig {
        limits: VmLimits {
            max_class_bytes,
            ..VmLimits::default()
        },
        ..VmConfig::default()
    }
}

const LIMIT_REACHED: &str = "java.lang.OutOfMemoryError: class memory limit reached";

/// `C0` to `C<len - 1>`, each extending the one before, each with a method
/// of 60000 bytes of code. The class memory counts such a class at about
/// 1.2 MiB, for the decoded code it will have; parsed, it holds 60 KB.
fn code_chain(len: usize) -> Classes {
    let mut code = vec![0x00; 59_999];
    code.push(0xb1);
    (0..len)
        .map(|i| {
            let name = format!("C{i}");
            let parent = match i {
                0 => "java/lang/Object".to_owned(),
                _ => format!("C{}", i - 1),
            };
            let mut c = ClassBuilder::new(&name, &parent, 49, 0);
            c.method(PUBLIC | STATIC, "m", "()V", 0, 0, &code, &[]);
            (name, c.build())
        })
        .collect()
}

#[test]
fn a_chain_of_classes_that_wait_for_their_superclasses_is_held_to_the_class_memory() {
    // Loaded from its leaf, each class waits, parsed, while the one above
    // it loads. Charged only once linked, all 250 were held, 15 MB, before
    // the first was judged, whatever the limit.
    let (peak, outcome) = peak_and_outcome(limited(2 << 20), || code_chain(250), "C249");
    assert_eq!(outcome, Err(LIMIT_REACHED.to_owned()));
    assert!(peak < ROOM, "the load held {peak} bytes at its peak");
}

/// `I` and `D0` to `D<len - 1>`, each extending the one before and naming
/// `I` `mentions` times.
fn named_chain(len: usize, mentions: u16) -> Classes {
    let mut classes = vec![("I".to_owned(), interface("I", &[]))];
    for i in 0..len {
        let name = format!("D{i}");
        let parent = match i {
            0 => "java/lang/Object".to_owned(),
            _ => format!("D{}", i - 1),
        };
        let mut c = ClassBuilder::new(&name, &parent, 49, 0);
        let named = c.class("I").to_be_bytes();
        let mut bytes = c.build();
        // With no fields, methods or attributes, the class file ends in
        // four counts of 2 bytes: the interfaces, then the three others.
        let at = bytes.len() - 8;
        let list = mentions.to_be_bytes().into_iter();
        let names = named.into_iter().cycle().take(2 * usize::from(mentions));
        bytes.splice(at..at + 2, list.chain(names));
        classes.push((name, bytes));
    }
    classes
}

#[test]
fn a_chain_of_classes_that_name_an_interface_over_and_over_is_held_to_the_class_memory() {
    // A class that waits holds a name for each time it names `I`, 128 KB
    // for 8192 times, which links to a list of one interface. Counted at
    // the 4 bytes of the list, 63 such classes waited, 8 MB, under a
    // limit of 2 MiB; before they were charged at all, 250 waited, 33 MB.
    let classes = || named_chain(250, 8192);
    let (peak, outcome) = peak_and_outcome(limited(2 << 20), classes, "D249");
    assert_eq!(outcome, Err(LIMIT_REACHED.to_owned()));
    assert!(peak < ROOM, "the load held {peak} bytes at its peak");
}
