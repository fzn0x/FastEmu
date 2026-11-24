import java.io.InputStream;
import java.util.Vector;

/**
 * Programs that fill memory through natives, which allocate without
 * checking the heap limit. Each case loops a bounded number of times and
 * returns the round at which it caught an OutOfMemoryError, or -1 when it
 * ran to the end without one. They use only CLDC APIs.
 */
public class HeapCases {
    /** s = s.concat(s): the string doubles every round, 2^21 characters at most. */
    static int concat() {
        String s = "ab";
        int i = 0;
        try {
            for (; i < 20; i++) {
                s = s.concat(s);
            }
        } catch (OutOfMemoryError e) {
            return i;
        }
        return -1;
    }

    /** One buffer takes 32 characters 200000 times: 6.4 million characters at most. */
    static int append() {
        StringBuffer sb = new StringBuffer();
        String piece = "01234567890123456789012345678901";
        int i = 0;
        try {
            for (; i < 200000; i++) {
                sb.append(piece);
            }
        } catch (OutOfMemoryError e) {
            return i;
        }
        return -1;
    }

    /** Many buffers, each grown by setLength and kept. */
    static int buffers() {
        Vector keep = new Vector();
        int i = 0;
        try {
            for (; i < 100; i++) {
                StringBuffer sb = new StringBuffer();
                sb.setLength(5000);
                keep.addElement(sb);
            }
        } catch (OutOfMemoryError e) {
            return i;
        }
        return -1;
    }

    static Exception deep(int depth) {
        if (depth == 0) {
            return new RuntimeException("deep");
        }
        return deep(depth - 1);
    }

    /** Exceptions made 100 frames down and kept: 1000 of them. */
    static int exceptions() {
        Vector keep = new Vector();
        int i = 0;
        try {
            for (; i < 1000; i++) {
                keep.addElement(deep(100));
            }
        } catch (OutOfMemoryError e) {
            return i;
        }
        return -1;
    }

    /** Exceptions made 300 frames down and dropped at once: 2000 of them. */
    static int exceptionGarbage() {
        int i = 0;
        for (; i < 2000; i++) {
            deep(300);
        }
        return i;
    }

    /** Resource streams of /big.bin (64 KiB), kept: 100 of them. */
    static int resources() {
        Vector keep = new Vector();
        int i = 0;
        try {
            for (; i < 100; i++) {
                InputStream in = HeapCases.class.getResourceAsStream("/big.bin");
                keep.addElement(in);
            }
        } catch (OutOfMemoryError e) {
            return i;
        }
        return -1;
    }

    /** toCharArray of an 8000 character string, kept: 300 of them. */
    static int chars() {
        String s = new String(new char[8000]);
        Vector keep = new Vector();
        int i = 0;
        try {
            for (; i < 300; i++) {
                keep.addElement(s.toCharArray());
            }
        } catch (OutOfMemoryError e) {
            return i;
        }
        return -1;
    }

    /** Clones of a 100000 byte array, kept: 100 of them. */
    static int clones() {
        byte[] source = new byte[100000];
        Vector keep = new Vector();
        int i = 0;
        try {
            for (; i < 100; i++) {
                keep.addElement(source.clone());
            }
        } catch (OutOfMemoryError e) {
            return i;
        }
        return -1;
    }

    /** Interned strings are roots: 10000 different ones. */
    static int intern() {
        int i = 0;
        try {
            for (; i < 10000; i++) {
                Integer.toString(i).intern();
            }
        } catch (OutOfMemoryError e) {
            return i;
        }
        return -1;
    }

    /**
     * 100 threads made with a 100000 character name and kept. Returns the
     * longest name any of them reports, or -1 when one threw
     * OutOfMemoryError.
     */
    static int threadNames() {
        char[] text = new char[100000];
        for (int i = 0; i < text.length; i++) {
            text[i] = 'n';
        }
        String name = new String(text);
        Vector keep = new Vector();
        int longest = 0;
        try {
            for (int i = 0; i < 100; i++) {
                Thread t = new Thread(new Idle(), name);
                keep.addElement(t);
                longest = Math.max(longest, t.getName().length());
            }
        } catch (OutOfMemoryError e) {
            return -1;
        }
        return longest;
    }

    static class Idle implements Runnable {
        public void run() {
        }
    }

    /** Allocates normally: what a game does after it let go of its data. */
    static int recover() {
        int[] numbers = new int[8192];
        Vector strings = new Vector();
        for (int i = 0; i < 50; i++) {
            strings.addElement("round " + i);
        }
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < 100; i++) {
            sb.append("text ").append(i);
        }
        return numbers.length + strings.size() + sb.length();
    }

    static volatile boolean stop;
    static volatile int spins;
    static volatile boolean spinnerFailed;

    /** A thread that allocates nothing: a plain loop with a native call. */
    static class Spinner implements Runnable {
        public void run() {
            try {
                while (!stop) {
                    spins++;
                    Math.abs(spins);
                }
            } catch (Throwable t) {
                spinnerFailed = true;
            }
        }
    }

    /** Waits until the spinner has run 2000 more rounds. */
    static boolean spinnerRuns() throws InterruptedException {
        int from = spins;
        for (int i = 0; i < 20000 && spins < from + 2000; i++) {
            Thread.sleep(1);
        }
        return spins >= from + 2000;
    }

    /**
     * Two threads share the heap. This one fills it through natives until
     * it gets OutOfMemoryError, holds the data while the other thread
     * spins without allocating, then lets go and does it thirty times
     * more. Returns 0 when this thread got OutOfMemoryError every time
     * and the other never did, and a sum of 1 (this thread missed one), 2
     * (the other got one) and 4 (the other did not run).
     */
    static int pair() throws InterruptedException {
        stop = false;
        spins = 0;
        spinnerFailed = false;
        int result = 0;
        Thread spinner = new Thread(new Spinner());
        spinner.start();
        if (!spinnerRuns()) {
            result |= 4;
        }
        String s = new String(new char[8000]);
        Vector keep = new Vector();
        boolean caught = false;
        try {
            for (int i = 0; i < 300; i++) {
                keep.addElement(s.toCharArray());
            }
        } catch (OutOfMemoryError e) {
            caught = true;
        }
        if (!caught) {
            result |= 1;
        }
        // The heap stays over its limit while this thread holds the data.
        if (!spinnerRuns()) {
            result |= 4;
        }
        keep = null;
        for (int round = 0; round < 30; round++) {
            String t = "ab";
            caught = false;
            try {
                for (int i = 0; i < 20; i++) {
                    t = t.concat(t);
                }
            } catch (OutOfMemoryError e) {
                caught = true;
            }
            if (!caught) {
                result |= 1;
            }
            Thread.yield();
        }
        stop = true;
        spinner.join();
        if (spinnerFailed) {
            result |= 2;
        }
        return result;
    }
}
