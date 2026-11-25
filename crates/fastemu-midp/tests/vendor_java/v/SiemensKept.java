package v;

import com.siemens.mp.game.Sprite;
import javax.microedition.lcdui.Image;

/**
 * Sprites made from one picture each copy its pixels into native frames
 * and make almost no Java object: kept ones must end in OutOfMemoryError,
 * and ones nobody keeps must be collected as they go. Each case is a
 * method of its own that returns how far it got, so that nothing it kept
 * is alive when the result is printed.
 */
public class SiemensKept {
    static final int MADE = 100;

    static int make(Image picture, boolean keep) {
        Sprite[] kept = new Sprite[MADE];
        int made = 0;
        try {
            for (; made < MADE; made++) {
                Sprite s = new Sprite(picture, null, 1);
                if (keep) {
                    kept[made] = s;
                }
            }
        } catch (OutOfMemoryError e) {
            kept = null;
        }
        return made;
    }

    public static void main(String[] args) throws Exception {
        Image picture = Image.createImage(96, 96);
        System.out.println("kept sprites: " + (make(picture, true) < MADE ? "oom" : "no oom"));
        System.out.println("dropped sprites: " + (make(picture, false) == MADE ? "ok" : "oom"));
        System.out.println("done");
    }
}
