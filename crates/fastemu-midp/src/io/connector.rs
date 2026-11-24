//! `javax.microedition.io.Connector`: the one way to open a connection.
//! The scheme of the name picks the protocol; a scheme FastEmu does not
//! know throws `ConnectionNotFoundException`, as on a phone that lacks
//! it.

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Throw, Value};

use super::interfaces::{INPUT_CONNECTION, NOT_FOUND, OUTPUT_CONNECTION};
use super::{datagram, http, socket, url};

const CONNECTOR: &str = "javax/microedition/io/Connector";

/// `Connector.READ`.
pub(crate) const READ: i32 = 1;
/// `Connector.WRITE`.
pub(crate) const WRITE: i32 = 2;
/// `Connector.READ_WRITE`.
pub(crate) const READ_WRITE: i32 = 3;

/// The arguments of `Connector.open` besides the name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Options {
    /// `READ`, `WRITE` or `READ_WRITE`.
    pub mode: i32,
    /// Whether the game asked for `InterruptedIOException` when the peer
    /// is too slow.
    pub timeouts: bool,
}

impl Options {
    /// Whether the connection may hand out input streams.
    pub(crate) fn can_read(self) -> bool {
        self.mode & READ != 0
    }

    /// Whether the connection may hand out output streams.
    pub(crate) fn can_write(self) -> bool {
        self.mode & WRITE != 0
    }
}

/// Opens the connection `name` names: `Connector.open(name, mode,
/// timeouts)`.
pub(crate) fn open(ctx: &mut Ctx<'_>, name: Value, mode: i32, timeouts: bool) -> JResult<ObjRef> {
    let Some(name) = ctx.opt_string(name)? else {
        return ctx.throw("java/lang/IllegalArgumentException", "Null URL");
    };
    if !(READ..=READ_WRITE).contains(&mode) {
        let message = format!("illegal mode {mode}");
        return ctx.throw("java/lang/IllegalArgumentException", &message);
    }
    let (scheme, rest) = match url::split_scheme(&name) {
        Ok(split) => split,
        Err(message) => return ctx.throw("java/lang/IllegalArgumentException", &message),
    };
    let options = Options { mode, timeouts };
    match scheme.as_str() {
        "http" | "https" => http::open(ctx, &scheme, rest, &name, options),
        "socket" => socket::open(ctx, rest, &name, options),
        "ssl" => socket::open_secure(ctx, rest, &name, options),
        "datagram" => datagram::open(ctx, rest, &name, options),
        "comm" => ctx.throw(
            NOT_FOUND,
            &format!("this phone has no serial ports: {name}"),
        ),
        _ => ctx.throw(
            NOT_FOUND,
            &format!("the requested protocol does not exist: {name}"),
        ),
    }
}

/// Opens `name` in `mode`, checks that the connection is an instance of
/// `kind`, asks it for a stream with `method` and closes the connection,
/// which stays open until the stream is closed: the `Connector.open*Stream`
/// helpers.
fn open_stream(
    ctx: &mut Ctx<'_>,
    name: Value,
    mode: i32,
    kind: &str,
    method: &str,
    desc: &str,
) -> JResult<Value> {
    let conn = open(ctx, name, mode, false)?;
    let class = ctx.find_class(kind)?;
    let result = if ctx.is_instance(conn, class) {
        ctx.call_interface(conn, method, desc, &[])
    } else {
        let name = ctx.opt_string(name)?.unwrap_or_default();
        let kind = kind.rsplit('/').next().unwrap_or(kind);
        let message = format!("{name} is not an {kind}");
        Err(ctx.new_exception("java/lang/IllegalArgumentException", Some(&message)))
    };
    let closed = ctx.call_interface(conn, "close", "()V", &[]);
    if matches!(closed, Err(Throw::Stop)) {
        return Err(Throw::Stop);
    }
    let stream = result?;
    closed?;
    Ok(stream)
}

/// The `Connector` class.
pub(crate) fn class() -> NativeClass {
    NativeClass::new(CONNECTOR)
        .constant("READ", "I", Value::Int(READ))
        .constant("WRITE", "I", Value::Int(WRITE))
        .constant("READ_WRITE", "I", Value::Int(READ_WRITE))
        .static_method(
            "open",
            "(Ljava/lang/String;)Ljavax/microedition/io/Connection;",
            |ctx, a| Ok(open(ctx, a[0], READ_WRITE, false)?.into()),
        )
        .static_method(
            "open",
            "(Ljava/lang/String;I)Ljavax/microedition/io/Connection;",
            |ctx, a| Ok(open(ctx, a[0], a[1].int(), false)?.into()),
        )
        .static_method(
            "open",
            "(Ljava/lang/String;IZ)Ljavax/microedition/io/Connection;",
            |ctx, a| Ok(open(ctx, a[0], a[1].int(), a[2].boolean())?.into()),
        )
        .static_method(
            "openInputStream",
            "(Ljava/lang/String;)Ljava/io/InputStream;",
            |ctx, a| {
                open_stream(
                    ctx,
                    a[0],
                    READ,
                    INPUT_CONNECTION,
                    "openInputStream",
                    "()Ljava/io/InputStream;",
                )
            },
        )
        .static_method(
            "openDataInputStream",
            "(Ljava/lang/String;)Ljava/io/DataInputStream;",
            |ctx, a| {
                open_stream(
                    ctx,
                    a[0],
                    READ,
                    INPUT_CONNECTION,
                    "openDataInputStream",
                    "()Ljava/io/DataInputStream;",
                )
            },
        )
        .static_method(
            "openOutputStream",
            "(Ljava/lang/String;)Ljava/io/OutputStream;",
            |ctx, a| {
                open_stream(
                    ctx,
                    a[0],
                    WRITE,
                    OUTPUT_CONNECTION,
                    "openOutputStream",
                    "()Ljava/io/OutputStream;",
                )
            },
        )
        .static_method(
            "openDataOutputStream",
            "(Ljava/lang/String;)Ljava/io/DataOutputStream;",
            |ctx, a| {
                open_stream(
                    ctx,
                    a[0],
                    WRITE,
                    OUTPUT_CONNECTION,
                    "openDataOutputStream",
                    "()Ljava/io/DataOutputStream;",
                )
            },
        )
}
