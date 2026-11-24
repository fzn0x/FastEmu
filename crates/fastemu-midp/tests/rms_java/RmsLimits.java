import javax.microedition.rms.RecordStore;
import javax.microedition.rms.RecordStoreFullException;

/**
 * The quota (the test sets it to 4096 bytes) and damaged files (the test
 * puts "broken" and "cut" in place first).
 */
public class RmsLimits extends RmsTest {
    public static void main(String[] args) throws Exception {
        say("stores " + stores(RecordStore.listRecordStores()));

        RecordStore broken = RecordStore.openRecordStore("broken", false);
        say("broken opens with " + broken.getNumRecords() + " records, next "
                + broken.getNextRecordID());
        say("broken takes record " + broken.addRecord(new byte[] {1}, 0, 1));
        broken.closeRecordStore();

        RecordStore cut = RecordStore.openRecordStore("cut", false);
        say("cut keeps " + cut.getNumRecords() + ": " + new String(cut.getRecord(1))
                + ", next " + cut.getNextRecordID());
        cut.closeRecordStore();

        RecordStore big = RecordStore.openRecordStore("big", true);
        int available = big.getSizeAvailable();
        say("available " + (available > 3500 && available < 4096));
        big.addRecord(new byte[2000], 0, 2000);
        try {
            big.addRecord(new byte[3000], 0, 3000);
            say("added past the quota");
        } catch (RecordStoreFullException e) {
            say("add " + name(e) + ", records " + big.getNumRecords());
        }
        try {
            big.setRecord(1, new byte[5000], 0, 5000);
            say("set past the quota");
        } catch (RecordStoreFullException e) {
            say("set " + name(e) + ", size " + big.getRecordSize(1));
        }
        int before = big.getSizeAvailable();
        big.setRecord(1, new byte[10], 0, 10);
        say("shrinking frees " + (big.getSizeAvailable() - before));
        big.closeRecordStore();
    }
}
