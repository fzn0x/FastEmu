//! A tiny class file assembler for tests that need exact bytecode: old
//! class versions, `jsr` and `ret`, and classes that extend natives.

#![allow(
    dead_code,
    reason = "each test file uses a different part of the assembler"
)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use fastemu_core::{LogLevel, LogLine};
use fastemu_jvm::{LogSink, MemorySource};

pub const PUBLIC: u16 = 0x0001;
pub const STATIC: u16 = 0x0008;
pub const SUPER: u16 = 0x0020;

/// Builds one class file.
pub struct ClassBuilder {
    major: u16,
    minor: u16,
    access: u16,
    this_class: u16,
    super_class: u16,
    pool: Vec<Vec<u8>>,
    pool_index: HashMap<Vec<u8>, u16>,
    fields: Vec<Vec<u8>>,
    methods: Vec<Vec<u8>>,
}

/// An exception table entry: start, end, handler (bytecode offsets) and
/// the class constant to catch, 0 for all.
pub type Handler = (u16, u16, u16, u16);

impl ClassBuilder {
    pub fn new(name: &str, super_name: &str, major: u16, minor: u16) -> Self {
        let mut b = Self {
            major,
            minor,
            access: PUBLIC | SUPER,
            this_class: 0,
            super_class: 0,
            pool: Vec::new(),
            pool_index: HashMap::new(),
            fields: Vec::new(),
            methods: Vec::new(),
        };
        b.this_class = b.class(name);
        b.super_class = b.class(super_name);
        b
    }

    /// Replaces the class access flags (`PUBLIC | SUPER` by default).
    pub fn access(&mut self, access: u16) -> &mut Self {
        self.access = access;
        self
    }

    fn entry(&mut self, bytes: Vec<u8>) -> u16 {
        if let Some(&i) = self.pool_index.get(&bytes) {
            return i;
        }
        self.pool.push(bytes.clone());
        let index = self.pool.len() as u16;
        self.pool_index.insert(bytes, index);
        index
    }

    pub fn utf8(&mut self, s: &str) -> u16 {
        let mut b = vec![1];
        b.extend((s.len() as u16).to_be_bytes());
        b.extend(s.as_bytes());
        self.entry(b)
    }

    pub fn class(&mut self, name: &str) -> u16 {
        let n = self.utf8(name);
        let mut b = vec![7];
        b.extend(n.to_be_bytes());
        self.entry(b)
    }

    pub fn string(&mut self, s: &str) -> u16 {
        let n = self.utf8(s);
        let mut b = vec![8];
        b.extend(n.to_be_bytes());
        self.entry(b)
    }

    pub fn integer(&mut self, v: i32) -> u16 {
        let mut b = vec![3];
        b.extend(v.to_be_bytes());
        self.entry(b)
    }

    fn member(&mut self, tag: u8, class: &str, name: &str, desc: &str) -> u16 {
        let c = self.class(class);
        let n = self.utf8(name);
        let d = self.utf8(desc);
        let mut nat = vec![12];
        nat.extend(n.to_be_bytes());
        nat.extend(d.to_be_bytes());
        let nat = self.entry(nat);
        let mut b = vec![tag];
        b.extend(c.to_be_bytes());
        b.extend(nat.to_be_bytes());
        self.entry(b)
    }

    pub fn field_ref(&mut self, class: &str, name: &str, desc: &str) -> u16 {
        self.member(9, class, name, desc)
    }

    pub fn method_ref(&mut self, class: &str, name: &str, desc: &str) -> u16 {
        self.member(10, class, name, desc)
    }

    pub fn interface_method_ref(&mut self, class: &str, name: &str, desc: &str) -> u16 {
        self.member(11, class, name, desc)
    }

    pub fn field(&mut self, access: u16, name: &str, desc: &str) -> &mut Self {
        let n = self.utf8(name);
        let d = self.utf8(desc);
        let mut f = Vec::new();
        f.extend(access.to_be_bytes());
        f.extend(n.to_be_bytes());
        f.extend(d.to_be_bytes());
        f.extend(0u16.to_be_bytes());
        self.fields.push(f);
        self
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the arguments are the fields of a method and its Code attribute"
    )]
    pub fn method(
        &mut self,
        access: u16,
        name: &str,
        desc: &str,
        max_stack: u16,
        max_locals: u16,
        code: &[u8],
        handlers: &[Handler],
    ) -> &mut Self {
        let n = self.utf8(name);
        let d = self.utf8(desc);
        let code_name = self.utf8("Code");
        let mut attr = Vec::new();
        attr.extend(max_stack.to_be_bytes());
        attr.extend(max_locals.to_be_bytes());
        attr.extend((code.len() as u32).to_be_bytes());
        attr.extend(code);
        attr.extend((handlers.len() as u16).to_be_bytes());
        for &(start, end, handler, catch) in handlers {
            for v in [start, end, handler, catch] {
                attr.extend(v.to_be_bytes());
            }
        }
        attr.extend(0u16.to_be_bytes());
        let mut m = Vec::new();
        m.extend(access.to_be_bytes());
        m.extend(n.to_be_bytes());
        m.extend(d.to_be_bytes());
        m.extend(1u16.to_be_bytes());
        m.extend(code_name.to_be_bytes());
        m.extend((attr.len() as u32).to_be_bytes());
        m.extend(attr);
        self.methods.push(m);
        self
    }

    /// A constructor that only calls the superclass's `<init>()V`.
    pub fn default_constructor(&mut self, super_name: &str) -> &mut Self {
        let init = self.method_ref(super_name, "<init>", "()V");
        let [hi, lo] = init.to_be_bytes();
        self.method(
            PUBLIC,
            "<init>",
            "()V",
            1,
            1,
            &[0x2a, 0xb7, hi, lo, 0xb1],
            &[],
        )
    }

    pub fn build(&self) -> Vec<u8> {
        let mut b = vec![0xCA, 0xFE, 0xBA, 0xBE];
        b.extend(self.minor.to_be_bytes());
        b.extend(self.major.to_be_bytes());
        b.extend((self.pool.len() as u16 + 1).to_be_bytes());
        for e in &self.pool {
            b.extend(e);
        }
        b.extend(self.access.to_be_bytes());
        b.extend(self.this_class.to_be_bytes());
        b.extend(self.super_class.to_be_bytes());
        b.extend(0u16.to_be_bytes());
        b.extend((self.fields.len() as u16).to_be_bytes());
        for f in &self.fields {
            b.extend(f);
        }
        b.extend((self.methods.len() as u16).to_be_bytes());
        for m in &self.methods {
            b.extend(m);
        }
        b.extend(0u16.to_be_bytes());
        b
    }
}

/// The two bytes of a constant pool index operand.
pub fn idx(i: u16) -> [u8; 2] {
    i.to_be_bytes()
}

/// A source holding the given classes.
pub fn source(classes: &[(&str, Vec<u8>)]) -> MemorySource {
    let mut s = MemorySource::new();
    for (name, bytes) in classes {
        s.add_class(*name, bytes.clone());
    }
    s
}

/// A log sink that keeps every line, and the lines it kept.
pub fn capture() -> (LogSink, Arc<Mutex<Vec<LogLine>>>) {
    let lines: Arc<Mutex<Vec<LogLine>>> = Arc::default();
    let sink = Arc::clone(&lines);
    (Arc::new(move |line| sink.lock().unwrap().push(line)), lines)
}

/// The text of the kept lines at `level`.
pub fn lines_at(lines: &Mutex<Vec<LogLine>>, level: LogLevel) -> Vec<String> {
    lines
        .lock()
        .unwrap()
        .iter()
        .filter(|l| l.level == level)
        .map(|l| l.text.clone())
        .collect()
}
