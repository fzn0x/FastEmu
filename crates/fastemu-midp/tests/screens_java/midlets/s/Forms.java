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
import javax.microedition.lcdui.ItemCommandListener;
import javax.microedition.lcdui.ItemStateListener;
import javax.microedition.lcdui.StringItem;
import javax.microedition.lcdui.TextField;
import javax.microedition.midlet.MIDlet;

/**
 * A settings form: a text field, a choice group, an interactive gauge
 * and a button. Every change the user makes is printed by the
 * ItemStateListener, and the button's command by its listener.
 */
public class Forms extends MIDlet implements ItemStateListener, ItemCommandListener, CommandListener {
    final Command done = new Command("Done", Command.OK, 1);
    final Command go = new Command("Go", Command.ITEM, 1);
    TextField name;
    ChoiceGroup color;
    Gauge level;
    Form form;

    protected void startApp() {
        form = new Form("Settings");
        form.append(new StringItem("Hello", "Pick below"));
        name = new TextField("Name", "", 10, TextField.ANY);
        form.append(name);
        color = new ChoiceGroup("Color", Choice.EXCLUSIVE, new String[] {"Red", "Green", "Blue"}, null);
        form.append(color);
        level = new Gauge("Level", true, 10, 5);
        form.append(level);
        StringItem button = new StringItem(null, "Go", Item.BUTTON);
        button.setDefaultCommand(go);
        button.setItemCommandListener(this);
        form.append(button);
        form.setItemStateListener(this);
        form.addCommand(done);
        form.setCommandListener(this);
        try {
            form.append(name);
        } catch (IllegalStateException e) {
            System.out.println("owned refused");
        }
        Display.getDisplay(this).setCurrent(form);
        System.out.println("form " + form.size());
    }

    public void itemStateChanged(Item item) {
        String value;
        if (item == name) {
            value = name.getString();
        } else if (item == color) {
            value = "" + color.getSelectedIndex();
        } else {
            value = "" + level.getValue();
        }
        System.out.println("changed " + item.getLabel() + " " + value);
    }

    public void commandAction(Command c, Item item) {
        System.out.println("item command " + c.getLabel());
        Display.getDisplay(this).setCurrentItem(name);
    }

    public void commandAction(Command c, Displayable d) {
        // Done locks the name, which then takes no more typing.
        name.setConstraints(TextField.ANY | TextField.UNEDITABLE);
        System.out.println("form command " + c.getLabel());
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
