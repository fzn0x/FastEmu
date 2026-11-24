package javax.microedition.lcdui;

public class AlertType {
    public static final AlertType INFO = null;
    public static final AlertType ERROR = null;

    protected AlertType() {}

    public boolean playSound(Display display) {
        return false;
    }
}
