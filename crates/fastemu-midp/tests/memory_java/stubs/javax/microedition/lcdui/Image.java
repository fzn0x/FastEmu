// A compile-time stand-in that adds createImage(InputStream) to the shared
// stub; it comes first on the source path. The tests' JARs never contain it.
package javax.microedition.lcdui;

public class Image {
    Image() {}

    public static Image createImage(int w, int h) {
        return null;
    }

    public static Image createImage(String name) throws java.io.IOException {
        return null;
    }

    public static Image createImage(java.io.InputStream stream) throws java.io.IOException {
        return null;
    }

    public static Image createImage(byte[] data, int offset, int length) {
        return null;
    }

    public static Image createImage(Image source) {
        return null;
    }

    public static Image createRGBImage(int[] rgb, int w, int h, boolean alpha) {
        return null;
    }

    public Graphics getGraphics() {
        return null;
    }

    public int getWidth() {
        return 0;
    }

    public int getHeight() {
        return 0;
    }

    public boolean isMutable() {
        return false;
    }
}
