// A compile-time stand-in for the runtime's MIDlet; the tests' JARs never
// contain it. Signatures follow docs/compat/api/midp-midlet.md.
package javax.microedition.midlet;

public abstract class MIDlet {
    protected MIDlet() {}

    protected abstract void startApp() throws MIDletStateChangeException;

    protected abstract void pauseApp();

    protected abstract void destroyApp(boolean unconditional) throws MIDletStateChangeException;

    public final void notifyDestroyed() {}

    public final void notifyPaused() {}

    public final void resumeRequest() {}

    public final String getAppProperty(String key) {
        return null;
    }

    public final boolean platformRequest(String url)
            throws javax.microedition.io.ConnectionNotFoundException {
        return false;
    }

    public final int checkPermission(String permission) {
        return 0;
    }
}
