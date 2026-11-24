// A compile-time stand-in for the runtime's class; the tests' JARs never
// contain it.
package javax.microedition.io;

public class ConnectionNotFoundException extends java.io.IOException {
    public ConnectionNotFoundException() {}

    public ConnectionNotFoundException(String s) {
        super(s);
    }
}
