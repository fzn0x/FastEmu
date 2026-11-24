package t;

import java.io.IOException;
import java.io.InputStream;
import javax.microedition.midlet.MIDlet;

/** Skips through a stream that never ends: a loop inside the runtime. */
public class StopLoop extends MIDlet {
    protected void startApp() {
        System.out.println("looping");
        try {
            new EndlessStream().skip(Long.MAX_VALUE);
        } catch (IOException e) {
            System.out.println("skip failed " + e);
        }
        System.out.println("skip returned");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {
        System.out.println("destroyApp " + unconditional);
    }
}

/**
 * A stream whose reads return at once, and hold no backward branch and no
 * call, so that nothing in them is a safepoint.
 */
class EndlessStream extends InputStream {
    public int read() {
        return 0;
    }

    public int read(byte[] b, int off, int len) {
        return len;
    }
}
