//! HttpConnection end to end: a Java program (tests/io_java/HttpTest.java)
//! talks to a scripted HTTP server on 127.0.0.1 through the VM.

mod io_support;

use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

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
