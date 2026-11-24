package javax.microedition.lcdui;

public abstract class CustomItem extends Item {
    protected CustomItem(String label) {}

    protected abstract int getMinContentWidth();

    protected abstract int getMinContentHeight();

    protected abstract int getPrefContentWidth(int height);

    protected abstract int getPrefContentHeight(int width);

    protected abstract void paint(Graphics g, int w, int h);

    protected void sizeChanged(int w, int h) {}

    protected boolean traverse(int dir, int viewportWidth, int viewportHeight, int[] visRect_inout) {
        return false;
    }

    protected void traverseOut() {}

    protected void keyPressed(int keyCode) {}

    protected void showNotify() {}

    protected void hideNotify() {}

    protected final void repaint() {}

    protected final void invalidate() {}
}
