import java.io.IOException;
import javax.microedition.io.Connection;
import javax.microedition.io.Connector;

/**
 * How many datagram connections a game may hold open. args[0] is a UDP
 * port to send to.
 */
public class DatagramLimits {
    static String refusal(String name) {
        try {
            Connector.open(name).close();
            return "opened";
        } catch (IOException e) {
            return e.getMessage();
        }
    }

    public static void main(String[] args) throws Exception {
        Connection[] held = new Connection[16];
        for (int i = 0; i < held.length; i++) {
            held[i] = Connector.open(i % 2 == 0 ? "datagram://:0" : "datagram://127.0.0.1:" + args[0]);
        }
        System.out.println("held " + held.length);
        System.out.println("17th " + refusal("datagram://:0"));
        System.out.println("client " + refusal("datagram://127.0.0.1:" + args[0]));
        held[3].close();
        System.out.println("after close " + refusal("datagram://:0"));
        // One the game never closes is freed by the collector.
        held[4] = null;
        for (int i = 0; i < 40; i++) {
            Connector.open("datagram://:0");
        }
        System.out.println("forgotten opened");
        for (int i = 0; i < held.length; i++) {
            if (held[i] != null) {
                held[i].close();
            }
        }
        System.out.println("all closed " + refusal("datagram://:0"));
        System.out.println("done");
    }
}
