//! Rust data attached to Java objects.

use std::any::Any;

use crate::value::{ObjRef, Value};

/// Data a class implemented in Rust keeps beside an object's Java fields:
/// a string's characters, a string buffer's contents, a thread's state, a
/// mutable image's pixels.
///
/// A payload that holds heap references must report each of them from
/// [`Payload::trace`], or the collector frees the objects they point to.
/// Payloads move between the threads of one virtual machine, so they are
/// `Send`.
pub trait Payload: Any + Send {
    /// Marks every heap reference the payload holds.
    fn trace(&self, tracer: &mut Tracer<'_>) {
        let _ = tracer;
    }

    /// Bytes the payload counts against the heap limit, beyond the object
    /// itself. A payload that grows is measured again at each collection.
    fn heap_bytes(&self) -> usize {
        0
    }
}

/// Collects the references a payload reports while the collector marks.
pub struct Tracer<'a> {
    pub(super) marks: &'a mut [u64],
    pub(super) gray: &'a mut Vec<u32>,
}

impl Tracer<'_> {
    /// Keeps `r` and everything it reaches alive.
    pub fn mark(&mut self, r: ObjRef) {
        let index = r.index();
        let (word, bit) = (index / 64, 1u64 << (index % 64));
        if let Some(w) = self.marks.get_mut(word)
            && *w & bit == 0
        {
            *w |= bit;
            self.gray.push(index as u32);
        }
    }

    pub fn mark_opt(&mut self, r: Option<ObjRef>) {
        if let Some(r) = r {
            self.mark(r);
        }
    }

    /// Marks `v` when it is a non-null reference.
    pub fn mark_value(&mut self, v: Value) {
        if let Value::Ref(Some(r)) = v {
            self.mark(r);
        }
    }
}
