package many;

import java.io.InputStream;
import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.media.Manager;
import javax.microedition.media.Player;
import javax.microedition.media.PlayerListener;
import javax.microedition.midlet.MIDlet;
import javax.microedition.rms.RecordEnumeration;
import javax.microedition.rms.RecordStore;

/**
 * One of many instances running side by side. Every frame shows, in
 * bands at the top, the code of the last key it got, how many keys it got
 * and its own id, so a test can tell from a frame which input reached
 * which instance. Its id comes from the application property Multi-Id.
 * At its first start it writes the id to a record store; # reads the
 * store back and * plays a sound.
 */
public class Multi extends MIDlet implements Runnable, PlayerListener {
    private final Board board = new Board();
    private String id;
    private volatile boolean running;
    private Thread loop;
    private Player player;

    protected void startApp() {
        if (id == null) {
            id = getAppProperty("Multi-Id");
            board.id = Integer.parseInt(id);
            write();
        }
        System.out.println("startApp " + id);
        Display.getDisplay(this).setCurrent(board);
        if (loop == null) {
            running = true;
            loop = new Thread(this);
            loop.start();
        }
    }

    protected void pauseApp() {
        System.out.println("pauseApp " + id);
    }

    protected void destroyApp(boolean unconditional) {
        running = false;
        System.out.println("destroyApp " + id);
    }

    /** The game loop: a repaint every 10 ms. */
    public void run() {
        while (running) {
            board.repaint();
            try {
                Thread.sleep(10);
            } catch (InterruptedException e) {
                return;
            }
        }
    }

    private void write() {
        try {
            RecordStore store = RecordStore.openRecordStore("multi", true);
            byte[] data = id.getBytes();
            store.addRecord(data, 0, data.length);
            store.closeRecordStore();
            System.out.println("rms wrote " + id);
        } catch (Exception e) {
            System.out.println("rms failed " + id + " " + e);
        }
    }

    private void read() {
        try {
            RecordStore store = RecordStore.openRecordStore("multi", false);
            RecordEnumeration records = store.enumerateRecords(null, null, false);
            StringBuffer seen = new StringBuffer();
            while (records.hasNextElement()) {
                seen.append(new String(records.nextRecord())).append(',');
            }
            records.destroy();
            store.closeRecordStore();
            System.out.println("rms read " + id + ": " + seen);
        } catch (Exception e) {
            System.out.println("rms failed " + id + " " + e);
        }
    }

    private void play() {
        try {
            InputStream in = getClass().getResourceAsStream("/beep.wav");
            player = Manager.createPlayer(in, "audio/x-wav");
            player.addPlayerListener(this);
            player.start();
        } catch (Exception e) {
            System.out.println("sound failed " + id + " " + e);
        }
    }

    public void playerUpdate(Player p, String event, Object data) {
        if (event.equals(PlayerListener.STARTED)) {
            System.out.println("sound started " + id);
        } else if (event.equals(PlayerListener.END_OF_MEDIA)) {
            System.out.println("sound ended " + id);
        }
    }

    class Board extends Canvas {
        int id;
        private int key;
        private int presses;

        protected void paint(Graphics g) {
            int w = getWidth();
            g.setColor(0x400000 | (key & 0xFFFF));
            g.fillRect(0, 0, w, 10);
            g.setColor(0x100000 | presses);
            g.fillRect(0, 10, w, 10);
            g.setColor(0x200000 | id);
            g.fillRect(0, 20, w, 10);
        }

        protected void keyPressed(int code) {
            key = code;
            presses++;
            if (code == '#') {
                read();
            } else if (code == '*') {
                play();
            }
            repaint();
        }
    }
}
