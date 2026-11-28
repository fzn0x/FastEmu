package fastemu.bench;

/**
 * Runs one repetition of a bench kernel with the sizes Bench.java gives
 * it at scale k, for the virtual machine's kernel test. The ids are the
 * test's order: cpu.int, cpu.call, cpu.sieve, cpu.sort, cpu.actors,
 * cpu.long, cpu.float, cpu.string, cpu.collections, mem.small,
 * mem.arrays, mem.gc.
 */
public final class Driver {
    private static Kernels.Link live;

    private Driver() {}

    /** Builds what a kernel keeps across its repetitions: mem.gc's live set. */
    public static void prepare(int id, int k) {
        if (id == 11) {
            live = Kernels.liveSet(2000 * k);
        }
    }

    public static long run(int id, int k) {
        switch (id) {
            case 0:
                return Kernels.intMix(100000 * k);
            case 1: {
                long s = 0;
                for (int i = 0; i < k / 10 + 1; i++) {
                    s += Kernels.fib(25);
                }
                return s;
            }
            case 2:
                return Kernels.sieve(100000, k / 4 + 1);
            case 3:
                return Kernels.sort(20000, k / 20 + 1);
            case 4:
                return Kernels.actors(500, 20 * k);
            case 5:
                return Kernels.longMath(50000 * k);
            case 6:
                return Kernels.floatMath(50000 * k);
            case 7:
                return Kernels.strings(1000 * k);
            case 8:
                return Kernels.collections(2000 * k);
            case 9:
                return Kernels.allocSmall(10000 * k);
            case 10:
                return Kernels.allocArrays(1000 * k);
            case 11:
                System.gc();
                return live.value + 1;
            default:
                throw new IllegalArgumentException("no kernel " + id);
        }
    }
}
