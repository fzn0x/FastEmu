//! Blocking network work, done with the VM lock released and in short
//! slices, so a stopping VM, an interrupt or a timeout ends it promptly.
//!
//! Every socket waits at most [`SLICE`] per system call. [`block_on`]
//! repeats a bounded attempt with the lock released, and between
//! attempts checks whether the VM is stopping, the thread was interrupted
//! or the deadline passed. Name lookups and connects, which std::net
//! cannot bound that way, run on a helper thread ([`off_thread`]), and a
//! game has at most [`MAX_HELPERS`] of those alive.

use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use fastemu_core::NetworkPolicy;
use fastemu_jvm::{Ctx, JResult, Throw};

use super::{lock, tls};
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

/// How many client connections a game may have open at once: sockets,
/// secure sockets, sockets its server accepted and HTTP connections. A
/// phone had a handful.
pub(crate) const MAX_CONNECTIONS: usize = 64;

/// How many server sockets a game may have open at once.
pub(crate) const MAX_SERVERS: usize = 8;

/// How many datagram connections a game may have open at once.
pub(crate) const MAX_DATAGRAMS: usize = 16;

/// How many bytes of request bodies a game's HTTP connections may hold
/// at once, before the requests go out.
pub(crate) const MAX_PENDING_BYTES: usize = 8 << 20;

/// How many helper threads of name lookups and connects a game may have
/// alive at once ([`off_thread`]). A helper the game stopped waiting for,
/// on an interrupt, at its deadline or at a stop, counts until it ends,
/// since its thread and its socket live until then. Each game thread
/// waits for one helper at a time, and a phone ran a handful of sockets.
pub(crate) const MAX_HELPERS: usize = 16;

/// The kinds of connection a game's count of open ones is kept for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A client socket, a secure socket, an accepted socket or an HTTP
    /// connection.
    Connection,
    /// A listening socket.
    Server,
    /// A UDP socket.
    Datagram,
}

impl Kind {
    fn index(self) -> usize {
        self as usize
    }

    fn limit(self) -> usize {
        match self {
            Kind::Connection => MAX_CONNECTIONS,
            Kind::Server => MAX_SERVERS,
            Kind::Datagram => MAX_DATAGRAMS,
        }
    }

    /// What the game reads when the limit is reached.
    fn refusal(self) -> &'static str {
        match self {
            Kind::Connection => "too many open connections",
            Kind::Server => "too many open server sockets",
            Kind::Datagram => "too many open datagram sockets",
        }
    }
}

/// What a game has open, shared by its [`Slot`]s, [`Pending`]s and
/// [`HelperPlace`]s.
#[derive(Default)]
struct Counts {
    open: [AtomicUsize; 3],
    pending: AtomicUsize,
    /// The helper threads alive ([`off_thread`]).
    helpers: Mutex<usize>,
    /// Told when a helper ends and gives its place back.
    helper_ended: Condvar,
}

impl Counts {
    /// Takes a place for a helper thread. A game that has [`MAX_HELPERS`]
    /// alive first waits up to `wait` for one of them to end, and gets
    /// `None` if none did.
    fn take_helper(self: &Arc<Self>, wait: Duration) -> Option<HelperPlace> {
        let mut alive = lock(&self.helpers);
        if *alive >= MAX_HELPERS {
            alive = self
                .helper_ended
                .wait_timeout(alive, wait)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        if *alive >= MAX_HELPERS {
            return None;
        }
        *alive += 1;
        Some(HelperPlace {
            counts: Arc::clone(self),
        })
    }

    /// Takes a place for a `kind` connection, if the game has one left.
    fn take(self: &Arc<Self>, kind: Kind) -> Option<Slot> {
        let counter = &self.open[kind.index()];
        counter
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < kind.limit()).then_some(n + 1)
            })
            .ok()?;
        Some(Slot {
            counts: Arc::clone(self),
            kind,
        })
    }
}

/// The counts of the instance, kept in the VM as an extension. They hold
/// no Java references.
#[derive(Default)]
struct ConnectionSlots(Arc<Counts>);

/// The counts of the instance `ctx` runs in, made on first use.
fn counts(ctx: &mut Ctx<'_>) -> Arc<Counts> {
    if let Some(slots) = ctx.extension::<ConnectionSlots>() {
        return Arc::clone(&slots.0);
    }
    let slots = ConnectionSlots::default();
    let counts = Arc::clone(&slots.0);
    ctx.set_extension(slots);
    counts
}

/// One of a game's open connections. It is given back when it is dropped,
/// so a connection keeps it until its socket is closed, and one the game
/// never closes gives it back when the collector frees the connection.
pub(crate) struct Slot {
    counts: Arc<Counts>,
    kind: Kind,
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.counts.open[self.kind.index()].fetch_sub(1, Ordering::AcqRel);
    }
}

/// A helper thread's place among the game's [`MAX_HELPERS`]. The helper
/// holds it until its work returns, so the place comes back when the
/// thread and any socket it opened go, whether or not the game still
/// waits for them.
struct HelperPlace {
    counts: Arc<Counts>,
}

impl Drop for HelperPlace {
    fn drop(&mut self) {
        *lock(&self.counts.helpers) -= 1;
        // Wakes one waiter, which takes the place unless a caller that did
        // not wait took it first. Any other waiter looks again after its
        // slice.
        self.counts.helper_ended.notify_one();
    }
}

/// Takes a place for a `kind` connection, or throws `IOException` when
/// the game holds all it may.
///
/// A game that holds its limit gets one collection and a second try. On a
/// phone the collector freed a connection nobody referenced, and games
/// rely on it: they open one for each request and close the stream but
/// not the connection, and nothing on the Java heap grows enough to start
/// a collection of its own. A game that really holds them is refused
/// again. Call it before the native makes any object it does not root.
pub(crate) fn acquire(ctx: &mut Ctx<'_>, kind: Kind) -> JResult<Slot> {
    let counts = counts(ctx);
    if let Some(slot) = counts.take(kind) {
        return Ok(slot);
    }
    ctx.gc();
    if let Some(slot) = counts.take(kind) {
        return Ok(slot);
    }
    super::throw_io(ctx, kind.refusal())
}

/// Request bytes a connection holds until its request goes out, counted
/// for the whole game. They are given back when it is dropped.
pub(crate) struct Pending {
    counts: Arc<Counts>,
    bytes: usize,
}

/// A new, empty count of pending request bytes.
pub(crate) fn pending(ctx: &mut Ctx<'_>) -> Pending {
    Pending {
        counts: counts(ctx),
        bytes: 0,
    }
}

impl Pending {
    /// Counts `more` bytes, unless the game would hold more than
    /// [`MAX_PENDING_BYTES`]; then nothing is counted.
    pub(crate) fn add(&mut self, more: usize) -> bool {
        let counted = self
            .counts
            .pending
            .try_update(Ordering::AcqRel, Ordering::Acquire, |held| {
                held.checked_add(more)
                    .filter(|total| *total <= MAX_PENDING_BYTES)
            })
            .is_ok();
        if counted {
            self.bytes += more;
        }
        counted
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.counts.pending.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

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

/// Checks, with the lock held, whether the VM is stopping (`Throw::Stop`),
/// whether the thread was interrupted or `deadline` passed
/// (`InterruptedIOException`). [`block_on`] does it between its tries; a
/// loop that calls it for each piece of a larger job does it between
/// pieces, because a peer that sends or takes a little at a time makes
/// every try succeed, and a loop of such tries would never get to it.
pub(crate) fn check(ctx: &mut Ctx<'_>, deadline: Option<Instant>) -> JResult<()> {
    if ctx.is_stopping() {
        return Err(Throw::Stop);
    }
    if ctx.take_interrupt() {
        return Err(interrupted(ctx, "interrupted"));
    }
    if deadline.is_some_and(|d| Instant::now() >= d) {
        return Err(interrupted(ctx, "timed out"));
    }
    Ok(())
}

/// Runs `attempt` with the VM lock released until it gives a value.
///
/// `attempt` makes one bounded try (one system call that waits at most
/// [`SLICE`]) and returns `Ok(None)` when it has nothing yet. Between
/// tries this [`check`]s, with the lock held. An I/O error becomes an
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
        check(ctx, deadline)?;
    }
}

/// An `InterruptedIOException` with `message`.
pub(crate) fn interrupted(ctx: &mut Ctx<'_>, message: &str) -> Throw {
    ctx.new_exception("java/io/InterruptedIOException", Some(message))
}

/// An error whose text FastEmu wrote for the game to read: the failures
/// of the HTTP wire format, a closed stream. [`io_exception`] passes the
/// text on, which it does for no error that came from the system.
pub(crate) fn failure(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

/// The Java exception for an I/O error: `InterruptedIOException` for a
/// timeout, `IOException` with a short message otherwise.
///
/// The operating system's own text differs between hosts, is in the
/// host's language and can name its paths, so the game never reads it.
/// The common kinds get fixed messages, an error made by [`failure`] or
/// by the TLS library keeps its text, and any other kind is "I/O error".
pub(crate) fn io_exception(ctx: &mut Ctx<'_>, e: &io::Error) -> Throw {
    let message = match e.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => {
            return interrupted(ctx, "timed out");
        }
        io::ErrorKind::ConnectionRefused => "connection refused",
        io::ErrorKind::ConnectionReset => "connection reset by the peer",
        io::ErrorKind::ConnectionAborted => "connection aborted",
        io::ErrorKind::BrokenPipe => "connection closed by the peer",
        io::ErrorKind::NotConnected => "not connected",
        io::ErrorKind::AddrInUse => "address already in use",
        io::ErrorKind::AddrNotAvailable => "address not available",
        io::ErrorKind::UnexpectedEof => "unexpected end of stream",
        io::ErrorKind::PermissionDenied => "permission denied",
        io::ErrorKind::HostUnreachable
        | io::ErrorKind::NetworkUnreachable
        | io::ErrorKind::NetworkDown => "network unreachable",
        // Only an error made from a text has one to give; a system error
        // has a code and no text of its own.
        _ if e.get_ref().is_some() => {
            return ctx.new_exception("java/io/IOException", Some(&e.to_string()));
        }
        _ => "I/O error",
    };
    ctx.new_exception("java/io/IOException", Some(message))
}

/// Runs `work` on a helper thread and waits for its result with the VM
/// lock released, for blocking calls std::net cannot bound (name lookups
/// and connects). If the VM stops, the thread is interrupted or
/// `deadline` passes first, the helper is left to finish on its own and
/// its result is dropped.
///
/// The helper takes one of the game's [`MAX_HELPERS`] places before it
/// starts and keeps it until `work` returns. A game that holds them all
/// waits for one within `deadline`, in slices with the same checks, as it
/// waits for a slow network, and starts no thread meanwhile. A stopping
/// VM starts none at all.
pub(crate) fn off_thread<T: Send + 'static>(
    ctx: &mut Ctx<'_>,
    deadline: Instant,
    what: &str,
    work: impl FnOnce() -> T + Send + 'static,
) -> JResult<T> {
    if ctx.is_stopping() {
        return Err(Throw::Stop);
    }
    let counts = counts(ctx);
    let place = block_on(ctx, Some(deadline), || Ok(counts.take_helper(SLICE)))?;
    // A place taken at once returns without the checks, and the VM may
    // have begun to stop meanwhile.
    if ctx.is_stopping() {
        return Err(Throw::Stop);
    }
    let (tx, rx) = mpsc::sync_channel(1);
    // A thread that cannot start drops the closure, and the place with
    // it.
    let spawned = std::thread::Builder::new()
        .name(format!("fastemu-{what}"))
        .spawn(move || {
            // When the game no longer waits, `send` hands the result back
            // and it is dropped here, a connect's socket with it, before
            // the place goes back.
            let _ = tx.send(work());
            drop(place);
        });
    if spawned.is_err() {
        let message = format!("cannot start a thread to {what}");
        return Err(ctx.new_exception("java/io/IOException", Some(&message)));
    }
    block_on(ctx, Some(deadline), || match rx.recv_timeout(SLICE) {
        Ok(v) => Ok(Some(v)),
        Err(RecvTimeoutError::Timeout) => Ok(None),
        Err(RecvTimeoutError::Disconnected) => Err(failure(format!("{what} failed"))),
    })
}

/// Why a lookup or connect failed.
enum ConnectError {
    /// The name has no address.
    Unknown,
    /// Every address failed; the last error.
    Io(io::Error),
}

/// The first address of `host:port`. An address literal needs no lookup;
/// a name is looked up on a helper thread. A name without addresses
/// throws `ConnectionNotFoundException`, as the CLDC javadoc says
/// `Connector.open` does when "the target of the name cannot be found".
pub(crate) fn resolve(ctx: &mut Ctx<'_>, host: &str, port: u16) -> JResult<SocketAddr> {
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return Ok(SocketAddr::new(ip, port));
    }
    let name = host.to_owned();
    let found = off_thread(
        ctx,
        Instant::now() + CONNECT_TIMEOUT,
        "resolve",
        move || {
            (name.as_str(), port)
                .to_socket_addrs()
                .ok()
                .and_then(|mut addrs| addrs.next())
        },
    )?;
    match found {
        Some(addr) => Ok(addr),
        None => {
            let message = format!("cannot find host {host}");
            Err(ctx.new_exception(super::NOT_FOUND, Some(&message)))
        }
    }
}

/// Tries each of `addrs` in turn with `attempt` until one connects. The
/// addresses share the time up to `deadline`: each gets what is left,
/// and none is tried once it has passed, so a helper the game stopped
/// waiting for ends by the game's deadline, not 30 s for each address.
fn connect_each<S>(
    addrs: impl IntoIterator<Item = SocketAddr>,
    deadline: Instant,
    mut attempt: impl FnMut(&SocketAddr, Duration) -> io::Result<S>,
) -> Result<S, ConnectError> {
    let mut last = None;
    for addr in addrs {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            last = Some(io::ErrorKind::TimedOut.into());
            break;
        }
        match attempt(&addr, left) {
            Ok(s) => return Ok(s),
            Err(e) => last = Some(e),
        }
    }
    Err(last.map_or(ConnectError::Unknown, ConnectError::Io))
}

/// Connects to `host:port`, trying each of its addresses, and sets the
/// slice timeouts on the new socket. The lookup, the wait for a helper's
/// place and every address share one [`CONNECT_TIMEOUT`].
pub(crate) fn connect(ctx: &mut Ctx<'_>, host: &str, port: u16) -> JResult<TcpStream> {
    let name = host.to_owned();
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    let result = off_thread(ctx, deadline, "connect", move || {
        let addrs: Vec<SocketAddr> = match (name.as_str(), port).to_socket_addrs() {
            Ok(a) => a.collect(),
            Err(_) => return Err(ConnectError::Unknown),
        };
        connect_each(addrs, deadline, TcpStream::connect_timeout)
    })?;
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

/// A connected TCP socket, with TLS on it or not: what HTTP and socket
/// connections read and write. `&Link` reads and writes, so one thread
/// may read while another writes.
pub(crate) struct Link {
    socket: TcpStream,
    tls: Option<tls::Session>,
}

impl Link {
    /// A link over the connected `socket`.
    pub(crate) fn plain(socket: TcpStream) -> Self {
        Self { socket, tls: None }
    }

    /// A link over `socket` that speaks TLS through `session`.
    pub(crate) fn secure(socket: TcpStream, session: tls::Session) -> Self {
        Self {
            socket,
            tls: Some(session),
        }
    }

    /// One bounded step of the TLS handshake: `Ok(true)` once it is done,
    /// at once for a plain link.
    pub(crate) fn handshake(&self) -> io::Result<bool> {
        self.tls.as_ref().map_or(Ok(true), tls::Session::handshake)
    }

    /// What `SecurityInfo` reports, for a TLS link.
    pub(crate) fn security(&self) -> Option<tls::SecurityDetails> {
        self.tls.as_ref().map(tls::Session::details)
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
        if let Some(session) = &self.tls {
            return session.available(&self.socket);
        }
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
        match &self.tls {
            Some(session) => session.read(buf),
            None => (&self.socket).read(buf),
        }
    }

    /// One bounded write; returns how much it took.
    pub(crate) fn write(&self, buf: &[u8]) -> io::Result<usize> {
        match &self.tls {
            Some(session) => session.write(buf),
            None => (&self.socket).write(buf),
        }
    }

    /// Pushes out what earlier writes left buffered.
    pub(crate) fn flush(&self) -> io::Result<()> {
        match &self.tls {
            Some(session) => session.flush(),
            None => (&self.socket).flush(),
        }
    }

    /// Closes both directions, which also wakes a thread blocked in
    /// [`Link::read`] on this socket.
    pub(crate) fn shutdown(&self) {
        let _ = self.socket.shutdown(Shutdown::Both);
    }
}

/// Writes all of `bytes` to `link`, blocking with the lock released. A
/// stopping VM ends it before it starts, and it is [`check`]ed between
/// the pieces a peer takes.
pub(crate) fn write_all(
    ctx: &mut Ctx<'_>,
    link: &Link,
    bytes: &[u8],
    deadline: Option<Instant>,
) -> JResult<()> {
    if ctx.is_stopping() {
        return Err(Throw::Stop);
    }
    let mut sent = 0;
    while sent < bytes.len() {
        if sent > 0 {
            check(ctx, deadline)?;
        }
        let rest = &bytes[sent..];
        let n = block_on(ctx, deadline, || match again(link.write(rest))? {
            Some(0) => Err(io::ErrorKind::WriteZero.into()),
            other => Ok(other),
        })?;
        sent += n;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fastemu_jvm::{MemorySource, Vm, VmConfig};

    use super::*;
    use crate::env::InstanceEnv;
    use crate::io::connector::{Options, READ_WRITE};

    /// Runs `f` on a VM with the MIDP classes whose instance may use the
    /// network.
    fn with_network(f: impl FnOnce(&mut Ctx<'_>)) {
        let vm = Vm::new(
            VmConfig::default(),
            Arc::new(MemorySource::new()),
            crate::classes(&fastemu_core::default_profile()),
        );
        vm.attach("test", |ctx| {
            let mut env = InstanceEnv::for_tests("unused");
            env.settings.network = NetworkPolicy::Allow;
            ctx.set_extension(env);
            f(ctx);
        });
    }

    /// The exception `io_exception` makes for `e`, as `class: message`.
    fn described(ctx: &mut Ctx<'_>, e: &io::Error) -> String {
        match io_exception(ctx, e) {
            Throw::Exception(exception) => ctx.describe_exception(exception),
            Throw::Stop => panic!("an I/O error does not stop the machine"),
        }
    }

    #[test]
    fn a_kind_without_a_fixed_text_reads_as_io_error() {
        with_network(|ctx| {
            for kind in [
                io::ErrorKind::InvalidInput,
                io::ErrorKind::OutOfMemory,
                io::ErrorKind::Unsupported,
                io::ErrorKind::Other,
            ] {
                assert_eq!(
                    described(ctx, &io::Error::from(kind)),
                    "java.io.IOException: I/O error",
                    "{kind:?}"
                );
            }
        });
    }

    #[test]
    fn the_text_of_a_system_error_does_not_reach_the_game() {
        with_network(|ctx| {
            // Whatever kind the system gives a code, its own text, which
            // ends in "(os error N)" and is in the host's language, stays
            // out of the exception.
            for code in [1, 2, 5, 13, 22, 32, 10013, 10022, 10048, 10051] {
                let e = io::Error::from_raw_os_error(code);
                let text = described(ctx, &e);
                assert!(text.starts_with("java.io.IOException: "), "{text}");
                assert!(!text.contains("os error"), "{code}: {text}");
                assert!(!text.contains(&e.to_string()), "{code}: {text}");
            }
        });
    }

    #[test]
    fn a_text_fastemu_wrote_reaches_the_game() {
        with_network(|ctx| {
            assert_eq!(
                described(ctx, &failure("the response ended 5 bytes early")),
                "java.io.IOException: the response ended 5 bytes early"
            );
            assert_eq!(
                described(ctx, &io::Error::from(io::ErrorKind::ConnectionRefused)),
                "java.io.IOException: connection refused"
            );
            assert_eq!(
                described(ctx, &io::Error::from(io::ErrorKind::TimedOut)),
                "java.io.InterruptedIOException: timed out"
            );
        });
    }

    #[test]
    fn each_kind_of_connection_has_its_own_limit_and_gives_a_place_back() {
        with_network(|ctx| {
            let counts = counts(ctx);
            for kind in [Kind::Connection, Kind::Server, Kind::Datagram] {
                let mut held: Vec<Slot> = (0..kind.limit())
                    .map(|_| counts.take(kind).expect("a place"))
                    .collect();
                assert!(counts.take(kind).is_none(), "{kind:?}");
                // The others have their own places.
                held.truncate(kind.limit() - 1);
                assert!(counts.take(kind).is_some(), "{kind:?}");
            }
            for kind in [Kind::Connection, Kind::Server, Kind::Datagram] {
                assert_eq!(counts.open[kind.index()].load(Ordering::Acquire), 0);
            }
        });
    }

    #[test]
    fn pending_request_bytes_are_counted_for_the_whole_game() {
        with_network(|ctx| {
            let mut first = pending(ctx);
            let mut second = pending(ctx);
            assert!(first.add(MAX_PENDING_BYTES - 10));
            assert!(!second.add(11));
            assert!(second.add(10));
            assert!(!second.add(1));
            drop(first);
            assert!(second.add(MAX_PENDING_BYTES - 10));
            assert!(!second.add(1));
            drop(second);
            assert_eq!(counts(ctx).pending.load(Ordering::Acquire), 0);
        });
    }

    #[test]
    fn a_collection_gives_back_the_places_of_connections_nobody_refers_to() {
        with_network(|ctx| {
            let options = Options {
                mode: READ_WRITE,
                timeouts: false,
            };
            let open = |ctx: &mut Ctx<'_>| {
                let rest = "//127.0.0.1:1/";
                super::super::http::open(ctx, "http", rest, "http://127.0.0.1:1/", options)
            };
            // A game that never closes its connections keeps opening them.
            for _ in 0..3 * MAX_CONNECTIONS {
                ctx.local_frame(|ctx| open(ctx).unwrap());
            }
            // One that keeps them is refused at its limit.
            let mut kept = 0;
            let refused = loop {
                match open(ctx) {
                    Ok(_) => kept += 1,
                    Err(Throw::Exception(e)) => break ctx.describe_exception(e),
                    Err(Throw::Stop) => panic!("stopped"),
                }
            };
            assert_eq!(kept, MAX_CONNECTIONS);
            assert_eq!(refused, "java.io.IOException: too many open connections");
        });
    }

    /// The gate of the helpers' works: each waits on it until the test
    /// drops the sender.
    fn gate() -> (mpsc::Sender<()>, Arc<std::sync::Mutex<mpsc::Receiver<()>>>) {
        let (open, wait) = mpsc::channel();
        (open, Arc::new(std::sync::Mutex::new(wait)))
    }

    /// What a call ended with, as `class: message`.
    fn outcome<T>(ctx: &mut Ctx<'_>, result: JResult<T>) -> String {
        match result {
            Ok(_) => "returned".to_owned(),
            Err(Throw::Exception(e)) => ctx.describe_exception(e),
            Err(Throw::Stop) => "stopped".to_owned(),
        }
    }

    /// Calls `off_thread` with a deadline that has passed, as an interrupt
    /// or a slow network leaves the game, so it stops waiting after one
    /// slice. The work tells `ran` it started, then waits for the gate.
    fn give_up(
        ctx: &mut Ctx<'_>,
        gate: &Arc<std::sync::Mutex<mpsc::Receiver<()>>>,
        ran: &mpsc::Sender<()>,
    ) -> String {
        let (gate, ran) = (Arc::clone(gate), ran.clone());
        let result = off_thread(ctx, Instant::now(), "test", move || {
            let _ = ran.send(());
            let _ = gate.lock().unwrap().recv();
        });
        outcome(ctx, result)
    }

    /// How many works told `started` they ran, counted once the channel
    /// closes: when every work that ran has ended and every one that never
    /// ran was dropped. A helper that never ends fails the test rather
    /// than hang it.
    fn count_started(started: &mpsc::Receiver<()>) -> usize {
        let mut count = 0;
        loop {
            match started.recv_timeout(Duration::from_secs(30)) {
                Ok(()) => count += 1,
                Err(RecvTimeoutError::Disconnected) => return count,
                Err(RecvTimeoutError::Timeout) => panic!("a helper still runs after {count}"),
            }
        }
    }

    #[test]
    fn a_game_keeps_at_most_16_helpers_it_stopped_waiting_for() {
        with_network(|ctx| {
            let (open, gate) = gate();
            let (ran, started) = mpsc::channel();
            for _ in 0..MAX_HELPERS + 4 {
                assert_eq!(
                    give_up(ctx, &gate, &ran),
                    "java.io.InterruptedIOException: timed out"
                );
            }
            drop(ran);
            drop(open);
            assert_eq!(count_started(&started), MAX_HELPERS);
        });
    }

    #[test]
    fn a_helper_that_ends_gives_its_place_back() {
        with_network(|ctx| {
            let (open, gate) = gate();
            let (ran, started) = mpsc::channel();
            for _ in 0..MAX_HELPERS {
                give_up(ctx, &gate, &ran);
            }
            // The game holds every place. Whether the helpers end before
            // the next call or while it waits, it gets one.
            let release = std::thread::spawn(move || drop(open));
            let deadline = Instant::now() + Duration::from_secs(30);
            for i in 0..MAX_HELPERS + 4 {
                assert_eq!(off_thread(ctx, deadline, "test", move || i).ok(), Some(i));
            }
            release.join().unwrap();
            drop(ran);
            assert_eq!(count_started(&started), MAX_HELPERS);
        });
    }

    #[test]
    fn a_helper_whose_work_panics_gives_its_place_back() {
        with_network(|ctx| {
            let (open, gate) = gate();
            let (ran, started) = mpsc::channel();
            for _ in 0..MAX_HELPERS - 1 {
                give_up(ctx, &gate, &ran);
            }
            let deadline = Instant::now() + Duration::from_secs(30);
            let result = off_thread(ctx, deadline, "test", || -> u32 {
                panic!("the work of a test helper panics")
            });
            assert_eq!(outcome(ctx, result), "java.io.IOException: test failed");
            // Only the place of the helper that panicked is free.
            assert_eq!(off_thread(ctx, deadline, "test", || 7).ok(), Some(7));
            drop(open);
            drop(ran);
            assert_eq!(count_started(&started), MAX_HELPERS - 1);
        });
    }

    #[test]
    fn an_interrupt_ends_the_wait_for_a_helper_place() {
        with_network(|ctx| {
            let (open, gate) = gate();
            let (ran, started) = mpsc::channel();
            for _ in 0..MAX_HELPERS {
                give_up(ctx, &gate, &ran);
            }
            // Thread.currentThread().interrupt(), as a game does.
            let me = ctx.current_thread();
            assert!(ctx.call_virtual(me, "interrupt", "()V", &[]).is_ok());
            let deadline = Instant::now() + Duration::from_secs(30);
            let late = ran.clone();
            let result = off_thread(ctx, deadline, "test", move || {
                let _ = late.send(());
            });
            assert_eq!(
                outcome(ctx, result),
                "java.io.InterruptedIOException: interrupted"
            );
            drop(open);
            drop(ran);
            assert_eq!(count_started(&started), MAX_HELPERS);
        });
    }

    #[test]
    fn a_pending_interrupt_does_not_keep_a_helper_from_starting() {
        with_network(|ctx| {
            let (ran, started) = mpsc::channel();
            let me = ctx.current_thread();
            assert!(ctx.call_virtual(me, "interrupt", "()V", &[]).is_ok());
            let deadline = Instant::now() + Duration::from_secs(30);
            let result = off_thread(ctx, deadline, "test", move || {
                let _ = ran.send(());
            });
            // A connect that answers within its first slice returns, and
            // one that takes longer sees the interrupt, but with a place
            // free it is always tried.
            let text = outcome(ctx, result);
            assert!(
                text == "returned" || text == "java.io.InterruptedIOException: interrupted",
                "{text}"
            );
            assert_eq!(count_started(&started), 1);
            ctx.take_interrupt();
        });
    }
}
