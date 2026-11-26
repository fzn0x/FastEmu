package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/**
 * Pauses itself once, from its first startApp, and returns: a game that
 * waits for the player in the paused state. Prints every lifecycle call
 * and its screen's showNotify and hideNotify.
 */
public class SelfPause extends MIDlet {
    private int starts;

    protected void startApp() {
        starts++;
        System.out.println("startApp " + starts);
        if (starts == 1) {
            Display.getDisplay(this).setCurrent(new SelfPauseScreen());
            notifyPaused();
            System.out.println("paused itself");
        }
    }

    protected void pauseApp() {
        System.out.println("pauseApp");
    }

    protected void destroyApp(boolean unconditional) {
        System.out.println("destroyApp " + unconditional);
    }
}

class SelfPauseScreen extends Canvas {
    protected void paint(Graphics g) {
        g.setColor(0x0000FF);
        g.fillRect(0, 0, getWidth(), getHeight());
    }

    protected void showNotify() {
        System.out.println("showNotify");
    }

    protected void hideNotify() {
        System.out.println("hideNotify");
    }
}
