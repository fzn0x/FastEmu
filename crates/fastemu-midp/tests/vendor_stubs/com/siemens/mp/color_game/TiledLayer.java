package com.siemens.mp.color_game;

import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

public class TiledLayer extends Layer {
    public TiledLayer(int columns, int rows, Image image, int tileWidth, int tileHeight) {
        super(image);
    }

    public int createAnimatedTile(int staticTileIndex) {
        return 0;
    }

    public void fillCells(int col, int row, int width, int height, int tileIndex) {}

    public int getAnimatedTile(int animatedTileIndex) {
        return 0;
    }

    public int getCell(int col, int row) {
        return 0;
    }

    public final int getCellHeight() {
        return 0;
    }

    public final int getCellWidth() {
        return 0;
    }

    public final int getColumns() {
        return 0;
    }

    public final int getRows() {
        return 0;
    }

    public final void paint(Graphics g) {}

    public void setAnimatedTile(int animatedTileIndex, int staticTileIndex) {}

    public void setCell(int col, int row, int tileIndex) {}

    public void setStaticTileSet(Image image, int tileWidth, int tileHeight) {}
}
