import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInput;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.EOFException;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.UTFDataFormatException;

// The byte array and data streams: every primitive and modified UTF-8
// through a round trip, end of data, bad UTF-8, and game streams of
// their own under the data streams.
public class DataStreams {
    // A game's own stream: the bytes 0, 1, 2, ... up to a limit, counting
    // how often each read method is called.
    static class Counting extends InputStream {
        int next;
        final int limit;
        int singles;

        Counting(int limit) {
            this.limit = limit;
        }

        public int read() {
            singles++;
            return next < limit ? next++ : -1;
        }
    }

    // A game's own sink that records what it is given, in hex.
    static class Recording extends OutputStream {
        final StringBuffer seen = new StringBuffer();

        public void write(int b) {
            seen.append(Integer.toHexString(b & 0xFF)).append(' ');
        }
    }

    static String hex(byte[] bytes) {
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < bytes.length; i++) {
            sb.append(Integer.toHexString(bytes[i] & 0xFF)).append(' ');
        }
        return sb.toString();
    }

    static void roundTrip() throws IOException {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        DataOutputStream out = new DataOutputStream(bytes);
        out.writeBoolean(true);
        out.writeBoolean(false);
        out.writeByte(-2);
        out.writeByte(300);
        out.writeShort(-12345);
        out.writeShort(65535);
        out.writeChar('€');
        out.writeInt(Integer.MIN_VALUE);
        out.writeInt(0x12345678);
        out.writeLong(Long.MIN_VALUE + 77);
        out.writeLong(-1L);
        out.writeFloat(3.5f);
        out.writeFloat(Float.NaN);
        out.writeFloat(-0.0f);
        out.writeDouble(Math.PI);
        out.writeDouble(Double.NEGATIVE_INFINITY);
        out.writeUTF("");
        out.writeUTF("héllo \u0000 € 😀");
        out.writeChars("Ab");
        out.write(new byte[] {9, 8, 7}, 1, 2);
        out.flush();
        System.out.println("size " + bytes.size());
        byte[] data = bytes.toByteArray();
        System.out.println(hex(data));

        DataInputStream in = new DataInputStream(new ByteArrayInputStream(data));
        System.out.println(in.available());
        System.out.println(in.readBoolean() + " " + in.readBoolean());
        System.out.println(in.readByte() + " " + in.readUnsignedByte());
        System.out.println(in.readShort() + " " + in.readUnsignedShort());
        System.out.println((int) in.readChar());
        System.out.println(in.readInt() + " " + in.readInt());
        System.out.println(in.readLong() + " " + in.readLong());
        System.out.println(in.readFloat() + " " + in.readFloat() + " " + in.readFloat());
        System.out.println(in.readDouble() + " " + in.readDouble());
        System.out.println("[" + in.readUTF() + "]");
        String s = in.readUTF();
        System.out.println(s.length() + " " + (int) s.charAt(6) + " " + s.endsWith("😀"));
        System.out.println(in.readChar() + "" + in.readChar());
        System.out.println(in.readByte() + " " + in.readByte() + " " + in.read());
        try {
            in.readInt();
            System.out.println("no EOF");
        } catch (EOFException e) {
            System.out.println("EOF " + e.getMessage());
        }
    }

    static void endOfData() throws IOException {
        DataInputStream in = new DataInputStream(new ByteArrayInputStream(new byte[] {1, 2, 3}));
        try {
            in.readInt();
        } catch (EOFException e) {
            System.out.println("readInt at 3 bytes: EOF, left " + in.available());
        }
        in = new DataInputStream(new ByteArrayInputStream(new byte[] {1, 2, 3, 4, 5}));
        byte[] buf = new byte[8];
        try {
            in.readFully(buf);
        } catch (EOFException e) {
            System.out.println("readFully: EOF, got " + hex(buf));
        }
        in = new DataInputStream(new ByteArrayInputStream(new byte[] {1, 2, 3, 4, 5}));
        System.out.println("skipBytes " + in.skipBytes(3) + " then " + in.read() + " skip " + in.skipBytes(10));
        in.readFully(buf, 0, 0);
        System.out.println("read at end " + in.read(buf) + " " + in.read(buf, 0, 0));
    }

    static void badUtf() throws IOException {
        byte[][] cases = {
            {0, 2, 'a', (byte) 0xC3},
            {0, 4, 'a', (byte) 0xE2, 'A', (byte) 0x80},
            {0, 1, (byte) 0x80},
            {0, 5, 'x', (byte) 0xF0, (byte) 0x9F, (byte) 0x98, (byte) 0x80},
            {0, 9, 'a', 'b'},
        };
        for (int i = 0; i < cases.length; i++) {
            DataInputStream in = new DataInputStream(new ByteArrayInputStream(cases[i]));
            try {
                System.out.println("ok " + in.readUTF());
            } catch (UTFDataFormatException e) {
                System.out.println("UTFDataFormatException " + e.getMessage());
            } catch (EOFException e) {
                System.out.println("EOFException");
            }
        }
        StringBuffer big = new StringBuffer();
        for (int i = 0; i < 21846; i++) {
            big.append('€');
        }
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        try {
            new DataOutputStream(bytes).writeUTF(big.toString());
            System.out.println("wrote " + bytes.size());
        } catch (UTFDataFormatException e) {
            System.out.println("too long, wrote " + bytes.size());
        }
    }

    static void gameStreams() throws IOException {
        Counting counting = new Counting(10);
        DataInputStream in = new DataInputStream(counting);
        System.out.println(in.readInt() + " after " + counting.singles + " reads");
        System.out.println(in.readShort() + " after " + counting.singles + " reads");
        byte[] buf = new byte[3];
        in.readFully(buf);
        System.out.println(hex(buf) + "after " + counting.singles + " reads");
        try {
            in.readLong();
            System.out.println("no EOF");
        } catch (EOFException e) {
            System.out.println("readLong past the end: EOF");
        }
    }

    static void gameStreamsEnd() throws IOException {
        Counting counting = new Counting(2);
        DataInputStream in = new DataInputStream(counting);
        try {
            in.readLong();
        } catch (EOFException e) {
            System.out.println("readLong over 2 bytes: EOF");
        }
        Recording sink = new Recording();
        DataOutputStream out = new DataOutputStream(sink);
        out.writeInt(0xCAFEBABE);
        out.writeUTF("é");
        out.writeLong(258);
        out.writeDouble(1.0);
        out.writeChars("Ă");
        out.close();
        System.out.println(sink.seen);
        DataInput di = new DataInputStream(new ByteArrayInputStream(new byte[] {0, 3, 'f', 'o', 'o'}));
        System.out.println(DataInputStream.readUTF(di));
    }

    static void byteArrays() throws IOException {
        ByteArrayOutputStream out = new ByteArrayOutputStream(0);
        for (int i = 0; i < 40; i++) {
            out.write(i);
        }
        out.write(new byte[] {65, 66, 67, 68}, 1, 2);
        System.out.println(out.size() + " " + out.toByteArray().length);
        out.reset();
        out.write("Grüße".getBytes("UTF-8"));
        System.out.println(out.size() + " " + out.toString());
        try {
            out.write(new byte[4], 3, 2);
        } catch (IndexOutOfBoundsException e) {
            System.out.println("write out of range");
        }
        try {
            new ByteArrayOutputStream(-1);
        } catch (IllegalArgumentException e) {
            System.out.println(e.getMessage());
        }

        ByteArrayInputStream in = new ByteArrayInputStream(new byte[] {10, 20, 30, 40, 50, 60}, 1, 4);
        System.out.println(in.available() + " " + in.read() + " " + in.markSupported());
        in.mark(0);
        System.out.println(in.skip(2) + " " + in.read() + " " + in.read() + " " + in.skip(-5));
        in.reset();
        byte[] buf = new byte[10];
        System.out.println(in.read(buf, 2, 10 - 2) + " " + hex(buf));
        System.out.println(in.read(buf, 0, 0) + " " + in.skip(100));
        in = new ByteArrayInputStream(new byte[] {1, 2}, 1, 100);
        System.out.println(in.available() + " " + in.read() + " " + in.read());
    }

    public static void main(String[] args) throws IOException {
        roundTrip();
        endOfData();
        badUtf();
        gameStreams();
        gameStreamsEnd();
        byteArrays();
    }
}
