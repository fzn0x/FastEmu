package com.siemens.mp.ui;

public class Image extends com.siemens.mp.misc.NativeMem {
    public static final int COLOR_BMP_8BIT = 5;

    public Image(int width, int height) {}

    public Image(byte[] bytes, int width, int height, boolean transparent) {}

    public Image(byte[] imageData) {}

    public int getWidth() {
        return 0;
    }

    public int getHeight() {
        return 0;
    }

    public static javax.microedition.lcdui.Image createImageFromBitmap(byte[] data, int w, int h) {
        return null;
    }

    public static javax.microedition.lcdui.Image createImageWithoutScaling(String name)
            throws java.io.IOException {
        return null;
    }

    public static javax.microedition.lcdui.Image createRGBImage(byte[] data, int w, int h,
            int type) throws java.io.IOException {
        return null;
    }

    public static javax.microedition.lcdui.Image createTransparentImageFromBitmap(byte[] data,
            int w, int h) {
        return null;
    }

    public static void mirrorImageHorizontally(javax.microedition.lcdui.Image image) {}

    public static void mirrorImageVertically(javax.microedition.lcdui.Image image) {}

    public static Image getNativeImage(javax.microedition.lcdui.Image image) {
        return null;
    }
}
