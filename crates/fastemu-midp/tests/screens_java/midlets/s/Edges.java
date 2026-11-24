package s;

import javax.microedition.lcdui.Choice;
import javax.microedition.lcdui.ChoiceGroup;
import javax.microedition.lcdui.Command;
import javax.microedition.lcdui.CommandListener;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Displayable;
import javax.microedition.lcdui.Form;
import javax.microedition.midlet.MIDlet;

/**
 * What a game may do to a form that the user is in the middle of: its
 * command empties the popup choice group whose list is open.
 */
public class Edges extends MIDlet implements CommandListener {
    ChoiceGroup popup;
    int emptied;

    protected void startApp() {
        Form form = new Form("Edges");
        popup = new ChoiceGroup("Pick", Choice.POPUP, new String[] {"A", "B"}, null);
        form.append(popup);
        form.addCommand(new Command("Empty", Command.SCREEN, 1));
        form.setCommandListener(this);
        Display.getDisplay(this).setCurrent(form);
        System.out.println("edges shown");
    }

    public void commandAction(Command c, Displayable d) {
        popup.deleteAll();
        emptied++;
        System.out.println("emptied " + emptied);
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
