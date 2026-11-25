import java.io.IOException;
import java.io.InputStream;
import javax.microedition.io.Connection;
import javax.microedition.io.Connector;
import javax.microedition.io.ServerSocketConnection;

/**
 * How many connections a game may hold open: sockets, an HTTP connection,
 * the sockets its own server accepts, and server sockets. args[0] is the
 * port of a listener that accepts and drops, args[1] a port nothing
 * listens on.
 */
public class SocketLimits {
    static String target;

    public static void main(String[] args) throws Exception {
        target = "socket://127.0.0.1:" + args[0];
        clients(args[1]);
        accepted();
        forgotten();
        servers();
        System.out.println("done");
    }

    static String refusal(String name) {
        try {
            Connector.open(name).close();
            return "opened";
        } catch (IOException e) {
            return e.getMessage();
        }
    }

    static void clients(String closed) throws IOException {
        // A connect that fails does not keep its place. Port 0 fails at
        // once on every system; a closed port can take seconds.
        int failed = 0;
        for (int i = 0; i < 70; i++) {
            if (!refusal("socket://127.0.0.1:0").equals("opened")) {
                failed++;
            }
        }
        System.out.println("failed " + failed);

        Connection[] held = new Connection[64];
        for (int i = 0; i < held.length; i++) {
            held[i] = Connector.open(target);
        }
        System.out.println("held " + held.length);
        System.out.println("65th " + refusal(target));
        System.out.println("http " + refusal("http://127.0.0.1:" + closed + "/"));
        // Closing one gives its place back at once.
        held[0].close();
        System.out.println("after close " + refusal(target));
        held[0] = Connector.open(target);
        // A connection stays open while one of its streams does.
        InputStream in = ((javax.microedition.io.StreamConnection) held[1]).openInputStream();
        held[1].close();
        System.out.println("stream open " + refusal(target));
        in.close();
        System.out.println("stream closed " + refusal(target));
        for (int i = 0; i < held.length; i++) {
            held[i].close();
        }
        System.out.println("all closed " + refusal(target));
    }

    static void accepted() throws IOException {
        ServerSocketConnection server = (ServerSocketConnection) Connector.open("socket://:0");
        String own = "socket://127.0.0.1:" + server.getLocalPort();
        Connection[] held = new Connection[62];
        for (int i = 0; i < held.length; i++) {
            held[i] = Connector.open(own);
        }
        // The 63rd place goes to a client of the game's own server, the
        // 64th to the connection the server accepts.
        Connection last = Connector.open(own);
        Connection mine = server.acceptAndOpen();
        System.out.println("accepted");
        try {
            server.acceptAndOpen();
            System.out.println("second accepted");
        } catch (IOException e) {
            System.out.println("second " + e.getMessage());
        }
        mine.close();
        Connection again = server.acceptAndOpen();
        System.out.println("accepted again");
        again.close();
        last.close();
        for (int i = 0; i < held.length; i++) {
            held[i].close();
        }
        server.close();
    }

    static void forgotten() throws IOException {
        // A game that never closes what it opens still gets places, as the
        // collector frees what nothing refers to.
        int opened = 0;
        for (int i = 0; i < 200; i++) {
            Connector.open(target);
            opened++;
        }
        System.out.println("forgotten " + opened);
    }

    static void servers() throws IOException {
        Connection[] held = new Connection[8];
        for (int i = 0; i < held.length; i++) {
            held[i] = Connector.open("socket://:0");
        }
        System.out.println("servers " + held.length);
        System.out.println("9th " + refusal("socket://:0"));
        held[0].close();
        System.out.println("server closed " + refusal("socket://:0"));
        for (int i = 0; i < held.length; i++) {
            held[i].close();
        }
    }
}
