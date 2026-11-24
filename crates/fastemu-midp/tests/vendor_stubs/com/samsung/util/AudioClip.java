// Compile-time stand-ins for the runtime's Samsung classes. Constant
// values follow docs/compat/api/vendors.md, since javac inlines them.
package com.samsung.util;

public class AudioClip {
    public static final int TYPE_MMF = 1;
    public static final int TYPE_MP3 = 2;
    public static final int TYPE_MIDI = 3;

    public AudioClip(int type, byte[] data, int offset, int length) {}

    public AudioClip(int type, String filename) throws java.io.IOException {}

    public void play(int loop, int volume) {}

    public void stop() {}

    public void pause() {}

    public void resume() {}

    public static boolean isSupported() {
        return false;
    }
}
