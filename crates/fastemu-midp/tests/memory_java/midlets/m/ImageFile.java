package m;

import java.io.IOException;
import java.io.InputStream;
import javax.microedition.lcdui.Image;
import javax.microedition.midlet.MIDlet;

/**
 * Loads image files whose headers declare 16384 by 2048 pixels (a PNG of
 * a few hundred bytes, a BMP of 54), a GIF of 34 bytes whose first frame
 * is 8192 by 4096 pixels on a screen of one, by resource name, from a
 * byte array, from a byte array through Nokia's DirectUtils and from a
 * stream, and then a real 2 by 2 PNG the same four ways.
 */
public class ImageFile extends MIDlet {
    private static byte[] read(String name) throws IOException {
        InputStream in = ImageFile.class.getResourceAsStream(name);
        byte[] data = new byte[4096];
        int n = 0;
        while (true) {
            if (n == data.length) {
                // A file larger than the buffer: read on into a bigger one.
                byte[] bigger = new byte[n * 2];
                System.arraycopy(data, 0, bigger, 0, n);
                data = bigger;
            }
            int got = in.read(data, n, data.length - n);
            if (got < 0) {
                break;
            }
            n += got;
        }
        byte[] exact = new byte[n];
        System.arraycopy(data, 0, exact, 0, n);
        return exact;
    }

    private static String outcome(Image image) {
        return "ok " + image.getWidth() + "x" + image.getHeight();
    }

    private static void load(String file) {
        String result;
        try {
            result = outcome(Image.createImage(file));
        } catch (OutOfMemoryError e) {
            result = "OutOfMemoryError";
        } catch (IOException e) {
            result = "IOException";
        }
        System.out.println(file + " by name " + result);
        try {
            byte[] data = read(file);
            result = outcome(Image.createImage(data, 0, data.length));
        } catch (OutOfMemoryError e) {
            result = "OutOfMemoryError";
        } catch (IOException e) {
            result = "IOException";
        } catch (IllegalArgumentException e) {
            result = "IllegalArgumentException";
        }
        System.out.println(file + " by bytes " + result);
        try {
            byte[] data = read(file);
            result = outcome(com.nokia.mid.ui.DirectUtils.createImage(data, 0, data.length));
        } catch (OutOfMemoryError e) {
            result = "OutOfMemoryError";
        } catch (IOException e) {
            result = "IOException";
        } catch (IllegalArgumentException e) {
            result = "IllegalArgumentException";
        }
        System.out.println(file + " by nokia bytes " + result);
        try {
            result = outcome(Image.createImage(ImageFile.class.getResourceAsStream(file)));
        } catch (OutOfMemoryError e) {
            result = "OutOfMemoryError";
        } catch (IOException e) {
            result = "IOException";
        }
        System.out.println(file + " by stream " + result);
    }

    protected void startApp() {
        load("/bomb.png");
        load("/bomb.bmp");
        load("/bomb.gif");
        load("/dot.png");
        System.out.println("done");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
