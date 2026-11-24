// Compile-time stub for the test harness's native class (see
// tests/media_support/mod.rs): it moves the offline audio engine's time.
public final class Pump {
    private Pump() {
    }

    /** Renders and discards ms milliseconds of the mix. */
    public static void advance(int ms) {
    }

    /** Renders ms milliseconds and returns the loudest sample times 1000. */
    public static int peak(int ms) {
        return 0;
    }

    /** How many players the engine holds. */
    public static int players() {
        return 0;
    }
}
