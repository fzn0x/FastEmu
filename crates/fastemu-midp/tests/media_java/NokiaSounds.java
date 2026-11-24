import com.nokia.mid.sound.Sound;
import com.nokia.mid.sound.SoundListener;

/** Nokia's Sound: ringing tones, beeps, WAV, its states and listener. */
public class NokiaSounds extends MediaUtil implements SoundListener {
    private final StringBuffer heard = new StringBuffer();
    private int count;

    public synchronized void soundStateChanged(Sound sound, int event) {
        if (heard.length() > 0) {
            heard.append(' ');
        }
        heard.append(event);
        count++;
        notifyAll();
    }

    synchronized String waitFor(int n) throws InterruptedException {
        long end = System.currentTimeMillis() + 5000;
        while (count < n) {
            long left = end - System.currentTimeMillis();
            if (left <= 0) {
                break;
            }
            wait(left);
        }
        return heard.toString();
    }

    public static void main(String[] args) throws Exception {
        NokiaSounds l = new NokiaSounds();
        Sound ring = new Sound(bytes("/ring.ott"), Sound.FORMAT_TONE);
        ring.setSoundListener(l);
        say("ring created", ring.getState());
        say("default gain", ring.getGain());
        ring.play(1);
        say("ring playing", ring.getState());
        say("heard", l.waitFor(1));
        Pump.advance(1100);
        say("heard", l.waitFor(2));
        say("ring after the end", ring.getState());

        // Two passes report playing once and stopped once.
        ring.play(2);
        Pump.advance(2100);
        say("heard", l.waitFor(4));
        ring.play(0);
        Pump.advance(2500);
        say("endless still playing", ring.getState());
        ring.play(1);
        say("restart", l.waitFor(7));
        ring.stop();
        say("heard", l.waitFor(8));
        say("ring stopped", ring.getState());
        ring.resume();
        say("ring resumed", ring.getState());
        ring.stop();

        ring.setGain(300);
        say("gain 300", ring.getGain());
        ring.setGain(-5);
        say("gain -5", ring.getGain());
        ring.setGain(128);
        say("gain 128", ring.getGain());
        try {
            ring.play(-1);
        } catch (IllegalArgumentException e) {
            threw("play -1", e);
        }
        ring.release();
        say("released", ring.getState());
        say("heard", l.waitFor(11));
        ring.play(1);
        say("released does not play", ring.getState());

        Sound beep = new Sound(880, 200);
        say("beep created", beep.getState());
        beep.play(1);
        say("beep heard", Pump.peak(50) > 0);
        Pump.advance(300);
        say("beep after the end", beep.getState());
        beep.init(bytes("/beep.wav"), Sound.FORMAT_WAV);
        say("re-initialized as wav", beep.getState());
        beep.play(1);
        say("wav heard", Pump.peak(50) > 0);
        beep.stop();

        Sound midiAsWav = new Sound(bytes("/tune.mid"), Sound.FORMAT_WAV);
        say("midi given as wav", midiAsWav.getState());

        Sound junk = new Sound(new byte[] {1, 2, 3}, Sound.FORMAT_WAV);
        say("junk", junk.getState());
        junk.play(1);
        say("junk after play", junk.getState());
        try {
            new Sound(null, Sound.FORMAT_TONE);
        } catch (NullPointerException e) {
            threw("null data", e);
        }
        try {
            new Sound(bytes("/ring.ott"), 99);
        } catch (IllegalArgumentException e) {
            threw("format 99", e);
        }
        try {
            new Sound(440, 0);
        } catch (IllegalArgumentException e) {
            threw("duration 0", e);
        }
        try {
            new Sound(30000, 100);
        } catch (IllegalArgumentException e) {
            threw("frequency 30000", e);
        }
        say("formats", Sound.getSupportedFormats()[0] + " " + Sound.getSupportedFormats()[1]);
        say("concurrent tones", Sound.getConcurrentSoundCount(Sound.FORMAT_TONE));
        try {
            Sound.getConcurrentSoundCount(7);
        } catch (IllegalArgumentException e) {
            threw("concurrent 7", e);
        }
        System.out.println("done");
    }
}
