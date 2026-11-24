package javax.microedition.rms;

/** Compile-time stub; the virtual machine supplies the real interface. */
public interface RecordListener {
    void recordAdded(RecordStore recordStore, int recordId);

    void recordChanged(RecordStore recordStore, int recordId);

    void recordDeleted(RecordStore recordStore, int recordId);
}
