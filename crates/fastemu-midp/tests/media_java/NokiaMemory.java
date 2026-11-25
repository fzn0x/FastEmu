import java.io.ByteArrayInputStream;
import java.util.Vector;

import com.nokia.mid.sound.Sound;
import javax.microedition.media.Manager;
import javax.microedition.media.Player;

/**
 * Nokia's Sound makes players in the game's sound group, so its limit of
 * sound memory is the same as MMAPI's.
 */
public class NokiaMemory extends MediaUtil {
    /** A file of one-byte AMR frames, which decodes to 320 times its size. */
    static byte[] emptyAmr(int frames) {
        byte[] amr = new byte[6 + frames];
        byte[] magic = {'#', '!', 'A', 'M', 'R', 10};
        for (int i = 0; i < magic.length; i++) {
            amr[i] = magic[i];
        }
        for (int i = 6; i < amr.length; i++) {
            amr[i] = 0x7C;
        }
        return amr;
    }

    public static void main(String[] args) throws Exception {
        // A sound that decodes to too much is no sound, as one that does
        // not decode, and it takes no room in the group.
        Sound huge = new Sound(emptyAmr(100000), Sound.FORMAT_WAV);
        say("a too large sound", huge.getState());
        byte[] sound = wav(1 << 20);
        Vector sounds = new Vector();
        try {
            for (int i = 0; i < 40; i++) {
                sounds.addElement(new Sound(sound, Sound.FORMAT_WAV));
            }
        } catch (OutOfMemoryError e) {
            threw("the 33rd sound", e);
        }
        say("sounds held", sounds.size());
        say("state of the first", ((Sound) sounds.elementAt(0)).getState());
        // Releasing one makes room for one.
        ((Sound) sounds.elementAt(0)).release();
        Sound again = new Sound(sound, Sound.FORMAT_WAV);
        say("room after a release", again.getState());
        // MMAPI players are in the same group: nothing is left for them.
        try {
            Player p = Manager.createPlayer(new ByteArrayInputStream(sound), "audio/x-wav");
            p.realize();
            say("an MMAPI player", true);
        } catch (OutOfMemoryError e) {
            threw("an MMAPI player", e);
        }
        System.out.println("done");
    }
}
