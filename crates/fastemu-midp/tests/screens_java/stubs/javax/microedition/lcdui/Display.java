// Compile-time stand-ins for the runtime's lcdui classes, with only what
// the screen tests call. Constant values follow
// docs/compat/api/midp-lcdui.md, since javac inlines them.
package javax.microedition.lcdui;

import javax.microedition.midlet.MIDlet;

public class Display {
    public static Display getDisplay(MIDlet m) {
        return null;
    }

    public void setCurrent(Displayable next) {}

    public void setCurrent(Alert alert, Displayable next) {}

    public void setCurrentItem(Item item) {}

    public Displayable getCurrent() {
        return null;
    }
}
