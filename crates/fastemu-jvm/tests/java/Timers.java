import java.util.Date;
import java.util.Timer;
import java.util.TimerTask;

// java.util.Timer and TimerTask: one-shot tasks in time order, fixed
// delay and fixed rate, cancel, the checks schedule makes, and a task
// that throws. Java SE ends the whole timer when a task throws, where
// FastEmu cancels only that task (rule 16), so the throwing task here
// is the last thing its timer does.
public class Timers {
    static final Object lock = new Object();
    static final StringBuffer log = new StringBuffer();
    static int done;

    static void finish() {
        synchronized (lock) {
            done++;
            lock.notifyAll();
        }
    }

    static void await(int n) throws InterruptedException {
        synchronized (lock) {
            while (done < n) {
                lock.wait();
            }
        }
    }

    static class Named extends TimerTask {
        final String name;

        Named(String name) {
            this.name = name;
        }

        public void run() {
            synchronized (lock) {
                log.append(name);
            }
            finish();
        }
    }

    // A repeating task that records the gaps between its scheduled
    // times and cancels itself after a few runs.
    static class Repeat extends TimerTask {
        final int runs;
        int n;
        long last;
        boolean steady = true;
        long smallest = Long.MAX_VALUE;

        Repeat(int runs) {
            this.runs = runs;
        }

        public void run() {
            long at = scheduledExecutionTime();
            if (n > 0) {
                long gap = at - last;
                steady &= gap == 20;
                smallest = Math.min(smallest, gap);
            }
            last = at;
            n++;
            if (n == runs) {
                cancel();
                finish();
            }
        }
    }

    static void expect(String what, Runnable r) {
        try {
            r.run();
            System.out.println(what + ": no exception");
        } catch (NullPointerException e) {
            // The JDK's message names its own fields.
            System.out.println(what + ": " + e.getClass().getName());
        } catch (RuntimeException e) {
            System.out.println(what + ": " + e.getClass().getName() + " " + e.getMessage());
        }
    }

    public static void main(String[] args) throws InterruptedException {
        final Timer timer = new Timer();
        timer.schedule(new Named("c"), 120);
        timer.schedule(new Named("a"), 20);
        timer.schedule(new Named("b"), 70);
        await(3);
        System.out.println("one-shots ran " + log);

        Repeat rate = new Repeat(6);
        timer.scheduleAtFixedRate(rate, 0, 20);
        await(4);
        System.out.println("fixed rate: " + rate.n + " runs, steady " + rate.steady + ", cancel again " + rate.cancel());

        Repeat delay = new Repeat(4);
        timer.schedule(delay, 5, 20);
        await(5);
        System.out.println("fixed delay: " + delay.n + " runs, gaps at least 20 " + (delay.smallest >= 20));

        final TimerTask fresh = new TimerTask() {
            public void run() {
            }
        };
        final Repeat used = rate;
        expect("schedule a cancelled task", new Runnable() {
            public void run() {
                timer.schedule(used, 10);
            }
        });
        expect("negative delay", new Runnable() {
            public void run() {
                timer.schedule(fresh, -1);
            }
        });
        expect("zero period", new Runnable() {
            public void run() {
                timer.schedule(fresh, 0, 0);
            }
        });
        expect("negative rate", new Runnable() {
            public void run() {
                timer.scheduleAtFixedRate(fresh, 0, -5);
            }
        });
        expect("negative date", new Runnable() {
            public void run() {
                timer.schedule(fresh, new Date(-5));
            }
        });
        expect("null task", new Runnable() {
            public void run() {
                timer.schedule(null, 10);
            }
        });
        System.out.println("cancel a fresh task " + fresh.cancel());
        expect("schedule it", new Runnable() {
            public void run() {
                timer.schedule(fresh, 10);
            }
        });

        final String[] where = new String[1];
        timer.schedule(new TimerTask() {
            public void run() {
                where[0] = Thread.currentThread().getName();
                finish();
            }
        }, new Date(System.currentTimeMillis() + 20));
        await(6);
        System.out.println("ran on a Timer thread " + where[0].startsWith("Timer-"));

        Named once = new Named("d");
        timer.schedule(once, 10);
        await(7);
        System.out.println("cancel after it ran " + once.cancel() + ", scheduled for a time " + (once.scheduledExecutionTime() > 0));

        timer.cancel();
        timer.cancel();
        expect("schedule on a cancelled timer", new Runnable() {
            public void run() {
                timer.schedule(new Named("x"), 5);
            }
        });

        Timer bad = new Timer();
        bad.schedule(new TimerTask() {
            public void run() {
                finish();
                throw new RuntimeException("task failure");
            }
        }, 5);
        await(8);
        Thread.sleep(50);
        bad.cancel();
        System.out.println("log " + log);
    }
}
