//! Checks the class library against the API inventory in
//! `docs/compat/api`: every class, field, constant, constructor and method
//! the CLDC 1.1 files list must exist with the listed descriptor, the
//! right `static` and `abstract` flags and the listed constant value.
//!
//! The inventory is the contract a game's bytecode links against, so a
//! member missing here is a `NoSuchMethodError` in some game.

use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

use crate::class::{ClassId, ConstValue};
use crate::classfile::access;
use crate::value::Value;
use crate::vm::VmState;
use crate::{MemorySource, Vm, VmConfig};

/// The inventory files this crate implements in full.
const FILES: [&str; 3] = ["cldc-java-lang.md", "cldc-java-util.md", "cldc-java-io.md"];

/// One class section of an inventory file.
#[derive(Default)]
struct ClassEntry {
    /// The internal name, such as `java/util/Vector`.
    name: String,
    interface: bool,
    is_abstract: bool,
    /// The superclass and interfaces the declaration names.
    supers: Vec<String>,
    members: Vec<Member>,
}

/// One row of a member table.
struct Member {
    kind: MemberKind,
    name: String,
    desc: String,
    flags: String,
    value: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MemberKind {
    Field,
    Constructor,
    Method,
}

fn internal(binary: &str) -> String {
    binary.replace('.', "/")
}

/// The text between the first pair of backticks in `cell`.
fn code(cell: &str) -> &str {
    cell.split('`').nth(1).unwrap_or("")
}

fn parse_declaration(line: &str, entry: &mut ClassEntry) {
    let decl = code(line);
    let words: Vec<&str> = decl.split_whitespace().collect();
    entry.interface = words.contains(&"interface");
    entry.is_abstract = words.contains(&"abstract") || entry.interface;
    let mut iter = words
        .iter()
        .skip_while(|w| !matches!(**w, "class" | "interface"));
    iter.next();
    entry.name = internal(iter.next().expect("a declaration names its class"));
    for w in iter {
        if !matches!(*w, "extends" | "implements") {
            entry.supers.push(internal(w.trim_end_matches(',')));
        }
    }
}

/// Parses every class section of one inventory file.
fn parse(text: &str) -> Vec<ClassEntry> {
    let mut classes: Vec<ClassEntry> = Vec::new();
    let mut header: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.starts_with("`public ") {
            let mut entry = ClassEntry::default();
            parse_declaration(line, &mut entry);
            classes.push(entry);
            header.clear();
            continue;
        }
        if !line.starts_with('|') || classes.is_empty() {
            if !line.starts_with('|') {
                header.clear();
            }
            continue;
        }
        let cells: Vec<String> = line
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim().to_owned())
            .collect();
        if cells.first().is_some_and(|c| c == "T") {
            header = cells;
            continue;
        }
        if cells.iter().all(|c| c.chars().all(|ch| ch == '-')) || header.len() < 3 {
            continue;
        }
        let column = |name: &str| {
            header
                .iter()
                .position(|h| h == name)
                .map(|i| cells.get(i).cloned().unwrap_or_default())
        };
        let flags = column("Flags").unwrap_or_default();
        let member = match header[1].as_str() {
            "Field" => Member {
                kind: MemberKind::Field,
                name: code(&cells[1]).to_owned(),
                desc: code(&column("Type").unwrap_or_default()).to_owned(),
                flags,
                value: column("Value").unwrap_or_default(),
            },
            kind @ ("Constructor" | "Method") => {
                let descriptor = column("Descriptor").unwrap_or_default();
                let descriptor = code(&descriptor);
                let open = descriptor.find('(').expect("a descriptor has parameters");
                Member {
                    kind: if kind == "Constructor" {
                        MemberKind::Constructor
                    } else {
                        MemberKind::Method
                    },
                    name: descriptor[..open].to_owned(),
                    desc: descriptor[open..].to_owned(),
                    flags,
                    value: String::new(),
                }
            }
            _ => continue,
        };
        classes
            .last_mut()
            .expect("checked above")
            .members
            .push(member);
    }
    classes
}

/// The constant the inventory gives for a field of type `desc`, if it
/// lists one this test can compare.
fn expected_constant(desc: &str, value: &str) -> Option<Value> {
    if value.starts_with("(not a constant)") || value.is_empty() {
        return None;
    }
    let first = code(value);
    let bits = value
        .split("bits `")
        .nth(1)
        .and_then(|rest| rest.split('`').next())
        .and_then(|hex| u64::from_str_radix(hex.trim_start_matches("0x"), 16).ok());
    match desc {
        "I" | "S" | "B" | "Z" => first.parse().ok().map(Value::Int),
        "J" => first.trim_end_matches('L').parse().ok().map(Value::Long),
        "C" => value
            .split('(')
            .nth(1)
            .and_then(|rest| rest.split(')').next())
            .and_then(|n| n.parse().ok())
            .map(Value::Int),
        "F" => bits.map(|b| Value::Float(f32::from_bits(b as u32))),
        "D" => bits.map(|b| Value::Double(f64::from_bits(b))),
        _ => None,
    }
}

fn same_constant(a: Value, b: Value) -> bool {
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        (Value::Double(x), Value::Double(y)) => x.to_bits() == y.to_bits(),
        _ => a == b,
    }
}

fn check_member(state: &VmState, class: ClassId, entry: &ClassEntry, m: &Member, out: &mut String) {
    let classes = &state.classes;
    let is_static = m.flags.split_whitespace().any(|f| f == "static");
    // Interface methods are abstract whether or not the table says so.
    let is_abstract = entry.interface || m.flags.split_whitespace().any(|f| f == "abstract");
    let what = format!("{}.{}{}", entry.name, m.name, m.desc);
    match m.kind {
        MemberKind::Field => {
            let Some((_, field)) = classes.resolve_field(class, &m.name, &m.desc) else {
                let _ = writeln!(out, "missing field {}.{}:{}", entry.name, m.name, m.desc);
                return;
            };
            if field.is_static() != is_static {
                let _ = writeln!(out, "field {what}: static is {}", field.is_static());
            }
            if let Some(expected) = expected_constant(&m.desc, &m.value) {
                match &field.constant {
                    Some(ConstValue::Value(v)) if same_constant(*v, expected) => {}
                    other => {
                        let _ = writeln!(
                            out,
                            "field {what}: constant {other:?}, expected {expected:?}"
                        );
                    }
                }
            }
        }
        MemberKind::Constructor => {
            if classes
                .get(class)
                .declared_method("<init>", &m.desc)
                .is_none()
            {
                let _ = writeln!(out, "missing constructor {what}");
            }
        }
        MemberKind::Method => {
            let found = if entry.interface {
                classes.resolve_interface_method(class, &m.name, &m.desc)
            } else {
                classes.resolve_method(class, &m.name, &m.desc)
            };
            let Some(method) = found else {
                let _ = writeln!(out, "missing method {what}");
                return;
            };
            if method.is_static() != is_static {
                let _ = writeln!(out, "method {what}: static is {}", method.is_static());
            }
            if method.is_abstract() != is_abstract {
                let _ = writeln!(out, "method {what}: abstract is {}", method.is_abstract());
            }
        }
    }
}

fn check_class(state: &mut VmState, entry: &ClassEntry, out: &mut String) {
    let class = match state.classes.load(&entry.name) {
        Ok(c) => c,
        Err(e) => {
            let _ = writeln!(out, "missing class {}: {}", entry.name, e.message);
            return;
        }
    };
    let c = Arc::clone(state.classes.get(class));
    if c.is_interface() != entry.interface {
        let _ = writeln!(
            out,
            "class {}: interface is {}",
            entry.name,
            c.is_interface()
        );
    }
    if c.is_abstract() != entry.is_abstract {
        let _ = writeln!(out, "class {}: abstract is {}", entry.name, c.is_abstract());
    }
    if c.access & access::FINAL != 0 && entry.is_abstract {
        let _ = writeln!(out, "class {}: final and abstract", entry.name);
    }
    for sup in &entry.supers {
        match state.classes.load(sup) {
            Ok(s) if state.classes.is_assignable(class, s) => {}
            _ => {
                let _ = writeln!(out, "class {}: not a {sup}", entry.name);
            }
        }
    }
    for m in &entry.members {
        check_member(state, class, entry, m, out);
    }
}

/// Every problem found, one per line.
fn problems() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/compat/api");
    let vm = Vm::new(
        VmConfig::default(),
        Arc::new(MemorySource::new()),
        Vec::new(),
    );
    let index = std::fs::read_to_string(dir.join("README.md")).expect("reading README.md");
    let mut out = String::new();
    vm.attach("inventory", |ctx| {
        for file in FILES {
            let text = std::fs::read_to_string(dir.join(file))
                .unwrap_or_else(|e| panic!("reading {file}: {e}"));
            let classes = parse(&text);
            let members: usize = classes.iter().map(|c| c.members.len()).sum();
            assert_eq!(
                (classes.len(), members),
                indexed_counts(&index, file),
                "{file}: the parser saw other counts than the index gives"
            );
            for entry in &classes {
                check_class(&mut ctx.state, entry, &mut out);
            }
        }
    });
    out
}

/// The class and member counts the README's index gives for `file`, so
/// the test knows its parser read every row.
fn indexed_counts(index: &str, file: &str) -> (usize, usize) {
    let row = index
        .lines()
        .find(|l| l.starts_with(&format!("| [{file}]")))
        .unwrap_or_else(|| panic!("README.md has no row for {file}"));
    let numbers: Vec<usize> = row
        .split('|')
        .filter_map(|cell| cell.trim().parse().ok())
        .collect();
    match numbers[..] {
        [classes, members] => (classes, members),
        _ => panic!("README.md row for {file} has no counts: {row}"),
    }
}

#[test]
fn every_inventory_member_links() {
    let problems = problems();
    assert!(problems.is_empty(), "the class library lacks:\n{problems}");
}

/// Rule 17 of `docs/compat/emulator-notes.md`: CLDC's `Thread` has no
/// `stop`, `suspend` or `resume`, so a game's `Thread` subclass may
/// declare its own with any signature and still link.
#[test]
fn thread_has_no_stop_suspend_or_resume() {
    let vm = Vm::new(
        VmConfig::default(),
        Arc::new(MemorySource::new()),
        Vec::new(),
    );
    vm.attach("inventory", |ctx| {
        let thread = ctx.find_class("java/lang/Thread").unwrap();
        let c = ctx.state.classes.get(thread);
        for m in &c.methods {
            assert!(
                !matches!(&*m.name, "stop" | "suspend" | "resume" | "destroy"),
                "Thread declares {}{}",
                m.name,
                m.desc
            );
        }
    });
}
