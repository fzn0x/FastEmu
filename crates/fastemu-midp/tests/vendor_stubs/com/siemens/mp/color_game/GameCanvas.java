package com.siemens.mp.color_game;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Graphics;

public abstract class GameCanvas extends Canvas {
    public static final int UP_PRESSED = 2;
    public static final int DOWN_PRESSED = 64;
    public static final int LEFT_PRESSED = 4;
    public static final int RIGHT_PRESSED = 32;
    public static final int FIRE_PRESSED = 256;

    protected GameCanvas(boolean suppressKeyEvents) {}

    public void flushGraphics() {}

    public void flushGraphics(int x, int y, int width, int height) {}

    protected Graphics getGraphics() {
        return null;
    }

    public int getKeyStates() {
        return 0;
    }

    public void paint(Graphics g) {}
}
