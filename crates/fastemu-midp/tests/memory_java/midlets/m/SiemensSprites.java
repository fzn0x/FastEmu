package m;

import com.siemens.mp.color_game.Sprite;
import java.util.Vector;
import javax.microedition.lcdui.Image;
import javax.microedition.midlet.MIDlet;

/**
 * Siemens' sprites that each keep a copy of a big frame sequence, kept in a
 * list, made by setFrameSequence and by the copy constructor.
 */
public class SiemensSprites extends MIDlet {
    protected void startApp() {
        Image frame = Image.createImage(16, 16);

        // Each sprite keeps its own copy of this array of 400 KB.
        int[] sequence = new int[100000];
        Vector kept = new Vector();
        int count = 0;
        boolean full = false;
        try {
            for (int i = 0; i < 50; i++) {
                Sprite sprite = new Sprite(frame);
                sprite.setFrameSequence(sequence);
                kept.addElement(sprite);
                count++;
            }
        } catch (OutOfMemoryError e) {
            // Let go of what the game kept before anything else is called,
            // as the heap is over its limit until then.
            full = true;
            kept = null;
        }
        kept = null;
        sequence = null;
        System.gc();
        System.out.println("siemens sprites " + count + " kept" + (full ? " then OutOfMemoryError" : ""));

        // A copy of a sprite has a frame sequence of its own: 80 KB each.
        int[] small = new int[20000];
        Sprite original = new Sprite(frame);
        original.setFrameSequence(small);
        small = null;
        kept = new Vector();
        count = 0;
        full = false;
        try {
            for (int i = 0; i < 50; i++) {
                kept.addElement(new Sprite(original));
                count++;
            }
        } catch (OutOfMemoryError e) {
            full = true;
            kept = null;
        }
        kept = null;
        System.gc();
        System.out.println("siemens copies " + count + " kept" + (full ? " then OutOfMemoryError" : ""));
        System.out.println("done");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
