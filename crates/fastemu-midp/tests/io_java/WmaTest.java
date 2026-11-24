import java.io.IOException;
import java.io.InterruptedIOException;
import javax.microedition.io.Connector;
import javax.wireless.messaging.BinaryMessage;
import javax.wireless.messaging.MessageConnection;
import javax.wireless.messaging.MessageListener;
import javax.wireless.messaging.TextMessage;

/** WMA: messages that are built and counted as on a phone and never sent. */
public class WmaTest {
    public static void main(String[] args) throws Exception {
        MessageConnection c = (MessageConnection) Connector.open("sms://+15551234567");
        TextMessage t = (TextMessage) c.newMessage(MessageConnection.TEXT_MESSAGE);
        System.out.println("address " + t.getAddress() + " timestamp " + t.getTimestamp()
                + " text " + t.getPayloadText());
        t.setPayloadText("Hello");
        System.out.println("segments " + c.numberOfSegments(t)
                + " " + segments(c, 'a', 160) + " " + segments(c, 'a', 161)
                + " " + segments(c, 'Ж', 70) + " " + segments(c, 'Ж', 71)
                + " " + segments(c, 'a', 500));
        try {
            c.send(t);
            System.out.println("send -> sent");
        } catch (IOException e) {
            System.out.println("send -> " + e.getClass().getName() + ": " + e.getMessage());
        }
        try {
            c.receive();
        } catch (IOException e) {
            System.out.println("receive -> " + e.getMessage());
        }
        try {
            c.setMessageListener(null);
        } catch (IOException e) {
            System.out.println("listener -> " + e.getMessage());
        }
        BinaryMessage b = (BinaryMessage) c.newMessage(MessageConnection.BINARY_MESSAGE,
                "sms://+1555:5000");
        b.setPayloadData(new byte[200]);
        System.out.println("binary " + b.getAddress() + " " + b.getPayloadData().length
                + " segments " + c.numberOfSegments(b));
        try {
            c.newMessage("mms");
        } catch (IllegalArgumentException e) {
            System.out.println("type -> " + e.getMessage());
        }
        try {
            t.setAddress("http://example.com/");
        } catch (IllegalArgumentException e) {
            System.out.println("address -> " + e.getMessage());
        }
        TextMessage none = (TextMessage) c.newMessage("text");
        none.setAddress(null);
        none.setPayloadText("x");
        try {
            c.send(none);
        } catch (IllegalArgumentException e) {
            System.out.println("unaddressed -> " + e.getMessage());
        }
        try {
            c.send(null);
        } catch (NullPointerException e) {
            System.out.println("null -> NullPointerException");
        }
        c.close();
        try {
            c.send(t);
        } catch (IOException e) {
            System.out.println("closed -> " + e.getMessage());
        }

        final MessageConnection server = (MessageConnection) Connector.open("sms://:5000");
        server.setMessageListener(new MessageListener() {
            public void notifyIncomingMessage(MessageConnection conn) {
                System.out.println("incoming message");
            }
        });
        try {
            Connector.open("sms://:5000");
        } catch (IOException e) {
            System.out.println("port -> " + e.getMessage());
        }
        final Object lock = new Object();
        final boolean[] started = new boolean[1];
        Thread waiter = new Thread() {
            public void run() {
                synchronized (lock) {
                    started[0] = true;
                    lock.notifyAll();
                }
                try {
                    server.receive();
                    System.out.println("received a message");
                } catch (InterruptedIOException e) {
                    System.out.println("receive on close -> " + e.getMessage());
                } catch (IOException e) {
                    System.out.println("receive failed -> " + e.getMessage());
                }
            }
        };
        waiter.start();
        // A receive that starts after the close fails another way, and a
        // busy machine may start the thread late: wait until it runs, then
        // give it time to get into its receive.
        synchronized (lock) {
            while (!started[0]) {
                lock.wait();
            }
        }
        Thread.sleep(300);
        server.close();
        waiter.join();
        MessageConnection again = (MessageConnection) Connector.open("sms://:5000");
        again.close();
        System.out.println("reopened");

        MessageConnection cbs = (MessageConnection) Connector.open("cbs://:50");
        try {
            cbs.send(cbs.newMessage("text", "sms://+1"));
        } catch (IOException e) {
            System.out.println("cbs -> " + e.getMessage());
        }
        cbs.close();
        try {
            Connector.open("cbs://+123");
        } catch (IllegalArgumentException e) {
            System.out.println("cbs client -> " + e.getMessage());
        }
        try {
            Connector.open("sms://");
        } catch (IllegalArgumentException e) {
            System.out.println("empty -> " + e.getMessage());
        }
        System.out.println("done");
    }

    /** The segments a text message of n copies of c needs. */
    static int segments(MessageConnection conn, char c, int n) {
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < n; i++) {
            sb.append(c);
        }
        TextMessage m = (TextMessage) conn.newMessage(MessageConnection.TEXT_MESSAGE);
        m.setPayloadText(sb.toString());
        return conn.numberOfSegments(m);
    }
}
