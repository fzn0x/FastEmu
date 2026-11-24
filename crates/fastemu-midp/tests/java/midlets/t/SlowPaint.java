package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/**
 * A game thread that works between repaint and serviceRepaints while a
 * long paint runs on the event thread, so serviceRepaints finds the
 * paint under way rather than pending.
 */
public class SlowPaint extends MIDlet implements Runnable {
    private SlowPaintCanvas canvas;

    protected void startApp() {
        canvas = new SlowPaintCanvas();
        Display.getDisplay(this).setCurrent(canvas);
        new Thread(this).start();
    }

    public void run() {
        try {
            while (!canvas.isShown()) {
                Thread.sleep(5);
            }
        } catch (InterruptedException e) {
            return;
        }
        int early = 0;
        for (int i = 1; i <= 6; i++) {
            canvas.frame = i;
            canvas.repaint();
            canvas.sink += SlowPaintCanvas.work(1000000);
            canvas.serviceRepaints();
            if (canvas.painted != i) {
                early++;
            }
        }
        System.out.println("slow paint done early " + early);
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class SlowPaintCanvas extends Canvas {
    volatile int frame;
    volatile int painted;
    volatile int sink;

    SlowPaintCanvas() {
        setFullScreenMode(true);
    }

    static int work(int n) {
        int x = 0;
        for (int i = 0; i < n; i++) {
            x = x * 31 + i;
        }
        return x;
    }

    protected void paint(Graphics g) {
        int f = frame;
        sink += work(2000000);
        g.setColor(f * 8, 0, 0);
        g.fillRect(0, 0, getWidth(), getHeight());
        painted = f;
    }
}
