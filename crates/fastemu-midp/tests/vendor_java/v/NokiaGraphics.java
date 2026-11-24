package v;

import com.nokia.mid.ui.DirectGraphics;
import com.nokia.mid.ui.DirectUtils;
import java.io.InputStream;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/**
 * DirectGraphics and DirectUtils on images: every pixel format drawn and
 * read back pixel by pixel, the manipulations, the ARGB colour shared with
 * Graphics, polygons and triangles, and the exceptions.
 */
public class NokiaGraphics {
    static String hex(int v) {
        return Integer.toHexString(v);
    }

    /** The ARGB pixels of an image, row by row, in hex. */
    static String argb(Image img) {
        int w = img.getWidth();
        int h = img.getHeight();
        int[] out = new int[w * h];
        img.getRGB(out, 0, w, 0, 0, w, h);
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < out.length; i++) {
            if (i > 0) {
                sb.append(' ');
            }
            sb.append(hex(out[i]));
        }
        return sb.toString();
    }

    /** An image's rows: # black, . white, r red, ? anything else. */
    static String shape(Image img) {
        int w = img.getWidth();
        int h = img.getHeight();
        int[] out = new int[w * h];
        img.getRGB(out, 0, w, 0, 0, w, h);
        StringBuffer sb = new StringBuffer();
        for (int y = 0; y < h; y++) {
            if (y > 0) {
                sb.append(' ');
            }
            for (int x = 0; x < w; x++) {
                int p = out[y * w + x];
                sb.append(p == 0xFF000000 ? '#' : p == 0xFFFFFFFF ? '.' : p == 0xFFFF0000 ? 'r' : '?');
            }
        }
        return sb.toString();
    }

    static String shorts(short[] a) {
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < a.length; i++) {
            if (i > 0) {
                sb.append(' ');
            }
            sb.append(hex(a[i] & 0xFFFF));
        }
        return sb.toString();
    }

    static String bytes(byte[] a) {
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < a.length; i++) {
            if (i > 0) {
                sb.append(' ');
            }
            sb.append(hex(a[i] & 0xFF));
        }
        return sb.toString();
    }

    static String ints(int[] a) {
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < a.length; i++) {
            if (i > 0) {
                sb.append(' ');
            }
            sb.append(hex(a[i]));
        }
        return sb.toString();
    }

    static DirectGraphics dg(Image img) {
        return DirectUtils.getDirectGraphics(img.getGraphics());
    }

    static Image white(int w, int h) {
        return Image.createImage(w, h);
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
        Image img = DirectUtils.createImage(2, 2, 0);
        Graphics g = img.getGraphics();
        DirectGraphics d = DirectUtils.getDirectGraphics(g);
        System.out.println("same object: " + (d == (Object) g));
        System.out.println("cast: " + (g instanceof DirectGraphics));
        System.out.println("native format: " + d.getNativePixelFormat());
        System.out.println("transparent: " + argb(img) + " mutable " + img.isMutable());
        System.out.println("translucent: " + argb(DirectUtils.createImage(1, 1, 0x40FF0000)));

        // Int formats onto white.
        int[] four = {0x80FF0000, 0x0000FF00, 0xFF0000FF, 0};
        Image i = white(2, 2);
        dg(i).drawPixels(four, true, 0, 2, 0, 0, 2, 2, 0, DirectGraphics.TYPE_INT_8888_ARGB);
        System.out.println("8888 alpha: " + argb(i));
        i = white(2, 2);
        dg(i).drawPixels(four, false, 0, 2, 0, 0, 2, 2, 0, DirectGraphics.TYPE_INT_8888_ARGB);
        System.out.println("8888 opaque: " + argb(i));
        i = white(2, 2);
        int[] rgb = {0x12345678, 0, 0x80FFFFFF, 0x00ABCDEF};
        dg(i).drawPixels(rgb, true, 0, 2, 0, 0, 2, 2, 0, DirectGraphics.TYPE_INT_888_RGB);
        System.out.println("888: " + argb(i));

        // Short formats onto white.
        i = white(2, 2);
        short[] s4444 = {(short) 0xF800, 0x0FFF, (short) 0x8F00, (short) 0xF0F0};
        dg(i).drawPixels(s4444, true, 0, 2, 0, 0, 2, 2, 0, DirectGraphics.TYPE_USHORT_4444_ARGB);
        System.out.println("4444 alpha: " + argb(i));
        i = white(2, 2);
        dg(i).drawPixels(s4444, false, 0, 2, 0, 0, 2, 2, 0, DirectGraphics.TYPE_USHORT_4444_ARGB);
        System.out.println("4444 opaque: " + argb(i));
        i = white(2, 2);
        short[] s444 = {0x0F00, 0x00F0, 0x000F, 0x0123};
        dg(i).drawPixels(s444, true, 0, 2, 0, 0, 2, 2, 0, DirectGraphics.TYPE_USHORT_444_RGB);
        System.out.println("444: " + argb(i));
        i = white(2, 2);
        short[] s565 = {(short) 0xF800, 0x07E0, 0x001F, (short) 0x8410};
        dg(i).drawPixels(s565, true, 0, 2, 0, 0, 2, 2, 0, DirectGraphics.TYPE_USHORT_565_RGB);
        System.out.println("565: " + argb(i));
        i = white(2, 2);
        short[] s555 = {0x7C00, 0x03E0, 0x001F, 0x4210};
        dg(i).drawPixels(s555, true, 0, 2, 0, 0, 2, 2, 0, DirectGraphics.TYPE_USHORT_555_RGB);
        System.out.println("555: " + argb(i));
        i = white(2, 2);
        short[] s1555 = {(short) 0xFC00, 0x7C00, (short) 0x83E0, 0x001F};
        dg(i).drawPixels(s1555, true, 0, 2, 0, 0, 2, 2, 0, DirectGraphics.TYPE_USHORT_1555_ARGB);
        System.out.println("1555 alpha: " + argb(i));

        // Byte formats.
        i = white(8, 2);
        byte[] bits = {(byte) 0xA5, 0x0F};
        dg(i).drawPixels(bits, null, 0, 8, 0, 0, 8, 2, 0, DirectGraphics.TYPE_BYTE_1_GRAY);
        System.out.println("1 gray: " + shape(i));
        i = white(8, 2);
        Graphics red = i.getGraphics();
        red.setColor(0xFF0000);
        red.fillRect(0, 0, 8, 2);
        byte[] mask = {(byte) 0xF0, (byte) 0xFF};
        dg(i).drawPixels(bits, mask, 0, 8, 0, 0, 8, 2, 0, DirectGraphics.TYPE_BYTE_1_GRAY);
        System.out.println("1 gray masked: " + shape(i));
        i = white(2, 8);
        byte[] columns = {0x01, (byte) 0x80};
        dg(i).drawPixels(columns, null, 0, 2, 0, 0, 2, 8, 0,
                DirectGraphics.TYPE_BYTE_1_GRAY_VERTICAL);
        System.out.println("1 gray vertical: " + shape(i));
        i = white(4, 1);
        dg(i).drawPixels(new byte[] {0x1B}, null, 0, 4, 0, 0, 4, 1, 0, DirectGraphics.TYPE_BYTE_2_GRAY);
        System.out.println("2 gray: " + argb(i));
        i = white(4, 1);
        dg(i).drawPixels(new byte[] {0x0F, 0x78}, null, 0, 4, 0, 0, 4, 1, 0,
                DirectGraphics.TYPE_BYTE_4_GRAY);
        System.out.println("4 gray: " + argb(i));
        i = white(4, 1);
        dg(i).drawPixels(new byte[] {0, (byte) 0x80, (byte) 0xFF, 0x40}, null, 0, 4, 0, 0, 4, 1, 0,
                DirectGraphics.TYPE_BYTE_8_GRAY);
        System.out.println("8 gray: " + argb(i));
        i = white(4, 1);
        dg(i).drawPixels(new byte[] {(byte) 0xE0, 0x1C, 0x03, (byte) 0x92}, null, 0, 4, 0, 0, 4, 1, 0,
                DirectGraphics.TYPE_BYTE_332_RGB);
        System.out.println("332: " + argb(i));

        // Manipulations, translation and clip.
        int[] redBlue = {0xFFFF0000, 0xFF0000FF};
        i = white(2, 2);
        dg(i).drawPixels(redBlue, false, 0, 2, 0, 0, 2, 1, DirectGraphics.ROTATE_90,
                DirectGraphics.TYPE_INT_8888_ARGB);
        System.out.println("rotate 90: " + argb(i));
        i = white(2, 1);
        dg(i).drawPixels(redBlue, false, 0, 2, 0, 0, 2, 1, DirectGraphics.FLIP_HORIZONTAL,
                DirectGraphics.TYPE_INT_8888_ARGB);
        System.out.println("flip: " + argb(i));
        i = white(2, 2);
        Graphics ig = i.getGraphics();
        ig.translate(1, 1);
        DirectUtils.getDirectGraphics(ig).drawPixels(new int[] {0xFF000000}, false, 0, 1, 0, 0, 1, 1,
                0, DirectGraphics.TYPE_INT_888_RGB);
        System.out.println("translated: " + shape(i));

        // getPixels in every format.
        Image src = DirectUtils.createImage(2, 2, 0);
        int[] colours = {0xFFFF0000, 0xFF00FF00, 0xFF0000FF, 0x7FFFFFFF};
        dg(src).drawPixels(colours, true, 0, 2, 0, 0, 2, 2, 0, DirectGraphics.TYPE_INT_8888_ARGB);
        DirectGraphics sd = dg(src);
        int[] ip = new int[4];
        sd.getPixels(ip, 0, 2, 0, 0, 2, 2, DirectGraphics.TYPE_INT_8888_ARGB);
        System.out.println("get 8888: " + ints(ip));
        sd.getPixels(ip, 0, 2, 0, 0, 2, 2, DirectGraphics.TYPE_INT_888_RGB);
        System.out.println("get 888: " + ints(ip));
        int[] shortFormats = {
            DirectGraphics.TYPE_USHORT_4444_ARGB, DirectGraphics.TYPE_USHORT_565_RGB,
            DirectGraphics.TYPE_USHORT_444_RGB, DirectGraphics.TYPE_USHORT_555_RGB,
            DirectGraphics.TYPE_USHORT_1555_ARGB
        };
        for (int f = 0; f < shortFormats.length; f++) {
            short[] sp = new short[4];
            sd.getPixels(sp, 0, 2, 0, 0, 2, 2, shortFormats[f]);
            System.out.println("get " + shortFormats[f] + ": " + shorts(sp));
        }
        byte[] bp = new byte[4];
        sd.getPixels(bp, null, 0, 2, 0, 0, 2, 2, DirectGraphics.TYPE_BYTE_8_GRAY);
        System.out.println("get 8 gray: " + bytes(bp));
        sd.getPixels(bp, null, 0, 2, 0, 0, 2, 2, DirectGraphics.TYPE_BYTE_332_RGB);
        System.out.println("get 332: " + bytes(bp));
        byte[] one = new byte[1];
        byte[] oneMask = new byte[1];
        sd.getPixels(one, oneMask, 0, 2, 0, 0, 2, 2, DirectGraphics.TYPE_BYTE_1_GRAY);
        System.out.println("get 1 gray: " + bytes(one) + " mask " + bytes(oneMask));

        // The ARGB colour is the Graphics colour.
        i = white(1, 1);
        ig = i.getGraphics();
        DirectGraphics id = DirectUtils.getDirectGraphics(ig);
        id.setARGBColor(0x80000000);
        System.out.println("alpha " + id.getAlphaComponent() + " colour " + hex(ig.getColor()));
        ig.fillRect(0, 0, 1, 1);
        System.out.println("blended fill: " + argb(i));
        ig.setColor(0x123456);
        System.out.println("setColor alpha: " + id.getAlphaComponent());

        // Triangles and polygons in their own colour.
        i = white(4, 4);
        id = dg(i);
        id.fillTriangle(0, 0, 3, 0, 0, 3, 0xFF000000);
        System.out.println("fill triangle: " + shape(i));
        i = white(4, 4);
        dg(i).drawTriangle(0, 0, 3, 0, 0, 3, 0xFF000000);
        System.out.println("draw triangle: " + shape(i));
        int[] xs = {9, 0, 3, 3, 0};
        int[] ys = {9, 0, 0, 3, 3};
        i = white(5, 5);
        dg(i).fillPolygon(xs, 1, ys, 1, 4, 0xFF000000);
        System.out.println("fill polygon: " + shape(i));
        i = white(5, 5);
        dg(i).drawPolygon(xs, 1, ys, 1, 4, 0xFF000000);
        System.out.println("draw polygon: " + shape(i));

        // drawImage with a manipulation.
        Image strip = white(2, 1);
        dg(strip).drawPixels(redBlue, false, 0, 2, 0, 0, 2, 1, 0, DirectGraphics.TYPE_INT_8888_ARGB);
        i = white(2, 2);
        dg(i).drawImage(strip, 0, 0, Graphics.TOP | Graphics.LEFT, DirectGraphics.ROTATE_90);
        System.out.println("image rotate 90: " + argb(i));
        i = white(2, 2);
        dg(i).drawImage(strip, 2, 2, Graphics.BOTTOM | Graphics.RIGHT,
                DirectGraphics.FLIP_VERTICAL | DirectGraphics.ROTATE_180);
        System.out.println("image flips: " + argb(i));

        // DirectUtils.createImage from bytes is mutable.
        InputStream in = NokiaGraphics.class.getResourceAsStream("/dot.png");
        byte[] png = new byte[1024];
        int n = in.read(png);
        Image decoded = DirectUtils.createImage(png, 0, n);
        System.out.println("decoded: " + argb(decoded) + " mutable " + decoded.isMutable());
        decoded.getGraphics().fillRect(0, 0, 1, 1);
        System.out.println("drawn on: " + argb(decoded));

        // Exceptions.
        final DirectGraphics fd = dg(white(2, 2));
        final Image fstrip = strip;
        final byte[] fpng = png;
        expect("size 0", new Call() {
            public void run() {
                DirectUtils.createImage(0, 5, 0);
            }
        });
        expect("junk bytes", new Call() {
            public void run() {
                DirectUtils.createImage(new byte[] {1, 2, 3}, 0, 3);
            }
        });
        expect("bad range", new Call() {
            public void run() {
                DirectUtils.createImage(fpng, -1, 2);
            }
        });
        expect("null bytes", new Call() {
            public void run() {
                DirectUtils.createImage(null, 0, 2);
            }
        });
        expect("int pixels as 565", new Call() {
            public void run() {
                fd.drawPixels(new int[4], false, 0, 2, 0, 0, 2, 2, 0, DirectGraphics.TYPE_USHORT_565_RGB);
            }
        });
        expect("short array too small", new Call() {
            public void run() {
                fd.drawPixels(new short[3], false, 0, 2, 0, 0, 2, 2, 0,
                        DirectGraphics.TYPE_USHORT_565_RGB);
            }
        });
        expect("manipulation 45", new Call() {
            public void run() {
                fd.drawPixels(new int[4], false, 0, 2, 0, 0, 2, 2, 45, DirectGraphics.TYPE_INT_888_RGB);
            }
        });
        expect("null pixels", new Call() {
            public void run() {
                fd.drawPixels((int[]) null, false, 0, 2, 0, 0, 2, 2, 0, DirectGraphics.TYPE_INT_888_RGB);
            }
        });
        expect("negative getPixels", new Call() {
            public void run() {
                fd.getPixels(new int[4], 0, 2, -1, 0, 2, 2, DirectGraphics.TYPE_INT_888_RGB);
            }
        });
        expect("null polygon", new Call() {
            public void run() {
                fd.fillPolygon(null, 0, new int[3], 0, 3, 0);
            }
        });
        expect("short polygon", new Call() {
            public void run() {
                fd.drawPolygon(new int[2], 0, new int[3], 0, 3, 0);
            }
        });
        expect("baseline image", new Call() {
            public void run() {
                fd.drawImage(fstrip, 0, 0, Graphics.BASELINE | Graphics.LEFT, 0);
            }
        });
        expect("null image", new Call() {
            public void run() {
                fd.drawImage(null, 0, 0, 0, 0);
            }
        });
        System.out.println("done");
    }
}
