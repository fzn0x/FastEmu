package t;

import javax.microedition.lcdui.Display;
import javax.microedition.midlet.MIDlet;

/**
 * Asks for 20000 callSerially calls while the event thread is busy: the
 * first runnable holds it until the game lets it go. Prints how many calls
 * were taken and how many threw OutOfMemoryError, then how many of the
 * taken runnables ran once the event thread was free.
 */
public class CallFlood extends MIDlet {
    private static final Object LOCK = new Object();
    private static boolean started;
    private static boolean go;
    private static int ran;

    protected void startApp() {
        final Display display = Display.getDisplay(this);
        display.callSerially(new Runnable() {
            public void run() {
                synchronized (LOCK) {
                    started = true;
                    LOCK.notifyAll();
                    while (!go) {
                        try {
                            LOCK.wait();
                        } catch (InterruptedException e) {
                        }
                    }
                }
            }
        });
        // The event thread has the first runnable now, so the queue is
        // empty and every call below waits behind it.
        synchronized (LOCK) {
            while (!started) {
                try {
                    LOCK.wait();
                } catch (InterruptedException e) {
                }
            }
        }
        Runnable count = new Runnable() {
            public void run() {
                ran++;
            }
        };
        int taken = 0;
        int refused = 0;
        String message = null;
        for (int i = 0; i < 20000; i++) {
            try {
                display.callSerially(count);
                taken++;
            } catch (OutOfMemoryError e) {
                refused++;
                message = e.getMessage();
            }
        }
        System.out.println("taken " + taken + " refused " + refused + " " + message);
        synchronized (LOCK) {
            go = true;
            LOCK.notifyAll();
        }
        // Once the event thread has made room, one more runnable reports.
        Runnable report = new Runnable() {
            public void run() {
                System.out.println("ran " + ran);
            }
        };
        while (true) {
            try {
                display.callSerially(report);
                break;
            } catch (OutOfMemoryError e) {
                try {
                    Thread.sleep(5);
                } catch (InterruptedException x) {
                }
            }
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
