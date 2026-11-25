// What a machine's threads are doing, as Vm.activity counts them. Each
// case prints "ready" once its threads are on their way to where the test
// looks at them; the case is chosen by the first argument.
public class ActivityCases {
    static final Object gate = new Object();
    static final Object a = new Object();
    static final Object b = new Object();
    static int arrived;
    static volatile int sink;
    static volatile boolean initializing;
    static volatile boolean inWait;

    // Waits on the gate for ever.
    static class Waiter extends Thread {
        public void run() {
            synchronized (gate) {
                try {
                    gate.wait();
                } catch (InterruptedException e) {
                    return;
                }
            }
        }
    }

    // Waits on the gate once, and says so while it holds the gate, so the
    // gate is free only once it waits.
    static class FlaggedWaiter extends Thread {
        public void run() {
            synchronized (gate) {
                inWait = true;
                try {
                    gate.wait();
                } catch (InterruptedException e) {
                    return;
                }
            }
        }
    }

    static class Sleeper extends Thread {
        public void run() {
            try {
                Thread.sleep(1000000);
            } catch (InterruptedException e) {
                return;
            }
        }
    }

    // Holds one lock, waits until the other thread holds the other, then
    // asks for it: the two block each other for ever.
    static class Crosser extends Thread {
        final Object first;
        final Object second;

        Crosser(Object first, Object second) {
            this.first = first;
            this.second = second;
        }

        public void run() {
            synchronized (first) {
                synchronized (gate) {
                    arrived++;
                    gate.notifyAll();
                    while (arrived < 2) {
                        try {
                            gate.wait();
                        } catch (InterruptedException e) {
                            return;
                        }
                    }
                }
                synchronized (second) {
                    sink++;
                }
            }
        }
    }

    // Its initializer waits for ever, so a second thread that uses the
    // class waits for the first to finish initializing it.
    static class Slow {
        static int value;

        static {
            initializing = true;
            synchronized (gate) {
                try {
                    gate.wait();
                } catch (InterruptedException e) {
                    // nothing
                }
            }
            value = 1;
        }
    }

    static class Initializer extends Thread {
        public void run() {
            sink = Slow.value;
        }
    }

    public static void main(String[] args) throws Exception {
        String mode = args[0];
        if (mode.equals("waits")) {
            Thread waiter = new Waiter();
            waiter.start();
            new Sleeper().start();
            System.out.println("ready");
            waiter.join();
        } else if (mode.equals("spins")) {
            System.out.println("ready");
            for (int i = 0; ; i++) {
                sink = i;
            }
        } else if (mode.equals("crosses")) {
            new Crosser(a, b).start();
            new Crosser(b, a).start();
            System.out.println("ready");
        } else if (mode.equals("reenters")) {
            // The waiter is woken but cannot take the gate back, since main
            // sleeps holding it.
            new FlaggedWaiter().start();
            while (!inWait) {
                Thread.sleep(5);
            }
            synchronized (gate) {
                gate.notifyAll();
                System.out.println("ready");
                Thread.sleep(1000000);
            }
        } else if (mode.equals("initializes")) {
            new Initializer().start();
            while (!initializing) {
                Thread.sleep(5);
            }
            System.out.println("ready");
            sink = Slow.value;
        }
    }
}
