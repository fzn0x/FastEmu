package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/** Its first key press never returns: the event thread sleeps in it for ever. */
public class StuckKey extends MIDlet {
    protected void startApp() {
        Display.getDisplay(this).setCurrent(new StuckKeyCanvas());
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class StuckKeyCanvas extends Canvas {
    protected void paint(Graphics g) {
        g.setColor(0x336699);
        g.fillRect(0, 0, getWidth(), getHeight());
    }

    protected void keyPressed(int code) {
        System.out.println("stuck");
        while (true) {
            try {
                Thread.sleep(1000);
            } catch (InterruptedException e) {
                return;
            }
        }
    }
}
