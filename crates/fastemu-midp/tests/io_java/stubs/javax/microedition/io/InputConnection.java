// Compile-time stub; the VM runs FastEmu's native class. See Connector.java.
package javax.microedition.io;

public interface InputConnection extends Connection {
    java.io.InputStream openInputStream() throws java.io.IOException;
    java.io.DataInputStream openDataInputStream() throws java.io.IOException;
}
