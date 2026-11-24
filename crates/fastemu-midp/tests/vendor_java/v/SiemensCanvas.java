package v;

import com.siemens.mp.color_game.GameCanvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/**
 * A Siemens GameCanvas that fills its off-screen buffer blue, flushes it
 * in a loop, and paints a red corner while fire is held or was pressed.
 */
public class SiemensCanvas extends MIDlet {
    protected void startApp() {
        BlueCanvas canvas = new BlueCanvas();
        Display.getDisplay(this).setCurrent(canvas);
        new Thread(canvas).start();
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class BlueCanvas extends GameCanvas implements Runnable {
    BlueCanvas() {
        super(false);
    }

    public void run() {
        Graphics g = getGraphics();
        g.setColor(0x0000FF);
        g.fillRect(0, 0, getWidth(), getHeight());
        boolean told = false;
        boolean fired = false;
        while (true) {
            flushGraphics();
            if (!told && isShown()) {
                told = true;
                System.out.println("flushed " + getWidth() + "x" + getHeight());
            }
            if ((getKeyStates() & FIRE_PRESSED) != 0 && !fired) {
                fired = true;
                g.setColor(0xFF0000);
                g.fillRect(0, 0, 10, 10);
                System.out.println("fire pressed");
            }
            try {
                Thread.sleep(10);
            } catch (InterruptedException e) {
                return;
            }
        }
    }
}
