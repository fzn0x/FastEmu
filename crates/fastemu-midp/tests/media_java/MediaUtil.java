import java.io.IOException;
import java.io.InputStream;

import javax.microedition.media.Manager;
import javax.microedition.media.MediaException;
import javax.microedition.media.Player;

/**
 * Printing and loading helpers the media test programs share; the
 * programs extend it to call them unqualified.
 */
public class MediaUtil {
    protected MediaUtil() {
    }

    public static void say(String what, long value) {
        System.out.println(what + ": " + value);
    }

    public static void say(String what, boolean value) {
        System.out.println(what + ": " + value);
    }

    public static void say(String what, String value) {
        System.out.println(what + ": " + value);
    }

    /** Prints the class of an exception a step threw. */
    public static void threw(String what, Throwable t) {
        System.out.println(what + ": " + t.getClass().getName());
    }

    public static InputStream resource(String name) {
        return MediaUtil.class.getResourceAsStream(name);
    }

    /** The bytes of a resource. */
    public static byte[] bytes(String name) throws IOException {
        InputStream in = resource(name);
        byte[] data = new byte[in.available()];
        int at = 0;
        while (at < data.length) {
            int n = in.read(data, at, data.length - at);
            if (n < 0) {
                break;
            }
            at += n;
        }
        return data;
    }

    public static Player player(String name, String type) throws IOException, MediaException {
        return Manager.createPlayer(resource(name), type);
    }

    public static String join(String[] items) {
        StringBuffer out = new StringBuffer();
        for (int i = 0; i < items.length; i++) {
            if (i > 0) {
                out.append(' ');
            }
            out.append(items[i]);
        }
        return out.toString();
    }

    /** A mono 16-bit WAV at 8 kHz holding {@code data} bytes of silence. */
    public static byte[] wav(int data) {
        byte[] out = new byte[44 + data];
        text(out, 0, "RIFF");
        little(out, 4, 36 + data, 4);
        text(out, 8, "WAVEfmt ");
        little(out, 16, 16, 4);
        little(out, 20, 1, 2);
        little(out, 22, 1, 2);
        little(out, 24, 8000, 4);
        little(out, 28, 16000, 4);
        little(out, 32, 2, 2);
        little(out, 34, 16, 2);
        text(out, 36, "data");
        little(out, 40, data, 4);
        return out;
    }

    private static void text(byte[] out, int at, String text) {
        for (int i = 0; i < text.length(); i++) {
            out[at + i] = (byte) text.charAt(i);
        }
    }

    private static void little(byte[] out, int at, int value, int size) {
        for (int i = 0; i < size; i++) {
            out[at + i] = (byte) (value >> (8 * i));
        }
    }
}
