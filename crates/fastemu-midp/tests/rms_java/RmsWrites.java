import javax.microedition.rms.RecordStore;

/**
 * Saves in bulk. "small" adds args[1] records of 100 bytes one at a time;
 * "heavy" adds one record of args[1] bytes and rewrites it args[2] times.
 */
public class RmsWrites extends RmsTest {
    public static void main(String[] args) throws Exception {
        if (args[0].equals("small")) {
            small(Integer.parseInt(args[1]));
        } else {
            heavy(Integer.parseInt(args[1]), Integer.parseInt(args[2]));
        }
    }

    static void small(int count) throws Exception {
        RecordStore rs = RecordStore.openRecordStore("small", true);
        for (int i = 0; i < count; i++) {
            rs.addRecord(new byte[100], 0, 100);
        }
        say("records " + rs.getNumRecords());
        rs.closeRecordStore();
    }

    static void heavy(int size, int times) throws Exception {
        RecordStore rs = RecordStore.openRecordStore("heavy", true);
        byte[] data = new byte[size];
        int id = rs.addRecord(data, 0, size);
        say("added " + id);
        for (int i = 1; i <= times; i++) {
            data[0] = (byte) i;
            rs.setRecord(id, data, 0, size);
        }
        say("rewrote " + times);
        rs.closeRecordStore();

        rs = RecordStore.openRecordStore("heavy", false);
        say("last " + rs.getRecord(id)[0] + " of " + rs.getRecordSize(id));
        rs.closeRecordStore();
    }
}
