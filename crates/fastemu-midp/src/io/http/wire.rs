//! HTTP/1.1 on the wire: writing a request, reading the status line and
//! headers of the response, and decoding its body (fixed length, chunked
//! or up to the end of the connection).
//!
//! Every request asks for `Connection: close`, so a response always ends
//! the connection and there is no keep-alive state to track.

use std::fmt::Write as _;

use crate::io::url::HttpUrl;

/// The most a response's status line and headers may take.
const MAX_HEAD: usize = 64 * 1024;

/// The longest chunk size line accepted, extensions included.
const MAX_CHUNK_LINE: usize = 1024;

/// Request headers FastEmu writes itself, whatever the game set.
const OWN_HEADERS: [&str; 3] = ["content-length", "connection", "transfer-encoding"];

/// The bytes of a request: `method` for `url` with the game's `headers`
/// and `body`, which the request takes over as its end, so a large body
/// is not copied.
///
/// A `Host` header is added unless the game set one (some games reach a
/// virtual host through an address). `User-Agent` is `user_agent` unless
/// the game set its own. `Content-Length` always matches the body, and
/// `Connection: close` replaces whatever the game asked for.
pub(crate) fn encode_request(
    method: &str,
    url: &HttpUrl,
    headers: &[(String, String)],
    mut body: Vec<u8>,
    user_agent: &str,
) -> Vec<u8> {
    let has = |name: &str| headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(name));
    let mut head = format!("{method} {} HTTP/1.1\r\n", url.target());
    if !has("host") {
        let _ = write!(head, "Host: {}\r\n", url.host_header());
    }
    for (key, value) in headers {
        if !OWN_HEADERS.iter().any(|own| key.eq_ignore_ascii_case(own)) {
            let _ = write!(head, "{key}: {value}\r\n");
        }
    }
    if !has("user-agent") {
        let _ = write!(head, "User-Agent: {user_agent}\r\n");
    }
    if !body.is_empty() || method == "POST" {
        let _ = write!(head, "Content-Length: {}\r\n", body.len());
    }
    head.push_str("Connection: close\r\n\r\n");
    body.splice(0..0, head.into_bytes());
    body
}

/// The status line and headers of a response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Head {
    /// The status code, or -1 when the reply was not HTTP.
    pub code: i32,
    /// The reason phrase, or `None` when the reply was not HTTP.
    pub message: Option<String>,
    /// The headers in the order they came, names as the server spelled
    /// them.
    pub headers: Vec<(String, String)>,
}

impl Head {
    /// What `HttpConnection` reports for a reply that is not HTTP: no
    /// code, no message, no headers. The whole reply is then the body.
    pub(crate) fn not_http() -> Self {
        Self {
            code: -1,
            message: None,
            headers: Vec::new(),
        }
    }

    /// The value of header `name`, ignoring case. When the header repeats,
    /// the last value wins, as in the MIDP reference implementation.
    pub(crate) fn get(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// What [`parse_head`] found at the start of the received bytes.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Parsed {
    /// More bytes are needed.
    Incomplete,
    /// A complete head, and how many bytes it took.
    Head(Head, usize),
    /// The reply does not start with `HTTP/`: an HTTP/0.9 server, or not
    /// a web server at all.
    NotHttp,
}

/// Looks for a response head at the start of `buf`. With `at_eof` the
/// connection has ended, so whatever arrived is all there is: a head
/// without its blank line is taken as it is.
pub(crate) fn parse_head(buf: &[u8], at_eof: bool) -> Result<Parsed, String> {
    const MAGIC: &[u8] = b"HTTP/";
    let start = &buf[..buf.len().min(MAGIC.len())];
    if !start.eq_ignore_ascii_case(&MAGIC[..start.len()]) {
        return Ok(Parsed::NotHttp);
    }
    if start.len() < MAGIC.len() {
        return Ok(if at_eof && !buf.is_empty() {
            Parsed::NotHttp
        } else {
            Parsed::Incomplete
        });
    }
    let end = head_end(buf);
    let (text, used) = match end {
        Some(end) => (&buf[..end], end),
        None if at_eof => (buf, buf.len()),
        None if buf.len() > MAX_HEAD => return Err("the response headers are too long".into()),
        None => return Ok(Parsed::Incomplete),
    };
    // Header text is ISO-8859-1 by the HTTP specification.
    let text: String = text.iter().map(|&b| char::from(b)).collect();
    let mut lines = text.lines();
    let (code, message) = status(lines.next().unwrap_or_default());
    let mut headers: Vec<(String, String)> = Vec::new();
    for line in lines {
        if line.starts_with([' ', '\t']) {
            if let Some((_, value)) = headers.last_mut() {
                value.push(' ');
                value.push_str(line.trim());
            }
        } else if let Some((key, value)) = line.split_once(':') {
            headers.push((key.trim().to_owned(), value.trim().to_owned()));
        }
    }
    Ok(Parsed::Head(
        Head {
            code,
            message,
            headers,
        },
        used,
    ))
}

/// Where the head ends: just past its blank line, which may end in CRLF
/// or in a bare LF.
fn head_end(buf: &[u8]) -> Option<usize> {
    let mut line_start = 0;
    for (i, &b) in buf.iter().enumerate() {
        if b == b'\n' {
            let line = &buf[line_start..i];
            if line.is_empty() || line == b"\r" {
                return Some(i + 1);
            }
            line_start = i + 1;
        }
    }
    None
}

/// The code and reason phrase of a status line such as `HTTP/1.1 404 Not
/// Found`. A line without a readable code gives -1, as
/// `getResponseCode` must.
fn status(line: &str) -> (i32, Option<String>) {
    let mut parts = line.splitn(3, ' ');
    let _version = parts.next();
    let code = parts
        .next()
        .filter(|c| !c.is_empty() && c.len() <= 3 && c.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|c| c.parse().ok());
    match code {
        Some(code) => (
            code,
            Some(parts.next().unwrap_or_default().trim().to_owned()),
        ),
        None => (-1, None),
    }
}

/// How a response's body is delimited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Framing {
    /// No body: a `HEAD` request, or a status that has none.
    Empty,
    /// `Content-Length` bytes.
    Length(u64),
    /// `Transfer-Encoding: chunked`.
    Chunked,
    /// Everything until the server closes the connection.
    UntilClose,
}

/// The framing of the body that follows `head`, by RFC 7230 section
/// 3.3.3. `head_request` says whether the request was `HEAD`.
pub(crate) fn framing(head_request: bool, head: &Head) -> Framing {
    if head.code == -1 {
        return Framing::UntilClose;
    }
    if head_request || (100..200).contains(&head.code) || head.code == 204 || head.code == 304 {
        return Framing::Empty;
    }
    if let Some(coding) = head.get("transfer-encoding") {
        let last = coding.rsplit(',').next().unwrap_or_default().trim();
        return if last.eq_ignore_ascii_case("chunked") {
            Framing::Chunked
        } else {
            Framing::UntilClose
        };
    }
    match head.get("content-length").map(|v| v.trim().parse::<u64>()) {
        Some(Ok(n)) => Framing::Length(n),
        _ => Framing::UntilClose,
    }
}

/// Where a [`Decoder`] is in the body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// This many bytes of a fixed-length body are left.
    Length(u64),
    /// The body runs until the connection ends.
    UntilClose,
    /// A chunk size line comes next.
    ChunkSize,
    /// This many bytes of the current chunk are left.
    ChunkData(u64),
    /// The line end after a chunk's data comes next.
    ChunkEnd,
    /// Trailer lines after the last chunk, up to a blank line.
    Trailer,
    /// The body is complete.
    Done,
}

/// Turns the bytes that follow the head into the body.
#[derive(Clone, Debug)]
pub(crate) struct Decoder {
    state: State,
}

impl Decoder {
    /// A decoder for a body framed by `framing`.
    pub(crate) fn new(framing: Framing) -> Self {
        let state = match framing {
            Framing::Empty | Framing::Length(0) => State::Done,
            Framing::Length(n) => State::Length(n),
            Framing::Chunked => State::ChunkSize,
            Framing::UntilClose => State::UntilClose,
        };
        Self { state }
    }

    /// Whether the whole body has been decoded.
    pub(crate) fn is_done(&self) -> bool {
        self.state == State::Done
    }

    /// How many body bytes `input` holds that [`Decoder::decode`] can give
    /// at once.
    pub(crate) fn ready(&self, input: &[u8]) -> usize {
        let len = input.len() as u64;
        match self.state {
            State::Length(n) | State::ChunkData(n) => n.min(len) as usize,
            State::UntilClose => input.len(),
            _ => 0,
        }
    }

    /// Takes the body out of `input` (bytes as they came from the wire)
    /// into `out`. Returns how many input bytes it used and how many body
    /// bytes it wrote. Using fewer than all of `input` while writing
    /// nothing means it needs more input.
    pub(crate) fn decode(
        &mut self,
        input: &[u8],
        out: &mut [u8],
    ) -> Result<(usize, usize), String> {
        let (mut used, mut made) = (0, 0);
        loop {
            let rest = &input[used..];
            let room = out.len() - made;
            match self.state {
                State::Done => break,
                State::Length(n) | State::ChunkData(n) => {
                    let take = (n.min(rest.len() as u64) as usize).min(room);
                    out[made..made + take].copy_from_slice(&rest[..take]);
                    used += take;
                    made += take;
                    let left = n - take as u64;
                    self.state = match (self.state, left) {
                        (State::Length(_), 0) => State::Done,
                        (State::Length(_), _) => State::Length(left),
                        (_, 0) => State::ChunkEnd,
                        _ => State::ChunkData(left),
                    };
                    if left != 0 {
                        break;
                    }
                }
                State::UntilClose => {
                    let take = rest.len().min(room);
                    out[made..made + take].copy_from_slice(&rest[..take]);
                    used += take;
                    made += take;
                    break;
                }
                State::ChunkSize => {
                    let Some(line) = line(rest, MAX_CHUNK_LINE)? else {
                        break;
                    };
                    let digits = line.text.split(';').next().unwrap_or_default().trim();
                    let size = u64::from_str_radix(digits, 16)
                        .map_err(|_| format!("bad chunk size {digits:?}"))?;
                    used += line.len;
                    self.state = if size == 0 {
                        State::Trailer
                    } else {
                        State::ChunkData(size)
                    };
                }
                State::ChunkEnd => {
                    if rest.starts_with(b"\r\n") {
                        used += 2;
                    } else if rest.starts_with(b"\n") {
                        used += 1;
                    } else if rest.is_empty() || rest == b"\r" {
                        break;
                    } else {
                        return Err("missing line end after a chunk".into());
                    }
                    self.state = State::ChunkSize;
                }
                State::Trailer => {
                    let Some(line) = line(rest, MAX_HEAD)? else {
                        break;
                    };
                    used += line.len;
                    if line.text.is_empty() {
                        self.state = State::Done;
                    }
                }
            }
            if made == out.len() {
                break;
            }
        }
        Ok((used, made))
    }

    /// The connection ended. That completes a body that runs until the
    /// close; any other unfinished body was cut short.
    pub(crate) fn end_of_input(&mut self) -> Result<(), String> {
        match self.state {
            State::Done => Ok(()),
            State::UntilClose => {
                self.state = State::Done;
                Ok(())
            }
            State::Length(n) => Err(format!("the response ended {n} bytes early")),
            _ => Err("the response ended inside a chunk".into()),
        }
    }
}

/// One line at the start of some bytes.
struct Line {
    /// The text, without its line end.
    text: String,
    /// The bytes it took, line end included.
    len: usize,
}

/// The first line of `bytes`, or `None` when its end has not arrived. A
/// line longer than `max` is an error.
fn line(bytes: &[u8], max: usize) -> Result<Option<Line>, String> {
    match bytes.iter().position(|&b| b == b'\n') {
        Some(end) => {
            let raw = bytes[..end].strip_suffix(b"\r").unwrap_or(&bytes[..end]);
            Ok(Some(Line {
                text: raw.iter().map(|&b| char::from(b)).collect(),
                len: end + 1,
            }))
        }
        None if bytes.len() > max => Err("a line of the response is too long".into()),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(code: i32, headers: &[(&str, &str)]) -> Head {
        Head {
            code,
            message: Some("OK".into()),
            headers: headers
                .iter()
                .map(|&(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
        }
    }

    /// Decodes `input` fed `step` bytes at a time into a buffer of `room`
    /// bytes per call, as a slow connection and a small read buffer would.
    fn decode_all(
        framing: Framing,
        input: &[u8],
        step: usize,
        room: usize,
    ) -> Result<Vec<u8>, String> {
        let mut decoder = Decoder::new(framing);
        let mut pending = Vec::new();
        let mut body = Vec::new();
        let mut fed = 0;
        while !decoder.is_done() {
            let mut out = vec![0; room];
            let (used, made) = decoder.decode(&pending, &mut out)?;
            pending.drain(..used);
            body.extend_from_slice(&out[..made]);
            if made == 0 && !decoder.is_done() {
                if fed == input.len() {
                    decoder.end_of_input()?;
                    break;
                }
                let next = (fed + step).min(input.len());
                pending.extend_from_slice(&input[fed..next]);
                fed = next;
            }
        }
        Ok(body)
    }

    #[test]
    fn requests_carry_the_headers_http_needs() {
        let url = HttpUrl::parse("http", "//example.com:8080/score?a=1#top").unwrap();
        let headers = vec![
            ("Content-Type".to_owned(), "text/plain".to_owned()),
            ("content-length".to_owned(), "99".to_owned()),
            ("Connection".to_owned(), "keep-alive".to_owned()),
        ];
        let bytes = encode_request("POST", &url, &headers, b"hi".to_vec(), "Phone/1.0");
        let text = String::from_utf8(bytes).unwrap();
        assert_eq!(
            text,
            "POST /score?a=1 HTTP/1.1\r\n\
             Host: example.com:8080\r\n\
             Content-Type: text/plain\r\n\
             User-Agent: Phone/1.0\r\n\
             Content-Length: 2\r\n\
             Connection: close\r\n\r\nhi"
        );
    }

    #[test]
    fn a_game_may_set_its_own_host_and_user_agent() {
        let url = HttpUrl::parse("http", "//10.0.0.1/").unwrap();
        let headers = vec![
            ("Host".to_owned(), "games.example".to_owned()),
            ("User-Agent".to_owned(), "Game/2".to_owned()),
        ];
        let text =
            String::from_utf8(encode_request("GET", &url, &headers, Vec::new(), "x")).unwrap();
        assert_eq!(
            text,
            "GET / HTTP/1.1\r\nHost: games.example\r\nUser-Agent: Game/2\r\nConnection: close\r\n\r\n"
        );
    }

    #[test]
    fn heads_parse_with_folded_and_repeated_headers() {
        let raw = b"HTTP/1.1 404 Not Found\r\nX-A: 1\r\nX-Long: a\r\n  b\r\nx-a: 2\r\n\r\nbody";
        let Parsed::Head(head, used) = parse_head(raw, false).unwrap() else {
            panic!("no head");
        };
        assert_eq!(used, raw.len() - 4);
        assert_eq!(head.code, 404);
        assert_eq!(head.message.as_deref(), Some("Not Found"));
        assert_eq!(head.get("X-LONG"), Some("a b"));
        assert_eq!(head.get("x-a"), Some("2"));
        assert_eq!(head.headers[0], ("X-A".to_owned(), "1".to_owned()));
    }

    #[test]
    fn heads_wait_for_their_blank_line() {
        assert_eq!(parse_head(b"HTT", false).unwrap(), Parsed::Incomplete);
        assert_eq!(
            parse_head(b"HTTP/1.0 200 OK\r\nA: b\r\n", false).unwrap(),
            Parsed::Incomplete
        );
        let Parsed::Head(head, _) = parse_head(b"HTTP/1.0 200 OK\nA: b\n\n", false).unwrap() else {
            panic!("bare line feeds end a head too");
        };
        assert_eq!(head.get("a"), Some("b"));
        let Parsed::Head(cut, used) = parse_head(b"HTTP/1.0 200 OK\r\nA: b", true).unwrap() else {
            panic!("a cut head is still a head");
        };
        assert_eq!((cut.get("A"), used), (Some("b"), 21));
    }

    #[test]
    fn replies_that_are_not_http_are_recognised() {
        assert_eq!(parse_head(b"<html>", false).unwrap(), Parsed::NotHttp);
        assert_eq!(parse_head(b"HT", true).unwrap(), Parsed::NotHttp);
        let Parsed::Head(head, _) = parse_head(b"HTTP/1.1 OK\r\n\r\n", false).unwrap() else {
            panic!("a bad status line still has a head");
        };
        assert_eq!((head.code, head.message), (-1, None));
    }

    #[test]
    fn heads_have_a_size_limit() {
        let mut raw = b"HTTP/1.1 200 OK\r\n".to_vec();
        raw.resize(MAX_HEAD + 10, b'a');
        assert!(parse_head(&raw, false).is_err());
    }

    #[test]
    fn framing_follows_rfc_7230() {
        let ok = |h: &[(&str, &str)]| framing(false, &head(200, h));
        assert_eq!(ok(&[("Content-Length", "12")]), Framing::Length(12));
        assert_eq!(
            ok(&[("Transfer-Encoding", "gzip, chunked")]),
            Framing::Chunked
        );
        assert_eq!(
            ok(&[("Transfer-Encoding", "chunked"), ("Content-Length", "3")]),
            Framing::Chunked
        );
        assert_eq!(ok(&[]), Framing::UntilClose);
        assert_eq!(ok(&[("Content-Length", "x")]), Framing::UntilClose);
        assert_eq!(
            framing(true, &head(200, &[("Content-Length", "5")])),
            Framing::Empty
        );
        assert_eq!(framing(false, &head(204, &[])), Framing::Empty);
        assert_eq!(framing(false, &head(304, &[])), Framing::Empty);
        assert_eq!(framing(false, &Head::not_http()), Framing::UntilClose);
    }

    #[test]
    fn chunked_bodies_decode_whatever_the_pieces() {
        let wire = b"5;ext=1\r\nhello\r\n7\r\n, world\r\n0\r\nTrailer: x\r\n\r\n";
        for step in [1, 2, 3, 7, wire.len()] {
            for room in [1, 4, 64] {
                let body = decode_all(Framing::Chunked, wire, step, room).unwrap();
                assert_eq!(body, b"hello, world", "step {step} room {room}");
            }
        }
    }

    #[test]
    fn fixed_and_open_ended_bodies_decode() {
        assert_eq!(
            decode_all(Framing::Length(5), b"helloEXTRA", 3, 2).unwrap(),
            b"hello"
        );
        assert_eq!(
            decode_all(Framing::UntilClose, b"to the end", 4, 3).unwrap(),
            b"to the end"
        );
        assert_eq!(decode_all(Framing::Empty, b"ignored", 1, 1).unwrap(), b"");
    }

    #[test]
    fn cut_bodies_are_errors() {
        assert!(decode_all(Framing::Length(10), b"short", 5, 64).is_err());
        assert!(decode_all(Framing::Chunked, b"5\r\nhel", 5, 64).is_err());
        assert!(decode_all(Framing::Chunked, b"zz\r\n", 5, 64).is_err());
        assert!(decode_all(Framing::Chunked, b"1\r\naXY", 5, 64).is_err());
    }

    #[test]
    fn ready_counts_only_body_bytes() {
        let decoder = Decoder::new(Framing::Length(3));
        assert_eq!(decoder.ready(b"abcdef"), 3);
        assert_eq!(Decoder::new(Framing::Chunked).ready(b"5\r\nhello"), 0);
        assert_eq!(Decoder::new(Framing::UntilClose).ready(b"abc"), 3);
    }
}
