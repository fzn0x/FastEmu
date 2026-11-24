package v;

import com.siemens.mp.color_game.Layer;
import com.siemens.mp.color_game.LayerManager;
import com.siemens.mp.color_game.Sprite;
import com.siemens.mp.color_game.TiledLayer;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/**
 * Siemens' com.siemens.mp.color_game layers: sprite frames and
 * sequences, collisions, tiled layers with animated tiles, and a layer
 * manager with a view window and a game's own layer.
 */
public class SiemensColor {
    static Image filled(int w, int h, int colour) {
        Image img = Image.createImage(w, h);
        Graphics g = img.getGraphics();
        g.setColor(colour);
        g.fillRect(0, 0, w, h);
        return img;
    }

    interface Call {
        void run() throws Exception;
    }

    static void expect(String what, Call call) {
        try {
            call.run();
            System.out.println(what + ": no exception");
        } catch (Exception e) {
            System.out.println(what + ": " + e.getClass().getName());
        }
    }

    public static void main(String[] args) {
        // Two 2 by 2 frames: all black, then a black left column.
        Image strip = Image.createImage(4, 2);
        Graphics sg = strip.getGraphics();
        sg.setColor(0);
        sg.fillRect(0, 0, 3, 2);
        final Sprite sp = new Sprite(Image.createImage(strip), 2, 2);
        System.out.println("frames: " + sp.getRawFrameCount() + " sequence " + sp.getFrameSequenceLength()
                + " size " + sp.getWidth() + "x" + sp.getHeight());
        Image target = Image.createImage(6, 3);
        sp.setPosition(1, 1);
        sp.paint(target.getGraphics());
        System.out.println("frame 0: " + SiemensGame.rows(target));
        sp.nextFrame();
        target = Image.createImage(6, 3);
        sp.paint(target.getGraphics());
        System.out.println("frame 1: " + SiemensGame.rows(target) + " index " + sp.getFrame());
        sp.setFrameSequence(new int[] {1, 1, 0});
        sp.prevFrame();
        System.out.println("sequence: " + sp.getFrameSequenceLength() + " at " + sp.getFrame());
        target = Image.createImage(6, 3);
        sp.paint(target.getGraphics());
        System.out.println("sequence frame: " + SiemensGame.rows(target));
        expect("frame 3", new Call() {
            public void run() {
                sp.setFrame(3);
            }
        });
        expect("raw frame 2", new Call() {
            public void run() {
                sp.setFrameSequence(new int[] {2});
            }
        });
        expect("empty sequence", new Call() {
            public void run() {
                sp.setFrameSequence(new int[0]);
            }
        });
        final Image fstrip = strip;
        expect("width 3", new Call() {
            public void run() {
                new Sprite(fstrip, 3, 2);
            }
        });
        Sprite copy = new Sprite(sp);
        System.out.println("copy: " + copy.getX() + "," + copy.getY() + " sequence "
                + copy.getFrameSequenceLength() + " at " + copy.getFrame());
        sp.setFrameSequence(null);
        System.out.println("default sequence: " + sp.getFrameSequenceLength() + " at " + sp.getFrame());

        // Collisions.
        Image solid = Image.createImage(2, 2);
        Image dot = Image.createRGBImage(new int[] {0xFF000000, 0, 0, 0}, 2, 2, true);
        Sprite a = new Sprite(solid);
        Sprite b = new Sprite(solid);
        b.setPosition(1, 1);
        boolean overlapping = a.collidesWith(b, false);
        b.setPosition(2, 0);
        System.out.println("rectangles: " + overlapping + " " + a.collidesWith(b, false));
        Sprite d = new Sprite(dot);
        d.setPosition(-1, -1);
        System.out.println("pixels: rectangle " + a.collidesWith(d, false) + " pixel " + a.collidesWith(d, true));
        d.setPosition(1, 1);
        System.out.println("opaque corner: " + a.collidesWith(d, true));
        System.out.println("image: " + a.collidesWith(dot, 1, 1, true) + " " + a.collidesWith(dot, 2, 2, false)
                + " " + a.collidesWith(dot, -1, -1, true));
        a.setCollisionRectangle(0, 0, 1, 1);
        System.out.println("small rectangle: " + a.collidesWith(d, false));
        a.setVisible(false);
        System.out.println("hidden: " + a.collidesWith(d, false) + " " + a.isVisible());

        // A tiled layer: tile 1 black, tile 2 white.
        Image tiles = Image.createImage(4, 2);
        Graphics tg = tiles.getGraphics();
        tg.setColor(0);
        tg.fillRect(0, 0, 2, 2);
        final TiledLayer t = new TiledLayer(3, 2, tiles, 2, 2);
        System.out.println("tiled: " + t.getColumns() + "x" + t.getRows() + " cells " + t.getCellWidth() + "x"
                + t.getCellHeight() + " size " + t.getWidth() + "x" + t.getHeight());
        t.setCell(0, 0, 1);
        t.fillCells(1, 1, 2, 1, 1);
        int animated = t.createAnimatedTile(1);
        t.setCell(2, 0, animated);
        target = filled(6, 4, 0xFF0000);
        t.paint(target.getGraphics());
        System.out.println("cells: " + SiemensGame.rows(target));
        t.setAnimatedTile(animated, 2);
        target = filled(6, 4, 0xFF0000);
        t.paint(target.getGraphics());
        System.out.println("animated: " + SiemensGame.rows(target) + " cell " + t.getCell(2, 0) + " shows "
                + t.getAnimatedTile(animated));
        expect("cell 3,0", new Call() {
            public void run() {
                t.setCell(3, 0, 1);
            }
        });
        expect("tile 3", new Call() {
            public void run() {
                t.setCell(0, 0, 3);
            }
        });
        expect("animated -2", new Call() {
            public void run() {
                t.getAnimatedTile(-2);
            }
        });
        final Image ftiles = tiles;
        expect("no columns", new Call() {
            public void run() {
                new TiledLayer(0, 1, ftiles, 2, 2);
            }
        });

        // The manager: the sprite over the tiled layer, then a view window.
        sp.setFrame(0);
        sp.setPosition(2, 0);
        LayerManager lm = new LayerManager();
        lm.append(t);
        lm.insert(sp, 0);
        System.out.println("layers: " + lm.getSize() + " top " + (lm.getLayerAt(0) == sp));
        target = Image.createImage(6, 4);
        lm.paint(target.getGraphics(), 0, 0);
        System.out.println("managed: " + SiemensGame.rows(target));
        lm.setViewWindow(2, 0, 2, 4);
        target = Image.createImage(6, 4);
        Graphics g = target.getGraphics();
        lm.paint(g, 1, 0);
        System.out.println("view: " + SiemensGame.rows(target) + " restored " + g.getTranslateX() + " "
                + g.getClipWidth());
        lm.setViewWindow(0, 0, 6, 4);
        Dot dotLayer = new Dot();
        dotLayer.setPosition(5, 3);
        lm.insert(dotLayer, 0);
        target = Image.createImage(6, 4);
        lm.paint(target.getGraphics(), 0, 0);
        System.out.println("own layer: " + SiemensGame.rows(target));
        lm.remove(dotLayer);
        System.out.println("removed: " + lm.getSize());
        final LayerManager flm = lm;
        expect("layer 2", new Call() {
            public void run() {
                flm.getLayerAt(2);
            }
        });

        t.setStaticTileSet(Image.createImage(2, 2), 2, 2);
        System.out.println("fewer tiles: " + t.getCell(0, 0));
        System.out.println("done");
    }
}

/** A game's own layer: one red pixel. */
class Dot extends Layer {
    Dot() {
        super(1, 1);
    }

    public void paint(Graphics g) {
        g.setColor(0xFF0000);
        g.fillRect(getX(), getY(), 1, 1);
    }
}
