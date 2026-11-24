import java.io.ByteArrayInputStream;

import javax.microedition.media.Manager;
import javax.microedition.media.Player;
import javax.microedition.media.control.ToneControl;

/** Tone sequences through ToneControl and files, and Manager.playTone. */
public class MediaTones extends MediaUtil {
    /** C4 and a rest, an eighth note each at 120 bpm: 500 ms in all. */
    static final byte[] SEQUENCE = {
        ToneControl.VERSION, 1,
        ToneControl.TEMPO, 30,
        ToneControl.C4, 8,
        ToneControl.SILENCE, 8,
    };

    public static void main(String[] args) throws Exception {
        Player t = Manager.createPlayer(Manager.TONE_DEVICE_LOCATOR);
        MediaRecorder r = new MediaRecorder();
        t.addPlayerListener(r);
        t.realize();
        say("tone player type", t.getContentType());
        say("empty duration", t.getDuration());
        ToneControl tc = (ToneControl) t.getControl("ToneControl");
        say("tone control", tc != null);
        try {
            tc.setSequence(null);
        } catch (IllegalArgumentException e) {
            threw("null sequence", e);
        }
        try {
            tc.setSequence(new byte[] {ToneControl.VERSION, 1, ToneControl.TEMPO, 1});
        } catch (IllegalArgumentException e) {
            threw("bad tempo", e);
        }
        tc.setSequence(SEQUENCE);
        say("duration", t.getDuration());
        t.start();
        Pump.advance(10);
        say("heard", Pump.peak(100) > 0);
        Pump.advance(500);
        say("events", r.waitFor(2, 5000));
        try {
            tc.setSequence(SEQUENCE);
        } catch (IllegalStateException e) {
            threw("sequence while prefetched", e);
        }
        t.deallocate();
        tc.setSequence(new byte[] {ToneControl.VERSION, 1, ToneControl.C4, 16});
        say("new duration", t.getDuration());
        t.close();

        // The same format from a stream.
        Player s = Manager.createPlayer(new ByteArrayInputStream(SEQUENCE), "audio/x-tone-seq");
        s.realize();
        say("stream duration", s.getDuration());
        say("stream tone control", s.getControl("ToneControl") != null);
        say("stream controls", s.getControls().length);
        s.close();

        // A tone stream that does not parse plays as silence, and its
        // ToneControl still takes a good sequence.
        byte[] broken = {ToneControl.VERSION, 1, ToneControl.TEMPO, 1};
        Player b = Manager.createPlayer(new ByteArrayInputStream(broken), "audio/x-tone-seq");
        b.realize();
        ToneControl btc = (ToneControl) b.getControl("ToneControl");
        btc.setSequence(SEQUENCE);
        say("silent tone player takes a sequence", b.getDuration());
        b.close();

        Manager.playTone(69, 100, 80);
        say("playTone heard", Pump.peak(50) > 0);
        Pump.advance(100);
        Manager.playTone(60, 100, 500);
        Manager.playTone(60, 100, -20);
        try {
            Manager.playTone(128, 100, 50);
        } catch (IllegalArgumentException e) {
            threw("note 128", e);
        }
        try {
            Manager.playTone(-1, 100, 50);
        } catch (IllegalArgumentException e) {
            threw("note -1", e);
        }
        try {
            Manager.playTone(60, 0, 50);
        } catch (IllegalArgumentException e) {
            threw("duration 0", e);
        }
        System.out.println("done");
    }
}
