package s;

import javax.microedition.lcdui.CustomItem;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Form;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.TextField;
import javax.microedition.midlet.MIDlet;

/**
 * A form with a game-drawn item above a text field, given the focus
 * from startApp, off the event thread. The item adds one more item to
 * the form while the form measures it.
 */
public class Customs extends MIDlet {
    protected void startApp() {
        Form form = new Form("Custom");
        Box box = new Box(form);
        form.append(box);
        form.append(new TextField("Below", "", 5, TextField.ANY));
        Display.getDisplay(this).setCurrent(form);
        Display.getDisplay(this).setCurrentItem(box);
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class Box extends CustomItem {
    final Form form;
    boolean grown;

    Box(Form form) {
        super(null);
        this.form = form;
    }

    protected int getMinContentWidth() {
        return 40;
    }

    protected int getMinContentHeight() {
        return 20;
    }

    protected int getPrefContentWidth(int height) {
        return 60;
    }

    protected int getPrefContentHeight(int width) {
        if (!grown) {
            grown = true;
            form.append(new Late());
        }
        return 30;
    }

    protected void paint(Graphics g, int w, int h) {
        g.setColor(0xFF0000);
        g.fillRect(0, 0, w, h);
        System.out.println("paint " + w + "x" + h);
    }

    protected void sizeChanged(int w, int h) {
        System.out.println("size " + w + "x" + h);
    }

    protected boolean traverse(int dir, int viewportWidth, int viewportHeight, int[] visRect) {
        System.out.println("traverse " + dir + " visible " + visRect[2] + "x" + visRect[3]
                + " on " + Thread.currentThread().getName());
        return false;
    }

    protected void traverseOut() {
        System.out.println("out");
    }

    protected void keyPressed(int keyCode) {
        System.out.println("key " + keyCode);
    }

    protected void showNotify() {
        System.out.println("show");
    }
}

/** The item Box adds; it says when it is first painted. */
class Late extends CustomItem {
    boolean painted;

    Late() {
        super(null);
    }

    protected int getMinContentWidth() {
        return 10;
    }

    protected int getMinContentHeight() {
        return 10;
    }

    protected int getPrefContentWidth(int height) {
        return 10;
    }

    protected int getPrefContentHeight(int width) {
        return 10;
    }

    protected void paint(Graphics g, int w, int h) {
        if (!painted) {
            painted = true;
            System.out.println("late paint");
        }
    }
}
