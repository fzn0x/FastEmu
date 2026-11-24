// Compile-time stand-ins for the runtime's LG classes.
package mmpp.media;

public final class BackLight {
    public static void on(int timeout) {}

    public static void off() {}

    public static int numColors() {
        return 0;
    }

    public static void setColor(int rgb) {}

    public static int getColor() {
        return 0;
    }
}
