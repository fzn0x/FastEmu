import java.io.IOException;
import java.io.InputStream;
import javax.microedition.io.Connector;
import javax.microedition.io.HttpConnection;

/**
 * Asks a server that sends what a server should not: interim responses
 * for ever, or a head, a body or the trailer of a chunked body one byte
 * at a time. args[0] is the port. args[1] is "head" to read only the
 * response code, "bulk" to read the body with one large read and
 * anything else to read the body a byte at a time.
 */
public class HttpHostile {
    public static void main(String[] args) throws Exception {
        HttpConnection c = (HttpConnection) Connector.open("http://127.0.0.1:" + args[0] + "/");
        System.out.println("asking");
        try {
            System.out.println("code " + c.getResponseCode());
            if (args[1].equals("bulk")) {
                InputStream in = c.openInputStream();
                System.out.println("read " + in.read(new byte[60000]));
            } else if (!args[1].equals("head")) {
                InputStream in = c.openInputStream();
                int n = 0;
                while (in.read() != -1) {
                    n++;
                }
                System.out.println("body " + n);
            }
        } catch (IOException e) {
            System.out.println("failed " + e.getClass().getName() + ": " + e.getMessage());
        }
        System.out.println("done");
    }
}
