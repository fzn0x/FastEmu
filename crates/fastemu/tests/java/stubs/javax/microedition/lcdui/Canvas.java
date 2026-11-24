package javax.microedition.lcdui;

public abstract class Canvas extends Displayable {
    protected Canvas() {}

    public void setFullScreenMode(boolean mode) {}

    public int getWidth() {
        return 0;
    }

    public int getHeight() {
        return 0;
    }

    protected abstract void paint(Graphics g);

    protected void keyPressed(int keyCode) {}

    protected void pointerPressed(int x, int y) {}

    protected void pointerDragged(int x, int y) {}

    protected void pointerReleased(int x, int y) {}

    public final void repaint() {}

    public final void serviceRepaints() {}
}
