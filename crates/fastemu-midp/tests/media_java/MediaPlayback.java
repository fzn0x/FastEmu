import javax.microedition.media.Player;
import javax.microedition.media.control.VolumeControl;

/** Looping, media time and volume, heard through the engine. */
public class MediaPlayback extends MediaUtil {
    public static void main(String[] args) throws Exception {
        // Three passes: each ends with END_OF_MEDIA, and the next begins
        // with STARTED at 0.
        Player p = player("/beep.wav", "audio/x-wav");
        MediaRecorder r = new MediaRecorder();
        p.addPlayerListener(r);
        p.setLoopCount(3);
        p.start();
        Pump.advance(700);
        say("three passes", r.waitFor(6, 5000));
        say("state after the passes", p.getState());
        p.close();

        // Endless loops run until stopped.
        Player forever = player("/beep.wav", "audio/x-wav");
        MediaRecorder rf = new MediaRecorder();
        forever.addPlayerListener(rf);
        forever.realize();
        forever.setLoopCount(-1);
        forever.start();
        Pump.advance(1050);
        rf.waitFor(11, 5000);
        say("endless passes", rf.count("endOfMedia"));
        say("still playing", forever.getState());
        say("time in the sixth pass", forever.getMediaTime());
        forever.stop();
        say("stopped endless", rf.waitForEvent("stopped", 5000));
        forever.close();

        // Media time follows the mix and seeks clamp to the media.
        Player t = player("/long.wav", "audio/x-wav");
        t.start();
        Pump.advance(100);
        say("time after 100 ms", t.getMediaTime());
        say("seek", t.setMediaTime(150000));
        say("time after seek", t.getMediaTime());
        Pump.advance(50);
        say("time 50 ms later", t.getMediaTime());
        say("seek before the start", t.setMediaTime(-5));
        say("seek past the end", t.setMediaTime(1000000000L));
        t.stop();
        say("seek while stopped", t.setMediaTime(250000));
        say("time while stopped", t.getMediaTime());
        t.close();

        // Volume: level and mute are heard.
        Player v = player("/long.wav", "audio/x-wav");
        v.realize();
        VolumeControl vc = (VolumeControl) v.getControl("VolumeControl");
        say("level", vc.getLevel());
        say("set 50", vc.setLevel(50));
        say("set 150", vc.setLevel(150));
        say("set -3", vc.setLevel(-3));
        say("set 80", vc.setLevel(80));
        v.start();
        Pump.advance(20);
        say("heard at 80", Pump.peak(50) > 0);
        vc.setMute(true);
        say("muted", vc.isMuted());
        say("level kept while muted", vc.getLevel());
        Pump.advance(20);
        say("heard while muted", Pump.peak(50) > 0);
        vc.setMute(false);
        Pump.advance(20);
        int loud = Pump.peak(50);
        vc.setLevel(20);
        Pump.advance(20);
        int quiet = Pump.peak(50);
        say("level 20 is quieter than 80", quiet > 0 && quiet < loud);
        v.close();
        System.out.println("done");
    }
}
