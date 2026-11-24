package s;

import javax.microedition.lcdui.Alert;
import javax.microedition.lcdui.AlertType;
import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Command;
import javax.microedition.lcdui.CommandListener;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Displayable;
import javax.microedition.lcdui.Gauge;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/**
 * A timed alert in front of a canvas, then a modal one with a listener
 * that the select key dismisses.
 */
public class Alerts extends MIDlet implements CommandListener {
    Display display;
    Behind behind;

    protected void startApp() {
        display = Display.getDisplay(this);
        behind = new Behind(this);
        Alert note = new Alert("Note", "Saved", null, AlertType.INFO);
        note.setTimeout(300);
        System.out.println("timeout " + note.getTimeout() + " default " + note.getDefaultTimeout());
        try {
            note.setTimeout(0);
        } catch (IllegalArgumentException e) {
            System.out.println("zero refused");
        }
        Gauge busy = new Gauge(null, false, Gauge.INDEFINITE, Gauge.CONTINUOUS_RUNNING);
        note.setIndicator(busy);
        try {
            note.setIndicator(new Gauge(null, true, 5, 1));
        } catch (IllegalArgumentException e) {
            System.out.println("interactive refused");
        }
        display.setCurrent(note, behind);
    }

    void showModal() {
        Alert ask = new Alert("Ask");
        ask.setString("Sure?");
        ask.setTimeout(Alert.FOREVER);
        ask.setCommandListener(this);
        final Gauge wait = new Gauge(null, false, Gauge.INDEFINITE, Gauge.CONTINUOUS_IDLE);
        ask.setIndicator(wait);
        display.setCurrent(ask);
        // The game starts the indicator once the alert is up.
        new Thread() {
            public void run() {
                try {
                    Thread.sleep(200);
                } catch (InterruptedException e) {
                    return;
                }
                wait.setValue(Gauge.CONTINUOUS_RUNNING);
                System.out.println("running");
            }
        }.start();
    }

    public void commandAction(Command c, Displayable d) {
        System.out.println("dismissed " + (c == Alert.DISMISS_COMMAND));
        display.setCurrent(behind);
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class Behind extends Canvas {
    final Alerts midlet;
    int shown;

    Behind(Alerts midlet) {
        this.midlet = midlet;
    }

    protected void paint(Graphics g) {
        g.setColor(0x0000FF);
        g.fillRect(0, 0, getWidth(), getHeight());
    }

    protected void showNotify() {
        shown++;
        System.out.println("behind " + shown);
        if (shown == 1) {
            midlet.showModal();
        }
    }
}
