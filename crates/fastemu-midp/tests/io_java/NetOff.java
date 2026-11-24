import javax.microedition.io.Connector;

/**
 * Connector.open with the network policy Deny, and names no protocol
 * handles. Every line names the exception and its message.
 */
public class NetOff {
    public static void main(String[] args) {
        open("http://127.0.0.1:1/");
        open("https://example.com/");
        open("socket://127.0.0.1:1");
        open("socket://:0");
        open("ssl://example.com:443");
        open("datagram://127.0.0.1:1");
        open("datagram://:0");
        open("comm:0;baudrate=9600");
        open("file:///root/x.txt");
        open("gopher://example.com/");
        open("nocolon");
        open(null);
        try {
            Connector.open("http://127.0.0.1/", 7);
        } catch (Exception e) {
            System.out.println("mode -> " + describe(e));
        }
        try {
            Connector.openInputStream("http://127.0.0.1/");
        } catch (Exception e) {
            System.out.println("stream -> " + describe(e));
        }
        System.out.println("done");
    }

    static void open(String name) {
        try {
            Connector.open(name);
            System.out.println(name + " -> opened");
        } catch (Exception e) {
            System.out.println(name + " -> " + describe(e));
        }
    }

    static String describe(Exception e) {
        return e.getClass().getName() + ": " + e.getMessage();
    }
}
