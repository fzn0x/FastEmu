import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.Reader;
import java.io.UnsupportedEncodingException;
import java.io.Writer;

// Readers and writers in every charset FastEmu has, the conversions of
// String, malformed input, and game readers, writers and streams of
// their own.
public class Readers {
    static final String[] CHARSETS = {
        "UTF-8", "ISO-8859-1", "US-ASCII", "windows-1250", "windows-1251",
        "windows-1252", "ISO-8859-2", "KOI8-R",
    };

    static final String SAMPLE =
        "Az éàüß łżę Жёя €™ 中 😀  ~";

    // A game's own stream over some bytes, which only overrides read().
    static class Slow extends InputStream {
        final byte[] data;
        int pos;

        Slow(byte[] data) {
            this.data = data;
        }

        public int read() {
            return pos < data.length ? data[pos++] & 0xFF : -1;
        }
    }

    // A game's own reader over a string.
    static class StringSource extends Reader {
        final String text;
        int pos;

        StringSource(String text) {
            this.text = text;
        }

        public int read(char[] buf, int off, int len) {
            if (pos >= text.length()) {
                return -1;
            }
            int n = Math.min(len, Math.min(3, text.length() - pos));
            text.getChars(pos, pos + n, buf, off);
            pos += n;
            return n;
        }

        public void close() {
            System.out.println("source closed");
        }
    }

    // A game's own writer that collects what it is given.
    static class Collect extends Writer {
        final StringBuffer seen = new StringBuffer();

        public void write(char[] buf, int off, int len) {
            seen.append('[').append(buf, off, len).append(']');
        }

        public void flush() {
            seen.append("(flush)");
        }

        public void close() {
            seen.append("(close)");
        }
    }

    static String hex(byte[] bytes) {
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < bytes.length; i++) {
            sb.append(Integer.toHexString(bytes[i] & 0xFF)).append(' ');
        }
        return sb.toString();
    }

    static String codes(String s) {
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < s.length(); i++) {
            sb.append(Integer.toHexString(s.charAt(i))).append(' ');
        }
        return sb.toString();
    }

    static String readAll(Reader r) throws IOException {
        StringBuffer sb = new StringBuffer();
        int c;
        while ((c = r.read()) != -1) {
            sb.append((char) c);
        }
        return sb.toString();
    }

    static void charsets() throws IOException {
        byte[] high = new byte[128];
        for (int i = 0; i < 128; i++) {
            high[i] = (byte) (0x80 + i);
        }
        for (int i = 0; i < CHARSETS.length; i++) {
            String cs = CHARSETS[i];
            System.out.println(cs + " decodes " + codes(new String(high, cs)));
            byte[] encoded = SAMPLE.getBytes(cs);
            System.out.println(cs + " encodes " + hex(encoded));
            String back = readAll(new InputStreamReader(new ByteArrayInputStream(encoded), cs));
            System.out.println(cs + " reads " + codes(back));
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            OutputStreamWriter w = new OutputStreamWriter(out, cs);
            w.write(SAMPLE);
            w.flush();
            System.out.println(cs + " writes " + hex(out.toByteArray()));
            System.out.println(cs + " part " + new String(encoded, 1, 4, cs).length());
        }
    }

    static void malformed() throws IOException {
        byte[][] cases = {
            {'A', (byte) 0xFF, 'B'},
            {(byte) 0xED, (byte) 0xA0, (byte) 0x80, 'A'},
            {'A', (byte) 0xE2, (byte) 0x82},
            {(byte) 0xF0, (byte) 0x9F, 'A', 'B'},
            {(byte) 0xC0, (byte) 0x80, (byte) 0xE0, (byte) 0x80, (byte) 0x80},
            {(byte) 0xF4, (byte) 0x90, (byte) 0x80, (byte) 0x80, 'z'},
            {(byte) 0xF0, (byte) 0x9F, (byte) 0x98, (byte) 0x80, (byte) 0xC3},
        };
        for (int i = 0; i < cases.length; i++) {
            String s = new String(cases[i], "UTF-8");
            String r = readAll(new InputStreamReader(new ByteArrayInputStream(cases[i]), "UTF-8"));
            String slow = readAll(new InputStreamReader(new Slow(cases[i]), "UTF-8"));
            System.out.println(codes(s) + "| " + codes(r) + "| " + codes(slow));
        }
    }

    static void readerMethods() throws IOException {
        byte[] bytes = "hello, wide € world".getBytes("UTF-8");
        InputStreamReader r = new InputStreamReader(new ByteArrayInputStream(bytes), "UTF-8");
        System.out.println(r.ready());
        char[] buf = new char[8];
        int n = r.read(buf, 1, 5);
        System.out.println(n + " [" + new String(buf, 1, n) + "]");
        System.out.println(r.skip(3) + " " + (char) r.read());
        n = r.read(buf);
        System.out.println(n + " [" + new String(buf, 0, n) + "]");
        System.out.println(r.read(buf, 0, 0) + " " + r.read(buf) + " " + r.ready());
        r.close();
        try {
            r.read();
            System.out.println("read after close");
        } catch (IOException e) {
            System.out.println("closed: " + e.getMessage());
        }

        InputStreamReader slow = new InputStreamReader(new Slow("abcé".getBytes("UTF-8")), "UTF-8");
        n = slow.read(buf, 0, 8);
        System.out.println(n + " [" + new String(buf, 0, n) + "] " + slow.read());

        StringSource source = new StringSource("0123456789");
        System.out.println((char) source.read() + " " + source.skip(4) + " " + source.read(buf) + " " + new String(buf, 0, 3));
        System.out.println(source.ready() + " " + source.markSupported());
        try {
            source.mark(1);
        } catch (IOException e) {
            System.out.println("mark: " + e.getMessage());
        }
        try {
            source.reset();
        } catch (IOException e) {
            System.out.println("reset: " + e.getMessage());
        }
        try {
            source.skip(-1);
        } catch (IllegalArgumentException e) {
            System.out.println("skip: " + e.getMessage());
        }
        System.out.println(readAll(source) + "|");
        source.close();
    }

    static void writerMethods() throws IOException {
        Collect c = new Collect();
        c.write('x');
        c.write("hello");
        c.write("abcdef", 2, 3);
        c.write(new char[] {'q', 'r'});
        c.flush();
        c.close();
        System.out.println(c.seen);
        try {
            c.write("abc", 2, 5);
        } catch (IndexOutOfBoundsException e) {
            System.out.println("write(String) out of range");
        }

        ByteArrayOutputStream out = new ByteArrayOutputStream();
        OutputStreamWriter w = new OutputStreamWriter(out, "UTF-8");
        w.write('\ud83d');
        w.write('\ude00');
        w.write("x\ud83d", 0, 2);
        w.write(new char[] {'\ude03', 'y'}, 0, 2);
        w.write(0x20ac);
        w.flush();
        System.out.println(hex(out.toByteArray()));
        w.close();
        try {
            w.write('z');
            System.out.println("write after close");
        } catch (IOException e) {
            System.out.println("closed: " + e.getMessage());
        }
        try {
            w.write(new char[2], 1, 2);
        } catch (IndexOutOfBoundsException e) {
            System.out.println("write(char[]) out of range");
        } catch (IOException e) {
            System.out.println("closed first");
        }

        out = new ByteArrayOutputStream();
        w = new OutputStreamWriter(out, "ISO-8859-1");
        w.write("café € 😀");
        w.close();
        System.out.println(hex(out.toByteArray()));
    }

    static void unknownNames() throws IOException {
        byte[] b = {65};
        try {
            new String(b, "x-no-such-charset");
        } catch (UnsupportedEncodingException e) {
            System.out.println("String: " + e.getMessage());
        }
        try {
            "A".getBytes("x-no-such-charset");
        } catch (UnsupportedEncodingException e) {
            System.out.println("getBytes: " + e.getMessage());
        }
        try {
            new InputStreamReader(new ByteArrayInputStream(b), "x-no-such-charset");
        } catch (UnsupportedEncodingException e) {
            System.out.println("reader: " + e.getMessage());
        }
        try {
            new OutputStreamWriter(new ByteArrayOutputStream(), "x-no-such-charset");
        } catch (UnsupportedEncodingException e) {
            System.out.println("writer: " + e.getMessage());
        }
        System.out.println(new String(b, "utf-8") + new String(b, "iso-8859-1") + new String(b, "Cp1251"));
    }

    static void defaults() throws IOException {
        String text = "défaut €";
        byte[] bytes = text.getBytes();
        System.out.println(hex(bytes) + "| " + new String(bytes).equals(text));
        System.out.println(readAll(new InputStreamReader(new ByteArrayInputStream(bytes))).equals(text));
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        OutputStreamWriter w = new OutputStreamWriter(out);
        w.write(text);
        w.flush();
        System.out.println(hex(out.toByteArray()) + "| " + out.toString().equals(text));
    }

    public static void main(String[] args) throws IOException {
        charsets();
        malformed();
        readerMethods();
        writerMethods();
        unknownNames();
        defaults();
    }
}
