package javax.microedition.lcdui;

public abstract class Canvas extends Displayable {
    public static final int UP = 1;
    public static final int DOWN = 6;
    public static final int LEFT = 2;
    public static final int RIGHT = 5;
    public static final int FIRE = 8;
    public static final int GAME_A = 9;
    public static final int KEY_NUM1 = 49;
    public static final int KEY_NUM5 = 53;

    protected Canvas() {}

    public boolean isDoubleBuffered() {
        return false;
    }

    public boolean hasPointerEvents() {
        return false;
    }

    public boolean hasRepeatEvents() {
        return false;
    }

    public int getKeyCode(int gameAction) {
        return 0;
    }

    public String getKeyName(int keyCode) {
        return null;
    }

    public int getGameAction(int keyCode) {
        return 0;
    }

    public void setFullScreenMode(boolean mode) {}

    protected void keyPressed(int keyCode) {}

    protected void keyRepeated(int keyCode) {}

    protected void keyReleased(int keyCode) {}

    protected void pointerPressed(int x, int y) {}

    protected void pointerReleased(int x, int y) {}

    protected void pointerDragged(int x, int y) {}

    public final void repaint(int x, int y, int width, int height) {}

    public final void repaint() {}

    public final void serviceRepaints() {}

    protected void showNotify() {}

    protected void hideNotify() {}

    protected abstract void paint(Graphics g);

    protected void sizeChanged(int w, int h) {}
}
