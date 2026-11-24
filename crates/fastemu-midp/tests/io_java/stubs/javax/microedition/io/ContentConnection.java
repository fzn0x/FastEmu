// Compile-time stub; the VM runs FastEmu's native class. See Connector.java.
package javax.microedition.io;

public interface ContentConnection extends StreamConnection {
    String getType();
    String getEncoding();
    long getLength();
}
