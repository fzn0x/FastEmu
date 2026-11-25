package t;

import javax.microedition.midlet.MIDlet;

/**
 * Pauses itself and asks to be resumed again and again from inside a
 * startApp that never returns, so that every lifecycle call the app
 * manager starts waits for its turn behind it. Prints how many threads the
 * machine has when it stops asking.
 */
public class PauseFlood extends MIDlet {
    protected void startApp() {
        int stalled = 0;
        for (int i = 0; i < 300 && stalled < 3; i++) {
            int before = Thread.activeCount();
            notifyPaused();
            resumeRequest();
            // Each resume starts a lifecycle thread, unless the machine has
            // no room for one: wait for it, then give up on a few misses.
            boolean grew = false;
            for (int wait = 0; wait < 200 && !grew; wait++) {
                try {
                    Thread.sleep(1);
                } catch (InterruptedException e) {
                }
                grew = Thread.activeCount() > before;
            }
            stalled = grew ? 0 : stalled + 1;
        }
        System.out.println("threads " + Thread.activeCount());
        // Never returns, as a game that runs its loop in startApp.
        while (true) {
            try {
                Thread.sleep(1000);
            } catch (InterruptedException e) {
            }
        }
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {
        System.out.println("destroyed " + unconditional);
    }
}
