package v;

import com.siemens.mp.game.ExtendedImage;
import com.siemens.mp.game.GraphicObjectManager;
import com.siemens.mp.game.Light;
import com.siemens.mp.game.Melody;
import com.siemens.mp.game.MelodyComposer;
import com.siemens.mp.game.Sound;
import com.siemens.mp.game.Sprite;
import com.siemens.mp.game.TiledBackground;
import com.siemens.mp.game.Vibrator;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/**
 * Siemens' com.siemens.mp.game and ui.Image: lights and vibration,
 * melodies, bitmaps, ExtendedImage pixels, sprites and tiled backgrounds
 * painted by a GraphicObjectManager.
 */
public class SiemensGame {
    /** An image's rows: # black, . white, r red, t transparent. */
    static String rows(Image img) {
        int w = img.getWidth();
        int h = img.getHeight();
        int[] out = new int[w * h];
        img.getRGB(out, 0, w, 0, 0, w, h);
        StringBuffer sb = new StringBuffer();
        for (int y = 0; y < h; y++) {
            if (y > 0) {
                sb.append('/');
            }
            for (int x = 0; x < w; x++) {
                sb.append(shade(out[y * w + x]));
            }
        }
        return sb.toString();
    }

    static char shade(int p) {
        if ((p >>> 24) == 0) {
            return 't';
        }
        return p == 0xFF000000 ? '#' : p == 0xFFFFFFFF ? '.' : p == 0xFFFF0000 ? 'r' : '?';
    }

    /** The shades at the given points of an image. */
    static String at(Image img, int[] points) {
        int[] p = new int[1];
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < points.length; i += 2) {
            img.getRGB(p, 0, 1, points[i], points[i + 1], 1, 1);
            sb.append(shade(p[0]));
        }
        return sb.toString();
    }

    static Image red(int w, int h) {
        Image img = Image.createImage(w, h);
        Graphics g = img.getGraphics();
        g.setColor(0xFF0000);
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

    public static void main(String[] args) throws Exception {
        Light.setLightOn();
        Light.setLightOn();
        Light.setLightOff();
        Vibrator.triggerVibrator(150);
        Vibrator.startVibrator();
        Vibrator.stopVibrator();
        Sound.playTone(440, 50);

        MelodyComposer composer = new MelodyComposer();
        composer.setBPM(120);
        composer.appendNote(MelodyComposer.TONE_A2, MelodyComposer.TONELENGTH_1_4);
        composer.appendNote(MelodyComposer.TONE_PAUSE, MelodyComposer.TONELENGTH_1_8);
        Melody melody = composer.getMelody();
        melody.play();
        Melody.stop();
        System.out.println("composer: " + composer.length() + " of " + MelodyComposer.maxLength());
        composer = new MelodyComposer(new int[] {0, 2, MelodyComposer.TONE_REPEAT, 1}, 90);
        System.out.println("composed: " + composer.length());
        composer.resetMelody();
        System.out.println("reset: " + composer.length());

        // Bitmaps.
        Image bits = com.siemens.mp.ui.Image.createImageFromBitmap(new byte[] {(byte) 0xA0, 0x0F}, 8, 2);
        System.out.println("bitmap: " + rows(bits));
        com.siemens.mp.ui.Image.mirrorImageHorizontally(bits);
        System.out.println("mirrored: " + rows(bits));
        com.siemens.mp.ui.Image.mirrorImageVertically(bits);
        System.out.println("flipped: " + rows(bits));
        Image two = com.siemens.mp.ui.Image.createTransparentImageFromBitmap(new byte[] {0x1B}, 4, 1);
        System.out.println("two bits: " + rows(two));
        Image rgb = com.siemens.mp.ui.Image.createRGBImage(new byte[] {(byte) 0xE0, (byte) 0xC0}, 2, 1,
                com.siemens.mp.ui.Image.COLOR_BMP_8BIT);
        System.out.println("rgb: " + rows(rgb));
        com.siemens.mp.ui.Image ui = new com.siemens.mp.ui.Image(16, 8);
        System.out.println("ui image: " + ui.getWidth() + "x" + ui.getHeight()
                + " native " + com.siemens.mp.ui.Image.getNativeImage(bits).getWidth());
        expect("rgb type 3", new Call() {
            public void run() throws Exception {
                com.siemens.mp.ui.Image.createRGBImage(new byte[4], 2, 2, 3);
            }
        });

        // ExtendedImage, one bit per pixel.
        Image plain = Image.createImage(8, 2);
        ExtendedImage e = new ExtendedImage(plain);
        System.out.println("same image: " + (e.getImage() == plain));
        e.setPixel(1, 0, (byte) 1);
        System.out.println("pixels: " + e.getPixel(1, 0) + " " + e.getPixel(0, 0) + " " + e.getPixel(9, 9));
        System.out.println("extended: " + rows(plain));
        e.setPixels(new byte[] {(byte) 0xF0, 0x01}, 0, 0, 8, 2);
        System.out.println("set pixels: " + rows(plain));
        byte[] back = new byte[2];
        e.getPixelBytes(back, 0, 0, 8, 2);
        System.out.println("pixel bytes: " + Integer.toHexString(back[0] & 0xFF) + " "
                + Integer.toHexString(back[1] & 0xFF));
        e.clear((byte) 1);
        System.out.println("cleared: " + rows(plain));
        expect("width 7", new Call() {
            public void run() {
                new ExtendedImage(Image.createImage(7, 2));
            }
        });
        final ExtendedImage fe = e;
        expect("x 3", new Call() {
            public void run() {
                fe.setPixels(new byte[2], 3, 0, 8, 2);
            }
        });

        // ExtendedImage, two bits per pixel.
        Image twoBit = com.siemens.mp.ui.Image.createTransparentImageFromBitmap(
                new byte[] {0x1B, (byte) 0xE4}, 8, 1);
        ExtendedImage t = new ExtendedImage(twoBit);
        System.out.println("two-bit: " + t.getPixel(0, 0) + t.getPixel(1, 0) + t.getPixel(2, 0)
                + t.getPixel(7, 0) + " copied " + (t.getImage() != twoBit));
        t.setPixel(0, 0, (byte) 2);
        System.out.println("two-bit set: " + rows(t.getImage()));

        // A sprite of two 8 by 1 frames: frame 0 black with its right half
        // masked out, frame 1 white then black.
        Sprite s = new Sprite(new byte[] {(byte) 0xFF, 0x0F}, 0, 8, 1,
                new byte[] {(byte) 0xF0, (byte) 0xFF}, 0, 2);
        GraphicObjectManager m = new GraphicObjectManager();
        m.addObject(s);
        s.setPosition(2, 1);
        Image target = Image.createImage(12, 3);
        m.paint(target, 0, 0);
        System.out.println("sprite: " + rows(target));
        s.setFrame(1);
        target = Image.createImage(12, 3);
        m.paint(target, 1, 0);
        System.out.println("frame 1 offset: " + rows(target) + " at " + s.getXPosition() + ","
                + s.getYPosition() + " frame " + s.getFrame());
        s.setVisible(false);
        target = Image.createImage(12, 3);
        m.paint(target, 0, 0);
        System.out.println("hidden: " + rows(target) + " " + s.getVisible());
        s.setVisible(true);

        Sprite s2 = new Sprite(new byte[] {(byte) 0xFF, 0x0F}, 0, 8, 1, null, 0, 2);
        s2.setPosition(9, 1);
        boolean near = s.isCollidingWith(s2);
        s2.setPosition(10, 1);
        System.out.println("collide: " + near + " " + s.isCollidingWith(s2) + " pos "
                + s.isCollidingWithPos(2, 1) + " " + s.isCollidingWithPos(10, 1));
        s.setCollisionRectangle(0, 0, 4, 1);
        s2.setPosition(5, 1);
        System.out.println("small rectangle: " + s.isCollidingWith(s2));
        s2.setPosition(6, 1);
        System.out.println("beside it: " + s.isCollidingWith(s2));

        // Sprites from MIDP images and their checks.
        Image frames = Image.createImage(8, 4);
        Graphics fg = frames.getGraphics();
        fg.setColor(0);
        fg.fillRect(0, 0, 8, 2);
        Image mask = Image.createImage(8, 4);
        Graphics mg = mask.getGraphics();
        mg.setColor(0);
        mg.fillRect(0, 0, 2, 4);
        Sprite fromImages = new Sprite(frames, mask, 2);
        GraphicObjectManager m2 = new GraphicObjectManager();
        m2.addObject(fromImages);
        target = red(8, 2);
        m2.paint(target, 0, 0);
        System.out.println("image sprite: " + rows(target));
        final Image fframes = frames;
        expect("three frames", new Call() {
            public void run() {
                new Sprite(fframes, null, 3);
            }
        });
        expect("transparent pixels", new Call() {
            public void run() {
                new Sprite(com.siemens.mp.ui.Image.createTransparentImageFromBitmap(new byte[2], 8, 1),
                        null, 1);
            }
        });
        expect("short bits", new Call() {
            public void run() {
                new Sprite(new byte[1], 0, 8, 1, null, 0, 2);
            }
        });

        // A tiled background: a box tile; the map shows it, black, white and
        // transparent, and repeats.
        byte[] box = {(byte) 0xFF, (byte) 0x81, (byte) 0x81, (byte) 0x81, (byte) 0x81, (byte) 0x81,
            (byte) 0x81, (byte) 0xFF};
        byte[] map = {3, 2, 1, 0};
        TiledBackground bg = new TiledBackground(box, null, map, 2, 2);
        GraphicObjectManager m3 = new GraphicObjectManager();
        m3.addObject(bg);
        int[] points = {0, 0, 1, 1, 8, 0, 0, 8, 8, 8, 16, 0, 17, 1, 0, 16};
        target = red(24, 24);
        m3.paint(target, 0, 0);
        System.out.println("tiles: " + at(target, points));
        bg.setPositionInMap(8, 0);
        target = red(24, 24);
        m3.paint(target, 0, 0);
        System.out.println("scrolled: " + at(target, points));
        map[0] = 2;
        bg.setPositionInMap(0, 0);
        target = red(24, 24);
        m3.paint(target, 0, 0);
        System.out.println("map changed: " + at(target, points));
        expect("tile 9", new Call() {
            public void run() {
                new TiledBackground(new byte[8], null, new byte[] {9}, 1, 1);
            }
        });
        expect("odd tile bytes", new Call() {
            public void run() {
                new TiledBackground(new byte[7], null, new byte[] {0}, 1, 1);
            }
        });

        // The manager's list.
        m.addObject(s2);
        m.insertObject(bg, 0);
        System.out.println("positions: " + m.getObjectPosition(bg) + " " + m.getObjectPosition(s) + " "
                + m.getObjectPosition(s2) + " first " + (m.getObjectAt(0) == bg));
        m.deleteObject(0);
        m.deleteObject(s2);
        System.out.println("after deleting: " + m.getObjectPosition(s) + " " + m.getObjectPosition(s2));
        final GraphicObjectManager fm = m;
        expect("object 5", new Call() {
            public void run() {
                fm.getObjectAt(5);
            }
        });
        final Image transparent = two;
        expect("paint on transparent", new Call() {
            public void run() {
                fm.paint(transparent, 0, 0);
            }
        });
        System.out.println("done");
    }
}
