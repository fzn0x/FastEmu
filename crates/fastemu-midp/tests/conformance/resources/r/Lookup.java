package r;

import java.io.IOException;
import java.io.InputStream;
import javax.microedition.lcdui.Image;
import javax.microedition.midlet.MIDlet;

/**
 * Looks resources up by every kind of name games use (relative, absolute,
 * from the default package, with doubled slashes, backslashes, dot
 * segments and the wrong case) and prints what each finds, then reads a
 * large deflated resource with one read call, as games do.
 */
public class Lookup extends MIDlet {
    public void startApp() {
        show("near.txt", Lookup.class.getResourceAsStream("near.txt"));
        show("sub/deep.txt", Lookup.class.getResourceAsStream("sub/deep.txt"));
        show("/top.txt", Lookup.class.getResourceAsStream("/top.txt"));
        show("top.txt from the default package", fromTop("top.txt"));
        show("//top.txt", Lookup.class.getResourceAsStream("//top.txt"));
        show("/r//sub///deep.txt", Lookup.class.getResourceAsStream("/r//sub///deep.txt"));
        show("/r\\sub\\deep.txt", Lookup.class.getResourceAsStream("/r\\sub\\deep.txt"));
        show("/./r/near.txt", Lookup.class.getResourceAsStream("/./r/near.txt"));
        show("/TOP.TXT", Lookup.class.getResourceAsStream("/TOP.TXT"));
        show("/mixed.txt", Lookup.class.getResourceAsStream("/mixed.txt"));
        show("/res/top.txt", Lookup.class.getResourceAsStream("/res/top.txt"));
        show("/missing.txt", Lookup.class.getResourceAsStream("/missing.txt"));
        show("an object's class", new Probe().getClass().getResourceAsStream("near.txt"));
        try {
            big();
        } catch (IOException e) {
            System.out.println("big failed: " + e);
        }
        image("img/dot.png");
        image("/img/dot.png");
        image("/img/none.png");
        System.out.println("lookup done");
        notifyDestroyed();
    }

    public void pauseApp() {
    }

    public void destroyApp(boolean unconditional) {
    }

    /** The whole stream as text, one char per byte. */
    static String text(InputStream in) {
        StringBuffer out = new StringBuffer();
        try {
            int b;
            while ((b = in.read()) >= 0) {
                out.append((char) b);
            }
            in.close();
        } catch (IOException e) {
            out.append("<" + e + ">");
        }
        return out.toString();
    }

    /** Looks `name` up from the class Top, in the default package. */
    static InputStream fromTop(String name) {
        try {
            Finder top = (Finder) Class.forName("Top").newInstance();
            return top.find(name);
        } catch (Exception e) {
            System.out.println("no Top: " + e);
            return null;
        }
    }

    static void show(String name, InputStream in) {
        System.out.println("res " + name + " = " + (in == null ? "null" : text(in)));
    }

    static int hash(byte[] data, int length) {
        int h = 0;
        for (int i = 0; i < length; i++) {
            h = 31 * h + (data[i] & 0xFF);
        }
        return h;
    }

    /** One read of the whole resource, then skip, mark and reset. */
    static void big() throws IOException {
        InputStream in = Lookup.class.getResourceAsStream("/big.bin");
        int available = in.available();
        byte[] all = new byte[available];
        int read = in.read(all);
        int end = in.read();
        in.close();
        System.out.println("big available " + available + " read " + read + " then " + end
                + " hash " + hash(all, read));
        InputStream again = Lookup.class.getResourceAsStream("/big.bin");
        long skipped = again.skip(1000);
        int at = again.read();
        again.mark(64);
        byte[] first = new byte[10];
        byte[] second = new byte[10];
        int one = again.read(first);
        again.reset();
        int two = again.read(second);
        boolean same = one == 10 && two == 10 && hash(first, 10) == hash(second, 10);
        System.out.println("big skip " + skipped + " byte " + at + " mark " + again.markSupported()
                + " reset " + same + " left " + again.available());
        again.close();
    }

    static void image(String name) {
        try {
            Image image = Image.createImage(name);
            System.out.println("image " + name + " " + image.getWidth() + "x" + image.getHeight());
        } catch (IOException e) {
            System.out.println("image " + name + " IOException");
        }
    }
}
