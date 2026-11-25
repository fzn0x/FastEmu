package m;

/**
 * Eight threads each pass Manager.createPlayer a stream that never ends
 * and lets the other threads run on each read.
 */
public class SoundThreads extends AtOnce {
    String name() {
        return "sound";
    }

    void work(int id) {
        sound(id);
    }
}
