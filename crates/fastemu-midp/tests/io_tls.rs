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

use fastemu_core::NetworkPolicy;
use fastemu_midp::io::tls::TrustedCertificates;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};

type TlsStream = StreamOwned<ServerConnection, TcpStream>;

fn data(name: &str) -> Vec<u8> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/io_data/tls");
    std::fs::read(dir.join(name)).unwrap()
}

/// A TLS server on 127.0.0.1 presenting the test certificate, handing
/// each connection to `serve`.
fn start_server(serve: fn(&mut TlsStream)) -> u16 {
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(data("server-key.der")));
    let config = ServerConfig::builder_with_provider(Arc::new(rustls_rustcrypto::provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![CertificateDer::from(data("server.der"))], key)
        .unwrap();
    let config = Arc::new(config);
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
