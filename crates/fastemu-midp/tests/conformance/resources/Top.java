import java.io.InputStream;

/**
 * A class in the default package, where obfuscators put every class. Code
 * in a package cannot name it, so the MIDlet loads it with Class.forName.
 */
public class Top implements r.Finder {
    /** A relative name from here resolves at the root of the JAR. */
    public InputStream find(String name) {
        return Top.class.getResourceAsStream(name);
    }
}
