//! A suite whose classes form a long chain ends as a crash with a short
//! message. It does not close the app: a stack overflow on the virtual
//! machine's thread would take the whole process down, and every other game
//! with it. A manifest that names a deep array class, or a class of 40000
//! bytes, lists no MIDlet, so the name never reaches the loader.
//!
//! The JARs are made here, so the tests need no JDK.

mod support;

use std::path::{Path, PathBuf};

use fastemu_core::InstanceStatus;
use fastemu_midp::suite::Suite;
use support::{Running, scratch, zip};

/// A class file of class `name` that extends `super_name` and has no
/// members.
fn class_file(name: &str, super_name: &str) -> Vec<u8> {
    let mut b = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 49];
    // Pool: 1 and 3 are names, 2 and 4 the classes that name them.
    b.extend(5u16.to_be_bytes());
    for (i, name) in [name, super_name].into_iter().enumerate() {
        b.push(1);
        b.extend((name.len() as u16).to_be_bytes());
        b.extend(name.as_bytes());
        b.push(7);
        b.extend(((2 * i + 1) as u16).to_be_bytes());
    }
    b.extend(0x0021u16.to_be_bytes());
    b.extend(2u16.to_be_bytes());
    b.extend(4u16.to_be_bytes());
    // No interfaces, fields, methods or attributes.
    b.extend([0; 8]);
    b
}

/// The JAR of a suite whose only MIDlet is class `midlet`, with `classes`.
fn suite(folder: &str, midlet: &str, classes: &[(String, Vec<u8>)]) -> PathBuf {
    let manifest = format!(
        "Manifest-Version: 1.0\r\nMIDlet-Name: Hostile\r\nMIDlet-Vendor: FastEmu\r\n\
         MIDlet-Version: 1.0\r\nMIDlet-1: Hostile, , {midlet}\r\n"
    );
    let mut entries = vec![("META-INF/MANIFEST.MF".to_owned(), manifest.into_bytes())];
    entries.extend(classes.iter().cloned());
    let path = scratch(folder).join("hostile.jar");
    std::fs::write(&path, zip(&entries, true)).expect("writing the JAR");
    path
}

/// The message a launch of `jar` crashes with.
fn crash_message(jar: &Path) -> String {
    let mut run = Running::launch(jar, 0);
    match run.wait_status(InstanceStatus::is_finished) {
        InstanceStatus::Crashed { message } => message,
        other => panic!("expected a crash, got {other:?}; the log:\n{}", run.dump()),
    }
}

/// `C0` to `C<len - 1>`, each extending the one before; `C0` extends
/// `MIDlet`.
fn chain(len: usize) -> Vec<(String, Vec<u8>)> {
    (0..len)
        .map(|i| {
            let super_name = match i {
                0 => "javax/microedition/midlet/MIDlet".to_owned(),
                _ => format!("C{}", i - 1),
            };
            (
                format!("C{i}.class"),
                class_file(&format!("C{i}"), &super_name),
            )
        })
        .collect()
}

/// How many MIDlets the suite in `jar` lists.
fn listed(jar: &Path) -> usize {
    Suite::open(jar)
        .expect("a suite that opens")
        .info
        .midlets
        .len()
}

#[test]
fn a_manifest_naming_a_very_deep_array_lists_no_midlet() {
    let name = format!("{}I", "[".repeat(40_000));
    let jar = suite("deep_array", &name, &[]);
    assert_eq!(listed(&jar), 0);
}

#[test]
fn a_manifest_naming_a_very_long_class_lists_no_midlet() {
    let jar = suite("long_name", &"a.".repeat(20_000), &[]);
    assert_eq!(listed(&jar), 0);
}

#[test]
fn a_manifest_naming_a_class_of_255_bytes_still_reaches_the_loader() {
    // The longest name a MIDlet entry may have is the longest the loader
    // takes, so a class that is not in the JAR ends the launch in the
    // error for a missing class.
    let name = format!("{}.G", "a".repeat(253));
    assert_eq!(name.len(), 255);
    let jar = suite("name_255", &name, &[]);
    assert_eq!(listed(&jar), 1);
    let message = crash_message(&jar);
    assert!(
        message.contains("java.lang.NoClassDefFoundError"),
        "{message}"
    );
    assert!(message.len() < 400, "{} bytes of message", message.len());
}

#[test]
fn a_chain_of_classes_past_the_limit_ends_as_a_crash() {
    let jar = suite("long_chain", "C299", &chain(300));
    let message = crash_message(&jar);
    assert!(
        message.contains("java.lang.ClassFormatError")
            && message.contains("the class hierarchy is over 256 classes deep"),
        "{message}"
    );
    assert!(message.len() < 400, "{} bytes of message", message.len());
}

#[test]
fn a_chain_within_the_limit_gets_as_far_as_the_constructor() {
    // The classes have no constructor, so the launch fails there, past the
    // loading the limits guard.
    let jar = suite("short_chain", "C199", &chain(200));
    let message = crash_message(&jar);
    assert!(message.contains("constructor"), "{message}");
    assert!(!message.contains("hierarchy"), "{message}");
}
