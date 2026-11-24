package t;

import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.game.GameCanvas;
import javax.microedition.midlet.MIDlet;

/**
 * Flushes a GameCanvas's buffer in stages, each after a press of fire:
 * a region only, a repaint without a flush, then thirty full flushes
 * timed against the frame rate.
 */
public class GameFlush extends MIDlet {
    protected void startApp() {
        GameFlushCanvas canvas = new GameFlushCanvas();
        Display.getDisplay(this).setCurrent(canvas);
        new Thread(canvas).start();
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class GameFlushCanvas extends GameCanvas implements Runnable {
    private volatile boolean painted;

    GameFlushCanvas() {
        super(true);
        setFullScreenMode(true);
    }

    private static void sleep(int ms) {
        try {
            Thread.sleep(ms);
        } catch (InterruptedException e) {
            throw new RuntimeException("interrupted");
        }
    }

    private void waitForFire() {
        while ((getKeyStates() & FIRE_PRESSED) == 0) {
            sleep(20);
        }
        while (getKeyStates() != 0) {
            sleep(20);
        }
    }

    public void paint(Graphics g) {
        super.paint(g);
        painted = true;
    }

    public void run() {
        // The first paint draws the white buffer; draw only after it.
        while (!painted) {
            sleep(10);
        }
        Graphics g = getGraphics();
        Graphics again = getGraphics();
        System.out.println("graphics new " + (g != again)
                + " clip " + g.getClipX() + " " + g.getClipY()
                + " " + g.getClipWidth() + " " + g.getClipHeight()
                + " color " + g.getColor());
        g.setColor(0xFF0000);
        g.fillRect(0, 0, getWidth(), getHeight());
        flushGraphics(10, 10, 20, 20);
        flushGraphics(0, 0, 0, 100);
        System.out.println("stage region");
        waitForFire();
        repaint();
        System.out.println("stage repaint");
        waitForFire();
        g.setColor(0x00FF00);
        g.fillRect(0, 0, getWidth(), getHeight());
        long start = System.currentTimeMillis();
        for (int i = 0; i < 30; i++) {
            flushGraphics();
        }
        long took = System.currentTimeMillis() - start;
        System.out.println("stage paced " + took);
    }
}
