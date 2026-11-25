//! `HttpConnection`: HTTP/1.1 requests with the three states of the MIDP
//! javadoc.
//!
//! In Setup the game sets the method and request headers and may write a
//! body. The request goes out (Connected) when the game first needs the
//! response, closes the output stream, or opens the input stream; the
//! response head is read when a getter first needs it. `close` and
//! closing every stream end the connection (Closed). Methods that are
//! not allowed in the current state throw `IOException`, the exception
//! the javadoc names. `https:` is the same over TLS (`super::tls`), with
//! `HttpsConnection.getSecurityInfo` on top.

mod date;
mod wire;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Throw, Value, Wake};

use self::wire::{Decoder, Framing, Head, Parsed};
use super::net::{self, IO_TIMEOUT, Link};
use super::streams::{self, Input, Output};
use super::tls::{self, SecurityDetails};
use super::url::HttpUrl;
use super::{Options, lock, opt_string_value, this, throw_io};

const HTTP_IMPL: &str = "fastemu/io/HttpConnectionImpl";
const HTTPS_IMPL: &str = "fastemu/io/HttpsConnectionImpl";

/// How long the response head may take to arrive after the request was
/// sent. A phone's network gave up on a silent server too.
const HEAD_TIMEOUT: Duration = Duration::from_secs(60);

/// The request methods MIDP allows, which are case-sensitive.
const METHODS: [&str; 3] = ["GET", "POST", "HEAD"];

/// The most request body one connection collects. Phones sent small forms
/// and some photos; nothing legitimate sends more than this from a game.
const MAX_REQUEST_BODY: usize = 4 << 20;

/// The most request headers a game may set on one connection.
const MAX_REQUEST_HEADERS: usize = 64;

/// The most the names and values of a connection's request headers may
/// take together.
const MAX_REQUEST_HEADER_BYTES: usize = 16 << 10;

/// How many interim `1xx` responses a server may send before the final
/// one. Real servers send one.
const MAX_INTERIM: u32 = 16;

/// Whether the request has gone out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// The request can still change.
    Setup,
    /// The request was sent, or sending it failed.
    Sent,
}

/// Everything one `HttpConnection` knows, shared by the connection object
/// and the streams it opened. It is only locked briefly, with the VM
/// lock held; the socket work happens on the [`Exchange`].
struct State {
    url: HttpUrl,
    /// The name the connection was opened with, for `getURL`.
    name: String,
    options: Options,
    method: String,
    /// The request headers in the order the game set them.
    headers: Vec<(String, String)>,
    /// The bytes of the names and values in `headers`.
    header_bytes: usize,
    /// The request body the output stream collected, until the request
    /// goes out.
    body: Vec<u8>,
    /// The body's share of what the game's connections may hold; given
    /// back when the request goes out.
    pending: Option<net::Pending>,
    phase: Phase,
    /// `close` was called.
    closed: bool,
    input_opened: bool,
    output_opened: bool,
    /// Streams opened and not yet closed.
    open_streams: u32,
    /// The socket, once connected; kept here to close it without waiting
    /// for the exchange.
    link: Option<Arc<Link>>,
    exchange: Option<Arc<Mutex<Exchange>>>,
    /// Another thread is reading the response head.
    head_pending: bool,
    response: Option<Arc<Head>>,
    /// Why connecting or reading the response failed: the exception class
    /// and message, thrown again by every later call that needs the
    /// response.
    failure: Option<(String, Option<String>)>,
    /// The TLS details of an `https:` connection, once connected.
    security: Option<SecurityDetails>,
}

impl State {
    /// Closes the socket once the connection and all its streams are
    /// closed.
    fn release_if_done(&mut self) {
        if self.closed && self.open_streams == 0 {
            self.exchange = None;
            if let Some(link) = self.link.take() {
                link.shutdown();
            }
        }
    }
}

type Shared = Arc<Mutex<State>>;

/// The payload of a connection object.
#[derive(Default)]
struct HttpPayload(Option<Shared>);

impl Payload for HttpPayload {}

/// The response side of a connection: the socket, the bytes received and
/// not yet used, and the body decoder. Locked only with the VM lock
/// released.
struct Exchange {
    link: Arc<Link>,
    buf: Vec<u8>,
    eof: bool,
    head_request: bool,
    decoder: Decoder,
    /// How many interim `1xx` heads came before the final one.
    interim: u32,
}

impl Exchange {
    fn new(link: Arc<Link>, head_request: bool) -> Self {
        Self {
            link,
            buf: Vec::new(),
            eof: false,
            head_request,
            decoder: Decoder::new(Framing::Empty),
            interim: 0,
        }
    }

    /// One bounded read from the socket. `Ok(false)` when nothing arrived
    /// within a slice.
    fn fill(&mut self) -> std::io::Result<bool> {
        let mut chunk = [0; 16 * 1024];
        match net::again(self.link.read(&mut chunk))? {
            None => Ok(false),
            Some(0) => {
                self.eof = true;
                Ok(true)
            }
            Some(n) => {
                self.buf.extend_from_slice(&chunk[..n]);
                Ok(true)
            }
        }
    }

    /// Reads the response head, skipping interim `1xx` responses, and sets
    /// up the body decoder. `Ok(None)` when it needs more time.
    ///
    /// One call parses what has arrived and takes in at most one read,
    /// and every interim head ends it. The caller then checks for a stop,
    /// an interrupt and its deadline, which a server that keeps sending
    /// (interim heads, a head one byte at a time) cannot keep from it.
    fn read_head(&mut self) -> std::io::Result<Option<Head>> {
        match wire::parse_head(&self.buf, self.eof).map_err(std::io::Error::other)? {
            Parsed::Head(head, used) => {
                self.buf.drain(..used);
                if (100..200).contains(&head.code) && head.code != 101 {
                    self.interim += 1;
                    if self.interim > MAX_INTERIM {
                        return Err(std::io::Error::other(
                            "the server sent too many interim responses",
                        ));
                    }
                    return Ok(None);
                }
                self.decoder = Decoder::new(wire::framing(self.head_request, &head));
                return Ok(Some(head));
            }
            Parsed::NotHttp => {
                self.decoder = Decoder::new(Framing::UntilClose);
                return Ok(Some(Head::not_http()));
            }
            Parsed::Incomplete if self.eof => {
                return Err(std::io::Error::other(
                    "the server closed the connection without a response",
                ));
            }
            Parsed::Incomplete => {}
        }
        self.fill()?;
        Ok(None)
    }

    /// Decodes body bytes into `out`: `Ok(Some(0))` at the end of the
    /// body, `Ok(None)` when nothing arrived within a slice or what
    /// arrived held no body bytes (a chunk header, a trailer line), so
    /// the caller can check for a stop and its deadline.
    fn read_body(&mut self, out: &mut [u8]) -> std::io::Result<Option<usize>> {
        let mut filled = false;
        loop {
            let (used, made) = self
                .decoder
                .decode(&self.buf, out)
                .map_err(std::io::Error::other)?;
            self.buf.drain(..used);
            if made > 0 || self.decoder.is_done() {
                return Ok(Some(made));
            }
            if self.eof {
                self.decoder.end_of_input().map_err(std::io::Error::other)?;
                return Ok(Some(0));
            }
            if filled || !self.fill()? {
                return Ok(None);
            }
            filled = true;
        }
    }
}

fn shared(ctx: &mut Ctx<'_>, conn: ObjRef) -> JResult<Shared> {
    match &ctx.payload::<HttpPayload>(conn)?.0 {
        Some(s) => Ok(Arc::clone(s)),
        None => throw_io(ctx, "the connection was not opened with Connector.open"),
    }
}

/// Opens an `http:` or `https:` connection: parses the name and checks
/// the network policy. Nothing goes on the network until the game needs
/// it.
pub(crate) fn open(
    ctx: &mut Ctx<'_>,
    scheme: &str,
    rest: &str,
    name: &str,
    options: Options,
) -> JResult<ObjRef> {
    let url = match HttpUrl::parse(scheme, rest) {
        Ok(url) => url,
        Err(message) => return ctx.throw("java/lang/IllegalArgumentException", &message),
    };
    net::check_network(ctx)?;
    let pending = net::pending(ctx);
    let class = ctx.find_class(if scheme == "https" {
        HTTPS_IMPL
    } else {
        HTTP_IMPL
    })?;
    let conn = ctx.new_object(class)?;
    let state = State {
        url,
        name: name.to_owned(),
        options,
        method: "GET".to_owned(),
        headers: Vec::new(),
        header_bytes: 0,
        body: Vec::new(),
        pending: Some(pending),
        phase: Phase::Setup,
        closed: false,
        input_opened: false,
        output_opened: false,
        open_streams: 0,
        link: None,
        exchange: None,
        head_pending: false,
        response: None,
        failure: None,
        security: None,
    };
    ctx.set_payload(conn, HttpPayload(Some(Arc::new(Mutex::new(state)))));
    Ok(conn)
}

/// The `User-Agent` FastEmu sends for games that set none: the phone's
/// platform and the profile, as phones sent.
fn user_agent(ctx: &Ctx<'_>) -> String {
    let platform = ctx.property("microedition.platform").unwrap_or("FastEmu");
    format!("{platform} Profile/MIDP-2.0 Configuration/CLDC-1.1")
}

/// Throws the remembered failure again.
fn rethrow(ctx: &mut Ctx<'_>, failure: &(String, Option<String>)) -> Throw {
    ctx.new_exception(&failure.0, failure.1.as_deref())
}

/// Remembers why the connection failed, so later calls fail the same
/// way, and passes the exception on.
fn remember(ctx: &mut Ctx<'_>, shared: &Shared, thrown: Throw) -> Throw {
    if let Throw::Exception(e) = thrown {
        let class = ctx.class_name(ctx.class_of(e)).to_owned();
        let message = match ctx.call_virtual(e, "getMessage", "()Ljava/lang/String;", &[]) {
            Ok(m) => ctx.opt_string(m).ok().flatten(),
            Err(Throw::Stop) => return Throw::Stop,
            Err(Throw::Exception(_)) => None,
        };
        lock(shared).failure = Some((class, message));
        ctx.notify_waiters();
    }
    thrown
}

/// Sends the request if it has not gone out: connects and writes the
/// request line, headers and body. `allow_closed` lets the output stream
/// send after the connection was closed, which the javadoc allows while a
/// stream is open.
fn ensure_sent(ctx: &mut Ctx<'_>, shared: &Shared, allow_closed: bool) -> JResult<()> {
    let agent = user_agent(ctx);
    let (url, request, _pending) = {
        let mut s = lock(shared);
        if s.closed && !allow_closed {
            drop(s);
            return throw_io(ctx, "connection is closed");
        }
        if let Some(failure) = s.failure.clone() {
            drop(s);
            return Err(rethrow(ctx, &failure));
        }
        if s.phase == Phase::Sent {
            return Ok(());
        }
        s.phase = Phase::Sent;
        let body = std::mem::take(&mut s.body);
        let request = wire::encode_request(&s.method, &s.url, &s.headers, body, &agent);
        // The body counts until the request is written.
        (s.url.clone(), request, s.pending.take())
    };
    let link = match send(ctx, &url, &request) {
        Ok(link) => Arc::new(link),
        Err(Throw::Stop) => return Err(Throw::Stop),
        Err(thrown) => return Err(remember(ctx, shared, thrown)),
    };
    let mut s = lock(shared);
    let exchange = Exchange::new(Arc::clone(&link), s.method == "HEAD");
    s.exchange = Some(Arc::new(Mutex::new(exchange)));
    s.security = link.security();
    s.link = Some(link);
    s.release_if_done();
    drop(s);
    ctx.notify_waiters();
    Ok(())
}

/// Connects to the server (with TLS for `https:`) and writes `request`.
fn send(ctx: &mut Ctx<'_>, url: &HttpUrl, request: &[u8]) -> JResult<Link> {
    let link = if url.scheme == "https" {
        tls::connect(ctx, &url.host, url.port)?
    } else {
        Link::plain(net::connect(ctx, &url.host, url.port)?)
    };
    net::write_all(ctx, &link, request, net::deadline(Some(IO_TIMEOUT)))?;
    Ok(link)
}

/// The response head, sending the request and reading the head first if
/// needed: the move from Setup to Connected.
fn ensure_response(ctx: &mut Ctx<'_>, shared: &Shared) -> JResult<Arc<Head>> {
    ensure_sent(ctx, shared, false)?;
    let deadline = Instant::now() + HEAD_TIMEOUT;
    let exchange = loop {
        let mut s = lock(shared);
        if s.closed {
            drop(s);
            return throw_io(ctx, "connection is closed");
        }
        if let Some(head) = &s.response {
            return Ok(Arc::clone(head));
        }
        if let Some(failure) = s.failure.clone() {
            drop(s);
            return Err(rethrow(ctx, &failure));
        }
        if !s.head_pending
            && let Some(exchange) = s.exchange.clone()
        {
            s.head_pending = true;
            break exchange;
        }
        // Another thread of the game is sending the request or reading
        // the head; wait for it.
        drop(s);
        if Instant::now() >= deadline {
            return Err(net::interrupted(ctx, "timed out"));
        }
        match ctx.wait_until(Some(deadline))? {
            Wake::Interrupted => {
                ctx.take_interrupt();
                return Err(net::interrupted(ctx, "interrupted"));
            }
            Wake::Paused => ctx.yield_now()?,
            Wake::Notified | Wake::TimedOut => {}
        }
    };
    let result = net::block_on(ctx, Some(deadline), || lock(&exchange).read_head());
    lock(shared).head_pending = false;
    match result {
        Ok(head) => {
            let head = Arc::new(head);
            lock(shared).response = Some(Arc::clone(&head));
            ctx.notify_waiters();
            Ok(head)
        }
        Err(Throw::Stop) => Err(Throw::Stop),
        Err(thrown) => Err(remember(ctx, shared, thrown)),
    }
}

/// The response head for a getter that cannot throw `IOException`
/// (`getType`, `getEncoding`, `getLength`): `None` when the connection
/// failed.
fn quiet_response(ctx: &mut Ctx<'_>, conn: ObjRef) -> JResult<Option<Arc<Head>>> {
    let shared = shared(ctx, conn)?;
    match ensure_response(ctx, &shared) {
        Ok(head) => Ok(Some(head)),
        Err(Throw::Stop) => Err(Throw::Stop),
        Err(Throw::Exception(_)) => Ok(None),
    }
}

fn response(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Arc<Head>> {
    let shared = shared(ctx, this(a))?;
    ensure_response(ctx, &shared)
}

/// The body of a response, as the input stream reads it.
struct BodyInput {
    shared: Shared,
    /// `None` when the response has no body.
    exchange: Option<Arc<Mutex<Exchange>>>,
    /// How long one read may wait, when the game asked for timeouts.
    timeout: Option<Duration>,
    closed: AtomicBool,
}

impl Input for BodyInput {
    /// Fills `buf` up to the end of the body. Games often read a reply
    /// with one `read(buffer)` sized by `getLength()`, which a phone's
    /// buffered HTTP stack answered in full.
    fn read(&self, ctx: &mut Ctx<'_>, buf: &mut [u8]) -> JResult<usize> {
        let Some(exchange) = &self.exchange else {
            return Ok(0);
        };
        let mut filled = 0;
        while filled < buf.len() {
            let dst = &mut buf[filled..];
            let read = net::block_on(ctx, net::deadline(self.timeout), || {
                if self.closed.load(Ordering::Acquire) {
                    return Err(std::io::Error::other("stream closed"));
                }
                lock(exchange).read_body(dst)
            });
            match read {
                Ok(0) => break,
                Ok(n) => filled += n,
                // Bytes already taken are returned; the error comes again
                // on the next read.
                Err(Throw::Exception(_)) if filled > 0 => break,
                Err(thrown) => return Err(thrown),
            }
        }
        Ok(filled)
    }

    fn available(&self) -> usize {
        self.exchange
            .as_ref()
            .and_then(|e| e.try_lock().ok().map(|e| e.decoder.ready(&e.buf)))
            .unwrap_or(0)
    }

    fn close(&self, _ctx: &mut Ctx<'_>) -> JResult<()> {
        if !self.closed.swap(true, Ordering::AcqRel) {
            let mut s = lock(&self.shared);
            s.open_streams -= 1;
            s.release_if_done();
        }
        Ok(())
    }
}

/// The request body, as the output stream collects it. The request goes
/// out when the stream closes or the game asks for the response.
/// `flush` sends nothing: the body has to be complete for its
/// `Content-Length`, and the emulators games were tested on buffered the
/// same way.
struct BodyOutput {
    shared: Shared,
    closed: AtomicBool,
}

impl Output for BodyOutput {
    fn write(&self, ctx: &mut Ctx<'_>, bytes: &[u8]) -> JResult<()> {
        let mut s = lock(&self.shared);
        if s.phase == Phase::Sent {
            drop(s);
            return throw_io(ctx, "the request was already sent");
        }
        if s.body.len() + bytes.len() > MAX_REQUEST_BODY {
            drop(s);
            return throw_io(ctx, "the request body is larger than 4 MiB");
        }
        if !s.pending.as_mut().is_some_and(|p| p.add(bytes.len())) {
            drop(s);
            return throw_io(ctx, "too much request data is waiting to be sent");
        }
        s.body.extend_from_slice(bytes);
        Ok(())
    }

    fn flush(&self, _ctx: &mut Ctx<'_>) -> JResult<()> {
        Ok(())
    }

    fn close(&self, ctx: &mut Ctx<'_>) -> JResult<()> {
        if self.closed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        let sent = ensure_sent(ctx, &self.shared, true);
        let mut s = lock(&self.shared);
        s.open_streams -= 1;
        s.release_if_done();
        sent
    }
}

fn open_input(ctx: &mut Ctx<'_>, conn: ObjRef) -> JResult<ObjRef> {
    let shared = shared(ctx, conn)?;
    {
        let s = lock(&shared);
        let refusal = if s.closed {
            Some("connection is closed")
        } else if !s.options.can_read() {
            Some("the connection was not opened for reading")
        } else if s.input_opened {
            Some("the input stream is already open")
        } else {
            None
        };
        if let Some(message) = refusal {
            drop(s);
            return throw_io(ctx, message);
        }
    }
    ensure_response(ctx, &shared)?;
    let (exchange, timeout) = {
        let mut s = lock(&shared);
        if s.input_opened {
            drop(s);
            return throw_io(ctx, "the input stream is already open");
        }
        s.input_opened = true;
        s.open_streams += 1;
        let has_body = s
            .exchange
            .as_ref()
            .is_some_and(|e| !lock(e).decoder.is_done());
        let exchange = if has_body { s.exchange.clone() } else { None };
        (exchange, s.options.timeouts.then_some(IO_TIMEOUT))
    };
    let input = BodyInput {
        shared,
        exchange,
        timeout,
        closed: AtomicBool::new(false),
    };
    streams::new_input(ctx, Arc::new(input))
}

fn open_output(ctx: &mut Ctx<'_>, conn: ObjRef) -> JResult<ObjRef> {
    let shared = shared(ctx, conn)?;
    {
        let mut s = lock(&shared);
        let refusal = if s.closed {
            Some("connection is closed")
        } else if !s.options.can_write() {
            Some("the connection was not opened for writing")
        } else if s.phase == Phase::Sent {
            Some("the request was already sent")
        } else if s.output_opened {
            Some("the output stream is already open")
        } else {
            None
        };
        if let Some(message) = refusal {
            drop(s);
            return throw_io(ctx, message);
        }
        s.output_opened = true;
        s.open_streams += 1;
    }
    let output = BodyOutput {
        shared,
        closed: AtomicBool::new(false),
    };
    streams::new_output(ctx, Arc::new(output))
}

/// Checks that the request can still change: `Ok(false)` once the output
/// stream is open (changes are then ignored, as the javadoc says),
/// `IOException` once it was sent or the connection closed.
fn may_change(ctx: &mut Ctx<'_>, s: &State) -> JResult<bool> {
    if s.closed {
        return throw_io(ctx, "connection is closed");
    }
    if s.phase == Phase::Sent {
        return throw_io(ctx, "the request was already sent");
    }
    Ok(!s.output_opened)
}

fn set_request_method(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let shared = shared(ctx, this(a))?;
    let method = ctx.non_null(a[1])?;
    let method = ctx.string(method)?;
    let s = lock(&shared);
    let change = may_change(ctx, &s);
    drop(s);
    if !change? {
        return Ok(Value::Void);
    }
    if !METHODS.contains(&method.as_str()) {
        return throw_io(ctx, &format!("unsupported request method {method}"));
    }
    lock(&shared).method = method;
    Ok(Value::Void)
}

fn set_request_property(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let shared = shared(ctx, this(a))?;
    let key = ctx.non_null(a[1])?;
    // Line breaks would end the header early; nothing legitimate has them.
    let key: String = ctx.string(key)?.replace(['\r', '\n'], "");
    let value = ctx.opt_string(a[2])?.map(|v| v.replace(['\r', '\n'], ""));
    let mut s = lock(&shared);
    let change = may_change(ctx, &s);
    if !change? {
        return Ok(Value::Void);
    }
    let at = s
        .headers
        .iter()
        .position(|(k, _)| k.eq_ignore_ascii_case(&key));
    // What the headers come to after this call.
    let old = at.map_or(0, |i| s.headers[i].0.len() + s.headers[i].1.len());
    let new = value.as_ref().map_or(0, |v| key.len() + v.len());
    let count = s.headers.len() + usize::from(at.is_none() && value.is_some())
        - usize::from(at.is_some() && value.is_none());
    let bytes = s.header_bytes - old + new;
    if count > MAX_REQUEST_HEADERS {
        drop(s);
        return throw_io(ctx, "too many request headers");
    }
    if bytes > MAX_REQUEST_HEADER_BYTES {
        drop(s);
        return throw_io(ctx, "the request headers are larger than 16 KiB");
    }
    s.header_bytes = bytes;
    match (at, value) {
        (Some(i), Some(v)) => s.headers[i].1 = v,
        (Some(i), None) => {
            s.headers.remove(i);
        }
        (None, Some(v)) => s.headers.push((key, v)),
        (None, None) => {}
    }
    Ok(Value::Void)
}

/// A getter of the parsed name.
fn url_getter(
    f: fn(&State) -> Option<String>,
) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> + Send + Sync + 'static {
    move |ctx, a| {
        let shared = shared(ctx, this(a))?;
        let text = f(&lock(&shared));
        opt_string_value(ctx, text.as_deref())
    }
}

/// A date header as milliseconds, or `default`.
fn date_header(head: &Head, name: &str, default: i64) -> i64 {
    head.get(name).and_then(date::parse).unwrap_or(default)
}

/// The `HttpConnection` class.
fn class() -> NativeClass {
    NativeClass::new(HTTP_IMPL)
        .implements("javax/microedition/io/HttpConnection")
        .payload::<HttpPayload>()
        .method("close", "()V", |ctx, a| {
            let shared = shared(ctx, this(a))?;
            let mut s = lock(&shared);
            s.closed = true;
            s.release_if_done();
            drop(s);
            ctx.notify_waiters();
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
        .method("getType", "()Ljava/lang/String;", |ctx, a| {
            let head = quiet_response(ctx, this(a))?;
            let value = head.as_ref().and_then(|h| h.get("content-type"));
            opt_string_value(ctx, value)
        })
        .method("getEncoding", "()Ljava/lang/String;", |ctx, a| {
            let head = quiet_response(ctx, this(a))?;
            let value = head.as_ref().and_then(|h| h.get("content-encoding"));
            opt_string_value(ctx, value)
        })
        .method("getLength", "()J", |ctx, a| {
            let head = quiet_response(ctx, this(a))?;
            let length = head
                .as_ref()
                .and_then(|h| h.get("content-length"))
                .and_then(|v| v.trim().parse::<i64>().ok());
            Ok(Value::Long(length.unwrap_or(-1)))
        })
        .method(
            "getURL",
            "()Ljava/lang/String;",
            url_getter(|s| Some(s.name.clone())),
        )
        .method(
            "getProtocol",
            "()Ljava/lang/String;",
            url_getter(|s| Some(s.url.scheme.clone())),
        )
        .method(
            "getHost",
            "()Ljava/lang/String;",
            url_getter(|s| Some(s.url.host.clone())),
        )
        .method(
            "getFile",
            "()Ljava/lang/String;",
            url_getter(|s| s.url.file.clone()),
        )
        .method(
            "getRef",
            "()Ljava/lang/String;",
            url_getter(|s| s.url.fragment.clone()),
        )
        .method(
            "getQuery",
            "()Ljava/lang/String;",
            url_getter(|s| s.url.query.clone()),
        )
        .method("getPort", "()I", |ctx, a| {
            let shared = shared(ctx, this(a))?;
            let port = lock(&shared).url.port;
            Ok(Value::Int(i32::from(port)))
        })
        .method(
            "getRequestMethod",
            "()Ljava/lang/String;",
            url_getter(|s| Some(s.method.clone())),
        )
        .method(
            "setRequestMethod",
            "(Ljava/lang/String;)V",
            set_request_method,
        )
        .method(
            "getRequestProperty",
            "(Ljava/lang/String;)Ljava/lang/String;",
            |ctx, a| {
                let shared = shared(ctx, this(a))?;
                let Some(key) = ctx.opt_string(a[1])? else {
                    return Ok(Value::Ref(None));
                };
                let value = lock(&shared)
                    .headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(&key))
                    .map(|(_, v)| v.clone());
                opt_string_value(ctx, value.as_deref())
            },
        )
        .method(
            "setRequestProperty",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            set_request_property,
        )
        .method("getResponseCode", "()I", |ctx, a| {
            Ok(Value::Int(response(ctx, a)?.code))
        })
        .method("getResponseMessage", "()Ljava/lang/String;", |ctx, a| {
            let head = response(ctx, a)?;
            opt_string_value(ctx, head.message.as_deref())
        })
        .method("getExpiration", "()J", |ctx, a| {
            let head = response(ctx, a)?;
            Ok(Value::Long(date_header(&head, "expires", 0)))
        })
        .method("getDate", "()J", |ctx, a| {
            let head = response(ctx, a)?;
            Ok(Value::Long(date_header(&head, "date", 0)))
        })
        .method("getLastModified", "()J", |ctx, a| {
            let head = response(ctx, a)?;
            Ok(Value::Long(date_header(&head, "last-modified", 0)))
        })
        .method(
            "getHeaderField",
            "(Ljava/lang/String;)Ljava/lang/String;",
            |ctx, a| {
                let head = response(ctx, a)?;
                let name = ctx.opt_string(a[1])?;
                let value = name.and_then(|n| head.get(&n).map(str::to_owned));
                opt_string_value(ctx, value.as_deref())
            },
        )
        .method("getHeaderFieldInt", "(Ljava/lang/String;I)I", |ctx, a| {
            let head = response(ctx, a)?;
            let name = ctx.opt_string(a[1])?;
            let value = name
                .and_then(|n| head.get(&n).map(str::to_owned))
                .and_then(|v| v.trim().parse::<i32>().ok());
            Ok(Value::Int(value.unwrap_or(a[2].int())))
        })
        .method("getHeaderFieldDate", "(Ljava/lang/String;J)J", |ctx, a| {
            let head = response(ctx, a)?;
            let name = ctx.opt_string(a[1])?.unwrap_or_default();
            Ok(Value::Long(date_header(&head, &name, a[2].long())))
        })
        .method("getHeaderField", "(I)Ljava/lang/String;", |ctx, a| {
            let head = response(ctx, a)?;
            let field = usize::try_from(a[1].int())
                .ok()
                .and_then(|i| head.headers.get(i));
            opt_string_value(ctx, field.map(|(_, v)| v.as_str()))
        })
        .method("getHeaderFieldKey", "(I)Ljava/lang/String;", |ctx, a| {
            let head = response(ctx, a)?;
            let field = usize::try_from(a[1].int())
                .ok()
                .and_then(|i| head.headers.get(i));
            opt_string_value(ctx, field.map(|(k, _)| k.as_str()))
        })
}

/// `HttpsConnection`: `HttpConnection` over TLS, and its security
/// information. Asking for it connects, as the javadoc says.
fn https_class() -> NativeClass {
    NativeClass::new(HTTPS_IMPL)
        .extends(HTTP_IMPL)
        .implements("javax/microedition/io/HttpsConnection")
        .method(
            "getSecurityInfo",
            "()Ljavax/microedition/io/SecurityInfo;",
            |ctx, a| {
                let shared = shared(ctx, this(a))?;
                ensure_sent(ctx, &shared, false)?;
                let details = lock(&shared).security.clone().unwrap_or_default();
                Ok(tls::security_info(ctx, details)?.into())
            },
        )
}

/// The classes of HTTP and HTTPS.
pub(crate) fn classes() -> Vec<NativeClass> {
    vec![class(), https_class()]
}
