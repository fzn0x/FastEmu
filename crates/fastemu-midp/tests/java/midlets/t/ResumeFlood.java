package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/**
 * Pauses itself and asks to be resumed in a tight loop with no sleep, from
 * inside a startApp, as a hostile game could. Its screen turns green once
 * it has asked SHOWN times. It gives up after MOST requests, or as soon as
 * destroyApp runs. Its screen's hideNotify and showNotify and its
 * destroyApp print how many requests it had made then, so a test can tell
 * whether the app's pause, resume and stop came while it still asked.
 */
public class ResumeFlood extends MIDlet {
    /** Requests made before the screen turns green. */
    static final int SHOWN = 1000000;
    /** Requests after which the game gives up. */
    static final int MOST = 4000000;

    static volatile int requests;
    static volatile boolean destroyed;
    static boolean started;

    protected void startApp() {
        if (started) {
            return;
        }
        started = true;
        FloodScreen screen = new FloodScreen();
        Display.getDisplay(this).setCurrent(screen);
        while (!destroyed && requests < MOST) {
            notifyPaused();
            resumeRequest();
            requests++;
            if (requests == SHOWN) {
                screen.repaint();
            }
        }
        System.out.println("flood ended at " + requests);
    }

    protected void pauseApp() {
        System.out.println("pauseApp at " + requests);
    }

    protected void destroyApp(boolean unconditional) {
        destroyed = true;
        System.out.println("destroyed at " + requests);
    }
}

class FloodScreen extends Canvas {
    protected void showNotify() {
        System.out.println("shown at " + ResumeFlood.requests);
    }

    protected void hideNotify() {
        System.out.println("hidden at " + ResumeFlood.requests);
    }

    protected void paint(Graphics g) {
        g.setColor(ResumeFlood.requests >= ResumeFlood.SHOWN ? 0x00FF00 : 0x000000);
        g.fillRect(0, 0, getWidth(), getHeight());
    }
}
