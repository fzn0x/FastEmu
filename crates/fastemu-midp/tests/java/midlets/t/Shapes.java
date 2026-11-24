package t;

import java.io.IOException;
import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Font;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;
import javax.microedition.midlet.MIDlet;

/** Draws shapes, text and images on a full-screen canvas. */
public class Shapes extends MIDlet {
    protected void startApp() {
        Display.getDisplay(this).setCurrent(new ShapesCanvas());
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class ShapesCanvas extends Canvas {
    ShapesCanvas() {
        setFullScreenMode(true);
    }

    protected void paint(Graphics g) {
        g.setColor(0xFF0000);
        g.fillRect(0, 0, getWidth(), getHeight());
        g.setColor(0x00FF00);
        g.fillRect(10, 10, 20, 20);
        g.setColor(0x0000FF);
        g.drawLine(0, 50, getWidth() - 1, 50);
        g.setColor(0xFFFFFF);
        g.drawString("Hello", 100, 100, Graphics.TOP | Graphics.LEFT);
        Image mutable = Image.createImage(4, 4);
        Graphics ig = mutable.getGraphics();
        ig.setColor(0x123456);
        ig.fillRect(0, 0, 4, 4);
        g.drawImage(mutable, 200, 200, Graphics.TOP | Graphics.LEFT);
        try {
            Image dot = Image.createImage("res/dot.png");
            g.drawImage(dot, 150, 150, Graphics.TOP | Graphics.LEFT);
        } catch (IOException e) {
            System.out.println("dot: " + e);
        }
        try {
            Image.createImage("/res/missing.png");
            System.out.println("missing: no exception");
        } catch (IOException e) {
            System.out.println("missing: IOException");
        }
        Font font = g.getFont();
        System.out.println("painted " + getWidth() + "x" + getHeight()
                + " font " + (font == Font.getDefaultFont())
                + " clip " + g.getClipWidth() + "x" + g.getClipHeight());
    }
}
