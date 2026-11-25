package m;

import java.util.Vector;
import javax.microedition.lcdui.Image;
import javax.microedition.lcdui.game.Sprite;
import javax.microedition.midlet.MIDlet;

/**
 * Gives the same 400 KB frame sequence to many sprites kept in a list;
 * each sprite keeps a copy of it.
 */
public class Sprites extends MIDlet {
    protected void startApp() {
        Image frame = Image.createImage(16, 16);
        int[] sequence = new int[100000];
        Vector kept = new Vector();
        Sprite sprite = null;
        int count = 0;
        try {
            for (int i = 0; i < 50; i++) {
                sprite = new Sprite(frame);
                sprite.setFrameSequence(sequence);
                kept.addElement(sprite);
                count++;
            }
            System.out.println("sprites " + count + " kept");
        } catch (OutOfMemoryError e) {
            // The sprite that went over the limit is not kept.
            sprite = null;
            System.out.println("sprites " + count + " kept then OutOfMemoryError");
        }
        kept = null;
        sequence = null;
        System.gc();

        // A copy of a sprite has a frame sequence of its own: 80 KB each.
        int[] small = new int[20000];
        Sprite original = new Sprite(frame);
        original.setFrameSequence(small);
        small = null;
        kept = new Vector();
        count = 0;
        boolean copiesFull = false;
        try {
            for (int i = 0; i < 50; i++) {
                kept.addElement(new Sprite(original));
                count++;
            }
        } catch (OutOfMemoryError e) {
            copiesFull = true;
            kept = null;
        }
        kept = null;
        System.gc();
        System.out.println("copies " + count + " kept" + (copiesFull ? " then OutOfMemoryError" : ""));
        System.out.println("done");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
