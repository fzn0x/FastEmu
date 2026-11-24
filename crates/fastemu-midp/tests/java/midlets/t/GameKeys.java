package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.game.GameCanvas;
import javax.microedition.midlet.MIDlet;

/**
 * A GameCanvas that suppresses game key events. A game thread polls
 * getKeyStates every 100 ms and logs each change; 0 switches to a plain
 * canvas and back.
 */
public class GameKeys extends MIDlet {
    protected void startApp() {
        GameKeyCanvas.start(this, true);
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class GameKeyCanvas extends GameCanvas implements Runnable {
    private final Display display;
    private final Canvas other;
    private int last = -1;
    private boolean lastShown;

    static void start(MIDlet midlet, boolean suppress) {
        Display display = Display.getDisplay(midlet);
        GameKeyCanvas canvas = new GameKeyCanvas(display, suppress);
        display.setCurrent(canvas);
        new Thread(canvas).start();
    }

    GameKeyCanvas(Display display, boolean suppress) {
        super(suppress);
        this.display = display;
        setFullScreenMode(true);
        other = new GamePlainCanvas(display, this);
    }

    public void run() {
        while (true) {
            int states = getKeyStates();
            boolean shown = isShown();
            if (states != last || shown != lastShown) {
                System.out.println("states " + states + " shown " + shown);
                last = states;
                lastShown = shown;
            }
            try {
                Thread.sleep(100);
            } catch (InterruptedException e) {
                return;
            }
        }
    }

    protected void keyPressed(int code) {
        System.out.println("pressed " + code);
        if (code == 48) {
            display.setCurrent(other);
        }
    }

    protected void keyRepeated(int code) {
        System.out.println("repeated " + code);
    }

    protected void keyReleased(int code) {
        System.out.println("released " + code);
    }
}

/** A plain canvas; 0 goes back to the game canvas. */
class GamePlainCanvas extends Canvas {
    private final Display display;
    private final Canvas back;

    GamePlainCanvas(Display display, Canvas back) {
        this.display = display;
        this.back = back;
        setFullScreenMode(true);
    }

    protected void paint(Graphics g) {
        g.setColor(0);
        g.fillRect(0, 0, getWidth(), getHeight());
    }

    protected void keyPressed(int code) {
        System.out.println("elsewhere pressed " + code);
        if (code == 48) {
            display.setCurrent(back);
        }
    }
}
