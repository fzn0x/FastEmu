// Compile-time stub: FastEmu implements this class in Rust. Signatures
// follow docs/compat/api/nokia.md.
package com.nokia.mid.sound;

public class Sound {
    public static final int FORMAT_TONE = 1;
    public static final int FORMAT_WAV = 5;
    public static final int SOUND_PLAYING = 0;
    public static final int SOUND_STOPPED = 1;
    public static final int SOUND_UNINITIALIZED = 3;

    public Sound(byte[] data, int type) {
    }

    public Sound(int freq, long duration) {
    }

    public void init(int freq, long duration) {
    }

    public void init(byte[] data, int type) {
    }

    public int getState() {
        return 0;
    }

    public void play(int loop) {
    }

    public void stop() {
    }

    public void resume() {
    }

    public void release() {
    }

    public void setGain(int gain) {
    }

    public int getGain() {
        return 0;
    }

    public static int getConcurrentSoundCount(int type) {
        return 0;
    }

    public static int[] getSupportedFormats() {
        return null;
    }

    public void setSoundListener(SoundListener listener) {
    }
}
