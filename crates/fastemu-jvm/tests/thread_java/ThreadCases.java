import java.util.Timer;
import java.util.TimerTask;

// The limit on threads: Thread.start and new Timer() throw
// OutOfMemoryError once the machine has as many live threads as its
// limit, and what failed leaves nothing behind. The thread of a timer the
// game dropped ends once no task is left, so dropped timers never use up
// the limit. Each case prints what it saw; the test reads the lines. Main
// is chosen by the first argument.
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

    // A task that keeps the thread it ran on, so the case can join it.
    static class Grab extends TimerTask {
        Thread thread;
        int runs;

        public synchronized void run() {
            thread = Thread.currentThread();
            runs++;
            notifyAll();
        }

        synchronized void await(int n) throws InterruptedException {
            while (runs < n) {
                wait();
            }
        }
    }

    // Timers the game drops at once never use up its threads: at the
    // limit, a collection finds them and their threads end.
    static void dropped() {
        int made = 0;
        try {
            for (int i = 0; i < 32; i++) {
                new Timer();
                made++;
            }
        } catch (OutOfMemoryError e) {
            System.out.println("refused " + e.getMessage());
        }
        System.out.println("dropped " + made);
    }

    // A timer for each effect: it runs one task, the game waits for it,
    // then drops the timer.
    static void used() throws InterruptedException {
        int made = 0;
        try {
            for (int i = 0; i < 32; i++) {
                Grab grab = new Grab();
                new Timer().schedule(grab, 0);
                grab.await(1);
                made++;
            }
        } catch (OutOfMemoryError e) {
            System.out.println("refused " + e.getMessage());
        }
        System.out.println("used " + made);
    }

    // Thread.start finds room too, once the dropped timers end.
    static void start() throws InterruptedException {
        for (int i = 0; i < 7; i++) {
            new Timer();
        }
        Thread t = new Thread(new Runnable() {
            public void run() {
            }
        });
        try {
            t.start();
            t.join();
            System.out.println("started true");
        } catch (OutOfMemoryError e) {
            System.out.println("started false " + e.getMessage());
        }
    }

    // A dropped timer with nothing left to run ends its thread at the
    // next collection.
    static void collected() throws InterruptedException {
        Grab grab = new Grab();
        new Timer().schedule(grab, 0);
        grab.await(1);
        Thread thread = grab.thread;
        System.gc();
        thread.join();
        System.out.println("ended " + !thread.isAlive());
    }

    // A dropped timer runs its repeating task on, and its thread ends
    // once the task is cancelled.
    static void repeating() throws InterruptedException {
        Grab ticks = new Grab();
        new Timer().schedule(ticks, 0, 1);
        ticks.await(1);
        System.gc();
        int before;
        synchronized (ticks) {
            before = ticks.runs;
        }
        ticks.await(before + 3);
        Thread thread = ticks.thread;
        System.out.println("still runs " + thread.isAlive());
        ticks.cancel();
        thread.join();
        System.out.println("ended " + !thread.isAlive());
    }

    static Timer keptTimer;
    static Timer foundTimer;
    static final Object hold = new Object();
    static boolean holding;
    static boolean released;

    // Holds its timer's thread until the case lets it go.
    static class Hold extends TimerTask {
        public void run() {
            synchronized (hold) {
                holding = true;
                hold.notifyAll();
                while (!released) {
                    try {
                        hold.wait();
                    } catch (InterruptedException e) {
                        return;
                    }
                }
            }
        }
    }

    // Holds its timer, and hands it to the case when it runs.
    static class Keeper extends TimerTask {
        final Timer timer;

        Keeper(Timer timer) {
            this.timer = timer;
        }

        public void run() {
            synchronized (hold) {
                foundTimer = timer;
                hold.notifyAll();
            }
        }
    }

    // Timers the game keeps go on working across collections: one in a
    // static field, and one the game reaches only through a task that
    // waits in its queue.
    static void kept() throws InterruptedException {
        keptTimer = new Timer();
        System.gc();
        Once once = new Once();
        keptTimer.schedule(once, 1);
        synchronized (once) {
            while (!once.ran) {
                once.wait();
            }
        }
        keptTimer.cancel();
        System.out.println("kept ok");
        Timer t = new Timer();
        t.schedule(new Hold(), 0);
        t.schedule(new Keeper(t), 0);
        t = null;
        synchronized (hold) {
            while (!holding) {
                hold.wait();
            }
        }
        // The Keeper waits behind the Hold, so it is the only way to the
        // timer while the collection runs.
        System.gc();
        synchronized (hold) {
            released = true;
            hold.notifyAll();
            while (foundTimer == null) {
                hold.wait();
            }
        }
        // The queue is empty now, and the timer still takes a task.
        Once later = new Once();
        foundTimer.schedule(later, 0);
        synchronized (later) {
            while (!later.ran) {
                later.wait();
            }
        }
        foundTimer.cancel();
        System.out.println("task ok");
    }

    static final Object held = new Object();
    static final Object seen = new Object();
    static int reports;

    // Its toString, which the report of an uncaught exception calls,
    // needs the monitor the case holds.
    static class Stuck extends RuntimeException {
        public String toString() {
            synchronized (seen) {
                reports++;
                seen.notifyAll();
            }
            synchronized (held) {
                return "Stuck";
            }
        }
    }

    static class Fail extends TimerTask {
        public void run() {
            throw new Stuck();
        }
    }

    // Two timers report an exception from a task while the case holds
    // what the report needs: one dropped, one cancelled. Their queues are
    // empty, but their threads run the game's code, so a start at the
    // limit must not wait for them: it is refused.
    static void reporting() throws InterruptedException {
        Thread[] made = new Thread[100];
        int started = 0;
        String message = null;
        synchronized (held) {
            new Timer().schedule(new Fail(), 0);
            Timer cancelled = new Timer();
            cancelled.schedule(new Fail(), 0);
            synchronized (seen) {
                while (reports < 2) {
                    seen.wait();
                }
            }
            cancelled.cancel();
            for (int i = 0; i < made.length && message == null; i++) {
                Thread t = new Sleeper();
                try {
                    t.start();
                    made[started] = t;
                    started++;
                } catch (OutOfMemoryError e) {
                    message = e.getMessage();
                }
            }
            System.out.println("refused " + message);
            System.gc();
            try {
                new Sleeper().start();
                System.out.println("started after gc");
            } catch (OutOfMemoryError e) {
                System.out.println("refused after gc " + e.getMessage());
            }
        }
        synchronized (gate) {
            open = true;
            gate.notifyAll();
        }
        for (int i = 0; i < started; i++) {
            made[i].join();
        }
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
        } else if (mode.equals("dropped")) {
            dropped();
        } else if (mode.equals("used")) {
            used();
        } else if (mode.equals("start")) {
            start();
        } else if (mode.equals("collected")) {
            collected();
        } else if (mode.equals("repeating")) {
            repeating();
        } else if (mode.equals("kept")) {
            kept();
        } else if (mode.equals("reporting")) {
            reporting();
        }
        System.out.println("end");
    }
}
