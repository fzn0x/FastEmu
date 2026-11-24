package t;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/** Runs its whole game loop inside startApp and never returns. */
public class Looper extends MIDlet {
    protected void startApp() {
        LooperCanvas canvas = new LooperCanvas();
        Display.getDisplay(this).setCurrent(canvas);
        boolean reported = false;
        for (int i = 0; ; i++) {
            canvas.color = (i % 2 == 0) ? 0x00FF00 : 0x0000FF;
            canvas.repaint();
            if (canvas.keys > 0 && !reported) {
                reported = true;
                System.out.println("loop sees key");
            }
            try {
                Thread.sleep(10);
            } catch (InterruptedException e) {
                return;
            }
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {
        System.out.println("destroyApp " + unconditional);
    }
}

class LooperCanvas extends Canvas {
    volatile int color;
    volatile int keys;

    LooperCanvas() {
        setFullScreenMode(true);
    }

    protected void paint(Graphics g) {
        g.setColor(color);
        g.fillRect(0, 0, getWidth(), getHeight());
    }

    protected void keyPressed(int code) {
        keys++;
        System.out.println("key in loop " + code);
    }
}
