// Threads with wait, notify, join, sleep, interrupt and synchronized.
public class Threads {
    static class Buffer {
        private final int[] items = new int[4];
        private int head, tail, count;

        synchronized void put(int v) throws InterruptedException {
            while (count == items.length) {
                wait();
            }
            items[tail] = v;
            tail = (tail + 1) % items.length;
            count++;
            notifyAll();
        }

        synchronized int take() throws InterruptedException {
            while (count == 0) {
                wait();
            }
            int v = items[head];
            head = (head + 1) % items.length;
            count--;
            notifyAll();
            return v;
        }
    }

    static class SlowInit {
        static int runs;
        static int value;

        static {
            runs++;
            try {
                Thread.sleep(50);
            } catch (InterruptedException e) {
                value = -1;
            }
            value = 42;
        }
    }

    static int shared;
    static int staticShared;
    static final Object LOCK = new Object();

    static synchronized void bumpStatic() {
        staticShared++;
    }

    public static void main(String[] args) throws Exception {
        final Buffer buffer = new Buffer();
        final long[] sum = new long[1];
        Thread producer = new Thread(new Runnable() {
            public void run() {
                try {
                    for (int i = 1; i <= 1000; i++) {
                        buffer.put(i);
                    }
                    buffer.put(-1);
                } catch (InterruptedException e) {
                    System.out.println("producer interrupted");
                }
            }
        }, "producer");
        Thread consumer = new Thread("consumer") {
            public void run() {
                try {
                    int v;
                    while ((v = buffer.take()) != -1) {
                        sum[0] += v;
                    }
                } catch (InterruptedException e) {
                    System.out.println("consumer interrupted");
                }
            }
        };
        producer.start();
        consumer.start();
        producer.join();
        consumer.join();
        System.out.println("sum " + sum[0] + " " + producer.getName() + " " + consumer.getName() + " alive " + producer.isAlive());

        Thread[] workers = new Thread[4];
        for (int w = 0; w < workers.length; w++) {
            workers[w] = new Thread() {
                public void run() {
                    for (int i = 0; i < 20000; i++) {
                        synchronized (LOCK) {
                            shared++;
                        }
                        if (i % 1000 == 0) {
                            Thread.yield();
                        }
                    }
                    for (int i = 0; i < 5000; i++) {
                        bumpStatic();
                    }
                }
            };
        }
        for (int w = 0; w < workers.length; w++) {
            workers[w].start();
        }
        for (int w = 0; w < workers.length; w++) {
            workers[w].join();
        }
        System.out.println("shared " + shared + " " + staticShared);

        // Each step waits for the one before it, so the output does not
        // depend on timing. Waits loop on their condition: HotSpot's
        // wait() may return without a notify or an interrupt (an
        // interrupt that ends a sleep can also unpark the next wait), as
        // Java allows.
        final Object gate = new Object();
        // interrupted sleep, interrupted wait, waiting
        final boolean[] state = new boolean[3];
        Thread sleeper = new Thread() {
            public void run() {
                try {
                    Thread.sleep(10000);
                    state[0] = false;
                } catch (InterruptedException e) {
                    state[0] = true;
                }
                synchronized (gate) {
                    state[2] = true;
                    gate.notifyAll();
                    try {
                        while (true) {
                            gate.wait();
                        }
                    } catch (InterruptedException e) {
                        state[1] = true;
                    }
                }
            }
        };
        sleeper.start();
        // Whether it lands before the sleep starts or during it, the
        // interrupt makes the sleep throw.
        Thread.sleep(50);
        sleeper.interrupt();
        synchronized (gate) {
            while (!state[2]) {
                gate.wait();
            }
            // Main holds gate again only once the sleeper's wait has
            // released it, so this interrupt lands in that wait.
            sleeper.interrupt();
        }
        sleeper.join();
        System.out.println("interrupted sleep " + state[0] + " interrupted wait " + state[1]);

        long start = System.currentTimeMillis();
        Thread.sleep(30);
        long slept = System.currentTimeMillis() - start;
        System.out.println("slept enough " + (slept >= 25));

        synchronized (gate) {
            long t0 = System.currentTimeMillis();
            long left = 40;
            while (left > 0) {
                gate.wait(left);
                left = 40 - (System.currentTimeMillis() - t0);
            }
            System.out.println("timed wait returned");
        }

        Thread self = Thread.currentThread();
        System.out.println("main thread " + self.getName() + " priority " + self.getPriority());
        Thread t = new Thread();
        t.setPriority(Thread.MAX_PRIORITY);
        System.out.println("new thread priority " + t.getPriority() + " alive " + t.isAlive());
        t.start();
        t.join();
        try {
            t.start();
        } catch (IllegalThreadStateException e) {
            System.out.println("restart refused");
        }
        try {
            t.setPriority(11);
        } catch (IllegalArgumentException e) {
            System.out.println("bad priority refused");
        }
        final int[] order = new int[3];
        final Object turn = new Object();
        final int[] next = {0};
        Thread[] ring = new Thread[3];
        for (int i = 0; i < 3; i++) {
            final int me = i;
            ring[i] = new Thread() {
                public void run() {
                    for (int round = 0; round < 3; round++) {
                        synchronized (turn) {
                            while (next[0] % 3 != me) {
                                try {
                                    turn.wait();
                                } catch (InterruptedException e) {
                                    return;
                                }
                            }
                            order[me] = order[me] * 10 + next[0];
                            next[0]++;
                            turn.notifyAll();
                        }
                    }
                }
            };
            ring[i].start();
        }
        for (int i = 0; i < 3; i++) {
            ring[i].join();
        }
        System.out.println("ring " + order[0] + " " + order[1] + " " + order[2]);
        final int[] seen = new int[3];
        Thread[] readers = new Thread[3];
        for (int i = 0; i < readers.length; i++) {
            final int me = i;
            readers[i] = new Thread() {
                public void run() {
                    seen[me] = SlowInit.value;
                }
            };
        }
        for (int i = 0; i < readers.length; i++) {
            readers[i].start();
        }
        for (int i = 0; i < readers.length; i++) {
            readers[i].join();
        }
        System.out.println("init once " + SlowInit.runs + " seen " + seen[0] + " " + seen[1] + " " + seen[2]);
        final Thread main = Thread.currentThread();
        Thread late = new Thread() {
            public void run() {
                try {
                    main.join();
                } catch (InterruptedException e) {
                    return;
                }
                System.out.println("late thread finished after main");
            }
        };
        late.start();
        System.out.println("main ends");
    }
}
