package m;

import javax.microedition.lcdui.game.GameCanvas;
import javax.microedition.midlet.MIDlet;

/** Makes a GameCanvas, whose buffer is a whole screen, on a heap too small for it. */
public class GameBuffer extends MIDlet {
    protected void startApp() {
        long before = Runtime.getRuntime().freeMemory();
        try {
            new GameCanvas(true) {
            };
            System.out.println("game canvas made");
        } catch (OutOfMemoryError e) {
            System.out.println("game canvas OutOfMemoryError");
        }
        System.out.println("game canvas free changed " + (before - Runtime.getRuntime().freeMemory()));
        System.out.println("done");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
