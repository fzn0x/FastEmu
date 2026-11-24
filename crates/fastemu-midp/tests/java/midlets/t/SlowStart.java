package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/** Loads for a while inside its first startApp and logs overlapping calls. */
public class SlowStart extends MIDlet {
    private int starts;
    private int inside;

    protected void startApp() {
        starts++;
        int me = starts;
        if (inside > 0) {
            System.out.println("overlap in startApp " + me);
        }
        inside++;
        System.out.println("start enter " + me);
        if (me == 1) {
            Display.getDisplay(this).setCurrent(new SlowStartCanvas());
            try {
                Thread.sleep(1500);
            } catch (InterruptedException e) {
                // carry on
            }
        }
        System.out.println("start exit " + me);
        inside--;
    }

    protected void pauseApp() {
        if (inside > 0) {
            System.out.println("overlap in pauseApp");
        }
        System.out.println("pauseApp");
    }

    protected void destroyApp(boolean unconditional) {
        System.out.println("destroyApp " + unconditional);
    }
}

class SlowStartCanvas extends Canvas {
    protected void paint(Graphics g) {
        g.setColor(0x00FF00);
        g.fillRect(0, 0, getWidth(), getHeight());
    }
}
