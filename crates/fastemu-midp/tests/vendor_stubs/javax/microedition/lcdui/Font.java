package javax.microedition.lcdui;

public final class Font {
    public static final int STYLE_PLAIN = 0;
    public static final int STYLE_BOLD = 1;
    public static final int SIZE_SMALL = 8;
    public static final int SIZE_MEDIUM = 0;
    public static final int SIZE_LARGE = 16;
    public static final int FACE_SYSTEM = 0;

    Font() {}

    public static Font getDefaultFont() {
        return null;
    }

    public static Font getFont(int face, int style, int size) {
        return null;
    }

    public int getHeight() {
        return 0;
    }

    public int stringWidth(String s) {
        return 0;
    }
}
