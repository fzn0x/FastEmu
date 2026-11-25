//! UDPDatagramConnection and Datagram end to end: a Java program
//! (tests/io_java/DatagramTest.java) talks to a Rust echo peer and to
//! itself.

mod io_support;

use std::net::UdpSocket;

use fastemu_core::NetworkPolicy;

/// A UDP peer on 127.0.0.1 that sends every datagram back to its sender.
fn start_echo_peer() -> u16 {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let mut buf = vec![0; 65_536];
        while let Ok((n, from)) = socket.recv_from(&mut buf) {
            let _ = socket.send_to(&buf[..n], from);
        }
    });
    port
}

#[test]
fn datagrams_carry_data_both_ways() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let peer = start_echo_peer().to_string();
    let output = io_support::run(classes, "DatagramTest", &[&peer], NetworkPolicy::Allow);
    let expected = "max 65507 nominal 1472 localport true
new 100 0 true
written 41
received 41 from peer true
true -5 1099511627776 true -2 Z 1.5 -0.25 7 ok
eof
truncated 6 012345
length invalid length 101
address invalid datagram address \"socket://x:1\"
size invalid datagram size -1
full the datagram buffer is full
closed connection is closed
server got hello server from client true
client got hello client
no address null
unaddressed the datagram has no address
receive on close -> connection is closed
done";
    io_support::assert_output(&output, expected);
}

#[test]
fn a_game_holds_at_most_16_datagram_connections() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    // Nothing is sent: the clients only name the peer.
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = peer.local_addr().unwrap().port().to_string();
    let output = io_support::run(classes, "DatagramLimits", &[&port], NetworkPolicy::Allow);
    let full = "too many open datagram sockets";
    let expected = format!(
        "held 16
17th {full}
client {full}
after close opened
forgotten opened
all closed opened
done"
    );
    io_support::assert_output(&output, &expected);
}
