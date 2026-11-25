package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/**
 * A game thread paints with serviceRepaints once the event thread has
 * painted, and its paint never returns: the thread sleeps in it for ever
 * and keeps the callback lock, so no key could be delivered.
 */
public class StuckPaint extends MIDlet implements Runnable {
    private StuckPaintCanvas canvas;

    protected void startApp() {
        canvas = new StuckPaintCanvas();
        Thread painter = new Thread(this);
        canvas.painter = painter;
        Display.getDisplay(this).setCurrent(canvas);
        painter.start();
    }

    public void run() {
        try {
            while (!canvas.drawn) {
                Thread.sleep(5);
            }
            // The event thread may take a repaint first; this goes on
            // until this thread paints, which never returns.
            while (true) {
                canvas.repaint();
                canvas.serviceRepaints();
                Thread.sleep(10);
            }
        } catch (InterruptedException e) {
            return;
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class StuckPaintCanvas extends Canvas {
    volatile Thread painter;
    volatile boolean drawn;

    protected void paint(Graphics g) {
        g.setColor(0x446622);
        g.fillRect(0, 0, getWidth(), getHeight());
        if (Thread.currentThread() != painter) {
            drawn = true;
            return;
        }
        System.out.println("stuck in paint");
        while (true) {
            try {
                Thread.sleep(1000);
            } catch (InterruptedException e) {
                return;
            }
        }
    }
}
