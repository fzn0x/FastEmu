import javax.microedition.media.Control;
import javax.microedition.media.Manager;
import javax.microedition.media.Player;
import javax.microedition.media.TimeBase;

/** Every state transition of a player and the exceptions around them. */
public class MediaStates extends MediaUtil {
    public static void main(String[] args) throws Exception {
        Player p = player("/beep.wav", "audio/x-wav");
        say("created", p.getState());
        say("duration unrealized", p.getDuration());
        say("time unrealized", p.getMediaTime());
        try {
            p.getContentType();
        } catch (IllegalStateException e) {
            threw("content type unrealized", e);
        }
        try {
            p.setMediaTime(0);
        } catch (IllegalStateException e) {
            threw("seek unrealized", e);
        }
        try {
            p.getControl("VolumeControl");
        } catch (IllegalStateException e) {
            threw("control unrealized", e);
        }
        p.setLoopCount(2);
        p.setLoopCount(1);
        p.realize();
        say("realized", p.getState());
        say("content type", p.getContentType());
        say("duration", p.getDuration());
        p.realize();
        say("realized again", p.getState());
        p.prefetch();
        say("prefetched", p.getState());
        p.start();
        say("started", p.getState());
        p.start();
        say("started again", p.getState());
        try {
            p.setLoopCount(3);
        } catch (IllegalStateException e) {
            threw("loop count while started", e);
        }
        p.stop();
        say("stopped", p.getState());
        p.stop();
        say("stopped again", p.getState());
        p.deallocate();
        say("deallocated", p.getState());
        p.start();
        say("started from realized", p.getState());
        p.deallocate();
        say("deallocated while started", p.getState());
        try {
            p.setLoopCount(0);
        } catch (IllegalArgumentException e) {
            threw("loop count 0", e);
        }
        try {
            p.getControl(null);
        } catch (IllegalArgumentException e) {
            threw("null control", e);
        }
        Control volume = p.getControl("VolumeControl");
        say("volume control", volume != null);
        say("same control by full name",
                volume == p.getControl("javax.microedition.media.control.VolumeControl"));
        say("video control", p.getControl("VideoControl") == null ? "null" : "some");
        say("controls", p.getControls().length);
        p.close();
        say("closed", p.getState());
        p.close();
        say("closed again", p.getState());
        try {
            p.realize();
        } catch (IllegalStateException e) {
            threw("realize closed", e);
        }
        try {
            p.prefetch();
        } catch (IllegalStateException e) {
            threw("prefetch closed", e);
        }
        try {
            p.start();
        } catch (IllegalStateException e) {
            threw("start closed", e);
        }
        try {
            p.stop();
        } catch (IllegalStateException e) {
            threw("stop closed", e);
        }
        try {
            p.deallocate();
        } catch (IllegalStateException e) {
            threw("deallocate closed", e);
        }
        try {
            p.getMediaTime();
        } catch (IllegalStateException e) {
            threw("time closed", e);
        }
        try {
            p.getDuration();
        } catch (IllegalStateException e) {
            threw("duration closed", e);
        }
        try {
            p.setLoopCount(2);
        } catch (IllegalStateException e) {
            threw("loop count closed", e);
        }
        try {
            p.addPlayerListener(new MediaRecorder());
        } catch (IllegalStateException e) {
            threw("listener closed", e);
        }
        try {
            p.getControls();
        } catch (IllegalStateException e) {
            threw("controls closed", e);
        }

        try {
            Manager.createPlayer(null, "audio/x-wav");
        } catch (IllegalArgumentException e) {
            threw("null stream", e);
        }
        try {
            Manager.createPlayer((String) null);
        } catch (IllegalArgumentException e) {
            threw("null locator", e);
        }
        try {
            Manager.createPlayer("capture://audio");
        } catch (Exception e) {
            threw("capture locator", e);
        }
        // Live MIDI plays nothing, but the player and its MIDIControl work.
        Player midiDevice = Manager.createPlayer(Manager.MIDI_DEVICE_LOCATOR);
        midiDevice.prefetch();
        say("midi device type", midiDevice.getContentType());
        javax.microedition.media.control.MIDIControl live =
            (javax.microedition.media.control.MIDIControl) midiDevice.getControl("MIDIControl");
        live.shortMidiEvent(0x90, 60, 100);
        say("midi device takes events", live != null);
        midiDevice.close();
        try {
            Manager.createPlayer("/missing.wav");
        } catch (Exception e) {
            threw("missing resource", e);
        }

        Player byLocator = Manager.createPlayer("resource:///beep.wav");
        byLocator.realize();
        say("resource locator", byLocator.getContentType());
        Player bare = Manager.createPlayer("beep.wav");
        bare.realize();
        say("bare locator duration", bare.getDuration());
        Player web = Manager.createPlayer("http://example.com/music.mid");
        web.realize();
        say("network locator", web.getContentType());

        TimeBase base = Manager.getSystemTimeBase();
        long t1 = base.getTime();
        long t2 = base.getTime();
        say("time base runs forward", t1 >= 0 && t2 >= t1);
        say("player time base", byLocator.getTimeBase() != null);
        byLocator.setTimeBase(null);
        byLocator.setTimeBase(base);
        try {
            byLocator.setTimeBase(new TimeBase() {
                public long getTime() {
                    return 0;
                }
            });
        } catch (Exception e) {
            threw("own time base", e);
        }

        say("types", join(Manager.getSupportedContentTypes(null)));
        say("device types", join(Manager.getSupportedContentTypes("device")));
        say("rtsp types", Manager.getSupportedContentTypes("rtsp").length);
        say("protocols", join(Manager.getSupportedProtocols(null)));
        say("tone protocols", join(Manager.getSupportedProtocols("audio/x-tone-seq")));
        System.out.println("done");
    }
}
