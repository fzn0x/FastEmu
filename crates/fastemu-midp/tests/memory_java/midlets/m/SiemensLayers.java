package m;

import com.siemens.mp.color_game.TiledLayer;
import java.util.Vector;
import javax.microedition.lcdui.Image;
import javax.microedition.midlet.MIDlet;

/** Siemens' tiled layers that each hold 32767 animated tiles, kept in a list. */
public class SiemensLayers extends MIDlet {
    protected void startApp() {
        Image tiles = Image.createImage(16, 16);

        // One layer has at most 32767 animated tiles.
        TiledLayer capped = new TiledLayer(2, 2, tiles, 8, 8);
        int tilesMade = 0;
        try {
            for (int i = 0; i < 40000; i++) {
                capped.createAnimatedTile(1);
                tilesMade++;
            }
            System.out.println("siemens animated " + tilesMade + " made");
        } catch (IllegalArgumentException e) {
            System.out.println("siemens animated " + tilesMade + " then IllegalArgumentException");
        }
        capped = null;
        System.gc();

        Vector kept = new Vector();
        int count = 0;
        boolean full = false;
        try {
            for (int i = 0; i < 12; i++) {
                TiledLayer layer = new TiledLayer(2, 2, tiles, 8, 8);
                for (int j = 0; j < 32767; j++) {
                    layer.createAnimatedTile(1);
                }
                kept.addElement(layer);
                count++;
            }
        } catch (OutOfMemoryError e) {
            // Let go of what the game kept before anything else is called,
            // as the heap is over its limit until then.
            full = true;
            kept = null;
        }
        System.gc();
        System.out.println("siemens layers " + count + " kept" + (full ? " then OutOfMemoryError" : ""));
        System.out.println("done");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
