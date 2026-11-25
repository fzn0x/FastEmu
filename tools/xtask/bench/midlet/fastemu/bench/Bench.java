package fastemu.bench;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Font;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;
import javax.microedition.lcdui.game.Sprite;
import javax.microedition.midlet.MIDlet;

/**
 * The benchmark MIDlet of `cargo xtask bench`. It prints one
 * "BENCH name key=value ..." line on System.out per result, on FastEmu
 * and on MicroEmulator alike, timed by its own clock.
 *
 * Application properties:
 * Bench-Mode: "kernels" (default) runs every kernel once, then exits;
 * "scene" draws a game-like scene at 30 frames a second until stopped.
 * Bench-Scale (default 100) scales the work of one repetition.
 * Bench-Rounds (default 5) and Bench-Round-Ms (default 500): each kernel
 * runs that many rounds, each round repeating it until that many
 * milliseconds passed.
 * Bench-Linger-Ms (default 0): how long the MIDlet waits after "done"
 * before it exits, so the bench can read its process counters.
 * Bench-Fail: the name of a kernel that throws, to test that one failed
 * kernel does not end the others.
 */
public final class Bench extends MIDlet implements Runnable {
    private static int rounds = 5;
    private static int roundMs = 500;
    private static String fail = "";

    private Scene canvas;
    private int scale = 100;
    private int lingerMs;
    private boolean scene;
    private volatile boolean stopped;

    static void line(String name, String fields) {
        System.out.println("BENCH " + name + " " + fields);
    }

    protected void startApp() {
        if (canvas != null) {
            return;
        }
        scale = number(getAppProperty("Bench-Scale"), 100);
        rounds = number(getAppProperty("Bench-Rounds"), 5);
        roundMs = number(getAppProperty("Bench-Round-Ms"), 500);
        lingerMs = number(getAppProperty("Bench-Linger-Ms"), 0);
        String failing = getAppProperty("Bench-Fail");
        fail = failing == null ? "" : failing.trim();
        scene = "scene".equals(getAppProperty("Bench-Mode"));
        canvas = new Scene();
        canvas.setFullScreenMode(true);
        Display.getDisplay(this).setCurrent(canvas);
        new Thread(this).start();
    }

    private static int number(String text, int fallback) {
        return text == null ? fallback : Integer.parseInt(text.trim());
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {
        stopped = true;
    }

    public void run() {
        try {
            while (!canvas.painted) {
                canvas.repaint();
                Thread.sleep(5);
            }
            line("screen", "w=" + canvas.getWidth() + " h=" + canvas.getHeight());
            line("clock", "step=" + clockStep());
            if (scene) {
                paced();
                return;
            }
            cpu();
            memory();
            draw();
            frames();
            Runtime rt = Runtime.getRuntime();
            line("heap", "total=" + rt.totalMemory() + " free=" + rt.freeMemory());
            line("done", "rounds=" + rounds);
            if (lingerMs > 0) {
                Thread.sleep(lingerMs);
            }
        } catch (Throwable t) {
            line("error", "name=run what=" + text(t));
        }
        notifyDestroyed();
    }

    /** A throwable as one word, for a "key=value" field. */
    private static String text(Throwable t) {
        return t.toString().replace(' ', '_');
    }

    /**
     * The step of System.currentTimeMillis: the largest of ten steps
     * between two changes of its value. A precise clock steps 1 ms; the
     * Windows system clock that HotSpot reads can step up to 16 ms.
     */
    private static long clockStep() {
        long last = System.currentTimeMillis();
        long changed = -1;
        long step = 0;
        int seen = 0;
        while (seen < 10) {
            long now = System.currentTimeMillis();
            if (now != last) {
                if (changed >= 0 && now - changed > step) {
                    step = now - changed;
                }
                if (changed >= 0) {
                    seen++;
                }
                changed = now;
                last = now;
            }
        }
        return step;
    }

    /** One repetition of a kernel; returns its checksum. */
    private interface Work {
        long run();
    }

    /**
     * Runs a kernel for `rounds` rounds. Each round repeats the kernel
     * until at least `roundMs` passed, so a fast machine is timed over
     * many repetitions and the clock's step costs about 3 percent at
     * most. `first` is the round's first repetition alone: the cold time.
     * A kernel that throws prints an error line, and the next one runs.
     */
    private static void rounds(String name, Work work) {
        try {
            for (int r = 1; r <= rounds; r++) {
                long t0 = System.currentTimeMillis();
                if (name.equals(fail)) {
                    throw new IllegalStateException("Bench-Fail");
                }
                long check = work.run();
                long first = System.currentTimeMillis() - t0;
                int reps = 1;
                long ms = first;
                while (ms < roundMs) {
                    check = work.run();
                    reps++;
                    ms = System.currentTimeMillis() - t0;
                }
                line(name, "round=" + r + " reps=" + reps + " ms=" + ms + " first=" + first + " check=" + check);
            }
        } catch (Throwable t) {
            line("error", "name=" + name + " what=" + text(t));
        }
    }

    private void cpu() {
        final int k = scale;
        rounds("cpu.int", new Work() { public long run() { return Kernels.intMix(100000 * k); } });
        rounds("cpu.call", new Work() { public long run() { return fibs(k); } });
        rounds("cpu.sieve", new Work() { public long run() { return Kernels.sieve(100000, k / 4 + 1); } });
        rounds("cpu.sort", new Work() { public long run() { return Kernels.sort(20000, k / 20 + 1); } });
        rounds("cpu.actors", new Work() { public long run() { return Kernels.actors(500, 20 * k); } });
        rounds("cpu.long", new Work() { public long run() { return Kernels.longMath(50000 * k); } });
        rounds("cpu.float", new Work() { public long run() { return Kernels.floatMath(50000 * k); } });
        rounds("cpu.string", new Work() { public long run() { return Kernels.strings(1000 * k); } });
        rounds("cpu.collections", new Work() { public long run() { return Kernels.collections(2000 * k); } });
    }

    private static long fibs(int k) {
        long s = 0;
        for (int i = 0; i < k / 10 + 1; i++) {
            s += Kernels.fib(25);
        }
        return s;
    }

    private void memory() {
        final int k = scale;
        rounds("mem.small", new Work() { public long run() { return Kernels.allocSmall(10000 * k); } });
        rounds("mem.arrays", new Work() { public long run() { return Kernels.allocArrays(1000 * k); } });
        // One full collection per repetition, with a live set to trace.
        final Kernels.Link[] live = { Kernels.liveSet(2000 * k) };
        rounds("mem.gc", new Work() { public long run() {
            System.gc();
            return live[0].value + 1;
        } });
        live[0] = null;
    }

    private void draw() {
        final int k = scale;
        final Image buf = Image.createImage(240, 320);
        final Graphics g = buf.getGraphics();
        final Image opaque = Scene.makeOpaque();
        final Image alpha = Scene.makeAlpha();
        final int[] argb = Scene.argb(64, 64);
        rounds("draw.fillrect", new Work() { public long run() {
            int x = 7;
            for (int i = 0; i < 4000 * k; i++) {
                x = x * 1103515245 + 12345;
                g.setColor(x >>> 8);
                g.fillRect((x >>> 4) % 200, (x >>> 12) % 280, 8 + (x >>> 20) % 56, 8 + (x >>> 24) % 56);
            }
            return buf.getWidth();
        } });
        rounds("draw.line", new Work() { public long run() {
            int x = 11;
            for (int i = 0; i < 4000 * k; i++) {
                x = x * 1103515245 + 12345;
                g.setColor(x >>> 8);
                g.drawLine((x >>> 4) % 240, (x >>> 12) % 320, (x >>> 18) % 240, (x >>> 23) % 320);
            }
            return 0;
        } });
        rounds("draw.image", new Work() { public long run() {
            for (int i = 0; i < 4000 * k; i++) {
                g.drawImage(opaque, (i * 37) % 220, (i * 53) % 300, Graphics.TOP | Graphics.LEFT);
            }
            return 0;
        } });
        rounds("draw.alpha", new Work() { public long run() {
            for (int i = 0; i < 2000 * k; i++) {
                g.drawImage(alpha, (i * 37) % 220, (i * 53) % 300, Graphics.TOP | Graphics.LEFT);
            }
            return 0;
        } });
        rounds("draw.region", new Work() { public long run() {
            for (int i = 0; i < 2000 * k; i++) {
                g.drawRegion(alpha, 0, 0, 32, 32, (i & 1) == 0 ? Sprite.TRANS_ROT90 : Sprite.TRANS_MIRROR,
                        (i * 37) % 220, (i * 53) % 300, Graphics.TOP | Graphics.LEFT);
            }
            return 0;
        } });
        rounds("draw.text", new Work() { public long run() {
            g.setFont(Font.getDefaultFont());
            for (int i = 0; i < 1000 * k; i++) {
                g.setColor(i * 7919);
                g.drawString("Score 1234567", (i * 37) % 200, (i * 53) % 300, Graphics.TOP | Graphics.LEFT);
            }
            return 0;
        } });
        rounds("draw.rgb", new Work() { public long run() {
            for (int i = 0; i < 400 * k; i++) {
                g.drawRGB(argb, 0, 64, (i * 37) % 176, (i * 53) % 256, 64, 64, true);
            }
            return 0;
        } });
        // The scene of the frame loop, drawn off screen: the cost of one
        // frame's painting, timed over many frames.
        rounds("draw.scene", new Work() {
            private int f;

            public long run() {
                Scene.draw(g, 240, 320, f++, opaque, alpha);
                return 0;
            }
        });
    }

    /** Repaints as fast as the emulator lets, for at least 6 x roundMs. */
    private void frames() {
        try {
            long t0 = System.currentTimeMillis();
            long until = roundMs * 6L;
            int n = 0;
            long ms = 0;
            while (n < 6 || ms < until) {
                canvas.frame = n++;
                canvas.repaint();
                canvas.serviceRepaints();
                ms = System.currentTimeMillis() - t0;
            }
            line("frames.free", "frames=" + n + " ms=" + ms);
        } catch (Throwable t) {
            line("error", "name=frames.free what=" + text(t));
        }
    }

    /** The scene at 30 frames a second, until the MIDlet is stopped. */
    private void paced() throws InterruptedException {
        long start = System.currentTimeMillis();
        long n = 0;
        long shown = 0;
        long since = start;
        while (!stopped) {
            canvas.frame++;
            canvas.repaint();
            canvas.serviceRepaints();
            shown++;
            n++;
            long next = start + n * 1000 / 30;
            long now = System.currentTimeMillis();
            if (next > now) {
                Thread.sleep(next - now);
            }
            if (now - since >= 5000) {
                line("scene", "frames=" + shown + " ms=" + (now - since));
                shown = 0;
                since = now;
            }
        }
    }

    /** A full-screen canvas that draws a game-like scene. */
    static final class Scene extends Canvas {
        volatile boolean painted;
        volatile int frame;
        private final Image opaque = makeOpaque();
        private final Image alpha = makeAlpha();

        static Image makeOpaque() {
            Image img = Image.createImage(32, 32);
            Graphics g = img.getGraphics();
            for (int y = 0; y < 32; y += 4) {
                g.setColor(0x203040 + y * 0x060402);
                g.fillRect(0, y, 32, 4);
            }
            g.setColor(0xffffff);
            g.drawRect(0, 0, 31, 31);
            return img;
        }

        static int[] argb(int w, int h) {
            int[] px = new int[w * h];
            for (int y = 0; y < h; y++) {
                for (int x = 0; x < w; x++) {
                    int dx = x - w / 2;
                    int dy = y - h / 2;
                    int a = dx * dx + dy * dy < (w / 2) * (w / 2) ? 0xff : 0x40;
                    px[y * w + x] = (a << 24) | ((x * 8) << 16) | ((y * 8) << 8) | 0x80;
                }
            }
            return px;
        }

        static Image makeAlpha() {
            return Image.createRGBImage(argb(32, 32), 32, 32, true);
        }

        /** 48 sprites, some with alpha, some mirrored, and a line of text. */
        static void draw(Graphics g, int w, int h, int f, Image opaque, Image alpha) {
            g.setColor(0x102030);
            g.fillRect(0, 0, w, h);
            for (int i = 0; i < 48; i++) {
                int x = (i * 37 + f * (i % 5 + 1)) % 220;
                int y = (i * 53 + f * (i % 3 + 1)) % 300;
                if (i % 3 == 0) {
                    g.drawRegion(alpha, 0, 0, 32, 32, Sprite.TRANS_MIRROR, x, y, Graphics.TOP | Graphics.LEFT);
                } else {
                    g.drawImage((i & 1) == 0 ? opaque : alpha, x, y, Graphics.TOP | Graphics.LEFT);
                }
            }
            g.setColor(0xffffff);
            g.drawString("Frame " + f, 4, 4, Graphics.TOP | Graphics.LEFT);
        }

        protected void paint(Graphics g) {
            draw(g, getWidth(), getHeight(), frame, opaque, alpha);
            painted = true;
        }
    }
}
