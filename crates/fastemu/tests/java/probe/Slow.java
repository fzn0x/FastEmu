package probe;

import javax.microedition.lcdui.Display;
import javax.microedition.midlet.MIDlet;

/**
 * Draws three pictures 400 ms apart, then keeps the last one on screen,
 * as a puzzle game that redraws only on a timer tick and then shows its
 * game-over screen.
 */
public class Slow extends MIDlet implements Runnable {
    private final Screen screen = new Screen();
    private boolean started;

    protected void startApp() {
        if (started) {
            return;
        }
        started = true;
        Display.getDisplay(this).setCurrent(screen);
        new Thread(this).start();
    }

    public void run() {
        for (int i = 0; i < 3; i++) {
            screen.tick++;
            screen.repaint();
            screen.serviceRepaints();
            try {
                Thread.sleep(400);
            } catch (InterruptedException e) {
                return;
            }
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
