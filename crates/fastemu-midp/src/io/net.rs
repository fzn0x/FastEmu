//! Blocking network work, done with the VM lock released and in short
//! slices, so a stopping VM, an interrupt or a timeout ends it promptly.
//!
//! Every socket waits at most [`SLICE`] per system call. [`block_on`]
//! repeats a bounded attempt with the lock released, and between
//! attempts checks whether the VM is stopping, the thread was interrupted
//! or the deadline passed. Name lookups and connects, which std::net
//! cannot bound that way, run on a helper thread ([`off_thread`]).

use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use fastemu_core::NetworkPolicy;
use fastemu_jvm::{Ctx, JResult, Throw};

use crate::env;

/// The longest one system call on a connection blocks with the VM lock
/// released before the thread checks for a stop, an interrupt or its
/// deadline.
pub(crate) const SLICE: Duration = Duration::from_millis(100);

/// How long resolving a name and connecting may take.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// How long a read or write may wait for the peer when the game opened
/// the connection with `timeouts` set.
pub(crate) const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// The message of the `IOException` every network open throws when the
/// instance's network policy is `Deny`.
pub(crate) const NETWORK_OFF: &str = "network access is off for this game";

/// Throws `IOException` unless this instance may use the network. A VM
/// without an [`env::InstanceEnv`] has no network either.
pub(crate) fn check_network(ctx: &mut Ctx<'_>) -> JResult<()> {
    let allowed = env::try_env(ctx).is_some_and(|e| e.settings.network == NetworkPolicy::Allow);
    if allowed {
        Ok(())
    } else {
        Err(ctx.new_exception("java/io/IOException", Some(NETWORK_OFF)))
    }
}

/// The deadline `timeout` from now, if any.
pub(crate) fn deadline(timeout: Option<Duration>) -> Option<Instant> {
    timeout.map(|t| Instant::now() + t)
}

/// Turns the timeout of one bounded attempt into "not yet".
pub(crate) fn again<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(e) if is_retry(&e) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Whether `e` only means the attempt ran out of time or was interrupted
/// by a signal.
fn is_retry(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
    )
}

/// Runs `attempt` with the VM lock released until it gives a value.
///
/// `attempt` makes one bounded try (one system call that waits at most
/// [`SLICE`]) and returns `Ok(None)` when it has nothing yet. Between
/// tries this checks, with the lock held, whether the VM is stopping
/// (`Throw::Stop`), whether the thread was interrupted or `deadline`
/// passed (`InterruptedIOException`). An I/O error becomes an
/// `IOException` ([`io_exception`]).
pub(crate) fn block_on<T>(
    ctx: &mut Ctx<'_>,
    deadline: Option<Instant>,
    mut attempt: impl FnMut() -> io::Result<Option<T>>,
) -> JResult<T> {
    loop {
        match ctx.without_lock(&mut attempt) {
            Ok(Some(v)) => return Ok(v),
            Ok(None) => {}
            Err(e) => return Err(io_exception(ctx, &e)),
        }
        if ctx.is_stopping() {
            return Err(Throw::Stop);
        }
        if ctx.take_interrupt() {
            return Err(interrupted(ctx, "interrupted"));
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            return Err(interrupted(ctx, "timed out"));
        }
    }
}

/// An `InterruptedIOException` with `message`.
pub(crate) fn interrupted(ctx: &mut Ctx<'_>, message: &str) -> Throw {
    ctx.new_exception("java/io/InterruptedIOException", Some(message))
}

/// The Java exception for an I/O error: `InterruptedIOException` for a
/// timeout, `IOException` with a short message otherwise. The operating
/// system's own text differs between hosts and leaks them, so the common
/// kinds get fixed messages.
pub(crate) fn io_exception(ctx: &mut Ctx<'_>, e: &io::Error) -> Throw {
    let message = match e.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => {
            return interrupted(ctx, "timed out");
        }
        io::ErrorKind::ConnectionRefused => "connection refused".to_owned(),
        io::ErrorKind::ConnectionReset => "connection reset by the peer".to_owned(),
        io::ErrorKind::ConnectionAborted => "connection aborted".to_owned(),
        io::ErrorKind::BrokenPipe => "connection closed by the peer".to_owned(),
        io::ErrorKind::NotConnected => "not connected".to_owned(),
        io::ErrorKind::AddrInUse => "address already in use".to_owned(),
        io::ErrorKind::AddrNotAvailable => "address not available".to_owned(),
        io::ErrorKind::UnexpectedEof => "unexpected end of stream".to_owned(),
        _ => e.to_string(),
    };
    ctx.new_exception("java/io/IOException", Some(&message))
}

/// Runs `work` on a helper thread and waits for its result with the VM
/// lock released, for blocking calls std::net cannot bound (name lookups
/// and connects). If the VM stops or `deadline` passes first, the helper
/// is left to finish on its own and its result is dropped.
pub(crate) fn off_thread<T: Send + 'static>(
    ctx: &mut Ctx<'_>,
    deadline: Instant,
    what: &str,
    work: impl FnOnce() -> T + Send + 'static,
) -> JResult<T> {
    let (tx, rx) = mpsc::sync_channel(1);
    let spawned = std::thread::Builder::new()
        .name(format!("fastemu-{what}"))
        .spawn(move || {
            let _ = tx.send(work());
        });
    if spawned.is_err() {
        let message = format!("cannot start a thread to {what}");
        return Err(ctx.new_exception("java/io/IOException", Some(&message)));
    }
    block_on(ctx, Some(deadline), || match rx.recv_timeout(SLICE) {
        Ok(v) => Ok(Some(v)),
        Err(RecvTimeoutError::Timeout) => Ok(None),
        Err(RecvTimeoutError::Disconnected) => Err(io::Error::other(format!("{what} failed"))),
    })
}

/// Why a lookup or connect failed.
enum ConnectError {
    /// The name has no address.
    Unknown,
    /// Every address failed; the last error.
    Io(io::Error),
}

/// Connects to `host:port`, trying each of its addresses, and sets the
/// slice timeouts on the new socket.
pub(crate) fn connect(ctx: &mut Ctx<'_>, host: &str, port: u16) -> JResult<TcpStream> {
    let name = host.to_owned();
    let result = off_thread(
        ctx,
        Instant::now() + CONNECT_TIMEOUT,
        "connect",
        move || {
            let addrs: Vec<SocketAddr> = match (name.as_str(), port).to_socket_addrs() {
                Ok(a) => a.collect(),
                Err(_) => return Err(ConnectError::Unknown),
            };
            let mut last = None;
            for addr in addrs {
                match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
                    Ok(s) => return Ok(s),
                    Err(e) => last = Some(e),
                }
            }
            Err(last.map_or(ConnectError::Unknown, ConnectError::Io))
        },
    )?;
    let stream = match result {
        Ok(s) => s,
        Err(ConnectError::Unknown) => {
            let message = format!("cannot find host {host}");
            return Err(ctx.new_exception(super::NOT_FOUND, Some(&message)));
        }
        Err(ConnectError::Io(e)) => return Err(io_exception(ctx, &e)),
    };
    set_slices(&stream).map_err(|e| io_exception(ctx, &e))?;
    Ok(stream)
}

/// Bounds every read and write on `stream` to one [`SLICE`].
pub(crate) fn set_slices(stream: &TcpStream) -> io::Result<()> {
    stream.set_read_timeout(Some(SLICE))?;
    stream.set_write_timeout(Some(SLICE))
}

/// A connected TCP socket: what HTTP and socket connections read and
/// write. `&Link` reads and writes, so one thread may read while another
/// writes.
pub(crate) struct Link {
    socket: TcpStream,
}

impl Link {
    /// A link over the connected `socket`.
    pub(crate) fn plain(socket: TcpStream) -> Self {
        Self { socket }
    }

    /// The socket, for its addresses and options.
    pub(crate) fn socket(&self) -> &TcpStream {
        &self.socket
    }

    /// How many bytes a read would return at once, without waiting.
    ///
    /// Games poll `available()` in their main loop to read only what has
    /// arrived, so this must be real. std::net has no `FIONREAD`, so the
    /// socket peeks with blocking switched off for a moment. A read or
    /// write that another thread starts meanwhile sees `WouldBlock`,
    /// which every caller here treats as "try again".
    pub(crate) fn available(&self) -> usize {
        let mut probe = [0; 16 * 1024];
        if self.socket.set_nonblocking(true).is_err() {
            return 0;
        }
        let n = self.socket.peek(&mut probe).unwrap_or(0);
        let _ = self.socket.set_nonblocking(false);
        n
    }

    /// One bounded read; `Ok(0)` at the end of the stream.
    pub(crate) fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        (&self.socket).read(buf)
    }

    /// One bounded write; returns how much it took.
    pub(crate) fn write(&self, buf: &[u8]) -> io::Result<usize> {
        (&self.socket).write(buf)
    }

    /// Pushes out what earlier writes left buffered.
    pub(crate) fn flush(&self) -> io::Result<()> {
        (&self.socket).flush()
    }

    /// Closes both directions, which also wakes a thread blocked in
    /// [`Link::read`] on this socket.
    pub(crate) fn shutdown(&self) {
        let _ = self.socket.shutdown(Shutdown::Both);
    }
}

/// Writes all of `bytes` to `link`, blocking with the lock released.
pub(crate) fn write_all(
    ctx: &mut Ctx<'_>,
    link: &Link,
    bytes: &[u8],
    deadline: Option<Instant>,
) -> JResult<()> {
    let mut sent = 0;
    while sent < bytes.len() {
        let rest = &bytes[sent..];
        let n = block_on(ctx, deadline, || match again(link.write(rest))? {
            Some(0) => Err(io::ErrorKind::WriteZero.into()),
            other => Ok(other),
        })?;
        sent += n;
    }
    Ok(())
}
