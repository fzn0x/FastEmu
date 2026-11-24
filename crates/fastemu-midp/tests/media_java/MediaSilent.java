import javax.microedition.media.Manager;
import javax.microedition.media.Player;
import javax.microedition.media.control.ToneControl;
import javax.microedition.media.control.VolumeControl;

/**
 * An instance with no audio engine: players go through their states and
 * report the real lengths and times, on the wall clock, in silence.
 */
public class MediaSilent extends MediaUtil {
    public static void main(String[] args) throws Exception {
        Player p = player("/beep.wav", "audio/x-wav");
        MediaRecorder r = new MediaRecorder();
        p.addPlayerListener(r);
        p.realize();
        say("type", p.getContentType());
        say("duration", p.getDuration());
        p.start();
        say("state after start", p.getState());
        long begun = System.currentTimeMillis();
        say("events", r.waitFor(2, 5000));
        long took = System.currentTimeMillis() - begun;
        say("took about the length", took > 100 && took < 2000);
        say("state at the end", p.getState());
        say("time at the end", p.getMediaTime());

        p.setLoopCount(2);
        p.start();
        say("two passes", r.waitFor(6, 5000));
        p.start();
        Thread.sleep(50);
        p.stop();
        long stoppedAt = p.getMediaTime();
        say("stopped inside the media", stoppedAt > 0 && stoppedAt < 200000);
        say("stop event", r.waitForEvent("stopped", 5000));
        p.setMediaTime(120000);
        say("seek", p.getMediaTime());

        VolumeControl vc = (VolumeControl) p.getControl("VolumeControl");
        vc.setLevel(30);
        say("volume event", r.waitForEvent("volumeChanged", 5000));
        say("level", vc.getLevel());
        p.close();
        say("closed", r.waitForEvent("closed", 5000));

        Player t = Manager.createPlayer(Manager.TONE_DEVICE_LOCATOR);
        t.realize();
        ToneControl tc = (ToneControl) t.getControl("ToneControl");
        tc.setSequence(MediaTones.SEQUENCE);
        say("tone duration", t.getDuration());
        try {
            tc.setSequence(new byte[] {ToneControl.VERSION, 1, ToneControl.TEMPO, 1});
        } catch (IllegalArgumentException e) {
            threw("bad tempo", e);
        }
        t.close();

        Manager.playTone(69, 100, 80);
        System.out.println("done");
    }
}
