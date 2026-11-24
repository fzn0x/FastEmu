// Compile-time stub; the VM runs FastEmu's native interface.
package javax.wireless.messaging;

import java.io.IOException;

public interface MessageConnection extends javax.microedition.io.Connection {
    String TEXT_MESSAGE = "text";
    String BINARY_MESSAGE = "binary";

    Message newMessage(String type);
    Message newMessage(String type, String address);
    void send(Message msg) throws IOException;
    Message receive() throws IOException;
    void setMessageListener(MessageListener l) throws IOException;
    int numberOfSegments(Message msg);
}
