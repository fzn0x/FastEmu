package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/** Logs every lifecycle call and its suite's properties. */
public class Lifecycle extends MIDlet {
    private LifecycleCanvas canvas;
    private int starts;

    protected void startApp() {
        starts++;
        System.out.println("startApp " + starts);
        if (canvas == null) {
            System.out.println("property " + getAppProperty("Game-Speed")
                    + " " + getAppProperty("MIDlet-Name")
                    + " " + getAppProperty("No-Such-Key"));
            System.out.println("platform " + System.getProperty("microedition.platform")
                    + " os " + System.getProperty("os.name"));
            canvas = new LifecycleCanvas();
            Display.getDisplay(this).setCurrent(canvas);
        }
    }

    protected void pauseApp() {
        System.out.println("pauseApp");
    }

    protected void destroyApp(boolean unconditional) {
        System.out.println("destroyApp " + unconditional);
    }
}

class LifecycleCanvas extends Canvas {
    protected void paint(Graphics g) {
        g.setColor(0x0000FF);
        g.fillRect(0, 0, getWidth(), getHeight());
    }

    protected void showNotify() {
        System.out.println("showNotify");
    }

    protected void hideNotify() {
        System.out.println("hideNotify");
    }
}
