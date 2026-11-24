import javax.microedition.media.MediaException;
import javax.microedition.media.Player;
import javax.microedition.media.control.MIDIControl;
import javax.microedition.media.control.StopTimeControl;

/** MIDIControl on a MIDI player, and StopTimeControl. */
public class MediaControls extends MediaUtil {
    public static void main(String[] args) throws Exception {
        Player m = player("/tune.mid", "audio/midi");
        m.realize();
        say("midi controls", m.getControls().length);
        MIDIControl mc = (MIDIControl) m.getControl("MIDIControl");
        say("midi control", mc != null);
        try {
            mc.shortMidiEvent(MIDIControl.NOTE_ON, 60, 100);
        } catch (IllegalStateException e) {
            threw("event before prefetch", e);
        }
        m.prefetch();
        mc.shortMidiEvent(MIDIControl.NOTE_ON, 60, 100);
        mc.shortMidiEvent(MIDIControl.CONTROL_CHANGE, 7, 90);
        try {
            mc.shortMidiEvent(0x70, 60, 100);
        } catch (IllegalArgumentException e) {
            threw("status 0x70", e);
        }
        try {
            mc.shortMidiEvent(0xF0, 0, 0);
        } catch (IllegalArgumentException e) {
            threw("system exclusive", e);
        }
        try {
            mc.shortMidiEvent(0x90, 128, 0);
        } catch (IllegalArgumentException e) {
            threw("data 128", e);
        }
        say("bank query", mc.isBankQuerySupported());
        try {
            mc.getProgram(0);
        } catch (MediaException e) {
            threw("program query", e);
        }
        try {
            mc.getProgram(16);
        } catch (IllegalArgumentException e) {
            threw("channel 16", e);
        }
        say("volume unknown", mc.getChannelVolume(1));
        mc.setChannelVolume(0, 100);
        say("volume set", mc.getChannelVolume(0));
        try {
            mc.setChannelVolume(0, 128);
        } catch (IllegalArgumentException e) {
            threw("volume 128", e);
        }
        mc.setProgram(0, -1, 5);
        mc.setProgram(0, 2, 5);
        try {
            mc.setProgram(0, 0, 200);
        } catch (IllegalArgumentException e) {
            threw("program 200", e);
        }
        byte[] notes = {(byte) 0x90, 64, 100, 67, 100, (byte) 0x80, 64, 0};
        say("long event", mc.longMidiEvent(notes, 0, notes.length));
        try {
            mc.longMidiEvent(notes, 4, 5);
        } catch (IllegalArgumentException e) {
            threw("long event past the end", e);
        }
        m.start();
        say("midi heard", Pump.peak(200) > 0);
        m.close();

        Player wav = player("/beep.wav", "audio/x-wav");
        wav.realize();
        say("wav midi control", wav.getControl("MIDIControl") == null ? "null" : "some");
        say("wav tone control", wav.getControl("ToneControl") == null ? "null" : "some");
        wav.close();

        // The player stops by itself at the stop time.
        Player s = player("/long.wav", "audio/x-wav");
        MediaRecorder r = new MediaRecorder();
        s.addPlayerListener(r);
        s.realize();
        StopTimeControl stc = (StopTimeControl) s.getControl("StopTimeControl");
        say("no stop time", stc.getStopTime() == StopTimeControl.RESET);
        stc.setStopTime(300000);
        say("stop time", stc.getStopTime());
        s.start();
        try {
            stc.setStopTime(400000);
        } catch (IllegalStateException e) {
            threw("second stop time while started", e);
        }
        for (int i = 0; i < 40 && !r.waitForEvent("stoppedAtTime", 20); i++) {
            Pump.advance(20);
        }
        say("stopped at time", r.waitForEvent("stoppedAtTime", 5000));
        say("state after the stop time", s.getState());
        long at = s.getMediaTime();
        say("near the stop time", at >= 300000 && at < 700000);
        say("stop time cleared", stc.getStopTime() == StopTimeControl.RESET);
        s.close();
        System.out.println("done");
    }
}
