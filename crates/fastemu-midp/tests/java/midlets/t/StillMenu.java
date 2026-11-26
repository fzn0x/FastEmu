package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/**
 * Draws a menu once and waits for a key, as a title screen does. When its
 * suite sets the property Spin to yes, a thread of its own also spins for
 * ever without drawing.
 */
public class StillMenu extends MIDlet implements Runnable {
    static volatile int sink;

    protected void startApp() {
        Display.getDisplay(this).setCurrent(new StillMenuCanvas());
        if ("yes".equals(getAppProperty("Spin"))) {
            new Thread(this).start();
        }
    }

    public void run() {
        System.out.println("spinning");
        for (int i = 0; ; i++) {
            sink = i;
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class StillMenuCanvas extends Canvas {
    protected void paint(Graphics g) {
        g.setColor(0x224466);
        g.fillRect(0, 0, getWidth(), getHeight());
    }

    protected void keyPressed(int code) {
        System.out.println("key " + code);
    }
}
