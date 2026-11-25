//! HttpsConnection and SecureConnection end to end: a Java program
//! (tests/io_java/TlsTest.java) talks to TLS servers on 127.0.0.1 whose
//! certificate comes from a test authority.
//!
//! tests/io_data/tls holds, in DER, a P-256 certificate authority
//! (`ca.der`, "FastEmu Test CA") and a server certificate it signed for
//! 127.0.0.1 and localhost (`server.der`, key `server-key.der` in
//! PKCS#8), both valid from 2026 to 2126. They were made once with
//! openssl and exist only for these tests.

mod io_support;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use fastemu_core::NetworkPolicy;
use fastemu_midp::io::tls::TrustedCertificates;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};

type TlsStream = StreamOwned<ServerConnection, TcpStream>;

fn data(name: &str) -> Vec<u8> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/io_data/tls");
    std::fs::read(dir.join(name)).unwrap()
}

/// The configuration of a server presenting the test certificate.
fn server_config() -> Arc<ServerConfig> {
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(data("server-key.der")));
    let config = ServerConfig::builder_with_provider(Arc::new(rustls_rustcrypto::provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![CertificateDer::from(data("server.der"))], key)
        .unwrap();
    Arc::new(config)
}

/// A TLS server on 127.0.0.1 presenting the test certificate, handing
/// each connection to `serve`.
fn start_server(serve: fn(&mut TlsStream)) -> u16 {
    let config = server_config();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for socket in listener.incoming() {
            let Ok(socket) = socket else { continue };
            let config = Arc::clone(&config);
            std::thread::spawn(move || {
                let conn = ServerConnection::new(config).unwrap();
                serve(&mut StreamOwned::new(conn, socket));
            });
        }
    });
    port
}

/// One line from `stream`, line feed included; `None` at the end or on
/// an error (such as a client that rejected the certificate).
fn read_line(stream: &mut TlsStream) -> Option<Vec<u8>> {
    let mut line = Vec::new();
    let mut byte = [0];
    while !line.ends_with(b"\n") {
        match stream.read(&mut byte) {
            Ok(1) => line.push(byte[0]),
            _ => return None,
        }
    }
    Some(line)
}

/// Answers one HTTPS request with `secure`.
fn https(stream: &mut TlsStream) {
    loop {
        match read_line(stream) {
            Some(line) if line == b"\r\n" => break,
            Some(_) => {}
            None => return,
        }
    }
    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\nsecure\n");
    stream.conn.send_close_notify();
    let _ = stream.flush();
}

/// Echoes lines back until the client hangs up.
fn echo(stream: &mut TlsStream) {
    while let Some(line) = read_line(stream) {
        if stream
            .write_all(&line)
            .and_then(|()| stream.flush())
            .is_err()
        {
            return;
        }
    }
}

#[test]
fn tls_connections_check_the_certificate_and_carry_data() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let https_port = start_server(https).to_string();
    let echo_port = start_server(echo).to_string();
    let trusted = io_support::run_with(
        classes,
        "TlsTest",
        &[&https_port, &echo_port, "trusted"],
        NetworkPolicy::Allow,
        |ctx| ctx.set_extension(TrustedCertificates(vec![data("ca.der")])),
    );
    io_support::assert_output(
        &trusted,
        "port true protocol https
info TLS 3.4 true
code 200 secure
default 443 true
ssl TLS 3.4
echo secret
available 6 again
done",
    );

    let untrusted = io_support::run(
        classes,
        "TlsTest",
        &[&https_port, &echo_port, "untrusted"],
        NetworkPolicy::Allow,
    );
    io_support::assert_output(&untrusted, "untrusted -> java.io.IOException true");
}

/// A server that sends what it has to say one byte every 10 ms until the
/// test drops it, and counts the bytes.
struct Dripper {
    port: u16,
    sent: Arc<AtomicUsize>,
    done: Arc<AtomicBool>,
}

impl Dripper {
    /// With `handshake` the server completes a real handshake and then
    /// drips one 16 KiB record of application data. Without it, it
    /// drips the body of a handshake record that claims 16 KiB, which
    /// the client's handshake cannot go on without.
    fn start(handshake: bool) -> Self {
        let config = server_config();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let sent = Arc::<AtomicUsize>::default();
        let done = Arc::<AtomicBool>::default();
        let (count, over) = (Arc::clone(&sent), Arc::clone(&done));
        std::thread::spawn(move || {
            for socket in listener.incoming() {
                let Ok(socket) = socket else { continue };
                let (config, count, over) = (Arc::clone(&config), count.clone(), over.clone());
                std::thread::spawn(move || {
                    let record = if handshake {
                        handshaken_record(config, &socket)
                    } else {
                        // The client's first flight, then a record header.
                        let mut hello = [0; 4096];
                        let _ = (&socket).read(&mut hello);
                        Some([&[0x16, 3, 3, 0x40, 0x00][..], &[0; 16384][..]].concat())
                    };
                    let Some(record) = record else { return };
                    for byte in record {
                        if over.load(Ordering::Relaxed) || (&socket).write_all(&[byte]).is_err() {
                            return;
                        }
                        count.fetch_add(1, Ordering::Relaxed);
                        std::thread::sleep(Duration::from_millis(10));
                    }
                });
            }
        });
        Self { port, sent, done }
    }

    /// Waits until the server has sent `bytes`.
    fn wait_for(&self, bytes: usize) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while self.sent.load(Ordering::Relaxed) < bytes {
            assert!(Instant::now() < deadline, "the server sent no data");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Dripper {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Relaxed);
    }
}

/// Completes a handshake with the client on `socket` and returns the
/// bytes of one record of 16000 bytes of data, which are not sent yet.
fn handshaken_record(config: Arc<ServerConfig>, mut socket: &TcpStream) -> Option<Vec<u8>> {
    let mut conn = ServerConnection::new(config).ok()?;
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .ok()?;
    while conn.is_handshaking() {
        conn.complete_io(&mut socket).ok()?;
    }
    conn.writer().write_all(&[b'x'; 16000]).ok()?;
    let mut record = Vec::new();
    while conn.wants_write() {
        conn.write_tls(&mut record).ok()?;
    }
    Some(record)
}

/// Starts `TlsDrip` against `server`, waits until the server has sent a
/// few bytes, stops the VM and checks that it ends.
fn stop_during_drip(classes: &Path, server: &Dripper, first: usize) {
    let port = server.port.to_string();
    let program =
        io_support::Program::start(classes, "TlsDrip", &[&port], NetworkPolicy::Allow, |ctx| {
            ctx.set_extension(TrustedCertificates(vec![data("ca.der")]));
        });
    server.wait_for(first);
    program.vm.stop();
    let ended = program.vm.wait_finished(Some(Duration::from_secs(10)));
    server.done.store(true, Ordering::Relaxed);
    let output = program.output();
    assert!(
        ended,
        "the VM did not end after stop(); it printed:\n{}\n{}",
        output.out, output.err
    );
}

#[test]
fn a_tls_handshake_that_trickles_in_ends_when_the_game_stops() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let server = Dripper::start(false);
    stop_during_drip(classes, &server, 100);
}

#[test]
fn a_tls_record_that_trickles_in_ends_when_the_game_stops() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let server = Dripper::start(true);
    stop_during_drip(classes, &server, 100);
}
