package fastemu.bench;

import java.util.Hashtable;
import java.util.Vector;

/**
 * CPU and memory kernels in plain CLDC 1.1. Each does a fixed amount of
 * work for its size and returns a checksum, so no compiler can drop the
 * work and two machines can be checked for the same answer.
 */
final class Kernels {
    private Kernels() {}

    /** Integer arithmetic: xorshift and adds. */
    static int intMix(int n) {
        int x = 0x12345678;
        int s = 0;
        for (int i = 0; i < n; i++) {
            x ^= x << 13;
            x ^= x >>> 17;
            x ^= x << 5;
            s += x & 0xff;
        }
        return s;
    }

    /** Method calls: naive Fibonacci. */
    static int fib(int n) {
        return n < 2 ? n : fib(n - 1) + fib(n - 2);
    }

    /** Arrays: a sieve of Eratosthenes up to n, repeated. */
    static int sieve(int n, int times) {
        int count = 0;
        for (int t = 0; t < times; t++) {
            boolean[] composite = new boolean[n + 1];
            for (int i = 2; i <= n; i++) {
                if (!composite[i]) {
                    count++;
                    for (int j = i + i; j <= n; j += i) {
                        composite[j] = true;
                    }
                }
            }
        }
        return count;
    }

    /** Arrays: heap sort of n pseudo random ints. */
    static int sort(int n, int times) {
        int check = 0;
        for (int t = 0; t < times; t++) {
            int[] a = new int[n];
            int x = 0x2545F491 + t;
            for (int i = 0; i < n; i++) {
                x ^= x << 13;
                x ^= x >>> 17;
                x ^= x << 5;
                a[i] = x;
            }
            for (int i = n / 2 - 1; i >= 0; i--) {
                down(a, i, n);
            }
            for (int end = n - 1; end > 0; end--) {
                int tmp = a[0];
                a[0] = a[end];
                a[end] = tmp;
                down(a, 0, end);
            }
            check = check * 31 + a[0] + a[n / 2] + a[n - 1];
        }
        return check;
    }

    private static void down(int[] a, int i, int n) {
        while (true) {
            int c = 2 * i + 1;
            if (c >= n) {
                return;
            }
            if (c + 1 < n && a[c + 1] > a[c]) {
                c++;
            }
            if (a[i] >= a[c]) {
                return;
            }
            int tmp = a[i];
            a[i] = a[c];
            a[c] = tmp;
            i = c;
        }
    }

    /** Objects: fields and virtual calls, as a game's actors update. */
    static int actors(int count, int steps) {
        Actor[] actors = new Actor[count];
        for (int i = 0; i < count; i++) {
            actors[i] = (i & 1) == 0 ? new Actor(i) : new Bouncer(i);
        }
        int s = 0;
        for (int step = 0; step < steps; step++) {
            for (int i = 0; i < count; i++) {
                actors[i].update();
                s += actors[i].x ^ actors[i].y;
            }
        }
        return s;
    }

    static class Actor {
        int x;
        int y;
        int vx;
        int vy;

        Actor(int seed) {
            x = seed * 7 % 240;
            y = seed * 13 % 320;
            vx = seed % 5 - 2;
            vy = seed % 7 - 3;
        }

        void update() {
            x += vx;
            y += vy;
            if (x < 0 || x >= 240) {
                vx = -vx;
            }
            if (y < 0 || y >= 320) {
                vy = -vy;
            }
        }
    }

    static final class Bouncer extends Actor {
        Bouncer(int seed) {
            super(seed);
        }

        void update() {
            super.update();
            vy += 1;
            if (vy > 6) {
                vy = -6;
            }
        }
    }

    /** 64-bit integer and fixed point math. */
    static long longMath(int n) {
        long a = 1;
        long b = 0x9E3779B97F4A7C15L;
        for (int i = 0; i < n; i++) {
            a = a * 6364136223846793005L + 1442695040888963407L;
            b ^= a >>> 29;
            b += (a >> 16) * 3;
        }
        return a ^ b;
    }

    /** Float and double math, as CLDC 1.1 games use them. */
    static int floatMath(int n) {
        float f = 0.5f;
        double d = 0.25;
        for (int i = 0; i < n; i++) {
            f = f * 1.0001f + 0.5f / (i + 1);
            d = d * 0.9999 + (double) f / 3.0;
            if (f > 1000f) {
                f -= 999f;
            }
        }
        return (int) (f * 1000) ^ (int) (d * 1000);
    }

    /** Strings: StringBuffer appends, toString, charAt and indexOf. */
    static int strings(int n) {
        int s = 0;
        for (int i = 0; i < n; i++) {
            StringBuffer sb = new StringBuffer();
            sb.append("score ").append(i).append(" lives ").append(i & 7).append('!');
            String text = sb.toString();
            s += text.length() + text.charAt(text.length() / 2) + text.indexOf("lives");
        }
        return s;
    }

    /** Hashtable and Vector with boxed keys. */
    static int collections(int n) {
        Hashtable table = new Hashtable();
        Vector list = new Vector();
        int s = 0;
        for (int i = 0; i < n; i++) {
            Integer key = new Integer(i & 1023);
            Object old = table.put(key, key);
            if (old != null) {
                s += ((Integer) old).intValue();
            }
            list.addElement(key);
            if (list.size() > 256) {
                list.removeElementAt(0);
            }
        }
        return s + table.size() + list.size();
    }

    /** Small objects: n allocations, the last 4096 kept alive in a ring. */
    static int allocSmall(int n) {
        Node[] ring = new Node[4096];
        int s = 0;
        for (int i = 0; i < n; i++) {
            Node node = new Node(i, ring[(i - 1) & 4095]);
            ring[i & 4095] = node;
            s += node.value;
        }
        return s;
    }

    static final class Node {
        final int value;
        final Node next;

        Node(int value, Node next) {
            this.value = value;
            this.next = next == null ? null : next.next;
        }
    }

    /** Arrays of mixed sizes, 16 bytes to 4 KiB, the last 128 kept. */
    static int allocArrays(int n) {
        byte[][] ring = new byte[128][];
        int x = 0x1234567;
        int s = 0;
        for (int i = 0; i < n; i++) {
            x ^= x << 13;
            x ^= x >>> 17;
            x ^= x << 5;
            byte[] b = new byte[16 + ((x >>> 8) & 4095)];
            b[b.length - 1] = (byte) i;
            ring[i & 127] = b;
            s += b.length;
        }
        return s;
    }

    /** A live set of n linked objects, for a full collection to trace. */
    static Link liveSet(int n) {
        Link head = null;
        for (int i = 0; i < n; i++) {
            head = new Link(i, head);
        }
        return head;
    }

    static final class Link {
        final int value;
        final Link next;

        Link(int value, Link next) {
            this.value = value;
            this.next = next;
        }
    }
}
