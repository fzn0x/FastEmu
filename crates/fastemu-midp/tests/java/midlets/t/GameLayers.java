package t;

import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;
import javax.microedition.lcdui.game.Layer;
import javax.microedition.lcdui.game.LayerManager;
import javax.microedition.lcdui.game.Sprite;
import javax.microedition.lcdui.game.TiledLayer;
import javax.microedition.midlet.MIDlet;

/**
 * Runs the rules of Sprite, TiledLayer and LayerManager and prints each
 * result, then "layers done". Pixels are read back from mutable images
 * the layers paint on.
 */
public class GameLayers extends MIDlet {
    protected void startApp() {
        GameChecks.sprites();
        GameChecks.transforms();
        GameChecks.collisions();
        GameChecks.tiles();
        GameChecks.managers();
        System.out.println("layers done");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

/** One call whose result or exception class is printed. */
abstract class GameTry {
    abstract Object run();

    final void as(String name) {
        String result;
        try {
            result = String.valueOf(run());
        } catch (RuntimeException e) {
            result = e.getClass().getName();
        }
        System.out.println(name + ": " + result);
    }
}

final class GameChecks {
    private GameChecks() {}

    static void say(String line) {
        System.out.println(line);
    }

    /** An image whose pixel (x, y) is 0x80xxyy. */
    static Image numbered(int w, int h) {
        int[] rgb = new int[w * h];
        for (int y = 0; y < h; y++) {
            for (int x = 0; x < w; x++) {
                rgb[y * w + x] = 0xFF800000 | (x << 8) | y;
            }
        }
        return Image.createRGBImage(rgb, w, h, true);
    }

    /** An image drawn from rows of # (opaque colour) and . (clear). */
    static Image drawn(int color, String[] rows) {
        int w = rows[0].length();
        int[] rgb = new int[w * rows.length];
        for (int y = 0; y < rows.length; y++) {
            for (int x = 0; x < w; x++) {
                rgb[y * w + x] = rows[y].charAt(x) == '#' ? 0xFF000000 | color : 0;
            }
        }
        return Image.createRGBImage(rgb, w, rows.length, true);
    }

    static String at(Image image, int x, int y) {
        int[] pixel = new int[1];
        image.getRGB(pixel, 0, 1, x, y, 1, 1);
        return Integer.toHexString(pixel[0] & 0xFFFFFF);
    }

    /** The pixel at (x, y) after the layer paints on a white image. */
    static String shown(Layer layer, int x, int y) {
        Image target = Image.createImage(20, 20);
        layer.paint(target.getGraphics());
        return at(target, x, y);
    }

    static String place(Sprite s) {
        return "at " + s.getX() + "," + s.getY() + " size " + s.getWidth() + "x" + s.getHeight()
                + " ref " + s.getRefPixelX() + "," + s.getRefPixelY();
    }

    static void sprites() {
        final Image img = numbered(8, 4);
        new GameTry() {
            Object run() {
                return new Sprite((Image) null);
            }
        }.as("null image");
        new GameTry() {
            Object run() {
                return new Sprite((Sprite) null);
            }
        }.as("null sprite");
        new GameTry() {
            Object run() {
                return new Sprite(img, 3, 2);
            }
        }.as("frame width 3");
        new GameTry() {
            Object run() {
                return new Sprite(img, 4, 3);
            }
        }.as("frame height 3");
        new GameTry() {
            Object run() {
                return new Sprite(img, 0, 2);
            }
        }.as("frame width 0");
        final Sprite s = new Sprite(img, 4, 2);
        say("frames " + s.getRawFrameCount() + " " + s.getFrameSequenceLength() + " "
                + s.getFrame() + " " + place(s) + " visible " + s.isVisible());
        new GameTry() {
            Object run() {
                s.setFrame(4);
                return "set";
            }
        }.as("frame 4");
        new GameTry() {
            Object run() {
                s.setFrame(-1);
                return "set";
            }
        }.as("frame -1");
        for (int i = 0; i < 5; i++) {
            s.nextFrame();
        }
        s.prevFrame();
        s.prevFrame();
        say("next and prev " + s.getFrame() + " shows " + shown(s, 0, 0));
        new GameTry() {
            Object run() {
                s.setFrameSequence(new int[0]);
                return "set";
            }
        }.as("empty sequence");
        new GameTry() {
            Object run() {
                s.setFrameSequence(new int[] {0, 4});
                return "set";
            }
        }.as("sequence with frame 4");
        int[] sequence = {3, 1, 2};
        s.setFrameSequence(sequence);
        sequence[0] = 0;
        say("sequence " + s.getFrameSequenceLength() + " " + s.getFrame() + " shows "
                + shown(s, 0, 0));
        s.nextFrame();
        say("sequence next shows " + shown(s, 0, 0));
        s.setFrameSequence(null);
        say("default " + s.getFrameSequenceLength() + " " + s.getFrame());
        s.setPosition(3, 4);
        s.move(-1, 2);
        say("moved " + place(s));

        Image mutable = Image.createImage(2, 2);
        Graphics mg = mutable.getGraphics();
        mg.setColor(0xABCDEF);
        mg.fillRect(0, 0, 2, 2);
        Sprite live = new Sprite(mutable);
        Sprite frozen = new Sprite(live);
        mg.setColor(0x123456);
        mg.fillRect(0, 0, 2, 2);
        say("live " + shown(live, 0, 0) + " frozen " + shown(frozen, 0, 0));

        final Sprite r = new Sprite(img, 4, 2);
        r.setFrameSequence(new int[] {2, 3});
        r.setFrame(1);
        r.setImage(numbered(8, 6), 4, 2);
        say("more frames " + r.getFrame() + " " + r.getFrameSequenceLength());
        r.setImage(numbered(4, 2), 4, 2);
        say("fewer frames " + r.getFrame() + " " + r.getFrameSequenceLength());
        new GameTry() {
            Object run() {
                r.setImage(null, 4, 2);
                return "set";
            }
        }.as("null new image");
        new GameTry() {
            Object run() {
                r.setImage(img, 3, 2);
                return "set";
            }
        }.as("new frame width 3");
        r.setImage(img, 4, 2);
        r.defineReferencePixel(1, 1);
        r.setTransform(Sprite.TRANS_ROT180);
        r.setRefPixelPosition(20, 20);
        say("before resize " + place(r));
        r.setImage(img, 2, 2);
        say("resized " + place(r));
    }

    static void transforms() {
        final Sprite s = new Sprite(numbered(8, 4), 4, 2);
        s.defineReferencePixel(1, 0);
        say("ref " + s.getRefPixelX() + "," + s.getRefPixelY());
        s.setRefPixelPosition(10, 10);
        say("placed " + place(s));
        s.setTransform(Sprite.TRANS_ROT90);
        say("rot90 " + place(s) + " shows " + shown(s, 10, 10));
        s.setTransform(Sprite.TRANS_MIRROR);
        say("mirror " + place(s) + " shows " + shown(s, 10, 10) + " corner " + shown(s, 11, 10));
        new GameTry() {
            Object run() {
                s.setTransform(8);
                return "set";
            }
        }.as("transform 8");
        new GameTry() {
            Object run() {
                s.setTransform(-1);
                return "set";
            }
        }.as("transform -1");
        s.defineReferencePixel(0, 0);
        say("redefined " + place(s));
        s.setFrame(2);
        s.setVisible(false);
        Sprite copy = new Sprite(s);
        say("copy " + place(copy) + " frame " + copy.getFrame() + " visible " + copy.isVisible());
        copy.setVisible(true);
        say("copy shows " + shown(copy, 11, 10) + " hidden original shows " + shown(s, 11, 10));
    }

    static void collisions() {
        final Image solid = drawn(0x0000FF, new String[] {"####", "####", "####", "####"});
        Image ring = drawn(0x00FF00, new String[] {"###", "#.#", "###"});
        Image ell = drawn(0x00FF00, new String[] {"#..", "#..", "###"});
        Image dot = drawn(0xFF0000, new String[] {"#"});
        final Sprite a = new Sprite(solid);
        Sprite b = new Sprite(solid);
        b.setPosition(3, 3);
        say("overlap " + a.collidesWith(b, false) + " " + a.collidesWith(b, true));
        b.setPosition(4, 0);
        say("touching " + a.collidesWith(b, false) + " " + a.collidesWith(b, true));
        b.setPosition(3, 3);
        b.setVisible(false);
        say("hidden " + a.collidesWith(b, false) + " " + b.collidesWith(a, true));
        new GameTry() {
            Object run() {
                return String.valueOf(a.collidesWith((Sprite) null, false));
            }
        }.as("null other");
        final Sprite hole = new Sprite(ring);
        say("hole " + hole.collidesWith(dot, 1, 1, false) + " " + hole.collidesWith(dot, 1, 1, true)
                + " " + hole.collidesWith(dot, 0, 1, true));
        hole.defineCollisionRectangle(1, 1, 1, 1);
        say("rectangle " + hole.collidesWith(dot, 0, 0, false) + " "
                + hole.collidesWith(dot, 1, 1, false));
        new GameTry() {
            Object run() {
                hole.defineCollisionRectangle(0, 0, -1, 0);
                return "set";
            }
        }.as("negative rectangle");
        Sprite l = new Sprite(ell);
        l.defineReferencePixel(1, 1);
        say("ell " + l.collidesWith(dot, 2, 0, true) + " " + l.collidesWith(dot, 0, 0, true));
        l.setTransform(Sprite.TRANS_MIRROR);
        say("mirrored ell " + l.collidesWith(dot, 2, 0, true) + " "
                + l.collidesWith(dot, 0, 0, true));

        Image tiles = drawn(0x0000FF, new String[] {"#.##", "..##"});
        TiledLayer t = new TiledLayer(3, 2, tiles, 2, 2);
        t.setPosition(10, 20);
        t.setCell(0, 0, 1);
        t.setCell(2, 1, 2);
        Sprite d = new Sprite(dot);
        d.setPosition(10, 20);
        say("tile pixel " + d.collidesWith(t, false) + " " + d.collidesWith(t, true));
        d.setPosition(11, 21);
        say("clear pixel " + d.collidesWith(t, false) + " " + d.collidesWith(t, true));
        d.setPosition(10, 22);
        say("empty cell " + d.collidesWith(t, false));
        int animated = t.createAnimatedTile(0);
        t.setCell(1, 0, animated);
        d.setPosition(12, 20);
        say("animated cell " + d.collidesWith(t, false) + " " + d.collidesWith(t, true));
        t.setAnimatedTile(animated, 2);
        say("animated tile " + d.collidesWith(t, true));
    }

    static void tiles() {
        final Image img = numbered(8, 4);
        new GameTry() {
            Object run() {
                return new TiledLayer(0, 1, img, 2, 2);
            }
        }.as("no columns");
        new GameTry() {
            Object run() {
                return new TiledLayer(1, 0, img, 2, 2);
            }
        }.as("no rows");
        new GameTry() {
            Object run() {
                return new TiledLayer(1, 1, null, 2, 2);
            }
        }.as("null tiles");
        new GameTry() {
            Object run() {
                return new TiledLayer(1, 1, img, 3, 2);
            }
        }.as("tile width 3");
        new GameTry() {
            Object run() {
                return new TiledLayer(1, 1, img, 2, 0);
            }
        }.as("tile height 0");
        final TiledLayer t = new TiledLayer(5, 3, img, 2, 2);
        say("grid " + t.getColumns() + "x" + t.getRows() + " cells " + t.getCellWidth() + "x"
                + t.getCellHeight() + " size " + t.getWidth() + "x" + t.getHeight() + " cell "
                + t.getCell(4, 2));
        new GameTry() {
            Object run() {
                t.setCell(5, 0, 1);
                return "set";
            }
        }.as("column 5");
        new GameTry() {
            Object run() {
                t.setCell(0, 3, 1);
                return "set";
            }
        }.as("row 3");
        new GameTry() {
            Object run() {
                t.setCell(0, 0, 9);
                return "set";
            }
        }.as("tile 9");
        new GameTry() {
            Object run() {
                t.setCell(0, 0, -1);
                return "set";
            }
        }.as("tile -1");
        new GameTry() {
            Object run() {
                return String.valueOf(t.getCell(-1, 0));
            }
        }.as("get column -1");
        t.setCell(4, 2, 8);
        say("set " + t.getCell(4, 2));
        final int first = t.createAnimatedTile(3);
        int second = t.createAnimatedTile(0);
        say("animated " + first + " " + second + " " + t.getAnimatedTile(first));
        new GameTry() {
            Object run() {
                return String.valueOf(t.createAnimatedTile(9));
            }
        }.as("animate tile 9");
        new GameTry() {
            Object run() {
                return String.valueOf(t.createAnimatedTile(-1));
            }
        }.as("animate tile -1");
        t.setAnimatedTile(first, 5);
        say("changed " + t.getAnimatedTile(first));
        new GameTry() {
            Object run() {
                t.setAnimatedTile(-3, 1);
                return "set";
            }
        }.as("animated -3");
        new GameTry() {
            Object run() {
                t.setAnimatedTile(first, 9);
                return "set";
            }
        }.as("animated to tile 9");
        new GameTry() {
            Object run() {
                return String.valueOf(t.getAnimatedTile(0));
            }
        }.as("get animated 0");
        t.fillCells(1, 1, 3, 2, first);
        say("filled " + t.getCell(0, 1) + " " + t.getCell(1, 1) + " " + t.getCell(3, 2) + " "
                + t.getCell(4, 2));
        new GameTry() {
            Object run() {
                t.fillCells(0, 0, -1, 1, 1);
                return "set";
            }
        }.as("fill -1 columns");
        new GameTry() {
            Object run() {
                t.fillCells(3, 0, 3, 1, 1);
                return "set";
            }
        }.as("fill past the edge");
        new GameTry() {
            Object run() {
                t.fillCells(0, 0, 1, 1, 9);
                return "set";
            }
        }.as("fill tile 9");
        t.setPosition(1, 1);
        say("painted " + shown(t, 3, 3) + " " + shown(t, 4, 4) + " " + shown(t, 1, 1) + " "
                + shown(t, 9, 5));
        t.setAnimatedTile(first, 0);
        say("blank " + shown(t, 3, 3));
        t.setStaticTileSet(numbered(8, 8), 2, 2);
        say("kept " + t.getCell(4, 2) + " " + t.getAnimatedTile(first));
        t.setStaticTileSet(numbered(4, 2), 2, 2);
        say("cleared " + t.getCell(4, 2));
        new GameTry() {
            Object run() {
                return String.valueOf(t.getAnimatedTile(first));
            }
        }.as("deleted animated");
        new GameTry() {
            Object run() {
                t.setStaticTileSet(null, 2, 2);
                return "set";
            }
        }.as("null tile set");
        new GameTry() {
            Object run() {
                t.setStaticTileSet(img, 3, 2);
                return "set";
            }
        }.as("tile set width 3");
        t.setStaticTileSet(numbered(8, 8), 4, 4);
        say("resized " + t.getWidth() + "x" + t.getHeight());
    }

    static String names(LayerManager m, Layer a, Layer b, Layer c) {
        StringBuffer names = new StringBuffer();
        for (int i = 0; i < m.getSize(); i++) {
            Layer l = m.getLayerAt(i);
            names.append(l == a ? " a" : l == b ? " b" : l == c ? " c" : " ?");
        }
        return names.toString();
    }

    static void managers() {
        Image dot = drawn(0xFF0000, new String[] {"#"});
        final LayerManager m = new LayerManager();
        final Sprite a = new Sprite(dot);
        Sprite b = new Sprite(dot);
        Sprite c = new Sprite(dot);
        new GameTry() {
            Object run() {
                m.append(null);
                return "set";
            }
        }.as("append null");
        new GameTry() {
            Object run() {
                m.insert(a, 1);
                return "set";
            }
        }.as("insert at 1 of 0");
        new GameTry() {
            Object run() {
                m.insert(a, -1);
                return "set";
            }
        }.as("insert at -1");
        new GameTry() {
            Object run() {
                return m.getLayerAt(0);
            }
        }.as("layer 0 of 0");
        new GameTry() {
            Object run() {
                m.remove(null);
                return "set";
            }
        }.as("remove null");
        m.append(a);
        m.append(b);
        m.insert(c, 1);
        say("order" + names(m, a, b, c));
        new GameTry() {
            Object run() {
                m.insert(a, 3);
                return "set";
            }
        }.as("insert a present layer at the end");
        m.insert(a, 2);
        say("moved" + names(m, a, b, c));
        m.append(c);
        say("appended" + names(m, a, b, c));
        m.remove(b);
        m.remove(b);
        say("removed" + names(m, a, b, c) + " size " + m.getSize());
        new GameTry() {
            Object run() {
                m.setViewWindow(0, 0, -1, 1);
                return "set";
            }
        }.as("negative window");

        Image red = drawn(0xFF0000, new String[] {"####", "####", "####", "####"});
        Image blue = drawn(0x0000FF, new String[] {"####", "####", "####", "####"});
        LayerManager view = new LayerManager();
        Sprite under = new Sprite(red);
        under.setPosition(5, 5);
        Sprite over = new Sprite(blue);
        over.setPosition(6, 6);
        view.append(over);
        view.append(under);
        view.setViewWindow(5, 5, 3, 3);
        Image target = Image.createImage(20, 20);
        Graphics g = target.getGraphics();
        g.setClip(0, 0, 15, 15);
        view.paint(g, 10, 10);
        String clip = g.getClipX() + " " + g.getClipY() + " " + g.getClipWidth() + " "
                + g.getClipHeight();
        g.setColor(0);
        g.fillRect(0, 0, 1, 1);
        say("window " + at(target, 10, 10) + " " + at(target, 11, 11) + " " + at(target, 12, 12)
                + " " + at(target, 13, 13) + " " + at(target, 9, 10) + " clip " + clip
                + " origin " + at(target, 0, 0));
        under.setVisible(false);
        Image again = Image.createImage(20, 20);
        view.paint(again.getGraphics(), 10, 10);
        say("hidden layer " + at(again, 10, 10) + " " + at(again, 11, 11));
    }
}
