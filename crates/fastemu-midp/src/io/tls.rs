//! TLS for `https://` and `ssl://`, and the `SecurityInfo` they report.
//!
//! rustls with the RustCrypto provider is pure Rust, so the same code
//! builds for every desktop target and for Android without a C
//! toolchain. It speaks TLS 1.2 and 1.3 and checks certificates against
//! Mozilla's root list (webpki-roots). The SSL 3.0 and TLS 1.0 servers
//! of the phones' era are long gone; the servers games still reach speak
//! TLS 1.2 or later.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Value};
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, ProtocolVersion, RootCertStore, StreamOwned};

use super::net::{self, CONNECT_TIMEOUT, Link};
use super::{lock, opt_string_value, this};

const SECURITY_INFO_IMPL: &str = "fastemu/io/SecurityInfoImpl";

/// Root certificates (DER) an instance trusts besides Mozilla's list. A
/// launcher installs it as a VM extension (`Ctx::set_extension`) for a
/// user who runs a game's server with a private certificate authority;
/// the tests install their own.
#[derive(Clone, Debug, Default)]
pub struct TrustedCertificates(pub Vec<Vec<u8>>);

/// The client configuration with Mozilla's roots and `extra`.
fn build_config(extra: &[Vec<u8>]) -> Result<Arc<ClientConfig>, String> {
    let mut roots = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    for der in extra {
        roots
            .add(CertificateDer::from(der.clone()))
            .map_err(|e| format!("a trusted certificate is not valid: {e}"))?;
    }
    let config = ClientConfig::builder_with_provider(Arc::new(rustls_rustcrypto::provider()))
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// The client configuration for this instance: the shared one, or one
/// with the instance's [`TrustedCertificates`].
fn config(ctx: &Ctx<'_>) -> Result<Arc<ClientConfig>, String> {
    static SHARED: OnceLock<Result<Arc<ClientConfig>, String>> = OnceLock::new();
    match ctx.extension::<TrustedCertificates>() {
        Some(extra) if !extra.0.is_empty() => build_config(&extra.0),
        _ => SHARED.get_or_init(|| build_config(&[])).clone(),
    }
}

/// What `SecurityInfo` reports about a connection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SecurityDetails {
    /// "3.3" for TLS 1.2 and "3.4" for TLS 1.3: the javadoc asks for the
    /// major and minor numbers of the protocol on the wire ("3.1" is TLS
    /// 1.0).
    version: String,
    /// The cipher suite's RFC name, such as `TLS_AES_128_GCM_SHA256`.
    cipher_suite: String,
}

/// The socket as rustls reads it. rustls reads again and again inside one
/// call while a handshake message or a record is incomplete, so a peer
/// that sends a byte within every slice would keep the call from ever
/// ending, and the caller would never look for a stop or its deadline.
/// A call takes in one read. Any other read in it says `WouldBlock`, as
/// a slice that brought nothing more does, and the caller tries again
/// with what rustls has kept.
struct Socket {
    tcp: TcpStream,
    /// This call has read already.
    read: bool,
}

impl Read for Socket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.read {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let n = self.tcp.read(buf)?;
        self.read = true;
        Ok(n)
    }
}

impl Write for Socket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tcp.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tcp.flush()
    }
}

/// A TLS client session over a socket. The stream is locked for each
/// bounded read or write, with the VM lock released.
pub(crate) struct Session {
    stream: Mutex<StreamOwned<ClientConnection, Socket>>,
    /// What [`Session::details`] reports, taken once the handshake is
    /// done. Natives ask for it with the VM lock held, and locking the
    /// stream then could wait for another thread's read slice.
    details: OnceLock<SecurityDetails>,
}

impl Session {
    /// One bounded step of the handshake: `Ok(true)` once it is done.
    pub(crate) fn handshake(&self) -> io::Result<bool> {
        let mut guard = lock(&self.stream);
        guard.sock.read = false;
        let StreamOwned { conn, sock } = &mut *guard;
        if conn.is_handshaking() {
            conn.complete_io(sock)?;
        }
        Ok(!conn.is_handshaking())
    }

    /// One bounded read of plaintext. A peer that closes the socket
    /// without TLS's close message ends the stream, as browsers accept.
    pub(crate) fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut guard = lock(&self.stream);
        guard.sock.read = false;
        match guard.read(buf) {
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(0),
            other => other,
        }
    }

    /// Takes `buf` into the session. Records that earlier writes left
    /// unsent go out first, so a timeout here has taken nothing; what this
    /// write cannot send at once goes out with the next write or flush.
    pub(crate) fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = lock(&self.stream);
        let StreamOwned { conn, sock } = &mut *guard;
        while conn.wants_write() {
            conn.write_tls(sock)?;
        }
        let n = conn.writer().write(buf)?;
        while conn.wants_write() {
            if conn.write_tls(sock).is_err() {
                break;
            }
        }
        Ok(n)
    }

    /// Sends the records earlier writes left unsent.
    pub(crate) fn flush(&self) -> io::Result<()> {
        let mut guard = lock(&self.stream);
        let StreamOwned { conn, sock } = &mut *guard;
        while conn.wants_write() {
            conn.write_tls(sock)?;
        }
        Ok(())
    }

    /// How much plaintext a read would return at once: what the session
    /// holds after taking in what has arrived on `socket`, without
    /// waiting. Another thread inside a read makes this 0.
    pub(crate) fn available(&self, socket: &TcpStream) -> usize {
        let Ok(mut guard) = self.stream.try_lock() else {
            return 0;
        };
        let StreamOwned { conn, sock } = &mut *guard;
        if socket.set_nonblocking(true).is_ok() {
            while conn.wants_read() {
                if !matches!(conn.read_tls(&mut sock.tcp), Ok(n) if n > 0) {
                    break;
                }
            }
            let _ = socket.set_nonblocking(false);
        }
        conn.process_new_packets()
            .map_or(0, |state| state.plaintext_bytes_to_read())
    }

    /// The protocol version and cipher suite, once the handshake is done.
    /// [`connect`] takes them while no other thread uses the stream.
    pub(crate) fn details(&self) -> SecurityDetails {
        self.details
            .get_or_init(|| Self::read_details(&lock(&self.stream)))
            .clone()
    }

    fn read_details(guard: &StreamOwned<ClientConnection, Socket>) -> SecurityDetails {
        let version = match guard.conn.protocol_version() {
            Some(ProtocolVersion::TLSv1_3) => "3.4",
            Some(ProtocolVersion::TLSv1_2) => "3.3",
            _ => "",
        };
        let suite = guard
            .conn
            .negotiated_cipher_suite()
            .and_then(|s| s.suite().as_str())
            .unwrap_or_default();
        // rustls names TLS 1.3 suites TLS13_*; RFC 8446 names them TLS_*.
        let cipher_suite = match suite.strip_prefix("TLS13_") {
            Some(rest) => format!("TLS_{rest}"),
            None => suite.to_owned(),
        };
        SecurityDetails {
            version: version.to_owned(),
            cipher_suite,
        }
    }
}

/// Connects to `host:port` and completes a TLS handshake that checks the
/// server's certificate for `host`.
pub(crate) fn connect(ctx: &mut Ctx<'_>, host: &str, port: u16) -> JResult<Link> {
    let config = config(ctx).map_err(|why| net::io_exception(ctx, &io::Error::other(why)))?;
    let Ok(name) = ServerName::try_from(host.to_owned()) else {
        let message = format!("{host} is not a valid server name");
        return Err(net::io_exception(ctx, &io::Error::other(message)));
    };
    let socket = net::connect(ctx, host, port)?;
    let session = ClientConnection::new(config, name)
        .map_err(io::Error::other)
        .and_then(|conn| {
            let tcp = socket.try_clone()?;
            Ok(StreamOwned::new(conn, Socket { tcp, read: false }))
        });
    let session = match session {
        Ok(stream) => Session {
            stream: Mutex::new(stream),
            details: OnceLock::new(),
        },
        Err(e) => return Err(net::io_exception(ctx, &e)),
    };
    let link = Link::secure(socket, session);
    net::block_on(ctx, Some(Instant::now() + CONNECT_TIMEOUT), || {
        match link.handshake() {
            Ok(true) => Ok(Some(())),
            Ok(false) => Ok(None),
            Err(e) if e.kind() == io::ErrorKind::InvalidData => Err(io::Error::other(format!(
                "TLS handshake with {host} failed: {e}"
            ))),
            Err(e) => net::again(Err(e)),
        }
    })?;
    // Take the details now, while no other thread reads the stream.
    let _ = link.security();
    Ok(link)
}

/// The payload of a `SecurityInfo` object.
#[derive(Default)]
struct InfoPayload(SecurityDetails);

impl Payload for InfoPayload {}

/// A new `SecurityInfo` object reporting `details`.
pub(crate) fn security_info(ctx: &mut Ctx<'_>, details: SecurityDetails) -> JResult<ObjRef> {
    let class = ctx.find_class(SECURITY_INFO_IMPL)?;
    let info = ctx.new_object(class)?;
    ctx.set_payload(info, InfoPayload(details));
    Ok(info)
}

/// The `SecurityInfo` class. `getServerCertificate` returns `null`:
/// FastEmu has no `javax.microedition.pki` certificates yet.
pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new(SECURITY_INFO_IMPL)
            .implements("javax/microedition/io/SecurityInfo")
            .payload::<InfoPayload>()
            .method(
                "getServerCertificate",
                "()Ljavax/microedition/pki/Certificate;",
                |_, _| Ok(Value::Ref(None)),
            )
            .method("getProtocolVersion", "()Ljava/lang/String;", |ctx, a| {
                let version = ctx.payload::<InfoPayload>(this(a))?.0.version.clone();
                opt_string_value(ctx, Some(&version))
            })
            .method("getProtocolName", "()Ljava/lang/String;", |ctx, _| {
                opt_string_value(ctx, Some("TLS"))
            })
            .method("getCipherSuite", "()Ljava/lang/String;", |ctx, a| {
                let suite = ctx.payload::<InfoPayload>(this(a))?.0.cipher_suite.clone();
                opt_string_value(ctx, Some(&suite))
            }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shared_configuration_builds_with_mozillas_roots() {
        let config = build_config(&[]).unwrap();
        assert!(config.alpn_protocols.is_empty());
        assert!(build_config(&[vec![1, 2, 3]]).is_err());
    }
}
