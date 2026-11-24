import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InterruptedIOException;
import java.io.OutputStream;
import java.io.PrintStream;

// PrintStream over other streams: every print method, the error flag in
// place of IOException, close, and a game's own subclass. Line breaks
// are compared without '\r', which the JDK writes on Windows.
public class PrintStreams {
    // A stream that fails every write, as a closed connection would.
    static class Broken extends OutputStream {
        final boolean interrupt;
        int flushes;

        Broken(boolean interrupt) {
            this.interrupt = interrupt;
        }

        public void write(int b) throws IOException {
            if (interrupt) {
                throw new InterruptedIOException("slow");
            }
            throw new IOException("gone");
        }

        public void flush() {
            flushes++;
        }
    }

    // A stream that says when it is flushed and closed.
    static class Noisy extends ByteArrayOutputStream {
        public void flush() {
            System.out.println("flush");
        }

        public void close() {
            System.out.println("close");
        }
    }

    // A game's print stream that sees every byte its print methods write.
    static class Tee extends PrintStream {
        final StringBuffer seen = new StringBuffer();

        Tee(OutputStream out) {
            super(out);
        }

        public void write(byte[] buf, int off, int len) {
            for (int i = 0; i < len; i++) {
                if (buf[off + i] != '\r') {
                    seen.append((char) buf[off + i]);
                }
            }
            super.write(buf, off, len);
        }
    }

    // One that shows each write(byte[], int, int) the print methods make,
    // in brackets, and counts write(int), which they never call.
    static class Calls extends PrintStream {
        final StringBuffer seen = new StringBuffer();
        int single;

        Calls(OutputStream out) {
            super(out);
        }

        public void write(byte[] buf, int off, int len) {
            seen.append('[');
            for (int i = 0; i < len; i++) {
                if (buf[off + i] != '\r') {
                    seen.append((char) buf[off + i]);
                }
            }
            seen.append(']');
            super.write(buf, off, len);
        }

        public void write(int b) {
            single++;
            super.write(b);
        }
    }

    // One whose print methods mark what they print: println goes through
    // them.
    static class Marked extends PrintStream {
        Marked(OutputStream out) {
            super(out);
        }

        public void print(String s) {
            super.print("<" + s + ">");
        }

        public void print(int i) {
            super.print(i * 2);
        }
    }

    // One that overrides nothing print uses.
    static class Plain extends PrintStream {
        Plain(OutputStream out) {
            super(out);
        }

        public void flush() {
            super.flush();
        }
    }

    static void overrides() {
        Calls calls = new Calls(new ByteArrayOutputStream());
        calls.print("ab");
        calls.print("");
        calls.println(5);
        calls.println("");
        calls.println();
        calls.print('c');
        calls.println(new char[] {'x', 'y'});
        System.out.println("calls " + calls.seen + " single " + calls.single);

        ByteArrayOutputStream marked = new ByteArrayOutputStream();
        Marked m = new Marked(marked);
        m.print("a");
        m.println("b");
        m.println(21);
        m.println(7L);
        m.println(new char[] {'c'});
        m.println((String) null);
        System.out.println("marked [" + text(marked) + "]");

        ByteArrayOutputStream plain = new ByteArrayOutputStream();
        Plain p = new Plain(plain);
        p.print("plain ");
        p.println(1.5);
        System.out.println("plain [" + text(plain) + "]");

        // print("") on a closed stream still finds it closed.
        PrintStream closed = new PrintStream(new ByteArrayOutputStream());
        closed.close();
        closed.print("");
        System.out.println("empty print on a closed stream: error " + closed.checkError());
    }

    // A stream that lets other threads run in the middle of each write.
    static class Slow extends OutputStream {
        final StringBuffer seen = new StringBuffer();

        public void write(int b) {
            seen.append((char) b);
        }

        public void write(byte[] buf, int off, int len) {
            for (int i = 0; i < len; i++) {
                seen.append((char) buf[off + i]);
            }
            try {
                Thread.sleep(1);
            } catch (InterruptedException e) {
                // Nothing interrupts these threads.
            }
        }
    }

    // println holds the print stream's monitor across its text and its
    // line break, so lines from two threads never mix.
    static void threads() throws InterruptedException {
        final Slow slow = new Slow();
        final PrintStream ps = new PrintStream(slow);
        Thread[] writers = new Thread[2];
        for (int t = 0; t < writers.length; t++) {
            final String line = t == 0 ? "aaaa" : "bbbb";
            writers[t] = new Thread() {
                public void run() {
                    for (int i = 0; i < 20; i++) {
                        ps.println(line);
                    }
                }
            };
            writers[t].start();
        }
        for (int t = 0; t < writers.length; t++) {
            writers[t].join();
        }
        String s = slow.seen.toString();
        int lines = 0;
        boolean intact = true;
        int start = 0;
        for (int i = 0; i < s.length(); i++) {
            if (s.charAt(i) == '\n') {
                String l = s.substring(start, i);
                if (l.endsWith("\r")) {
                    l = l.substring(0, l.length() - 1);
                }
                intact &= l.equals("aaaa") || l.equals("bbbb");
                lines++;
                start = i + 1;
            }
        }
        System.out.println("threads: " + lines + " lines, intact " + intact + ", rest " + (s.length() - start));
    }

    static String text(ByteArrayOutputStream bytes) {
        String s = bytes.toString();
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < s.length(); i++) {
            if (s.charAt(i) != '\r') {
                sb.append(s.charAt(i));
            }
        }
        return sb.toString();
    }

    public static void main(String[] args) throws IOException, InterruptedException {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        PrintStream ps = new PrintStream(bytes);
        ps.print(true);
        ps.print('c');
        ps.print(-42);
        ps.print(1L << 40);
        ps.print(1.5f);
        ps.print(-0.25);
        ps.print(new char[] {'x', 'y'});
        ps.print("sé");
        ps.print((String) null);
        ps.print((Object) null);
        ps.print(new StringBuffer("sb"));
        ps.println();
        ps.println(false);
        ps.println('d');
        ps.println(7);
        ps.println(8L);
        ps.println(9.5f);
        ps.println(1e100);
        ps.println(new char[] {'z'});
        ps.println("str");
        ps.println(new Integer(3));
        ps.write('W');
        ps.write(new byte[] {'a', 'b', 'c'}, 1, 2);
        ps.flush();
        System.out.println("[" + text(bytes) + "] " + ps.checkError());
        try {
            ps.print((char[]) null);
        } catch (NullPointerException e) {
            System.out.println("print(null char[]) NPE");
        }
        try {
            ps.write(new byte[2], 1, 2);
        } catch (IndexOutOfBoundsException e) {
            System.out.println("write out of range");
        }

        Broken broken = new Broken(false);
        PrintStream failing = new PrintStream(broken);
        failing.print("lost");
        failing.write(1);
        System.out.println("no exception, error " + failing.checkError() + " flushes " + broken.flushes);

        PrintStream interrupted = new PrintStream(new Broken(true));
        interrupted.print("x");
        System.out.println("interrupted " + Thread.interrupted() + " error " + interrupted.checkError());

        Noisy noisy = new Noisy();
        PrintStream closing = new PrintStream(noisy);
        closing.print("before");
        closing.close();
        closing.close();
        closing.print("after");
        System.out.println(noisy.size() + " " + closing.checkError());

        ByteArrayOutputStream under = new ByteArrayOutputStream();
        Tee tee = new Tee(under);
        tee.print("tee ");
        tee.println(12);
        tee.print(new Object() {
            public String toString() {
                return "obj";
            }
        });
        System.out.println("[" + tee.seen + "] [" + text(under) + "]");

        PrintStream nested = new PrintStream(new PrintStream(new Broken(false)));
        nested.print("deep");
        System.out.println("nested error " + nested.checkError());
        try {
            new PrintStream((OutputStream) null);
        } catch (NullPointerException e) {
            System.out.println("null stream NPE");
        }
        overrides();
        threads();
    }
}
