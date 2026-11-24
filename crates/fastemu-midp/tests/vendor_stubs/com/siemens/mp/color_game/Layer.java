package com.siemens.mp.color_game;

import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

public abstract class Layer {
    protected Layer(int width, int height) {}

    protected Layer(Image image) {}

    public final int getHeight() {
        return 0;
    }

    public final int getWidth() {
        return 0;
    }

    public final int getX() {
        return 0;
    }

    public final int getY() {
        return 0;
    }

    public final boolean isVisible() {
        return false;
    }

    public void move(int dx, int dy) {}

    public void setPosition(int x, int y) {}

    public void setVisible(boolean visible) {}

    public abstract void paint(Graphics g);
}
