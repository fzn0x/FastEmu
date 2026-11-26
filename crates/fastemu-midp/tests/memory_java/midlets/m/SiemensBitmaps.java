package m;

import com.siemens.mp.color_game.TiledLayer;
import java.io.InputStream;
import javax.microedition.lcdui.Image;
import javax.microedition.midlet.MIDlet;

/**
 * Siemens' bitmaps and tiled layers that the heap cannot hold: a one-bit
 * bitmap that is 1 MiB packed and 32 MiB as an image, and a layer of
 * 4096 by 4096 cells. Then a GIF of 34 bytes whose first frame is 8192 by
 * 4096 pixels on a screen of one, through Siemens' loaders by name and
 * from bytes.
 */
public class SiemensBitmaps extends MIDlet {
    /** The simple name of the class of `e`. */
    private static String outcome(Throwable e) {
        String name = e.getClass().getName();
        return name.substring(name.lastIndexOf('.') + 1);
    }

    private static void frameBomb() {
        try {
            com.siemens.mp.ui.Image.createImageWithoutScaling("/bomb.gif");
            System.out.println("siemens gif by name made");
        } catch (Throwable e) {
            System.out.println("siemens gif by name " + outcome(e));
        }
        try {
            InputStream in = SiemensBitmaps.class.getResourceAsStream("/bomb.gif");
            byte[] data = new byte[34];
            int n = 0;
            while (n < data.length) {
                n += in.read(data, n, data.length - n);
            }
            new com.siemens.mp.ui.Image(data);
            System.out.println("siemens gif by bytes made");
        } catch (Throwable e) {
            System.out.println("siemens gif by bytes " + outcome(e));
        }
    }

    protected void startApp() {
        byte[] bits = new byte[4096 * 2048 / 8];
        try {
            com.siemens.mp.ui.Image.createImageFromBitmap(bits, 4096, 2048);
            System.out.println("siemens bitmap made");
        } catch (OutOfMemoryError e) {
            System.out.println("siemens bitmap OutOfMemoryError");
        }
        bits = null;

        Image tiles = Image.createImage(16, 16);
        try {
            new TiledLayer(4096, 4096, tiles, 8, 8);
            System.out.println("siemens layer made");
        } catch (OutOfMemoryError e) {
            System.out.println("siemens layer OutOfMemoryError");
        }
        frameBomb();
        System.out.println("done");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
