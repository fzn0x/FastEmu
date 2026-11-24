import javax.microedition.rms.RecordStore;
import javax.microedition.rms.RecordStoreNotFoundException;

/**
 * Stores of other suites, opened by vendor and suite name. Runs as suite
 * "Mine" of vendor "Me"; the test puts the other suite's stores in place.
 */
public class RmsShared extends RmsTest {
    static final String VENDOR = "Other Vendor";
    static final String SUITE = "Other Game";

    public static void main(String[] args) throws Exception {
        RecordStore scores = RecordStore.openRecordStore("scores", VENDOR, SUITE);
        say("shared " + new String(scores.getRecord(1)) + " records " + scores.getNumRecords());
        try {
            scores.addRecord(new byte[1], 0, 1);
        } catch (SecurityException e) {
            say("read-only add " + name(e));
        }
        try {
            scores.deleteRecord(1);
        } catch (SecurityException e) {
            say("read-only delete " + name(e));
        }
        try {
            scores.setMode(RecordStore.AUTHMODE_ANY, true);
        } catch (SecurityException e) {
            say("setMode " + name(e));
        }
        say("same object " + (RecordStore.openRecordStore("scores", VENDOR, SUITE) == scores));
        try {
            RecordStore.openRecordStore("private", VENDOR, SUITE);
        } catch (SecurityException e) {
            say("private " + name(e));
        }
        try {
            RecordStore.openRecordStore("nothing", VENDOR, SUITE);
        } catch (RecordStoreNotFoundException e) {
            say("nothing " + name(e));
        }
        try {
            RecordStore.openRecordStore("scores", "Nobody", "Nothing");
        } catch (RecordStoreNotFoundException e) {
            say("unknown suite " + name(e));
        }

        RecordStore board = RecordStore.openRecordStore("board", VENDOR, SUITE);
        say("writable board adds " + board.addRecord("mine".getBytes(), 0, 4));
        board.closeRecordStore();

        RecordStore mine = RecordStore.openRecordStore("mine", true);
        say("own by name " + (RecordStore.openRecordStore("mine", "Me", "Mine") == mine));
        try {
            RecordStore.openRecordStore("absent", "Me", "Mine");
        } catch (RecordStoreNotFoundException e) {
            say("own absent " + name(e));
        }
        try {
            RecordStore.openRecordStore("mine", null, "Mine");
        } catch (IllegalArgumentException e) {
            say("null vendor " + name(e));
        }
        say("stores " + stores(RecordStore.listRecordStores()));
    }
}
