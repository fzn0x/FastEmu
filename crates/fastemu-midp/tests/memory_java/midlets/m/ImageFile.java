package m;

import java.io.IOException;
import java.io.InputStream;
import javax.microedition.lcdui.Image;
import javax.microedition.midlet.MIDlet;

/**
 * Loads image files whose headers declare 16384 by 2048 pixels (a PNG of
 * a few hundred bytes, a BMP of 54), by resource name, from a byte array
 * and from a stream, and then a real 2 by 2 PNG the same three ways.
 */
public class ImageFile extends MIDlet {
    private static byte[] read(String name) throws IOException {
        InputStream in = ImageFile.class.getResourceAsStream(name);
        byte[] data = new byte[4096];
        int n = 0;
        while (true) {
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
        load("/dot.png");
        System.out.println("done");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
