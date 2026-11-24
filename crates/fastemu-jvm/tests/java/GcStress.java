// Allocates far more than the heap holds while keeping a small live set,
// so the collector must free garbage and keep every live object intact.
import java.lang.ref.WeakReference;

public class GcStress {
    static class Node {
        final int value;
        Node next;
        int[] payload;

        Node(int value, Node next) {
            this.value = value;
            this.next = next;
            this.payload = new int[] {value, value * 2};
        }
    }

    public static void main(String[] args) {
        Node live = null;
        for (int i = 0; i < 2000; i++) {
            live = new Node(i, live);
        }
        long garbageBytes = 0;
        String keep = "";
        for (int round = 0; round < 3000; round++) {
            int[] chunk = new int[4096];
            chunk[round % chunk.length] = round;
            garbageBytes += chunk.length * 4;
            StringBuffer sb = new StringBuffer();
            for (int k = 0; k < 20; k++) {
                sb.append(k).append(',');
            }
            if (round % 500 == 0) {
                keep = keep + sb.toString().length() + ";";
            }
            Object[] tree = new Object[8];
            for (int k = 0; k < tree.length; k++) {
                tree[k] = new byte[100 + k];
            }
        }
        long total = 0;
        int count = 0;
        for (Node n = live; n != null; n = n.next) {
            total += n.value + n.payload[1];
            count++;
        }
        System.out.println("allocated more than 40 MB: " + (garbageBytes > 40000000));
        System.out.println("live nodes " + count + " total " + total);
        System.out.println("kept " + keep);
        Object target = new Object();
        WeakReference weak = new WeakReference(target);
        System.out.println("weak before " + (weak.get() == target));
        Runtime rt = Runtime.getRuntime();
        System.gc();
        System.out.println("memory sane " + (rt.freeMemory() > 0 && rt.freeMemory() <= rt.totalMemory()));
        try {
            int[][] hog = new int[1000][];
            for (int i = 0; i < hog.length; i++) {
                hog[i] = new int[1000000];
            }
            System.out.println("no OutOfMemoryError " + hog.length);
        } catch (OutOfMemoryError e) {
            System.out.println("OutOfMemoryError caught");
        }
        int[] after = new int[1000];
        after[999] = 5;
        System.out.println("allocation works after OOM " + after[999] + " weak still " + (weak.get() == target));
    }
}
