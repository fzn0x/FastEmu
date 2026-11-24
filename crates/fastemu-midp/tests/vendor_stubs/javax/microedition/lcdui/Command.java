package javax.microedition.lcdui;

public class Command {
    public static final int SCREEN = 1;
    public static final int BACK = 2;
    public static final int OK = 4;
    public static final int EXIT = 7;

    public Command(String label, int type, int priority) {}

    public String getLabel() {
        return null;
    }

    public int getCommandType() {
        return 0;
    }
}
