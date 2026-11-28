//! Every method of a machine, by id.
//!
//! Frames, resolved call sites and stack traces name a method by its
//! [`MethodId`], and the interpreter borrows the method from here through
//! the machine's shared part, which outlives every thread. So a call and
//! a return touch no reference count, and the interpreter can hold the
//! method of the frame it runs across calls that take the whole context.
//!
//! Methods are added once, when their class links, and never taken out:
//! classes are never unloaded. The arena holds them in fixed chunks, each
//! filled once, so a method stays where it is while more are added and a
//! reader needs no lock.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use super::Method;

/// Identifies a method within one virtual machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MethodId(u32);

const CHUNK_BITS: u32 = 10;
const CHUNK: usize = 1 << CHUNK_BITS;
const CHUNKS: usize = 1024;
/// The most methods one machine holds: far more than the class memory
/// limit lets a game define.
pub(crate) const MAX_METHODS: usize = CHUNK * CHUNKS;

type Chunk = Box<[OnceLock<Arc<Method>>]>;

pub(crate) struct MethodArena {
    chunks: Box<[OnceLock<Chunk>]>,
    /// How many ids were given out.
    len: AtomicU32,
}

impl MethodArena {
    pub(crate) fn new() -> Self {
        Self {
            chunks: (0..CHUNKS).map(|_| OnceLock::new()).collect(),
            len: AtomicU32::new(0),
        }
    }

    /// The method `id`. Ids come only from [`MethodArena::add`] of this
    /// arena, and methods are never taken out, so it is there.
    #[inline]
    pub(crate) fn get(&self, id: MethodId) -> &Arc<Method> {
        let i = id.0 as usize;
        self.chunks[i >> CHUNK_BITS]
            .get()
            .and_then(|chunk| chunk[i & (CHUNK - 1)].get())
            .expect("a method id names a method of its machine")
    }

    /// The method `id`, as [`MethodArena::get`] has it.
    #[inline]
    pub(crate) fn method(&self, id: MethodId) -> &Method {
        self.get(id)
    }

    /// Whether `n` more methods fit.
    pub(crate) fn has_room(&self, n: usize) -> bool {
        self.len.load(Ordering::Relaxed) as usize + n <= MAX_METHODS
    }

    /// Adds the method `make` builds with its id, and returns it.
    ///
    /// # Panics
    ///
    /// When the arena is full; callers check [`MethodArena::has_room`]
    /// first.
    pub(crate) fn add(&self, make: impl FnOnce(MethodId) -> Method) -> Arc<Method> {
        let i = self.len.fetch_add(1, Ordering::Relaxed) as usize;
        assert!(i < MAX_METHODS, "the method arena is full");
        let id = MethodId(i as u32);
        let method = Arc::new(make(id));
        let chunk = self.chunks[i >> CHUNK_BITS]
            .get_or_init(|| (0..CHUNK).map(|_| OnceLock::new()).collect());
        if chunk[i & (CHUNK - 1)].set(Arc::clone(&method)).is_err() {
            unreachable!("each id is given out once");
        }
        method
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn methods_keep_their_ids_across_chunks() {
        let arena = MethodArena::new();
        let mut added = Vec::new();
        for _ in 0..CHUNK + 5 {
            added.push(arena.add(Method::placeholder));
        }
        for m in &added {
            assert!(Arc::ptr_eq(arena.get(m.id), m));
        }
        assert_eq!(added[CHUNK + 4].id, MethodId((CHUNK + 4) as u32));
        assert!(arena.has_room(MAX_METHODS - CHUNK - 5));
        assert!(!arena.has_room(MAX_METHODS - CHUNK - 4));
    }
}
