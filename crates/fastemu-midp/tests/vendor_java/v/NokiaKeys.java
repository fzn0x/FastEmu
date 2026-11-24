package v;

import com.nokia.mid.ui.DeviceControl;
import com.nokia.mid.ui.DirectGraphics;
import com.nokia.mid.ui.FullCanvas;
import javax.microedition.lcdui.Command;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/**
 * A FullCanvas that logs its size, its key codes and game actions, and
 * paints the whole screen green through DirectGraphics. Key 1 asks for
 * the backlight and the vibration motor.
 */
public class NokiaKeys extends MIDlet {
    protected void startApp() {
        Display.getDisplay(this).setCurrent(new KeysCanvas());
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class KeysCanvas extends FullCanvas {
    private boolean painted;

    KeysCanvas() {
        try {
            addCommand(new Command("Back", Command.BACK, 1));
            System.out.println("addCommand: allowed");
        } catch (IllegalStateException e) {
            System.out.println("addCommand: IllegalStateException");
        }
        try {
            setCommandListener(null);
            System.out.println("setCommandListener: allowed");
        } catch (IllegalStateException e) {
            System.out.println("setCommandListener: IllegalStateException");
        }
        FullCanvas see = new FullCanvas(true) {
            protected void paint(Graphics g) {}
        };
        System.out.println("transparency: " + getTransparency() + " " + see.getTransparency()
                + " " + see.getWidth() + "x" + see.getHeight());
    }

    protected void showNotify() {
        System.out.println("shown " + getWidth() + "x" + getHeight());
    }

    protected void paint(Graphics g) {
        DirectGraphics d = (DirectGraphics) g;
        d.setARGBColor(0xFF00FF00);
        g.fillRect(0, 0, getWidth(), getHeight());
        if (!painted) {
            painted = true;
            System.out.println("painted");
        }
    }

    protected void keyPressed(int code) {
        System.out.println("pressed " + code + " action " + getGameAction(code));
        if (code == KEY_NUM1) {
            DeviceControl.setLights(0, 100);
            DeviceControl.setLights(0, 100);
            DeviceControl.startVibra(50, 300);
            DeviceControl.setLights(7, 500);
        }
    }

    protected void keyReleased(int code) {
        System.out.println("released " + code);
    }
}
