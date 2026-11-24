// Compile-time stub; the VM runs FastEmu's native interface. Members from
// docs/compat/api/jsr-later.md.
package javax.wireless.messaging;

public interface Message {
    String getAddress();
    void setAddress(String addr);
    java.util.Date getTimestamp();
}
