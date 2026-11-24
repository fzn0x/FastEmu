//! A Java virtual machine for Java ME games, with the CLDC 1.1 class library
//! implemented in Rust.
//!
//! A [`Vm`] runs one game: its classes come from a [`ClassSource`], its
//! threads are OS threads that take turns under one lock, and its heap is
//! collected precisely. Classes implemented in Rust ([`NativeClass`]) and
//! classes from bytecode share one class table, so game classes extend
//! and override native ones. Native code works through a [`Ctx`].
//!
//! `docs/architecture/jvm.md` describes the design and the rules native
//! code must follow.

mod charset;
mod class;
pub mod classfile;
mod ctx;
pub mod descriptor;
mod gc;
mod heap;
mod init;
mod interp;
#[cfg(test)]
mod inventory;
mod io;
mod lang;
mod monitor;
mod native;
mod source;
mod thread;
mod util;
mod value;
mod vm;

pub use class::{ArrayType, ClassId, PayloadFactory};
pub use ctx::{ArrayElem, Ctx, FieldId, GlobalRef, JResult, StaticFieldId, Throw, Wake};
pub use heap::{HeapStats, Payload, Tracer};
pub use native::{NativeClass, NativeFn};
pub use source::{ClassSource, DirSource, MemorySource};
pub use thread::ThreadId;
pub use value::{ObjRef, Value};
pub use vm::{Diagnostic, DiagnosticSink, LogSink, Task, Vm, VmConfig, WeakVm};

/// Every class of the CLDC 1.1 library: `java.lang`, `java.io` and
/// `java.util`.
fn library() -> Vec<NativeClass> {
    let mut all = lang::classes();
    all.extend(io::classes());
    all.extend(util::classes());
    all
}
