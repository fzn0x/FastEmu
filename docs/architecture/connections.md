# Connections

Games open every network and messaging connection through the Generic
Connection Framework: `javax.microedition.io.Connector.open(name)`, where
the scheme of the name picks the protocol. FastEmu implements the
framework in `crates/fastemu-midp/src/io` and the Wireless Messaging API
(JSR-120) in `crates/fastemu-midp/src/wma`, as native classes on the
virtual machine (see [jvm.md](jvm.md) for the rules native code follows).

Games used the network for score upload, ads, licence checks,
downloadable content and a few multiplayer modes. Most of the servers are
gone, so the first job is to fail the way a phone without signal failed:
an `IOException` the game catches, never a crash or a hang. The second is
to make real connections work for the games and servers that still do.

## What a name opens

| Name | Returns | Network policy | Notes |
| --- | --- | --- | --- |
| `http://host[:port]/file?query#ref` | `HttpConnection` | checked | HTTP/1.1, lazy connect |
| `https://...` | `HttpsConnection` | checked | the same over TLS |
| `socket://host:port` | `SocketConnection` | checked | TCP client |
| `socket://:port`, `socket://:` | `ServerSocketConnection` | checked | TCP server on every interface |
| `ssl://host:port` | `SecureConnection` | checked | TCP client over TLS |
| `datagram://host:port` | `UDPDatagramConnection` | checked | UDP, sends to that address |
| `datagram://:port`, `datagram://:` | `UDPDatagramConnection` | checked | UDP, receives from anyone |
| `sms://+number[:port]` | `MessageConnection` | not checked | client: sends only |
| `sms://:port`, `cbs://:port` | `MessageConnection` | not checked | server: receives |
| `comm:...` | throws `ConnectionNotFoundException` | | this phone has no serial ports |
| anything else (`file:`, `btspp:`, ...) | throws `ConnectionNotFoundException` | | "the requested protocol does not exist" |

`Connector.open` also throws `IllegalArgumentException` for a `null`
name ("Null URL", as the reference implementation says), a name without
a scheme, a mode other than `READ`, `WRITE` or `READ_WRITE`, and a name
its protocol cannot parse (a socket without a port, an SMS address that
is not a number). `Connector.openInputStream` and the other three stream
helpers open the connection, take one stream and close the connection;
the stream keeps it alive.

The mode is enforced: a connection opened `READ` refuses
`openOutputStream` and one opened `WRITE` refuses `openInputStream`,
both with `IOException`. Each connection hands out at most one input and
one output stream, as the reference implementation did.

## The network policy

`InstanceEnv.settings.network` (`fastemu_core::NetworkPolicy`) decides.
With `Deny`, the default, every open of a network scheme throws
`IOException("network access is off for this game")` from
`Connector.open` itself, before any name lookup. A VM without an
`InstanceEnv` has no network either. With `Allow`, connections are real
`std::net` sockets to any address. `Allow` means the whole network: a game
with it can reach programs on the computer itself, devices on the local
network and link-local addresses, not only the internet.

Messaging is not network access: `sms://` and `cbs://` open under either
policy, as a phone without signal still opened them, and their sends
fail anyway (see below).

## Blocking without freezing the VM

A Java thread runs only while it holds its VM's lock, so native code that
blocks with the lock held freezes every thread of the game. All socket
work therefore runs through `io::net`:

- Every socket gets read and write timeouts of one slice, 100 ms.
- `net::block_on(ctx, deadline, attempt)` runs one bounded attempt (one
  system call) inside `Ctx::without_lock`, and between attempts, with
  the lock held, checks three things: a stopping VM unwinds with
  `Throw::Stop`; an interrupted thread (`Thread.interrupt`) gets
  `InterruptedIOException("interrupted")`; a passed deadline gives
  `InterruptedIOException("timed out")`.
- Name lookups and connects cannot be bounded that way, so
  `net::off_thread` runs them on a helper thread and waits for its answer
  in slices. A thread that stops waiting, at a stop, an interrupt or its
  deadline, leaves the helper to finish on its own; its result is
  dropped, and a socket it opened is closed. Each helper holds one of
  the game's 16 places for helpers until it ends (see [What a game may
  hold](#what-a-game-may-hold)), and a stopping VM starts none.

Deadlines: resolving and connecting may take 30 s together, the wait for
a helper's place included. The addresses of a name share them: each is
tried with the time left and none after it, so a helper ends by then,
unless the name lookup itself takes longer, which only the system's
resolver bounds. A read or write waits at most 30 s when the game opened
the connection with `timeouts` set, and for as long as it takes
otherwise (the VM's stop still ends it). HTTP waits at most 60 s for a
response head whatever the flag says, as a phone's network gave up on a
silent server too.

An attempt must end after one system call, whatever the peer sends. A
loop inside one attempt would never get back to the three checks above,
so a server that sends without pause could keep a stopped game's thread
alive. The HTTP reads follow this: reading a head takes in one read per
attempt and gives up the attempt after each interim response, and
reading a body gives it up after a read that brought no body bytes (a
chunk header, a trailer line). A job that takes one attempt for each
piece, such as filling a read buffer or writing a large array, calls
`net::check` between the pieces, because a peer that sends or takes a
little at a time makes every attempt succeed. rustls reads again and
again inside one call while a handshake message or a record is
incomplete, so `io::tls` gives it a socket that allows one read per call
and says `WouldBlock` to the next, which ends the attempt as a slice
with nothing more does.

I/O errors become `IOException` with short fixed messages ("connection
refused", "connection reset by the peer"), not the host's own text,
which differs between systems, is in the host's language and names the
host. The kinds without a message of their own read "I/O error". Only
the texts FastEmu wrote itself ("the response ended 5 bytes early") and
the TLS library's ("TLS handshake with host failed: ...") pass on. A
name without addresses throws `ConnectionNotFoundException("cannot find
host ...")`.

## What a game may hold

A game that may use the network can still not make the host hold more
than a phone would. Every limit but the one on helpers throws
`IOException` from the call that passes it, with a fixed text, and none
of them is a setting.

| What | Limit | Where it is checked |
| --- | --- | --- |
| open connections: sockets, secure sockets, accepted sockets and HTTP connections | 64 | `Connector.open`, `acceptAndOpen` |
| open server sockets | 8 | `Connector.open` |
| open datagram connections | 16 | `Connector.open` |
| helper threads of name lookups and connects, the ones the game stopped waiting for included | 16 | `Connector.open`, the first HTTP request, `send` to a datagram address by name; past it the call waits |
| the request body of one HTTP connection | 4 MiB | `OutputStream.write` |
| the request bodies of all of a game's HTTP connections, not yet sent | 8 MiB | `OutputStream.write` |
| request headers of one HTTP connection | 64, and 16 KiB of names and values | `setRequestProperty` |
| a connection name | 8 KiB | `Connector.open` (`IllegalArgumentException`) |
| interim `1xx` responses before the final one | 16 | the response getters |
| the trailer of a chunked body | 16 KiB | `InputStream.read` |
| the size line of one chunk, extensions included | 1 KiB | `InputStream.read` |
| a response head | 64 KiB | the response getters |

Each open connection of a game has a place, which `net::acquire` takes
before anything opens and which is given back when the socket is closed:
the connection and all its streams are closed, or the collector frees a
connection the game never closed. A game at its limit gets one
collection and a second try before the exception, because games written
for phones open a connection for each request and close the stream but
not the connection. A connection that fails to open gives its place
back at once, while a helper still connecting for it keeps its own place
among the 16 for helpers until it ends. A closed connection lets go of
its socket at once, so the descriptor is free when the place is, not
when the collector gets to the object. A server that has no place for a
client leaves it in the listener's queue: `acceptAndOpen` throws before
it accepts.

The counts are an extension of the VM (`ConnectionSlots`), so every game
has its own, and they hold no Java references.

A name lookup or a connect takes one of the game's 16 places for helpers
before its thread starts, and the helper gives it back when its work
ends, not when the game stops waiting. The count keeps a game that
interrupts its own connects in a loop from leaving a thread and a socket
behind each time. At the limit the call does not throw: it waits for a
helper to end, in slices with the same checks as any network wait, so
an interrupt, a stop or its 30 s end it, and no thread starts meanwhile.
The game sees what a slow network shows it, and a game that opens a few
connections at a time never waits.

A request body is kept in the host until the request goes out, and the
heap does not count it, so it has its own two limits: 4 MiB for a
connection and 8 MiB for all of a game's connections together. Opening
64 connections that each hold 4 MiB would otherwise make the host hold
256 MiB for a game whose heap is 2 MiB. The share of a request goes back
when the request is written. Names and headers are kept in the host for
as long as the connection lives, which is why they are limited too.

What stays open on purpose: what `Allow` reaches (see the policy section),
as on a phone on a network, and server sockets that listen on every
interface. The time zone and the local address (`getLocalAddress`) are
visible to the game. Allowing the network is the user's choice per game.

[SECURITY.md](../../SECURITY.md#limits-a-game-cannot-pass) lists these
limits with every other limit a game meets, and what Allow leaves open
among its [known limits](../../SECURITY.md#known-limits).

## HTTP

`io::http` speaks HTTP/1.1 with the three states of the MIDP javadoc.

- **Setup**: `setRequestMethod` (GET, POST or HEAD, case-sensitive; any
  other throws `IOException`) and `setRequestProperty` (which replaces a
  header of the same name, ignoring case) work. Opening the output stream
  makes later changes ignored, as the javadoc says.
- **Connected**: the request goes out when the game first needs the
  response (any response getter, `openInputStream`) or closes the output
  stream. After that `setRequestMethod`, `setRequestProperty` and
  `openOutputStream` throw `IOException("the request was already sent")`.
  The response head is read when a getter first needs it.
- **Closed**: after `close`, every method that needs the server throws
  `IOException("connection is closed")`; `getURL`, `getHost` and the other
  getters of the name keep working. Streams opened before `close` keep
  working, and the socket closes when the last of them closes.

The javadoc names `IOException` for every state error; none throws
`IllegalStateException`.

A failure to connect or to read the response is remembered: every later
call that needs the response throws the same exception again.
`getType`, `getEncoding` and `getLength` declare no `IOException`, so a
failed connection makes them return `null` and -1 instead.

The request carries `Host` (unless the game set one: some reach a
virtual host through an address), the game's headers in the order it set
them, `User-Agent` (the platform, then `Profile/MIDP-2.0
Configuration/CLDC-1.1`, unless the game set one), `Content-Length` for
a body or a POST, and `Connection: close`, so every response ends with
the connection and there is no keep-alive state. Bytes HTTP does not
allow in the request target (spaces, controls, text beyond ASCII) are
percent-encoded, so a game that put a space in a query string still
sends a valid request. Redirects are not followed: phones handed 3xx
codes to the game.

The output stream collects the body in memory and sends it, with its
length, when the stream closes or the response is first needed.
`flush` sends nothing: a body has to be complete for its
`Content-Length`, and the emulators these games were tested on buffered
the same way. A stream that outlives its connection still sends when it
closes, which keeps fire-and-forget score uploads working. The body is
limited (see [What a game may hold](#what-a-game-may-hold)): a write that
would pass 4 MiB throws `IOException` and keeps what was collected. The
request takes over the body's buffer, so a large body is not copied.

The response: a status line that is not HTTP makes `getResponseCode`
-1, `getResponseMessage` `null`, and the whole reply the body (an
HTTP/0.9 server). Interim `1xx` responses are skipped. Headers keep the
server's spelling and order for `getHeaderField(int)` and
`getHeaderFieldKey(int)` (index 0 is the first header, not the status
line); a name lookup ignores case and the last of repeated headers wins.
Dates parse in the three forms RFC 2616 lists. The body is framed by RFC
7230: none for HEAD, 1xx, 204 and 304; chunked; `Content-Length`; or up
to the close. A body cut short throws `IOException`.

`read(byte[])` on a response body fills the buffer up to the end of the
body. Games often size one read by `getLength()`, which a phone's
buffered HTTP stack answered in full. `available()` counts the body bytes
already received.

## Sockets

`io::socket` implements `SocketConnection` over a `TcpStream`, shared by
the connection and its streams.

- `read(byte[])` returns what has arrived, at least one byte: socket
  protocols send a request and wait for the reply, so a read must not
  wait to fill its buffer.
- `available()` is real, because games poll it every frame to read only
  what arrived. std::net has no `FIONREAD`, so the socket peeks with
  blocking switched off for a moment; a read or write another thread
  starts meanwhile sees `WouldBlock`, which every caller treats as "try
  again".
- The five options go through socket2: `DELAY` 0 turns Nagle's algorithm
  off (the javadoc's "small buffer writing delay"), `LINGER` is seconds,
  `KEEPALIVE` is 0 or 1, `RCVBUF` and `SNDBUF` are the kernel's buffer
  sizes. A negative value or an unknown option throws
  `IllegalArgumentException`; `getSocketOption` returns -1 when the
  system cannot tell.
- `getLocalAddress` of a server bound to every interface reports the
  address the host would use to reach the network, as a phone reported
  its own IP.

`ServerSocketConnection.acceptAndOpen` polls a non-blocking listener
every 50 ms, so a stop or a `close` from another thread ends the wait.
`close` drops the listener, so the game can listen on the same port
again at once. Closing a client connection lets go of the socket as soon
as its last stream is closed.

## Datagrams

`io::datagram` implements `UDPDatagramConnection` over a `UdpSocket`.
A client (`datagram://host:port`) resolves the address once and sends
there unless a datagram names another; a server (`datagram://:port`)
receives from anyone and its datagrams carry their own addresses.
`getMaximumLength` is 65507 (the largest UDP payload over IPv4) and
`getNominalLength` 1472 (one Ethernet frame). `close` drops the socket,
which frees its port at once; a `receive` waiting on another thread
ends within one read slice.

A `Datagram` has a buffer, an offset, a length, a read/write pointer and
an address (`datagram://host:port`). Following the reference
implementation: `reset` zeroes the offset, length and pointer; writes go
at the pointer and set the length to it, and writing past the buffer
throws `IOException`; reads stop at the length with `EOFException`;
`receive` puts the datagram at the offset, truncated to the room there
(a full-size buffer is used underneath, since Windows reports a datagram
larger than the buffer as an error), sets the length and the sender's
address, and leaves the pointer alone. `send` and `receive` go through
the `Datagram` interface, so a game's own `Datagram` class works too.

`Datagram` extends `java.io.DataInput` and `DataOutput`, which the CLDC
library provides; `DatagramImpl` implements every method of both on its
buffer.

## TLS

`io::tls` runs `https://` and `ssl://` on rustls with the RustCrypto
provider. It is pure Rust, so it builds for every desktop target and for
`aarch64-linux-android` without a C toolchain. It speaks TLS 1.2 and 1.3
and checks the server's certificate against Mozilla's roots
(webpki-roots). The SSL 3.0 and TLS 1.0 servers of the phones' era are
gone; the servers games still reach speak TLS 1.2 or later.

The handshake completes right after the connect, so a bad certificate
fails as `IOException("TLS handshake with host failed: ...")`.
`SecurityInfo` reports the protocol name `TLS`, the version numbers the
javadoc asks for ("3.3" for TLS 1.2, "3.4" for TLS 1.3; "3.1" would be
TLS 1.0) and the cipher suite's RFC name. `getServerCertificate` returns
`null` until `javax.microedition.pki` exists.

A launcher can add roots for one instance with the
`fastemu_midp::io::tls::TrustedCertificates` extension (DER
certificates), for a game server with a private authority. The tests
use it with their own authority in `tests/io_data/tls`.

## Messaging (WMA)

`wma` implements `MessageConnection`, `TextMessage`, `BinaryMessage` and
`MessageListener`. FastEmu never sends a message and none ever arrives:

- `send` checks the message as JSR-120 asks (`NullPointerException` for
  `null`, `IllegalArgumentException` without an address or when it needs
  more than three segments), logs the attempt at `LogLevel::Warn` so the
  user can see that a game tried to send an SMS (often a paid one), and
  throws `IOException("sending messages is not supported")`, as a phone
  without service did. It never pretends a message was sent.
- `receive` on a client connection throws `IOException`; on a server it
  waits until the connection closes and then throws
  `InterruptedIOException`, as the javadoc says.
- `setMessageListener` keeps the listener (in a Java field, where the
  collector sees it) and never calls it. A client connection refuses it
  with `IOException`.
- `numberOfSegments` counts as a GSM phone did: GSM 7-bit text when every
  character is in the GSM 03.38 alphabet (160 characters alone, 152 per
  segment of a longer message, less with a port), UCS-2 otherwise (70 and
  66), binary as octets (140 and 133); 0 beyond three segments.
- A server port can be open once per VM; a second open throws
  `IOException` until the first closes. `cbs://` connections only
  receive.
- `getTimestamp` is `null`: no message was ever received.

## PushRegistry

A phone could start a MIDlet for an inbound message or connection, or at
a set time. FastEmu starts games only when the user does, so
`registerConnection` checks its arguments (a connection string, a
filter, a MIDlet the suite lists) and logs the request, and nothing is
kept: `listConnections` is empty, `getMIDlet` and `getFilter` are
`null`, `unregisterConnection` is `false`, and `registerAlarm` returns 0,
no alarm having been set before.

## Testing

- Unit tests sit beside the code: name parsing, the HTTP wire format and
  body decoding fed in every split, dates, datagram addresses and strict
  modified UTF-8, segment counts, the interfaces' constants. The helpers'
  limit and the shared connect deadline are unit tests in `io::net`, not
  tests against a slow network, since no loopback address drops packets
  on every system: Windows refuses a full listener after 2 s of retries,
  Linux and macOS drop the SYN. The limit tests hold helpers on a gate
  the test opens, and the deadline tests run `connect_each` with a fake
  connect.
- `tests/io_*.rs` and `tests/wma_messages.rs` run Java programs from
  `tests/io_java` on the VM against peers on 127.0.0.1: a scripted HTTP
  server (GET, POST, HEAD, fixed, chunked and open-ended bodies, a reply
  that is not HTTP, the state errors, a refused connection), a server
  that never answers (interrupt and stop), servers that send interim
  responses or trailers for ever or a head one byte at a time (the
  game ends with an error, or when the VM stops), TCP and UDP echo peers,
  TLS servers with a test authority (and ones that send a handshake or a
  record one byte at a time, which end when the VM stops), and the `Deny`
  policy. The limit tests open a few more than the limits allow (65
  sockets, 9 listeners, 17 UDP sockets) and write 5 MiB of body, all on
  loopback. Connects that must fail go to port 0, which fails at once on
  every system. The programs compile with `javac --release 8` against
  stubs of the javax.microedition classes in `tests/io_java/stubs`;
  without a JDK the tests print why and pass, unless
  `FASTEMU_REQUIRE_JAVA=1` is set.
- Until the CLDC library has `java.io.DataInput` and `DataOutput`, the
  tests add stand-ins for both, so `Datagram` loads; the
  `openDataInputStream` case runs once `java.io.DataInputStream` exists
  and is skipped with a note before.
