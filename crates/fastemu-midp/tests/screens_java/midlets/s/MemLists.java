package s;

import javax.microedition.lcdui.Choice;
import javax.microedition.lcdui.ChoiceGroup;
import javax.microedition.lcdui.Form;
import javax.microedition.lcdui.List;
import javax.microedition.midlet.MIDlet;

/**
 * Elements added without end. A list and a choice group keep each element
 * natively and make no Java object for it, so the heap has to count them:
 * a game that does this must get OutOfMemoryError, and one that clears
 * its list must get its room back. Each case is a method of its own that
 * returns how far it got, so that nothing it kept is alive when the
 * result is printed.
 */
public class MemLists extends MIDlet {
    static final int CALLS = 100000;

    protected void startApp() {
        System.out.println("list: " + result(appendTo(new List("t", Choice.MULTIPLE))));
        System.out.println("group: " + result(appendToGroup()));
        System.out.println("reuse: " + reuse());
        System.out.println("dropped: " + dropped());
        System.out.println("lists done");
    }

    static String result(int calls) {
        return calls < CALLS ? "oom after " + calls : "no oom after " + calls;
    }

    /** Appends one string again and again; how many calls it took. */
    static int appendTo(Choice choice) {
        String s = "x";
        int calls = 0;
        try {
            for (; calls < CALLS; calls++) {
                choice.append(s, null);
            }
        } catch (OutOfMemoryError e) {
            choice = null;
        }
        return calls;
    }

    static int appendToGroup() {
        ChoiceGroup group = new ChoiceGroup("g", Choice.MULTIPLE);
        new Form("f").append(group);
        return appendTo(group);
    }

    /** A list that is emptied each round never runs out. */
    static String reuse() {
        List list = new List("t", Choice.IMPLICIT);
        String s = "x";
        int round = 0;
        try {
            for (; round < 20; round++) {
                for (int i = 0; i < 20000; i++) {
                    list.append(s, null);
                }
                list.deleteAll();
            }
        } catch (OutOfMemoryError e) {
            list = null;
            return "oom in round " + round;
        }
        return "ok " + list.size();
    }

    /** Lists nobody keeps are collected with what they hold. */
    static String dropped() {
        String s = "x";
        int made = 0;
        try {
            for (; made < 200; made++) {
                List dropped = new List("t", Choice.IMPLICIT);
                for (int i = 0; i < 2000; i++) {
                    dropped.append(s, null);
                }
            }
        } catch (OutOfMemoryError e) {
            return "oom after " + made;
        }
        return "ok " + made;
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
