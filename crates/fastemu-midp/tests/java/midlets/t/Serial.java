package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/** A runnable that posts itself with callSerially two thousand times. */
public class Serial extends MIDlet implements Runnable {
    private Display display;
    private SerialCanvas canvas;
    private int runs;

    protected void startApp() {
        display = Display.getDisplay(this);
        canvas = new SerialCanvas();
        display.setCurrent(canvas);
        display.callSerially(this);
        display.callSerially(null);
    }

    public void run() {
        runs++;
        if (runs % 500 == 0) {
            canvas.repaint();
        }
        if (runs < 2000) {
            display.callSerially(this);
        } else {
            System.out.println("serial done " + runs + " paints " + canvas.paints);
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class SerialCanvas extends Canvas {
    int paints;

    protected void paint(Graphics g) {
        paints++;
    }
}
