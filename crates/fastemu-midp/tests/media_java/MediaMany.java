import javax.microedition.media.Player;

/** Hundreds of players: closed ones and forgotten ones leave the engine. */
public class MediaMany extends MediaUtil {
    /** Realizes players and forgets them; the frame goes when it returns. */
    static void forget(int count) throws Exception {
        for (int i = 0; i < count; i++) {
            Player p = player("/beep.wav", "audio/x-wav");
            p.addPlayerListener(new MediaRecorder());
            p.realize();
        }
    }

    /** Starts players and forgets them. */
    static void startAndForget(int count, MediaRecorder r) throws Exception {
        for (int i = 0; i < count; i++) {
            Player p = player("/beep.wav", "audio/x-wav");
            p.addPlayerListener(r);
            p.start();
        }
    }

    public static void main(String[] args) throws Exception {
        for (int i = 0; i < 300; i++) {
            Player p = player("/beep.wav", "audio/x-wav");
            p.realize();
            p.prefetch();
            p.start();
            p.close();
        }
        say("after closing 300", Pump.players());

        forget(300);
        System.gc();
        say("after a collection", Pump.players());

        // Started players the game forgot keep playing, and their
        // listeners still hear the end.
        MediaRecorder r = new MediaRecorder();
        startAndForget(5, r);
        System.gc();
        say("playing after a collection", Pump.players());
        Pump.advance(300);
        r.waitFor(10, 5000);
        say("ends heard", r.count("endOfMedia"));
        // The event thread holds the last one until its listener returns,
        // which a busy machine may delay: collect until they are all gone,
        // for five seconds.
        int left = Pump.players();
        for (int i = 0; i < 250 && left > 0; i++) {
            Thread.sleep(20);
            System.gc();
            left = Pump.players();
        }
        say("after they ended", left);
        System.out.println("done");
    }
}
