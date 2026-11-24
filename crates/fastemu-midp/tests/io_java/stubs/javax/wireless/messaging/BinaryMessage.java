// Compile-time stub; the VM runs FastEmu's native interface.
package javax.wireless.messaging;

public interface BinaryMessage extends Message {
    byte[] getPayloadData();
    void setPayloadData(byte[] data);
}
