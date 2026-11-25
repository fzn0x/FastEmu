import java.io.IOException;
import java.io.OutputStream;
import javax.microedition.io.Connector;
import javax.microedition.io.HttpConnection;

/**
 * What a game may collect for a request: the body of one connection, the
 * body of all its connections together, the request headers and the
 * name. Nothing reaches a server: the requests go to port 0, where a
 * connect fails at once on every system.
 */
public class HttpLimits {
    static String base;

    public static void main(String[] args) throws Exception {
        base = "http://127.0.0.1:0/up";
        body();
        budget();
        headers();
        name();
        System.out.println("done");
    }

    static OutputStream post() throws IOException {
        HttpConnection c = (HttpConnection) Connector.open(base);
        c.setRequestMethod(HttpConnection.POST);
        return c.openOutputStream();
    }

    static void body() throws IOException {
        OutputStream out = post();
        byte[] chunk = new byte[65536];
        int written = 0;
        try {
            for (int i = 0; i < 80; i++) {
                out.write(chunk, 0, chunk.length);
                written++;
            }
            System.out.println("body not refused");
        } catch (IOException e) {
            System.out.println("body " + written + " " + e.getMessage());
        }
        // The refusal leaves what was collected alone, and the request
        // can still go out; it fails because nothing listens.
        try {
            out.close();
            System.out.println("sent");
        } catch (IOException e) {
            System.out.println("send failed");
        }
    }

    static void budget() throws IOException {
        // Two bodies of 3 MiB and 2 MiB of a third fit in what a game may
        // hold; the next MiB does not.
        OutputStream[] outs = { post(), post(), post() };
        byte[] chunk = new byte[1 << 20];
        int accepted = 0;
        try {
            for (int i = 0; i < 3; i++) {
                for (int j = 0; j < 3; j++) {
                    outs[i].write(chunk, 0, chunk.length);
                    accepted++;
                }
            }
            System.out.println("budget not refused");
        } catch (IOException e) {
            System.out.println("budget " + accepted + " MiB " + e.getMessage());
        }
        // Sending a request gives its share back.
        try {
            outs[0].close();
            System.out.println("first sent");
        } catch (IOException e) {
            System.out.println("first send failed");
        }
        try {
            outs[2].write(chunk, 0, chunk.length);
            System.out.println("room again");
        } catch (IOException e) {
            System.out.println("no room " + e.getMessage());
        }
    }

    static void headers() throws IOException {
        HttpConnection c = (HttpConnection) Connector.open(base);
        int set = 0;
        try {
            for (int i = 0; i < 100; i++) {
                c.setRequestProperty("X-" + i, "v");
                set++;
            }
            System.out.println("headers not refused");
        } catch (IOException e) {
            System.out.println("headers " + set + " " + e.getMessage());
        }
        // A header that is there can change, and one can go for another.
        c.setRequestProperty("x-0", "other");
        c.setRequestProperty("X-1", null);
        c.setRequestProperty("Y", "new");
        System.out.println("replaced " + c.getRequestProperty("X-0") + " " + c.getRequestProperty("X-1")
                + " " + c.getRequestProperty("Y"));

        HttpConnection d = (HttpConnection) Connector.open(base);
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < 9000; i++) {
            sb.append('a');
        }
        String big = sb.toString();
        d.setRequestProperty("A", big);
        try {
            d.setRequestProperty("B", big);
            System.out.println("bytes not refused");
        } catch (IOException e) {
            System.out.println("bytes " + e.getMessage());
        }
        // Taking one away gives its bytes back.
        d.setRequestProperty("A", null);
        d.setRequestProperty("B", big);
        System.out.println("swapped " + (d.getRequestProperty("B").length()));
    }

    static void name() {
        StringBuffer sb = new StringBuffer(base);
        for (int i = 0; i < 9000; i++) {
            sb.append('a');
        }
        try {
            Connector.open(sb.toString());
            System.out.println("name not refused");
        } catch (Exception e) {
            System.out.println("name " + e.getClass().getName() + ": " + e.getMessage());
        }
    }
}
