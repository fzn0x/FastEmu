// Integer and long arithmetic at the edges: overflow, division, shifts,
// narrowing conversions and comparisons.
public class Arith {
    static int idiv(int a, int b) {
        try {
            return a / b;
        } catch (ArithmeticException e) {
            System.out.println("idiv " + a + "/" + b + ": " + e.getMessage());
            return 0;
        }
    }

    static long ldiv(long a, long b) {
        try {
            return a % b;
        } catch (ArithmeticException e) {
            System.out.println("lrem " + a + "%" + b + ": " + e.getMessage());
            return 0;
        }
    }

    public static void main(String[] args) {
        int max = Integer.MAX_VALUE;
        int min = Integer.MIN_VALUE;
        System.out.println(max + 1);
        System.out.println(min - 1);
        System.out.println(max * 2);
        System.out.println(min / -1);
        System.out.println(min % -1);
        System.out.println(-7 / 2);
        System.out.println(-7 % 2);
        System.out.println(7 % -2);
        System.out.println(-min);
        System.out.println(idiv(5, 0));
        System.out.println(idiv(-9, 4));

        long lmax = Long.MAX_VALUE;
        long lmin = Long.MIN_VALUE;
        System.out.println(lmax + 1);
        System.out.println(lmin / -1);
        System.out.println(lmin % -1L);
        System.out.println(ldiv(10, 0));
        System.out.println(ldiv(-10, 3));
        System.out.println(lmax * lmax);
        System.out.println(123456789L * 987654321L);

        int x = 0x12345678;
        for (int s = -1; s <= 33; s += 17) {
            System.out.println("shift " + s + ": " + (x << s) + " " + (x >> s) + " " + (x >>> s));
        }
        int neg = -12345;
        System.out.println((neg >> 4) + " " + (neg >>> 4) + " " + (neg << 28));
        long y = 0x123456789ABCDEFL;
        for (int s = -1; s <= 65; s += 33) {
            System.out.println("lshift " + s + ": " + (y << s) + " " + (y >> s) + " " + (y >>> s));
        }
        System.out.println((-1L >>> 1) + " " + (-1L >> 63) + " " + (1L << 63));

        System.out.println((byte) 200);
        System.out.println((short) 70000);
        System.out.println((char) 65 + "" + (int) (char) -1);
        System.out.println((int) 3000000000L);
        System.out.println((long) -1 & 0xFFFFFFFFL);
        System.out.println((byte) -129 + " " + (short) -32769);
        char c = 'A';
        c += 2;
        c++;
        System.out.println(c);
        byte b = 127;
        b++;
        System.out.println(b);
        short sh = -32768;
        sh--;
        System.out.println(sh);

        System.out.println((5 & 3) + " " + (5 | 3) + " " + (5 ^ 3) + " " + (~5));
        System.out.println((5L & 3L) + " " + (5L | 3L) + " " + (5L ^ 3L) + " " + (~5L));
        long a1 = 5, a2 = 7;
        System.out.println((a1 < a2) + " " + (a1 > a2) + " " + (a1 == a2) + " " + (a1 != a2));
        System.out.println(Long.MIN_VALUE < Long.MAX_VALUE);

        int acc = 1;
        for (int i = 0; i < 40; i++) {
            acc = acc * 31 + i;
        }
        System.out.println(acc);
        long lacc = 1;
        for (int i = 0; i < 70; i++) {
            lacc = lacc * 1000003 ^ i;
        }
        System.out.println(lacc);

        int i = 10;
        i += i++ + ++i;
        System.out.println(i);
        int j = 5;
        j = j++ * 2 + j--;
        System.out.println(j);
        int[] counts = new int[3];
        counts[1] += 5;
        counts[1] <<= 2;
        counts[2] -= counts[1]--;
        System.out.println(counts[0] + " " + counts[1] + " " + counts[2]);
        boolean t = true, f = false;
        System.out.println((t & f) + " " + (t | f) + " " + (t ^ f) + " " + !t);
        System.out.println(Math.abs(Integer.MIN_VALUE) + " " + Math.abs(-5L) + " " + Math.max(3, 9) + " " + Math.min(-3L, 4L));
    }
}
