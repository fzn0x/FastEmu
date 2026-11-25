package m;

import javax.microedition.lcdui.Alert;
import javax.microedition.lcdui.Choice;
import javax.microedition.lcdui.Command;
import javax.microedition.lcdui.CommandListener;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Displayable;
import javax.microedition.lcdui.List;
import javax.microedition.midlet.MIDlet;

/**
 * A menu, then an alert over it, then the same menu again: a screen that
 * is no longer shown gives its buffer back, and must paint all of its
 * content when it is shown again.
 */
public class Returns extends MIDlet implements CommandListener {
    Display display;
    List menu;
    Alert ask;

    protected void startApp() {
        display = Display.getDisplay(this);
        menu = new List("Menu", Choice.IMPLICIT);
        menu.append("Alpha", null);
        menu.append("Beta", null);
        menu.append("Gamma", null);
        menu.setCommandListener(this);
        ask = new Alert("Ask");
        ask.setString("Sure?");
        ask.setTimeout(Alert.FOREVER);
        ask.setCommandListener(this);
        display.setCurrent(menu);
        System.out.println("menu shown");
    }

    public void commandAction(Command c, Displayable d) {
        if (d == menu) {
            System.out.println("selected");
            display.setCurrent(ask);
        } else {
            System.out.println("dismissed");
            display.setCurrent(menu);
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
