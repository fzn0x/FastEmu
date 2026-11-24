//! `HttpConnection`: HTTP/1.1 requests with the three states of the MIDP
//! javadoc.
//!
//! In Setup the game sets the method and request headers and may write a
//! body. The request goes out (Connected) when the game first needs the
//! response, closes the output stream, or opens the input stream; the
//! response head is read when a getter first needs it. `close` and
//! closing every stream end the connection (Closed). Methods that are
//! not allowed in the current state throw `IOException`, the exception
//! the javadoc names.

mod date;
mod wire;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Throw, Value, Wake};

use self::wire::{Decoder, Framing, Head, Parsed};
use super::net::{self, IO_TIMEOUT, Link};
use super::streams::{self, Input, Output};
use super::url::HttpUrl;
use super::{Options, lock, opt_string_value, this, throw_io};

const HTTP_IMPL: &str = "fastemu/io/HttpConnectionImpl";

/// How long the response head may take to arrive after the request was
/// sent. A phone's network gave up on a silent server too.
const HEAD_TIMEOUT: Duration = Duration::from_secs(60);

/// The request methods MIDP allows, which are case-sensitive.
const METHODS: [&str; 3] = ["GET", "POST", "HEAD"];

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
    /// The request body the output stream collected.
    body: Vec<u8>,
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
}

impl Exchange {
    fn new(link: Arc<Link>, head_request: bool) -> Self {
        Self {
            link,
            buf: Vec::new(),
            eof: false,
            head_request,
            decoder: Decoder::new(Framing::Empty),
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
    fn read_head(&mut self) -> std::io::Result<Option<Head>> {
        loop {
            match wire::parse_head(&self.buf, self.eof).map_err(std::io::Error::other)? {
                Parsed::Head(head, used) => {
                    self.buf.drain(..used);
                    if (100..200).contains(&head.code) && head.code != 101 {
                        continue;
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
            if !self.fill()? {
                return Ok(None);
            }
        }
    }

    /// Decodes body bytes into `out`: `Ok(Some(0))` at the end of the
    /// body, `Ok(None)` when nothing arrived within a slice.
    fn read_body(&mut self, out: &mut [u8]) -> std::io::Result<Option<usize>> {
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
            if !self.fill()? {
                return Ok(None);
            }
        }
    }
}

fn shared(ctx: &mut Ctx<'_>, conn: ObjRef) -> JResult<Shared> {
    match &ctx.payload::<HttpPayload>(conn)?.0 {
        Some(s) => Ok(Arc::clone(s)),
        None => throw_io(ctx, "the connection was not opened with Connector.open"),
    }
}

/// Opens an `http:` connection: parses the name and checks the network
/// policy. Nothing goes on the network until the game needs it.
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
    let class = ctx.find_class(HTTP_IMPL)?;
    let conn = ctx.new_object(class)?;
    let state = State {
        url,
        name: name.to_owned(),
        options,
        method: "GET".to_owned(),
        headers: Vec::new(),
        body: Vec::new(),
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
        let message = ctx
            .call_virtual(e, "getMessage", "()Ljava/lang/String;", &[])
            .ok()
            .and_then(|m| ctx.opt_string(m).ok().flatten());
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
    let (url, request) = {
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
        let request = wire::encode_request(&s.method, &s.url, &s.headers, &s.body, &agent);
        (s.url.clone(), request)
    };
    let link = match send(ctx, &url, &request) {
        Ok(link) => Arc::new(link),
        Err(Throw::Stop) => return Err(Throw::Stop),
        Err(thrown) => return Err(remember(ctx, shared, thrown)),
    };
    let mut s = lock(shared);
    let exchange = Exchange::new(Arc::clone(&link), s.method == "HEAD");
    s.exchange = Some(Arc::new(Mutex::new(exchange)));
    s.link = Some(link);
    s.release_if_done();
    drop(s);
    ctx.notify_waiters();
    Ok(())
}

/// Connects to the server and writes `request`.
fn send(ctx: &mut Ctx<'_>, url: &HttpUrl, request: &[u8]) -> JResult<Link> {
    let socket = net::connect(ctx, &url.host, url.port)?;
    let link = Link::plain(socket);
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
            let n = net::block_on(ctx, net::deadline(self.timeout), || {
                if self.closed.load(Ordering::Acquire) {
                    return Err(std::io::Error::other("stream closed"));
                }
                lock(exchange).read_body(dst)
            })?;
            if n == 0 {
                break;
            }
            filled += n;
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

/// The classes of HTTP.
pub(crate) fn classes() -> Vec<NativeClass> {
    vec![class()]
}
