// Compile-time stub: FastEmu implements this class in Rust. Signatures
// follow docs/compat/api/midp-media.md.
package javax.microedition.media;

import java.io.IOException;
import java.io.InputStream;

public final class Manager {
    public static final String TONE_DEVICE_LOCATOR = "device://tone";
    public static final String MIDI_DEVICE_LOCATOR = "device://midi";

    private Manager() {
    }

    public static String[] getSupportedContentTypes(String protocol) {
        return null;
    }

    public static String[] getSupportedProtocols(String contentType) {
        return null;
    }

    public static Player createPlayer(String locator) throws IOException, MediaException {
        return null;
    }

    public static Player createPlayer(InputStream stream, String type)
            throws IOException, MediaException {
        return null;
    }

    public static Player createPlayer(javax.microedition.media.protocol.DataSource source)
            throws IOException, MediaException {
        return null;
    }

    public static void playTone(int note, int duration, int volume) throws MediaException {
    }

    public static TimeBase getSystemTimeBase() {
        return null;
    }
}
