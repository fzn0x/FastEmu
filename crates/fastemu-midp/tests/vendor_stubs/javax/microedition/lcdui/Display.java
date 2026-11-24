// Compile-time stand-ins for the runtime's lcdui classes. Constant values
// follow docs/compat/api/midp-lcdui.md, since javac inlines them.
package javax.microedition.lcdui;

import javax.microedition.midlet.MIDlet;

public class Display {
    public static final int COLOR_BACKGROUND = 0;
    public static final int COLOR_FOREGROUND = 1;

    public static Display getDisplay(MIDlet m) {
        return null;
    }

    public void setCurrent(Displayable next) {}

    public Displayable getCurrent() {
        return null;
    }

    public void callSerially(Runnable r) {}

    public boolean vibrate(int duration) {
        return false;
    }

    public boolean isColor() {
        return false;
    }

    public int numColors() {
        return 0;
    }

    public int getColor(int specifier) {
        return 0;
    }
}
