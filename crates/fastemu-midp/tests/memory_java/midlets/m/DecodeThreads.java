package m;

/**
 * Eight threads each make /big.png, 1024 by 1024 pixels, three times by
 * name, dropping each image.
 */
public class DecodeThreads extends AtOnce {
    String name() {
        return "decode";
    }

    void work(int id) {
        decode(id);
    }
}
