import java.io.ByteArrayInputStream;

import javax.microedition.media.Manager;
import javax.microedition.media.Player;

/** Wrong and missing content types, and sound FastEmu cannot decode. */
public class MediaFormats extends MediaUtil {
    public static void main(String[] args) throws Exception {
        // The bytes decide, whatever the game says.
        Player wavAsMidi = player("/beep.wav", "audio/midi");
        wavAsMidi.realize();
        say("wav labelled midi", wavAsMidi.getContentType());
        say("its duration", wavAsMidi.getDuration());
        wavAsMidi.start();
        say("heard", Pump.peak(50) > 0);
        wavAsMidi.close();

        Player midiAsWav = player("/tune.mid", "audio/x-wav");
        midiAsWav.realize();
        say("midi labelled wav", midiAsWav.getContentType());
        midiAsWav.close();

        Player untyped = player("/beep.wav", null);
        untyped.realize();
        say("no content type", untyped.getContentType());
        untyped.close();

        Player made = player("/beep.wav", "made/up");
        made.realize();
        say("made-up content type", made.getContentType());
        made.close();

        // Data FastEmu cannot play becomes a silent player that runs
        // through its states and ends after a second.
        byte[] junk = "not a sound at all".getBytes();
        Player unknown = Manager.createPlayer(new ByteArrayInputStream(junk), "audio/x-unknown");
        MediaRecorder r = new MediaRecorder();
        unknown.addPlayerListener(r);
        say("unsupported created", unknown.getState());
        unknown.realize();
        say("unsupported type", unknown.getContentType());
        say("unsupported duration", unknown.getDuration());
        unknown.prefetch();
        unknown.start();
        say("unsupported started", unknown.getState());
        // Past the fade-out of the first player's close.
        Pump.advance(20);
        say("unsupported heard", Pump.peak(50) > 0);
        long begun = System.currentTimeMillis();
        say("unsupported events", r.waitFor(2, 5000));
        long took = System.currentTimeMillis() - begun;
        say("ended after about a second", took > 700 && took < 4000);
        say("unsupported at the end", unknown.getState());
        say("unsupported time at the end", unknown.getMediaTime());
        unknown.close();
        say("unsupported closed", r.waitFor(3, 5000));

        Player nothing = Manager.createPlayer(new ByteArrayInputStream(junk), null);
        nothing.realize();
        say("unknown with no type", nothing.getContentType());
        nothing.close();

        // A WAV header over data that does not decode.
        byte[] broken = {'R', 'I', 'F', 'F', 0, 0, 0, 0, 'W', 'A', 'V', 'E', 'j', 'u', 'n', 'k'};
        Player bad = Manager.createPlayer(new ByteArrayInputStream(broken), "audio/x-wav");
        bad.start();
        say("broken wav", bad.getState() + " " + bad.getContentType() + " " + bad.getDuration());
        bad.close();

        Player empty = Manager.createPlayer(new ByteArrayInputStream(new byte[0]), "audio/x-wav");
        empty.start();
        say("empty stream", empty.getState());
        empty.close();
        System.out.println("done");
    }
}
