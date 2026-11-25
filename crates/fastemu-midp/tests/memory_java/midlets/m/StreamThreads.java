package m;

/**
 * Eight threads each pass Image.createImage a stream that never ends and
 * lets the other threads run on each read.
 */
public class StreamThreads extends AtOnce {
    String name() {
        return "stream";
    }

    void work(int id) {
        image(id);
    }
}
