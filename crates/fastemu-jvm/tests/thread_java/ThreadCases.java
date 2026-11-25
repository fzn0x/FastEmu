import java.util.Timer;
import java.util.TimerTask;

// The limit on threads: Thread.start and new Timer() throw
// OutOfMemoryError once the machine has as many live threads as its
// limit, and what failed leaves nothing behind. Each case prints what it
// saw; the test reads the lines. Main is chosen by the first argument.
public class ThreadCases {
    static final Object gate = new Object();
    static boolean open;

    // Waits for the gate to open, so a thread that starts stays alive.
    static class Sleeper extends Thread {
        public void run() {
            synchronized (gate) {
                while (!open) {
                    try {
                        gate.wait();
                    } catch (InterruptedException e) {
                        return;
                    }
                }
            }
        }
    }

    static class Once extends TimerTask {
        boolean ran;

        public synchronized void run() {
            ran = true;
            notifyAll();
        }
    }

    // Waits until only the threads in `left` are alive.
    static void settle(int left) throws InterruptedException {
        for (int i = 0; i < 2000 && Thread.activeCount() > left; i++) {
            Thread.sleep(10);
        }
    }

    static void threads() throws InterruptedException {
        Thread[] made = new Thread[100];
        int started = 0;
        Thread refused = null;
        String message = null;
        int active = 0;
        for (int i = 0; i < made.length && refused == null; i++) {
            Thread t = new Sleeper();
            try {
                t.start();
                made[started++] = t;
            } catch (OutOfMemoryError e) {
                refused = t;
                message = e.getMessage();
                active = Thread.activeCount();
            }
        }
        System.out.println("threads started " + started + " active " + active);
        System.out.println("message " + message);
        // A thread that did not start can start once there is room.
        synchronized (gate) {
            open = true;
            gate.notifyAll();
        }
        for (int i = 0; i < started; i++) {
            made[i].join();
        }
        refused.start();
        refused.join();
        System.out.println("restarted " + !refused.isAlive());
    }

    static void timers() throws InterruptedException {
        Timer[] made = new Timer[100];
        int started = 0;
        boolean refused = false;
        String message = null;
        for (int i = 0; i < made.length && !refused; i++) {
            try {
                made[started] = new Timer();
                started++;
            } catch (OutOfMemoryError e) {
                refused = true;
                message = e.getMessage();
            }
        }
        System.out.println("timers started " + started + " active " + Thread.activeCount());
        System.out.println("message " + message);
        // A timer that was made works, and cancelling them all gives the
        // threads back.
        Once once = new Once();
        made[0].schedule(once, 1);
        synchronized (once) {
            while (!once.ran) {
                once.wait();
            }
        }
        for (int i = 0; i < started; i++) {
            made[i].cancel();
        }
        settle(1);
        System.out.println("settled " + Thread.activeCount());
        Timer again = new Timer();
        Once second = new Once();
        again.schedule(second, 1);
        synchronized (second) {
            while (!second.ran) {
                second.wait();
            }
        }
        again.cancel();
        System.out.println("again ok");
    }

    static void nul() throws InterruptedException {
        final String name = "bad\u0000name\u0007";
        Thread t = new Thread(new Runnable() {
            public void run() {
                System.out.println("ran " + Thread.currentThread().getName().length());
            }
        }, name);
        t.start();
        t.join();
        System.out.println("name " + (t.getName().equals(name)));
    }

    public static void main(String[] args) throws Exception {
        String mode = args[0];
        if (mode.equals("threads")) {
            threads();
        } else if (mode.equals("timers")) {
            timers();
        } else if (mode.equals("nul")) {
            nul();
        }
        System.out.println("end");
    }
}
