import javax.microedition.media.Player;
import javax.microedition.media.PlayerListener;
import javax.microedition.media.control.VolumeControl;

/** Listener events: their order, data, names and thread. */
public class MediaEvents extends MediaUtil {
    /** A listener that always throws. */
    static class Thrower implements PlayerListener {
        public void playerUpdate(Player player, String event, Object data) {
            throw new RuntimeException("listener failed on " + event);
        }
    }

    public static void main(String[] args) throws Exception {
        Player p = player("/beep.wav", "audio/x-wav");
        MediaRecorder r = new MediaRecorder();
        p.addPlayerListener(r);
        p.addPlayerListener(r);
        p.addPlayerListener(null);
        p.start();
        say("state after start", p.getState());
        say("events", r.waitFor(1, 5000));
        Pump.advance(300);
        say("events", r.waitFor(2, 5000));
        say("state at the end", p.getState());
        say("time at the end", p.getMediaTime());
        p.start();
        Pump.advance(50);
        p.stop();
        say("events", r.waitFor(4, 5000));
        say("time after stop", p.getMediaTime());
        p.start();
        say("events", r.waitFor(5, 5000));
        p.close();
        say("events", r.waitFor(6, 5000));
        say("names interned", r.interned());
        say("listener thread", r.lastThread());

        // A removed listener hears nothing more.
        Player q = player("/beep.wav", "audio/x-wav");
        MediaRecorder kept = new MediaRecorder();
        MediaRecorder removed = new MediaRecorder();
        q.addPlayerListener(new Thrower());
        q.addPlayerListener(removed);
        q.addPlayerListener(kept);
        q.removePlayerListener(removed);
        q.removePlayerListener(null);
        q.start();
        say("after a throwing listener", kept.waitFor(1, 5000));
        say("removed listener", removed.size());
        q.close();
        kept.waitFor(2, 5000);

        // Volume changes carry the player's VolumeControl.
        Player v = player("/beep.wav", "audio/x-wav");
        MediaRecorder rv = new MediaRecorder();
        v.addPlayerListener(rv);
        v.realize();
        VolumeControl vc = (VolumeControl) v.getControl("VolumeControl");
        vc.setLevel(40);
        vc.setLevel(40);
        vc.setMute(true);
        vc.setMute(true);
        say("volume events", rv.waitFor(2, 5000));
        say("volume data is the control", rv.lastData() == vc);
        v.close();

        // Closing an unrealized player still tells its listeners.
        Player u = player("/beep.wav", "audio/x-wav");
        MediaRecorder ru = new MediaRecorder();
        u.addPlayerListener(ru);
        u.close();
        say("unrealized close", ru.waitFor(1, 5000));
        System.out.println("done");
    }
}
