package javax.microedition.lcdui;

public class Graphics {
    public static final int HCENTER = 1;
    public static final int VCENTER = 2;
    public static final int LEFT = 4;
    public static final int RIGHT = 8;
    public static final int TOP = 16;
    public static final int BOTTOM = 32;
    public static final int BASELINE = 64;

    Graphics() {}

    public void setColor(int rgb) {}

    public void setColor(int r, int g, int b) {}

    public int getColor() {
        return 0;
    }

    public void setFont(Font font) {}

    public Font getFont() {
        return null;
    }

    public void translate(int x, int y) {}

    public int getTranslateX() {
        return 0;
    }

    public void setClip(int x, int y, int w, int h) {}

    public int getClipX() {
        return 0;
    }

    public int getClipY() {
        return 0;
    }

    public int getClipWidth() {
        return 0;
    }

    public int getClipHeight() {
        return 0;
    }

    public void drawLine(int x1, int y1, int x2, int y2) {}

    public void fillRect(int x, int y, int w, int h) {}

    public void drawRect(int x, int y, int w, int h) {}

    public void fillArc(int x, int y, int w, int h, int start, int arc) {}

    public void fillTriangle(int x1, int y1, int x2, int y2, int x3, int y3) {}

    public void drawString(String s, int x, int y, int anchor) {}

    public void drawImage(Image img, int x, int y, int anchor) {}

    public void drawRegion(Image src, int xs, int ys, int w, int h, int t, int xd, int yd, int a) {}

    public void drawRGB(int[] rgb, int offset, int scan, int x, int y, int w, int h, boolean alpha) {}
}
