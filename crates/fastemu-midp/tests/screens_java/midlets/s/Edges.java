package s;

import javax.microedition.lcdui.Choice;
import javax.microedition.lcdui.ChoiceGroup;
import javax.microedition.lcdui.Command;
import javax.microedition.lcdui.CommandListener;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Displayable;
import javax.microedition.lcdui.Form;
import javax.microedition.lcdui.Gauge;
import javax.microedition.lcdui.Item;
import javax.microedition.lcdui.ItemStateListener;
import javax.microedition.midlet.MIDlet;

/**
 * What a game may do to a form that the user is in the middle of, and
 * the extreme values it may give: its command empties the popup choice
 * group whose list is open, and a gauge runs up to Integer.MAX_VALUE.
 */
public class Edges extends MIDlet implements CommandListener, ItemStateListener {
    ChoiceGroup popup;
    Gauge big;
    int emptied;

    protected void startApp() {
        Form form = new Form("Edges");
        popup = new ChoiceGroup("Pick", Choice.POPUP, new String[] {"A", "B"}, null);
        form.append(popup);
        big = new Gauge("Big", true, Integer.MAX_VALUE, Integer.MAX_VALUE);
        form.append(big);
        form.addCommand(new Command("Empty", Command.SCREEN, 1));
        form.setCommandListener(this);
        form.setItemStateListener(this);
        Display.getDisplay(this).setCurrent(form);
        System.out.println("edges shown");
    }

    public void commandAction(Command c, Displayable d) {
        popup.deleteAll();
        emptied++;
        System.out.println("emptied " + emptied);
    }

    public void itemStateChanged(Item item) {
        if (item == big) {
            System.out.println("gauge " + big.getValue());
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
