package javax.microedition.rms;

/** Compile-time stub; the virtual machine supplies the real class. */
public class RecordStore {
    public static final int AUTHMODE_PRIVATE = 0;
    public static final int AUTHMODE_ANY = 1;

    private RecordStore() {
    }

    public static void deleteRecordStore(String recordStoreName)
            throws RecordStoreException, RecordStoreNotFoundException {
        throw new RuntimeException("stub");
    }

    public static RecordStore openRecordStore(String recordStoreName, boolean createIfNecessary)
            throws RecordStoreException, RecordStoreFullException, RecordStoreNotFoundException {
        throw new RuntimeException("stub");
    }

    public static RecordStore openRecordStore(String recordStoreName, boolean createIfNecessary,
            int authmode, boolean writable)
            throws RecordStoreException, RecordStoreFullException, RecordStoreNotFoundException {
        throw new RuntimeException("stub");
    }

    public static RecordStore openRecordStore(String recordStoreName, String vendorName,
            String suiteName) throws RecordStoreException, RecordStoreNotFoundException {
        throw new RuntimeException("stub");
    }

    public void setMode(int authmode, boolean writable) throws RecordStoreException {
        throw new RuntimeException("stub");
    }

    public void closeRecordStore() throws RecordStoreNotOpenException, RecordStoreException {
        throw new RuntimeException("stub");
    }

    public static String[] listRecordStores() {
        throw new RuntimeException("stub");
    }

    public String getName() throws RecordStoreNotOpenException {
        throw new RuntimeException("stub");
    }

    public int getVersion() throws RecordStoreNotOpenException {
        throw new RuntimeException("stub");
    }

    public int getNumRecords() throws RecordStoreNotOpenException {
        throw new RuntimeException("stub");
    }

    public int getSize() throws RecordStoreNotOpenException {
        throw new RuntimeException("stub");
    }

    public int getSizeAvailable() throws RecordStoreNotOpenException {
        throw new RuntimeException("stub");
    }

    public long getLastModified() throws RecordStoreNotOpenException {
        throw new RuntimeException("stub");
    }

    public void addRecordListener(RecordListener listener) {
        throw new RuntimeException("stub");
    }

    public void removeRecordListener(RecordListener listener) {
        throw new RuntimeException("stub");
    }

    public int getNextRecordID() throws RecordStoreNotOpenException, RecordStoreException {
        throw new RuntimeException("stub");
    }

    public int addRecord(byte[] data, int offset, int numBytes)
            throws RecordStoreNotOpenException, RecordStoreException, RecordStoreFullException {
        throw new RuntimeException("stub");
    }

    public void deleteRecord(int recordId)
            throws RecordStoreNotOpenException, InvalidRecordIDException, RecordStoreException {
        throw new RuntimeException("stub");
    }

    public int getRecordSize(int recordId)
            throws RecordStoreNotOpenException, InvalidRecordIDException, RecordStoreException {
        throw new RuntimeException("stub");
    }

    public int getRecord(int recordId, byte[] buffer, int offset)
            throws RecordStoreNotOpenException, InvalidRecordIDException, RecordStoreException {
        throw new RuntimeException("stub");
    }

    public byte[] getRecord(int recordId)
            throws RecordStoreNotOpenException, InvalidRecordIDException, RecordStoreException {
        throw new RuntimeException("stub");
    }

    public void setRecord(int recordId, byte[] newData, int offset, int numBytes)
            throws RecordStoreNotOpenException, InvalidRecordIDException, RecordStoreException,
            RecordStoreFullException {
        throw new RuntimeException("stub");
    }

    public RecordEnumeration enumerateRecords(RecordFilter filter, RecordComparator comparator,
            boolean keepUpdated) throws RecordStoreNotOpenException {
        throw new RuntimeException("stub");
    }
}
