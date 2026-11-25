import java.io.InputStream;
import javax.microedition.io.Connector;
import javax.microedition.io.SecureConnection;

/**
 * Opens a secure socket to a server that sends one byte at a time, and
 * reads from it. args[0] is the port. The program prints "open" when the
 * handshake is over and "read" with the byte it got, which a server that
 * drips never lets it come to.
 */
public class TlsDrip {
    public static void main(String[] args) throws Exception {
        System.out.println("opening");
        SecureConnection s = (SecureConnection) Connector.open("ssl://127.0.0.1:" + args[0]);
        System.out.println("open");
        InputStream in = s.openInputStream();
        byte[] bytes = new byte[16000];
        System.out.println("read " + in.read(bytes, 0, bytes.length));
    }
}
