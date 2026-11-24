package com.siemens.mp.color_game;

import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

public class Sprite extends Layer {
    public Sprite(Image image) {
        super(image);
    }

    public Sprite(Image image, int frameWidth, int frameHeight) {
        super(image);
    }

    public Sprite(Sprite s) {
        super(0, 0);
    }

    public boolean collidesWith(Sprite s, boolean pixelLevel) {
        return false;
    }

    public boolean collidesWith(Image image, int x, int y, boolean pixelLevel) {
        return false;
    }

    public final int getFrame() {
        return 0;
    }

    public int getFrameSequenceLength() {
        return 0;
    }

    public int getRawFrameCount() {
        return 0;
    }

    public void nextFrame() {}

    public final void paint(Graphics g) {}

    public void prevFrame() {}

    public void setCollisionRectangle(int x, int y, int width, int height) {}

    public void setFrame(int index) {}

    public void setFrameSequence(int[] sequence) {}

    public void setImage(Image image, int frameWidth, int frameHeight) {}
}
