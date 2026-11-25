import javax.microedition.rms.RecordStore;
import javax.microedition.rms.RecordStoreFullException;
import javax.microedition.rms.RecordStoreNotFoundException;

/** A suite may have 64 record stores: the 65th is refused. */
public class RmsStores extends RmsTest {
    static int count() throws Exception {
        String[] names = RecordStore.listRecordStores();
        return names == null ? 0 : names.length;
    }

    static void make(String label, String name) throws Exception {
        try {
            RecordStore.openRecordStore(name, true).closeRecordStore();
            say(label + " made, " + count() + " stores");
        } catch (RecordStoreFullException e) {
            say(label + " " + name(e) + ", " + count() + " stores");
        }
    }

    public static void main(String[] args) throws Exception {
        for (int i = 0; i < 64; i++) {
            RecordStore.openRecordStore("s" + i, true).closeRecordStore();
        }
        say("made " + count());
        make("65th", "extra");

        // A store that exists opens and changes as before, and one that
        // does not exist is missing and not full.
        RecordStore first = RecordStore.openRecordStore("s0", true);
        first.addRecord(new byte[] {1, 2, 3}, 0, 3);
        say("s0 holds " + first.getNumRecords() + " record");
        first.closeRecordStore();
        try {
            RecordStore.openRecordStore("nothing", false);
            say("nothing opened");
        } catch (RecordStoreNotFoundException e) {
            say("nothing " + name(e));
        }

        // Deleting a store makes room for one more.
        RecordStore.deleteRecordStore("s10");
        say("deleted, " + count() + " stores");
        make("again", "extra");
        make("one more", "extra2");
        say("done");
    }
}
