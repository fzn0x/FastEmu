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

    static int keepNativeThrown(Object[] keep, int depth) {
        if (depth > 0) {
            return keepNativeThrown(keep, depth - 1);
        }
        String text = "abc";
        int i = 0;
        try {
            for (; i < keep.length; i++) {
                try {
                    text.charAt(10);
                } catch (StringIndexOutOfBoundsException e) {
                    keep[i] = e;
                }
            }
        } catch (OutOfMemoryError e) {
            return i;
        }
        return -1;
    }

    /**
     * Exceptions that a native throws 100 frames down, kept in an array
     * made first: 2000 of them. Nothing in the loop allocates with new or
     * returns from a native without an exception.
     */
    static int nativeThrown() {
        return keepNativeThrown(new Object[2000], 100);
    }

    static int keepFaultThrown(Object[] keep, int depth) {
        if (depth > 0) {
            return keepFaultThrown(keep, depth - 1);
        }
        Object nothing = null;
        int i = 0;
        try {
            for (; i < keep.length; i++) {
                try {
                    nothing.hashCode();
                } catch (NullPointerException e) {
                    keep[i] = e;
                }
            }
        } catch (OutOfMemoryError e) {
            return i;
        }
        return -1;
    }

    /** As nativeThrown, with a NullPointerException the interpreter throws. */
    static int faultThrown() {
        return keepFaultThrown(new Object[2000], 100);
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

    /**
     * Resource streams of /big.bin (64 KiB), kept until one throws. Returns
     * how many it held. Each stream's array is made, then the stream object
     * is built around it by a second native, so the native that made the
     * array has allocated before it calls another one.
     */
    static int streamsHeld() {
        Vector keep = new Vector();
        int i = 0;
        try {
            for (; i < 100; i++) {
                keep.addElement(HeapCases.class.getResourceAsStream("/big.bin"));
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

    /**
     * This thread grows a buffer past the heap limit with natives and keeps
     * it, so the live data stays over the limit after the error. The other
     * thread allocates nothing. Returns 0 when this thread got the error
     * from the append, no call that allocates nothing (length, sleep, join)
     * threw in either thread, and a sum of 1 (no error from the append), 2
     * (the other thread got one), 4 (the other thread did not run) and 8
     * (a call of this thread that allocates nothing threw). The sizes fit
     * the 256 KiB heap of the tests: the array and the buffer, 150000 and
     * 147000 bytes, come to more than the heap.
     */
    static int keptBuffer() throws InterruptedException {
        stop = false;
        spins = 0;
        spinnerFailed = false;
        int result = 0;
        Thread spinner = new Thread(new Spinner());
        spinner.start();
        if (!spinnerRuns()) {
            result |= 4;
        }
        byte[] held = new byte[150000];
        StringBuffer sb = new StringBuffer();
        String piece = "01234567890123456789012345678901";
        boolean caught = false;
        try {
            for (int i = 0; i < 200000; i++) {
                sb.append(piece);
            }
        } catch (OutOfMemoryError e) {
            caught = true;
        }
        if (!caught) {
            result |= 1;
        }
        // The buffer is still held, and the heap is over its limit.
        int kept = held.length;
        try {
            kept += sb.length();
        } catch (OutOfMemoryError e) {
            result |= 8;
        }
        for (int round = 0; round < 3; round++) {
            try {
                if (!spinnerRuns()) {
                    result |= 4;
                }
            } catch (OutOfMemoryError e) {
                result |= 8;
            }
        }
        stop = true;
        try {
            spinner.join();
        } catch (OutOfMemoryError e) {
            result |= 8;
        }
        if (spinnerFailed) {
            result |= 2;
        }
        return kept > 0 ? result : -1;
    }

    static volatile boolean holderGrown;
    static volatile boolean holderDone;
    static volatile int holderResult;
    static volatile int holderKept;

    /** Grows a buffer past the heap limit, keeps it, and waits. */
    static class Holder implements Runnable {
        public void run() {
            byte[] held = new byte[150000];
            StringBuffer sb = new StringBuffer();
            String piece = "01234567890123456789012345678901";
            try {
                for (int i = 0; i < 200000; i++) {
                    sb.append(piece);
                }
                holderResult |= 1;
            } catch (OutOfMemoryError e) {
                // The error is expected.
            }
            holderGrown = true;
            try {
                while (!holderDone) {
                    Thread.sleep(1);
                }
            } catch (Throwable t) {
                holderResult |= 2;
            }
            holderKept = held.length + sb.length();
        }
    }

    /**
     * Another thread grows a buffer past the heap limit and keeps it while
     * this thread sleeps and wakes. Returns 0 when the holder got the error
     * from its append and neither thread got one from a call that
     * allocates nothing; a sum of 1 (the holder got no error), 2 (the
     * holder's sleep threw) and 8 (this thread's sleep or join threw).
     */
    static int sleepingWhileAnotherKeeps() throws InterruptedException {
        holderGrown = false;
        holderDone = false;
        holderResult = 0;
        int result = 0;
        Thread holder = new Thread(new Holder());
        holder.start();
        try {
            for (int i = 0; i < 20000 && !holderGrown; i++) {
                Thread.sleep(1);
            }
            // The holder keeps its data and sleeps; this thread does too.
            for (int i = 0; i < 20; i++) {
                Thread.sleep(1);
            }
        } catch (OutOfMemoryError e) {
            result |= 8;
        }
        holderDone = true;
        try {
            holder.join();
        } catch (OutOfMemoryError e) {
            result |= 8;
        }
        return result | holderResult;
    }
}
