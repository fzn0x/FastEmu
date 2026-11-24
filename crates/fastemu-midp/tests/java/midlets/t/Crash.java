package t;

import javax.microedition.midlet.MIDlet;

/** Throws out of startApp. */
public class Crash extends MIDlet {
    protected void startApp() {
        throw new RuntimeException("boom");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {
        System.out.println("destroyApp after crash");
    }
}
