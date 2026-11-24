package s;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Command;
import javax.microedition.lcdui.CommandListener;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Displayable;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.TextBox;
import javax.microedition.lcdui.TextField;
import javax.microedition.lcdui.Ticker;
import javax.microedition.midlet.MIDlet;

/**
 * A text box for a name, under a ticker: OK prints what was typed and
 * moves on to a canvas. The API checks print too.
 */
public class Texts extends MIDlet implements CommandListener {
    final Command ok = new Command("OK", Command.OK, 1);
    TextBox box;

    protected void startApp() {
        box = new TextBox("Name", "", 8, TextField.ANY);
        box.addCommand(ok);
        box.setCommandListener(this);
        box.setTicker(new Ticker("Type a name for the high score table"));
        Display.getDisplay(this).setCurrent(box);
        checks();
    }

    void checks() {
        try {
            new TextBox("Long", "abc", 2, TextField.ANY);
        } catch (IllegalArgumentException e) {
            System.out.println("too long refused");
        }
        TextBox number = new TextBox("Number", "12", 5, TextField.NUMERIC);
        try {
            number.setString("x");
        } catch (IllegalArgumentException e) {
            System.out.println("numeric refused");
        }
        number.insert("3", 0);
        char[] chars = new char[5];
        System.out.println("number " + number.getString() + " " + number.getChars(chars) + " " + chars[0]);
        try {
            number.delete(2, 9);
        } catch (StringIndexOutOfBoundsException e) {
            System.out.println("range refused");
        }
        System.out.println("max " + number.setMaxSize(2) + " " + number.getString());
        number.setConstraints(TextField.ANY | TextField.PASSWORD);
        System.out.println("kept " + number.getString());
    }

    public void commandAction(Command c, Displayable d) {
        System.out.println("text [" + box.getString() + "] caret " + box.getCaretPosition());
        Display.getDisplay(this).setCurrent(new Plain());
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class Plain extends Canvas {
    protected void paint(Graphics g) {
        g.setColor(0x00FF00);
        g.fillRect(0, 0, getWidth(), getHeight());
    }

    protected void showNotify() {
        System.out.println("canvas shown");
    }
}
