// A compile-time stand-in for the runtime's MIDlet; the tests' JARs never
// contain it. Signatures follow docs/compat/api/midp-midlet.md.
package javax.microedition.midlet;

public abstract class MIDlet {
    protected MIDlet() {}

    protected abstract void startApp();

    protected abstract void pauseApp();

    protected abstract void destroyApp(boolean unconditional);

    public final void notifyDestroyed() {}
}
