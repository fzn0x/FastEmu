import javax.microedition.rms.InvalidRecordIDException;
import javax.microedition.rms.RecordStore;
import javax.microedition.rms.RecordStoreException;
import javax.microedition.rms.RecordStoreNotFoundException;
import javax.microedition.rms.RecordStoreNotOpenException;

/** Record ids, record data, store metadata, opening and closing, names. */
public class RmsBasics extends RmsTest {
    public static void main(String[] args) throws Exception {
        say("stores " + stores(RecordStore.listRecordStores()));
        try {
            RecordStore.openRecordStore("save", false);
            say("opened a missing store");
        } catch (RecordStoreNotFoundException e) {
            say("missing " + name(e));
        }

        RecordStore rs = RecordStore.openRecordStore("save", true);
        say("name " + rs.getName() + " records " + rs.getNumRecords()
                + " next " + rs.getNextRecordID());
        int v0 = rs.getVersion();
        int a = rs.addRecord("alpha".getBytes(), 0, 5);
        int b = rs.addRecord(new byte[] {1, 2, 3, 4}, 1, 2);
        int c = rs.addRecord(null, 0, 0);
        say("ids " + a + " " + b + " " + c);
        say("version grew " + (rs.getVersion() - v0));
        say("record 1 " + new String(rs.getRecord(1)));
        byte[] two = rs.getRecord(2);
        say("record 2 " + two.length + " " + two[0] + " " + two[1]);
        say("record 3 " + (rs.getRecord(3) == null ? "null" : "data")
                + " size " + rs.getRecordSize(3));

        rs.deleteRecord(2);
        say("after delete next " + rs.getNextRecordID() + " count " + rs.getNumRecords());
        say("id after delete " + rs.addRecord(new byte[] {9}, 0, 1));
        try {
            rs.getRecord(2);
        } catch (InvalidRecordIDException e) {
            say("deleted " + name(e));
        }
        try {
            rs.getRecordSize(0);
        } catch (InvalidRecordIDException e) {
            say("zero " + name(e));
        }
        try {
            rs.deleteRecord(2);
        } catch (InvalidRecordIDException e) {
            say("delete twice " + name(e));
        }
        try {
            rs.setRecord(77, new byte[1], 0, 1);
        } catch (InvalidRecordIDException e) {
            say("set missing " + name(e));
        }

        rs.setRecord(1, "longer value".getBytes(), 0, 12);
        say("set " + new String(rs.getRecord(1)) + " size " + rs.getRecordSize(1));
        byte[] buf = new byte[20];
        int n = rs.getRecord(1, buf, 3);
        say("into " + n + " " + new String(buf, 3, n));
        try {
            rs.getRecord(1, new byte[5], 0);
        } catch (ArrayIndexOutOfBoundsException e) {
            say("small buffer " + name(e));
        }
        try {
            rs.getRecord(1, buf, 10);
        } catch (ArrayIndexOutOfBoundsException e) {
            say("late offset " + name(e));
        }
        try {
            rs.addRecord(new byte[3], 2, 2);
        } catch (ArrayIndexOutOfBoundsException e) {
            say("bad range " + name(e));
        }
        try {
            rs.addRecord(null, 0, 1);
        } catch (NullPointerException e) {
            say("null data " + name(e));
        }
        say("count " + rs.getNumRecords());

        int size = rs.getSize();
        int available = rs.getSizeAvailable();
        say("size " + (size > 30 && size < 200) + " available "
                + (available > 1040000 && available < 1048576));
        say("modified " + (rs.getLastModified() > 1600000000000L));

        RecordStore again = RecordStore.openRecordStore("save", false);
        say("same object " + (again == rs));
        rs.closeRecordStore();
        say("still open " + again.getNumRecords());
        try {
            RecordStore.deleteRecordStore("save");
        } catch (RecordStoreException e) {
            say("delete open " + name(e));
        }
        again.closeRecordStore();
        try {
            rs.getNumRecords();
        } catch (RecordStoreNotOpenException e) {
            say("closed " + name(e));
        }
        try {
            rs.closeRecordStore();
        } catch (RecordStoreNotOpenException e) {
            say("close twice " + name(e));
        }
        try {
            rs.addRecord(new byte[1], 0, 1);
        } catch (RecordStoreNotOpenException e) {
            say("add closed " + name(e));
        }
        RecordStore third = RecordStore.openRecordStore("save", false);
        say("new object " + (third != rs) + " records " + third.getNumRecords()
                + " next " + third.getNextRecordID());
        third.closeRecordStore();

        try {
            RecordStore.openRecordStore("", true);
        } catch (IllegalArgumentException e) {
            say("empty name " + name(e));
        }
        try {
            RecordStore.openRecordStore("123456789012345678901234567890123", true);
        } catch (IllegalArgumentException e) {
            say("long name " + name(e));
        }
        RecordStore.openRecordStore("12345678901234567890123456789012", true).closeRecordStore();
        RecordStore upper = RecordStore.openRecordStore("SAVE", true);
        say("case sensitive " + upper.getNumRecords());
        upper.closeRecordStore();
        RecordStore.openRecordStore("con:/\\*?\u5b58", true).closeRecordStore();
        say("stores " + stores(RecordStore.listRecordStores()));
        RecordStore.deleteRecordStore("SAVE");
        try {
            RecordStore.deleteRecordStore("SAVE");
        } catch (RecordStoreNotFoundException e) {
            say("delete missing " + name(e));
        }
        try {
            RecordStore.openRecordStore("SAVE", false);
        } catch (RecordStoreNotFoundException e) {
            say("deleted store " + name(e));
        }

        try {
            RecordStore.openRecordStore("x", true, 5, false);
        } catch (IllegalArgumentException e) {
            say("bad mode " + name(e));
        }
        RecordStore shared = RecordStore.openRecordStore("shared", true,
                RecordStore.AUTHMODE_ANY, true);
        try {
            shared.setMode(7, false);
        } catch (IllegalArgumentException e) {
            say("bad setMode " + name(e));
        }
        int version = shared.getVersion();
        shared.setMode(RecordStore.AUTHMODE_PRIVATE, false);
        say("setMode keeps the version " + (shared.getVersion() == version));
        shared.closeRecordStore();
        say("stores " + stores(RecordStore.listRecordStores()));
    }
}
