package s;

import javax.microedition.lcdui.Form;
import javax.microedition.lcdui.TextBox;
import javax.microedition.lcdui.TextField;
import javax.microedition.midlet.MIDlet;

/**
 * Text boxes and fields made from one large string. Each copies its text
 * natively and makes almost no Java object, so the heap has to count the
 * copies: kept ones end in OutOfMemoryError, dropped ones are collected.
 * Each case is a method of its own that returns how far it got, so that
 * nothing it kept is alive when the result is printed.
 */
public class MemTexts extends MIDlet {
    static final int CHARS = 20000;
    static final int KEPT = 50;

    static String big() {
        char[] chars = new char[CHARS];
        for (int i = 0; i < CHARS; i++) {
            chars[i] = 'a';
        }
        return new String(chars);
    }

    protected void startApp() {
        String big = big();
        System.out.println("dropped boxes: " + dropped(big));
        System.out.println("boxes: " + result(keepBoxes(big)));
        System.out.println("fields: " + result(keepFields(big)));
        System.out.println("set again: " + setAgain(big));
        System.out.println("texts done");
    }

    static String result(int kept) {
        return kept < KEPT ? "oom after " + kept : "no oom after " + kept;
    }

    /** The same boxes, each dropped, are collected as they go. */
    static String dropped(String big) {
        int made = 0;
        try {
            for (; made < 500; made++) {
                new TextBox("t", big, CHARS, 0);
            }
        } catch (OutOfMemoryError e) {
            return "oom after " + made;
        }
        return "ok " + made;
    }

    static int keepBoxes(String big) {
        Object[] kept = new Object[KEPT];
        int made = 0;
        try {
            for (; made < KEPT; made++) {
                kept[made] = new TextBox("t", big, CHARS, 0);
            }
        } catch (OutOfMemoryError e) {
            kept = null;
        }
        return made;
    }

    static int keepFields(String big) {
        Object[] kept = new Object[KEPT];
        Form form = new Form("f");
        int made = 0;
        try {
            for (; made < KEPT; made++) {
                TextField field = new TextField("t", big, CHARS, 0);
                form.append(field);
                kept[made] = field;
            }
        } catch (OutOfMemoryError e) {
            kept = null;
            form = null;
        }
        return made;
    }

    /** The text of a box that is set again replaces its old text. */
    static String setAgain(String big) {
        TextBox box = new TextBox("t", "", CHARS, 0);
        int sets = 0;
        try {
            for (; sets < 200; sets++) {
                box.setString(big);
            }
        } catch (OutOfMemoryError e) {
            box = null;
            return "oom after " + sets;
        }
        return "ok " + box.size();
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
