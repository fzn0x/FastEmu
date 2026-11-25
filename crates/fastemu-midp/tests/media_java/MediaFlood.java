import javax.microedition.media.Player;
import javax.microedition.media.PlayerListener;
import javax.microedition.media.control.VolumeControl;

/**
 * A game that makes events faster than its listener takes them: the
 * listener sticks on its first event while the game flips its volume
 * thousands of times, and the host's memory must not follow.
 */
public class MediaFlood extends MediaUtil {
    /** Holds every event until released, and counts them. */
    static class Stuck implements PlayerListener {
        private boolean released;
        private int count;
        private String last = "none";

        public synchronized void playerUpdate(Player player, String event, Object data) {
            count++;
            last = event;
            while (!released) {
                try {
                    wait(20);
                } catch (InterruptedException e) {
                    return;
                }
            }
        }

        synchronized void release() {
            released = true;
            notifyAll();
        }

        synchronized int count() {
            return count;
        }

        synchronized String last() {
            return last;
        }

        /** Waits until no event has come for a while. */
        void settle() throws InterruptedException {
            int seen = -1;
            for (int i = 0; i < 500; i++) {
                int now = count();
                if (now == seen) {
                    return;
                }
                seen = now;
                Thread.sleep(100);
            }
        }
    }

    public static void main(String[] args) throws Exception {
        Player p = player("/long.wav", "audio/x-wav");
        Stuck listener = new Stuck();
        p.addPlayerListener(listener);
        p.realize();
        VolumeControl volume = (VolumeControl) p.getControl("VolumeControl");
        p.start();
        // The first event holds the listener. Behind it, the game flips
        // its volume 5000 times.
        for (int i = 0; i < 5000; i++) {
            volume.setLevel(i % 2 == 0 ? 10 : 90);
        }
        listener.release();
        listener.settle();
        int heard = listener.count();
        say("events heard are bounded", heard < 1200);
        say("some were heard", heard > 500);
        p.stop();
        p.close();
        listener.settle();
        // The close is never one of the dropped.
        say("last event", listener.last());
        System.out.println("done");
    }
}
