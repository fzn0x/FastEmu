# javax.microedition.io (CLDC 1.1 GCF and MIDP 2.0)

`javax.microedition.io`: the Generic Connection Framework (GCF). CLDC
defines the framework (`Connector` and the connection interfaces); MIDP
1.0 adds `HttpConnection`, and MIDP 2.0 adds HTTPS, sockets, datagrams
over UDP, serial ports and `PushRegistry`. Every connection is opened from
a URL with `Connector.open`; the scheme picks the protocol, and a scheme
the phone does not know throws `ConnectionNotFoundException`.

Games use the network for score upload, ads, licence checks and
downloadable content. FastEmu's `NetworkPolicy::Deny` (the default) must
fail these the way a phone without signal does: `Connector.open` or the
first read throws `IOException`, never a crash or a hang. Games almost
always catch `IOException` around network code and carry on.

URL schemes games pass to `Connector.open`, and where each is defined:

| Scheme | Returns | Defined by |
|---|---|---|
| `http://` | `HttpConnection` | MIDP 1.0; the package description says implementations MUST support HTTP 1.1 |
| `https://` | `HttpsConnection` | MIDP 2.0 |
| `socket://host:port` | `SocketConnection` | MIDP 2.0 |
| `socket://:port` | `ServerSocketConnection` | MIDP 2.0 |
| `ssl://host:port` | `SecureConnection` | MIDP 2.0 |
| `datagram://host:port` | `UDPDatagramConnection` | MIDP 2.0 |
| `comm:port;baudrate=n` | `CommConnection` | MIDP 2.0 |
| `file:///root/path` | `FileConnection` | JSR-75, see [jsr-later.md](jsr-later.md) |
| `sms://`, `cbs://`, `mms://` | `MessageConnection` | JSR-120 and JSR-205, see [jsr-later.md](jsr-later.md) |
| `btspp://`, `btl2cap://`, `btgoep://` | Bluetooth connections | JSR-82, see [jsr-later.md](jsr-later.md) |

`HttpConnection` has three states. In Setup only `setRequestMethod` and
`setRequestProperty` work. Any method that needs the response
(`openInputStream`, `getResponseCode`, `getHeaderField`, `getLength` and
the other getters of response data) sends the request and moves to
Connected. `close` and closing every stream opened from the connection
move to Closed.

Column meanings and tier rules: [README](README.md).

## Sources

- [MIDP 2.0 javadoc (JSR-118)](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/index.html), hosted by Oracle: the spec, and the source of every member, throws entry and since tag.
- [CLDC 1.1 javadoc (JSR-139)](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/index.html), hosted by Oracle: the spec, and the source of every member, throws entry and since tag.
- MicroEmulator 2.0.4 API stub jar [midpapi20](https://repo1.maven.org/maven2/org/microemu/midpapi20/2.0.4/midpapi20-2.0.4.jar) (Maven Central): compiled class files, read for descriptors, flags and `ConstantValue` attributes.
- MicroEmulator 2.0.4 API stub jars [cldcapi10](https://repo1.maven.org/maven2/org/microemu/cldcapi10/2.0.4/cldcapi10-2.0.4.jar) and [cldcapi11](https://repo1.maven.org/maven2/org/microemu/cldcapi11/2.0.4/cldcapi11-2.0.4.jar) (Maven Central): compiled class files, read for descriptors, flags and `ConstantValue` attributes.
- [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/tree/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java) at commit `e270ed91e67b5cafc18c1991b92ba5249cf41c50` (Apache-2.0): implementation source, checked for constant values and member lists.
- [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/tree/aab3350832c0d82aff1771394d28bff6f66b0aa4/src) at commit `aab3350832c0d82aff1771394d28bff6f66b0aa4` (GPL-3.0): implementation source, checked for constant values and member lists.
- [MicroEmulator](https://github.com/barteo/microemu/tree/e47b656d983deaa67810c776662444483bdbf134/microemulator) at commit `e47b656d983deaa67810c776662444483bdbf134` (LGPL-2.1 or Apache-2.0): implementation source, checked for constant values and member lists.

## Classes

| Class | Kind | Tier | Since | Members |
|---|---|---|---|---|
| [Connection](#connection) | interface | 1 | CLDC | 1 |
| [ConnectionNotFoundException](#connectionnotfoundexception) | class | 1 | CLDC | 2 |
| [Connector](#connector) | class | 1 | CLDC | 10 |
| [ContentConnection](#contentconnection) | interface | 2 | CLDC | 3 |
| [Datagram](#datagram) | interface | 3 | CLDC | 9 |
| [DatagramConnection](#datagramconnection) | interface | 3 | CLDC | 8 |
| [InputConnection](#inputconnection) | interface | 2 | CLDC | 2 |
| [OutputConnection](#outputconnection) | interface | 2 | CLDC | 2 |
| [StreamConnection](#streamconnection) | interface | 2 | CLDC | 0 |
| [StreamConnectionNotifier](#streamconnectionnotifier) | interface | 3 | CLDC | 1 |
| [CommConnection](#commconnection) | interface | 3 | MIDP 2.0 | 2 |
| [HttpConnection](#httpconnection) | interface | 2 | MIDP 1.0 | 62 |
| [HttpsConnection](#httpsconnection) | interface | 3 | MIDP 2.0 | 2 |
| [PushRegistry](#pushregistry) | class | 3 | MIDP 2.0 | 6 |
| [SecureConnection](#secureconnection) | interface | 3 | MIDP 2.0 | 1 |
| [SecurityInfo](#securityinfo) | interface | 3 | MIDP 2.0 | 4 |
| [ServerSocketConnection](#serversocketconnection) | interface | 3 | MIDP 2.0 | 2 |
| [SocketConnection](#socketconnection) | interface | 3 | MIDP 2.0 | 11 |
| [UDPDatagramConnection](#udpdatagramconnection) | interface | 3 | MIDP 2.0 | 2 |

## Generic Connection Framework (CLDC)

### Connection

`public interface javax.microedition.io.Connection`

Tier 1. Since CLDC. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/javax/microedition/io/Connection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/Connection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/Connection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-cldc/src/main/java/javax/microedition/io/Connection.java).

Base of every connection type.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `void close()` | `close()V` |  | CLDC | `IOException` If an I/O error occurs |  |

### ConnectionNotFoundException

`public class javax.microedition.io.ConnectionNotFoundException extends java.io.IOException`

Tier 1. Since CLDC. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/javax/microedition/io/ConnectionNotFoundException.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/ConnectionNotFoundException.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/ConnectionNotFoundException.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-cldc/src/main/java/javax/microedition/io/ConnectionNotFoundException.java).

Thrown by `Connector.open` for unknown schemes and by `MIDlet.platformRequest`.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | CLDC |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | CLDC |  |  |

### Connector

`public class javax.microedition.io.Connector extends java.lang.Object`

Tier 1. Since CLDC. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/javax/microedition/io/Connector.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/Connector.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/Connector.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-cldc/src/main/java/javax/microedition/io/Connector.java).

The only way to open a connection.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `READ` | `I` | `1` | static | CLDC |  |
| 2 | `WRITE` | `I` | `2` | static | CLDC |  |
| 2 | `READ_WRITE` | `I` | `3` | static | CLDC |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `Connection open(String name)` | `open(Ljava/lang/String;)Ljavax/microedition/io/Connection;` | static | CLDC | `IllegalArgumentException` If a parameter is invalid; `ConnectionNotFoundException` If the target of the name cannot be found, or if the requested protocol type is not supported; `IOException` If some other kind of I/O error occurs; `SecurityException` May be thrown if access to the protocol handler is prohibited | Same as `open(name, READ_WRITE)`. |
| 2 | `Connection open(String name, int mode)` | `open(Ljava/lang/String;I)Ljavax/microedition/io/Connection;` | static | CLDC | `IllegalArgumentException` If a parameter is invalid; `ConnectionNotFoundException` If the target of the name cannot be found, or if the requested protocol type is not supported; `IOException` If some other kind of I/O error occurs; `SecurityException` May be thrown if access to the protocol handler is prohibited |  |
| 3 | `Connection open(String name, int mode, boolean timeouts)` | `open(Ljava/lang/String;IZ)Ljavax/microedition/io/Connection;` | static | CLDC | `IllegalArgumentException` If a parameter is invalid; `ConnectionNotFoundException` If the target of the name cannot be found, or if the requested protocol type is not supported; `IOException` If some other kind of I/O error occurs; `SecurityException` May be thrown if access to the protocol handler is prohibited | The flag asks for `InterruptedIOException` on timeouts. |
| 2 | `DataInputStream openDataInputStream(String name)` | `openDataInputStream(Ljava/lang/String;)Ljava/io/DataInputStream;` | static | CLDC | `IllegalArgumentException` If a parameter is invalid; `ConnectionNotFoundException` If the target of the name cannot be found, or if the requested protocol type is not supported; `IOException` If some other kind of I/O error occurs; `SecurityException` May be thrown if access to the protocol handler is prohibited |  |
| 3 | `DataOutputStream openDataOutputStream(String name)` | `openDataOutputStream(Ljava/lang/String;)Ljava/io/DataOutputStream;` | static | CLDC | `IllegalArgumentException` If a parameter is invalid; `ConnectionNotFoundException` If the target of the name cannot be found, or if the requested protocol type is not supported; `IOException` If some other kind of I/O error occurs; `SecurityException` May be thrown if access to the protocol handler is prohibited |  |
| 2 | `InputStream openInputStream(String name)` | `openInputStream(Ljava/lang/String;)Ljava/io/InputStream;` | static | CLDC | `IllegalArgumentException` If a parameter is invalid; `ConnectionNotFoundException` If the target of the name cannot be found, or if the requested protocol type is not supported; `IOException` If some other kind of I/O error occurs; `SecurityException` May be thrown if access to the protocol handler is prohibited | Opens the URL and returns its input stream in one call; some games fetch a whole HTTP reply this way. |
| 3 | `OutputStream openOutputStream(String name)` | `openOutputStream(Ljava/lang/String;)Ljava/io/OutputStream;` | static | CLDC | `IllegalArgumentException` If a parameter is invalid; `ConnectionNotFoundException` If the target of the name cannot be found, or if the requested protocol type is not supported; `IOException` If some other kind of I/O error occurs; `SecurityException` May be thrown if access to the protocol handler is prohibited |  |

### ContentConnection

`public interface javax.microedition.io.ContentConnection extends javax.microedition.io.StreamConnection`

Tier 2. Since CLDC. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/javax/microedition/io/ContentConnection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/ContentConnection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/ContentConnection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-cldc/src/main/java/javax/microedition/io/ContentConnection.java).

`HttpConnection` extends it.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `String getType()` | `getType()Ljava/lang/String;` |  | CLDC |  |  |
| 2 | `String getEncoding()` | `getEncoding()Ljava/lang/String;` |  | CLDC |  |  |
| 2 | `long getLength()` | `getLength()J` |  | CLDC |  | -1 when the length is unknown. |

### Datagram

`public interface javax.microedition.io.Datagram extends java.io.DataInput, java.io.DataOutput`

Tier 3. Since CLDC. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/javax/microedition/io/Datagram.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/Datagram.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/Datagram.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-cldc/src/main/java/javax/microedition/io/Datagram.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `String getAddress()` | `getAddress()Ljava/lang/String;` |  | CLDC |  |  |
| 3 | `byte[] getData()` | `getData()[B` |  | CLDC |  |  |
| 3 | `int getLength()` | `getLength()I` |  | CLDC |  |  |
| 3 | `int getOffset()` | `getOffset()I` |  | CLDC |  |  |
| 3 | `void setAddress(String addr)` | `setAddress(Ljava/lang/String;)V` |  | CLDC | `IllegalArgumentException` if the address is not valid; `IOException` if a some kind of I/O error occurs |  |
| 3 | `void setAddress(Datagram reference)` | `setAddress(Ljavax/microedition/io/Datagram;)V` |  | CLDC | `IllegalArgumentException` if the address is not valid |  |
| 3 | `void setLength(int len)` | `setLength(I)V` |  | CLDC | `IllegalArgumentException` if the length or length plus offset fall outside the buffer |  |
| 3 | `void setData(byte[] buffer, int offset, int len)` | `setData([BII)V` |  | CLDC | `IllegalArgumentException` if the length or offset or offset plus length fall outside the buffer, or if the buffer parameter is invalid |  |
| 3 | `void reset()` | `reset()V` |  | CLDC |  |  |

### DatagramConnection

`public interface javax.microedition.io.DatagramConnection extends javax.microedition.io.Connection`

Tier 3. Since CLDC. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/javax/microedition/io/DatagramConnection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/DatagramConnection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/DatagramConnection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-cldc/src/main/java/javax/microedition/io/DatagramConnection.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `int getMaximumLength()` | `getMaximumLength()I` |  | CLDC | `IOException` If an I/O error occurs |  |
| 3 | `int getNominalLength()` | `getNominalLength()I` |  | CLDC | `IOException` If an I/O error occurs |  |
| 3 | `void send(Datagram dgram)` | `send(Ljavax/microedition/io/Datagram;)V` |  | CLDC | `IOException` If an I/O error occurs; `InterruptedIOException` Timeout or interrupt occurred |  |
| 3 | `void receive(Datagram dgram)` | `receive(Ljavax/microedition/io/Datagram;)V` |  | CLDC | `IOException` If an I/O error occurs; `InterruptedIOException` Timeout or interrupt occurred |  |
| 3 | `Datagram newDatagram(int size)` | `newDatagram(I)Ljavax/microedition/io/Datagram;` |  | CLDC | `IOException` If an I/O error occurs; `IllegalArgumentException` if the size is negative or larger than the maximum size |  |
| 3 | `Datagram newDatagram(int size, String addr)` | `newDatagram(ILjava/lang/String;)Ljavax/microedition/io/Datagram;` |  | CLDC | `IOException` If an I/O error occurs; `IllegalArgumentException` if the size is negative or larger than the maximum size, or if the address parameter is invalid |  |
| 3 | `Datagram newDatagram(byte[] buf, int size)` | `newDatagram([BI)Ljavax/microedition/io/Datagram;` |  | CLDC | `IOException` If an I/O error occurs; `IllegalArgumentException` if the size is negative or larger than the maximum size or the given buffer's length, or if the buffer parameter is invalid |  |
| 3 | `Datagram newDatagram(byte[] buf, int size, String addr)` | `newDatagram([BILjava/lang/String;)Ljavax/microedition/io/Datagram;` |  | CLDC | `IOException` If an I/O error occurs; `IllegalArgumentException` if the size is negative or larger than the maximum size or the given buffer's length, or if the address or buffer parameter is invalid |  |

### InputConnection

`public interface javax.microedition.io.InputConnection extends javax.microedition.io.Connection`

Tier 2. Since CLDC. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/javax/microedition/io/InputConnection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/InputConnection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/InputConnection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-cldc/src/main/java/javax/microedition/io/InputConnection.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `InputStream openInputStream()` | `openInputStream()Ljava/io/InputStream;` |  | CLDC | `IOException` If an I/O error occurs |  |
| 2 | `DataInputStream openDataInputStream()` | `openDataInputStream()Ljava/io/DataInputStream;` |  | CLDC | `IOException` If an I/O error occurs |  |

### OutputConnection

`public interface javax.microedition.io.OutputConnection extends javax.microedition.io.Connection`

Tier 2. Since CLDC. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/javax/microedition/io/OutputConnection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/OutputConnection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/OutputConnection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-cldc/src/main/java/javax/microedition/io/OutputConnection.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `OutputStream openOutputStream()` | `openOutputStream()Ljava/io/OutputStream;` |  | CLDC | `IOException` If an I/O error occurs |  |
| 2 | `DataOutputStream openDataOutputStream()` | `openDataOutputStream()Ljava/io/DataOutputStream;` |  | CLDC | `IOException` If an I/O error occurs |  |

### StreamConnection

`public interface javax.microedition.io.StreamConnection extends javax.microedition.io.InputConnection, javax.microedition.io.OutputConnection`

Tier 2. Since CLDC. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/javax/microedition/io/StreamConnection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/StreamConnection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/StreamConnection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-cldc/src/main/java/javax/microedition/io/StreamConnection.java).

Declares nothing; joins `InputConnection` and `OutputConnection`.

Declares no members of its own.

### StreamConnectionNotifier

`public interface javax.microedition.io.StreamConnectionNotifier extends javax.microedition.io.Connection`

Tier 3. Since CLDC. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/javax/microedition/io/StreamConnectionNotifier.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/StreamConnectionNotifier.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/StreamConnectionNotifier.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-cldc/src/main/java/javax/microedition/io/StreamConnectionNotifier.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `StreamConnection acceptAndOpen()` | `acceptAndOpen()Ljavax/microedition/io/StreamConnection;` |  | CLDC | `IOException` If an I/O error occurs |  |

## MIDP 2.0 additions

### CommConnection

`public interface javax.microedition.io.CommConnection extends javax.microedition.io.StreamConnection`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/io/CommConnection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/CommConnection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/CommConnection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/io/CommConnection.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `int getBaudRate()` | `getBaudRate()I` |  | 2.0 |  |  |
| 3 | `int setBaudRate(int baudrate)` | `setBaudRate(I)I` |  | 2.0 |  |  |

### HttpConnection

`public interface javax.microedition.io.HttpConnection extends javax.microedition.io.ContentConnection`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/io/HttpConnection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/HttpConnection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/HttpConnection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/io/HttpConnection.java).

MIDP 1.0. The MIDP package description says implementations MUST support HTTP 1.1, through this interface.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `HEAD` | `Ljava/lang/String;` | `"HEAD"` | static | 1.0 |  |
| 2 | `GET` | `Ljava/lang/String;` | `"GET"` | static | 1.0 |  |
| 2 | `POST` | `Ljava/lang/String;` | `"POST"` | static | 1.0 |  |
| 2 | `HTTP_OK` | `I` | `200` | static | 1.0 | `getResponseCode()` is compared with it. |
| 3 | `HTTP_CREATED` | `I` | `201` | static | 1.0 |  |
| 3 | `HTTP_ACCEPTED` | `I` | `202` | static | 1.0 |  |
| 3 | `HTTP_NOT_AUTHORITATIVE` | `I` | `203` | static | 1.0 |  |
| 3 | `HTTP_NO_CONTENT` | `I` | `204` | static | 1.0 |  |
| 3 | `HTTP_RESET` | `I` | `205` | static | 1.0 |  |
| 3 | `HTTP_PARTIAL` | `I` | `206` | static | 1.0 |  |
| 3 | `HTTP_MULT_CHOICE` | `I` | `300` | static | 1.0 |  |
| 3 | `HTTP_MOVED_PERM` | `I` | `301` | static | 1.0 |  |
| 3 | `HTTP_MOVED_TEMP` | `I` | `302` | static | 1.0 |  |
| 3 | `HTTP_SEE_OTHER` | `I` | `303` | static | 1.0 |  |
| 3 | `HTTP_NOT_MODIFIED` | `I` | `304` | static | 1.0 |  |
| 3 | `HTTP_USE_PROXY` | `I` | `305` | static | 1.0 |  |
| 3 | `HTTP_TEMP_REDIRECT` | `I` | `307` | static | 1.0 |  |
| 3 | `HTTP_BAD_REQUEST` | `I` | `400` | static | 1.0 |  |
| 3 | `HTTP_UNAUTHORIZED` | `I` | `401` | static | 1.0 |  |
| 3 | `HTTP_PAYMENT_REQUIRED` | `I` | `402` | static | 1.0 |  |
| 3 | `HTTP_FORBIDDEN` | `I` | `403` | static | 1.0 |  |
| 3 | `HTTP_NOT_FOUND` | `I` | `404` | static | 1.0 |  |
| 3 | `HTTP_BAD_METHOD` | `I` | `405` | static | 1.0 |  |
| 3 | `HTTP_NOT_ACCEPTABLE` | `I` | `406` | static | 1.0 |  |
| 3 | `HTTP_PROXY_AUTH` | `I` | `407` | static | 1.0 |  |
| 3 | `HTTP_CLIENT_TIMEOUT` | `I` | `408` | static | 1.0 |  |
| 3 | `HTTP_CONFLICT` | `I` | `409` | static | 1.0 |  |
| 3 | `HTTP_GONE` | `I` | `410` | static | 1.0 |  |
| 3 | `HTTP_LENGTH_REQUIRED` | `I` | `411` | static | 1.0 |  |
| 3 | `HTTP_PRECON_FAILED` | `I` | `412` | static | 1.0 |  |
| 3 | `HTTP_ENTITY_TOO_LARGE` | `I` | `413` | static | 1.0 |  |
| 3 | `HTTP_REQ_TOO_LONG` | `I` | `414` | static | 1.0 |  |
| 3 | `HTTP_UNSUPPORTED_TYPE` | `I` | `415` | static | 1.0 |  |
| 3 | `HTTP_UNSUPPORTED_RANGE` | `I` | `416` | static | 1.0 |  |
| 3 | `HTTP_EXPECT_FAILED` | `I` | `417` | static | 1.0 |  |
| 3 | `HTTP_INTERNAL_ERROR` | `I` | `500` | static | 1.0 |  |
| 3 | `HTTP_NOT_IMPLEMENTED` | `I` | `501` | static | 1.0 |  |
| 3 | `HTTP_BAD_GATEWAY` | `I` | `502` | static | 1.0 |  |
| 3 | `HTTP_UNAVAILABLE` | `I` | `503` | static | 1.0 |  |
| 3 | `HTTP_GATEWAY_TIMEOUT` | `I` | `504` | static | 1.0 |  |
| 3 | `HTTP_VERSION` | `I` | `505` | static | 1.0 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `String getURL()` | `getURL()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `String getProtocol()` | `getProtocol()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `String getHost()` | `getHost()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `String getFile()` | `getFile()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `String getRef()` | `getRef()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `String getQuery()` | `getQuery()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `int getPort()` | `getPort()I` |  | 1.0 |  |  |
| 3 | `String getRequestMethod()` | `getRequestMethod()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `void setRequestMethod(String method)` | `setRequestMethod(Ljava/lang/String;)V` |  | 1.0 | `IOException` if the method cannot be reset or if the requested method isn't valid for HTTP | Setup state only; throws `IOException` after the request was sent. |
| 3 | `String getRequestProperty(String key)` | `getRequestProperty(Ljava/lang/String;)Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `void setRequestProperty(String key, String value)` | `setRequestProperty(Ljava/lang/String;Ljava/lang/String;)V` |  | 1.0 | `IOException` is thrown if the connection is in the connected state | Setup state only. |
| 2 | `int getResponseCode()` | `getResponseCode()I` |  | 1.0 | `IOException` if an error occurred connecting to the server | Sends the request if it has not been sent. Returns -1 when the reply is not valid HTTP. |
| 3 | `String getResponseMessage()` | `getResponseMessage()Ljava/lang/String;` |  | 1.0 | `IOException` if an error occurred connecting to the server |  |
| 3 | `long getExpiration()` | `getExpiration()J` |  | 1.0 | `IOException` if an error occurred connecting to the server |  |
| 3 | `long getDate()` | `getDate()J` |  | 1.0 | `IOException` if an error occurred connecting to the server |  |
| 3 | `long getLastModified()` | `getLastModified()J` |  | 1.0 | `IOException` if an error occurred connecting to the server |  |
| 2 | `String getHeaderField(String name)` | `getHeaderField(Ljava/lang/String;)Ljava/lang/String;` |  | 1.0 | `IOException` if an error occurred connecting to the server |  |
| 3 | `int getHeaderFieldInt(String name, int def)` | `getHeaderFieldInt(Ljava/lang/String;I)I` |  | 1.0 | `IOException` if an error occurred connecting to the server |  |
| 3 | `long getHeaderFieldDate(String name, long def)` | `getHeaderFieldDate(Ljava/lang/String;J)J` |  | 1.0 | `IOException` if an error occurred connecting to the server |  |
| 3 | `String getHeaderField(int n)` | `getHeaderField(I)Ljava/lang/String;` |  | 1.0 | `IOException` if an error occurred connecting to the server |  |
| 3 | `String getHeaderFieldKey(int n)` | `getHeaderFieldKey(I)Ljava/lang/String;` |  | 1.0 | `IOException` if an error occurred connecting to the server |  |

### HttpsConnection

`public interface javax.microedition.io.HttpsConnection extends javax.microedition.io.HttpConnection`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/io/HttpsConnection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/HttpsConnection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/HttpsConnection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/io/HttpsConnection.java).

MIDP 2.0.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `SecurityInfo getSecurityInfo()` | `getSecurityInfo()Ljavax/microedition/io/SecurityInfo;` |  | 2.0 | `IOException` if an arbitrary connection failure occurs |  |
| 3 | `int getPort()` | `getPort()I` |  | 2.0 |  |  |

### PushRegistry

`public class javax.microedition.io.PushRegistry extends java.lang.Object`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/io/PushRegistry.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/PushRegistry.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/PushRegistry.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/io/PushRegistry.java).

MIDP 2.0. Lets the phone start a MIDlet for an inbound connection or at a set time. FastEmu can accept registrations and never fire them.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void registerConnection(String connection, String midlet, String filter)` | `registerConnection(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V` | static | 2.0 | `IllegalArgumentException` if the connection string is not valid, or if the filter string is not valid; `ConnectionNotFoundException` if the runtime system does not support push delivery for the requested connection protocol; `IOException` if the connection is already registered or if there are insufficient resources to handle the registration request; `ClassNotFoundException` if the MIDlet class name can not be found in the current MIDlet suite or if this class is not included in any of the MIDlet-<n> records in the descriptor file or the jar file manifest; `SecurityException` if the MIDlet does not have permission to register a connection |  |
| 3 | `boolean unregisterConnection(String connection)` | `unregisterConnection(Ljava/lang/String;)Z` | static | 2.0 | `SecurityException` if the connection was registered by another MIDlet suite |  |
| 3 | `String[] listConnections(boolean available)` | `listConnections(Z)[Ljava/lang/String;` | static | 2.0 |  |  |
| 3 | `String getMIDlet(String connection)` | `getMIDlet(Ljava/lang/String;)Ljava/lang/String;` | static | 2.0 |  |  |
| 3 | `String getFilter(String connection)` | `getFilter(Ljava/lang/String;)Ljava/lang/String;` | static | 2.0 |  |  |
| 3 | `long registerAlarm(String midlet, long time)` | `registerAlarm(Ljava/lang/String;J)J` | static | 2.0 | `ConnectionNotFoundException` if the runtime system does not support alarm based application launch; `ClassNotFoundException` if the MIDlet class name can not be found in the current MIDlet suite or if this class is not included in any of the MIDlet-<n> records in the descriptor file or the jar file manifest or if the midlet argument is null; `SecurityException` if the MIDlet does not have permission to register an alarm | Returns the previous alarm time, or 0 when there was none. |

### SecureConnection

`public interface javax.microedition.io.SecureConnection extends javax.microedition.io.SocketConnection`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/io/SecureConnection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/SecureConnection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/SecureConnection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/io/SecureConnection.java).

MIDP 2.0.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `SecurityInfo getSecurityInfo()` | `getSecurityInfo()Ljavax/microedition/io/SecurityInfo;` |  | 2.0 | `IOException` if an arbitrary connection failure occurs |  |

### SecurityInfo

`public interface javax.microedition.io.SecurityInfo`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/io/SecurityInfo.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/SecurityInfo.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/SecurityInfo.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/io/SecurityInfo.java).

MIDP 2.0.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `Certificate getServerCertificate()` | `getServerCertificate()Ljavax/microedition/pki/Certificate;` |  | 2.0 |  |  |
| 3 | `String getProtocolVersion()` | `getProtocolVersion()Ljava/lang/String;` |  | 2.0 |  |  |
| 3 | `String getProtocolName()` | `getProtocolName()Ljava/lang/String;` |  | 2.0 |  |  |
| 3 | `String getCipherSuite()` | `getCipherSuite()Ljava/lang/String;` |  | 2.0 |  |  |

### ServerSocketConnection

`public interface javax.microedition.io.ServerSocketConnection extends javax.microedition.io.StreamConnectionNotifier`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/io/ServerSocketConnection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/ServerSocketConnection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/ServerSocketConnection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/io/ServerSocketConnection.java).

MIDP 2.0.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `String getLocalAddress()` | `getLocalAddress()Ljava/lang/String;` |  | 2.0 | `IOException` if the connection was closed |  |
| 3 | `int getLocalPort()` | `getLocalPort()I` |  | 2.0 | `IOException` if the connection was closed |  |

### SocketConnection

`public interface javax.microedition.io.SocketConnection extends javax.microedition.io.StreamConnection`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/io/SocketConnection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/SocketConnection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/SocketConnection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/io/SocketConnection.java).

MIDP 2.0. Online multiplayer games used it.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `DELAY` | `B` | `0` | static | 2.0 |  |
| 3 | `LINGER` | `B` | `1` | static | 2.0 |  |
| 3 | `KEEPALIVE` | `B` | `2` | static | 2.0 |  |
| 3 | `RCVBUF` | `B` | `3` | static | 2.0 |  |
| 3 | `SNDBUF` | `B` | `4` | static | 2.0 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void setSocketOption(byte option, int value)` | `setSocketOption(BI)V` |  | 2.0 | `IllegalArgumentException` if the value is not valid (e.g. negative value) or if the option identifier is not valid; `IOException` if the connection was closed |  |
| 3 | `int getSocketOption(byte option)` | `getSocketOption(B)I` |  | 2.0 | `IllegalArgumentException` if the option identifier is not valid; `IOException` if the connection was closed |  |
| 3 | `String getLocalAddress()` | `getLocalAddress()Ljava/lang/String;` |  | 2.0 | `IOException` if the connection was closed |  |
| 3 | `int getLocalPort()` | `getLocalPort()I` |  | 2.0 | `IOException` if the connection was closed |  |
| 3 | `String getAddress()` | `getAddress()Ljava/lang/String;` |  | 2.0 | `IOException` if the connection was closed |  |
| 3 | `int getPort()` | `getPort()I` |  | 2.0 | `IOException` if the connection was closed |  |

### UDPDatagramConnection

`public interface javax.microedition.io.UDPDatagramConnection extends javax.microedition.io.DatagramConnection`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/io/UDPDatagramConnection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/io/UDPDatagramConnection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/io/UDPDatagramConnection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/io/UDPDatagramConnection.java).

MIDP 2.0.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `String getLocalAddress()` | `getLocalAddress()Ljava/lang/String;` |  | 2.0 | `IOException` if the connection was closed |  |
| 3 | `int getLocalPort()` | `getLocalPort()I` |  | 2.0 | `IOException` if the connection was closed |  |

## Cross-check

Every member above comes from the spec javadoc. The script that writes this file compared it with the stub and implementation sources listed under Sources: constant values, member lists and `static` flags. Member lists are compared only where the parsed source holds the implementation's whole class hierarchy. What differs is listed here; a class not listed matched everywhere it was compared.

No differences.
