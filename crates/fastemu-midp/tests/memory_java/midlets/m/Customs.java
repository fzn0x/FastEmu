package m;

import java.util.Vector;
import javax.microedition.lcdui.CustomItem;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Form;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/**
 * Shows a form with a small custom item and one of 240 by 8192 pixels,
 * then ten more forms in turn, each with a small custom item, and keeps
 * them all. Prints what each part took of the heap.
 */
public class Customs extends MIDlet {
    private static final Object LOCK = new Object();

    static class Pane extends CustomItem {
        private final int height;
        boolean painted;

        Pane(int height) {
            super(null);
            this.height = height;
        }

        protected int getMinContentWidth() {
            return 10;
        }

        protected int getMinContentHeight() {
            return 10;
        }

        protected int getPrefContentWidth(int h) {
            return 1000;
        }

        protected int getPrefContentHeight(int w) {
            return height;
        }

        protected void paint(Graphics g, int w, int h) {
            synchronized (LOCK) {
                painted = true;
                LOCK.notifyAll();
            }
        }
    }

    private static boolean waitPainted(Pane pane) {
        long end = System.currentTimeMillis() + 10000;
        synchronized (LOCK) {
            while (!pane.painted) {
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

    private static void sleep(int ms) {
        try {
            Thread.sleep(ms);
        } catch (InterruptedException e) {
        }
    }

    protected void startApp() {
        Display display = Display.getDisplay(this);
        System.gc();
        long base = Runtime.getRuntime().freeMemory();

        // The small item comes first, so the form paints it and then meets
        // the big one, which starts under it, on the screen.
        Pane small = new Pane(100);
        Pane big = new Pane(8192);
        Form form = new Form("Customs");
        form.append(small);
        form.append(big);
        display.setCurrent(form);
        if (!waitPainted(small)) {
            System.out.println("custom small item was not painted");
            return;
        }
        sleep(500);
        System.gc();
        System.out.println("custom big painted " + big.painted);
        System.out.println("custom form counts " + (base - Runtime.getRuntime().freeMemory()));

        // Ten forms shown in turn and kept. Each holds a surface of 240 by
        // 20 pixels for its item; only the form on the screen holds the
        // pixels of its own.
        Vector kept = new Vector();
        kept.addElement(form);
        int shown = 0;
        Pane pane = null;
        boolean full = false;
        try {
            for (int i = 0; i < 10; i++) {
                pane = new Pane(20);
                Form next = new Form("Form " + i);
                next.append(pane);
                kept.addElement(next);
                display.setCurrent(next);
                if (!waitPainted(pane)) {
                    System.out.println("custom form " + i + " was not painted");
                    break;
                }
                shown++;
            }
        } catch (OutOfMemoryError e) {
            // Let go of what the game kept before anything else is called,
            // as the heap is over its limit until then.
            full = true;
            pane = null;
            kept = null;
        }
        System.gc();
        System.out.println("custom forms " + shown + " shown" + (full ? " then OutOfMemoryError" : ""));
        System.out.println("custom forms count " + (base - Runtime.getRuntime().freeMemory()));
        System.out.println("done");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
