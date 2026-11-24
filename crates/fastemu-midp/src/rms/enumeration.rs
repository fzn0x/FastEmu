//! `RecordEnumeration`: a store's records, picked by the game's
//! `RecordFilter` and ordered by its `RecordComparator`.
//!
//! The enumeration keeps the matching ids in order and a cursor, as the
//! MIDP reference implementation does: after a reset `nextRecordId` gives
//! the first id and `previousRecordId` the last. Without a comparator the
//! ids ascend; the spec leaves that order open, and J2ME Loader and
//! freej2me-plus both ascend. Sorting is stable, so records the comparator
//! calls equivalent keep ascending ids. An enumeration that is kept updated
//! listens to its store like any `RecordListener` and updates itself on
//! the thread that changed the store.
//!
//! Filters and comparators are game code. While they run, other threads
//! may run too, so every reference the natives hold across a call is
//! rooted, and the methods that call them are synchronized on the
//! enumeration, as the reference implementation's are.

use fastemu_jvm::classfile::access;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value};

use super::api::LISTENER_DESC;
use super::store::{self, Lookup};
use super::{
    INVALID_RECORD_ID_EXCEPTION, RECORD_ENUMERATION, RECORD_LISTENER,
    RECORD_STORE_NOT_OPEN_EXCEPTION,
};

/// The class FastEmu's enumerations are made of. It is not part of MIDP;
/// games see it only as a `RecordEnumeration`.
const ENUMERATION_IMPL: &str = "fastemu/rms/RecordEnumerationImpl";
const SYNCHRONIZED: u16 = access::PUBLIC | access::SYNCHRONIZED;

/// Where an enumeration stands among its ids.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Cursor {
    /// Before the first and after the last: as made, or after `reset`.
    #[default]
    Reset,
    /// On the id at this index, the one returned last.
    At(usize),
    /// Between two ids, after the one it was on was removed: `next` gives
    /// the id at this index and `previous` the one before it.
    Gap(usize),
}

/// The ids of an enumeration, in order, and its cursor.
#[derive(Debug, Default, PartialEq, Eq)]
struct Order {
    ids: Vec<i32>,
    cursor: Cursor,
}

impl Order {
    /// Replaces the ids and resets the cursor, as `rebuild` does.
    fn replace(&mut self, ids: Vec<i32>) {
        self.ids = ids;
        self.cursor = Cursor::Reset;
    }

    fn next_index(&self) -> Option<usize> {
        let i = match self.cursor {
            Cursor::Reset => 0,
            Cursor::At(i) => i + 1,
            Cursor::Gap(i) => i,
        };
        (i < self.ids.len()).then_some(i)
    }

    fn previous_index(&self) -> Option<usize> {
        match self.cursor {
            Cursor::Reset => self.ids.len().checked_sub(1),
            Cursor::At(i) | Cursor::Gap(i) => i.checked_sub(1),
        }
    }

    /// Moves to the next id and returns it; `None` at the end, where the
    /// cursor stays.
    fn next(&mut self) -> Option<i32> {
        let i = self.next_index()?;
        self.cursor = Cursor::At(i);
        Some(self.ids[i])
    }

    /// Moves to the previous id and returns it; `None` at the start.
    fn previous(&mut self) -> Option<i32> {
        let i = self.previous_index()?;
        self.cursor = Cursor::At(i);
        Some(self.ids[i])
    }

    fn has_next(&self) -> bool {
        self.next_index().is_some()
    }

    fn has_previous(&self) -> bool {
        self.previous_index().is_some()
    }

    /// Inserts `id` at index `at`, keeping the cursor on the same id.
    fn insert(&mut self, at: usize, id: i32) {
        let at = at.min(self.ids.len());
        self.ids.insert(at, id);
        self.cursor = match self.cursor {
            Cursor::At(i) if at <= i => Cursor::At(i + 1),
            Cursor::Gap(i) if at < i => Cursor::Gap(i + 1),
            other => other,
        };
    }

    /// The id the cursor is on, if it is on one.
    fn current(&self) -> Option<i32> {
        match self.cursor {
            Cursor::At(i) => self.ids.get(i).copied(),
            Cursor::Reset | Cursor::Gap(_) => None,
        }
    }

    /// Puts the cursor back on `id` after the record moved, if it is still
    /// here. A walk that changes the record it is on then moves on to the
    /// next one instead of meeting the changed record again.
    fn settle_on(&mut self, id: i32) {
        if let Some(at) = self.ids.iter().position(|&x| x == id) {
            self.cursor = Cursor::At(at);
        }
    }

    /// Removes `id`, keeping the cursor on the same id, or between its
    /// neighbours when it was on `id`. Returns whether `id` was there.
    fn remove(&mut self, id: i32) -> bool {
        let Some(at) = self.ids.iter().position(|&x| x == id) else {
            return false;
        };
        self.ids.remove(at);
        self.cursor = match self.cursor {
            Cursor::At(i) if at < i => Cursor::At(i - 1),
            Cursor::At(i) if at == i => Cursor::Gap(i),
            Cursor::Gap(i) if at < i => Cursor::Gap(i - 1),
            other => other,
        };
        true
    }
}

/// What an enumeration object holds beside its (absent) Java fields.
#[derive(Default)]
struct EnumState {
    /// The store, or `None` once the enumeration is destroyed.
    store: Option<ObjRef>,
    filter: Option<ObjRef>,
    comparator: Option<ObjRef>,
    kept_updated: bool,
    order: Order,
}

impl Payload for EnumState {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        for obj in [self.store, self.filter, self.comparator]
            .into_iter()
            .flatten()
        {
            tracer.mark(obj);
        }
    }

    fn heap_bytes(&self) -> usize {
        self.order.ids.len() * 4
    }
}

/// The enumeration class.
pub(crate) fn class() -> NativeClass {
    NativeClass::new(ENUMERATION_IMPL)
        .implements(RECORD_ENUMERATION)
        .implements(RECORD_LISTENER)
        .final_class()
        .payload::<EnumState>()
        .method("numRecords", "()I", |ctx, a| {
            let this = live(ctx, a)?.0;
            let n = ctx.payload::<EnumState>(this)?.order.ids.len();
            Ok(Value::Int(n as i32))
        })
        .method("nextRecordId", "()I", |ctx, a| {
            let (this, _) = live(ctx, a)?;
            Ok(Value::Int(step(ctx, this, Order::next)?))
        })
        .method("previousRecordId", "()I", |ctx, a| {
            let (this, _) = live(ctx, a)?;
            Ok(Value::Int(step(ctx, this, Order::previous)?))
        })
        .method("nextRecord", "()[B", |ctx, a| {
            step_record(ctx, a, Order::next)
        })
        .method("previousRecord", "()[B", |ctx, a| {
            step_record(ctx, a, Order::previous)
        })
        .method("hasNextElement", "()Z", |ctx, a| {
            let (this, store) = live(ctx, a)?;
            let open = store::is_open(ctx, store)?;
            Ok(Value::from(
                open && ctx.payload::<EnumState>(this)?.order.has_next(),
            ))
        })
        .method("hasPreviousElement", "()Z", |ctx, a| {
            let (this, store) = live(ctx, a)?;
            let open = store::is_open(ctx, store)?;
            Ok(Value::from(
                open && ctx.payload::<EnumState>(this)?.order.has_previous(),
            ))
        })
        .method("reset", "()V", |ctx, a| {
            let (this, _) = live(ctx, a)?;
            ctx.payload_mut::<EnumState>(this)?.order.cursor = Cursor::Reset;
            Ok(Value::Void)
        })
        .method("isKeptUpdated", "()Z", |ctx, a| {
            let (this, _) = live(ctx, a)?;
            Ok(Value::from(ctx.payload::<EnumState>(this)?.kept_updated))
        })
        .method_with("rebuild", "()V", SYNCHRONIZED, |ctx, a| {
            let (this, _) = live(ctx, a)?;
            rebuild(ctx, this)?;
            Ok(Value::Void)
        })
        .method_with("keepUpdated", "(Z)V", SYNCHRONIZED, |ctx, a| {
            let (this, _) = live(ctx, a)?;
            keep_updated(ctx, this, a[1].boolean())?;
            Ok(Value::Void)
        })
        .method_with("destroy", "()V", SYNCHRONIZED, |ctx, a| {
            let this = ctx.non_null(a[0])?;
            destroy(ctx, this)?;
            Ok(Value::Void)
        })
        // The store calls these. One destroyed by an earlier listener of the
        // same change may still be called once; it ignores the call.
        .method_with("recordAdded", LISTENER_DESC, SYNCHRONIZED, |ctx, a| {
            let this = ctx.non_null(a[0])?;
            add_if_matching(ctx, this, a[2].int())?;
            Ok(Value::Void)
        })
        .method_with("recordChanged", LISTENER_DESC, SYNCHRONIZED, |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let id = a[2].int();
            let order = &mut ctx.payload_mut::<EnumState>(this)?.order;
            let was_current = order.current() == Some(id);
            order.remove(id);
            add_if_matching(ctx, this, id)?;
            if was_current {
                ctx.payload_mut::<EnumState>(this)?.order.settle_on(id);
            }
            Ok(Value::Void)
        })
        .method_with("recordDeleted", LISTENER_DESC, SYNCHRONIZED, |ctx, a| {
            let this = ctx.non_null(a[0])?;
            ctx.payload_mut::<EnumState>(this)?.order.remove(a[2].int());
            Ok(Value::Void)
        })
}

/// A new enumeration of `store`, as `enumerateRecords` makes it.
pub(crate) fn create(
    ctx: &mut Ctx<'_>,
    store: ObjRef,
    filter: Option<ObjRef>,
    comparator: Option<ObjRef>,
    keep: bool,
) -> JResult<ObjRef> {
    let class = ctx.find_class(ENUMERATION_IMPL)?;
    let this = ctx.new_object(class)?;
    {
        let state = ctx.payload_mut::<EnumState>(this)?;
        state.store = Some(store);
        state.filter = filter;
        state.comparator = comparator;
    }
    if keep {
        keep_updated(ctx, this, true)?;
    } else {
        rebuild(ctx, this)?;
    }
    Ok(this)
}

/// The enumeration `a[0]` and its store, or `IllegalStateException` once
/// it is destroyed.
fn live(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<(ObjRef, ObjRef)> {
    let this = ctx.non_null(a[0])?;
    match ctx.payload::<EnumState>(this)?.store {
        Some(store) => Ok((this, store)),
        None => ctx.throw(
            "java/lang/IllegalStateException",
            "the record enumeration was destroyed",
        ),
    }
}

/// Moves the cursor in `direction`, or throws `InvalidRecordIDException` when
/// there is no record that way.
fn step(ctx: &mut Ctx<'_>, this: ObjRef, direction: fn(&mut Order) -> Option<i32>) -> JResult<i32> {
    match direction(&mut ctx.payload_mut::<EnumState>(this)?.order) {
        Some(id) => Ok(id),
        None => ctx.throw(INVALID_RECORD_ID_EXCEPTION, "no more records"),
    }
}

/// `nextRecord` and `previousRecord`: the data of the record the cursor
/// moves to.
fn step_record(
    ctx: &mut Ctx<'_>,
    a: &[Value],
    direction: fn(&mut Order) -> Option<i32>,
) -> JResult<Value> {
    let (this, store) = live(ctx, a)?;
    if !store::is_open(ctx, store)? {
        return ctx.throw(
            RECORD_STORE_NOT_OPEN_EXCEPTION,
            "the record store is not open",
        );
    }
    let id = step(ctx, this, direction)?;
    let bytes = store::record(ctx, store, id)?;
    store::byte_array(ctx, &bytes)
}

/// The game objects an enumeration calls, rooted so they survive the
/// calls even if the game destroys the enumeration meanwhile.
struct Callbacks {
    store: ObjRef,
    filter: Option<ObjRef>,
    comparator: Option<ObjRef>,
}

fn callbacks(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<Option<Callbacks>> {
    let state = ctx.payload::<EnumState>(this)?;
    let Some(store) = state.store else {
        return Ok(None);
    };
    let (filter, comparator) = (state.filter, state.comparator);
    for obj in [Some(store), filter, comparator].into_iter().flatten() {
        ctx.root(obj);
    }
    Ok(Some(Callbacks {
        store,
        filter,
        comparator,
    }))
}

/// A record as filters and comparators get it: a new `byte[]`, or `null`
/// for a record with no data. `None` when the record is gone.
fn candidate(ctx: &mut Ctx<'_>, store: ObjRef, id: i32) -> JResult<Option<Value>> {
    match store::lookup(ctx, store, id)? {
        Lookup::Found(bytes) => Ok(Some(store::byte_array(ctx, &bytes)?)),
        Lookup::Missing | Lookup::Closed => Ok(None),
    }
}

fn matches(ctx: &mut Ctx<'_>, filter: Option<ObjRef>, record: Value) -> JResult<bool> {
    match filter {
        None => Ok(true),
        Some(filter) => Ok(ctx
            .call_interface(filter, "matches", "([B)Z", &[record])?
            .boolean()),
    }
}

/// The comparator's answer. Games return any negative or positive number,
/// not only `PRECEDES` and `FOLLOWS`, so only the sign counts.
fn compare(ctx: &mut Ctx<'_>, comparator: ObjRef, a: Value, b: Value) -> JResult<i32> {
    let result = ctx.call_interface(comparator, "compare", "([B[B)I", &[a, b])?;
    Ok(result.int().signum())
}

/// Builds the ids again from the store's records, as `rebuild` does.
fn rebuild(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<()> {
    let Some(cb) = callbacks(ctx, this)? else {
        return Ok(());
    };
    let ids = store::record_ids(ctx, cb.store)?;
    let mut items: Vec<(i32, Value)> = Vec::with_capacity(ids.len());
    if cb.filter.is_none() && cb.comparator.is_none() {
        items.extend(ids.into_iter().map(|id| (id, Value::NULL)));
    } else {
        for id in ids {
            let Some(record) = candidate(ctx, cb.store, id)? else {
                continue;
            };
            if matches(ctx, cb.filter, record)? {
                items.push((id, record));
            }
        }
    }
    if let Some(comparator) = cb.comparator {
        merge_sort(&mut items, |a, b| {
            Ok(compare(ctx, comparator, a.1, b.1)? > 0)
        })?;
    }
    let state = ctx.payload_mut::<EnumState>(this)?;
    if state.store.is_some() {
        state
            .order
            .replace(items.into_iter().map(|(id, _)| id).collect());
    }
    Ok(())
}

/// Puts record `id` where it belongs if it exists and the filter takes
/// it, keeping the cursor where it was.
fn add_if_matching(ctx: &mut Ctx<'_>, this: ObjRef, id: i32) -> JResult<()> {
    let Some(cb) = callbacks(ctx, this)? else {
        return Ok(());
    };
    let Some(record) = candidate(ctx, cb.store, id)? else {
        return Ok(());
    };
    if !matches(ctx, cb.filter, record)? {
        return Ok(());
    }
    let ids = ctx.payload::<EnumState>(this)?.order.ids.clone();
    let at = match cb.comparator {
        None => ids.partition_point(|&other| other < id),
        Some(comparator) => {
            let mut at = ids.len();
            for (i, &other) in ids.iter().enumerate() {
                let Some(theirs) = candidate(ctx, cb.store, other)? else {
                    continue;
                };
                let order = compare(ctx, comparator, record, theirs)?;
                if order < 0 || (order == 0 && other > id) {
                    at = i;
                    break;
                }
            }
            at
        }
    };
    let state = ctx.payload_mut::<EnumState>(this)?;
    if state.store.is_some() && !state.order.ids.contains(&id) {
        state.order.insert(at, id);
    }
    Ok(())
}

fn keep_updated(ctx: &mut Ctx<'_>, this: ObjRef, keep: bool) -> JResult<()> {
    let state = ctx.payload_mut::<EnumState>(this)?;
    let Some(store) = state.store else {
        return Ok(());
    };
    if state.kept_updated == keep {
        return Ok(());
    }
    state.kept_updated = keep;
    if keep {
        store::add_listener(ctx, store, this)?;
        rebuild(ctx, this)
    } else {
        store::remove_listener(ctx, store, this)
    }
}

/// Lets go of the store and the callbacks. Destroying an enumeration twice
/// does nothing: no game depends on the exception, and a lenient second
/// call cannot break one.
fn destroy(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<()> {
    let state = std::mem::take(ctx.payload_mut::<EnumState>(this)?);
    if let (Some(store), true) = (state.store, state.kept_updated) {
        store::remove_listener(ctx, store, this)?;
    }
    Ok(())
}

/// A stable merge sort over `items` with a comparison that may fail (a
/// Java call that throws or a VM that stops). `follows(a, b)` says whether
/// `a` goes after `b`. Unlike the standard library's sorts it accepts any
/// answers, even ones that are not a consistent order, as buggy game
/// comparators give. On an error `items` is left as it was.
fn merge_sort<T: Copy, E>(
    items: &mut Vec<T>,
    mut follows: impl FnMut(T, T) -> Result<bool, E>,
) -> Result<(), E> {
    let n = items.len();
    let mut src = items.clone();
    let mut dst = Vec::with_capacity(n);
    let mut width = 1;
    while width < n {
        dst.clear();
        let mut start = 0;
        while start < n {
            let mid = (start + width).min(n);
            let end = (start + 2 * width).min(n);
            let (mut i, mut j) = (start, mid);
            while i < mid && j < end {
                if follows(src[i], src[j])? {
                    dst.push(src[j]);
                    j += 1;
                } else {
                    dst.push(src[i]);
                    i += 1;
                }
            }
            dst.extend_from_slice(&src[i..mid]);
            dst.extend_from_slice(&src[j..end]);
            start = end;
        }
        std::mem::swap(&mut src, &mut dst);
        width *= 2;
    }
    *items = src;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(ids: &[i32]) -> Order {
        Order {
            ids: ids.to_vec(),
            cursor: Cursor::Reset,
        }
    }

    #[test]
    fn a_reset_cursor_reaches_both_ends() {
        let mut o = order(&[1, 2, 3]);
        assert!(o.has_next() && o.has_previous());
        assert_eq!(o.previous(), Some(3));
        o.cursor = Cursor::Reset;
        assert_eq!(o.next(), Some(1));
        assert!(!o.has_previous());
        assert_eq!(o.previous(), None);
        assert_eq!(o.next(), Some(2));
        assert_eq!(o.next(), Some(3));
        assert!(!o.has_next());
        // At the end the cursor stays, so the error repeats until a reset.
        assert_eq!(o.next(), None);
        assert_eq!(o.next(), None);
        assert_eq!(o.previous(), Some(2));
    }

    #[test]
    fn an_empty_order_has_nothing_either_way() {
        let mut o = order(&[]);
        assert!(!o.has_next() && !o.has_previous());
        assert_eq!(o.next(), None);
        assert_eq!(o.previous(), None);
    }

    #[test]
    fn inserts_keep_the_cursor_on_its_record() {
        let mut o = order(&[10, 20, 30]);
        o.next();
        o.next();
        // Before the current record.
        o.insert(0, 5);
        assert_eq!(o.cursor, Cursor::At(2));
        assert_eq!(o.next(), Some(30));
        // After it.
        o.insert(4, 40);
        assert_eq!(o.next(), Some(40));
    }

    #[test]
    fn removing_the_current_record_leaves_a_gap() {
        let mut o = order(&[1, 2, 3, 4]);
        o.next();
        o.next();
        assert!(o.remove(2));
        assert_eq!(o.cursor, Cursor::Gap(1));
        assert_eq!(o.next(), Some(3));
        o.remove(3);
        assert_eq!(o.previous(), Some(1));
        assert!(!o.remove(99));
        // A removal before a gap moves it back.
        let mut o = order(&[1, 2, 3, 4]);
        o.cursor = Cursor::Gap(2);
        o.remove(1);
        assert_eq!(o.next(), Some(3));
    }

    #[test]
    fn a_changed_current_record_keeps_the_cursor() {
        let mut o = order(&[1, 2, 3]);
        assert_eq!(o.next(), Some(1));
        assert_eq!(o.current(), Some(1));
        // A change removes the record and puts it back where it belongs.
        o.remove(1);
        o.insert(0, 1);
        o.settle_on(1);
        assert_eq!(o.next(), Some(2));
        // A record the filter no longer takes leaves the cursor in its gap.
        o.remove(2);
        o.settle_on(2);
        assert_eq!(o.next(), Some(3));
    }

    #[test]
    fn the_sort_is_stable() {
        let mut items: Vec<(i32, i32)> = (1..=9).map(|id| (id, id % 3)).collect();
        merge_sort(&mut items, |a, b| Ok::<_, ()>(a.1 > b.1)).unwrap();
        let ids: Vec<i32> = items.iter().map(|x| x.0).collect();
        assert_eq!(ids, [3, 6, 9, 1, 4, 7, 2, 5, 8]);
    }

    #[test]
    fn the_sort_survives_inconsistent_answers() {
        let mut items: Vec<i32> = (0..50).collect();
        let mut flip = false;
        merge_sort(&mut items, |_, _| {
            flip = !flip;
            Ok::<_, ()>(flip)
        })
        .unwrap();
        let mut sorted = items.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..50).collect::<Vec<_>>());
    }

    #[test]
    fn the_sort_stops_at_the_first_error() {
        let mut items = vec![3, 1, 2];
        let mut calls = 0;
        let result = merge_sort(&mut items, |_, _| {
            calls += 1;
            Err("thrown")
        });
        assert_eq!(result, Err("thrown"));
        assert_eq!(calls, 1);
        assert_eq!(items, [3, 1, 2]);
    }
}
