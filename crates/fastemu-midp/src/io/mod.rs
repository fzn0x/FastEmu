//! `javax.microedition.io`: the Generic Connection Framework.
//!
//! Every connection a game makes starts at `Connector.open(name)`, and
//! the scheme of the name picks the protocol (`connector`). HTTP and
//! HTTPS (`http`), TCP sockets with or without TLS (`socket`, [`tls`])
//! and UDP datagrams (`datagram`) make real connections over `std::net`
//! when the instance's [`fastemu_core::NetworkPolicy`] allows it, and
//! fail with `IOException` the way a phone without signal does when it
//! does not. `docs/architecture/connections.md` describes the design.
//!
//! Blocking work (resolving names, connecting, reading and writing) runs
//! with the VM lock released, in slices short enough that a stopping VM
//! ends it at once (`net`).

mod connector;
mod datagram;
mod http;
mod interfaces;
mod net;
mod push;
mod socket;
mod streams;
pub mod tls;
pub(crate) mod url;

use std::sync::{Mutex, MutexGuard, PoisonError};

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value};

use connector::Options;
use interfaces::NOT_FOUND;

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    let mut classes = interfaces::classes();
    classes.push(connector::class());
    classes.extend(streams::classes());
    classes.extend(http::classes());
    classes.extend(socket::classes());
    classes.extend(datagram::classes());
    classes.push(push::class());
    classes.extend(tls::classes());
    classes
}

/// `this` of an instance native. The interpreter null-checks receivers,
/// so it is always present.
pub(crate) fn this(args: &[Value]) -> ObjRef {
    args[0]
        .obj()
        .expect("instance methods always have a receiver")
}

/// Throws `IOException` with `message`.
pub(crate) fn throw_io<T>(ctx: &mut Ctx<'_>, message: &str) -> JResult<T> {
    Err(ctx.new_exception("java/io/IOException", Some(message)))
}

/// A Java string, or `null` for `None`, as a return value.
pub(crate) fn opt_string_value(ctx: &mut Ctx<'_>, text: Option<&str>) -> JResult<Value> {
    Ok(match text {
        Some(t) => ctx.new_string(t)?.into(),
        None => Value::Ref(None),
    })
}

/// Locks `mutex`, ignoring poisoning: the data these mutexes guard stays
/// consistent between statements, and a panic stops the VM anyway.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Checks that `[offset, offset + len)` lies within an array of `size`
/// elements, throwing `IndexOutOfBoundsException` as `InputStream` and
/// `OutputStream` do when it does not.
pub(crate) fn check_range(
    ctx: &mut Ctx<'_>,
    offset: i32,
    len: i32,
    size: usize,
) -> JResult<(usize, usize)> {
    match (usize::try_from(offset), usize::try_from(len)) {
        (Ok(off), Ok(n)) if off.checked_add(n).is_some_and(|end| end <= size) => Ok((off, n)),
        _ => ctx.throw(
            "java/lang/IndexOutOfBoundsException",
            &format!("offset {offset}, length {len}, size {size}"),
        ),
    }
}

/// Copies `n` bytes at `off` out of the Java `byte[]` `arr`.
pub(crate) fn copy_out(ctx: &Ctx<'_>, arr: ObjRef, off: usize, n: usize) -> JResult<Vec<u8>> {
    Ok(ctx.array::<i8>(arr)?[off..off + n]
        .iter()
        .map(|&b| b as u8)
        .collect())
}

/// Copies `bytes` into the Java `byte[]` `arr` at `off`.
pub(crate) fn copy_in(ctx: &mut Ctx<'_>, arr: ObjRef, off: usize, bytes: &[u8]) -> JResult<()> {
    let dst = &mut ctx.array_mut::<i8>(arr)?[off..off + bytes.len()];
    for (d, &b) in dst.iter_mut().zip(bytes) {
        *d = b as i8;
    }
    Ok(())
}
