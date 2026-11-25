import java.io.ByteArrayInputStream;
import java.io.InputStream;
import java.util.Vector;

import javax.microedition.media.Manager;
import javax.microedition.media.MediaException;
import javax.microedition.media.Player;

/**
 * What a game's sound costs: the bytes a player keeps count on the heap
 * until it realizes, and one game holds only so much sound and so many
 * players, whatever the heap says.
 */
public class MediaMemory extends MediaUtil {
    /** One mebibyte of sound: about the size of its decoded samples too. */
    static final int ONE_MIB = 1 << 20;

    /** A stream that never ends and never writes more than zeros. */
    static class Endless extends InputStream {
        public int read() {
            return 0;
        }

        public int read(byte[] b, int off, int len) {
            return len;
        }
    }

    static Player make(byte[] sound, String type) throws Exception {
        return Manager.createPlayer(new ByteArrayInputStream(sound), type);
    }

    static long free() {
        return Runtime.getRuntime().freeMemory();
    }

    /** The heap counts a player's bytes while it keeps them, and no more. */
    static void bytesOnTheHeap() throws Exception {
        byte[] sound = wav(ONE_MIB);
        System.gc();
        long base = free();
        Player p = make(sound, "audio/x-wav");
        say("source counted on the heap", base - free() >= sound.length);
        p.realize();
        say("source dropped after realize", base - free() < sound.length / 8);
        p.close();
        long before = free();
        Player q = make(sound, "audio/x-wav");
        say("source counted again", before - free() >= sound.length);
        q.close();
        say("source dropped on close", before - free() < sound.length / 8);
    }

    /** 40 players of one mebibyte, kept: the 32 MiB budget ends it. */
    static void soundBudget() throws Exception {
        byte[] sound = wav(ONE_MIB);
        Vector players = new Vector();
        try {
            for (int i = 0; i < 40; i++) {
                Player p = make(sound, "audio/x-wav");
                p.realize();
                players.addElement(p);
            }
        } catch (OutOfMemoryError e) {
            threw("the 33rd sound", e);
        }
        say("players held by the sound budget", players.size());
        // Closing one makes room for one.
        ((Player) players.elementAt(0)).close();
        Player again = make(sound, "audio/x-wav");
        again.realize();
        say("room after a close", again.getState());
        try {
            make(sound, "audio/x-wav").realize();
        } catch (OutOfMemoryError e) {
            threw("full again", e);
        }
    }

    /** Players the game forgot are collected for room, as on a phone. */
    static void forgotten() throws Exception {
        byte[] sound = wav(ONE_MIB);
        for (int i = 0; i < 100; i++) {
            make(sound, "audio/x-wav").realize();
        }
        say("forgotten players make room", true);
    }

    /** Small sounds, kept: the limit of 128 players ends it. */
    static void playerCount() throws Exception {
        byte[] sound = wav(200);
        Vector players = new Vector();
        try {
            for (int i = 0; i < 200; i++) {
                Player p = make(sound, "audio/x-wav");
                p.realize();
                players.addElement(p);
            }
        } catch (OutOfMemoryError e) {
            threw("the 129th player", e);
        }
        say("players held by the count", players.size());
        // Tone players count too.
        try {
            Manager.createPlayer(Manager.TONE_DEVICE_LOCATOR).realize();
        } catch (OutOfMemoryError e) {
            threw("a tone player", e);
        }
        // A tone from Manager.playTone is not a player.
        Manager.playTone(69, 100, 100);
        say("playTone still works", true);
    }

    /** A file of one-byte AMR frames, which decodes to 320 times its size. */
    static void amrOfEmptyFrames() throws Exception {
        byte[] amr = new byte[6 + 100000];
        byte[] magic = {'#', '!', 'A', 'M', 'R', '\n'};
        for (int i = 0; i < magic.length; i++) {
            amr[i] = magic[i];
        }
        for (int i = 6; i < amr.length; i++) {
            amr[i] = 0x7C;
        }
        Player p = make(amr, "audio/amr");
        say("amr state", p.getState());
        try {
            p.realize();
            say("amr realized", true);
        } catch (MediaException e) {
            threw("amr of empty frames", e);
        }
        try {
            p.start();
        } catch (MediaException e) {
            threw("amr started", e);
        }
        say("amr state after", p.getState());
        p.close();
    }

    /** A stream that never ends stops at the heap. */
    static void endlessStream() throws Exception {
        try {
            Manager.createPlayer(new Endless(), "audio/x-wav");
            say("endless stream", true);
        } catch (OutOfMemoryError e) {
            threw("endless stream", e);
        }
    }

    public static void main(String[] args) throws Exception {
        bytesOnTheHeap();
        System.gc();
        soundBudget();
        System.gc();
        forgotten();
        System.gc();
        playerCount();
        System.gc();
        amrOfEmptyFrames();
        endlessStream();
        System.out.println("done");
    }
}
