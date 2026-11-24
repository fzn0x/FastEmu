package ev;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Graphics;

/** A canvas that logs its callbacks and fills itself with its colour plus its paint count. */
class OrderCanvas extends Canvas {
    private final Order midlet;
    private final String name;
    private final int color;
    private int paints;

    OrderCanvas(Order midlet, String name, int color) {
        this.midlet = midlet;
        this.name = name;
        this.color = color;
    }

    public void paint(Graphics g) {
        paints++;
        Order.log(name + " paint " + paints);
        g.setColor(color + paints);
        g.fillRect(0, 0, getWidth(), getHeight());
    }

    protected void showNotify() {
        Order.log(name + " showNotify");
    }

    protected void hideNotify() {
        Order.log(name + " hideNotify");
    }

    protected void sizeChanged(int w, int h) {
        Order.log(name + " sizeChanged " + w + "x" + h);
    }

    protected void keyPressed(int code) {
        midlet.keyPressed(code);
    }
}
