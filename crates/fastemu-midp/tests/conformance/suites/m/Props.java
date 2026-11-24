package m;

import javax.microedition.midlet.MIDlet;

/** Prints the application properties the corpus tests look for. */
public class Props extends MIDlet {
    private static final String[] KEYS = {
        "MIDlet-Name", "MIDlet-Vendor", "Custom", "Wrapped", "Jad-Only", "Manifest-Only",
    };

    public void startApp() {
        for (int i = 0; i < KEYS.length; i++) {
            System.out.println("prop " + KEYS[i] + " = " + getAppProperty(KEYS[i]));
        }
        System.out.println("props done");
        notifyDestroyed();
    }

    public void pauseApp() {
    }

    public void destroyApp(boolean unconditional) {
    }
}
