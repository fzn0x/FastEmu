package m;

import java.util.Vector;
import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/**
 * Shows canvas after canvas and keeps every one in a list, as a game that
 * makes a new screen for each level might. Prints what the first shown
 * canvas takes of the heap and how many are shown before the heap is full.
 */
public class Canvases extends MIDlet {
    private static final Object LOCK = new Object();

    static class Page extends Canvas {
        boolean painted;

        protected void paint(Graphics g) {
            g.setColor(0x336699);
            g.fillRect(0, 0, getWidth(), getHeight());
            synchronized (LOCK) {
                painted = true;
                LOCK.notifyAll();
            }
        }
    }

    private static boolean waitPainted(Page page) {
        long end = System.currentTimeMillis() + 10000;
        synchronized (LOCK) {
            while (!page.painted) {
                long left = end - System.currentTimeMillis();
                if (left <= 0) {
                    return false;
                }
                try {
                    LOCK.wait(left);
                } catch (InterruptedException e) {
                }
            }
        }
        return true;
    }

    protected void startApp() {
        Display display = Display.getDisplay(this);
        System.gc();
        long base = Runtime.getRuntime().freeMemory();
        Vector kept = new Vector();
        Page page = null;
        int shown = 0;
        boolean full = false;
        try {
            for (int i = 0; i < 50; i++) {
                page = new Page();
                kept.addElement(page);
                display.setCurrent(page);
                if (!waitPainted(page)) {
                    System.out.println("canvas " + i + " was not painted");
                    break;
                }
                shown++;
                if (i == 0) {
                    System.gc();
                    System.out.println("first canvas counts " + (base - Runtime.getRuntime().freeMemory()));
                }
            }
        } catch (OutOfMemoryError e) {
            // Let go of what the game kept before anything else is called:
            // the heap is over its limit until then, and a call to the
            // runtime that returns while it is would throw again.
            full = true;
            page = null;
            kept = null;
        }
        kept = null;
        System.gc();
        System.out.println("canvases " + shown + " shown" + (full ? " then OutOfMemoryError" : ""));
        System.out.println("done");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
