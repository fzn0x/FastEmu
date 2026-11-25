package v;

import com.siemens.mp.game.MelodyComposer;

/**
 * A MelodyComposer keeps its notes natively and makes no Java object for
 * them: kept composers must end in OutOfMemoryError, and ones a game
 * drops or resets must give their room back. Each case is a method of
 * its own that returns how far it got, so that nothing it kept is alive
 * when the result is printed.
 */
public class SiemensComposers {
    static final int MADE = 64;

    /** Composers made from one array of notes, kept or not. */
    static int composed(int[] notes, boolean keep) {
        MelodyComposer[] kept = new MelodyComposer[MADE];
        int made = 0;
        try {
            for (; made < MADE; made++) {
                MelodyComposer c = new MelodyComposer(notes, 120);
                if (keep) {
                    kept[made] = c;
                }
            }
        } catch (OutOfMemoryError e) {
            kept = null;
        }
        return made;
    }

    /** Composers that take 4096 notes one at a time, kept, reset or not. */
    static int appended(boolean reset) throws Exception {
        MelodyComposer[] kept = new MelodyComposer[MADE];
        int made = 0;
        try {
            for (; made < MADE; made++) {
                MelodyComposer c = new MelodyComposer();
                for (int i = 0; i < 4096; i++) {
                    c.appendNote(MelodyComposer.TONE_A2, MelodyComposer.TONELENGTH_1_4);
                }
                if (reset) {
                    c.resetMelody();
                }
                kept[made] = c;
            }
        } catch (OutOfMemoryError e) {
            kept = null;
        }
        return made;
    }

    public static void main(String[] args) throws Exception {
        int[] notes = new int[2 * MelodyComposer.maxLength()];
        System.out.println("kept composed: " + (composed(notes, true) < MADE ? "oom" : "no oom"));
        System.out.println("dropped composed: " + (composed(notes, false) == MADE ? "ok" : "oom"));
        notes = null;
        System.out.println("kept appended: " + (appended(false) < MADE ? "oom" : "no oom"));
        System.out.println("reset appended: " + (appended(true) == MADE ? "ok" : "oom"));
        System.out.println("done");
    }
}
