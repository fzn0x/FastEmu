package m;

import java.io.IOException;
import java.io.InputStream;
import javax.microedition.lcdui.Image;
import javax.microedition.midlet.MIDlet;

/**
 * Eight threads do the same work at the same time, each what work() says.
 * They note what they got in arrays made first, as the heap may be full
 * while the others work, and the last one to finish prints every thread's
 * line, "name id outcome count", and then "done".
 */
public abstract class AtOnce extends MIDlet {
    static final int THREADS = 8;
    static final Object LOCK = new Object();
    static boolean go;
    static int finished;
    /** What each thread got: 0 nothing yet, 1 made, 2 IOException, 3 OutOfMemoryError, 4 another error. */
    static final int[] OUTCOME = new int[THREADS];
    /** What each thread counted: the images it made, or the bytes its stream gave. */
    static final long[] COUNT = new long[THREADS];
    static final String[] NAMES = {"none", "made", "IOException", "OutOfMemoryError", "other"};

    /** Makes the image /big.png three times by name and drops it. */
    static void decode(int id) {
        OUTCOME[id] = 1;
        for (int i = 0; i < 3; i++) {
            try {
                if (Image.createImage("/big.png").getWidth() == 1024) {
                    COUNT[id]++;
                }
            } catch (IOException e) {
                OUTCOME[id] = 2;
            } catch (OutOfMemoryError e) {
                OUTCOME[id] = 3;
            }
        }
    }

    /** A stream that never ends and lets the other threads run on each read. */
    static class Endless extends InputStream {
        long bytes;

        public int read() {
            bytes++;
            return 0;
        }

        public int read(byte[] b, int off, int len) {
            Thread.yield();
            // The stream ends in the end, so a runtime without a limit
            // reads 64 MiB a thread and not all the memory there is.
            if (bytes >= 64L * 1024 * 1024) {
                return -1;
            }
            bytes += len;
            return len;
        }
    }

    /** Passes Image.createImage a stream that never ends. */
    static void image(int id) {
        Endless endless = new Endless();
        try {
            Image.createImage(endless);
            OUTCOME[id] = 1;
        } catch (IOException e) {
            OUTCOME[id] = 2;
        } catch (OutOfMemoryError e) {
            OUTCOME[id] = 3;
        }
        COUNT[id] = endless.bytes;
    }

    /** The name the lines start with. */
    abstract String name();

    /** What thread `id` does. */
    abstract void work(int id);

    protected void startApp() {
        final AtOnce midlet = this;
        for (int i = 0; i < THREADS; i++) {
            final int id = i;
            new Thread() {
                public void run() {
                    synchronized (LOCK) {
                        while (!go) {
                            try {
                                LOCK.wait();
                            } catch (InterruptedException e) {
                                return;
                            }
                        }
                    }
                    try {
                        midlet.work(id);
                    } catch (Throwable t) {
                        OUTCOME[id] = 4;
                    } finally {
                        synchronized (LOCK) {
                            finished++;
                            if (finished == THREADS) {
                                for (int j = 0; j < THREADS; j++) {
                                    System.out.println(midlet.name() + " " + j + " "
                                            + NAMES[OUTCOME[j]] + " " + COUNT[j]);
                                }
                                System.out.println("done");
                            }
                        }
                    }
                }
            }.start();
        }
        synchronized (LOCK) {
            go = true;
            LOCK.notifyAll();
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
