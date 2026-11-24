package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/** Logs every key and pointer callback; its first key press throws. */
public class Keys extends MIDlet {
    protected void startApp() {
        Display.getDisplay(this).setCurrent(new KeysCanvas());
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class KeysCanvas extends Canvas {
    private int presses;

    KeysCanvas() {
        setFullScreenMode(true);
    }

    protected void paint(Graphics g) {
        g.setColor(0);
        g.fillRect(0, 0, getWidth(), getHeight());
    }

    protected void showNotify() {
        System.out.println("shown pointer " + hasPointerEvents());
    }

    protected void keyPressed(int code) {
        presses++;
        if (presses == 1) {
            Object nothing = null;
            nothing.hashCode();
        }
        System.out.println("pressed " + code + " action " + getGameAction(code)
                + " name " + getKeyName(code));
    }

    protected void keyRepeated(int code) {
        System.out.println("repeated " + code);
    }

    protected void keyReleased(int code) {
        System.out.println("released " + code);
    }

    protected void pointerPressed(int x, int y) {
        System.out.println("pointerPressed " + x + " " + y);
    }

    protected void pointerDragged(int x, int y) {
        System.out.println("pointerDragged " + x + " " + y);
    }

    protected void pointerReleased(int x, int y) {
        System.out.println("pointerReleased " + x + " " + y);
    }
}
