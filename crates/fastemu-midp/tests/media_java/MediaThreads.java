import javax.microedition.media.Manager;
import javax.microedition.media.Player;
import javax.microedition.media.control.ToneControl;

/**
 * A game that has started as many threads as it may still gets its sound:
 * the thread that delivers player events is the library's, so it draws on
 * the host's threads past the game's limit.
 */
public class MediaThreads extends MediaUtil {
    static final Object gate = new Object();
    static boolean open;

    static class Sleeper extends Thread {
        public void run() {
            synchronized (gate) {
                while (!open) {
                    try {
                        gate.wait();
                    } catch (InterruptedException e) {
                        return;
                    }
                }
            }
        }
    }

    public static void main(String[] args) throws Exception {
        Thread[] made = new Thread[200];
        int started = 0;
        boolean refused = false;
        while (started < made.length && !refused) {
            Thread t = new Sleeper();
            try {
                t.start();
                made[started++] = t;
            } catch (OutOfMemoryError e) {
                refused = true;
            }
        }
        say("game threads refused", refused);
        int share = Thread.activeCount();

        Player t = Manager.createPlayer(Manager.TONE_DEVICE_LOCATOR);
        MediaRecorder r = new MediaRecorder();
        t.addPlayerListener(r);
        t.realize();
        ToneControl tc = (ToneControl) t.getControl("ToneControl");
        tc.setSequence(new byte[] {
            ToneControl.VERSION, 1,
            ToneControl.TEMPO, 30,
            ToneControl.C4, 8,
            ToneControl.SILENCE, 8,
        });
        t.start();
        Pump.advance(10);
        Pump.advance(500);
        say("events", r.waitFor(2, 5000));
        say("past the game's share", Thread.activeCount() > share);
        t.close();

        synchronized (gate) {
            open = true;
            gate.notifyAll();
        }
        for (int i = 0; i < started; i++) {
            made[i].join();
        }
        System.out.println("done");
    }
}
