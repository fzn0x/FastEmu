# java.io (CLDC 1.1)

The `java.io` classes of CLDC 1.1. CLDC has no files, no `File`, no
`BufferedReader` and no `Serializable`: games read resources from their
JAR through `Class.getResourceAsStream`, keep data in record stores, and
pack both with `DataInputStream` and `DataOutputStream` over byte arrays.
Every multi-byte value in the `Data` streams is big-endian.

Column meanings and tier rules: [README](README.md).

## Sources

- [CLDC 1.1 javadoc (JSR-139)](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/index.html), hosted by Oracle: the spec, and the source of every member, throws entry and since tag.
- [CLDC 1.0 javadoc (JSR-30)](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.0/jsr030/index.html): compared member by member to mark what CLDC 1.0 lacks (Since column).
- MicroEmulator 2.0.4 API stub jars [cldcapi10](https://repo1.maven.org/maven2/org/microemu/cldcapi10/2.0.4/cldcapi10-2.0.4.jar) and [cldcapi11](https://repo1.maven.org/maven2/org/microemu/cldcapi11/2.0.4/cldcapi11-2.0.4.jar) (Maven Central): compiled class files, read for descriptors, flags and `ConstantValue` attributes.

## Classes

| Class | Kind | Tier | Since | Members |
|---|---|---|---|---|
| [ByteArrayInputStream](#bytearrayinputstream) | class | 1 | CLDC 1.0 | 14 |
| [ByteArrayOutputStream](#bytearrayoutputstream) | class | 1 | CLDC 1.0 | 11 |
| [DataInput](#datainput) | interface | 2 | CLDC 1.0 | 14 |
| [DataInputStream](#datainputstream) | class | 1 | CLDC 1.0 | 26 |
| [DataOutput](#dataoutput) | interface | 2 | CLDC 1.0 | 13 |
| [DataOutputStream](#dataoutputstream) | class | 1 | CLDC 1.0 | 16 |
| [EOFException](#eofexception) | class | 2 | CLDC 1.0 | 2 |
| [IOException](#ioexception) | class | 1 | CLDC 1.0 | 2 |
| [InputStream](#inputstream) | class | 1 | CLDC 1.0 | 10 |
| [InputStreamReader](#inputstreamreader) | class | 2 | CLDC 1.0 | 10 |
| [InterruptedIOException](#interruptedioexception) | class | 3 | CLDC 1.0 | 3 |
| [OutputStream](#outputstream) | class | 1 | CLDC 1.0 | 6 |
| [OutputStreamWriter](#outputstreamwriter) | class | 3 | CLDC 1.0 | 7 |
| [PrintStream](#printstream) | class | 2 | CLDC 1.0 | 26 |
| [Reader](#reader) | class | 2 | CLDC 1.0 | 12 |
| [UTFDataFormatException](#utfdataformatexception) | class | 3 | CLDC 1.0 | 2 |
| [UnsupportedEncodingException](#unsupportedencodingexception) | class | 2 | CLDC 1.0 | 2 |
| [Writer](#writer) | class | 3 | CLDC 1.0 | 10 |

## java.io

### ByteArrayInputStream

`public class java.io.ByteArrayInputStream extends java.io.InputStream`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/ByteArrayInputStream.html).

Wraps record store data and resources that were read into memory.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `buf` | `[B` | (not a constant) | protected not final | 1.0 | `protected` field. |
| 3 | `pos` | `I` | (not a constant) | protected not final | 1.0 | `protected` field. |
| 3 | `mark` | `I` | (not a constant) | protected not final | 1.0 | `protected` field. |
| 3 | `count` | `I` | (not a constant) | protected not final | 1.0 | `protected` field. |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `(byte[] buf)` | `<init>([B)V` |  | 1.0 |  |  |
| 1 | `(byte[] buf, int offset, int length)` | `<init>([BII)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `int read()` | `read()I` |  | 1.0 |  |  |
| 1 | `int read(byte[] b, int off, int len)` | `read([BII)I` |  | 1.0 |  |  |
| 2 | `long skip(long n)` | `skip(J)J` |  | 1.0 |  |  |
| 1 | `int available()` | `available()I` |  | 1.0 |  |  |
| 3 | `boolean markSupported()` | `markSupported()Z` |  | 1.0 |  |  |
| 3 | `void mark(int readAheadLimit)` | `mark(I)V` |  | 1.0 |  | `protected` field. |
| 2 | `void reset()` | `reset()V` |  | 1.0 |  |  |
| 1 | `void close()` | `close()V` |  | 1.0 | `IOException` if an I/O error occurs |  |

### ByteArrayOutputStream

`public class java.io.ByteArrayOutputStream extends java.io.OutputStream`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/ByteArrayOutputStream.html).

Collects data before `RecordStore.addRecord` or `setRecord`.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `buf` | `[B` | (not a constant) | protected not final | 1.0 | `protected` field. |
| 3 | `count` | `I` | (not a constant) | protected not final | 1.0 | `protected` field. |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(int size)` | `<init>(I)V` |  | 1.0 | `IllegalArgumentException` if size is negative |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `void write(int b)` | `write(I)V` |  | 1.0 |  |  |
| 1 | `void write(byte[] b, int off, int len)` | `write([BII)V` |  | 1.0 |  |  |
| 1 | `void reset()` | `reset()V` |  | 1.0 |  |  |
| 1 | `byte[] toByteArray()` | `toByteArray()[B` |  | 1.0 |  |  |
| 1 | `int size()` | `size()I` |  | 1.0 |  |  |
| 3 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `void close()` | `close()V` |  | 1.0 | `IOException` if an I/O error occurs |  |

### DataInput

`public interface java.io.DataInput`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/DataInput.html).

Interface; games use it through `DataInputStream`.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void readFully(byte[] b)` | `readFully([B)V` |  | 1.0 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `void readFully(byte[] b, int off, int len)` | `readFully([BII)V` |  | 1.0 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `int skipBytes(int n)` | `skipBytes(I)I` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `boolean readBoolean()` | `readBoolean()Z` |  | 1.0 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `byte readByte()` | `readByte()B` |  | 1.0 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `int readUnsignedByte()` | `readUnsignedByte()I` |  | 1.0 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `short readShort()` | `readShort()S` |  | 1.0 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `int readUnsignedShort()` | `readUnsignedShort()I` |  | 1.0 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `char readChar()` | `readChar()C` |  | 1.0 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `int readInt()` | `readInt()I` |  | 1.0 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `long readLong()` | `readLong()J` |  | 1.0 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `float readFloat()` | `readFloat()F` |  | 1.1 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `double readDouble()` | `readDouble()D` |  | 1.1 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `String readUTF()` | `readUTF()Ljava/lang/String;` |  | 1.0 | `EOFException` if this stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs; `UTFDataFormatException` if the bytes do not represent a valid UTF-8 encoding of a string |  |

### DataInputStream

`public class java.io.DataInputStream extends java.io.InputStream implements java.io.DataInput`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/DataInputStream.html).

Reads level data, saved games and network replies.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `in` | `Ljava/io/InputStream;` | (not a constant) | protected not final | 1.0 | `protected` field: the wrapped stream. |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `(InputStream in)` | `<init>(Ljava/io/InputStream;)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `int read()` | `read()I` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `int read(byte[] b)` | `read([B)I` | final | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `int read(byte[] b, int off, int len)` | `read([BII)I` | final | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `void readFully(byte[] b)` | `readFully([B)V` | final | 1.0 | `EOFException` if this input stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs | Reads exactly the array's length or throws `EOFException`. |
| 1 | `void readFully(byte[] b, int off, int len)` | `readFully([BII)V` | final | 1.0 | `EOFException` if this input stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs |  |
| 2 | `int skipBytes(int n)` | `skipBytes(I)I` | final | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `boolean readBoolean()` | `readBoolean()Z` | final | 1.0 | `EOFException` if this input stream has reached the end; `IOException` if an I/O error occurs |  |
| 1 | `byte readByte()` | `readByte()B` | final | 1.0 | `EOFException` if this input stream has reached the end; `IOException` if an I/O error occurs |  |
| 1 | `int readUnsignedByte()` | `readUnsignedByte()I` | final | 1.0 | `EOFException` if this input stream has reached the end; `IOException` if an I/O error occurs |  |
| 1 | `short readShort()` | `readShort()S` | final | 1.0 | `EOFException` if this input stream reaches the end before reading two bytes; `IOException` if an I/O error occurs |  |
| 1 | `int readUnsignedShort()` | `readUnsignedShort()I` | final | 1.0 | `EOFException` if this input stream reaches the end before reading two bytes; `IOException` if an I/O error occurs |  |
| 2 | `char readChar()` | `readChar()C` | final | 1.0 | `EOFException` if this input stream reaches the end before reading two bytes; `IOException` if an I/O error occurs |  |
| 1 | `int readInt()` | `readInt()I` | final | 1.0 | `EOFException` if this input stream reaches the end before reading four bytes; `IOException` if an I/O error occurs |  |
| 2 | `long readLong()` | `readLong()J` | final | 1.0 | `EOFException` if this input stream reaches the end before reading eight bytes; `IOException` if an I/O error occurs |  |
| 2 | `float readFloat()` | `readFloat()F` | final | 1.1 | `EOFException` if this input stream reaches the end before reading four bytes; `IOException` if an I/O error occurs |  |
| 2 | `double readDouble()` | `readDouble()D` | final | 1.1 | `EOFException` if this input stream reaches the end before reading eight bytes; `IOException` if an I/O error occurs |  |
| 1 | `String readUTF()` | `readUTF()Ljava/lang/String;` | final | 1.0 | `EOFException` if this input stream reaches the end before reading all the bytes; `IOException` if an I/O error occurs | Reads an unsigned 16-bit length, then that many bytes of modified UTF-8 (U+0000 as two bytes, no 4-byte forms). Throws `UTFDataFormatException` for bad bytes and `EOFException` when the data ends early. |
| 3 | `String readUTF(DataInput in)` | `readUTF(Ljava/io/DataInput;)Ljava/lang/String;` | static final | 1.0 | `EOFException` if the input stream reaches the end before all the bytes; `IOException` if an I/O error occurs; `UTFDataFormatException` if the bytes do not represent a valid UTF-8 encoding of a Unicode string |  |
| 2 | `long skip(long n)` | `skip(J)J` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `int available()` | `available()I` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `void close()` | `close()V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 3 | `void mark(int readlimit)` | `mark(I)V` |  | 1.0 |  |  |
| 3 | `void reset()` | `reset()V` |  | 1.0 | `IOException` if the stream has not been marked or if the mark has been invalidated |  |
| 3 | `boolean markSupported()` | `markSupported()Z` |  | 1.0 |  |  |

### DataOutput

`public interface java.io.DataOutput`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/DataOutput.html).

Interface; games use it through `DataOutputStream`.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void write(int b)` | `write(I)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void write(byte[] b)` | `write([B)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void write(byte[] b, int off, int len)` | `write([BII)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void writeBoolean(boolean v)` | `writeBoolean(Z)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void writeByte(int v)` | `writeByte(I)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void writeShort(int v)` | `writeShort(I)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void writeChar(int v)` | `writeChar(I)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void writeInt(int v)` | `writeInt(I)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void writeLong(long v)` | `writeLong(J)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void writeFloat(float v)` | `writeFloat(F)V` |  | 1.1 | `IOException` if an I/O error occurs |  |
| 2 | `void writeDouble(double v)` | `writeDouble(D)V` |  | 1.1 | `IOException` if an I/O error occurs |  |
| 2 | `void writeChars(String s)` | `writeChars(Ljava/lang/String;)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void writeUTF(String s)` | `writeUTF(Ljava/lang/String;)V` |  | 1.0 | `IOException` if an I/O error occurs |  |

### DataOutputStream

`public class java.io.DataOutputStream extends java.io.OutputStream implements java.io.DataOutput`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/DataOutputStream.html).

Writes saved games for `RecordStore`.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `out` | `Ljava/io/OutputStream;` | (not a constant) | protected not final | 1.0 | `protected` field: the wrapped stream. |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `(OutputStream out)` | `<init>(Ljava/io/OutputStream;)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `void write(int b)` | `write(I)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `void write(byte[] b, int off, int len)` | `write([BII)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `void flush()` | `flush()V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `void close()` | `close()V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `void writeBoolean(boolean v)` | `writeBoolean(Z)V` | final | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `void writeByte(int v)` | `writeByte(I)V` | final | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `void writeShort(int v)` | `writeShort(I)V` | final | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void writeChar(int v)` | `writeChar(I)V` | final | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `void writeInt(int v)` | `writeInt(I)V` | final | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void writeLong(long v)` | `writeLong(J)V` | final | 1.0 | `IOException` if an I/O error occurs |  |
| 2 | `void writeFloat(float v)` | `writeFloat(F)V` | final | 1.1 | `IOException` if an I/O error occurs |  |
| 2 | `void writeDouble(double v)` | `writeDouble(D)V` | final | 1.1 | `IOException` if an I/O error occurs |  |
| 3 | `void writeChars(String s)` | `writeChars(Ljava/lang/String;)V` | final | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `void writeUTF(String str)` | `writeUTF(Ljava/lang/String;)V` | final | 1.0 | `IOException` if an I/O error occurs | Writes an unsigned 16-bit byte count, then modified UTF-8. The javadoc only lists `IOException`; Java SE throws its subclass `UTFDataFormatException` when the encoded form is longer than 65535 bytes, and FastEmu should do the same. |

### EOFException

`public class java.io.EOFException extends java.io.IOException`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/EOFException.html).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### IOException

`public class java.io.IOException extends java.lang.Exception`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/IOException.html).

Declared by nearly every I/O call; games catch it everywhere.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### InputStream

`public abstract class java.io.InputStream extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/InputStream.html).

What `Class.getResourceAsStream` and `Connector.openInputStream` return.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `int read()` | `read()I` | abstract | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `int read(byte[] b)` | `read([B)I` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `int read(byte[] b, int off, int len)` | `read([BII)I` |  | 1.0 | `IOException` if an I/O error occurs | Returns the count read, which may be less than asked, or -1 at the end. |
| 2 | `long skip(long n)` | `skip(J)J` |  | 1.0 | `IOException` if an I/O error occurs | May skip fewer bytes than asked; returns the count. |
| 1 | `int available()` | `available()I` |  | 1.0 | `IOException` if an I/O error occurs | Many games size a buffer with `available()` and then call `read(buf)` once. For a JAR resource FastEmu should report all remaining bytes here and let `read(byte[])` fill the whole buffer, or those games load half a file. |
| 1 | `void close()` | `close()V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 3 | `void mark(int readlimit)` | `mark(I)V` |  | 1.0 |  |  |
| 3 | `void reset()` | `reset()V` |  | 1.0 | `IOException` if this stream has not been marked or if the mark has been invalidated |  |
| 3 | `boolean markSupported()` | `markSupported()Z` |  | 1.0 |  |  |

### InputStreamReader

`public class java.io.InputStreamReader extends java.io.Reader`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/InputStreamReader.html).

Text files with a given encoding. The one-argument constructor uses `microedition.encoding`.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(InputStream is)` | `<init>(Ljava/io/InputStream;)V` |  | 1.0 |  |  |
| 2 | `(InputStream is, String enc)` | `<init>(Ljava/io/InputStream;Ljava/lang/String;)V` |  | 1.0 | `UnsupportedEncodingException` If the named encoding is not supported |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `int read()` | `read()I` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 2 | `int read(char[] cbuf, int off, int len)` | `read([CII)I` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `long skip(long n)` | `skip(J)J` |  | 1.0 | `IllegalArgumentException` If n is negative; `IOException` If an I/O error occurs |  |
| 3 | `boolean ready()` | `ready()Z` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `boolean markSupported()` | `markSupported()Z` |  | 1.0 |  |  |
| 3 | `void mark(int readAheadLimit)` | `mark(I)V` |  | 1.0 | `IOException` If the stream does not support mark(), or if some other I/O error occurs |  |
| 3 | `void reset()` | `reset()V` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 2 | `void close()` | `close()V` |  | 1.0 | `IOException` If an I/O error occurs |  |

### InterruptedIOException

`public class java.io.InterruptedIOException extends java.io.IOException`

Tier 3. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/InterruptedIOException.html).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `bytesTransferred` | `I` | (not a constant) | not final | 1.0 | Public, not final. |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  | 1.0 |  |  |
| 3 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### OutputStream

`public abstract class java.io.OutputStream extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/OutputStream.html).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `void write(int b)` | `write(I)V` | abstract | 1.0 | `IOException` if an I/O error occurs. In particular, an IOException may be thrown if the output stream has been closed |  |
| 1 | `void write(byte[] b)` | `write([B)V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `void write(byte[] b, int off, int len)` | `write([BII)V` |  | 1.0 | `IOException` if an I/O error occurs. In particular, an IOException is thrown if the output stream is closed |  |
| 1 | `void flush()` | `flush()V` |  | 1.0 | `IOException` if an I/O error occurs |  |
| 1 | `void close()` | `close()V` |  | 1.0 | `IOException` if an I/O error occurs |  |

### OutputStreamWriter

`public class java.io.OutputStreamWriter extends java.io.Writer`

Tier 3. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/OutputStreamWriter.html).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(OutputStream os)` | `<init>(Ljava/io/OutputStream;)V` |  | 1.0 |  |  |
| 3 | `(OutputStream os, String enc)` | `<init>(Ljava/io/OutputStream;Ljava/lang/String;)V` |  | 1.0 | `UnsupportedEncodingException` If the named encoding is not supported |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void write(int c)` | `write(I)V` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `void write(char[] cbuf, int off, int len)` | `write([CII)V` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `void write(String str, int off, int len)` | `write(Ljava/lang/String;II)V` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `void flush()` | `flush()V` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `void close()` | `close()V` |  | 1.0 | `IOException` If an I/O error occurs |  |

### PrintStream

`public class java.io.PrintStream extends java.io.OutputStream`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/PrintStream.html).

The class of `System.out` and `System.err`. It never throws `IOException`; errors set a flag read by `checkError`.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(OutputStream out)` | `<init>(Ljava/io/OutputStream;)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void flush()` | `flush()V` |  | 1.0 |  |  |
| 2 | `void close()` | `close()V` |  | 1.0 |  |  |
| 3 | `boolean checkError()` | `checkError()Z` |  | 1.0 |  |  |
| 3 | `void setError()` | `setError()V` | protected | 1.0 |  | `protected`. |
| 2 | `void write(int b)` | `write(I)V` |  | 1.0 |  |  |
| 2 | `void write(byte[] buf, int off, int len)` | `write([BII)V` |  | 1.0 |  |  |
| 2 | `void print(boolean b)` | `print(Z)V` |  | 1.0 |  |  |
| 2 | `void print(char c)` | `print(C)V` |  | 1.0 |  |  |
| 2 | `void print(int i)` | `print(I)V` |  | 1.0 |  |  |
| 2 | `void print(long l)` | `print(J)V` |  | 1.0 |  |  |
| 3 | `void print(float f)` | `print(F)V` |  | 1.1 |  |  |
| 3 | `void print(double d)` | `print(D)V` |  | 1.1 |  |  |
| 3 | `void print(char[] s)` | `print([C)V` |  | 1.0 | `NullPointerException` If s is null |  |
| 2 | `void print(String s)` | `print(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 2 | `void print(Object obj)` | `print(Ljava/lang/Object;)V` |  | 1.0 |  |  |
| 2 | `void println()` | `println()V` |  | 1.0 |  |  |
| 2 | `void println(boolean x)` | `println(Z)V` |  | 1.0 |  |  |
| 2 | `void println(char x)` | `println(C)V` |  | 1.0 |  |  |
| 2 | `void println(int x)` | `println(I)V` |  | 1.0 |  |  |
| 2 | `void println(long x)` | `println(J)V` |  | 1.0 |  |  |
| 3 | `void println(float x)` | `println(F)V` |  | 1.1 |  |  |
| 3 | `void println(double x)` | `println(D)V` |  | 1.1 |  |  |
| 3 | `void println(char[] x)` | `println([C)V` |  | 1.0 |  |  |
| 2 | `void println(String x)` | `println(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 2 | `void println(Object x)` | `println(Ljava/lang/Object;)V` |  | 1.0 |  |  |

### Reader

`public abstract class java.io.Reader extends java.lang.Object`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/Reader.html).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `lock` | `Ljava/lang/Object;` | (not a constant) | protected not final | 1.0 | `protected` field. |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` | protected | 1.0 |  |  |
| 3 | `(Object lock)` | `<init>(Ljava/lang/Object;)V` | protected | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `int read()` | `read()I` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 2 | `int read(char[] cbuf)` | `read([C)I` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 2 | `int read(char[] cbuf, int off, int len)` | `read([CII)I` | abstract | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `long skip(long n)` | `skip(J)J` |  | 1.0 | `IllegalArgumentException` If n is negative; `IOException` If an I/O error occurs |  |
| 3 | `boolean ready()` | `ready()Z` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `boolean markSupported()` | `markSupported()Z` |  | 1.0 |  |  |
| 3 | `void mark(int readAheadLimit)` | `mark(I)V` |  | 1.0 | `IOException` If the stream does not support mark(), or if some other I/O error occurs |  |
| 3 | `void reset()` | `reset()V` |  | 1.0 | `IOException` If the stream has not been marked, or if the mark has been invalidated, or if the stream does not support reset(), or if some other I/O error occurs |  |
| 2 | `void close()` | `close()V` | abstract | 1.0 | `IOException` If an I/O error occurs |  |

### UTFDataFormatException

`public class java.io.UTFDataFormatException extends java.io.IOException`

Tier 3. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/UTFDataFormatException.html).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  | 1.0 |  |  |
| 3 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### UnsupportedEncodingException

`public class java.io.UnsupportedEncodingException extends java.io.IOException`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/UnsupportedEncodingException.html).

Thrown when a game names an encoding the phone lacks.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### Writer

`public abstract class java.io.Writer extends java.lang.Object`

Tier 3. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/io/Writer.html).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `lock` | `Ljava/lang/Object;` | (not a constant) | protected not final | 1.0 | `protected` field. |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` | protected | 1.0 |  |  |
| 3 | `(Object lock)` | `<init>(Ljava/lang/Object;)V` | protected | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void write(int c)` | `write(I)V` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `void write(char[] cbuf)` | `write([C)V` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `void write(char[] cbuf, int off, int len)` | `write([CII)V` | abstract | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `void write(String str)` | `write(Ljava/lang/String;)V` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `void write(String str, int off, int len)` | `write(Ljava/lang/String;II)V` |  | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `void flush()` | `flush()V` | abstract | 1.0 | `IOException` If an I/O error occurs |  |
| 3 | `void close()` | `close()V` | abstract | 1.0 | `IOException` If an I/O error occurs |  |

## Cross-check

Every member above comes from the javadoc named first under Sources. The script that writes this file compared it with the stub and implementation sources listed under Sources: constant values, member lists and `static` flags. Member lists are compared only where the parsed source holds the implementation's whole class hierarchy. What differs is listed here; a class not listed matched everywhere it was compared.

No differences.
