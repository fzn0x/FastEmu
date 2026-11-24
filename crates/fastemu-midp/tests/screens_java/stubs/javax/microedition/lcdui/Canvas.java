package javax.microedition.lcdui;

public abstract class Canvas extends Displayable {
    protected Canvas() {}

    protected abstract void paint(Graphics g);

    protected void showNotify() {}

    protected void hideNotify() {}
}
