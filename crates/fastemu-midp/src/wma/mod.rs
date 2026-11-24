//! `javax.wireless.messaging` (WMA, JSR-120): text and binary messages
//! that fail cleanly, so games that send SMS keep running.
//!
//! `Connector.open("sms://+15551234567")` opens a client connection that
//! only sends; `sms://:5000` and `cbs://:5000` open server connections
//! that receive. FastEmu never sends a message and none ever arrives:
//! `send` checks the message as the javadoc asks and then throws
//! `IOException`, as a phone without service did, and logs the attempt so
//! the user can see that a game tried to send an SMS (often a paid one).
//! `receive` waits until the connection closes, and a listener is kept but
//! never called. The message objects and segment counting work as on a
//! phone.

mod segments;

use std::collections::HashSet;

use fastemu_core::LogLevel;
use fastemu_jvm::classfile::access::PRIVATE;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Value, Wake};

use crate::io::{this, throw_io, url};

const MESSAGE: &str = "javax/wireless/messaging/Message";
const TEXT_MESSAGE: &str = "javax/wireless/messaging/TextMessage";
const BINARY_MESSAGE: &str = "javax/wireless/messaging/BinaryMessage";
const MESSAGE_CONNECTION: &str = "javax/wireless/messaging/MessageConnection";
const CONNECTION_IMPL: &str = "fastemu/wma/MessageConnectionImpl";
const TEXT_IMPL: &str = "fastemu/wma/TextMessageImpl";
const BINARY_IMPL: &str = "fastemu/wma/BinaryMessageImpl";

/// The message the `IOException` of every `send` carries.
const CANNOT_SEND: &str = "sending messages is not supported";

/// What a message connection was opened as.
#[derive(Clone, Debug, Default)]
struct Endpoint {
    /// `sms` or `cbs`.
    scheme: String,
    /// For a client, the address messages go to: `sms://+number[:port]`.
    address: Option<String>,
    /// For a server, the port it receives on.
    port: Option<u16>,
    /// Whether the address or the server names a port, which takes room
    /// in every segment.
    has_port: bool,
}

/// The payload of a message connection object. The listener lives in a
/// Java field, where the collector sees it.
#[derive(Default)]
struct ConnectionPayload {
    endpoint: Endpoint,
    closed: bool,
}

impl Payload for ConnectionPayload {}

/// The server ports this VM's open message connections hold, so a second
/// open of one fails as the javadoc says.
#[derive(Default)]
struct OpenPorts(HashSet<(String, u16)>);

/// Parses what follows `sms:` or `cbs:`: `//+number`, `//+number:port`
/// or `//:port`.
fn endpoint(scheme: &str, rest: &str) -> Result<Endpoint, String> {
    let Some(after) = rest.strip_prefix("//") else {
        return Err(format!("{scheme}:{rest} does not start with {scheme}://"));
    };
    let (number, port) = match after.split_once(':') {
        Some((n, p)) => (n, Some(p)),
        None => (after, None),
    };
    let port = match port {
        Some(p) => Some(
            p.parse::<u16>()
                .map_err(|_| format!("invalid port {p:?} in {scheme}:{rest}"))?,
        ),
        None => None,
    };
    if number.is_empty() {
        let Some(port) = port else {
            return Err(format!("{scheme}:{rest} has neither a number nor a port"));
        };
        return Ok(Endpoint {
            scheme: scheme.to_owned(),
            address: None,
            port: Some(port),
            has_port: true,
        });
    }
    let digits = number.strip_prefix('+').unwrap_or(number);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("invalid phone number {number:?}"));
    }
    if scheme == "cbs" {
        return Err(format!(
            "cell broadcast connections only receive: {scheme}:{rest}"
        ));
    }
    Ok(Endpoint {
        scheme: scheme.to_owned(),
        address: Some(format!("{scheme}:{rest}")),
        port: None,
        has_port: port.is_some(),
    })
}

/// Opens `sms://` or `cbs://`. Messages need no network permission: a
/// phone without signal still opened them.
pub(crate) fn open(ctx: &mut Ctx<'_>, scheme: &str, rest: &str) -> JResult<ObjRef> {
    let endpoint = match endpoint(scheme, rest) {
        Ok(e) => e,
        Err(message) => return ctx.throw("java/lang/IllegalArgumentException", &message),
    };
    if let Some(port) = endpoint.port {
        if ctx.extension::<OpenPorts>().is_none() {
            ctx.set_extension(OpenPorts::default());
        }
        let ports = ctx.extension_mut::<OpenPorts>().expect("installed above");
        if !ports.0.insert((endpoint.scheme.clone(), port)) {
            return throw_io(ctx, &format!("{scheme} port {port} is already in use"));
        }
    }
    let class = ctx.find_class(CONNECTION_IMPL)?;
    let conn = ctx.new_object(class)?;
    ctx.set_payload(
        conn,
        ConnectionPayload {
            endpoint,
            closed: false,
        },
    );
    Ok(conn)
}

/// The endpoint of an open connection, or `IOException` once closed.
fn open_endpoint(ctx: &mut Ctx<'_>, conn: ObjRef) -> JResult<Endpoint> {
    let payload = ctx.payload::<ConnectionPayload>(conn)?;
    if payload.closed {
        return throw_io(ctx, "connection is closed");
    }
    Ok(payload.endpoint.clone())
}

/// Checks a message address: `null`, or an `sms://` or `cbs://` name.
fn check_address(ctx: &mut Ctx<'_>, address: Value) -> JResult<()> {
    let Some(text) = ctx.opt_string(address)? else {
        return Ok(());
    };
    let valid = url::split_scheme(&text)
        .is_ok_and(|(scheme, rest)| (scheme == "sms" || scheme == "cbs") && rest.starts_with("//"));
    if valid {
        Ok(())
    } else {
        let message = format!("invalid message address {text:?}");
        ctx.throw("java/lang/IllegalArgumentException", &message)
    }
}

/// A new message of class `class` addressed to `address`.
fn new_message(
    ctx: &mut Ctx<'_>,
    conn: ObjRef,
    kind: Value,
    address: Option<Value>,
) -> JResult<Value> {
    let kind = ctx.opt_string(kind)?;
    let class = match kind.as_deref() {
        Some("text") => TEXT_IMPL,
        Some("binary") => BINARY_IMPL,
        _ => {
            let message = format!("unknown message type {}", kind.as_deref().unwrap_or("null"));
            return ctx.throw("java/lang/IllegalArgumentException", &message);
        }
    };
    let address = match address {
        Some(given) => {
            check_address(ctx, given)?;
            given
        }
        None => {
            let own = ctx
                .payload::<ConnectionPayload>(conn)?
                .endpoint
                .address
                .clone();
            match own {
                Some(text) => ctx.new_string(&text)?.into(),
                None => Value::Ref(None),
            }
        }
    };
    let class_id = ctx.find_class(class)?;
    let message = ctx.new_object(class_id)?;
    let field = ctx.field_id(class, "address", "Ljava/lang/String;")?;
    ctx.set_field(message, field, address);
    Ok(message.into())
}

/// The segments `message` needs, or 0 when it is too long or of a class
/// FastEmu does not know.
fn segments_of(ctx: &mut Ctx<'_>, message: ObjRef, endpoint: &Endpoint) -> JResult<i32> {
    let address = ctx.call_interface(message, "getAddress", "()Ljava/lang/String;", &[])?;
    let has_port = match ctx.opt_string(address)? {
        Some(text) => text
            .rsplit_once(':')
            .is_some_and(|(_, p)| p.parse::<u16>().is_ok()),
        None => endpoint.has_port,
    };
    let text_class = ctx.find_class(TEXT_MESSAGE)?;
    let binary_class = ctx.find_class(BINARY_MESSAGE)?;
    if ctx.is_instance(message, text_class) {
        let text = ctx.call_interface(message, "getPayloadText", "()Ljava/lang/String;", &[])?;
        let units = match text.obj() {
            Some(s) => ctx.string_utf16(s)?.to_vec(),
            None => Vec::new(),
        };
        Ok(segments::text(&units, has_port))
    } else if ctx.is_instance(message, binary_class) {
        let data = ctx.call_interface(message, "getPayloadData", "()[B", &[])?;
        let len = match data.obj() {
            Some(arr) => ctx.array_len(arr)?,
            None => 0,
        };
        Ok(segments::binary(len, has_port))
    } else {
        Ok(0)
    }
}

/// `send`: checks the message, logs the attempt and fails, because
/// FastEmu never sends a message.
fn send(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let message = ctx.non_null(a[1])?;
    let endpoint = open_endpoint(ctx, this(a))?;
    if endpoint.scheme == "cbs" {
        return throw_io(ctx, "cell broadcast connections cannot send messages");
    }
    let address = ctx.call_interface(message, "getAddress", "()Ljava/lang/String;", &[])?;
    let Some(address) = ctx.opt_string(address)? else {
        return ctx.throw(
            "java/lang/IllegalArgumentException",
            "the message has no address",
        );
    };
    if segments_of(ctx, message, &endpoint)? == 0 {
        return ctx.throw(
            "java/lang/IllegalArgumentException",
            "the message is too long to send",
        );
    }
    ctx.log(
        LogLevel::Warn,
        format!("the game tried to send a message to {address}; FastEmu does not send messages"),
    );
    throw_io(ctx, CANNOT_SEND)
}

/// `receive`: no message ever arrives, so it waits until the connection
/// closes and then throws `InterruptedIOException`, as the javadoc says.
fn receive(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let conn = this(a);
    let endpoint = open_endpoint(ctx, conn)?;
    if endpoint.port.is_none() {
        return throw_io(ctx, "a client connection cannot receive messages");
    }
    loop {
        if ctx.payload::<ConnectionPayload>(conn)?.closed {
            return ctx.throw(
                "java/io/InterruptedIOException",
                "the connection was closed during receive",
            );
        }
        match ctx.wait_until(None)? {
            Wake::Interrupted => {
                ctx.take_interrupt();
                return ctx.throw("java/io/InterruptedIOException", "interrupted");
            }
            Wake::Paused => ctx.yield_now()?,
            Wake::Notified | Wake::TimedOut => {}
        }
    }
}

fn close(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let payload = ctx.payload_mut::<ConnectionPayload>(this(a))?;
    if !std::mem::replace(&mut payload.closed, true)
        && let Some(port) = payload.endpoint.port
    {
        let key = (payload.endpoint.scheme.clone(), port);
        if let Some(ports) = ctx.extension_mut::<OpenPorts>() {
            ports.0.remove(&key);
        }
    }
    ctx.notify_waiters();
    Ok(Value::Void)
}

fn connection_class() -> NativeClass {
    NativeClass::new(CONNECTION_IMPL)
        .implements(MESSAGE_CONNECTION)
        .payload::<ConnectionPayload>()
        .field_with(
            "listener",
            "Ljavax/wireless/messaging/MessageListener;",
            PRIVATE,
        )
        .method("close", "()V", close)
        .method(
            "newMessage",
            "(Ljava/lang/String;)Ljavax/wireless/messaging/Message;",
            |ctx, a| new_message(ctx, this(a), a[1], None),
        )
        .method(
            "newMessage",
            "(Ljava/lang/String;Ljava/lang/String;)Ljavax/wireless/messaging/Message;",
            |ctx, a| new_message(ctx, this(a), a[1], Some(a[2])),
        )
        .method("send", "(Ljavax/wireless/messaging/Message;)V", send)
        .method("receive", "()Ljavax/wireless/messaging/Message;", receive)
        .method(
            "setMessageListener",
            "(Ljavax/wireless/messaging/MessageListener;)V",
            |ctx, a| {
                let endpoint = open_endpoint(ctx, this(a))?;
                if endpoint.port.is_none() {
                    return throw_io(ctx, "a client connection cannot receive messages");
                }
                let field = ctx.field_id(
                    CONNECTION_IMPL,
                    "listener",
                    "Ljavax/wireless/messaging/MessageListener;",
                )?;
                ctx.set_field(this(a), field, a[1]);
                Ok(Value::Void)
            },
        )
        .method(
            "numberOfSegments",
            "(Ljavax/wireless/messaging/Message;)I",
            |ctx, a| {
                let message = ctx.non_null(a[1])?;
                let endpoint = ctx.payload::<ConnectionPayload>(this(a))?.endpoint.clone();
                Ok(Value::Int(segments_of(ctx, message, &endpoint)?))
            },
        )
}

/// A message class: an address, no timestamp (no message was ever
/// received), and a payload field `payload` of type `desc` read and
/// written by `getter` and `setter`.
fn message_class(
    name: &'static str,
    interface: &str,
    (getter, setter, desc): (&str, &str, &'static str),
) -> NativeClass {
    let get_desc = format!("(){desc}");
    let set_desc = format!("({desc})V");
    NativeClass::new(name)
        .implements(interface)
        .field_with("address", "Ljava/lang/String;", PRIVATE)
        .field_with("payload", desc, PRIVATE)
        .method("getAddress", "()Ljava/lang/String;", move |ctx, a| {
            let field = ctx.field_id(name, "address", "Ljava/lang/String;")?;
            Ok(ctx.get_field(this(a), field))
        })
        .method("setAddress", "(Ljava/lang/String;)V", move |ctx, a| {
            check_address(ctx, a[1])?;
            let field = ctx.field_id(name, "address", "Ljava/lang/String;")?;
            ctx.set_field(this(a), field, a[1]);
            Ok(Value::Void)
        })
        .method("getTimestamp", "()Ljava/util/Date;", |_, _| {
            Ok(Value::Ref(None))
        })
        .method(getter, &get_desc, move |ctx, a| {
            let field = ctx.field_id(name, "payload", desc)?;
            Ok(ctx.get_field(this(a), field))
        })
        .method(setter, &set_desc, move |ctx, a| {
            let field = ctx.field_id(name, "payload", desc)?;
            ctx.set_field(this(a), field, a[1]);
            Ok(Value::Void)
        })
}

fn interface(name: &str, supers: &[&str], methods: &[(&str, &str)]) -> NativeClass {
    let mut class = NativeClass::interface(name);
    for s in supers {
        class = class.implements(s);
    }
    for (method, desc) in methods {
        class = class.abstract_method(method, desc);
    }
    class
}

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    vec![
        interface(
            MESSAGE,
            &[],
            &[
                ("getAddress", "()Ljava/lang/String;"),
                ("setAddress", "(Ljava/lang/String;)V"),
                ("getTimestamp", "()Ljava/util/Date;"),
            ],
        ),
        interface(
            TEXT_MESSAGE,
            &[MESSAGE],
            &[
                ("getPayloadText", "()Ljava/lang/String;"),
                ("setPayloadText", "(Ljava/lang/String;)V"),
            ],
        ),
        interface(
            BINARY_MESSAGE,
            &[MESSAGE],
            &[("getPayloadData", "()[B"), ("setPayloadData", "([B)V")],
        ),
        interface(
            MESSAGE_CONNECTION,
            &["javax/microedition/io/Connection"],
            &[
                (
                    "newMessage",
                    "(Ljava/lang/String;)Ljavax/wireless/messaging/Message;",
                ),
                (
                    "newMessage",
                    "(Ljava/lang/String;Ljava/lang/String;)Ljavax/wireless/messaging/Message;",
                ),
                ("send", "(Ljavax/wireless/messaging/Message;)V"),
                ("receive", "()Ljavax/wireless/messaging/Message;"),
                (
                    "setMessageListener",
                    "(Ljavax/wireless/messaging/MessageListener;)V",
                ),
                ("numberOfSegments", "(Ljavax/wireless/messaging/Message;)I"),
            ],
        )
        .string_constant("TEXT_MESSAGE", "text")
        .string_constant("BINARY_MESSAGE", "binary"),
        interface(
            "javax/wireless/messaging/MessageListener",
            &[],
            &[(
                "notifyIncomingMessage",
                "(Ljavax/wireless/messaging/MessageConnection;)V",
            )],
        ),
        connection_class(),
        message_class(
            TEXT_IMPL,
            TEXT_MESSAGE,
            ("getPayloadText", "setPayloadText", "Ljava/lang/String;"),
        ),
        message_class(
            BINARY_IMPL,
            BINARY_MESSAGE,
            ("getPayloadData", "setPayloadData", "[B"),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_tell_clients_from_servers() {
        let client = endpoint("sms", "//+15551234567").unwrap();
        assert_eq!(client.address.as_deref(), Some("sms://+15551234567"));
        assert_eq!((client.port, client.has_port), (None, false));
        let ported = endpoint("sms", "//5551234:5000").unwrap();
        assert!(ported.has_port);
        let server = endpoint("sms", "//:5000").unwrap();
        assert_eq!((server.address, server.port), (None, Some(5000)));
        assert!(endpoint("cbs", "//:50").is_ok());
        assert!(endpoint("cbs", "//+1").is_err());
        assert!(endpoint("sms", "//").is_err());
        assert!(endpoint("sms", "//+").is_err());
        assert!(endpoint("sms", "//12a").is_err());
        assert!(endpoint("sms", "//:99999").is_err());
        assert!(endpoint("sms", "+1").is_err());
    }
}
