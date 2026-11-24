package com.siemens.mp.game;

public class MelodyComposer {
    public static final int TONE_C0 = 0;
    public static final int TONE_A2 = 33;
    public static final int TONE_PAUSE = 58;
    public static final int TONE_REPEAT = 61;
    public static final int TONELENGTH_1_4 = 2;
    public static final int TONELENGTH_1_8 = 3;
    public static final int BPM = 60;

    public MelodyComposer() {}

    public MelodyComposer(int[] notes, int bpm) {}

    public void setBPM(int bpm) {}

    public void appendNote(int note, int length) throws Exception {}

    public Melody getMelody() {
        return null;
    }

    public void resetMelody() {}

    public int length() {
        return 0;
    }

    public static int maxLength() {
        return 0;
    }
}
