package t;

import javax.microedition.lcdui.Command;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.game.GameCanvas;
import javax.microedition.midlet.MIDlet;

/**
 * A GameCanvas that is not full screen and has a command, so the soft
 * key bar takes the bottom of the screen. Its buffer still has the whole
 * screen's size; a full flush shows only the part above the bar.
 */
public class GameBars extends MIDlet {
    protected void startApp() {
        GameBarsCanvas canvas = new GameBarsCanvas();
        Display.getDisplay(this).setCurrent(canvas);
        new Thread(canvas).start();
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class GameBarsCanvas extends GameCanvas implements Runnable {
    private volatile boolean painted;

    GameBarsCanvas() {
        super(false);
        addCommand(new Command("Back", Command.BACK, 1));
    }

    public void paint(Graphics g) {
        super.paint(g);
        painted = true;
    }

    public void run() {
        while (!painted) {
            try {
                Thread.sleep(10);
            } catch (InterruptedException e) {
                return;
            }
        }
        Graphics g = getGraphics();
        g.setColor(0xFF0000);
        g.fillRect(0, 0, 1000, 1000);
        flushGraphics();
        System.out.println("bars height " + getHeight()
                + " buffer " + g.getClipWidth() + "x" + g.getClipHeight());
    }
}
