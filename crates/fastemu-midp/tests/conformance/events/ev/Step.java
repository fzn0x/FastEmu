package ev;

/** A callSerially runnable (ids 1 to 3) or the game thread's body (id 0). */
class Step implements Runnable {
    private final Order midlet;
    private final int id;

    Step(Order midlet, int id) {
        this.midlet = midlet;
        this.id = id;
    }

    public void run() {
        if (id == 0) {
            midlet.game();
        } else {
            midlet.step(id);
        }
    }
}
