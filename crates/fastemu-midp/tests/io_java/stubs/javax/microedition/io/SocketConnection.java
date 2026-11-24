// Compile-time stub; the VM runs FastEmu's native class. See Connector.java.
package javax.microedition.io;

public interface SocketConnection extends StreamConnection {
    byte DELAY = 0;
    byte LINGER = 1;
    byte KEEPALIVE = 2;
    byte RCVBUF = 3;
    byte SNDBUF = 4;

    void setSocketOption(byte option, int value) throws java.io.IOException;
    int getSocketOption(byte option) throws java.io.IOException;
    String getLocalAddress() throws java.io.IOException;
    int getLocalPort() throws java.io.IOException;
    String getAddress() throws java.io.IOException;
    int getPort() throws java.io.IOException;
}
