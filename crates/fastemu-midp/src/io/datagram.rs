//! `datagram://`: `UDPDatagramConnection` over a `std::net` UDP socket,
//! and the `Datagram` objects it sends and receives.
//!
//! `datagram://host:port` is a client: its datagrams go to that address
//! unless they name another. `datagram://:port` (or `datagram://:` for
//! any port) receives from anyone, and its datagrams carry their own
//! addresses. A `Datagram` is a buffer with an offset, a length, a
//! read/write pointer and an address, read and written through
//! `DataInput` and `DataOutput`.

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};

use fastemu_jvm::classfile::encode_modified_utf8;
use fastemu_jvm::{ArrayType, Ctx, FieldId, JResult, NativeClass, ObjRef, Payload, Value};

use super::net::{self, IO_TIMEOUT};
use super::{Options, check_range, copy_in, copy_out, lock, opt_string_value, this, throw_io, url};

const UDP_IMPL: &str = "fastemu/io/UDPDatagramConnectionImpl";
const DATAGRAM_IMPL: &str = "fastemu/io/DatagramImpl";

/// The largest UDP payload over IPv4, what `getMaximumLength` reports.
const MAX_LENGTH: i32 = 65_507;

/// What `getNominalLength` reports: the payload of one Ethernet frame
/// (1500 bytes less the IP and UDP headers), which travels without being
/// split.
const NOMINAL_LENGTH: i32 = 1472;

/// The socket of one datagram connection.
struct UdpShared {
    /// The socket, until `close` drops it so the port is free for the game
    /// to bind again at once rather than when the collector frees the
    /// connection object. A send or receive works on its own handle, so a
    /// thread waiting for a datagram never holds up another one's send.
    socket: Mutex<Option<Arc<UdpSocket>>>,
    /// The socket's place among the game's datagram connections, given
    /// back by `close` with the socket.
    slot: Mutex<Option<net::Slot>>,
    /// For a client, the address the name gave, as text and resolved.
    target: Option<(String, SocketAddr)>,
    options: Options,
}

impl UdpShared {
    /// The socket, or `None` once the connection is closed.
    fn socket(&self) -> Option<Arc<UdpSocket>> {
        lock(&self.socket).clone()
    }
}

/// The payload of a datagram connection object.
#[derive(Default)]
struct UdpPayload(Option<Arc<UdpShared>>);

impl Payload for UdpPayload {}

/// A datagram address, `datagram://host:port`, split.
fn parse_address(text: &str) -> Option<(String, u16)> {
    let (scheme, rest) = url::split_scheme(text).ok()?;
    let target = url::host_port(rest).ok()?;
    (scheme == "datagram").then_some(())?;
    Some((target.host?, target.port?))
}

/// The address of `addr` as a datagram address string.
fn address_text(addr: SocketAddr) -> String {
    match addr {
        SocketAddr::V4(a) => format!("datagram://{}:{}", a.ip(), a.port()),
        SocketAddr::V6(a) => format!("datagram://[{}]:{}", a.ip(), a.port()),
    }
}

/// Opens `datagram://host:port` or `datagram://:port`.
pub(crate) fn open(ctx: &mut Ctx<'_>, rest: &str, name: &str, options: Options) -> JResult<ObjRef> {
    let target = match url::host_port(rest) {
        Ok(t) => t,
        Err(message) => return ctx.throw("java/lang/IllegalArgumentException", &message),
    };
    let client = match (&target.host, target.port) {
        (Some(host), Some(port)) => Some((host.clone(), port)),
        (Some(_), None) => {
            let message = format!("no port in {name}");
            return ctx.throw("java/lang/IllegalArgumentException", &message);
        }
        (None, _) => None,
    };
    net::check_network(ctx)?;
    let slot = net::acquire(ctx, net::Kind::Datagram)?;
    let (bind, target) = match client {
        Some((host, port)) => {
            let addr = net::resolve(ctx, &host, port)?;
            let any: SocketAddr = if addr.is_ipv6() {
                "[::]:0".parse().expect("a valid address")
            } else {
                "0.0.0.0:0".parse().expect("a valid address")
            };
            (any, Some((format!("datagram://{host}:{port}"), addr)))
        }
        None => (
            SocketAddr::from(([0, 0, 0, 0], target.port.unwrap_or(0))),
            None,
        ),
    };
    let socket = UdpSocket::bind(bind).and_then(|s| {
        s.set_read_timeout(Some(net::SLICE))?;
        s.set_write_timeout(Some(net::SLICE))?;
        Ok(s)
    });
    let socket = match socket {
        Ok(s) => s,
        Err(e) => return Err(net::io_exception(ctx, &e)),
    };
    let class = ctx.find_class(UDP_IMPL)?;
    let conn = ctx.new_object(class)?;
    let shared = UdpShared {
        socket: Mutex::new(Some(Arc::new(socket))),
        slot: Mutex::new(Some(slot)),
        target,
        options,
    };
    ctx.set_payload(conn, UdpPayload(Some(Arc::new(shared))));
    Ok(conn)
}

/// The connection `conn` and its socket, or `IOException` once closed.
fn shared(ctx: &mut Ctx<'_>, conn: ObjRef) -> JResult<(Arc<UdpShared>, Arc<UdpSocket>)> {
    let shared = match &ctx.payload::<UdpPayload>(conn)?.0 {
        Some(s) => Arc::clone(s),
        None => return throw_io(ctx, "the connection was not opened with Connector.open"),
    };
    let Some(socket) = shared.socket() else {
        return throw_io(ctx, "connection is closed");
    };
    Ok((shared, socket))
}

/// The fields of `DatagramImpl`.
struct Fields {
    buf: FieldId,
    offset: FieldId,
    length: FieldId,
    pos: FieldId,
    address: FieldId,
}

fn fields(ctx: &mut Ctx<'_>) -> JResult<Fields> {
    Ok(Fields {
        buf: ctx.field_id(DATAGRAM_IMPL, "buf", "[B")?,
        offset: ctx.field_id(DATAGRAM_IMPL, "offset", "I")?,
        length: ctx.field_id(DATAGRAM_IMPL, "length", "I")?,
        pos: ctx.field_id(DATAGRAM_IMPL, "pos", "I")?,
        address: ctx.field_id(DATAGRAM_IMPL, "address", "Ljava/lang/String;")?,
    })
}

fn illegal<T>(ctx: &mut Ctx<'_>, message: &str) -> JResult<T> {
    ctx.throw("java/lang/IllegalArgumentException", message)
}

/// A new `Datagram` over `buf`, `len` bytes long, addressed to `address`.
fn new_datagram(
    ctx: &mut Ctx<'_>,
    buf: ObjRef,
    len: i32,
    address: Option<ObjRef>,
) -> JResult<ObjRef> {
    let class = ctx.find_class(DATAGRAM_IMPL)?;
    let dgram = ctx.new_object(class)?;
    let f = fields(ctx)?;
    ctx.set_field(dgram, f.buf, buf.into());
    ctx.set_field(dgram, f.length, Value::Int(len));
    ctx.set_field(dgram, f.address, address.into());
    Ok(dgram)
}

/// Checks a datagram address given as a Java string: `null` is allowed
/// (no address), anything else must be `datagram://host:port`.
fn checked_address(ctx: &mut Ctx<'_>, address: Value) -> JResult<Option<ObjRef>> {
    let Some(text) = ctx.opt_string(address)? else {
        return Ok(None);
    };
    if parse_address(&text).is_none() {
        return illegal(ctx, &format!("invalid datagram address {text:?}"));
    }
    Ok(address.obj())
}

/// `newDatagram` in its four forms: a buffer (`None` for a new one of
/// `size` bytes) and an address (`None` for the connection's own).
fn new_datagram_for(
    ctx: &mut Ctx<'_>,
    conn: ObjRef,
    buf: Option<Value>,
    size: i32,
    address: Option<Value>,
) -> JResult<Value> {
    let (shared, _) = shared(ctx, conn)?;
    if !(0..=MAX_LENGTH).contains(&size) {
        return illegal(ctx, &format!("invalid datagram size {size}"));
    }
    let buf = match buf {
        Some(given) => {
            let Some(arr) = given.obj() else {
                return illegal(ctx, "the datagram buffer is null");
            };
            if ctx.array_len(arr)? < size as usize {
                return illegal(ctx, &format!("size {size} is larger than the buffer"));
            }
            arr
        }
        None => ctx.new_array(ArrayType::Byte, size)?,
    };
    let address = match address {
        Some(given) => checked_address(ctx, given)?,
        None => match &shared.target {
            Some((text, _)) => Some(ctx.new_string(text)?),
            None => None,
        },
    };
    Ok(new_datagram(ctx, buf, size, address)?.into())
}

/// `send`: the datagram's bytes from its offset, to its address or the
/// connection's.
fn send(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let (shared, socket) = shared(ctx, this(a))?;
    let dgram = ctx.non_null(a[1])?;
    let address = ctx.call_interface(dgram, "getAddress", "()Ljava/lang/String;", &[])?;
    let data = ctx.call_interface(dgram, "getData", "()[B", &[])?;
    let offset = ctx.call_interface(dgram, "getOffset", "()I", &[])?.int();
    let length = ctx.call_interface(dgram, "getLength", "()I", &[])?.int();
    let to = match ctx.opt_string(address)? {
        Some(text) => match parse_address(&text) {
            Some((host, port)) => match &shared.target {
                Some((own, addr)) if *own == text => *addr,
                _ => net::resolve(ctx, &host, port)?,
            },
            None => return illegal(ctx, &format!("invalid datagram address {text:?}")),
        },
        None => match &shared.target {
            Some((_, addr)) => *addr,
            None => return throw_io(ctx, "the datagram has no address"),
        },
    };
    let bytes = match data.obj() {
        Some(arr) => {
            let size = ctx.array_len(arr)?;
            let (off, n) = check_range(ctx, offset, length, size)?;
            copy_out(ctx, arr, off, n)?
        }
        None => Vec::new(),
    };
    let deadline = net::deadline(shared.options.timeouts.then_some(IO_TIMEOUT));
    net::block_on(ctx, deadline, || net::again(socket.send_to(&bytes, to)))?;
    Ok(Value::Void)
}

/// `receive`: waits for a datagram and puts it in the buffer at the
/// offset, truncated to the room there, with the sender's address.
fn receive(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let (shared, socket) = shared(ctx, this(a))?;
    let dgram = ctx.non_null(a[1])?;
    let data = ctx.call_interface(dgram, "getData", "()[B", &[])?;
    let offset = ctx.call_interface(dgram, "getOffset", "()I", &[])?.int();
    let room = match data.obj() {
        Some(arr) => ctx.array_len(arr)?.saturating_sub(offset.max(0) as usize),
        None => 0,
    };
    let deadline = net::deadline(shared.options.timeouts.then_some(IO_TIMEOUT));
    // Receive into a full-size buffer: on Windows a datagram larger than
    // the buffer is an error, not a truncation.
    let mut buf = vec![0; MAX_LENGTH as usize];
    let (n, from) = net::block_on(ctx, deadline, || {
        if shared.socket().is_none() {
            return Err(io::Error::other("connection is closed"));
        }
        net::again(socket.recv_from(&mut buf))
    })?;
    // A game's own Datagram class may give an offset past its buffer; it
    // then has no room, and nothing is copied.
    let n = n.min(room);
    if let Some(arr) = data.obj().filter(|_| n > 0) {
        copy_in(ctx, arr, offset.max(0) as usize, &buf[..n])?;
    }
    ctx.call_interface(dgram, "setLength", "(I)V", &[Value::Int(n as i32)])?;
    let address = ctx.new_string(&address_text(from))?;
    ctx.call_interface(
        dgram,
        "setAddress",
        "(Ljava/lang/String;)V",
        &[address.into()],
    )?;
    Ok(Value::Void)
}

fn connection_class() -> NativeClass {
    NativeClass::new(UDP_IMPL)
        .implements("javax/microedition/io/UDPDatagramConnection")
        .payload::<UdpPayload>()
        .method("close", "()V", |ctx, a| {
            if let Some(shared) = &ctx.payload::<UdpPayload>(this(a))?.0 {
                lock(&shared.socket).take();
                lock(&shared.slot).take();
            }
            Ok(Value::Void)
        })
        .method("getMaximumLength", "()I", |ctx, a| {
            shared(ctx, this(a))?;
            Ok(Value::Int(MAX_LENGTH))
        })
        .method("getNominalLength", "()I", |ctx, a| {
            shared(ctx, this(a))?;
            Ok(Value::Int(NOMINAL_LENGTH))
        })
        .method("send", "(Ljavax/microedition/io/Datagram;)V", send)
        .method("receive", "(Ljavax/microedition/io/Datagram;)V", receive)
        .method(
            "newDatagram",
            "(I)Ljavax/microedition/io/Datagram;",
            |ctx, a| new_datagram_for(ctx, this(a), None, a[1].int(), None),
        )
        .method(
            "newDatagram",
            "(ILjava/lang/String;)Ljavax/microedition/io/Datagram;",
            |ctx, a| new_datagram_for(ctx, this(a), None, a[1].int(), Some(a[2])),
        )
        .method(
            "newDatagram",
            "([BI)Ljavax/microedition/io/Datagram;",
            |ctx, a| new_datagram_for(ctx, this(a), Some(a[1]), a[2].int(), None),
        )
        .method(
            "newDatagram",
            "([BILjava/lang/String;)Ljavax/microedition/io/Datagram;",
            |ctx, a| new_datagram_for(ctx, this(a), Some(a[1]), a[2].int(), Some(a[3])),
        )
        .method("getLocalAddress", "()Ljava/lang/String;", |ctx, a| {
            let (_, socket) = shared(ctx, this(a))?;
            match socket.local_addr() {
                Ok(addr) => opt_string_value(ctx, Some(&addr.ip().to_string())),
                Err(e) => Err(net::io_exception(ctx, &e)),
            }
        })
        .method("getLocalPort", "()I", |ctx, a| {
            let (_, socket) = shared(ctx, this(a))?;
            match socket.local_addr() {
                Ok(addr) => Ok(Value::Int(i32::from(addr.port()))),
                Err(e) => Err(net::io_exception(ctx, &e)),
            }
        })
}

/// Where a datagram's data starts and how far it may be read and
/// written.
struct Window {
    buf: Option<ObjRef>,
    offset: i32,
    length: i32,
    pos: i32,
}

fn window(ctx: &mut Ctx<'_>, f: &Fields, dgram: ObjRef) -> Window {
    Window {
        buf: ctx.get_field(dgram, f.buf).obj(),
        offset: ctx.get_field(dgram, f.offset).int(),
        length: ctx.get_field(dgram, f.length).int(),
        pos: ctx.get_field(dgram, f.pos).int(),
    }
}

/// Reads `n` bytes at the pointer, or throws `EOFException` when fewer
/// than `n` are left before the datagram's length.
fn take(ctx: &mut Ctx<'_>, dgram: ObjRef, n: usize) -> JResult<Vec<u8>> {
    let f = fields(ctx)?;
    let w = window(ctx, &f, dgram);
    let left = (w.length - w.pos).max(0) as usize;
    let start = (w.offset + w.pos).max(0) as usize;
    let fits = match w.buf {
        Some(arr) => start + n <= ctx.array_len(arr)?,
        None => n == 0,
    };
    if n > left || !fits {
        return ctx.throw("java/io/EOFException", "end of the datagram");
    }
    let bytes = match w.buf {
        Some(arr) => copy_out(ctx, arr, start, n)?,
        None => Vec::new(),
    };
    ctx.set_field(dgram, f.pos, Value::Int(w.pos + n as i32));
    Ok(bytes)
}

/// Writes `bytes` at the pointer and makes the length end after them, as
/// the reference implementation does.
fn put(ctx: &mut Ctx<'_>, dgram: ObjRef, bytes: &[u8]) -> JResult<()> {
    let f = fields(ctx)?;
    let w = window(ctx, &f, dgram);
    let start = (w.offset + w.pos).max(0) as usize;
    let Some(arr) = w.buf else {
        return throw_io(ctx, "the datagram has no buffer");
    };
    if start + bytes.len() > ctx.array_len(arr)? {
        return throw_io(ctx, "the datagram buffer is full");
    }
    copy_in(ctx, arr, start, bytes)?;
    let pos = w.pos + bytes.len() as i32;
    ctx.set_field(dgram, f.pos, Value::Int(pos));
    ctx.set_field(dgram, f.length, Value::Int(pos));
    Ok(())
}

/// Reads `N` bytes at the pointer.
fn take_n<const N: usize>(ctx: &mut Ctx<'_>, dgram: ObjRef) -> JResult<[u8; N]> {
    let bytes = take(ctx, dgram, N)?;
    let mut out = [0; N];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Decodes modified UTF-8 strictly, as `readUTF` must: `None` for bytes
/// that are not valid.
fn decode_utf(bytes: &[u8]) -> Option<Vec<u16>> {
    let mut out = Vec::with_capacity(bytes.len());
    let cont = |b: Option<&u8>| {
        b.filter(|&&b| b & 0xC0 == 0x80)
            .map(|&b| u16::from(b & 0x3F))
    };
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b & 0x80 == 0 {
            out.push(u16::from(b));
            i += 1;
        } else if b & 0xE0 == 0xC0 {
            out.push((u16::from(b & 0x1F) << 6) | cont(bytes.get(i + 1))?);
            i += 2;
        } else if b & 0xF0 == 0xE0 {
            let (c1, c2) = (cont(bytes.get(i + 1))?, cont(bytes.get(i + 2))?);
            out.push((u16::from(b & 0x0F) << 12) | (c1 << 6) | c2);
            i += 3;
        } else {
            return None;
        }
    }
    Some(out)
}

fn read_utf(ctx: &mut Ctx<'_>, dgram: ObjRef) -> JResult<Value> {
    let len = u16::from_be_bytes(take_n::<2>(ctx, dgram)?) as usize;
    let bytes = take(ctx, dgram, len)?;
    match decode_utf(&bytes) {
        Some(units) => Ok(ctx.new_string_utf16(units)?.into()),
        None => ctx.throw("java/io/UTFDataFormatException", "malformed input"),
    }
}

fn write_utf(ctx: &mut Ctx<'_>, dgram: ObjRef, text: Value) -> JResult<Value> {
    let s = ctx.non_null(text)?;
    let units = ctx.string_utf16(s)?.to_vec();
    let bytes = encode_modified_utf8(&units);
    let Ok(len) = u16::try_from(bytes.len()) else {
        let message = format!("encoded string too long: {} bytes", bytes.len());
        return ctx.throw("java/io/UTFDataFormatException", &message);
    };
    put(ctx, dgram, &[&len.to_be_bytes()[..], &bytes].concat())?;
    Ok(Value::Void)
}

/// `readFully(byte[], int, int)`.
fn read_fully(ctx: &mut Ctx<'_>, dgram: ObjRef, arr: Value, off: i32, len: i32) -> JResult<Value> {
    let arr = ctx.non_null(arr)?;
    let size = ctx.array_len(arr)?;
    let (off, n) = check_range(ctx, off, len, size)?;
    let bytes = take(ctx, dgram, n)?;
    copy_in(ctx, arr, off, &bytes)?;
    Ok(Value::Void)
}

/// `write(byte[], int, int)`.
fn write_bytes(ctx: &mut Ctx<'_>, dgram: ObjRef, arr: Value, off: i32, len: i32) -> JResult<Value> {
    let arr = ctx.non_null(arr)?;
    let size = ctx.array_len(arr)?;
    let (off, n) = check_range(ctx, off, len, size)?;
    let bytes = copy_out(ctx, arr, off, n)?;
    put(ctx, dgram, &bytes)?;
    Ok(Value::Void)
}

/// A `DataOutput` method that writes the bytes `encode` makes of its
/// argument.
fn writer(
    encode: fn(Value) -> Vec<u8>,
) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> + Send + Sync + 'static {
    move |ctx, a| {
        put(ctx, this(a), &encode(a[1]))?;
        Ok(Value::Void)
    }
}

/// The `Datagram` implementation: the `Datagram` accessors and every
/// `DataInput` and `DataOutput` method.
fn datagram_class() -> NativeClass {
    use fastemu_jvm::classfile::access::PRIVATE;
    NativeClass::new(DATAGRAM_IMPL)
        .implements("javax/microedition/io/Datagram")
        .field_with("buf", "[B", PRIVATE)
        .field_with("offset", "I", PRIVATE)
        .field_with("length", "I", PRIVATE)
        .field_with("pos", "I", PRIVATE)
        .field_with("address", "Ljava/lang/String;", PRIVATE)
        .method("getAddress", "()Ljava/lang/String;", |ctx, a| {
            let f = fields(ctx)?;
            Ok(ctx.get_field(this(a), f.address))
        })
        .method("getData", "()[B", |ctx, a| {
            let f = fields(ctx)?;
            Ok(ctx.get_field(this(a), f.buf))
        })
        .method("getLength", "()I", |ctx, a| {
            let f = fields(ctx)?;
            Ok(ctx.get_field(this(a), f.length))
        })
        .method("getOffset", "()I", |ctx, a| {
            let f = fields(ctx)?;
            Ok(ctx.get_field(this(a), f.offset))
        })
        .method("setAddress", "(Ljava/lang/String;)V", |ctx, a| {
            if a[1].is_null() {
                return illegal(ctx, "the address is null");
            }
            let address = checked_address(ctx, a[1])?;
            let f = fields(ctx)?;
            ctx.set_field(this(a), f.address, address.into());
            Ok(Value::Void)
        })
        .method(
            "setAddress",
            "(Ljavax/microedition/io/Datagram;)V",
            |ctx, a| {
                let other = ctx.non_null(a[1])?;
                let address =
                    ctx.call_interface(other, "getAddress", "()Ljava/lang/String;", &[])?;
                if address.is_null() {
                    return illegal(ctx, "the datagram has no address");
                }
                let f = fields(ctx)?;
                ctx.set_field(this(a), f.address, address);
                Ok(Value::Void)
            },
        )
        .method("setLength", "(I)V", |ctx, a| {
            let f = fields(ctx)?;
            let w = window(ctx, &f, this(a));
            let size = match w.buf {
                Some(arr) => ctx.array_len(arr)?,
                None => 0,
            };
            let len = a[1].int();
            if len < 0 || (w.offset as usize).saturating_add(len as usize) > size {
                return illegal(ctx, &format!("invalid length {len}"));
            }
            ctx.set_field(this(a), f.length, Value::Int(len));
            Ok(Value::Void)
        })
        .method("setData", "([BII)V", |ctx, a| {
            let Some(arr) = a[1].obj() else {
                return illegal(ctx, "the buffer is null");
            };
            let size = ctx.array_len(arr)?;
            let (offset, len) = (a[2].int(), a[3].int());
            let fits =
                offset >= 0 && len >= 0 && (offset as usize).saturating_add(len as usize) <= size;
            if !fits {
                return illegal(ctx, &format!("offset {offset} and length {len} do not fit"));
            }
            let f = fields(ctx)?;
            let dgram = this(a);
            ctx.set_field(dgram, f.buf, arr.into());
            ctx.set_field(dgram, f.offset, Value::Int(offset));
            ctx.set_field(dgram, f.length, Value::Int(len));
            ctx.set_field(dgram, f.pos, Value::Int(0));
            Ok(Value::Void)
        })
        .method("reset", "()V", |ctx, a| {
            let f = fields(ctx)?;
            let dgram = this(a);
            for field in [f.offset, f.length, f.pos] {
                ctx.set_field(dgram, field, Value::Int(0));
            }
            Ok(Value::Void)
        })
        .method("readFully", "([B)V", |ctx, a| {
            let arr = ctx.non_null(a[1])?;
            let len = ctx.array_len(arr)? as i32;
            read_fully(ctx, this(a), a[1], 0, len)
        })
        .method("readFully", "([BII)V", |ctx, a| {
            read_fully(ctx, this(a), a[1], a[2].int(), a[3].int())
        })
        .method("skipBytes", "(I)I", |ctx, a| {
            let f = fields(ctx)?;
            let dgram = this(a);
            let w = window(ctx, &f, dgram);
            let n = a[1].int().clamp(0, (w.length - w.pos).max(0));
            ctx.set_field(dgram, f.pos, Value::Int(w.pos + n));
            Ok(Value::Int(n))
        })
        .method("readBoolean", "()Z", |ctx, a| {
            Ok(Value::from(take_n::<1>(ctx, this(a))?[0] != 0))
        })
        .method("readByte", "()B", |ctx, a| {
            Ok(Value::Int(i32::from(take_n::<1>(ctx, this(a))?[0] as i8)))
        })
        .method("readUnsignedByte", "()I", |ctx, a| {
            Ok(Value::Int(i32::from(take_n::<1>(ctx, this(a))?[0])))
        })
        .method("readShort", "()S", |ctx, a| {
            Ok(Value::Int(i32::from(i16::from_be_bytes(take_n(
                ctx,
                this(a),
            )?))))
        })
        .method("readUnsignedShort", "()I", |ctx, a| {
            Ok(Value::Int(i32::from(u16::from_be_bytes(take_n(
                ctx,
                this(a),
            )?))))
        })
        .method("readChar", "()C", |ctx, a| {
            Ok(Value::Int(i32::from(u16::from_be_bytes(take_n(
                ctx,
                this(a),
            )?))))
        })
        .method("readInt", "()I", |ctx, a| {
            Ok(Value::Int(i32::from_be_bytes(take_n(ctx, this(a))?)))
        })
        .method("readLong", "()J", |ctx, a| {
            Ok(Value::Long(i64::from_be_bytes(take_n(ctx, this(a))?)))
        })
        .method("readFloat", "()F", |ctx, a| {
            Ok(Value::Float(f32::from_bits(u32::from_be_bytes(take_n(
                ctx,
                this(a),
            )?))))
        })
        .method("readDouble", "()D", |ctx, a| {
            Ok(Value::Double(f64::from_bits(u64::from_be_bytes(take_n(
                ctx,
                this(a),
            )?))))
        })
        .method("readUTF", "()Ljava/lang/String;", |ctx, a| {
            read_utf(ctx, this(a))
        })
        .method("write", "(I)V", writer(|v| vec![v.int() as u8]))
        .method("write", "([B)V", |ctx, a| {
            let arr = ctx.non_null(a[1])?;
            let len = ctx.array_len(arr)? as i32;
            write_bytes(ctx, this(a), a[1], 0, len)
        })
        .method("write", "([BII)V", |ctx, a| {
            write_bytes(ctx, this(a), a[1], a[2].int(), a[3].int())
        })
        .method(
            "writeBoolean",
            "(Z)V",
            writer(|v| vec![u8::from(v.int() != 0)]),
        )
        .method("writeByte", "(I)V", writer(|v| vec![v.int() as u8]))
        .method(
            "writeShort",
            "(I)V",
            writer(|v| (v.int() as u16).to_be_bytes().to_vec()),
        )
        .method(
            "writeChar",
            "(I)V",
            writer(|v| (v.int() as u16).to_be_bytes().to_vec()),
        )
        .method(
            "writeInt",
            "(I)V",
            writer(|v| v.int().to_be_bytes().to_vec()),
        )
        .method(
            "writeLong",
            "(J)V",
            writer(|v| v.long().to_be_bytes().to_vec()),
        )
        .method(
            "writeFloat",
            "(F)V",
            writer(|v| v.float().to_bits().to_be_bytes().to_vec()),
        )
        .method(
            "writeDouble",
            "(D)V",
            writer(|v| v.double().to_bits().to_be_bytes().to_vec()),
        )
        .method("writeChars", "(Ljava/lang/String;)V", |ctx, a| {
            let s = ctx.non_null(a[1])?;
            let bytes: Vec<u8> = ctx
                .string_utf16(s)?
                .iter()
                .flat_map(|u| u.to_be_bytes())
                .collect();
            put(ctx, this(a), &bytes)?;
            Ok(Value::Void)
        })
        .method("writeUTF", "(Ljava/lang/String;)V", |ctx, a| {
            write_utf(ctx, this(a), a[1])
        })
}

/// The classes of `datagram://`.
pub(crate) fn classes() -> Vec<NativeClass> {
    vec![connection_class(), datagram_class()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datagram_addresses_need_a_host_and_a_port() {
        assert_eq!(
            parse_address("datagram://10.0.0.1:5000"),
            Some(("10.0.0.1".to_owned(), 5000))
        );
        assert_eq!(
            parse_address("DATAGRAM://[::1]:7"),
            Some(("::1".to_owned(), 7))
        );
        assert_eq!(parse_address("datagram://:5000"), None);
        assert_eq!(parse_address("datagram://host"), None);
        assert_eq!(parse_address("socket://host:1"), None);
        assert_eq!(parse_address("garbage"), None);
    }

    #[test]
    fn read_utf_rejects_malformed_bytes() {
        let units: Vec<u16> = "h\u{e9}\u{20ac}\u{0}".encode_utf16().collect();
        assert_eq!(decode_utf(&encode_modified_utf8(&units)), Some(units));
        assert_eq!(decode_utf(&[0xC3]), None);
        assert_eq!(decode_utf(&[0xE2, 0x82]), None);
        assert_eq!(decode_utf(&[0xF0, 0x9F, 0x98, 0x80]), None);
        assert_eq!(decode_utf(&[0x80]), None);
    }

    const OPTIONS: Options = Options {
        mode: crate::io::connector::READ_WRITE,
        timeouts: false,
    };

    /// Runs `f` on a VM with the MIDP classes and `more`, whose instance
    /// may use the network.
    fn with_network(more: Vec<NativeClass>, f: impl FnOnce(&mut Ctx<'_>)) {
        use std::sync::Arc;

        use fastemu_core::NetworkPolicy;
        use fastemu_jvm::{MemorySource, Vm, VmConfig};

        use crate::env::InstanceEnv;

        let mut classes = crate::classes(&fastemu_core::default_profile());
        classes.extend(more);
        let vm = Vm::new(VmConfig::default(), Arc::new(MemorySource::new()), classes);
        vm.attach("test", |ctx| {
            let mut env = InstanceEnv::for_tests("unused");
            env.settings.network = NetworkPolicy::Allow;
            ctx.set_extension(env);
            f(ctx);
        });
    }

    fn local_port(ctx: &mut Ctx<'_>, conn: ObjRef) -> u16 {
        let port = ctx.call_interface(conn, "getLocalPort", "()I", &[]);
        u16::try_from(port.unwrap().int()).unwrap()
    }

    #[test]
    fn a_closed_datagram_server_frees_its_port() {
        with_network(Vec::new(), |ctx| {
            let first = open(ctx, "//:0", "datagram://:0", OPTIONS).unwrap();
            let port = local_port(ctx, first);
            ctx.call_interface(first, "close", "()V", &[]).unwrap();
            let rest = format!("//:{port}");
            let again = open(ctx, &rest, &format!("datagram:{rest}"), OPTIONS);
            assert!(again.is_ok(), "port {port} is still taken");
        });
    }

    #[test]
    fn a_game_datagram_whose_offset_is_past_its_buffer_receives_nothing() {
        // A game's own Datagram class, with an offset beyond its data.
        let odd = NativeClass::new("test/OddDatagram")
            .implements("javax/microedition/io/Datagram")
            .method("getData", "()[B", |ctx, _| {
                Ok(ctx.new_array(ArrayType::Byte, 3)?.into())
            })
            .method("getOffset", "()I", |_, _| Ok(Value::Int(10)))
            .method("setLength", "(I)V", |_, _| Ok(Value::Void))
            .method("setAddress", "(Ljava/lang/String;)V", |_, _| {
                Ok(Value::Void)
            });
        // Datagram extends these; the CLDC library may not have them yet.
        let mut more = vec![odd];
        more.extend(["java/io/DataInput", "java/io/DataOutput"].map(NativeClass::interface));
        with_network(more, |ctx| {
            let conn = open(ctx, "//:0", "datagram://:0", OPTIONS).unwrap();
            let port = local_port(ctx, conn);
            let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
            peer.send_to(b"hello", ("127.0.0.1", port)).unwrap();
            let class = ctx.find_class("test/OddDatagram").unwrap();
            let dgram = ctx.new_object(class).unwrap();
            let desc = "(Ljavax/microedition/io/Datagram;)V";
            ctx.call_interface(conn, "receive", desc, &[dgram.into()])
                .unwrap();
        });
    }
}
