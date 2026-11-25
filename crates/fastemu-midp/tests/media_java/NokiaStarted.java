import com.nokia.mid.sound.Sound;

/** A game plays more sounds than the engine starts at once. */
public class NokiaStarted extends MediaUtil {
    public static void main(String[] args) throws Exception {
        byte[] data = bytes("/long.wav");
        Sound[] playing = new Sound[32];
        for (int i = 0; i < playing.length; i++) {
            playing[i] = new Sound(data, Sound.FORMAT_WAV);
            playing[i].play(0);
        }
        say("the 32nd is playing", playing[31].getState());
        // The 33rd cannot start: no exception, and it stays stopped.
        Sound extra = new Sound(data, Sound.FORMAT_WAV);
        extra.play(1);
        say("the 33rd", extra.getState());
        // A stop gives its place to the next.
        playing[0].stop();
        extra.play(1);
        say("after a stop", extra.getState());
        extra.stop();

        // A start that failed leaves nothing holding the sound.
        playing[0].play(0);
        int before = Pump.players();
        for (int i = 0; i < 5; i++) {
            new Sound(data, Sound.FORMAT_WAV).play(1);
        }
        System.gc();
        say("refused sounds are collected", Pump.players() == before);
        System.out.println("done");
    }
}
