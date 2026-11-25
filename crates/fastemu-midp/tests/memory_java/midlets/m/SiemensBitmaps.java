package m;

import com.siemens.mp.color_game.TiledLayer;
import javax.microedition.lcdui.Image;
import javax.microedition.midlet.MIDlet;

/**
 * Siemens' bitmaps and tiled layers that the heap cannot hold: a one-bit
 * bitmap that is 1 MiB packed and 32 MiB as an image, and a layer of
 * 4096 by 4096 cells.
 */
public class SiemensBitmaps extends MIDlet {
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
        System.out.println("done");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
