package s;

import javax.microedition.midlet.MIDlet;
import javax.microedition.rms.InvalidRecordIDException;
import javax.microedition.rms.RecordComparator;
import javax.microedition.rms.RecordEnumeration;
import javax.microedition.rms.RecordFilter;
import javax.microedition.rms.RecordStore;
import javax.microedition.rms.RecordStoreException;
import javax.microedition.rms.RecordStoreNotFoundException;
import javax.microedition.rms.RecordStoreNotOpenException;

/**
 * Adds, deletes and re-adds records, closes and reopens the store and
 * enumerates it; launched a second time on the same data, it checks what
 * survived. Prints "rms" lines.
 */
public class Records extends MIDlet implements RecordFilter, RecordComparator {
    static void log(String what) {
        System.out.println("rms " + what);
    }

    public void startApp() {
        try {
            if (RecordStore.listRecordStores() == null) {
                first();
            } else {
                second();
            }
        } catch (RecordStoreException e) {
            log("failed: " + e);
        }
        notifyDestroyed();
    }

    public void pauseApp() {
    }

    public void destroyApp(boolean unconditional) {
    }

    static int add(RecordStore store, String text) throws RecordStoreException {
        byte[] data = text.getBytes();
        return store.addRecord(data, 0, data.length);
    }

    static String text(byte[] data) {
        return data == null ? "null" : new String(data);
    }

    String ids(RecordEnumeration e) throws RecordStoreException {
        StringBuffer out = new StringBuffer();
        while (e.hasNextElement()) {
            out.append(' ').append(e.nextRecordId());
        }
        e.destroy();
        return out.toString();
    }

    /** Keeps records that start with 'f'. */
    public boolean matches(byte[] candidate) {
        return candidate != null && candidate.length > 0 && candidate[0] == 'f';
    }

    /** Orders records by length, a record with no data first. */
    public int compare(byte[] a, byte[] b) {
        int la = a == null ? 0 : a.length;
        int lb = b == null ? 0 : b.length;
        return la < lb ? PRECEDES : la > lb ? FOLLOWS : EQUIVALENT;
    }

    void first() throws RecordStoreException {
        log("phase 1");
        try {
            RecordStore.openRecordStore("save", false);
            log("opened a missing store");
        } catch (RecordStoreNotFoundException e) {
            log("missing store: RecordStoreNotFoundException");
        }
        RecordStore store = RecordStore.openRecordStore("save", true);
        log("new store: records " + store.getNumRecords() + " next " + store.getNextRecordID());
        int one = add(store, "one");
        int two = add(store, "two");
        int three = add(store, "three");
        log("added " + one + " " + two + " " + three);
        store.deleteRecord(three);
        log("after deleting 3: next " + store.getNextRecordID());
        log("re-added " + add(store, "four"));
        store.deleteRecord(two);
        try {
            store.getRecord(two);
            log("read a deleted record");
        } catch (InvalidRecordIDException e) {
            log("deleted 2: InvalidRecordIDException");
        }
        int empty = store.addRecord(null, 0, 0);
        log("empty record " + empty + " reads " + text(store.getRecord(empty)));
        log("enumerated" + ids(store.enumerateRecords(null, null, false)));
        log("sorted" + ids(store.enumerateRecords(null, this, false)));
        log("filtered" + ids(store.enumerateRecords(this, null, false)));
        RecordStore again = RecordStore.openRecordStore("save", false);
        log("opened twice: same object " + (again == store));
        store.closeRecordStore();
        log("closed once: records " + store.getNumRecords());
        store.closeRecordStore();
        try {
            store.getNumRecords();
            log("read a closed store");
        } catch (RecordStoreNotOpenException e) {
            log("closed twice: RecordStoreNotOpenException");
        }
        RecordStore reopened = RecordStore.openRecordStore("save", false);
        log("reopened: new object " + (reopened != store) + " records " + reopened.getNumRecords()
                + " next " + reopened.getNextRecordID() + " record 4 " + text(reopened.getRecord(4)));
        try {
            RecordStore.deleteRecordStore("save");
            log("deleted an open store");
        } catch (RecordStoreException e) {
            log("deleting it while open: RecordStoreException");
        }
        reopened.closeRecordStore();
        RecordStore.openRecordStore("scratch", true).closeRecordStore();
        RecordStore.deleteRecordStore("scratch");
        log("stores " + names());
        log("phase 1 done");
    }

    static String names() {
        String[] names = RecordStore.listRecordStores();
        StringBuffer out = new StringBuffer();
        for (int i = 0; names != null && i < names.length; i++) {
            out.append(i == 0 ? "" : " ").append(names[i]);
        }
        return out.toString();
    }

    void second() throws RecordStoreException {
        log("phase 2");
        log("stores " + names());
        RecordStore store = RecordStore.openRecordStore("save", false);
        log("kept: records " + store.getNumRecords() + " next " + store.getNextRecordID()
                + " enumerated" + ids(store.enumerateRecords(null, null, false)));
        log("record 1 " + text(store.getRecord(1)) + " record 4 " + text(store.getRecord(4)));
        log("added " + add(store, "six"));
        store.closeRecordStore();
        log("phase 2 done");
    }
}
