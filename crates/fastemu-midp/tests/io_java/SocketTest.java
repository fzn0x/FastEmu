import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import javax.microedition.io.Connector;
import javax.microedition.io.ServerSocketConnection;
import javax.microedition.io.SocketConnection;
import javax.microedition.io.StreamConnection;

/**
 * SocketConnection against the Rust echo peer in io_socket.rs, then a
 * ServerSocketConnection the test connects to. Arguments: the echo
 * peer's port and a port nothing listens on.
 */
public class SocketTest {
    public static void main(String[] args) throws Exception {
        String echo = "socket://127.0.0.1:" + args[0];
        SocketConnection c = (SocketConnection) Connector.open(echo);
        c.setSocketOption(SocketConnection.DELAY, 0);
        c.setSocketOption(SocketConnection.KEEPALIVE, 1);
        c.setSocketOption(SocketConnection.LINGER, 5);
        System.out.println("delay " + c.getSocketOption(SocketConnection.DELAY)
                + " keepalive " + c.getSocketOption(SocketConnection.KEEPALIVE)
                + " linger " + c.getSocketOption(SocketConnection.LINGER));
        System.out.println("rcvbuf positive " + (c.getSocketOption(SocketConnection.RCVBUF) > 0));
        try {
            c.setSocketOption((byte) 9, 1);
        } catch (IllegalArgumentException e) {
            System.out.println("option " + e.getMessage());
        }
        try {
            c.setSocketOption(SocketConnection.LINGER, -1);
        } catch (IllegalArgumentException e) {
            System.out.println("negative " + e.getMessage());
        }
        System.out.println("address " + c.getAddress()
                + " port " + (c.getPort() == Integer.parseInt(args[0]))
                + " local " + c.getLocalAddress()
                + " localport " + (c.getLocalPort() > 0));

        InputStream in = c.openInputStream();
        OutputStream out = c.openOutputStream();
        out.write("ping\n".getBytes());
        out.flush();
        System.out.println("echo " + readLine(in));
        out.write("more\n".getBytes());
        out.flush();
        for (int i = 0; i < 500 && in.available() < 10; i++) {
            Thread.sleep(10);
        }
        System.out.println("available " + in.available());
        System.out.println("more " + readLine(in));
        try {
            c.openInputStream();
        } catch (IOException e) {
            System.out.println("second " + e.getMessage());
        }
        c.close();
        out.write("after close\n".getBytes());
        out.flush();
        System.out.println("still " + readLine(in));
        try {
            c.getAddress();
        } catch (IOException e) {
            System.out.println("closed " + e.getMessage());
        }
        out.close();
        in.close();

        SocketConnection d = (SocketConnection) Connector.open(echo);
        OutputStream dout = d.openOutputStream();
        dout.write("bye\n".getBytes());
        dout.flush();
        InputStream din = d.openInputStream();
        System.out.println("eof [" + readLine(din) + "] " + din.read());
        din.close();
        dout.close();
        d.close();

        try {
            Connector.open("socket://127.0.0.1:" + args[1]);
        } catch (IOException e) {
            System.out.println("refused " + e.getClass().getName() + ": " + e.getMessage());
        }
        try {
            Connector.open("socket://127.0.0.1");
        } catch (IllegalArgumentException e) {
            System.out.println("noport " + e.getMessage());
        }

        ServerSocketConnection server = (ServerSocketConnection) Connector.open("socket://:");
        System.out.println("listening " + server.getLocalPort());
        StreamConnection peer = server.acceptAndOpen();
        InputStream pin = peer.openInputStream();
        OutputStream pout = peer.openOutputStream();
        String line = readLine(pin);
        pout.write(("got " + line + "\n").getBytes());
        pout.close();
        pin.close();
        peer.close();
        server.close();
        try {
            server.acceptAndOpen();
        } catch (IOException e) {
            System.out.println("server closed " + e.getMessage());
        }
        System.out.println("done");
    }

    /** One line without its line feed; what arrived before the end. */
    static String readLine(InputStream in) throws IOException {
        StringBuffer sb = new StringBuffer();
        int c;
        while ((c = in.read()) != -1 && c != '\n') {
            sb.append((char) c);
        }
        return sb.toString();
    }
}
