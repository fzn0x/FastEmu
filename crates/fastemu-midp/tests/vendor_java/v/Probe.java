package v;

import javax.microedition.midlet.MIDlet;

/**
 * Probes for the vendor APIs with Class.forName, as games do to learn
 * which phone they run on, and logs the classes it found.
 */
public class Probe extends MIDlet {
    static final String[] NAMES = {
        "com.nokia.mid.ui.FullCanvas",
        "com.nokia.mid.ui.DirectGraphics",
        "com.nokia.mid.ui.DirectUtils",
        "com.nokia.mid.ui.DeviceControl",
        "com.nokia.mid.sound.Sound",
        "com.siemens.mp.game.Light",
        "com.siemens.mp.game.Sprite",
        "com.siemens.mp.color_game.GameCanvas",
        "com.siemens.mp.ui.Image",
        "com.samsung.util.Vibration",
        "com.samsung.util.AudioClip",
        "com.motorola.multimedia.Lighting",
        "mmpp.media.BackLight",
        "emulator.Emulator",
        "java.applet.Applet",
    };

    protected void startApp() {
        StringBuffer found = new StringBuffer("found:");
        for (int i = 0; i < NAMES.length; i++) {
            try {
                Class.forName(NAMES[i]);
                found.append(' ').append(NAMES[i]);
            } catch (ClassNotFoundException e) {
                // Not on this phone.
            }
        }
        System.out.println(found.toString());
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}
