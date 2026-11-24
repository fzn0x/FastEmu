//! The object heap: a slab of objects addressed by handles, with a precise,
//! non-moving mark and sweep collector.
//!
//! Allocation never collects. It only counts bytes against the limit and
//! asks for a collection when the heap fills up; the interpreter runs the
//! collection at its next safepoint, where every reference a thread holds
//! is in a root the collector can see.
//!
//! Each object remembers the size it was last counted at, so a payload
//! that changes size after allocation can be counted again with
//! [`Heap::recount`] and a collection can tell what it freed.

mod payload;

use std::any::Any;
use std::collections::HashMap;

pub use payload::{Payload, Tracer};

use crate::class::ClassId;
use crate::value::{ObjRef, Value};

/// What an object holds: its Java fields, or an array's elements.
///
/// `pub` only so the sealed array element trait can name it; the module
/// is private.
pub enum Body {
    /// Instance fields, one value per field, in layout order.
    Fields(Box<[Value]>),
    /// `boolean[]` and `byte[]`.
    Bytes(Box<[i8]>),
    Chars(Box<[u16]>),
    Shorts(Box<[i16]>),
    Ints(Box<[i32]>),
    Longs(Box<[i64]>),
    Floats(Box<[f32]>),
    Doubles(Box<[f64]>),
    Refs(Box<[Option<ObjRef>]>),
}

impl Body {
    /// The array length, or `None` for instances.
    pub(crate) fn array_len(&self) -> Option<usize> {
        Some(match self {
            Body::Fields(_) => return None,
            Body::Bytes(a) => a.len(),
            Body::Chars(a) => a.len(),
            Body::Shorts(a) => a.len(),
            Body::Ints(a) => a.len(),
            Body::Longs(a) => a.len(),
            Body::Floats(a) => a.len(),
            Body::Doubles(a) => a.len(),
            Body::Refs(a) => a.len(),
        })
    }

    /// A copy, for `clone()` on arrays.
    pub(crate) fn duplicate(&self) -> Body {
        match self {
            Body::Fields(a) => Body::Fields(a.clone()),
            Body::Bytes(a) => Body::Bytes(a.clone()),
            Body::Chars(a) => Body::Chars(a.clone()),
            Body::Shorts(a) => Body::Shorts(a.clone()),
            Body::Ints(a) => Body::Ints(a.clone()),
            Body::Longs(a) => Body::Longs(a.clone()),
            Body::Floats(a) => Body::Floats(a.clone()),
            Body::Doubles(a) => Body::Doubles(a.clone()),
            Body::Refs(a) => Body::Refs(a.clone()),
        }
    }

    /// The bytes the body counts against the heap limit.
    pub(crate) fn bytes(&self) -> usize {
        match self {
            Body::Fields(f) => f.iter().map(|v| if v.is_wide() { 8 } else { 4 }).sum(),
            Body::Bytes(a) => 4 + a.len(),
            Body::Chars(a) => 4 + a.len() * 2,
            Body::Shorts(a) => 4 + a.len() * 2,
            Body::Ints(a) => 4 + a.len() * 4,
            Body::Longs(a) => 4 + a.len() * 8,
            Body::Floats(a) => 4 + a.len() * 4,
            Body::Doubles(a) => 4 + a.len() * 8,
            Body::Refs(a) => 4 + a.len() * 4,
        }
    }
}

/// One object on the heap.
pub(crate) struct HeapObject {
    pub(crate) class: ClassId,
    /// The size this object is counted at in [`Heap::used`]: what
    /// [`HeapObject::size`] returned when the object was placed on the
    /// heap, last recounted or last measured by a collection. It fits in
    /// the padding after `class`. A size past `u32::MAX` counts as
    /// `u32::MAX`.
    counted: u32,
    pub(crate) body: Body,
    pub(crate) payload: Option<Box<dyn Payload>>,
}

impl HeapObject {
    /// An object not yet counted; [`Heap::alloc`] counts it.
    pub(crate) fn new(class: ClassId, body: Body, payload: Option<Box<dyn Payload>>) -> Self {
        Self {
            class,
            counted: 0,
            body,
            payload,
        }
    }

    /// The bytes this object counts against the heap limit, close to what
    /// a phone's virtual machine would use for it.
    fn size(&self) -> usize {
        let payload = self.payload.as_ref().map_or(0, |p| p.heap_bytes());
        (8 + self.body.bytes() + payload).next_multiple_of(8)
    }

    /// [`HeapObject::size`] as stored in `counted`.
    fn measure(&self) -> u32 {
        u32::try_from(self.size()).unwrap_or(u32::MAX)
    }

    pub(crate) fn payload<T: Payload>(&self) -> Option<&T> {
        let any: &dyn Any = self.payload.as_deref()?;
        any.downcast_ref()
    }

    pub(crate) fn payload_mut<T: Payload>(&mut self) -> Option<&mut T> {
        let any: &mut dyn Any = self.payload.as_deref_mut()?;
        any.downcast_mut()
    }
}

/// Counters for logs and tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeapStats {
    /// Bytes in use, counted as a phone would.
    pub used_bytes: usize,
    /// The configured heap size.
    pub limit_bytes: usize,
    /// Objects on the heap, live or not yet collected.
    pub live_objects: usize,
    /// Collections run so far.
    pub collections: u64,
    /// Objects freed by all collections so far.
    pub freed_objects: u64,
}

pub(crate) struct Heap {
    objects: Vec<Option<HeapObject>>,
    free: Vec<u32>,
    used: usize,
    limit: usize,
    /// Allocation past this many bytes asks for a collection.
    threshold: usize,
    gc_requested: bool,
    marks: Vec<u64>,
    gray: Vec<u32>,
    live: usize,
    collections: u64,
    freed: u64,
}

impl Heap {
    pub(crate) fn new(limit: usize) -> Self {
        let limit = limit.max(64 * 1024);
        Self {
            objects: Vec::new(),
            free: Vec::new(),
            used: 0,
            limit,
            threshold: limit - limit / 8,
            gc_requested: false,
            marks: Vec::new(),
            gray: Vec::new(),
            live: 0,
            collections: 0,
            freed: 0,
        }
    }

    pub(crate) fn limit(&self) -> usize {
        self.limit
    }

    pub(crate) fn used(&self) -> usize {
        self.used
    }

    pub(crate) fn stats(&self) -> HeapStats {
        HeapStats {
            used_bytes: self.used,
            limit_bytes: self.limit,
            live_objects: self.live,
            collections: self.collections,
            freed_objects: self.freed,
        }
    }

    /// Whether the next safepoint should collect.
    #[inline]
    pub(crate) fn gc_requested(&self) -> bool {
        self.gc_requested
    }

    /// Whether allocating `bytes` more would pass the heap limit.
    #[inline]
    pub(crate) fn would_exceed(&self, bytes: usize) -> bool {
        self.used.saturating_add(bytes) > self.limit
    }

    /// Whether a native should refuse to make an object of `bytes`: more
    /// than the whole heap, or more than twice the heap with what is
    /// counted now. The second bound is twice the limit, not the limit,
    /// because the count includes garbage not yet collected and a native
    /// cannot collect (it may hold references the collector cannot see);
    /// it still keeps one native call from asking for more than about
    /// three times the heap.
    #[inline]
    pub(crate) fn refuses(&self, bytes: usize) -> bool {
        bytes > self.limit || self.used.saturating_add(bytes) > self.limit.saturating_mul(2)
    }

    /// Estimated bytes of an array of `len` elements of `elem_bytes` each.
    pub(crate) fn array_bytes(len: usize, elem_bytes: usize) -> usize {
        (12usize.saturating_add(len.saturating_mul(elem_bytes))).next_multiple_of(8)
    }

    /// Places `obj` on the heap. Never collects; asks for a collection
    /// when the heap is filling up.
    pub(crate) fn alloc(&mut self, mut obj: HeapObject) -> ObjRef {
        obj.counted = obj.measure();
        self.used += obj.counted as usize;
        self.live += 1;
        if self.used > self.threshold {
            self.gc_requested = true;
        }
        match self.free.pop() {
            Some(index) => {
                self.objects[index as usize] = Some(obj);
                ObjRef::from_index(index as usize)
            }
            None => {
                self.objects.push(Some(obj));
                ObjRef::from_index(self.objects.len() - 1)
            }
        }
    }

    /// Counts the change in the size of `r` since it was last counted, as
    /// an allocation of the difference would be: `used` follows, and a
    /// heap past its threshold asks for a collection.
    ///
    /// A payload that grows or shrinks after allocation (a buffer, a stack
    /// trace, a thread's name) calls this once its size has changed, so
    /// the heap does not learn of it only at the next collection.
    pub(crate) fn recount(&mut self, r: ObjRef) {
        let obj = self.get_mut(r);
        let after = obj.measure();
        let before = std::mem::replace(&mut obj.counted, after);
        self.used = self
            .used
            .saturating_add(after as usize)
            .saturating_sub(before as usize);
        if self.used > self.threshold {
            self.gc_requested = true;
        }
    }

    /// Replaces the payload of `r` and counts the change in its size, so
    /// a payload set after allocation asks for a collection as an
    /// allocation of its size would.
    pub(crate) fn set_payload(&mut self, r: ObjRef, payload: Box<dyn Payload>) {
        self.get_mut(r).payload = Some(payload);
        self.recount(r);
    }

    /// The object `r` names.
    ///
    /// # Panics
    ///
    /// When `r` names a freed object: a native kept a reference that was
    /// not rooted across a safepoint, which is a bug in that native.
    #[inline]
    pub(crate) fn get(&self, r: ObjRef) -> &HeapObject {
        match self.objects.get(r.index()) {
            Some(Some(obj)) => obj,
            _ => dangling(r),
        }
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, r: ObjRef) -> &mut HeapObject {
        match self.objects.get_mut(r.index()) {
            Some(Some(obj)) => obj,
            _ => dangling(r),
        }
    }

    /// Two different objects at once, for copies between arrays.
    ///
    /// # Panics
    ///
    /// When `a` and `b` are the same object, or either is freed.
    pub(crate) fn pair_mut(&mut self, a: ObjRef, b: ObjRef) -> (&mut HeapObject, &mut HeapObject) {
        let (i, j) = (a.index(), b.index());
        assert_ne!(i, j, "pair_mut needs two different objects");
        let (low, high) = self.objects.split_at_mut(i.max(j));
        let (first, second) = (&mut low[i.min(j)], &mut high[0]);
        let (first, second) = (
            first.as_mut().unwrap_or_else(|| dangling(a)),
            second.as_mut().unwrap_or_else(|| dangling(b)),
        );
        if i < j {
            (first, second)
        } else {
            (second, first)
        }
    }

    #[inline]
    pub(crate) fn class_of(&self, r: ObjRef) -> ClassId {
        self.get(r).class
    }

    /// Whether `r` names a live object.
    #[cfg(test)]
    pub(crate) fn contains(&self, r: ObjRef) -> bool {
        matches!(self.objects.get(r.index()), Some(Some(_)))
    }

    /// Frees every object the roots do not reach.
    ///
    /// `roots` marks the roots. `weak` maps each `WeakReference` object to
    /// its referent: entries for dead references are dropped and referents
    /// that died are cleared. Returns how many objects were freed.
    pub(crate) fn collect(
        &mut self,
        roots: impl FnOnce(&mut Tracer<'_>),
        weak: &mut HashMap<ObjRef, Option<ObjRef>>,
    ) -> usize {
        let words = self.objects.len().div_ceil(64);
        self.marks.clear();
        self.marks.resize(words, 0);
        self.gray.clear();
        {
            let Heap {
                objects,
                marks,
                gray,
                ..
            } = self;
            let mut tracer = Tracer { marks, gray };
            roots(&mut tracer);
            while let Some(index) = tracer.gray.pop() {
                let Some(obj) = &objects[index as usize] else {
                    continue;
                };
                match &obj.body {
                    Body::Fields(fields) => {
                        for &v in fields {
                            tracer.mark_value(v);
                        }
                    }
                    Body::Refs(elems) => {
                        for &r in elems {
                            tracer.mark_opt(r);
                        }
                    }
                    _ => {}
                }
                if let Some(payload) = &obj.payload {
                    payload.trace(&mut tracer);
                }
            }
        }
        let marked = |marks: &[u64], r: ObjRef| {
            let i = r.index();
            marks.get(i / 64).is_some_and(|w| w & (1 << (i % 64)) != 0)
        };
        weak.retain(|&k, _| marked(&self.marks, k));
        for referent in weak.values_mut() {
            if referent.is_some_and(|r| !marked(&self.marks, r)) {
                *referent = None;
            }
        }
        let mut freed = 0;
        let mut used = 0;
        let mut live = 0;
        for (index, slot) in self.objects.iter_mut().enumerate() {
            let Some(obj) = slot else { continue };
            if self.marks[index / 64] & (1 << (index % 64)) == 0 {
                *slot = None;
                self.free.push(index as u32);
                freed += 1;
            } else {
                obj.counted = obj.measure();
                used += obj.counted as usize;
                live += 1;
            }
        }
        // Trailing free slots shrink the slab, so a burst of garbage does
        // not keep a large table alive.
        while matches!(self.objects.last(), Some(None)) {
            self.objects.pop();
        }
        let len = self.objects.len() as u32;
        self.free.retain(|&i| i < len);
        self.free.sort_unstable_by(|a, b| b.cmp(a));
        self.used = used;
        self.live = live;
        self.collections += 1;
        self.freed += freed as u64;
        self.gc_requested = false;
        self.threshold = (used + self.limit / 8).clamp(self.limit - self.limit / 8, self.limit);
        freed
    }
}

#[cold]
#[inline(never)]
fn dangling(r: ObjRef) -> ! {
    panic!("{r:?} names a freed object: a native held an unrooted reference across a safepoint")
}

/// The identity hash code of `r`: stable for the object's life, as
/// `System.identityHashCode` requires.
pub(crate) fn identity_hash(r: ObjRef) -> i32 {
    (r.to_raw().wrapping_mul(0x9E37_79B9) >> 1) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(fields: Vec<Value>) -> HeapObject {
        HeapObject::new(ClassId(0), Body::Fields(fields.into()), None)
    }

    struct Holder(Option<ObjRef>);

    impl Payload for Holder {
        fn trace(&self, tracer: &mut Tracer<'_>) {
            tracer.mark_opt(self.0);
        }
        fn heap_bytes(&self) -> usize {
            100
        }
    }

    #[test]
    fn unreachable_objects_are_freed() {
        let mut heap = Heap::new(1 << 20);
        let leaf = heap.alloc(instance(vec![Value::Int(1)]));
        let root = heap.alloc(instance(vec![Value::from(leaf)]));
        let garbage = heap.alloc(instance(vec![Value::NULL]));
        let freed = heap.collect(|t| t.mark(root), &mut HashMap::new());
        assert_eq!(freed, 1);
        assert!(heap.contains(root) && heap.contains(leaf));
        assert!(!heap.contains(garbage));
        assert_eq!(heap.stats().live_objects, 2);
    }

    #[test]
    fn payloads_trace_their_references_and_count_bytes() {
        let mut heap = Heap::new(1 << 20);
        let target = heap.alloc(instance(Vec::new()));
        let holder = heap.alloc(HeapObject::new(
            ClassId(0),
            Body::Fields(Box::new([])),
            Some(Box::new(Holder(Some(target)))),
        ));
        heap.collect(|t| t.mark(holder), &mut HashMap::new());
        assert!(heap.contains(target));
        assert!(heap.used() >= 100);
        assert!(heap.get(holder).payload::<Holder>().is_some());
    }

    #[test]
    fn cycles_die_together_and_slots_are_reused() {
        let mut heap = Heap::new(1 << 20);
        let a = heap.alloc(instance(vec![Value::NULL]));
        let b = heap.alloc(instance(vec![Value::from(a)]));
        if let Body::Fields(f) = &mut heap.get_mut(a).body {
            f[0] = Value::from(b);
        }
        let keep = heap.alloc(instance(Vec::new()));
        assert_eq!(heap.collect(|t| t.mark(keep), &mut HashMap::new()), 2);
        let again = heap.alloc(instance(Vec::new()));
        assert!(again.index() < 2, "a freed slot is reused");
    }

    #[test]
    fn weak_referents_clear_when_they_die() {
        let mut heap = Heap::new(1 << 20);
        let referent = heap.alloc(instance(Vec::new()));
        let weak_ref = heap.alloc(instance(Vec::new()));
        let mut weak = HashMap::from([(weak_ref, Some(referent))]);
        heap.collect(|t| t.mark(weak_ref), &mut weak);
        assert_eq!(weak[&weak_ref], None);
        assert!(!heap.contains(referent));
        heap.collect(|_| {}, &mut weak);
        assert!(weak.is_empty(), "dead weak references leave the table");
    }

    #[test]
    fn filling_the_heap_requests_a_collection() {
        let mut heap = Heap::new(64 * 1024);
        assert!(!heap.gc_requested());
        while !heap.gc_requested() {
            heap.alloc(HeapObject::new(
                ClassId(0),
                Body::Ints(vec![0; 256].into()),
                None,
            ));
        }
        assert!(heap.would_exceed(64 * 1024));
        heap.collect(|_| {}, &mut HashMap::new());
        assert_eq!(heap.used(), 0);
        assert!(!heap.gc_requested());
    }

    /// A payload whose size a test changes after the object is placed.
    struct Sized(usize);

    impl Payload for Sized {
        fn heap_bytes(&self) -> usize {
            self.0
        }
    }

    fn sized(heap: &mut Heap, bytes: usize) -> ObjRef {
        heap.alloc(HeapObject::new(
            ClassId(0),
            Body::Fields(Box::new([])),
            Some(Box::new(Sized(bytes))),
        ))
    }

    /// Changes the size of `r`'s payload the way a native does before it
    /// calls `recount`: the heap is not told.
    fn resize(heap: &mut Heap, r: ObjRef, bytes: usize) {
        heap.get_mut(r)
            .payload_mut::<Sized>()
            .expect("a Sized payload")
            .0 = bytes;
    }

    #[test]
    fn recount_counts_a_payload_that_grew() {
        let mut heap = Heap::new(1 << 20);
        let r = sized(&mut heap, 1000);
        assert_eq!(heap.used(), 1008);
        resize(&mut heap, r, 5000);
        assert_eq!(heap.used(), 1008, "nothing counts the growth until asked");
        heap.recount(r);
        assert_eq!(heap.used(), 5008);
        heap.recount(r);
        assert_eq!(heap.used(), 5008, "a second recount adds nothing");
    }

    #[test]
    fn recount_takes_off_a_payload_that_shrank() {
        let mut heap = Heap::new(1 << 20);
        let keep = sized(&mut heap, 2000);
        let r = sized(&mut heap, 5000);
        resize(&mut heap, r, 100);
        heap.recount(r);
        assert_eq!(heap.used(), 2008 + 112);
        assert!(heap.contains(keep));
    }

    #[test]
    fn recount_past_the_threshold_requests_a_collection() {
        let mut heap = Heap::new(64 * 1024);
        let r = sized(&mut heap, 100);
        resize(&mut heap, r, 60_000);
        assert!(!heap.gc_requested(), "growth is not seen before recount");
        heap.recount(r);
        assert!(heap.gc_requested());
        assert!(heap.used() > 60_000);
    }

    #[test]
    fn recount_below_the_threshold_asks_for_nothing() {
        let mut heap = Heap::new(64 * 1024);
        let r = sized(&mut heap, 100);
        resize(&mut heap, r, 4000);
        heap.recount(r);
        assert!(!heap.gc_requested());
    }

    #[test]
    fn set_payload_replaces_the_size_the_object_was_counted_at() {
        let mut heap = Heap::new(1 << 20);
        let r = sized(&mut heap, 1000);
        // The payload grew in place and nobody recounted it, so the heap
        // still has the object at 1008 bytes.
        resize(&mut heap, r, 5000);
        heap.set_payload(r, Box::new(Sized(2000)));
        assert_eq!(heap.used(), 2008, "taken off at 1008, not at 5008");
    }

    #[test]
    fn a_collection_measures_every_live_object_again() {
        let mut heap = Heap::new(1 << 20);
        let live = sized(&mut heap, 1000);
        let dead = sized(&mut heap, 1000);
        resize(&mut heap, live, 5000);
        resize(&mut heap, dead, 5000);
        assert_eq!(heap.used(), 2016);
        heap.collect(|t| t.mark(live), &mut HashMap::new());
        assert_eq!(heap.used(), 5008, "the live payload at its new size");
        heap.recount(live);
        assert_eq!(heap.used(), 5008, "the collection already counted it");
        resize(&mut heap, live, 100);
        heap.recount(live);
        assert_eq!(heap.used(), 112);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn a_size_past_four_gigabytes_counts_as_the_most_that_fits() {
        let mut heap = Heap::new(1 << 20);
        let r = sized(&mut heap, 1 << 40);
        assert_eq!(heap.used(), u32::MAX as usize);
        resize(&mut heap, r, 100);
        heap.recount(r);
        assert_eq!(heap.used(), 112);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn the_counted_size_fits_in_the_padding() {
        assert_eq!(
            std::mem::size_of::<HeapObject>(),
            std::mem::size_of::<(Body, Option<Box<dyn Payload>>, u64)>()
        );
    }

    #[test]
    fn a_native_may_not_ask_for_more_than_the_heap_or_twice_it_with_what_is_counted() {
        let mut heap = Heap::new(64 * 1024);
        assert!(!heap.refuses(64 * 1024), "the whole heap is allowed");
        assert!(heap.refuses(64 * 1024 + 1));
        assert!(heap.refuses(usize::MAX));
        sized(&mut heap, 60_000);
        assert!(!heap.refuses(64 * 1024), "60 KB and 64 KB are under 128 KB");
        sized(&mut heap, 60_000);
        assert!(heap.refuses(64 * 1024), "120 KB and 64 KB are over 128 KB");
        assert!(!heap.refuses(1000));
    }

    #[test]
    fn identity_hashes_are_stable_and_spread() {
        let a = ObjRef::from_index(0);
        let b = ObjRef::from_index(1);
        assert_eq!(identity_hash(a), identity_hash(a));
        assert_ne!(identity_hash(a), identity_hash(b));
        assert!(identity_hash(b) >= 0);
    }
}
