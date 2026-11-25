//! HttpConnection end to end: a Java program (tests/io_java/HttpTest.java)
//! talks to a scripted HTTP server on 127.0.0.1 through the VM.

mod io_support;

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use fastemu_core::NetworkPolicy;
use io_support::{HttpServer, Request};

/// The scripted server: one reply per path.
fn respond(request: &Request, stream: &mut TcpStream) {
    let head_only = request.method == "HEAD";
    let reply: Vec<u8> = match request.path() {
        "/hello" => {
            let seen = request.header("X-Custom").unwrap_or("none");
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\
                 Date: Sun, 06 Nov 1994 08:49:37 GMT\r\nX-Number: 42\r\nX-Seen: {seen}\r\n\r\n"
            );
            let body: &[u8] = if head_only { b"" } else { b"hello" };
            [head.as_bytes(), body].concat()
        }
        "/chunked" => {
            let head = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n";
            let parts: [&[u8]; 5] = [
                head,
                b"3\r\nhel\r\n",
                b"4\r\nlo, \r\n",
                b"5\r\nworld\r\n",
                b"0\r\n\r\n",
            ];
            // Longer than the VM's read slice, so the body arrives over
            // several of them.
            io_support::write_slowly(stream, &parts, Duration::from_millis(150));
            return;
        }
        "/echo" => {
            let late = request.header("X-Late").unwrap_or("none");
            let head = format!(
                "HTTP/1.1 200 OK\r\nX-Method: {}\r\nX-Late: {late}\r\nContent-Length: {}\r\n\r\n",
                request.method,
                request.body.len()
            );
            [head.as_bytes(), &request.body].concat()
        }
        "/missing" => b"HTTP/1.1 404 Not Found\r\nContent-Length: 4\r\n\r\nnope".to_vec(),
        "/close" => b"HTTP/1.0 200 OK\r\nContent-Type: text/plain\r\n\r\nuntil the end".to_vec(),
        "/short" => b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nhello".to_vec(),
        "/raw" => b"just bytes".to_vec(),
        "/hang" => {
            std::thread::sleep(Duration::from_secs(20));
            return;
        }
        _ => b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n".to_vec(),
    };
    let _ = stream.write_all(&reply);
}

#[test]
fn http_connections_follow_the_midp_javadoc() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let server = HttpServer::start(respond);
    let port = server.port.to_string();
    let closed = io_support::closed_port().to_string();
    let data = io_support::has_class("java/io/DataInputStream");
    if !data {
        eprintln!("skipping Connector.openDataInputStream: java.io.DataInputStream is missing");
    }
    let mut args = vec![port.as_str(), closed.as_str()];
    if data {
        args.push("data");
    }
    let output = io_support::run(classes, "HttpTest", &args, NetworkPolicy::Allow);
    let mut expected = String::from(
        "prop def
code 200 OK
type text/plain length 5 encoding null
date 784111777000 expires 0 modified 0
number 42 missing -7
headerdate 784111777000 bad 5
h0 Content-Type=text/plain
h1 Content-Length=5
h2 Date=Sun, 06 Nov 1994 08:49:37 GMT
h3 X-Number=42
h4 X-Seen=def
h99 null null
body hello
chunked 200 length -1 read 12 hello, world then -1
after the request was already sent
post 200 POST none score=100&name=ab
head 200 5 -1
missing 404 Not Found nope
close 200 -1 until the end
short 5 hello then the response ended 5 bytes early
raw -1 null just bytes
put unsupported request method PUT
method the request was already sent
property the request was already sent
output the request was already sent
input the input stream is already open
url after close true
closed connection is closed
stream after close hello
readonly the connection was not opened for writing
http example.com 8080 /a/b.php x=1&y=2 frag GET
80 null null null
connector hello
",
    );
    if data {
        expected.push_str("data hello\n");
    }
    expected.push_str(
        "length -1 type null
refused java.io.IOException connection refused
again connection refused
done",
    );
    io_support::assert_output(&output, &expected);

    let get = server.request_for("/hello");
    assert_eq!(get.method, "GET");
    assert_eq!(get.target, "/hello?x=1");
    assert_eq!(
        get.header("Host"),
        Some(format!("127.0.0.1:{port}").as_str())
    );
    assert_eq!(get.header("Connection"), Some("close"));
    assert!(
        get.header("User-Agent")
            .is_some_and(|ua| ua.ends_with("Profile/MIDP-2.0 Configuration/CLDC-1.1")),
        "{:?}",
        get.header("User-Agent")
    );
    let post = server.request_for("/echo");
    assert_eq!(post.method, "POST");
    assert_eq!(post.header("Content-Length"), Some("17"));
    assert_eq!(
        post.header("Content-Type"),
        Some("application/x-www-form-urlencoded")
    );
    assert_eq!(post.header("X-Late"), None);
    assert_eq!(post.body, b"score=100&name=ab");
    let upload = server.request_for("/upload");
    assert_eq!(upload.method, "GET");
    assert_eq!(upload.body, b"fire and forget");
}

#[test]
fn waits_for_a_silent_server_end_on_interrupt_and_on_stop() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let server = HttpServer::start(respond);
    let port = server.port.to_string();
    let program =
        io_support::Program::start(classes, "HttpHang", &[&port], NetworkPolicy::Allow, |_| {});
    assert!(
        program.wait_for_line("blocking", Duration::from_secs(20)),
        "{}",
        program.output().out
    );
    std::thread::sleep(Duration::from_millis(300));
    let stopped_at = std::time::Instant::now();
    program.vm.stop();
    assert!(program.vm.wait_finished(Some(Duration::from_secs(2))));
    assert!(stopped_at.elapsed() < Duration::from_secs(1));
    io_support::assert_output(
        &program.output(),
        "interrupt -> java.io.InterruptedIOException: interrupted\nblocking",
    );
}

#[test]
fn every_network_open_fails_when_the_network_is_off() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let output = io_support::run(classes, "NetOff", &[], NetworkPolicy::Deny);
    let off = "java.io.IOException: network access is off for this game";
    let missing =
        "javax.microedition.io.ConnectionNotFoundException: the requested protocol does not exist";
    let expected = format!(
        "http://127.0.0.1:1/ -> {off}
https://example.com/ -> {off}
socket://127.0.0.1:1 -> {off}
socket://:0 -> {off}
ssl://example.com:443 -> {off}
datagram://127.0.0.1:1 -> {off}
datagram://:0 -> {off}
comm:0;baudrate=9600 -> javax.microedition.io.ConnectionNotFoundException: this phone has no serial ports: comm:0;baudrate=9600
file:///root/x.txt -> {missing}: file:///root/x.txt
gopher://example.com/ -> {missing}: gopher://example.com/
nocolon -> java.lang.IllegalArgumentException: no ':' in the connection name \"nocolon\"
null -> java.lang.IllegalArgumentException: Null URL
mode -> java.lang.IllegalArgumentException: illegal mode 7
stream -> {off}
done"
    );
    io_support::assert_output(&output, &expected);
}

#[test]
fn a_game_cannot_make_the_host_hold_unbounded_request_data() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let output = io_support::run(classes, "HttpLimits", &[], NetworkPolicy::Allow);
    let expected = "body 64 the request body is larger than 4 MiB
send failed
budget 8 MiB too much request data is waiting to be sent
first send failed
room again
headers 64 too many request headers
replaced other null new
bytes the request headers are larger than 16 KiB
swapped 9000
name java.lang.IllegalArgumentException: the connection name is longer than 8192 bytes
done";
    io_support::assert_output(&output, expected);
}

/// What a hostile server has done so far, shared with the test.
#[derive(Default)]
struct Script {
    /// How many bytes it has sent.
    sent: AtomicUsize,
    /// Set by the test when it is over, to end the server's threads.
    done: AtomicBool,
}

impl Script {
    /// Sends `bytes`, waiting for room in the socket; `false` when the
    /// client is gone or the test is over.
    fn push(&self, stream: &mut TcpStream, mut bytes: &[u8]) -> bool {
        while !bytes.is_empty() {
            if self.done.load(Ordering::Relaxed) {
                return false;
            }
            match stream.write(bytes) {
                Ok(0) => return false,
                Ok(n) => {
                    self.sent.fetch_add(n, Ordering::Relaxed);
                    bytes = &bytes[n..];
                }
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(_) => return false,
            }
        }
        true
    }

    /// Sends one byte of `filler` every 10 ms for as long as it can.
    fn drip(&self, stream: &mut TcpStream, filler: u8) {
        while self.push(stream, &[filler]) {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

const CHUNKED_HEAD: &[u8] = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\nA\r\n0\r\n";

/// Interim responses as fast as the client takes them.
fn flood_interim(stream: &mut TcpStream, script: &Script) {
    let block = b"HTTP/1.1 100 Continue\r\n\r\n".repeat(600);
    while script.push(stream, &block) {}
}

/// A chunked body, then trailer lines as fast as the client takes them.
fn flood_trailers(stream: &mut TcpStream, script: &Script) {
    if script.push(stream, CHUNKED_HEAD) {
        let block = b"X-Trailer: yes\r\n".repeat(1000);
        while script.push(stream, &block) {}
    }
}

/// A head that never ends: one byte every 10 ms.
fn drip_head(stream: &mut TcpStream, script: &Script) {
    if script.push(stream, b"HTTP/1.1 200 OK\r\nX-Drip: ") {
        script.drip(stream, b'b');
    }
}

/// A body that is far from complete: one byte every 10 ms.
fn drip_body(stream: &mut TcpStream, script: &Script) {
    if script.push(
        stream,
        b"HTTP/1.1 200 OK\r\nContent-Length: 1000000\r\n\r\n",
    ) {
        script.drip(stream, b'x');
    }
}

/// A chunked body whose trailer never ends: one byte every 10 ms.
fn drip_trailer(stream: &mut TcpStream, script: &Script) {
    if script.push(stream, CHUNKED_HEAD) {
        script.drip(stream, b'X');
    }
}

/// A server on 127.0.0.1 that reads a request and runs `script` on the
/// connection, until the test drops it.
struct Hostile {
    port: u16,
    script: Arc<Script>,
}

impl Hostile {
    fn start(script: fn(&mut TcpStream, &Script)) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let shared = Arc::<Script>::default();
        let state = Arc::clone(&shared);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let state = Arc::clone(&state);
                std::thread::spawn(move || {
                    let mut head = Vec::new();
                    let mut byte = [0];
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    while !head.ends_with(b"\r\n\r\n") {
                        match stream.read(&mut byte) {
                            Ok(1) => head.push(byte[0]),
                            _ => return,
                        }
                    }
                    // A client that stops reading must not hold the thread.
                    stream
                        .set_write_timeout(Some(Duration::from_millis(100)))
                        .unwrap();
                    script(&mut stream, &state);
                });
            }
        });
        Self {
            port,
            script: shared,
        }
    }

    /// Waits until the server has sent `bytes`.
    fn wait_for(&self, bytes: usize) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while self.script.sent.load(Ordering::Relaxed) < bytes {
            assert!(Instant::now() < deadline, "the server sent no data");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Hostile {
    fn drop(&mut self) {
        self.script.done.store(true, Ordering::Relaxed);
    }
}

/// Starts `HttpHostile` against `server` in `mode`.
fn start_hostile(classes: &Path, server: &Hostile, mode: &str) -> io_support::Program {
    let port = server.port.to_string();
    io_support::Program::start(
        classes,
        "HttpHostile",
        &[&port, mode],
        NetworkPolicy::Allow,
        |_| {},
    )
}

/// Runs `HttpHostile` against a server that sends for ever, until it
/// finishes by itself, and returns what it printed.
fn run_hostile(classes: &Path, server: &Hostile, mode: &str) -> io_support::Output {
    let program = start_hostile(classes, server, mode);
    let finished = program.vm.wait_finished(Some(Duration::from_secs(20)));
    // Let a program that is stuck go, so that nothing outlives the test.
    server.script.done.store(true, Ordering::Relaxed);
    program.vm.stop();
    let output = program.output();
    assert!(
        finished,
        "the program is stuck in a read; it printed:\n{}\n{}",
        output.out, output.err
    );
    output
}

#[test]
fn endless_interim_responses_end_with_an_error() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let server = Hostile::start(flood_interim);
    let output = run_hostile(classes, &server, "head");
    io_support::assert_output(
        &output,
        "asking\nfailed java.io.IOException: the server sent too many interim responses\ndone",
    );
}

#[test]
fn endless_trailers_end_with_an_error() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let server = Hostile::start(flood_trailers);
    let output = run_hostile(classes, &server, "body");
    io_support::assert_output(
        &output,
        "asking\ncode 200\nfailed java.io.IOException: the trailer of the response is too long\ndone",
    );
}

/// Starts `HttpHostile` against a server that trickles, waits until the
/// server has sent a few bytes of it, stops the VM and checks that it
/// ends.
fn stop_during(classes: &Path, server: &Hostile, mode: &str) {
    let program = start_hostile(classes, server, mode);
    server.wait_for(100);
    program.vm.stop();
    let ended = program.vm.wait_finished(Some(Duration::from_secs(10)));
    server.script.done.store(true, Ordering::Relaxed);
    let output = program.output();
    assert!(
        ended,
        "the VM did not end after stop(); it printed:\n{}\n{}",
        output.out, output.err
    );
}

#[test]
fn a_head_that_trickles_in_ends_when_the_game_stops() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let server = Hostile::start(drip_head);
    stop_during(classes, &server, "head");
}

#[test]
fn a_trailer_that_trickles_in_ends_when_the_game_stops() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let server = Hostile::start(drip_trailer);
    stop_during(classes, &server, "body");
}

#[test]
fn a_body_that_trickles_in_ends_when_the_game_stops() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let server = Hostile::start(drip_body);
    stop_during(classes, &server, "bulk");
}
