package probe;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;
import javax.microedition.rms.RecordStore;
import javax.microedition.rms.RecordStoreException;

/**
 * Counts its runs in a record store, draws a new picture every 10 ms and
 * prints every key and pointer event it gets.
 */
public class Probe extends MIDlet implements Runnable {
    private final Screen screen = new Screen();
    private boolean started;

    protected void startApp() {
        if (started) {
            return;
        }
        started = true;
        try {
            RecordStore store = RecordStore.openRecordStore("runs", true);
            System.out.println("runs before " + store.getNumRecords());
            byte[] run = {1};
            store.addRecord(run, 0, run.length);
            store.closeRecordStore();
        } catch (RecordStoreException e) {
            System.out.println("record store failed: " + e);
        }
        Display.getDisplay(this).setCurrent(screen);
        new Thread(this).start();
    }

    public void run() {
        while (true) {
            screen.tick++;
            screen.repaint();
            screen.serviceRepaints();
            try {
                Thread.sleep(10);
            } catch (InterruptedException e) {
                return;
            }
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class Screen extends Canvas {
    int tick;

    Screen() {
        setFullScreenMode(true);
    }

    protected void paint(Graphics g) {
        g.setColor(tick * 0x010203);
        g.fillRect(0, 0, getWidth(), getHeight());
    }

    protected void keyPressed(int keyCode) {
        System.out.println("key " + keyCode);
    }

    protected void pointerPressed(int x, int y) {
        System.out.println("pressed " + x + " " + y);
    }

    protected void pointerDragged(int x, int y) {
        System.out.println("dragged " + x + " " + y);
    }

    protected void pointerReleased(int x, int y) {
        System.out.println("released " + x + " " + y);
    }
}
