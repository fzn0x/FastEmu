package javax.microedition.lcdui;

public class Gauge extends Item {
    public static final int INDEFINITE = -1;
    public static final int CONTINUOUS_IDLE = 0;
    public static final int CONTINUOUS_RUNNING = 2;

    public Gauge(String label, boolean interactive, int maxValue, int initialValue) {}

    public int getValue() {
        return 0;
    }

    public void setValue(int value) {}
}
