import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import javax.microedition.io.Connector;
import javax.microedition.io.HttpConnection;
import javax.microedition.io.HttpsConnection;
import javax.microedition.io.SecureConnection;
import javax.microedition.io.SecurityInfo;

/**
 * HttpsConnection and SecureConnection against the TLS servers in
 * io_tls.rs. Arguments: the HTTPS port, the TLS echo port, and
 * "trusted" when the VM trusts the test certificate authority.
 */
public class TlsTest {
    public static void main(String[] args) throws Exception {
        String https = "https://127.0.0.1:" + args[0] + "/secure";
        if (!args[2].equals("trusted")) {
            HttpConnection c = (HttpConnection) Connector.open(https);
            try {
                c.getResponseCode();
                System.out.println("untrusted -> connected");
            } catch (IOException e) {
                System.out.println("untrusted -> " + e.getClass().getName() + " "
                        + e.getMessage().startsWith("TLS handshake with 127.0.0.1 failed"));
            }
            c.close();
            return;
        }
        HttpsConnection c = (HttpsConnection) Connector.open(https);
        System.out.println("port " + (c.getPort() == Integer.parseInt(args[0]))
                + " protocol " + c.getProtocol());
        SecurityInfo info = c.getSecurityInfo();
        System.out.println("info " + info.getProtocolName() + " " + info.getProtocolVersion()
                + " " + info.getCipherSuite().startsWith("TLS_"));
        System.out.println("code " + c.getResponseCode() + " " + readLine(c.openInputStream()));
        c.close();

        HttpConnection d = (HttpConnection) Connector.open("https://example.com/");
        System.out.println("default " + d.getPort() + " " + (d instanceof HttpsConnection));
        d.close();

        SecureConnection s = (SecureConnection) Connector.open("ssl://127.0.0.1:" + args[1]);
        System.out.println("ssl " + s.getSecurityInfo().getProtocolName()
                + " " + s.getSecurityInfo().getProtocolVersion());
        OutputStream out = s.openOutputStream();
        InputStream in = s.openInputStream();
        out.write("secret\n".getBytes());
        out.flush();
        System.out.println("echo " + readLine(in));
        out.write("again\n".getBytes());
        out.flush();
        for (int i = 0; i < 500 && in.available() < 6; i++) {
            Thread.sleep(10);
        }
        System.out.println("available " + in.available() + " " + readLine(in));
        s.close();
        in.close();
        out.close();
        System.out.println("done");
    }

    static String readLine(InputStream in) throws IOException {
        StringBuffer sb = new StringBuffer();
        int c;
        while ((c = in.read()) != -1 && c != '\n') {
            sb.append((char) c);
        }
        return sb.toString();
    }
}
