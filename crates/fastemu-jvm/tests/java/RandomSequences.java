import java.util.Random;

// java.util.Random with fixed seeds: games build levels from seeded
// sequences, so every value must be the JDK's. A game's subclass that
// overrides next(int) and setSeed must see the same calls.
public class RandomSequences {
    static class Scripted extends Random {
        int calls;
        int seeds;

        Scripted() {
            super(5);
        }

        protected int next(int bits) {
            calls++;
            return (calls * 0x9E3779B9) >>> (32 - bits);
        }

        public void setSeed(long seed) {
            seeds++;
            super.setSeed(seed);
        }
    }

    public static void main(String[] args) {
        long[] seeds = {0, 1, 42, -1, 123456789L, Long.MIN_VALUE, 0x5DEECE66DL};
        for (int i = 0; i < seeds.length; i++) {
            Random r = new Random(seeds[i]);
            StringBuffer sb = new StringBuffer();
            sb.append(seeds[i]).append(':');
            sb.append(' ').append(r.nextInt());
            sb.append(' ').append(r.nextInt(6));
            sb.append(' ').append(r.nextInt(100));
            sb.append(' ').append(r.nextInt(1 << 16));
            sb.append(' ').append(r.nextInt(Integer.MAX_VALUE));
            sb.append(' ').append(r.nextLong());
            sb.append(' ').append(r.nextFloat());
            sb.append(' ').append(r.nextDouble());
            System.out.println(sb);
        }

        Random level = new Random(2008);
        StringBuffer map = new StringBuffer();
        for (int y = 0; y < 6; y++) {
            for (int x = 0; x < 30; x++) {
                map.append(".#~^".charAt(level.nextInt(4)));
            }
            map.append('\n');
        }
        System.out.print(map);

        Random reseeded = new Random(1);
        int first = reseeded.nextInt();
        reseeded.nextInt();
        reseeded.setSeed(1);
        System.out.println(first == reseeded.nextInt());

        long sum = 0;
        Random many = new Random(77);
        for (int i = 0; i < 100000; i++) {
            sum += many.nextInt(1000) + (many.nextInt() >>> 20);
        }
        System.out.println("sum " + sum);

        Scripted s = new Scripted();
        System.out.println(s.seeds + " " + s.nextInt() + " " + s.nextInt(10) + " " + s.nextInt(16) + " " + s.nextLong()
            + " " + s.nextFloat() + " " + s.nextDouble() + " calls " + s.calls);

        try {
            new Random(3).nextInt(0);
        } catch (IllegalArgumentException e) {
            System.out.println("nextInt(0): IllegalArgumentException");
        }
        try {
            new Random(3).nextInt(-5);
        } catch (IllegalArgumentException e) {
            System.out.println("nextInt(-5): IllegalArgumentException");
        }
        Random a = new Random();
        Random b = new Random(a.nextLong());
        System.out.println(b.nextInt(10) < 10);
    }
}
