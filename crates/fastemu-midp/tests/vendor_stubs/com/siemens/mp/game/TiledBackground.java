package com.siemens.mp.game;

import javax.microedition.lcdui.Image;

public class TiledBackground extends GraphicObject {
    public TiledBackground(Image tilePixels, Image tileMask, byte[] map, int w, int h) {}

    public TiledBackground(ExtendedImage tilePixels, ExtendedImage tileMask, byte[] map, int w,
            int h) {}

    public TiledBackground(byte[] tilePixels, byte[] tileMask, byte[] map, int w, int h) {}

    public void setPositionInMap(int x, int y) {}
}
