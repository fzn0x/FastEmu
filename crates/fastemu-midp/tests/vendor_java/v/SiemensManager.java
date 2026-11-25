package v;

import com.siemens.mp.game.GraphicObjectManager;
import com.siemens.mp.game.Sprite;

/**
 * A GraphicObjectManager holds an object once: adding it again adds
 * nothing, however often a game does it. Inserting the same object keeps
 * a place for each, and the heap counts them.
 */
public class SiemensManager {
    static Sprite sprite() {
        return new Sprite(new byte[] {(byte) 0xFF}, 0, 8, 1, null, 0, 1);
    }

    /** What getObjectAt says of a place: the object's number, or none. */
    static String at(GraphicObjectManager m, int index, Sprite[] known) {
        try {
            Object o = m.getObjectAt(index);
            for (int i = 0; i < known.length; i++) {
                if (known[i] == o) {
                    return "#" + i;
                }
            }
            return "other";
        } catch (ArrayIndexOutOfBoundsException e) {
            return "none";
        }
    }

    public static void main(String[] args) throws Exception {
        Sprite a = sprite();
        Sprite b = sprite();
        Sprite[] known = {a, b};
        GraphicObjectManager m = new GraphicObjectManager();
        m.addObject(a);
        m.addObject(a);
        System.out.println("added twice: " + at(m, 0, known) + " " + at(m, 1, known));
        m.addObject(b);
        m.addObject(a);
        System.out.println("then another: " + at(m, 0, known) + " " + at(m, 1, known) + " "
                + at(m, 2, known) + " a at " + m.getObjectPosition(a));
        m.deleteObject(a);
        System.out.println("deleted once: " + at(m, 0, known) + " " + at(m, 1, known));

        // The same object added without end keeps one place.
        GraphicObjectManager many = new GraphicObjectManager();
        for (int i = 0; i < 200000; i++) {
            many.addObject(a);
        }
        System.out.println("200000 adds: " + at(many, 0, known) + " " + at(many, 1, known));

        // The same object inserted without end is counted against the
        // heap, and ends in OutOfMemoryError.
        GraphicObjectManager inserted = new GraphicObjectManager();
        int calls = 0;
        try {
            for (; calls < 1000000; calls++) {
                inserted.insertObject(a, calls);
            }
            System.out.println("inserts: no oom after " + calls);
        } catch (OutOfMemoryError e) {
            inserted = null;
            System.out.println("inserts: oom " + (calls > 1000 && calls < 1000000));
        }
        System.out.println("done");
    }
}
