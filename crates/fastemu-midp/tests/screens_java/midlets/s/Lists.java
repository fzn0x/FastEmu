package s;

import javax.microedition.lcdui.Choice;
import javax.microedition.lcdui.Command;
import javax.microedition.lcdui.CommandListener;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Displayable;
import javax.microedition.lcdui.List;
import javax.microedition.midlet.MIDlet;

/**
 * An IMPLICIT list as a menu; the select key and the soft keys print
 * what the listener sees. The API checks print too.
 */
public class Lists extends MIDlet implements CommandListener {
    final Command exit = new Command("Exit", Command.EXIT, 1);
    final Command open = new Command("Open", Command.ITEM, 1);
    List menu;

    protected void startApp() {
        menu = new List("Menu", Choice.IMPLICIT);
        menu.append("Alpha", null);
        menu.append("Beta", null);
        menu.append("Gamma", null);
        menu.addCommand(exit);
        menu.addCommand(open);
        menu.setCommandListener(this);
        Display.getDisplay(this).setCurrent(menu);
        checks();
    }

    void checks() {
        List many = new List("Many", Choice.MULTIPLE);
        many.append("a", null);
        many.append("b", null);
        many.append("c", null);
        many.setSelectedFlags(new boolean[] {true, false, true});
        boolean[] flags = new boolean[4];
        System.out.println("flags " + many.getSelectedFlags(flags) + " " + flags[2] + " " + many.getSelectedIndex());
        List one = new List("One", Choice.EXCLUSIVE);
        one.append("x", null);
        one.append("y", null);
        one.setSelectedIndex(1, true);
        one.delete(1);
        System.out.println("exclusive " + one.getSelectedIndex() + " " + one.isSelected(0));
        try {
            one.getString(5);
        } catch (IndexOutOfBoundsException e) {
            System.out.println("index refused");
        }
        try {
            new List("Bad", 99);
        } catch (IllegalArgumentException e) {
            System.out.println("type refused");
        }
        try {
            many.getSelectedFlags(new boolean[1]);
        } catch (IllegalArgumentException e) {
            System.out.println("short array refused");
        }
    }

    public void commandAction(Command c, Displayable d) {
        int at = menu.getSelectedIndex();
        if (c == List.SELECT_COMMAND) {
            System.out.println("select " + at + " " + menu.getString(at));
        } else {
            System.out.println("command " + c.getLabel() + " " + at);
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
