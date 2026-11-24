import java.io.IOException;
import javax.microedition.io.Connector;
import javax.microedition.io.HttpConnection;

/**
 * Waits on a server that never answers: first on a thread that main
 * interrupts, then on main itself until the host stops the VM.
 */
public class HttpHang implements Runnable {
    static String base;

    public void run() {
        try {
            HttpConnection c = (HttpConnection) Connector.open(base + "/hang");
            c.getResponseCode();
            System.out.println("interrupt -> returned");
        } catch (IOException e) {
            System.out.println("interrupt -> " + e.getClass().getName() + ": " + e.getMessage());
        }
    }

    public static void main(String[] args) throws Exception {
        base = "http://127.0.0.1:" + args[0];
        Thread t = new Thread(new HttpHang());
        t.start();
        Thread.sleep(300);
        t.interrupt();
        t.join();
        System.out.println("blocking");
        HttpConnection c = (HttpConnection) Connector.open(base + "/hang");
        c.getResponseCode();
        System.out.println("not reached");
    }
}
