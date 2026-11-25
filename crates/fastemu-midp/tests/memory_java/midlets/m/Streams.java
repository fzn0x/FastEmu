package m;

import java.io.IOException;
import java.io.InputStream;
import javax.microedition.lcdui.Image;
import javax.microedition.midlet.MIDlet;

/**
 * Passes Image.createImage a stream that never ends and one that never
 * gives a byte. Prints the exception and how many reads each got.
 */
public class Streams extends MIDlet {
    /** Every read returns all the bytes it was asked for. */
    static class Endless extends InputStream {
        int calls;
        long bytes;

        public int read() {
            bytes++;
            return 0;
        }

        public int read(byte[] b, int off, int len) {
            calls++;
            if (bytes >= 64L * 1024 * 1024) {
                // The stream ends in the end, so a runtime without a limit
                // reads 64 MiB and not all the memory there is.
                return -1;
            }
            bytes += len;
            return len;
        }
    }

    /** Every read returns 0 bytes. */
    static class Empty extends InputStream {
        int calls;

        public int read() {
            return 0;
        }

        public int read(byte[] b, int off, int len) {
            calls++;
            // Ends after 5000 reads, so a runtime without a limit does not
            // spin for ever.
            return calls > 5000 ? -1 : 0;
        }
    }

    protected void startApp() {
        Endless endless = new Endless();
        try {
            Image.createImage(endless);
            System.out.println("endless image made");
        } catch (IOException e) {
            System.out.println("endless IOException " + e.getMessage() + " calls " + endless.calls
                    + " bytes " + endless.bytes);
        } catch (OutOfMemoryError e) {
            System.out.println("endless OutOfMemoryError calls " + endless.calls
                    + " bytes " + endless.bytes);
        }
        Empty empty = new Empty();
        try {
            Image.createImage(empty);
            System.out.println("empty image made");
        } catch (IOException e) {
            System.out.println("empty IOException " + e.getMessage() + " calls " + empty.calls);
        }
        System.out.println("done");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
