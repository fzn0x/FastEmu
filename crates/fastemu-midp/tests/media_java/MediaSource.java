import java.io.IOException;

import javax.microedition.media.Control;
import javax.microedition.media.Manager;
import javax.microedition.media.Player;
import javax.microedition.media.protocol.ContentDescriptor;
import javax.microedition.media.protocol.DataSource;
import javax.microedition.media.protocol.SourceStream;

/** A player fed from the game's own DataSource. */
public class MediaSource extends MediaUtil {
    /** A stream over a byte array, handing out at most 100 bytes a read. */
    static class Bytes implements SourceStream {
        private final byte[] data;
        private int at;

        Bytes(byte[] data) {
            this.data = data;
        }

        public ContentDescriptor getContentDescriptor() {
            return new ContentDescriptor("audio/x-wav");
        }

        public long getContentLength() {
            return data.length;
        }

        public int read(byte[] b, int off, int len) {
            if (at >= data.length) {
                return -1;
            }
            int n = Math.min(Math.min(len, 100), data.length - at);
            System.arraycopy(data, at, b, off, n);
            at += n;
            return n;
        }

        public int getTransferSize() {
            return 100;
        }

        public long seek(long where) {
            at = (int) where;
            return at;
        }

        public long tell() {
            return at;
        }

        public int getSeekType() {
            return RANDOM_ACCESSIBLE;
        }

        public Control[] getControls() {
            return new Control[0];
        }

        public Control getControl(String type) {
            return null;
        }
    }

    /** A source of one stream, or of none, or one that cannot connect. */
    static class Source extends DataSource {
        private final SourceStream[] streams;
        private final boolean broken;
        boolean connected;
        boolean started;

        Source(SourceStream[] streams, boolean broken) {
            super("memory://sound");
            this.streams = streams;
            this.broken = broken;
        }

        public String getContentType() {
            return "audio/x-wav";
        }

        public void connect() throws IOException {
            if (broken) {
                throw new IOException("cannot connect");
            }
            connected = true;
        }

        public void disconnect() {
            connected = false;
        }

        public void start() {
            started = true;
        }

        public void stop() {
            started = false;
        }

        public SourceStream[] getStreams() {
            return streams;
        }

        public Control[] getControls() {
            return new Control[0];
        }

        public Control getControl(String type) {
            return null;
        }
    }

    public static void main(String[] args) throws Exception {
        Source source = new Source(new SourceStream[] {new Bytes(bytes("/beep.wav"))}, false);
        say("locator", source.getLocator());
        Player p = Manager.createPlayer(source);
        say("connected and started", source.connected && source.started);
        p.realize();
        say("type", p.getContentType());
        say("duration", p.getDuration());
        p.start();
        say("heard", Pump.peak(50) > 0);
        p.close();
        try {
            Manager.createPlayer((DataSource) null);
        } catch (IllegalArgumentException e) {
            threw("null source", e);
        }
        try {
            Manager.createPlayer(new Source(new SourceStream[0], false));
        } catch (Exception e) {
            threw("no stream", e);
        }
        try {
            Manager.createPlayer(new Source(null, true));
        } catch (Exception e) {
            threw("broken source", e);
        }
        say("descriptor", new ContentDescriptor("audio/midi").getContentType());
        System.out.println("done");
    }
}
