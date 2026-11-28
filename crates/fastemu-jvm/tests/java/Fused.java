// The sequences the interpreter fuses into one instruction, at the edges:
// shifts by 0, 31, 32, 33 and -1, MIN_VALUE and MAX_VALUE, overflow, and
// loops that count up, down and by steps.
public class Fused {
    static final int[] VALUES = {
        0, 1, -1, 2, 7, 31, 32, 33, -32, 100, 123456789, Integer.MAX_VALUE, Integer.MIN_VALUE,
    };

    static void pairs(int a, int b) {
        int add = a + b;
        int sub = a - b;
        int mul = a * b;
        int and = a & b;
        int or = a | b;
        int xor = a ^ b;
        int shl = a << b;
        int shr = a >> b;
        int ushr = a >>> b;
        int mix = add + sub * mul - (and | or) ^ xor;
        System.out.println(a + " " + b + ": " + add + " " + sub + " " + mul + " " + and + " "
            + or + " " + xor + " " + shl + " " + shr + " " + ushr + " " + mix);
    }

    static void constants(int a) {
        int s0 = a << 0;
        int s31 = a << 31;
        int s32 = a << 32;
        int s33 = a << 33;
        int sm = a << -1;
        int r31 = a >> 31;
        int u33 = a >>> 33;
        int add = a + 100;
        int sub = a - 32768;
        int mul = a * -7;
        int and = a & 0xff;
        int or = a | 0x80000000;
        int xor = a ^ -1;
        System.out.println(a + ": " + s0 + " " + s31 + " " + s32 + " " + s33 + " " + sm + " "
            + r31 + " " + u33 + " " + add + " " + sub + " " + mul + " " + and + " " + or + " "
            + xor);
    }

    static int compares(int a, int b) {
        int r = 0;
        if (a == b) r |= 1;
        if (a != b) r |= 2;
        if (a < b) r |= 4;
        if (a >= b) r |= 8;
        if (a > b) r |= 16;
        if (a <= b) r |= 32;
        if (a < 10) r |= 64;
        if (a >= -5) r |= 128;
        if (a == 0x7fffffff) r |= 256;
        if (a != -1) r |= 512;
        if (a > 32) r |= 1024;
        if (a <= -32) r |= 2048;
        return r;
    }

    static int loops(int n) {
        int s = 0;
        for (int i = 0; i < n; i++) {
            s += i ^ (s << 1);
        }
        for (int i = n; i > 0; i -= 3) {
            s -= i;
        }
        int j = 0;
        while (j <= 100) {
            j += 7;
            s ^= j;
        }
        int k = n;
        while (true) {
            if (k < 5) {
                break;
            }
            k -= 4;
            s += k;
        }
        return s;
    }

    static final long[] LONGS = { 0L, 1L, -1L, 40L, Long.MAX_VALUE, Long.MIN_VALUE, 0x123456789abcdefL };
    static final double[] DOUBLES = {
        0.0, -0.0, 1.5, -2.25, 1e300, 1e-300, Double.NaN,
        Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY,
    };

    static void longs(long a, long b) {
        long add = a + 3L;
        long sub = a - 0x7fffffffffffL;
        long mul = a * 6364136223846793005L;
        long and = a & 0xffL;
        long or = a | Long.MIN_VALUE;
        long xor = a ^ -1L;
        long mix = add + b;
        mix = mix * b;
        int r = 0;
        if (a < b) r |= 1;
        if (a >= b) r |= 2;
        if (a == b) r |= 4;
        if (a != b) r |= 8;
        if (a > b) r |= 16;
        if (a <= b) r |= 32;
        System.out.println(a + " " + b + ": " + add + " " + sub + " " + mul + " " + and + " "
            + or + " " + xor + " " + mix + " " + r);
    }

    static void floats(double x, double y) {
        float f = (float) x;
        float g = (float) y;
        float fa = f + 1.5f;
        float fs = f - 0.25f;
        float fm = f * 2f;
        float fd = f / 3f;
        float fr = f % 7f;
        float fmix = fa + g;
        fmix = fmix * g;
        double da = x + 1.5;
        double ds = x - 0.25;
        double dm = x * 2.0;
        double dd = x / 3.0;
        double dr = x % 7.0;
        double dmix = da + y;
        dmix = dmix / y;
        int r = 0;
        if (f < g) r |= 1;
        if (f > g) r |= 2;
        if (f <= g) r |= 4;
        if (f >= g) r |= 8;
        if (f == g) r |= 16;
        if (f != g) r |= 32;
        if (x < y) r |= 64;
        if (x > y) r |= 128;
        if (x <= y) r |= 256;
        if (x >= y) r |= 512;
        if (x == y) r |= 1024;
        if (x != y) r |= 2048;
        System.out.println(x + " " + y + ": " + fa + " " + fs + " " + fm + " " + fd + " " + fr
            + " " + fmix + " " + da + " " + ds + " " + dm + " " + dd + " " + dr + " " + dmix
            + " " + r);
    }

    public static void main(String[] args) {
        for (int i = 0; i < LONGS.length; i++) {
            for (int k = 0; k < LONGS.length; k++) {
                longs(LONGS[i], LONGS[k]);
            }
        }
        for (int i = 0; i < DOUBLES.length; i++) {
            for (int k = 0; k < DOUBLES.length; k++) {
                floats(DOUBLES[i], DOUBLES[k]);
            }
        }
        for (int i = 0; i < VALUES.length; i++) {
            StringBuffer line = new StringBuffer();
            for (int k = 0; k < VALUES.length; k++) {
                pairs(VALUES[i], VALUES[k]);
                line.append(compares(VALUES[i], VALUES[k])).append(' ');
            }
            System.out.println(line);
            constants(VALUES[i]);
        }
        System.out.println(loops(1000));
        System.out.println(loops(0));
        System.out.println(loops(-3));
    }
}
