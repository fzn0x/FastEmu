package javax.microedition.rms;

public class RecordStore {
    private RecordStore() {}

    public static RecordStore openRecordStore(String recordStoreName, boolean createIfNecessary)
            throws RecordStoreException {
        return null;
    }

    public int getNumRecords() throws RecordStoreException {
        return 0;
    }

    public int addRecord(byte[] data, int offset, int numBytes) throws RecordStoreException {
        return 0;
    }

    public void closeRecordStore() throws RecordStoreException {}
}
