package javax.microedition.lcdui;

public class TextField extends Item {
    public static final int ANY = 0;
    public static final int NUMERIC = 2;
    public static final int PASSWORD = 65536;
    public static final int UNEDITABLE = 131072;

    public TextField(String label, String text, int maxSize, int constraints) {}

    public String getString() {
        return null;
    }

    public void setString(String text) {}

    public int size() {
        return 0;
    }
}
