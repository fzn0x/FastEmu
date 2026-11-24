//! `socket://`: `SocketConnection` (a TCP client) and
//! `ServerSocketConnection` (a TCP server), over `std::net`, and
//! `ssl://`: `SecureConnection`, a client socket with TLS.
//!
//! A connection hands out one input and one output stream, as the MIDP
//! reference implementation did. Closing the connection while a stream
//! is open keeps the socket open until the streams close too, as the
//! `Connection.close` javadoc says.

use std::io;
use std::net::{IpAddr, TcpListener, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Value};
use socket2::SockRef;

use super::net::{self, IO_TIMEOUT, Link};
use super::streams::{self, Input, Output};
use super::{Options, lock, this, throw_io, tls, url};

const SOCKET_IMPL: &str = "fastemu/io/SocketConnectionImpl";
const SECURE_IMPL: &str = "fastemu/io/SecureConnectionImpl";
const SERVER_IMPL: &str = "fastemu/io/ServerSocketConnectionImpl";

/// How often a waiting `acceptAndOpen` looks for a new connection.
const ACCEPT_POLL: Duration = Duration::from_millis(50);

/// What a connection and its streams have open.
#[derive(Default)]
struct Holds {
    closed: bool,
    input_opened: bool,
    output_opened: bool,
    open_streams: u32,
}

/// One TCP connection, shared by the connection object and its streams.
struct SocketShared {
    link: Link,
    options: Options,
    holds: Mutex<Holds>,
}

impl SocketShared {
    /// A connection over `link`, opened with `options`.
    fn new(link: Link, options: Options) -> Self {
        Self {
            link,
            options,
            holds: Mutex::new(Holds::default()),
        }
    }

    /// How long one read or write may wait for the peer.
    fn timeout(&self) -> Option<Duration> {
        self.options.timeouts.then_some(IO_TIMEOUT)
    }

    /// Shuts the socket once the connection and its streams are closed.
    fn release_if_done(&self, holds: &Holds) {
        if holds.closed && holds.open_streams == 0 {
            self.link.shutdown();
        }
    }
}

/// The payload of a socket connection object.
#[derive(Default)]
struct SocketPayload(Option<Arc<SocketShared>>);

impl Payload for SocketPayload {}

/// The socket of connection `conn`, or `IOException` once it is closed.
fn open_socket(ctx: &mut Ctx<'_>, conn: ObjRef) -> JResult<Arc<SocketShared>> {
    let shared = match &ctx.payload::<SocketPayload>(conn)?.0 {
        Some(s) => Arc::clone(s),
        None => return throw_io(ctx, "the connection was not opened with Connector.open"),
    };
    if lock(&shared.holds).closed {
        return throw_io(ctx, "connection is closed");
    }
    Ok(shared)
}

/// A new connection object of class `class` over `shared`.
fn new_connection(ctx: &mut Ctx<'_>, class: &str, shared: SocketShared) -> JResult<ObjRef> {
    let class = ctx.find_class(class)?;
    let conn = ctx.new_object(class)?;
    ctx.set_payload(conn, SocketPayload(Some(Arc::new(shared))));
    Ok(conn)
}

/// Opens `socket://host:port` (a client) or `socket://:port` (a server).
pub(crate) fn open(ctx: &mut Ctx<'_>, rest: &str, name: &str, options: Options) -> JResult<ObjRef> {
    let target = match url::host_port(rest) {
        Ok(t) => t,
        Err(message) => return ctx.throw("java/lang/IllegalArgumentException", &message),
    };
    let Some(host) = target.host else {
        net::check_network(ctx)?;
        return open_server(ctx, target.port.unwrap_or(0), options);
    };
    let Some(port) = target.port else {
        let message = format!("no port in {name}");
        return ctx.throw("java/lang/IllegalArgumentException", &message);
    };
    net::check_network(ctx)?;
    let socket = net::connect(ctx, &host, port)?;
    new_connection(
        ctx,
        SOCKET_IMPL,
        SocketShared::new(Link::plain(socket), options),
    )
}

/// Opens `ssl://host:port`: a client socket with TLS on it.
pub(crate) fn open_secure(
    ctx: &mut Ctx<'_>,
    rest: &str,
    name: &str,
    options: Options,
) -> JResult<ObjRef> {
    let target = match url::host_port(rest) {
        Ok(t) => t,
        Err(message) => return ctx.throw("java/lang/IllegalArgumentException", &message),
    };
    let (Some(host), Some(port)) = (target.host, target.port) else {
        let message = format!("{name} needs a host and a port");
        return ctx.throw("java/lang/IllegalArgumentException", &message);
    };
    net::check_network(ctx)?;
    let link = tls::connect(ctx, &host, port)?;
    new_connection(ctx, SECURE_IMPL, SocketShared::new(link, options))
}

/// What reads from a socket connection's input stream.
struct SocketInput {
    shared: Arc<SocketShared>,
    closed: AtomicBool,
}

impl Input for SocketInput {
    /// Returns what has arrived, at least one byte: socket protocols send
    /// a request and wait for the answer, so a read must not wait to fill
    /// its buffer.
    fn read(&self, ctx: &mut Ctx<'_>, buf: &mut [u8]) -> JResult<usize> {
        let link = &self.shared.link;
        net::block_on(ctx, net::deadline(self.shared.timeout()), || {
            if self.closed.load(Ordering::Acquire) {
                return Err(io::Error::other("stream closed"));
            }
            net::again(link.read(buf))
        })
    }

    fn available(&self) -> usize {
        self.shared.link.available()
    }

    fn close(&self, _ctx: &mut Ctx<'_>) -> JResult<()> {
        if !self.closed.swap(true, Ordering::AcqRel) {
            let mut holds = lock(&self.shared.holds);
            holds.open_streams -= 1;
            self.shared.release_if_done(&holds);
        }
        Ok(())
    }
}

/// What writes to a socket connection's output stream.
struct SocketOutput {
    shared: Arc<SocketShared>,
    closed: AtomicBool,
}

impl Output for SocketOutput {
    fn write(&self, ctx: &mut Ctx<'_>, bytes: &[u8]) -> JResult<()> {
        let deadline = net::deadline(self.shared.timeout());
        net::write_all(ctx, &self.shared.link, bytes, deadline)
    }

    fn flush(&self, ctx: &mut Ctx<'_>) -> JResult<()> {
        let link = &self.shared.link;
        net::block_on(ctx, net::deadline(self.shared.timeout()), || {
            net::again(link.flush())
        })
    }

    fn close(&self, ctx: &mut Ctx<'_>) -> JResult<()> {
        if self.closed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        let flushed = self.flush(ctx);
        let mut holds = lock(&self.shared.holds);
        holds.open_streams -= 1;
        self.shared.release_if_done(&holds);
        flushed
    }
}

fn open_input(ctx: &mut Ctx<'_>, conn: ObjRef) -> JResult<ObjRef> {
    let shared = open_socket(ctx, conn)?;
    let refusal = {
        let mut holds = lock(&shared.holds);
        if !shared.options.can_read() {
            Some("the connection was not opened for reading")
        } else if holds.input_opened {
            Some("the input stream is already open")
        } else {
            holds.input_opened = true;
            holds.open_streams += 1;
            None
        }
    };
    if let Some(message) = refusal {
        return throw_io(ctx, message);
    }
    let input = SocketInput {
        shared,
        closed: AtomicBool::new(false),
    };
    streams::new_input(ctx, Arc::new(input))
}

fn open_output(ctx: &mut Ctx<'_>, conn: ObjRef) -> JResult<ObjRef> {
    let shared = open_socket(ctx, conn)?;
    let refusal = {
        let mut holds = lock(&shared.holds);
        if !shared.options.can_write() {
            Some("the connection was not opened for writing")
        } else if holds.output_opened {
            Some("the output stream is already open")
        } else {
            holds.output_opened = true;
            holds.open_streams += 1;
            None
        }
    };
    if let Some(message) = refusal {
        return throw_io(ctx, message);
    }
    let output = SocketOutput {
        shared,
        closed: AtomicBool::new(false),
    };
    streams::new_output(ctx, Arc::new(output))
}

/// The `SocketConnection` option codes, `DELAY` to `SNDBUF`.
const DELAY: i32 = 0;
const LINGER: i32 = 1;
const KEEPALIVE: i32 = 2;
const RCVBUF: i32 = 3;
const SNDBUF: i32 = 4;

fn check_option(ctx: &mut Ctx<'_>, option: i32) -> JResult<()> {
    if (DELAY..=SNDBUF).contains(&option) {
        Ok(())
    } else {
        let message = format!("unknown socket option {option}");
        ctx.throw("java/lang/IllegalArgumentException", &message)
    }
}

/// `setSocketOption`. `DELAY` 0 turns Nagle's algorithm off, as the
/// javadoc's "small buffer writing delay" means; `LINGER` is in seconds,
/// 0 for none.
fn set_option(socket: &SockRef<'_>, option: i32, value: i32) -> io::Result<()> {
    let value_usize = value as usize;
    match option {
        DELAY => socket.set_tcp_nodelay(value == 0),
        LINGER => socket.set_linger((value > 0).then(|| Duration::from_secs(value as u64))),
        KEEPALIVE => socket.set_keepalive(value != 0),
        RCVBUF => socket.set_recv_buffer_size(value_usize),
        _ => socket.set_send_buffer_size(value_usize),
    }
}

/// `getSocketOption`, -1 when the system cannot tell, as the javadoc
/// allows.
fn get_option(socket: &SockRef<'_>, option: i32) -> i32 {
    let value = match option {
        DELAY => socket.tcp_nodelay().map(|nodelay| usize::from(!nodelay)),
        LINGER => socket
            .linger()
            .map(|l| l.map_or(0, |d| d.as_secs() as usize)),
        KEEPALIVE => socket.keepalive().map(usize::from),
        RCVBUF => socket.recv_buffer_size(),
        _ => socket.send_buffer_size(),
    };
    value.map_or(-1, |v| i32::try_from(v).unwrap_or(i32::MAX))
}

/// The address `ip` as `getLocalAddress` reports it. An unspecified
/// address (a server bound to every interface) becomes the address the
/// host would use to reach the network, as a phone reported its own IP.
fn report_address(ip: IpAddr) -> String {
    if !ip.is_unspecified() {
        return ip.to_string();
    }
    // Connecting a UDP socket sends nothing; it only picks a route.
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("192.0.2.1:9")?;
            s.local_addr()
        })
        .map_or_else(|_| "127.0.0.1".to_owned(), |a| a.ip().to_string())
}

/// The members of a client socket connection, for this class and for
/// the secure one built on it.
fn client_class(name: &str, interface: &str) -> NativeClass {
    NativeClass::new(name)
        .implements(interface)
        .payload::<SocketPayload>()
        .method("close", "()V", |ctx, a| {
            if let Some(shared) = ctx.payload::<SocketPayload>(this(a))?.0.clone() {
                let mut holds = lock(&shared.holds);
                holds.closed = true;
                shared.release_if_done(&holds);
            }
            Ok(Value::Void)
        })
        .method("openInputStream", "()Ljava/io/InputStream;", |ctx, a| {
            Ok(open_input(ctx, this(a))?.into())
        })
        .method(
            "openDataInputStream",
            "()Ljava/io/DataInputStream;",
            |ctx, a| {
                let stream = open_input(ctx, this(a))?;
                Ok(streams::data_input(ctx, stream)?.into())
            },
        )
        .method("openOutputStream", "()Ljava/io/OutputStream;", |ctx, a| {
            Ok(open_output(ctx, this(a))?.into())
        })
        .method(
            "openDataOutputStream",
            "()Ljava/io/DataOutputStream;",
            |ctx, a| {
                let stream = open_output(ctx, this(a))?;
                Ok(streams::data_output(ctx, stream)?.into())
            },
        )
        .method("setSocketOption", "(BI)V", |ctx, a| {
            let (option, value) = (a[1].int(), a[2].int());
            check_option(ctx, option)?;
            if value < 0 {
                let message = format!("negative value {value} for socket option {option}");
                return ctx.throw("java/lang/IllegalArgumentException", &message);
            }
            let shared = open_socket(ctx, this(a))?;
            match set_option(&SockRef::from(shared.link.socket()), option, value) {
                Ok(()) => Ok(Value::Void),
                Err(e) => Err(net::io_exception(ctx, &e)),
            }
        })
        .method("getSocketOption", "(B)I", |ctx, a| {
            let option = a[1].int();
            check_option(ctx, option)?;
            let shared = open_socket(ctx, this(a))?;
            Ok(Value::Int(get_option(
                &SockRef::from(shared.link.socket()),
                option,
            )))
        })
        .method("getLocalAddress", "()Ljava/lang/String;", |ctx, a| {
            let shared = open_socket(ctx, this(a))?;
            let address = shared.link.socket().local_addr();
            address_value(ctx, address.map(|a| report_address(a.ip())))
        })
        .method("getLocalPort", "()I", |ctx, a| {
            let shared = open_socket(ctx, this(a))?;
            let address = shared.link.socket().local_addr();
            port_value(ctx, address.map(|a| a.port()))
        })
        .method("getAddress", "()Ljava/lang/String;", |ctx, a| {
            let shared = open_socket(ctx, this(a))?;
            let address = shared.link.socket().peer_addr();
            address_value(ctx, address.map(|a| a.ip().to_string()))
        })
        .method("getPort", "()I", |ctx, a| {
            let shared = open_socket(ctx, this(a))?;
            let address = shared.link.socket().peer_addr();
            port_value(ctx, address.map(|a| a.port()))
        })
}

fn address_value(ctx: &mut Ctx<'_>, address: io::Result<String>) -> JResult<Value> {
    match address {
        Ok(text) => Ok(ctx.new_string(&text)?.into()),
        Err(e) => Err(net::io_exception(ctx, &e)),
    }
}

fn port_value(ctx: &mut Ctx<'_>, port: io::Result<u16>) -> JResult<Value> {
    match port {
        Ok(port) => Ok(Value::Int(i32::from(port))),
        Err(e) => Err(net::io_exception(ctx, &e)),
    }
}

/// A listening socket, shared with the threads waiting in
/// `acceptAndOpen`. `close` drops the listener, so the port is free for
/// the game to listen on again at once rather than when the collector
/// frees the connection object.
struct ServerShared {
    listener: Mutex<Option<TcpListener>>,
    options: Options,
}

impl ServerShared {
    /// The address the listener is bound to.
    fn local_addr(&self) -> io::Result<std::net::SocketAddr> {
        match &*lock(&self.listener) {
            Some(listener) => listener.local_addr(),
            None => Err(io::ErrorKind::NotConnected.into()),
        }
    }
}

/// The payload of a server socket connection object.
#[derive(Default)]
struct ServerPayload(Option<Arc<ServerShared>>);

impl Payload for ServerPayload {}

fn open_server(ctx: &mut Ctx<'_>, port: u16, options: Options) -> JResult<ObjRef> {
    let listener = match TcpListener::bind(("0.0.0.0", port)) {
        Ok(l) => l,
        Err(e) => return Err(net::io_exception(ctx, &e)),
    };
    // Accepting polls, so a stopping VM or a close ends the wait.
    if let Err(e) = listener.set_nonblocking(true) {
        return Err(net::io_exception(ctx, &e));
    }
    let class = ctx.find_class(SERVER_IMPL)?;
    let conn = ctx.new_object(class)?;
    let shared = ServerShared {
        listener: Mutex::new(Some(listener)),
        options,
    };
    ctx.set_payload(conn, ServerPayload(Some(Arc::new(shared))));
    Ok(conn)
}

fn server(ctx: &mut Ctx<'_>, conn: ObjRef) -> JResult<Arc<ServerShared>> {
    let shared = match &ctx.payload::<ServerPayload>(conn)?.0 {
        Some(s) => Arc::clone(s),
        None => return throw_io(ctx, "the connection was not opened with Connector.open"),
    };
    if lock(&shared.listener).is_none() {
        return throw_io(ctx, "connection is closed");
    }
    Ok(shared)
}

/// `acceptAndOpen`: waits for a client and returns its connection.
fn accept(ctx: &mut Ctx<'_>, conn: ObjRef) -> JResult<ObjRef> {
    let shared = server(ctx, conn)?;
    let deadline = net::deadline(shared.options.timeouts.then_some(IO_TIMEOUT));
    let socket = net::block_on(ctx, deadline, || {
        // The listener does not block, so `close` waits for this lock
        // only as long as one accept call takes.
        let accepted = match &*lock(&shared.listener) {
            Some(listener) => listener.accept(),
            None => return Err(io::Error::other("connection is closed")),
        };
        match accepted {
            Ok((socket, _)) => Ok(Some(socket)),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(ACCEPT_POLL);
                Ok(None)
            }
            Err(e) => Err(e),
        }
    })?;
    let ready = socket
        .set_nonblocking(false)
        .and_then(|()| net::set_slices(&socket));
    if let Err(e) = ready {
        return Err(net::io_exception(ctx, &e));
    }
    new_connection(
        ctx,
        SOCKET_IMPL,
        SocketShared::new(Link::plain(socket), shared.options),
    )
}

fn server_class() -> NativeClass {
    NativeClass::new(SERVER_IMPL)
        .implements("javax/microedition/io/ServerSocketConnection")
        .payload::<ServerPayload>()
        .method("close", "()V", |ctx, a| {
            if let Some(shared) = &ctx.payload::<ServerPayload>(this(a))?.0 {
                lock(&shared.listener).take();
            }
            Ok(Value::Void)
        })
        .method(
            "acceptAndOpen",
            "()Ljavax/microedition/io/StreamConnection;",
            |ctx, a| Ok(accept(ctx, this(a))?.into()),
        )
        .method("getLocalAddress", "()Ljava/lang/String;", |ctx, a| {
            let shared = server(ctx, this(a))?;
            let address = shared.local_addr();
            address_value(ctx, address.map(|a| report_address(a.ip())))
        })
        .method("getLocalPort", "()I", |ctx, a| {
            let shared = server(ctx, this(a))?;
            let address = shared.local_addr();
            port_value(ctx, address.map(|a| a.port()))
        })
}

/// `SecureConnection`: a socket connection with TLS, and its security
/// information.
fn secure_class() -> NativeClass {
    client_class(SECURE_IMPL, "javax/microedition/io/SecureConnection").method(
        "getSecurityInfo",
        "()Ljavax/microedition/io/SecurityInfo;",
        |ctx, a| {
            let shared = open_socket(ctx, this(a))?;
            let details = shared.link.security().unwrap_or_default();
            Ok(tls::security_info(ctx, details)?.into())
        },
    )
}

/// The classes of `socket://` and `ssl://`.
pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        client_class(SOCKET_IMPL, "javax/microedition/io/SocketConnection"),
        secure_class(),
        server_class(),
    ]
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fastemu_core::NetworkPolicy;
    use fastemu_jvm::{MemorySource, Vm, VmConfig};

    use super::*;
    use crate::env::InstanceEnv;
    use crate::io::connector::READ_WRITE;

    #[test]
    fn a_closed_server_socket_frees_its_port() {
        let vm = Vm::new(
            VmConfig::default(),
            Arc::new(MemorySource::new()),
            crate::classes(&fastemu_core::default_profile()),
        );
        vm.attach("test", |ctx| {
            let mut env = InstanceEnv::for_tests("unused");
            env.settings.network = NetworkPolicy::Allow;
            ctx.set_extension(env);
            let options = Options {
                mode: READ_WRITE,
                timeouts: false,
            };
            let first = open(ctx, "//:0", "socket://:0", options).unwrap();
            let port = ctx
                .call_interface(first, "getLocalPort", "()I", &[])
                .unwrap()
                .int();
            ctx.call_interface(first, "close", "()V", &[]).unwrap();
            let rest = format!("//:{port}");
            let again = open(ctx, &rest, &format!("socket:{rest}"), options);
            assert!(again.is_ok(), "port {port} is still taken");
        });
    }
}
