package javax.microedition.lcdui;

public abstract class Item {
    public static final int LAYOUT_2 = 16384;
    public static final int BUTTON = 2;

    Item() {}

    public String getLabel() {
        return null;
    }

    public void setLabel(String label) {}

    public void addCommand(Command cmd) {}

    public void setDefaultCommand(Command cmd) {}

    public void setItemCommandListener(ItemCommandListener l) {}

    public void setLayout(int layout) {}

    public int getPreferredWidth() {
        return 0;
    }

    public void notifyStateChanged() {}
}
