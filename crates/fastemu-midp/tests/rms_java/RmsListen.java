import javax.microedition.rms.RecordListener;
import javax.microedition.rms.RecordStore;
import javax.microedition.rms.RecordStoreException;

/** Record listeners: order, duplicates, removal, threads and closing. */
public class RmsListen extends RmsTest implements RecordListener {
    static Thread writer;

    private final String tag;

    RmsListen(String tag) {
        this.tag = tag;
    }

    public void recordAdded(RecordStore store, int id) {
        try {
            say(tag + " added " + id + " to " + store.getName() + ", which has "
                    + store.getNumRecords() + " records");
        } catch (RecordStoreException e) {
            say(tag + " " + name(e));
        }
        if (writer != null) {
            say(tag + " on the writing thread " + (Thread.currentThread() == writer));
        }
    }

    public void recordChanged(RecordStore store, int id) {
        say(tag + " changed " + id);
    }

    public void recordDeleted(RecordStore store, int id) {
        say(tag + " deleted " + id);
    }

    public static void main(String[] args) throws Exception {
        final RecordStore rs = RecordStore.openRecordStore("events", true);
        RmsListen a = new RmsListen("a");
        RmsListen b = new RmsListen("b");
        rs.addRecordListener(a);
        rs.addRecordListener(b);
        rs.addRecordListener(a);
        rs.addRecordListener(null);
        int id = rs.addRecord(new byte[] {1}, 0, 1);
        rs.setRecord(id, new byte[] {2}, 0, 1);
        rs.removeRecordListener(a);
        rs.removeRecordListener(a);
        rs.deleteRecord(id);

        RecordStore other = RecordStore.openRecordStore("other", true);
        other.addRecordListener(new RecordListener() {
            public void recordAdded(RecordStore store, int recordId) {
                throw new RuntimeException("boom");
            }

            public void recordChanged(RecordStore store, int recordId) {
            }

            public void recordDeleted(RecordStore store, int recordId) {
            }
        });
        try {
            other.addRecord(new byte[] {3}, 0, 1);
            say("no exception");
        } catch (RuntimeException e) {
            say("listener threw " + e.getMessage() + ", the record stays: "
                    + other.getNumRecords());
        }
        other.closeRecordStore();

        Thread t = new Thread() {
            public void run() {
                try {
                    rs.addRecord(new byte[] {4}, 0, 1);
                } catch (RecordStoreException e) {
                    say("thread " + name(e));
                }
            }
        };
        writer = t;
        t.start();
        t.join();
        writer = null;

        rs.closeRecordStore();
        RecordStore reopened = RecordStore.openRecordStore("events", false);
        reopened.addRecord(new byte[] {5}, 0, 1);
        reopened.addRecordListener(new RmsListen("c"));
        reopened.closeRecordStore();
        reopened.addRecordListener(new RmsListen("d"));
        RecordStore last = RecordStore.openRecordStore("events", false);
        last.addRecord(new byte[] {6}, 0, 1);
        say("done with " + last.getNumRecords() + " records");
        last.closeRecordStore();
    }
}
