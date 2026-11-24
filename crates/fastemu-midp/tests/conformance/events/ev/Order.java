package ev;

import java.util.Timer;
import javax.microedition.lcdui.Display;
import javax.microedition.midlet.MIDlet;

/**
 * Logs the order of the UI callbacks: sizeChanged, showNotify, paint,
 * callSerially runnables, a timer task, a game thread and keyPressed,
 * calling serviceRepaints from a callback, from the timer and from the
 * game thread. Each step starts the next, so the order is fixed; the
 * test compares the "ev" lines with the order MIDP gives.
 */
public class Order extends MIDlet {
    private Display display;
    private OrderCanvas first;
    private OrderCanvas second;
    private Timer timer;

    static void log(String what) {
        System.out.println("ev " + what);
    }

    public void startApp() {
        if (first != null) {
            return;
        }
        log("startApp");
        display = Display.getDisplay(this);
        first = new OrderCanvas(this, "c1", 0x204000);
        second = new OrderCanvas(this, "c2", 0x0000F0);
        display.setCurrent(first);
        display.callSerially(new Step(this, 1));
    }

    public void pauseApp() {
    }

    public void destroyApp(boolean unconditional) {
    }

    /** A callSerially runnable, on the event thread. */
    void step(int id) {
        if (id == 1) {
            log("serial A");
            first.repaint();
            first.serviceRepaints();
            log("serial A done");
            display.callSerially(new Step(this, 2));
        } else if (id == 2) {
            log("serial B");
            timer = new Timer();
            timer.schedule(new Tick(this), 10);
        } else {
            log("serial C");
            display.setCurrent(second);
        }
    }

    /** The timer task, on the timer's thread. */
    void tick() {
        log("timer");
        first.repaint();
        first.serviceRepaints();
        log("timer done");
        timer.cancel();
        new Thread(new Step(this, 0)).start();
    }

    /** The game thread. */
    void game() {
        log("thread");
        first.repaint();
        first.serviceRepaints();
        log("thread done");
        log("ready for keys");
    }

    /** keyPressed of the first canvas, on the event thread. */
    void keyPressed(int code) {
        log("c1 keyPressed " + code);
        first.repaint();
        first.serviceRepaints();
        log("c1 keyPressed served");
        display.callSerially(new Step(this, 3));
        first.repaint();
        log("c1 keyPressed done");
    }
}
