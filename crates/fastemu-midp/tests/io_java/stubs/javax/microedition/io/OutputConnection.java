// Compile-time stub; the VM runs FastEmu's native class. See Connector.java.
package javax.microedition.io;

public interface OutputConnection extends Connection {
    java.io.OutputStream openOutputStream() throws java.io.IOException;
    java.io.DataOutputStream openDataOutputStream() throws java.io.IOException;
}
