package s;

import javax.microedition.lcdui.TextBox;
import javax.microedition.lcdui.TextField;
import javax.microedition.midlet.MIDlet;

/**
 * A text box with a maximum size of Integer.MAX_VALUE that takes one
 * string again and again: the text grows natively with no Java object
 * made, and the heap has to count it. Each case is a method of its own
 * that returns how far it got, so that nothing it kept is alive when the
 * result is printed.
 */
public class MemInsert extends MIDlet {
    static final int CALLS = 1000;

    static String chunk() {
        char[] chars = new char[1000];
        for (int i = 0; i < chars.length; i++) {
            chars[i] = 'b';
        }
        return new String(chars);
    }

    protected void startApp() {
        String chunk = chunk();
        System.out.println("max size: " + new TextBox("t", "", 1, 0).setMaxSize(Integer.MAX_VALUE));
        System.out.println("box: " + result(insertIntoBox(chunk)));
        System.out.println("field: " + result(insertIntoField(chunk)));
        System.out.println("steady: " + steady(chunk));
        System.out.println("inserts done");
    }

    static String result(int calls) {
        return calls < CALLS ? "oom after " + calls : "no oom after " + calls;
    }

    static int insertIntoBox(String chunk) {
        TextBox box = new TextBox("t", "", 1, TextField.ANY);
        box.setMaxSize(Integer.MAX_VALUE);
        int calls = 0;
        try {
            for (; calls < CALLS; calls++) {
                box.insert(chunk, box.size());
            }
        } catch (OutOfMemoryError e) {
            box = null;
        }
        return calls;
    }

    static int insertIntoField(String chunk) {
        TextField field = new TextField("t", "", 1, TextField.ANY);
        field.setMaxSize(Integer.MAX_VALUE);
        int calls = 0;
        try {
            for (; calls < CALLS; calls++) {
                field.insert(chunk, field.size());
            }
        } catch (OutOfMemoryError e) {
            field = null;
        }
        return calls;
    }

    /** Text that is put in and taken out again stays small. */
    static String steady(String chunk) {
        TextBox box = new TextBox("t", "", 1, TextField.ANY);
        box.setMaxSize(Integer.MAX_VALUE);
        int calls = 0;
        try {
            for (; calls < 400; calls++) {
                box.insert(chunk, 0);
                box.delete(0, chunk.length());
            }
        } catch (OutOfMemoryError e) {
            box = null;
            return "oom after " + calls;
        }
        return "ok " + box.size();
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
