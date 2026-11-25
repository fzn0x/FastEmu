//! SocketConnection and ServerSocketConnection end to end: a Java program
//! (tests/io_java/SocketTest.java) talks to a Rust echo peer, then
//! accepts a connection from this test.

mod io_support;

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use fastemu_core::NetworkPolicy;

/// A line-based peer: answers `ping` with `ping`, `more` with `more data`,
/// echoes other lines, and hangs up on `bye`.
fn start_echo_peer() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            std::thread::spawn(move || {
                let mut writer = stream.try_clone().unwrap();
                for line in BufReader::new(stream).lines() {
                    let Ok(line) = line else { break };
                    let reply = match line.as_str() {
                        "bye" => break,
                        "more" => "more data".to_owned(),
                        other => other.to_owned(),
                    };
                    if writer.write_all(format!("{reply}\n").as_bytes()).is_err() {
                        break;
                    }
                }
            });
        }
    });
    port
}

#[test]
fn sockets_connect_listen_and_carry_data() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let echo = start_echo_peer().to_string();
    let closed = io_support::closed_port().to_string();
    let program = io_support::Program::start(
        classes,
        "SocketTest",
        &[&echo, &closed],
        NetworkPolicy::Allow,
        |_| {},
    );
    // Wait for the program's server socket, then be its client.
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    let port = loop {
        let out = program.output().out;
        if let Some(port) = out
            .lines()
            .find_map(|l| l.strip_prefix("listening "))
            .and_then(|p| p.parse::<u16>().ok())
        {
            break port;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the program never listened:\n{out}\n{}",
            program.output().err
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
    client.write_all(b"hello from rust\n").unwrap();
    let mut reply = String::new();
    BufReader::new(&client).read_line(&mut reply).unwrap();
    assert_eq!(reply, "got hello from rust\n");

    let output = program.finish();
    let expected = format!(
        "delay 0 keepalive 1 linger 5
rcvbuf positive true
option unknown socket option 9
negative negative value -1 for socket option 1
address 127.0.0.1 port true local 127.0.0.1 localport true
echo ping
available 10
more more data
second the input stream is already open
still after close
closed connection is closed
eof [] -1
refused java.io.IOException: connection refused
noport no port in socket://127.0.0.1
listening {port}
server closed connection is closed
done"
    );
    io_support::assert_output(&output, &expected);
}

/// A listener that accepts every connection and closes it at once, so the
/// test holds no more descriptors than the game does.
fn start_dropping_listener() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || for _ in listener.incoming() {});
    port
}

#[test]
fn a_game_holds_at_most_64_connections_and_8_server_sockets() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let listener = start_dropping_listener().to_string();
    let closed = io_support::closed_port().to_string();
    let output = io_support::run(
        classes,
        "SocketLimits",
        &[&listener, &closed],
        NetworkPolicy::Allow,
    );
    let full = "too many open connections";
    let expected = format!(
        "failed 70
held 64
65th {full}
http {full}
after close opened
stream open {full}
stream closed opened
all closed opened
accepted
second {full}
accepted again
forgotten 200
servers 8
9th too many open server sockets
server closed opened
done"
    );
    io_support::assert_output(&output, &expected);
}
