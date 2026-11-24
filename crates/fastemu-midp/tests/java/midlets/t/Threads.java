package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/** A game thread that paints with repaint and serviceRepaints. */
public class Threads extends MIDlet implements Runnable {
    private ThreadsCanvas canvas;

    protected void startApp() {
        canvas = new ThreadsCanvas();
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
        for (int i = 1; i <= 30; i++) {
            canvas.frame = i;
            canvas.repaint();
            canvas.serviceRepaints();
            if (canvas.painted != i) {
                System.out.println("serviceRepaints returned early " + i + " " + canvas.painted);
            }
        }
        System.out.println("thread done " + canvas.painted);
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class ThreadsCanvas extends Canvas {
    volatile int frame;
    volatile int painted;

    ThreadsCanvas() {
        setFullScreenMode(true);
    }

    protected void paint(Graphics g) {
        g.setColor(frame * 8, 0, 0);
        g.fillRect(0, 0, getWidth(), getHeight());
        painted = frame;
    }
}
