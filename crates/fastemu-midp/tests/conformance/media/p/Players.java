package p;

import javax.microedition.midlet.MIDlet;

/**
 * Plays every sound format FastEmu knows, and data it does not, from a
 * stream with the right content type and with none, and from two
 * locators; all at once, then prints each player's content type,
 * duration and listener events.
 */
public class Players extends MIDlet {
    private static final String[][] STREAMS = {
        {"beep.wav", "audio/x-wav"},
        {"beep.wav", null},
        {"tune.mid", "audio/midi"},
        {"tune.mid", null},
        {"tone.jts", "audio/x-tone-seq"},
        {"tone.jts", null},
        {"sine.mp3", "audio/mpeg"},
        {"sine.mp3", null},
        {"speech.amr", "audio/amr"},
        {"speech.amr", null},
        {"junk.bin", "audio/x-unknown"},
        {"junk.bin", null},
    };
    private static final String[] LOCATORS = {"resource:/beep.wav", "/tune.mid"};

    public void startApp() {
        Track[] tracks = new Track[STREAMS.length + LOCATORS.length];
        for (int i = 0; i < STREAMS.length; i++) {
            tracks[i] = Track.fromStream(STREAMS[i][0], STREAMS[i][1]);
        }
        for (int i = 0; i < LOCATORS.length; i++) {
            tracks[STREAMS.length + i] = Track.fromLocator(LOCATORS[i]);
        }
        for (int i = 0; i < tracks.length; i++) {
            tracks[i].start();
        }
        for (int i = 0; i < tracks.length; i++) {
            tracks[i].await("endOfMedia");
            tracks[i].close();
        }
        for (int i = 0; i < tracks.length; i++) {
            tracks[i].await("closed");
            System.out.println("player " + tracks[i].summary());
        }
        System.out.println("players done");
        notifyDestroyed();
    }

    public void pauseApp() {
    }

    public void destroyApp(boolean unconditional) {
    }
}
