package m;

import java.util.Vector;
import javax.microedition.lcdui.Image;
import javax.microedition.lcdui.game.TiledLayer;
import javax.microedition.midlet.MIDlet;

/**
 * Makes tiled layers the heap cannot hold, layers it can, a layer with
 * more animated tiles than a layer may have, and many layers kept in a
 * list. Prints what happened to each.
 */
public class Tiles extends MIDlet {
    protected void startApp() {
        Image tiles = Image.createImage(16, 16);
        try {
            new TiledLayer(4096, 4096, tiles, 8, 8);
            System.out.println("big layer made");
        } catch (OutOfMemoryError e) {
            System.out.println("big layer OutOfMemoryError");
        }
        TiledLayer small = new TiledLayer(20, 20, tiles, 8, 8);
        small.fillCells(0, 0, 20, 20, 3);
        System.out.println("small layer " + small.getColumns() + "x" + small.getRows()
                + " cell " + small.getCell(19, 19));

        TiledLayer layer = new TiledLayer(4, 4, tiles, 8, 8);
        int made = 0;
        int last = 0;
        try {
            for (int i = 0; i < 40000; i++) {
                last = layer.createAnimatedTile(1);
                made++;
            }
            System.out.println("animated " + made + " made");
        } catch (OutOfMemoryError e) {
            System.out.println("animated " + made + " then OutOfMemoryError last " + last);
        }
        System.out.println("animated tile 1 shows " + layer.getAnimatedTile(-1));

        Vector kept = new Vector();
        int count = 0;
        try {
            for (int i = 0; i < 100; i++) {
                kept.addElement(new TiledLayer(300, 300, tiles, 8, 8));
                count++;
            }
            System.out.println("layers " + count + " kept");
        } catch (OutOfMemoryError e) {
            System.out.println("layers " + count + " kept then OutOfMemoryError");
        }
        kept = null;
        layer = null;
        System.gc();
        TiledLayer again = new TiledLayer(30, 30, tiles, 8, 8);
        System.out.println("done " + again.getColumns());
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
