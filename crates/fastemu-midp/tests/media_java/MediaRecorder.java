import javax.microedition.media.Player;
import javax.microedition.media.PlayerListener;
import javax.microedition.media.control.VolumeControl;

/**
 * Records player events as "name:data" and lets a test wait for them. It
 * runs on the media event thread; the test's main thread waits on it.
 */
public class MediaRecorder implements PlayerListener {
    private final String[] events = new String[512];
    private int count;
    private boolean interned = true;
    private Object lastData;
    private String lastThread;

    public synchronized void playerUpdate(Player player, String event, Object data) {
        // The names must be the constants themselves, so == works.
        if (event != STARTED && event != STOPPED && event != END_OF_MEDIA
                && event != CLOSED && event != VOLUME_CHANGED
                && event != STOPPED_AT_TIME && event != ERROR) {
            interned = false;
        }
        String text = event;
        if (data instanceof Long) {
            text = text + ":" + ((Long) data).longValue();
        } else if (data == null) {
            text = text + ":null";
        } else if (data instanceof VolumeControl) {
            text = text + ":VolumeControl";
        } else {
            text = text + ":" + data;
        }
        if (count < events.length) {
            events[count] = text;
            count++;
        }
        lastData = data;
        lastThread = Thread.currentThread().getName();
        notifyAll();
    }

    /** Waits until n events arrived or ms passed, and returns them all. */
    public synchronized String waitFor(int n, long ms) throws InterruptedException {
        long end = System.currentTimeMillis() + ms;
        while (count < n) {
            long left = end - System.currentTimeMillis();
            if (left <= 0) {
                break;
            }
            wait(left);
        }
        return joined();
    }

    /** Waits until an event starting with prefix arrived, or ms passed. */
    public synchronized boolean waitForEvent(String prefix, long ms) throws InterruptedException {
        long end = System.currentTimeMillis() + ms;
        while (!has(prefix)) {
            long left = end - System.currentTimeMillis();
            if (left <= 0) {
                return false;
            }
            wait(left);
        }
        return true;
    }

    public synchronized int count(String prefix) {
        int n = 0;
        for (int i = 0; i < count; i++) {
            if (events[i].startsWith(prefix)) {
                n++;
            }
        }
        return n;
    }

    private boolean has(String prefix) {
        return count(prefix) > 0;
    }

    public synchronized String joined() {
        StringBuffer out = new StringBuffer();
        for (int i = 0; i < count; i++) {
            if (i > 0) {
                out.append(' ');
            }
            out.append(events[i]);
        }
        return out.toString();
    }

    public synchronized String last() {
        return count == 0 ? "" : events[count - 1];
    }

    public synchronized int size() {
        return count;
    }

    public synchronized void clear() {
        count = 0;
    }

    public synchronized boolean interned() {
        return interned;
    }

    public synchronized Object lastData() {
        return lastData;
    }

    public synchronized String lastThread() {
        return lastThread;
    }
}
