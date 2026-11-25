package v;

import java.util.Vector;

import com.samsung.util.AudioClip;

/**
 * Samsung's audio clips are players of the game's sound group, so a game
 * holds only so many of them.
 */
public class SamsungMemory {
    public static void main(String[] args) throws Exception {
        byte[] midi = {'M', 'T', 'h', 'd', 0, 0, 0, 6, 0, 0, 0, 1, 0, 96, 'M', 'T', 'r', 'k', 0, 0, 0, 4, 0,
            (byte) 0xFF, 0x2F, 0};
        Vector clips = new Vector();
        try {
            for (int i = 0; i < 200; i++) {
                clips.addElement(new AudioClip(AudioClip.TYPE_MIDI, midi, 0, midi.length));
            }
        } catch (OutOfMemoryError e) {
            System.out.println("the 129th clip: " + e.getClass().getName());
        }
        System.out.println("clips held: " + clips.size());
        System.out.println("done");
    }
}
