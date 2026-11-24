package javax.microedition.lcdui;

public abstract class Displayable {
    Displayable() {}

    public String getTitle() {
        return null;
    }

    public void setTitle(String s) {}

    public void setTicker(Ticker ticker) {}

    public Ticker getTicker() {
        return null;
    }

    public boolean isShown() {
        return false;
    }

    public void addCommand(Command cmd) {}

    public void removeCommand(Command cmd) {}

    public void setCommandListener(CommandListener l) {}

    public int getWidth() {
        return 0;
    }

    public int getHeight() {
        return 0;
    }
}
