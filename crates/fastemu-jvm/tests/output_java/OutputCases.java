/**
 * Programs that make the machine keep text the game chose: output without
 * a line break, and names of classes that do not exist. They use only CLDC
 * APIs, and loop a bounded number of times.
 */
public class OutputCases {
    /** A string of {@code chars} letters. */
    static String text(int chars) {
        String s = "abcdefghij";
        while (s.length() < chars) {
            s = s.concat(s);
        }
        return s.substring(0, chars);
    }

    /** Prints {@code chars} letters {@code times} times, never a line break. */
    static void print(int times, int chars) {
        String s = text(chars);
        for (int i = 0; i < times; i++) {
            System.out.print(s);
        }
    }

    /** The same on System.err. */
    static void printErr(int times, int chars) {
        String s = text(chars);
        for (int i = 0; i < times; i++) {
            System.err.print(s);
        }
    }

    /** Prints one line of {@code chars} letters. */
    static void println(int chars) {
        System.out.println(text(chars));
    }

    /**
     * Asks for {@code count} classes that do not exist, each by a name of
     * {@code pad} letters and its number. Returns how many Class.forName
     * calls threw ClassNotFoundException.
     */
    static int misses(int count, int pad) {
        String padding = text(pad);
        int thrown = 0;
        for (int i = 0; i < count; i++) {
            try {
                Class.forName("m" + i + padding);
            } catch (ClassNotFoundException e) {
                thrown++;
            }
        }
        return thrown;
    }
}
