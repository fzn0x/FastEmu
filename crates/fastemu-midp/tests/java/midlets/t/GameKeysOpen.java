package t;

import javax.microedition.midlet.MIDlet;

/** GameKeys without suppression: game keys still get key events. */
public class GameKeysOpen extends MIDlet {
    protected void startApp() {
        GameKeyCanvas.start(this, false);
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
