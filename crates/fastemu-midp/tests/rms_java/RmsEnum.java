import javax.microedition.rms.InvalidRecordIDException;
import javax.microedition.rms.RecordComparator;
import javax.microedition.rms.RecordEnumeration;
import javax.microedition.rms.RecordFilter;
import javax.microedition.rms.RecordStore;
import javax.microedition.rms.RecordStoreNotOpenException;

/** Enumeration orders, filters, comparators, cursors and keepUpdated. */
public class RmsEnum extends RmsTest {
    static String ids(RecordEnumeration e) throws Exception {
        StringBuffer out = new StringBuffer();
        while (e.hasNextElement()) {
            if (out.length() > 0) {
                out.append(',');
            }
            out.append(e.nextRecordId());
        }
        return out.toString();
    }

    static int value(byte[] record) {
        return record == null ? -1 : record[0];
    }

    public static void main(String[] args) throws Exception {
        RecordStore rs = RecordStore.openRecordStore("scores", true);
        byte[] values = {50, 10, 40, 10, 30};
        for (int i = 0; i < values.length; i++) {
            rs.addRecord(new byte[] {values[i]}, 0, 1);
        }
        rs.addRecord(null, 0, 0);

        RecordEnumeration all = rs.enumerateRecords(null, null, false);
        say("all " + all.numRecords() + ": " + ids(all));

        RecordComparator ascending = new RecordComparator() {
            public int compare(byte[] a, byte[] b) {
                int x = value(a);
                int y = value(b);
                return x < y ? PRECEDES : x > y ? FOLLOWS : EQUIVALENT;
            }
        };
        say("sorted " + ids(rs.enumerateRecords(null, ascending, false)));

        RecordFilter big = new RecordFilter() {
            public boolean matches(byte[] candidate) {
                return candidate != null && candidate[0] >= 30;
            }
        };
        say("filtered " + ids(rs.enumerateRecords(big, ascending, false)));
        say("filter only " + ids(rs.enumerateRecords(big, null, false)));

        RecordComparator descending = new RecordComparator() {
            public int compare(byte[] a, byte[] b) {
                return value(b) - value(a);
            }
        };
        say("descending " + ids(rs.enumerateRecords(big, descending, false)));

        RecordEnumeration e = rs.enumerateRecords(big, ascending, false);
        say("previous first " + e.previousRecordId());
        e.reset();
        say("next " + e.nextRecordId() + " has previous " + e.hasPreviousElement());
        say("next " + e.nextRecordId() + " previous " + e.previousRecordId());
        say("data " + e.nextRecord()[0]);
        say("last " + e.nextRecordId() + " has next " + e.hasNextElement());
        try {
            e.nextRecordId();
            say("went past the end");
        } catch (InvalidRecordIDException x) {
            say("end " + name(x));
        }
        try {
            e.nextRecord();
        } catch (InvalidRecordIDException x) {
            say("still end " + name(x));
        }
        say("back " + new String(new byte[] {(byte) ('0' + e.previousRecord()[0] / 10)}));
        e.reset();
        say("after reset " + e.nextRecordId());

        RecordEnumeration stale = rs.enumerateRecords(null, null, false);
        rs.deleteRecord(1);
        say("stale count " + stale.numRecords());
        try {
            stale.nextRecord();
        } catch (InvalidRecordIDException x) {
            say("stale record " + name(x));
        }
        say("stale next " + stale.nextRecordId());
        stale.rebuild();
        say("rebuilt " + stale.numRecords() + ": " + ids(stale));

        RecordEnumeration live = rs.enumerateRecords(big, ascending, true);
        say("live " + live.isKeptUpdated() + " " + ids(live));
        say("added " + rs.addRecord(new byte[] {35}, 0, 1) + " "
                + rs.addRecord(new byte[] {5}, 0, 1));
        live.reset();
        say("live after add " + ids(live));
        rs.setRecord(5, new byte[] {45}, 0, 1);
        rs.setRecord(2, new byte[] {60}, 0, 1);
        live.reset();
        say("live after set " + ids(live));
        rs.deleteRecord(3);
        live.reset();
        say("live after delete " + ids(live));
        live.keepUpdated(false);
        rs.deleteRecord(7);
        live.reset();
        say("frozen " + ids(live) + " " + live.isKeptUpdated());

        RecordEnumeration walk = rs.enumerateRecords(null, null, true);
        say("walk " + walk.nextRecordId() + " " + walk.nextRecordId());
        rs.addRecord(new byte[] {1}, 0, 1);
        say("walk on " + walk.nextRecordId() + " of " + walk.numRecords());
        rs.deleteRecord(5);
        say("walk after delete " + walk.nextRecordId() + " back " + walk.previousRecordId());

        walk.destroy();
        try {
            walk.numRecords();
        } catch (IllegalStateException x) {
            say("destroyed " + name(x));
        }
        walk.destroy();
        say("destroyed twice");
        rs.addRecord(new byte[] {2}, 0, 1);
        say("store goes on " + rs.getNumRecords());

        RecordEnumeration open = rs.enumerateRecords(null, null, false);
        rs.closeRecordStore();
        say("closed has next " + open.hasNextElement() + " count " + open.numRecords());
        try {
            open.nextRecord();
        } catch (RecordStoreNotOpenException x) {
            say("closed " + name(x));
        }
        try {
            rs.enumerateRecords(null, null, false);
        } catch (RecordStoreNotOpenException x) {
            say("enumerate closed " + name(x));
        }

        // A walk that rewrites each record it reaches, as games do when
        // they update every save slot, must move on, not see it again.
        RecordStore up = RecordStore.openRecordStore("updates", true);
        for (int i = 0; i < 3; i++) {
            up.addRecord(new byte[] {(byte) i}, 0, 1);
        }
        say("update each " + rewriteAll(up, up.enumerateRecords(null, null, true)));
        say("update sorted " + rewriteAll(up, up.enumerateRecords(null, ascending, true)));
        up.closeRecordStore();
    }

    /** Sets each record the walk reaches to its id; the ids it saw. */
    static String rewriteAll(RecordStore rs, RecordEnumeration e) throws Exception {
        StringBuffer out = new StringBuffer();
        for (int guard = 0; e.hasNextElement() && guard < 10; guard++) {
            int id = e.nextRecordId();
            rs.setRecord(id, new byte[] {(byte) id}, 0, 1);
            if (out.length() > 0) {
                out.append(',');
            }
            out.append(id);
        }
        e.destroy();
        return out.toString();
    }
}
