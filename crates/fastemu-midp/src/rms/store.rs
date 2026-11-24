//! `javax.microedition.rms.RecordStore`.
//!
//! A store open in any number of places is one Java object; its payload
//! ([`StoreHandle`]) names the store and holds its listeners, and the
//! [`Rms`](super::state::Rms) state holds the contents. Every change is on
//! disk before the method returns; the listeners then run on the thread
//! that made it, as MIDP says.

use std::sync::Arc;

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Throw, Tracer, Value};

use super::api::LISTENER_DESC;
use super::enumeration;
use super::format::StoreData;
use super::name::StoreName;
use super::state::{self, Owner, StoreKey, lock_files, now_millis, rms};
use super::{
    INVALID_RECORD_ID_EXCEPTION, RECORD_STORE, RECORD_STORE_EXCEPTION, RECORD_STORE_FULL_EXCEPTION,
    RECORD_STORE_NOT_FOUND_EXCEPTION, RECORD_STORE_NOT_OPEN_EXCEPTION,
};

const AUTHMODE_PRIVATE: i32 = 0;
const AUTHMODE_ANY: i32 = 1;
const SECURITY_EXCEPTION: &str = "java/lang/SecurityException";
const ILLEGAL_ARGUMENT: &str = "java/lang/IllegalArgumentException";
const OUT_OF_BOUNDS: &str = "java/lang/ArrayIndexOutOfBoundsException";
const OPEN_DESC: &str = "Ljavax/microedition/rms/RecordStore;";
const ENUMERATE_DESC: &str = concat!(
    "(Ljavax/microedition/rms/RecordFilter;Ljavax/microedition/rms/RecordComparator;Z)",
    "Ljavax/microedition/rms/RecordEnumeration;"
);

/// What a `RecordStore` object knows beside its (absent) Java fields.
#[derive(Default)]
pub(crate) struct StoreHandle {
    key: Option<Arc<StoreKey>>,
    open: bool,
    /// Whether this suite may change the store: always for its own, and
    /// for another suite's only when that suite made it writable.
    writable: bool,
    /// The `RecordListener`s, in the order they were added.
    listeners: Vec<ObjRef>,
}

impl Payload for StoreHandle {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        for &listener in &self.listeners {
            tracer.mark(listener);
        }
    }

    fn heap_bytes(&self) -> usize {
        self.listeners.len() * 4
    }
}

/// The mode a store is created with.
#[derive(Clone, Copy)]
struct Mode {
    shared: bool,
    writable: bool,
}

/// The `RecordStore` class.
pub(crate) fn class() -> NativeClass {
    NativeClass::new(RECORD_STORE)
        .payload::<StoreHandle>()
        .constant("AUTHMODE_PRIVATE", "I", Value::Int(AUTHMODE_PRIVATE))
        .constant("AUTHMODE_ANY", "I", Value::Int(AUTHMODE_ANY))
        .static_method(
            "openRecordStore",
            &format!("(Ljava/lang/String;Z){OPEN_DESC}"),
            |ctx, a| {
                let name = store_name(ctx, a[0])?;
                let create = a[1].boolean().then_some(Mode {
                    shared: false,
                    writable: false,
                });
                open(
                    ctx,
                    StoreKey {
                        owner: Owner::Own,
                        name,
                    },
                    create,
                )
            },
        )
        .static_method(
            "openRecordStore",
            &format!("(Ljava/lang/String;ZIZ){OPEN_DESC}"),
            |ctx, a| {
                let shared = auth_mode(ctx, a[2].int())?;
                let name = store_name(ctx, a[0])?;
                let create = a[1].boolean().then_some(Mode {
                    shared,
                    writable: a[3].boolean(),
                });
                open(
                    ctx,
                    StoreKey {
                        owner: Owner::Own,
                        name,
                    },
                    create,
                )
            },
        )
        .static_method(
            "openRecordStore",
            &format!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;){OPEN_DESC}"),
            |ctx, a| {
                let name = store_name(ctx, a[0])?;
                let (Some(vendor), Some(suite)) = (ctx.opt_string(a[1])?, ctx.opt_string(a[2])?)
                else {
                    return ctx.throw(
                        ILLEGAL_ARGUMENT,
                        "the vendor and suite names must not be null",
                    );
                };
                let owner = rms(ctx).owner(&vendor, &suite);
                open(ctx, StoreKey { owner, name }, None)
            },
        )
        .static_method("deleteRecordStore", "(Ljava/lang/String;)V", delete_store)
        .static_method("listRecordStores", "()[Ljava/lang/String;", list_stores)
        .method("closeRecordStore", "()V", close)
        .method("setMode", "(IZ)V", set_mode)
        .method("getName", "()Ljava/lang/String;", |ctx, a| {
            let units = read(ctx, a, |_, _, data| data.name.units().to_vec())?;
            Ok(ctx.new_string_utf16(units)?.into())
        })
        .method("getVersion", "()I", |ctx, a| {
            read(ctx, a, |_, _, data| Value::Int(data.version))
        })
        .method("getNumRecords", "()I", |ctx, a| {
            read(ctx, a, |_, _, data| {
                Value::Int(clamp(data.records.len() as u64))
            })
        })
        .method("getSize", "()I", |ctx, a| {
            read(ctx, a, |_, _, data| Value::Int(clamp(data.size())))
        })
        .method("getSizeAvailable", "()I", |ctx, a| {
            read(ctx, a, |rms, key, _| {
                Value::Int(clamp(rms.available(&key.owner)))
            })
        })
        .method("getLastModified", "()J", |ctx, a| {
            read(ctx, a, |_, _, data| Value::Long(data.last_modified))
        })
        .method("getNextRecordID", "()I", |ctx, a| {
            read(ctx, a, |_, _, data| Value::Int(data.next_id))
        })
        .method(
            "addRecordListener",
            "(Ljavax/microedition/rms/RecordListener;)V",
            |ctx, a| {
                let this = ctx.non_null(a[0])?;
                if let Some(listener) = a[1].obj() {
                    add_listener(ctx, this, listener)?;
                }
                Ok(Value::Void)
            },
        )
        .method(
            "removeRecordListener",
            "(Ljavax/microedition/rms/RecordListener;)V",
            |ctx, a| {
                let this = ctx.non_null(a[0])?;
                if let Some(listener) = a[1].obj() {
                    remove_listener(ctx, this, listener)?;
                }
                Ok(Value::Void)
            },
        )
        .method("addRecord", "([BII)I", add_record)
        .method("setRecord", "(I[BII)V", set_record)
        .method("deleteRecord", "(I)V", delete_record)
        .method("getRecordSize", "(I)I", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let bytes = record(ctx, this, a[1].int())?;
            Ok(Value::Int(bytes.len() as i32))
        })
        .method("getRecord", "(I)[B", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let bytes = record(ctx, this, a[1].int())?;
            byte_array(ctx, &bytes)
        })
        .method("getRecord", "(I[BI)I", get_record_into)
        .method("enumerateRecords", ENUMERATE_DESC, |ctx, a| {
            let this = ctx.non_null(a[0])?;
            open_key(ctx, this)?;
            let e = enumeration::create(ctx, this, a[1].obj(), a[2].obj(), a[3].boolean())?;
            Ok(e.into())
        })
}

/// A store name argument, or `IllegalArgumentException`.
fn store_name(ctx: &mut Ctx<'_>, v: Value) -> JResult<StoreName> {
    let name = match v.obj() {
        Some(s) => StoreName::new(ctx.string_utf16(s)?),
        None => None,
    };
    match name {
        Some(name) => Ok(name),
        None => ctx.throw(
            ILLEGAL_ARGUMENT,
            "a record store name has 1 to 32 characters",
        ),
    }
}

/// Whether an authorization mode argument is `AUTHMODE_ANY`, or
/// `IllegalArgumentException` for a mode that is neither.
fn auth_mode(ctx: &mut Ctx<'_>, mode: i32) -> JResult<bool> {
    match mode {
        AUTHMODE_PRIVATE => Ok(false),
        AUTHMODE_ANY => Ok(true),
        _ => ctx.throw(ILLEGAL_ARGUMENT, &format!("no authorization mode {mode}")),
    }
}

/// A size as a Java `int`.
fn clamp(n: u64) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// Opens (or creates, with `create`) the store `key` and returns its one
/// object.
fn open(ctx: &mut Ctx<'_>, key: StoreKey, create: Option<Mode>) -> JResult<Value> {
    if let Some(obj) = reopen(ctx, &key) {
        return Ok(obj.into());
    }
    let files = rms(ctx).files();
    let _files = lock_files(ctx, &files);
    // Another thread may have opened it while this one waited.
    if let Some(obj) = reopen(ctx, &key) {
        return Ok(obj.into());
    }
    if !state::load(ctx, &key)? {
        let Some(mode) = create else {
            return ctx.throw(
                RECORD_STORE_NOT_FOUND_EXCEPTION,
                &format!("no record store named {}", key.name),
            );
        };
        let mut data = StoreData::new(key.name.clone(), now_millis());
        data.shared = mode.shared;
        data.writable = mode.writable;
        if !rms(ctx).fits(&key.owner, 0, data.size()) {
            return ctx.throw(
                RECORD_STORE_FULL_EXCEPTION,
                "no room is left for another record store",
            );
        }
        state::save(ctx, &key, data)?;
    }
    let writable = match &key.owner {
        Owner::Own => true,
        Owner::Other { .. } => {
            let access = rms(ctx).data(&key).map(|d| (d.shared, d.writable));
            match access {
                Some((true, writable)) => writable,
                _ => {
                    rms(ctx).unload(&key);
                    return ctx.throw(
                        SECURITY_EXCEPTION,
                        "the record store is private to the suite that made it",
                    );
                }
            }
        }
    };
    let class = ctx.find_class(RECORD_STORE)?;
    let obj = ctx.new_object(class)?;
    let shared_key = Arc::new(key.clone());
    *ctx.payload_mut::<StoreHandle>(obj)? = StoreHandle {
        key: Some(shared_key),
        open: true,
        writable,
        listeners: Vec::new(),
    };
    let object = ctx.new_global(obj);
    rms(ctx)
        .open
        .insert(key, state::OpenStore { object, opens: 1 });
    Ok(obj.into())
}

/// The object of a store that is already open, counting one more open.
fn reopen(ctx: &mut Ctx<'_>, key: &StoreKey) -> Option<ObjRef> {
    let mut open = rms(ctx).open.remove(key)?;
    open.opens = open.opens.saturating_add(1);
    let obj = ctx.global(&open.object);
    rms(ctx).open.insert(key.clone(), open);
    Some(obj)
}

fn close(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let (key, _) = open_key(ctx, this)?;
    let rms = rms(ctx);
    let last = match rms.open.get_mut(&*key) {
        Some(open) if open.opens > 1 => {
            open.opens -= 1;
            false
        }
        _ => true,
    };
    if last {
        rms.open.remove(&*key);
        rms.unload(&key);
        let handle = ctx.payload_mut::<StoreHandle>(this)?;
        handle.open = false;
        handle.listeners.clear();
    }
    Ok(Value::Void)
}

fn delete_store(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let text = ctx.non_null(a[0])?;
    let Some(name) = StoreName::new(ctx.string_utf16(text)?) else {
        let text = ctx.string(text)?;
        return ctx.throw(
            RECORD_STORE_NOT_FOUND_EXCEPTION,
            &format!("no record store named {text}"),
        );
    };
    let key = StoreKey {
        owner: Owner::Own,
        name,
    };
    let files = rms(ctx).files();
    let _files = lock_files(ctx, &files);
    if rms(ctx).open.contains_key(&key) {
        return ctx.throw(
            RECORD_STORE_EXCEPTION,
            "the record store is open; close it first",
        );
    }
    if !state::delete(ctx, &key)? {
        return ctx.throw(
            RECORD_STORE_NOT_FOUND_EXCEPTION,
            &format!("no record store named {}", key.name),
        );
    }
    Ok(Value::Void)
}

fn list_stores(ctx: &mut Ctx<'_>, _: &[Value]) -> JResult<Value> {
    let names = {
        let files = rms(ctx).files();
        let _files = lock_files(ctx, &files);
        state::scan(ctx, &Owner::Own)?;
        rms(ctx).names(&Owner::Own)
    };
    if names.is_empty() {
        return Ok(Value::NULL);
    }
    let string = ctx.find_class("java/lang/String")?;
    let array = ctx.new_object_array(string, names.len() as i32)?;
    for (i, name) in names.into_iter().enumerate() {
        let s = ctx.new_string_utf16(name.units().to_vec())?;
        ctx.array_mut::<Option<ObjRef>>(array)?[i] = Some(s);
    }
    Ok(array.into())
}

fn set_mode(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let (key, _) = open_key(ctx, this)?;
    let shared = auth_mode(ctx, a[1].int())?;
    if key.owner != Owner::Own {
        return ctx.throw(
            SECURITY_EXCEPTION,
            "only the suite that made a record store may change its mode",
        );
    }
    let writable = a[2].boolean();
    mutate(ctx, this, |data| {
        data.shared = shared;
        data.writable = writable;
        Ok(())
    })?;
    Ok(Value::Void)
}

fn add_record(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    check_writable(ctx, this)?;
    let bytes = copy_in(ctx, a[1], a[2].int(), a[3].int())?;
    let now = now_millis();
    let id = mutate(ctx, this, |data| {
        data.add(bytes, now)
            .ok_or(Refused::Full("the record store has used every record id"))
    })?;
    notify(ctx, this, "recordAdded", id)?;
    Ok(Value::Int(id))
}

fn set_record(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let id = a[1].int();
    check_writable(ctx, this)?;
    let bytes = copy_in(ctx, a[2], a[3].int(), a[4].int())?;
    let now = now_millis();
    mutate(ctx, this, |data| {
        if data.set(id, bytes, now) {
            Ok(())
        } else {
            Err(Refused::NoRecord(id))
        }
    })?;
    notify(ctx, this, "recordChanged", id)?;
    Ok(Value::Void)
}

fn delete_record(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let id = a[1].int();
    check_writable(ctx, this)?;
    let now = now_millis();
    mutate(ctx, this, |data| {
        if data.delete(id, now) {
            Ok(())
        } else {
            Err(Refused::NoRecord(id))
        }
    })?;
    notify(ctx, this, "recordDeleted", id)?;
    Ok(Value::Void)
}

fn get_record_into(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let bytes = record(ctx, this, a[1].int())?;
    let buffer = ctx.non_null(a[2])?;
    let offset = a[3].int();
    let room = ctx.array_len(buffer)?;
    let fits = usize::try_from(offset)
        .ok()
        .filter(|&off| off <= room && bytes.len() <= room - off);
    let Some(offset) = fits else {
        return ctx.throw(
            OUT_OF_BOUNDS,
            &format!(
                "a record of {} bytes does not fit at offset {offset} of byte[{room}]",
                bytes.len()
            ),
        );
    };
    let target = &mut ctx.array_mut::<i8>(buffer)?[offset..offset + bytes.len()];
    for (slot, &byte) in target.iter_mut().zip(bytes.iter()) {
        *slot = byte as i8;
    }
    Ok(Value::Int(bytes.len() as i32))
}

/// Why an edit did not happen.
enum Refused {
    NoRecord(i32),
    Full(&'static str),
}

/// Changes the open store `this`: applies `edit` to a copy of its
/// contents, checks the quota, writes the copy to disk and then makes it
/// the contents. Readers on other threads see the old contents until the
/// new ones are on disk, and a failed write changes nothing.
fn mutate<R>(
    ctx: &mut Ctx<'_>,
    this: ObjRef,
    edit: impl FnOnce(&mut StoreData) -> Result<R, Refused>,
) -> JResult<R> {
    let files = rms(ctx).files();
    let _files = lock_files(ctx, &files);
    // Checked again under the lock: another thread may have closed it.
    let key = check_writable(ctx, this)?;
    let Some(mut next) = rms(ctx).data(&key).cloned() else {
        return Err(not_open(ctx));
    };
    let old = next.size();
    let result = match edit(&mut next) {
        Ok(result) => result,
        Err(Refused::NoRecord(id)) => {
            return ctx.throw(INVALID_RECORD_ID_EXCEPTION, &format!("no record {id}"));
        }
        Err(Refused::Full(why)) => return ctx.throw(RECORD_STORE_FULL_EXCEPTION, why),
    };
    if !rms(ctx).fits(&key.owner, old, next.size()) {
        return ctx.throw(
            RECORD_STORE_FULL_EXCEPTION,
            "the suite's record stores have no room left",
        );
    }
    state::save(ctx, &key, next)?;
    Ok(result)
}

/// Calls each listener's `method` for record `id`, in the order they were
/// added. A listener added or removed by another listener takes effect
/// from the next change.
fn notify(ctx: &mut Ctx<'_>, this: ObjRef, method: &str, id: i32) -> JResult<()> {
    let listeners = ctx.payload::<StoreHandle>(this)?.listeners.clone();
    for &listener in &listeners {
        ctx.root(listener);
    }
    for listener in listeners {
        ctx.call_interface(
            listener,
            method,
            LISTENER_DESC,
            &[this.into(), Value::Int(id)],
        )?;
    }
    Ok(())
}

/// Adds a listener to an open store, once. A closed store gets none: its
/// object never changes again.
pub(crate) fn add_listener(ctx: &mut Ctx<'_>, store: ObjRef, listener: ObjRef) -> JResult<()> {
    let handle = ctx.payload_mut::<StoreHandle>(store)?;
    if handle.open && !handle.listeners.contains(&listener) {
        handle.listeners.push(listener);
    }
    Ok(())
}

/// Removes a listener, if the store has it.
pub(crate) fn remove_listener(ctx: &mut Ctx<'_>, store: ObjRef, listener: ObjRef) -> JResult<()> {
    ctx.payload_mut::<StoreHandle>(store)?
        .listeners
        .retain(|&l| l != listener);
    Ok(())
}

/// The key of the open store `this`, and whether this suite may change
/// it. Throws `RecordStoreNotOpenException` for a closed store.
fn open_key(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<(Arc<StoreKey>, bool)> {
    let handle = ctx.payload::<StoreHandle>(this)?;
    let found = match &handle.key {
        Some(key) if handle.open => Some((Arc::clone(key), handle.writable)),
        _ => None,
    };
    found.ok_or_else(|| not_open(ctx))
}

/// The key of the open store `this`, if this suite may change it; throws
/// `SecurityException` for another suite's read-only store.
fn check_writable(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<Arc<StoreKey>> {
    let (key, writable) = open_key(ctx, this)?;
    if writable {
        Ok(key)
    } else {
        ctx.throw(
            SECURITY_EXCEPTION,
            "the record store is read-only for this suite",
        )
    }
}

fn not_open(ctx: &mut Ctx<'_>) -> Throw {
    ctx.new_exception(
        RECORD_STORE_NOT_OPEN_EXCEPTION,
        Some("the record store is not open"),
    )
}

/// Runs `f` on the contents of the open store `a[0]`.
fn read<R>(
    ctx: &mut Ctx<'_>,
    a: &[Value],
    f: impl FnOnce(&state::Rms, &StoreKey, &StoreData) -> R,
) -> JResult<R> {
    let this = ctx.non_null(a[0])?;
    let (key, _) = open_key(ctx, this)?;
    let rms: &state::Rms = rms(ctx);
    let result = rms.data(&key).map(|data| f(rms, &key, data));
    result.ok_or_else(|| not_open(ctx))
}

/// What a store holds for a record id.
pub(crate) enum Lookup {
    Closed,
    Missing,
    Found(Arc<[u8]>),
}

/// Looks up record `id` of `store` without throwing.
pub(crate) fn lookup(ctx: &mut Ctx<'_>, store: ObjRef, id: i32) -> JResult<Lookup> {
    let handle = ctx.payload::<StoreHandle>(store)?;
    let key = match &handle.key {
        Some(key) if handle.open => Arc::clone(key),
        _ => return Ok(Lookup::Closed),
    };
    Ok(match rms(ctx).data(&key) {
        None => Lookup::Closed,
        Some(data) => data
            .records
            .get(&id)
            .map_or(Lookup::Missing, |bytes| Lookup::Found(Arc::clone(bytes))),
    })
}

/// The data of record `id` of `store`, throwing
/// `RecordStoreNotOpenException` or `InvalidRecordIDException`.
pub(crate) fn record(ctx: &mut Ctx<'_>, store: ObjRef, id: i32) -> JResult<Arc<[u8]>> {
    match lookup(ctx, store, id)? {
        Lookup::Found(bytes) => Ok(bytes),
        Lookup::Missing => ctx.throw(INVALID_RECORD_ID_EXCEPTION, &format!("no record {id}")),
        Lookup::Closed => Err(not_open(ctx)),
    }
}

/// The ids of `store`'s records in ascending order; none when it is
/// closed.
pub(crate) fn record_ids(ctx: &mut Ctx<'_>, store: ObjRef) -> JResult<Vec<i32>> {
    let handle = ctx.payload::<StoreHandle>(store)?;
    let key = match &handle.key {
        Some(key) if handle.open => Arc::clone(key),
        _ => return Ok(Vec::new()),
    };
    Ok(rms(ctx)
        .data(&key)
        .map(|data| data.records.keys().copied().collect())
        .unwrap_or_default())
}

/// Whether `store` is open.
pub(crate) fn is_open(ctx: &Ctx<'_>, store: ObjRef) -> JResult<bool> {
    Ok(ctx.payload::<StoreHandle>(store)?.open)
}

/// A new `byte[]` with a record's data, or `null` for a record with no
/// data, as `getRecord` returns it.
pub(crate) fn byte_array(ctx: &mut Ctx<'_>, bytes: &[u8]) -> JResult<Value> {
    if bytes.is_empty() {
        Ok(Value::NULL)
    } else {
        Ok(ctx.new_byte_array(bytes)?.into())
    }
}

/// Copies `len` bytes from `offset` of a Java `byte[]` that may be `null`
/// when `len` is 0, throwing what `System.arraycopy` would.
fn copy_in(ctx: &mut Ctx<'_>, data: Value, offset: i32, len: i32) -> JResult<Vec<u8>> {
    let Some(array) = data.obj() else {
        return match len {
            0 => Ok(Vec::new()),
            n if n < 0 => ctx.throw(OUT_OF_BOUNDS, &format!("negative length {n}")),
            _ => ctx.throw("java/lang/NullPointerException", "no data for a record"),
        };
    };
    let size = ctx.array_len(array)?;
    let range = usize::try_from(offset)
        .ok()
        .zip(usize::try_from(len).ok())
        .filter(|&(off, n)| off <= size && n <= size - off);
    let Some((offset, len)) = range else {
        return ctx.throw(
            OUT_OF_BOUNDS,
            &format!("{len} bytes from offset {offset} do not fit in byte[{size}]"),
        );
    };
    let bytes = ctx.array::<i8>(array)?[offset..offset + len]
        .iter()
        .map(|&b| b as u8)
        .collect();
    Ok(bytes)
}
