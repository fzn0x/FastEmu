package m;

import java.util.Vector;
import javax.microedition.lcdui.Image;
import javax.microedition.midlet.MIDlet;

/**
 * Makes images the heap cannot hold, and images it can: a 4096 by 4096
 * image, a copy of one that fits twice, and a hundred 400 by 400 images
 * kept in a list. Prints what happened to each.
 */
public class BigImage extends MIDlet {
    protected void startApp() {
        long before = Runtime.getRuntime().freeMemory();
        try {
            Image.createImage(4096, 4096);
            System.out.println("big image made");
        } catch (OutOfMemoryError e) {
            System.out.println("big image OutOfMemoryError");
        }
        try {
            Image.createImage(Integer.MAX_VALUE, Integer.MAX_VALUE);
            System.out.println("huge image made");
        } catch (OutOfMemoryError e) {
            System.out.println("huge image OutOfMemoryError");
        }
        try {
            Image.createRGBImage(new int[16], 8192, 8192, false);
            System.out.println("rgb image made");
        } catch (ArrayIndexOutOfBoundsException e) {
            System.out.println("rgb image ArrayIndexOutOfBoundsException");
        } catch (OutOfMemoryError e) {
            System.out.println("rgb image OutOfMemoryError");
        }
        try {
            Image.createImage(0, 5);
            System.out.println("empty image made");
        } catch (IllegalArgumentException e) {
            System.out.println("empty image IllegalArgumentException");
        }
        long after = Runtime.getRuntime().freeMemory();
        System.out.println("free changed " + (before - after));
        Image small = Image.createImage(64, 64);
        System.out.println("small image " + small.getWidth() + "x" + small.getHeight());

        Vector kept = new Vector();
        int made = 0;
        try {
            for (int i = 0; i < 100; i++) {
                kept.addElement(Image.createImage(400, 400));
                made++;
            }
            System.out.println("kept " + made + " images");
        } catch (OutOfMemoryError e) {
            System.out.println("kept " + made + " images then OutOfMemoryError");
        }
        kept = null;
        System.gc();
        // The heap works again once the images are let go.
        Image again = Image.createImage(100, 100);
        System.out.println("done " + again.getWidth());
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
