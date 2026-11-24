package r;

import java.io.InputStream;

/** Looks a resource up relative to the class that implements it. */
public interface Finder {
    InputStream find(String name);
}
