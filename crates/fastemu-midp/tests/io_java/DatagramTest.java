import java.io.EOFException;
import java.io.IOException;
import javax.microedition.io.Connector;
import javax.microedition.io.Datagram;
import javax.microedition.io.UDPDatagramConnection;

/**
 * UDPDatagramConnection and Datagram against the Rust echo peer in
 * io_datagram.rs (argument: its port), then a server and a client
 * connection talking to each other.
 */
public class DatagramTest {
    public static void main(String[] args) throws Exception {
        String peer = "datagram://127.0.0.1:" + args[0];
        UDPDatagramConnection c = (UDPDatagramConnection) Connector.open(peer);
        System.out.println("max " + c.getMaximumLength() + " nominal " + c.getNominalLength()
                + " localport " + (c.getLocalPort() > 0));
        Datagram d = c.newDatagram(100);
        System.out.println("new " + d.getLength() + " " + d.getOffset() + " "
                + d.getAddress().equals(peer));
        d.reset();
        d.writeUTF("hi é");
        d.writeInt(-5);
        d.writeLong(1L << 40);
        d.writeBoolean(true);
        d.writeShort(-2);
        d.writeChar('Z');
        d.writeFloat(1.5f);
        d.writeDouble(-0.25);
        d.write(7);
        d.writeChars("ok");
        System.out.println("written " + d.getLength());
        d.setAddress(peer);
        c.send(d);

        Datagram r = c.newDatagram(200);
        c.receive(r);
        System.out.println("received " + r.getLength() + " from peer " + r.getAddress().equals(peer));
        System.out.println(r.readUTF().equals("hi é") + " " + r.readInt() + " " + r.readLong()
                + " " + r.readBoolean() + " " + r.readShort() + " " + r.readChar()
                + " " + r.readFloat() + " " + r.readDouble() + " " + r.readByte()
                + " " + r.readChar() + r.readChar());
        try {
            r.readByte();
        } catch (EOFException e) {
            System.out.println("eof");
        }

        byte[] small = new byte[10];
        Datagram t = c.newDatagram(small, 10);
        t.setData(small, 4, 6);
        c.send(c.newDatagram("0123456789ABCDEF".getBytes(), 16));
        c.receive(t);
        System.out.println("truncated " + t.getLength() + " " + new String(small, 4, 6));

        try {
            d.setLength(101);
        } catch (IllegalArgumentException e) {
            System.out.println("length " + e.getMessage());
        }
        try {
            d.setAddress("socket://x:1");
        } catch (IllegalArgumentException e) {
            System.out.println("address " + e.getMessage());
        }
        try {
            c.newDatagram(-1);
        } catch (IllegalArgumentException e) {
            System.out.println("size " + e.getMessage());
        }
        Datagram full = c.newDatagram(2);
        full.reset();
        full.writeShort(1);
        try {
            full.writeByte(1);
        } catch (IOException e) {
            System.out.println("full " + e.getMessage());
        }
        c.close();
        try {
            c.send(d);
        } catch (IOException e) {
            System.out.println("closed " + e.getMessage());
        }

        UDPDatagramConnection server = (UDPDatagramConnection) Connector.open("datagram://:");
        int port = server.getLocalPort();
        UDPDatagramConnection client =
                (UDPDatagramConnection) Connector.open("datagram://127.0.0.1:" + port);
        Datagram m = client.newDatagram(16);
        m.reset();
        m.writeUTF("hello server");
        client.send(m);
        Datagram got = server.newDatagram(64);
        server.receive(got);
        String from = got.getAddress();
        System.out.println("server got " + got.readUTF() + " from client "
                + from.equals("datagram://127.0.0.1:" + client.getLocalPort()));
        Datagram reply = server.newDatagram(16);
        reply.reset();
        reply.writeUTF("hello client");
        reply.setAddress(got);
        server.send(reply);
        Datagram back = client.newDatagram(64);
        client.receive(back);
        System.out.println("client got " + back.readUTF());
        Datagram none = server.newDatagram(4);
        System.out.println("no address " + none.getAddress());
        try {
            server.send(none);
        } catch (IOException e) {
            System.out.println("unaddressed " + e.getMessage());
        }
        server.close();
        client.close();

        final UDPDatagramConnection lonely = (UDPDatagramConnection) Connector.open("datagram://:");
        final Datagram nothing = lonely.newDatagram(8);
        Thread waiter = new Thread() {
            public void run() {
                try {
                    lonely.receive(nothing);
                    System.out.println("received from nobody");
                } catch (IOException e) {
                    System.out.println("receive on close -> " + e.getMessage());
                }
            }
        };
        waiter.start();
        Thread.sleep(300);
        lonely.close();
        waiter.join();
        System.out.println("done");
    }
}
