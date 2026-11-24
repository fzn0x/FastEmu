package p;

import java.io.InputStream;
import javax.microedition.media.Manager;
import javax.microedition.media.MediaException;
import javax.microedition.media.Player;
import javax.microedition.media.PlayerListener;

/** One player and the events its listener heard. */
class Track implements PlayerListener {
    /** How long a test waits for one event. */
    private static final long WAIT = 10000;

    private final String label;
    private final StringBuffer events = new StringBuffer();
    private Player player;
    private String type = "none";
    private long duration;
    private String error = "";

    private Track(String label) {
        this.label = label;
    }

    static Track fromStream(String file, String type) {
        Track track = new Track(file + " as " + type);
        try {
            InputStream in = Players.class.getResourceAsStream("/" + file);
            track.setup(Manager.createPlayer(in, type));
        } catch (Exception e) {
            track.error = " error " + e;
        }
        return track;
    }

    static Track fromLocator(String locator) {
        Track track = new Track(locator);
        try {
            track.setup(Manager.createPlayer(locator));
        } catch (Exception e) {
            track.error = " error " + e;
        }
        return track;
    }

    private void setup(Player p) throws MediaException {
        player = p;
        p.addPlayerListener(this);
        p.realize();
        type = p.getContentType();
        duration = p.getDuration();
    }

    void start() {
        try {
            if (player != null) {
                player.start();
            }
        } catch (MediaException e) {
            error = " error " + e;
        }
    }

    void close() {
        if (player != null) {
            player.close();
        }
    }

    public synchronized void playerUpdate(Player p, String event, Object data) {
        events.append(' ').append(event);
        notifyAll();
    }

    /** Waits until the listener heard `event`, or gives up. */
    synchronized void await(String event) {
        long end = System.currentTimeMillis() + WAIT;
        while (events.toString().indexOf(event) < 0) {
            long left = end - System.currentTimeMillis();
            if (left <= 0 || player == null) {
                return;
            }
            try {
                wait(left);
            } catch (InterruptedException e) {
                return;
            }
        }
    }

    String summary() {
        return label + ": type " + type + " duration " + duration + " events" + events + error;
    }
}
