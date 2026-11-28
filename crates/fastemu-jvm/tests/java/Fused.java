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

    public static void main(String[] args) {
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
