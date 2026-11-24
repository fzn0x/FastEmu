import javax.microedition.rms.RecordStore;
import javax.microedition.rms.RecordStoreException;
import javax.microedition.rms.RecordStoreNotFoundException;

/**
 * Saves that outlive the virtual machine: "write" makes stores, "read" (in
 * a new machine on the same folder) checks them, "threads" adds records
 * from several threads at once, and "list" prints the stores.
 */
public class RmsPersist extends RmsTest {
    public static void main(String[] args) throws Exception {
        String mode = args[0];
        if (mode.equals("write")) {
            write();
        } else if (mode.equals("read")) {
            read();
        } else if (mode.equals("threads")) {
            threads();
        }
        say("stores " + stores(RecordStore.listRecordStores()));
    }

    static void write() throws Exception {
        RecordStore rs = RecordStore.openRecordStore("progress", true);
        rs.addRecord("level 3".getBytes(), 0, 7);
        rs.addRecord("score 900".getBytes(), 0, 9);
        rs.addRecord(new byte[] {1}, 0, 1);
        rs.deleteRecord(3);
        rs.addRecord(null, 0, 0);
        say("written version " + rs.getVersion() + " next " + rs.getNextRecordID());
        rs.closeRecordStore();

        RecordStore other = RecordStore.openRecordStore("Progress", true);
        other.addRecord("other".getBytes(), 0, 5);
        other.closeRecordStore();
        RecordStore unicode = RecordStore.openRecordStore("\u5b58\u6863", true);
        unicode.addRecord(new byte[] {42}, 0, 1);
        unicode.closeRecordStore();
        RecordStore.openRecordStore("temp", true).closeRecordStore();
        RecordStore.deleteRecordStore("temp");
    }

    static void read() throws Exception {
        RecordStore rs = RecordStore.openRecordStore("progress", false);
        say("records " + rs.getNumRecords() + " version " + rs.getVersion()
                + " next " + rs.getNextRecordID());
        say("1 " + new String(rs.getRecord(1)) + ", 2 " + new String(rs.getRecord(2))
                + ", 4 " + rs.getRecord(4));
        say("new id " + rs.addRecord(new byte[] {7}, 0, 1));
        rs.closeRecordStore();
        say("Progress " + new String(RecordStore.openRecordStore("Progress", false).getRecord(1)));
        say("unicode " + RecordStore.openRecordStore("\u5b58\u6863", false).getRecord(1)[0]);
        try {
            RecordStore.openRecordStore("temp", false);
        } catch (RecordStoreNotFoundException e) {
            say("temp " + name(e));
        }
    }

    static void threads() throws Exception {
        final RecordStore rs = RecordStore.openRecordStore("busy", true);
        Thread[] threads = new Thread[4];
        for (int t = 0; t < threads.length; t++) {
            final int tag = t;
            threads[t] = new Thread() {
                public void run() {
                    try {
                        for (int i = 0; i < 25; i++) {
                            int id = rs.addRecord(new byte[] {(byte) tag, (byte) i}, 0, 2);
                            if (rs.getRecord(id)[0] != tag) {
                                say("record " + id + " holds the wrong data");
                            }
                        }
                    } catch (RecordStoreException e) {
                        say("thread " + name(e));
                    }
                }
            };
            threads[t].start();
        }
        for (int t = 0; t < threads.length; t++) {
            threads[t].join();
        }
        say("busy " + rs.getNumRecords() + " next " + rs.getNextRecordID());
        rs.closeRecordStore();
    }
}
