package t;

import javax.microedition.io.ConnectionNotFoundException;
import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.midlet.MIDlet;

/** Asks the phone to open links when keys are pressed and logs each answer. */
public class Links extends MIDlet {
    protected void startApp() {
        Display.getDisplay(this).setCurrent(new LinksCanvas(this));
        System.out.println("links ready");
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}

    /** Asks for url and logs what platformRequest returned or threw. */
    void request(String name, String url) {
        try {
            System.out.println(name + " " + platformRequest(url));
        } catch (ConnectionNotFoundException e) {
            System.out.println(name + " " + e);
        } catch (RuntimeException e) {
            System.out.println(name + " " + e);
        }
    }
}

class LinksCanvas extends Canvas {
    private final Links midlet;

    LinksCanvas(Links midlet) {
        this.midlet = midlet;
    }

    protected void paint(Graphics g) {}

    protected void keyPressed(int code) {
        switch (code) {
            case KEY_NUM1:
                midlet.request("first", "HTTP://Example.com/first");
                break;
            case KEY_NUM2:
                midlet.request("second", "https://example.com/second");
                midlet.request("third", "https://example.com/third?x=1");
                break;
            case KEY_NUM3:
                midlet.request("cancel", "");
                break;
            case KEY_NUM4:
                midlet.request("tel", "tel:+15550100");
                midlet.request("sms", "sms:+15550100");
                midlet.request("file", "file:///c:/game.txt");
                midlet.request("install", "http://example.com/game.jad");
                midlet.request("app", "localapp://jam/launch?midlet-name=Game");
                midlet.request("null", null);
                System.out.println("refused all");
                break;
            default:
                break;
        }
    }
}
