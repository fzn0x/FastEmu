package javax.microedition.lcdui;

public class Alert extends Screen {
    public static final int FOREVER = -2;
    public static final Command DISMISS_COMMAND = null;

    public Alert(String title) {}

    public Alert(String title, String alertText, Image alertImage, AlertType alertType) {}

    public int getDefaultTimeout() {
        return 0;
    }

    public int getTimeout() {
        return 0;
    }

    public void setTimeout(int time) {}

    public void setString(String str) {}

    public void setIndicator(Gauge indicator) {}
}
