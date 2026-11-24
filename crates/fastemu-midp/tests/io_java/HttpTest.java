import java.io.DataInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import javax.microedition.io.Connector;
import javax.microedition.io.HttpConnection;

/**
 * HttpConnection against the Rust test server in io_http.rs. Arguments:
 * the server's port, a port nothing listens on, and "data" when
 * java.io.DataInputStream exists.
 */
public class HttpTest {
    static String base;

    public static void main(String[] args) throws Exception {
        base = "http://127.0.0.1:" + args[0];
        get();
        chunked();
        post();
        head();
        notFound();
        untilClose();
        notHttp();
        states();
        urlParts();
        connectorStream();
        if (args.length > 2 && args[2].equals("data")) {
            dataStream();
        }
        refused(args[1]);
        System.out.println("done");
    }

    static String readAll(InputStream in) throws IOException {
        StringBuffer sb = new StringBuffer();
        int c;
        while ((c = in.read()) != -1) {
            sb.append((char) c);
        }
        return sb.toString();
    }

    static void get() throws IOException {
        HttpConnection c = (HttpConnection) Connector.open(base + "/hello?x=1");
        c.setRequestProperty("X-Custom", "abc");
        c.setRequestProperty("x-custom", "def");
        System.out.println("prop " + c.getRequestProperty("X-CUSTOM"));
        System.out.println("code " + c.getResponseCode() + " " + c.getResponseMessage());
        System.out.println("type " + c.getType() + " length " + c.getLength()
                + " encoding " + c.getEncoding());
        System.out.println("date " + c.getDate() + " expires " + c.getExpiration()
                + " modified " + c.getLastModified());
        System.out.println("number " + c.getHeaderFieldInt("x-number", -1)
                + " missing " + c.getHeaderFieldInt("x-none", -7));
        System.out.println("headerdate " + c.getHeaderFieldDate("Date", 5)
                + " bad " + c.getHeaderFieldDate("X-Number", 5));
        for (int i = 0; c.getHeaderFieldKey(i) != null; i++) {
            System.out.println("h" + i + " " + c.getHeaderFieldKey(i) + "=" + c.getHeaderField(i));
        }
        System.out.println("h99 " + c.getHeaderField(99) + " " + c.getHeaderFieldKey(-1));
        InputStream in = c.openInputStream();
        System.out.println("body " + readAll(in));
        in.close();
        c.close();
    }

    static void chunked() throws IOException {
        HttpConnection c = (HttpConnection) Connector.open(base + "/chunked");
        InputStream in = c.openInputStream();
        byte[] buf = new byte[100];
        int n = in.read(buf);
        System.out.println("chunked " + c.getResponseCode() + " length " + c.getLength()
                + " read " + n + " " + new String(buf, 0, n) + " then " + in.read());
        in.close();
        c.close();
    }

    static void post() throws IOException {
        HttpConnection c = (HttpConnection) Connector.open(base + "/echo");
        c.setRequestMethod(HttpConnection.POST);
        c.setRequestProperty("Content-Type", "application/x-www-form-urlencoded");
        OutputStream out = c.openOutputStream();
        out.write("score=100&".getBytes());
        out.flush();
        c.setRequestProperty("X-Late", "ignored");
        out.write("name=ab".getBytes());
        out.close();
        try {
            c.setRequestProperty("X-After", "x");
        } catch (IOException e) {
            System.out.println("after " + e.getMessage());
        }
        InputStream in = c.openInputStream();
        System.out.println("post " + c.getResponseCode() + " " + c.getHeaderField("X-Method")
                + " " + c.getHeaderField("X-Late") + " " + readAll(in));
        in.close();
        c.close();
    }

    static void head() throws IOException {
        HttpConnection c = (HttpConnection) Connector.open(base + "/hello");
        c.setRequestMethod(HttpConnection.HEAD);
        InputStream in = c.openInputStream();
        System.out.println("head " + c.getResponseCode() + " " + c.getLength() + " " + in.read());
        in.close();
        c.close();
    }

    static void notFound() throws IOException {
        HttpConnection c = (HttpConnection) Connector.open(base + "/missing");
        System.out.println("missing " + c.getResponseCode() + " " + c.getResponseMessage()
                + " " + readAll(c.openInputStream()));
        c.close();
    }

    static void untilClose() throws IOException {
        HttpConnection c = (HttpConnection) Connector.open(base + "/close");
        System.out.println("close " + c.getResponseCode() + " " + c.getLength()
                + " " + readAll(c.openInputStream()));
        c.close();
    }

    static void notHttp() throws IOException {
        HttpConnection c = (HttpConnection) Connector.open(base + "/raw");
        System.out.println("raw " + c.getResponseCode() + " " + c.getResponseMessage()
                + " " + readAll(c.openInputStream()));
        c.close();
    }

    static void states() throws IOException {
        HttpConnection c = (HttpConnection) Connector.open(base + "/hello");
        try {
            c.setRequestMethod("PUT");
        } catch (IOException e) {
            System.out.println("put " + e.getMessage());
        }
        c.getResponseCode();
        try {
            c.setRequestMethod(HttpConnection.POST);
        } catch (IOException e) {
            System.out.println("method " + e.getMessage());
        }
        try {
            c.setRequestProperty("A", "b");
        } catch (IOException e) {
            System.out.println("property " + e.getMessage());
        }
        try {
            c.openOutputStream();
        } catch (IOException e) {
            System.out.println("output " + e.getMessage());
        }
        InputStream in = c.openInputStream();
        try {
            c.openInputStream();
        } catch (IOException e) {
            System.out.println("input " + e.getMessage());
        }
        c.close();
        System.out.println("url after close " + c.getURL().equals(base + "/hello"));
        try {
            c.getResponseCode();
        } catch (IOException e) {
            System.out.println("closed " + e.getMessage());
        }
        System.out.println("stream after close " + readAll(in));
        in.close();
        c.close();
        HttpConnection r = (HttpConnection) Connector.open(base + "/hello", Connector.READ);
        try {
            r.openOutputStream();
        } catch (IOException e) {
            System.out.println("readonly " + e.getMessage());
        }
        r.close();
    }

    static void urlParts() throws IOException {
        HttpConnection c = (HttpConnection) Connector.open(
                "http://example.com:8080/a/b.php?x=1&y=2#frag");
        System.out.println(c.getProtocol() + " " + c.getHost() + " " + c.getPort() + " "
                + c.getFile() + " " + c.getQuery() + " " + c.getRef() + " "
                + c.getRequestMethod());
        c.close();
        HttpConnection d = (HttpConnection) Connector.open("http://example.com");
        System.out.println(d.getPort() + " " + d.getFile() + " " + d.getQuery() + " " + d.getRef());
        d.close();
    }

    static void connectorStream() throws IOException {
        InputStream in = Connector.openInputStream(base + "/hello");
        System.out.println("connector " + readAll(in));
        in.close();
        // The connection is closed already; closing the stream still sends.
        OutputStream out = Connector.openOutputStream(base + "/upload");
        out.write("fire and forget".getBytes());
        out.close();
    }

    static void dataStream() throws IOException {
        DataInputStream in = Connector.openDataInputStream(base + "/hello");
        byte[] b = new byte[5];
        in.readFully(b);
        System.out.println("data " + new String(b));
        in.close();
    }

    static void refused(String port) throws IOException {
        HttpConnection c = (HttpConnection) Connector.open("http://127.0.0.1:" + port + "/x");
        System.out.println("length " + c.getLength() + " type " + c.getType());
        try {
            c.getResponseCode();
        } catch (IOException e) {
            System.out.println("refused " + e.getClass().getName() + " " + e.getMessage());
        }
        try {
            c.getHeaderField("Date");
        } catch (IOException e) {
            System.out.println("again " + e.getMessage());
        }
        c.close();
    }
}
