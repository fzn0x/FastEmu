package com.siemens.mp.game;

import javax.microedition.lcdui.Image;

public class Sprite extends GraphicObject {
    public Sprite(Image pixels, Image mask, int numFrames) {}

    public Sprite(ExtendedImage pixels, ExtendedImage mask, int numFrames) {}

    public Sprite(byte[] pixels, int pixelOffset, int width, int height, byte[] mask,
            int maskOffset, int numFrames) {}

    public void setPosition(int x, int y) {}

    public int getXPosition() {
        return 0;
    }

    public int getYPosition() {
        return 0;
    }

    public void setCollisionRectangle(int x, int y, int width, int height) {}

    public boolean isCollidingWith(Sprite other) {
        return false;
    }

    public boolean isCollidingWithPos(int x, int y) {
        return false;
    }

    public void setFrame(int frame) {}

    public int getFrame() {
        return 0;
    }
}
