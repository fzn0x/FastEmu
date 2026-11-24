package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Command;
import javax.microedition.lcdui.CommandListener;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Displayable;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/**
 * A canvas with commands. The suite's MIDlet-1 gives it one command,
 * MIDlet-2 three, MIDlet-3 one in full-screen mode; the variant comes from
 * the subclass that is started.
 */
public class Commands extends MIDlet implements CommandListener {
    final Command exit = new Command("Exit", Command.EXIT, 1);
    final Command alpha = new Command("Alpha", Command.SCREEN, 1);
    final Command beta = new Command("Beta", Command.SCREEN, 2);

    int variant() {
        return 1;
    }

    protected void startApp() {
        CommandCanvas canvas = new CommandCanvas();
        int variant = variant();
        if (variant == 3) {
            canvas.setFullScreenMode(true);
            canvas.addCommand(alpha);
        } else {
            canvas.addCommand(exit);
        }
        if (variant == 2) {
            canvas.addCommand(alpha);
            canvas.addCommand(beta);
        }
        canvas.setCommandListener(this);
        Display.getDisplay(this).setCurrent(canvas);
    }

    public void commandAction(Command c, Displayable d) {
        System.out.println("command " + c.getLabel());
        if (c == exit) {
            notifyDestroyed();
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class CommandCanvas extends Canvas {
    protected void paint(Graphics g) {
        g.setColor(0x00FF00);
        g.fillRect(0, 0, getWidth(), getHeight());
        System.out.println("size " + getWidth() + "x" + getHeight());
    }

    protected void sizeChanged(int w, int h) {
        System.out.println("sizeChanged " + w + "x" + h);
    }

    protected void keyPressed(int code) {
        System.out.println("pressed " + code);
    }
}
