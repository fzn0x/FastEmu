package ev;

import java.util.TimerTask;

/** The timer task. */
class Tick extends TimerTask {
    private final Order midlet;

    Tick(Order midlet) {
        this.midlet = midlet;
    }

    public void run() {
        midlet.tick();
    }
}
