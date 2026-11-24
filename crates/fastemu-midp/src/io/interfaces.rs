//! The interfaces of the Generic Connection Framework and
//! `ConnectionNotFoundException`, exactly as `docs/compat/api/midp-io.md`
//! lists them. The classes that implement them live beside the protocol
//! code.

use fastemu_jvm::{NativeClass, Value};

use super::this;

/// `javax.microedition.io.Connection`.
pub(crate) const CONNECTION: &str = "javax/microedition/io/Connection";
/// `javax.microedition.io.InputConnection`.
pub(crate) const INPUT_CONNECTION: &str = "javax/microedition/io/InputConnection";
/// `javax.microedition.io.OutputConnection`.
pub(crate) const OUTPUT_CONNECTION: &str = "javax/microedition/io/OutputConnection";
/// `javax.microedition.io.StreamConnection`.
pub(crate) const STREAM_CONNECTION: &str = "javax/microedition/io/StreamConnection";
/// `javax.microedition.io.ConnectionNotFoundException`.
pub(crate) const NOT_FOUND: &str = "javax/microedition/io/ConnectionNotFoundException";

const CONTENT_CONNECTION: &str = "javax/microedition/io/ContentConnection";
const NOTIFIER: &str = "javax/microedition/io/StreamConnectionNotifier";
const DATAGRAM_CONNECTION: &str = "javax/microedition/io/DatagramConnection";
const SOCKET_CONNECTION: &str = "javax/microedition/io/SocketConnection";
const HTTP_CONNECTION: &str = "javax/microedition/io/HttpConnection";

/// The `HttpConnection` constants: the three methods and the status
/// codes, with the values the MIDP 2.0 javadoc gives.
const HTTP_CODES: &[(&str, i32)] = &[
    ("HTTP_OK", 200),
    ("HTTP_CREATED", 201),
    ("HTTP_ACCEPTED", 202),
    ("HTTP_NOT_AUTHORITATIVE", 203),
    ("HTTP_NO_CONTENT", 204),
    ("HTTP_RESET", 205),
    ("HTTP_PARTIAL", 206),
    ("HTTP_MULT_CHOICE", 300),
    ("HTTP_MOVED_PERM", 301),
    ("HTTP_MOVED_TEMP", 302),
    ("HTTP_SEE_OTHER", 303),
    ("HTTP_NOT_MODIFIED", 304),
    ("HTTP_USE_PROXY", 305),
    ("HTTP_TEMP_REDIRECT", 307),
    ("HTTP_BAD_REQUEST", 400),
    ("HTTP_UNAUTHORIZED", 401),
    ("HTTP_PAYMENT_REQUIRED", 402),
    ("HTTP_FORBIDDEN", 403),
    ("HTTP_NOT_FOUND", 404),
    ("HTTP_BAD_METHOD", 405),
    ("HTTP_NOT_ACCEPTABLE", 406),
    ("HTTP_PROXY_AUTH", 407),
    ("HTTP_CLIENT_TIMEOUT", 408),
    ("HTTP_CONFLICT", 409),
    ("HTTP_GONE", 410),
    ("HTTP_LENGTH_REQUIRED", 411),
    ("HTTP_PRECON_FAILED", 412),
    ("HTTP_ENTITY_TOO_LARGE", 413),
    ("HTTP_REQ_TOO_LONG", 414),
    ("HTTP_UNSUPPORTED_TYPE", 415),
    ("HTTP_UNSUPPORTED_RANGE", 416),
    ("HTTP_EXPECT_FAILED", 417),
    ("HTTP_INTERNAL_ERROR", 500),
    ("HTTP_NOT_IMPLEMENTED", 501),
    ("HTTP_BAD_GATEWAY", 502),
    ("HTTP_UNAVAILABLE", 503),
    ("HTTP_GATEWAY_TIMEOUT", 504),
    ("HTTP_VERSION", 505),
];

/// An interface named `name` with the abstract methods `methods`, each a
/// name and descriptor.
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

fn http_connection() -> NativeClass {
    let mut class = interface(
        HTTP_CONNECTION,
        &[CONTENT_CONNECTION],
        &[
            ("getURL", "()Ljava/lang/String;"),
            ("getProtocol", "()Ljava/lang/String;"),
            ("getHost", "()Ljava/lang/String;"),
            ("getFile", "()Ljava/lang/String;"),
            ("getRef", "()Ljava/lang/String;"),
            ("getQuery", "()Ljava/lang/String;"),
            ("getPort", "()I"),
            ("getRequestMethod", "()Ljava/lang/String;"),
            ("setRequestMethod", "(Ljava/lang/String;)V"),
            (
                "getRequestProperty",
                "(Ljava/lang/String;)Ljava/lang/String;",
            ),
            (
                "setRequestProperty",
                "(Ljava/lang/String;Ljava/lang/String;)V",
            ),
            ("getResponseCode", "()I"),
            ("getResponseMessage", "()Ljava/lang/String;"),
            ("getExpiration", "()J"),
            ("getDate", "()J"),
            ("getLastModified", "()J"),
            ("getHeaderField", "(Ljava/lang/String;)Ljava/lang/String;"),
            ("getHeaderFieldInt", "(Ljava/lang/String;I)I"),
            ("getHeaderFieldDate", "(Ljava/lang/String;J)J"),
            ("getHeaderField", "(I)Ljava/lang/String;"),
            ("getHeaderFieldKey", "(I)Ljava/lang/String;"),
        ],
    )
    .string_constant("HEAD", "HEAD")
    .string_constant("GET", "GET")
    .string_constant("POST", "POST");
    for &(name, code) in HTTP_CODES {
        class = class.constant(name, "I", Value::Int(code));
    }
    class
}

fn socket_connection() -> NativeClass {
    let mut class = interface(
        SOCKET_CONNECTION,
        &[STREAM_CONNECTION],
        &[
            ("setSocketOption", "(BI)V"),
            ("getSocketOption", "(B)I"),
            ("getLocalAddress", "()Ljava/lang/String;"),
            ("getLocalPort", "()I"),
            ("getAddress", "()Ljava/lang/String;"),
            ("getPort", "()I"),
        ],
    );
    for (name, value) in [
        ("DELAY", 0),
        ("LINGER", 1),
        ("KEEPALIVE", 2),
        ("RCVBUF", 3),
        ("SNDBUF", 4),
    ] {
        class = class.constant(name, "B", Value::Int(value));
    }
    class
}

/// `ConnectionNotFoundException`, an `IOException` for unknown schemes
/// and targets that cannot be found.
fn not_found() -> NativeClass {
    const SUPER: &str = "java/io/IOException";
    NativeClass::new(NOT_FOUND)
        .extends(SUPER)
        .method("<init>", "()V", |ctx, a| {
            ctx.call_special(SUPER, this(a), "<init>", "()V", &[])?;
            Ok(Value::Void)
        })
        .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
            ctx.call_special(SUPER, this(a), "<init>", "(Ljava/lang/String;)V", &[a[1]])?;
            Ok(Value::Void)
        })
}

/// Every interface of the framework, and its one exception class.
pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        interface(CONNECTION, &[], &[("close", "()V")]),
        interface(
            INPUT_CONNECTION,
            &[CONNECTION],
            &[
                ("openInputStream", "()Ljava/io/InputStream;"),
                ("openDataInputStream", "()Ljava/io/DataInputStream;"),
            ],
        ),
        interface(
            OUTPUT_CONNECTION,
            &[CONNECTION],
            &[
                ("openOutputStream", "()Ljava/io/OutputStream;"),
                ("openDataOutputStream", "()Ljava/io/DataOutputStream;"),
            ],
        ),
        interface(
            STREAM_CONNECTION,
            &[INPUT_CONNECTION, OUTPUT_CONNECTION],
            &[],
        ),
        interface(
            CONTENT_CONNECTION,
            &[STREAM_CONNECTION],
            &[
                ("getType", "()Ljava/lang/String;"),
                ("getEncoding", "()Ljava/lang/String;"),
                ("getLength", "()J"),
            ],
        ),
        interface(
            NOTIFIER,
            &[CONNECTION],
            &[(
                "acceptAndOpen",
                "()Ljavax/microedition/io/StreamConnection;",
            )],
        ),
        interface(
            DATAGRAM_CONNECTION,
            &[CONNECTION],
            &[
                ("getMaximumLength", "()I"),
                ("getNominalLength", "()I"),
                ("send", "(Ljavax/microedition/io/Datagram;)V"),
                ("receive", "(Ljavax/microedition/io/Datagram;)V"),
                ("newDatagram", "(I)Ljavax/microedition/io/Datagram;"),
                (
                    "newDatagram",
                    "(ILjava/lang/String;)Ljavax/microedition/io/Datagram;",
                ),
                ("newDatagram", "([BI)Ljavax/microedition/io/Datagram;"),
                (
                    "newDatagram",
                    "([BILjava/lang/String;)Ljavax/microedition/io/Datagram;",
                ),
            ],
        ),
        interface(
            "javax/microedition/io/UDPDatagramConnection",
            &[DATAGRAM_CONNECTION],
            &[
                ("getLocalAddress", "()Ljava/lang/String;"),
                ("getLocalPort", "()I"),
            ],
        ),
        // Datagram reads and writes its buffer through java.io's DataInput
        // and DataOutput, which the CLDC library provides.
        interface(
            "javax/microedition/io/Datagram",
            &["java/io/DataInput", "java/io/DataOutput"],
            &[
                ("getAddress", "()Ljava/lang/String;"),
                ("getData", "()[B"),
                ("getLength", "()I"),
                ("getOffset", "()I"),
                ("setAddress", "(Ljava/lang/String;)V"),
                ("setAddress", "(Ljavax/microedition/io/Datagram;)V"),
                ("setLength", "(I)V"),
                ("setData", "([BII)V"),
                ("reset", "()V"),
            ],
        ),
        interface(
            "javax/microedition/io/CommConnection",
            &[STREAM_CONNECTION],
            &[("getBaudRate", "()I"), ("setBaudRate", "(I)I")],
        ),
        http_connection(),
        interface(
            "javax/microedition/io/HttpsConnection",
            &[HTTP_CONNECTION],
            &[
                ("getSecurityInfo", "()Ljavax/microedition/io/SecurityInfo;"),
                ("getPort", "()I"),
            ],
        ),
        socket_connection(),
        interface(
            "javax/microedition/io/SecureConnection",
            &[SOCKET_CONNECTION],
            &[("getSecurityInfo", "()Ljavax/microedition/io/SecurityInfo;")],
        ),
        interface(
            "javax/microedition/io/SecurityInfo",
            &[],
            &[
                (
                    "getServerCertificate",
                    "()Ljavax/microedition/pki/Certificate;",
                ),
                ("getProtocolVersion", "()Ljava/lang/String;"),
                ("getProtocolName", "()Ljava/lang/String;"),
                ("getCipherSuite", "()Ljava/lang/String;"),
            ],
        ),
        interface(
            "javax/microedition/io/ServerSocketConnection",
            &[NOTIFIER],
            &[
                ("getLocalAddress", "()Ljava/lang/String;"),
                ("getLocalPort", "()I"),
            ],
        ),
        not_found(),
    ]
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fastemu_jvm::{MemorySource, Vm, VmConfig};

    use super::*;

    #[test]
    fn the_constants_have_the_javadoc_values() {
        let vm = Vm::new(
            VmConfig::default(),
            Arc::new(MemorySource::new()),
            crate::classes(&fastemu_core::default_profile()),
        );
        vm.attach("test", |ctx| {
            let mut read = |class: &str, name: &str, desc: &str| {
                let field = ctx.static_field_id(class, name, desc).unwrap();
                ctx.get_static(field).unwrap()
            };
            assert_eq!(read(HTTP_CONNECTION, "HTTP_OK", "I").int(), 200);
            assert_eq!(read(HTTP_CONNECTION, "HTTP_VERSION", "I").int(), 505);
            assert_eq!(read(SOCKET_CONNECTION, "SNDBUF", "B").int(), 4);
            let get = read(HTTP_CONNECTION, "GET", "Ljava/lang/String;");
            let get = ctx.string(get.obj().unwrap()).unwrap();
            assert_eq!(get, "GET");
        });
    }

    #[test]
    fn connection_not_found_is_an_io_exception() {
        let vm = Vm::new(
            VmConfig::default(),
            Arc::new(MemorySource::new()),
            crate::classes(&fastemu_core::default_profile()),
        );
        vm.attach("test", |ctx| {
            let message = ctx.new_string("gone").unwrap();
            let e = ctx
                .construct(NOT_FOUND, "(Ljava/lang/String;)V", &[message.into()])
                .unwrap();
            let io = ctx.find_class("java/io/IOException").unwrap();
            assert!(ctx.is_instance(e, io));
            assert_eq!(
                ctx.describe_exception(e),
                "javax.microedition.io.ConnectionNotFoundException: gone"
            );
            for name in [CONNECTION, STREAM_CONNECTION, HTTP_CONNECTION] {
                ctx.find_class(name).unwrap();
            }
        });
    }
}
