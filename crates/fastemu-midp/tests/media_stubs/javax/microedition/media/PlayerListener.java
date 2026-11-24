// Compile-time stub: FastEmu implements this interface in Rust.
package javax.microedition.media;

public interface PlayerListener {
    String STARTED = "started";
    String STOPPED = "stopped";
    String END_OF_MEDIA = "endOfMedia";
    String VOLUME_CHANGED = "volumeChanged";
    String ERROR = "error";
    String CLOSED = "closed";
    String STOPPED_AT_TIME = "stoppedAtTime";

    void playerUpdate(Player player, String event, Object eventData);
}
