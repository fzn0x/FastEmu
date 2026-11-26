package t;

import javax.microedition.midlet.MIDlet;

/**
 * Asks to be resumed from its own pauseApp, as a game that does not want
 * to stay paused might. Prints every lifecycle call.
 */
public class ResumeInPause extends MIDlet {
    private int starts;

    protected void startApp() {
        starts++;
        System.out.println("startApp " + starts);
    }

    protected void pauseApp() {
        resumeRequest();
        System.out.println("pauseApp");
    }

    protected void destroyApp(boolean unconditional) {
        System.out.println("destroyApp " + unconditional);
    }
}
