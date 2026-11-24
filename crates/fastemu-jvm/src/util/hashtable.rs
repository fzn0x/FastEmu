//! `java.util.Hashtable` and the `Enumeration` of its keys and values.
//!
//! The table is the JDK's own design, bucket for bucket: an array of
//! chained entries, 11 buckets to start, a bucket index of `(hash &
//! 0x7FFFFFFF) % length`, new entries at the head of their chain, and a
//! rehash to `2n + 1` buckets past a load of 0.75. Enumerations walk the
//! buckets from the last one down, so keys come out in the order they do
//! on Java, and games that save or draw in key order see the same order.
//!
//! Keys are hashed and compared through their own `hashCode` and
//! `equals` (see [`super::keys`] for the fast paths). Every public method
//! holds the table's monitor, as Java's `synchronized` ones do.

use super::keys;
use super::vector::element_text;
use crate::classfile::access::{PROTECTED, PUBLIC, SYNCHRONIZED};
use crate::ctx::{Ctx, JResult};
use crate::heap::{Payload, Tracer};
use crate::lang::{string_value, this, throw};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

const HASHTABLE: &str = "java/util/Hashtable";
const ENTRY: &str = "java/util/Hashtable$Entry";
const ENUMERATOR: &str = "java/util/Hashtable$Enumerator";

/// Java's load factor, applied in `float` as Java computes thresholds.
const LOAD_FACTOR: f32 = 0.75;

/// What a `Hashtable` keeps.
#[derive(Default)]
struct Table {
    /// The buckets: an `Object[]` of entries, so enumerations can hold on
    /// to the array they started on, as Java's do.
    buckets: Option<ObjRef>,
    count: i32,
    threshold: i32,
}

impl Payload for Table {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.buckets);
    }
}

/// One mapping, chained to the next entry of its bucket.
#[derive(Default)]
struct Entry {
    hash: i32,
    key: Option<ObjRef>,
    value: Option<ObjRef>,
    next: Option<ObjRef>,
}

impl Payload for Entry {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.key);
        tracer.mark_opt(self.value);
        tracer.mark_opt(self.next);
    }
}

/// What an enumeration keeps: Java's `Hashtable.Enumerator`.
#[derive(Default)]
struct Cursor {
    /// The bucket array the enumeration started on.
    buckets: Option<ObjRef>,
    /// Buckets left to visit, from this index down.
    index: usize,
    /// The next entry of the current bucket.
    entry: Option<ObjRef>,
    /// Whether it returns keys (`keys()`) or values (`elements()`).
    keys: bool,
}

impl Payload for Cursor {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.buckets);
        tracer.mark_opt(self.entry);
    }
}

/// The threshold of a table of `capacity` buckets.
fn threshold(capacity: usize) -> i32 {
    (capacity as f32 * LOAD_FACTOR) as i32
}

fn bucket_of(hash: i32, len: usize) -> usize {
    (hash & 0x7FFF_FFFF) as usize % len
}

fn table<'a>(ctx: &'a mut Ctx<'_>, ht: ObjRef) -> JResult<&'a mut Table> {
    ctx.payload_mut::<Table>(ht)
}

fn entry<'a>(ctx: &'a Ctx<'_>, e: ObjRef) -> JResult<&'a Entry> {
    ctx.payload::<Entry>(e)
}

/// The bucket array and its length; Java's 11 buckets for an object no
/// constructor ran on.
fn buckets(ctx: &mut Ctx<'_>, ht: ObjRef) -> JResult<(ObjRef, usize)> {
    let arr = match table(ctx, ht)?.buckets {
        Some(arr) => arr,
        None => {
            init(ctx, ht, 11)?;
            table(ctx, ht)?.buckets.expect("init made the buckets")
        }
    };
    Ok((arr, ctx.array_len(arr)?))
}

/// The head of bucket `i`, rooted.
fn head(ctx: &mut Ctx<'_>, arr: ObjRef, i: usize) -> JResult<Option<ObjRef>> {
    let e = ctx.array::<Option<ObjRef>>(arr)?[i];
    Ok(e.map(|e| ctx.root(e)))
}

/// The entry after `e`, rooted.
fn next(ctx: &mut Ctx<'_>, e: ObjRef) -> JResult<Option<ObjRef>> {
    let n = entry(ctx, e)?.next;
    Ok(n.map(|n| ctx.root(n)))
}

fn new_buckets(ctx: &mut Ctx<'_>, capacity: usize) -> JResult<ObjRef> {
    let object = ctx.find_class("java/lang/Object")?;
    ctx.new_object_array(object, capacity as i32)
}

fn init(ctx: &mut Ctx<'_>, ht: ObjRef, capacity: i32) -> JResult<Value> {
    if capacity < 0 {
        let message = format!("Illegal Capacity: {capacity}");
        return throw(ctx, "java/lang/IllegalArgumentException", &message);
    }
    let capacity = capacity.max(1) as usize;
    let arr = new_buckets(ctx, capacity)?;
    let t = table(ctx, ht)?;
    t.buckets = Some(arr);
    t.threshold = threshold(capacity);
    Ok(Value::Void)
}

/// The entry of `ht` whose key equals `key` (`hash` is the key's hash),
/// with the entry before it in its bucket and the bucket's index.
type Found = (Option<ObjRef>, ObjRef, usize);

fn find(ctx: &mut Ctx<'_>, ht: ObjRef, key: ObjRef, hash: i32) -> JResult<Option<Found>> {
    let (arr, len) = buckets(ctx, ht)?;
    let index = bucket_of(hash, len);
    let mut prev = None;
    let mut cur = head(ctx, arr, index)?;
    while let Some(e) = cur {
        let d = entry(ctx, e)?;
        if d.hash == hash
            && let Some(stored) = d.key
        {
            // As in Java: the stored key's equals, given the argument.
            let stored = ctx.root(stored);
            if keys::equals(ctx, stored, Some(key))? {
                return Ok(Some((prev, e, index)));
            }
        }
        prev = Some(e);
        cur = next(ctx, e)?;
    }
    Ok(None)
}

/// `rehash()`: twice the buckets plus one, the entries moved bucket by
/// bucket from the last, each to the head of its new chain.
fn rehash(ctx: &mut Ctx<'_>, ht: ObjRef) -> JResult<()> {
    let (old, old_len) = buckets(ctx, ht)?;
    let capacity = old_len * 2 + 1;
    if capacity > i32::MAX as usize - 8 {
        return Err(ctx.out_of_memory());
    }
    let new = new_buckets(ctx, capacity)?;
    let t = table(ctx, ht)?;
    t.buckets = Some(new);
    t.threshold = threshold(capacity);
    for i in (0..old_len).rev() {
        let mut cur = ctx.array::<Option<ObjRef>>(old)?[i];
        while let Some(e) = cur {
            let hash = entry(ctx, e)?.hash;
            cur = entry(ctx, e)?.next;
            let index = bucket_of(hash, capacity);
            let head = ctx.array::<Option<ObjRef>>(new)?[index];
            ctx.payload_mut::<Entry>(e)?.next = head;
            ctx.array_mut::<Option<ObjRef>>(new)?[index] = Some(e);
        }
    }
    Ok(())
}

fn put(ctx: &mut Ctx<'_>, ht: ObjRef, key: Value, value: Value) -> JResult<Value> {
    let value = ctx.non_null(value)?;
    let key = ctx.non_null(key)?;
    let mut hash = keys::hash_code(ctx, key)?;
    if let Some((_, e, _)) = find(ctx, ht, key, hash)? {
        let d = ctx.payload_mut::<Entry>(e)?;
        let old = d.value.replace(value);
        return Ok(old.map_or(Value::NULL, |o| ctx.root(o).into()));
    }
    let t = table(ctx, ht)?;
    if t.count >= t.threshold {
        rehash(ctx, ht)?;
        // Java hashes the key again after a rehash.
        hash = keys::hash_code(ctx, key)?;
    }
    let (arr, len) = buckets(ctx, ht)?;
    let index = bucket_of(hash, len);
    let class = ctx.find_class(ENTRY)?;
    let e = ctx.new_object(class)?;
    let head = ctx.array::<Option<ObjRef>>(arr)?[index];
    ctx.set_payload(
        e,
        Entry {
            hash,
            key: Some(key),
            value: Some(value),
            next: head,
        },
    );
    ctx.array_mut::<Option<ObjRef>>(arr)?[index] = Some(e);
    table(ctx, ht)?.count += 1;
    Ok(Value::NULL)
}

fn get(ctx: &mut Ctx<'_>, ht: ObjRef, key: Value) -> JResult<Option<ObjRef>> {
    let key = ctx.non_null(key)?;
    let hash = keys::hash_code(ctx, key)?;
    match find(ctx, ht, key, hash)? {
        Some((_, e, _)) => {
            let v = entry(ctx, e)?.value;
            Ok(v.map(|v| ctx.root(v)))
        }
        None => Ok(None),
    }
}

fn remove(ctx: &mut Ctx<'_>, ht: ObjRef, key: Value) -> JResult<Value> {
    let key = ctx.non_null(key)?;
    let hash = keys::hash_code(ctx, key)?;
    let Some((prev, e, index)) = find(ctx, ht, key, hash)? else {
        return Ok(Value::NULL);
    };
    let after = entry(ctx, e)?.next;
    match prev {
        Some(p) => ctx.payload_mut::<Entry>(p)?.next = after,
        None => {
            let (arr, _) = buckets(ctx, ht)?;
            ctx.array_mut::<Option<ObjRef>>(arr)?[index] = after;
        }
    }
    table(ctx, ht)?.count -= 1;
    // The entry keeps its next link, so an enumeration standing on it
    // goes on to the rest of the bucket, as in Java.
    let old = ctx.payload_mut::<Entry>(e)?.value.take();
    Ok(old.map_or(Value::NULL, |o| ctx.root(o).into()))
}

/// Every entry in enumeration order, rooted.
fn entries(ctx: &mut Ctx<'_>, ht: ObjRef) -> JResult<Vec<ObjRef>> {
    let (arr, len) = buckets(ctx, ht)?;
    let mut all = Vec::new();
    for i in (0..len).rev() {
        let mut cur = head(ctx, arr, i)?;
        while let Some(e) = cur {
            all.push(e);
            cur = next(ctx, e)?;
        }
    }
    Ok(all)
}

fn enumerate(ctx: &mut Ctx<'_>, ht: ObjRef, keys: bool) -> JResult<Value> {
    let (arr, len) = buckets(ctx, ht)?;
    let class = ctx.find_class(ENUMERATOR)?;
    let cursor = ctx.new_object(class)?;
    ctx.set_payload(
        cursor,
        Cursor {
            buckets: Some(arr),
            index: len,
            entry: None,
            keys,
        },
    );
    Ok(cursor.into())
}

/// Moves the enumeration `obj` to its next entry, if it is not on one,
/// and returns that entry.
fn advance(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<Option<ObjRef>> {
    let c = ctx.payload::<Cursor>(obj)?;
    let (arr, mut index, mut e) = (c.buckets, c.index, c.entry);
    let Some(arr) = arr else {
        return Ok(None);
    };
    while e.is_none() && index > 0 {
        index -= 1;
        e = ctx.array::<Option<ObjRef>>(arr)?[index];
    }
    let c = ctx.payload_mut::<Cursor>(obj)?;
    c.index = index;
    c.entry = e;
    Ok(e.map(|e| ctx.root(e)))
}

fn enumerator_class() -> NativeClass {
    NativeClass::new(ENUMERATOR)
        .final_class()
        .implements("java/util/Enumeration")
        .payload::<Cursor>()
        .method("hasMoreElements", "()Z", |ctx, a| {
            Ok(Value::from(advance(ctx, this(a))?.is_some()))
        })
        .method("nextElement", "()Ljava/lang/Object;", |ctx, a| {
            let Some(e) = advance(ctx, this(a))? else {
                return throw(
                    ctx,
                    "java/util/NoSuchElementException",
                    "Hashtable Enumerator",
                );
            };
            let d = entry(ctx, e)?;
            let (after, key, value) = (d.next, d.key, d.value);
            let c = ctx.payload_mut::<Cursor>(this(a))?;
            c.entry = after;
            let out = if c.keys { key } else { value };
            Ok(out.map_or(Value::NULL, |o| ctx.root(o).into()))
        })
}

/// A public method that holds the table's monitor, as Java's
/// `synchronized` methods do.
fn sync(
    class: NativeClass,
    name: &str,
    desc: &str,
    f: impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> + Send + Sync + 'static,
) -> NativeClass {
    class.method_with(name, desc, PUBLIC | SYNCHRONIZED, f)
}

fn hashtable_class() -> NativeClass {
    let class = NativeClass::new(HASHTABLE)
        .payload::<Table>()
        .method("<init>", "()V", |ctx, a| init(ctx, this(a), 11))
        .method("<init>", "(I)V", |ctx, a| init(ctx, this(a), a[1].int()))
        .method_with("rehash", "()V", PROTECTED, |ctx, a| {
            rehash(ctx, this(a))?;
            Ok(Value::Void)
        });
    let class = sync(class, "size", "()I", |ctx, a| {
        Ok(Value::Int(table(ctx, this(a))?.count))
    });
    let class = sync(class, "isEmpty", "()Z", |ctx, a| {
        Ok(Value::from(table(ctx, this(a))?.count == 0))
    });
    let class = sync(class, "keys", "()Ljava/util/Enumeration;", |ctx, a| {
        enumerate(ctx, this(a), true)
    });
    let class = sync(class, "elements", "()Ljava/util/Enumeration;", |ctx, a| {
        enumerate(ctx, this(a), false)
    });
    let class = sync(class, "contains", "(Ljava/lang/Object;)Z", |ctx, a| {
        let wanted = ctx.non_null(a[1])?;
        for e in entries(ctx, this(a))? {
            let Some(v) = entry(ctx, e)?.value else {
                continue;
            };
            let v = ctx.root(v);
            if keys::equals(ctx, v, Some(wanted))? {
                return Ok(Value::from(true));
            }
        }
        Ok(Value::from(false))
    });
    let class = sync(class, "containsKey", "(Ljava/lang/Object;)Z", |ctx, a| {
        let key = ctx.non_null(a[1])?;
        let hash = keys::hash_code(ctx, key)?;
        Ok(Value::from(find(ctx, this(a), key, hash)?.is_some()))
    });
    let class = sync(
        class,
        "get",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        |ctx, a| Ok(get(ctx, this(a), a[1])?.map_or(Value::NULL, Value::from)),
    );
    let class = sync(
        class,
        "put",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        |ctx, a| put(ctx, this(a), a[1], a[2]),
    );
    let class = sync(
        class,
        "remove",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        |ctx, a| remove(ctx, this(a), a[1]),
    );
    let class = sync(class, "clear", "()V", |ctx, a| {
        let (arr, _) = buckets(ctx, this(a))?;
        ctx.array_mut::<Option<ObjRef>>(arr)?.fill(None);
        table(ctx, this(a))?.count = 0;
        Ok(Value::Void)
    });
    sync(class, "toString", "()Ljava/lang/String;", |ctx, a| {
        let ht = this(a);
        let mut text = String::from("{");
        for (i, e) in entries(ctx, ht)?.into_iter().enumerate() {
            if i > 0 {
                text.push_str(", ");
            }
            let d = entry(ctx, e)?;
            let (key, value) = (d.key, d.value);
            // The key's toString may remove the entry, so hold both.
            let (key, value) = (key.map(|k| ctx.root(k)), value.map(|v| ctx.root(v)));
            for (part, sep) in [(key, "="), (value, "")] {
                if part == Some(ht) {
                    text.push_str("(this Map)");
                } else {
                    text.push_str(&element_text(ctx, part)?);
                }
                text.push_str(sep);
            }
        }
        text.push('}');
        string_value(ctx, &text)
    })
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        hashtable_class(),
        NativeClass::new(ENTRY).final_class().payload::<Entry>(),
        enumerator_class(),
    ]
}

#[cfg(test)]
mod tests {
    use crate::ctx::Throw;
    use crate::{MemorySource, Value, Vm, VmConfig};

    /// Bytecode can make an object without running its constructor (no
    /// verifier stops it); the library then acts on defaults instead of
    /// stopping the virtual machine.
    #[test]
    fn objects_no_constructor_ran_on_still_work() {
        let source = std::sync::Arc::new(MemorySource::new());
        let vm = Vm::new(VmConfig::default(), source, Vec::new());
        let (got, more, next) = vm.attach("test", |ctx| {
            let class = ctx.find_class(super::HASHTABLE).unwrap();
            let ht = ctx.new_object(class).unwrap();
            let k = ctx.new_string("k").unwrap();
            let desc = "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;";
            ctx.call_virtual(ht, "put", desc, &[k.into(), k.into()])
                .unwrap();
            let get = "(Ljava/lang/Object;)Ljava/lang/Object;";
            let got = ctx.call_virtual(ht, "get", get, &[k.into()]).unwrap();
            let class = ctx.find_class("java/util/Vector$Elements").unwrap();
            let e = ctx.new_object(class).unwrap();
            let more = ctx.call_virtual(e, "hasMoreElements", "()Z", &[]).unwrap();
            let next = match ctx.call_virtual(e, "nextElement", "()Ljava/lang/Object;", &[]) {
                Err(Throw::Exception(x)) => ctx.describe_exception(x),
                other => format!("{other:?}"),
            };
            (got == Value::from(k), more, next)
        });
        assert!(got);
        assert_eq!(more, Value::from(false));
        assert_eq!(next, "java.util.NoSuchElementException: Vector Enumeration");
    }
}
