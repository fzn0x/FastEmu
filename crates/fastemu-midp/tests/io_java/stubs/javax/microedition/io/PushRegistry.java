// Compile-time stub; the VM runs FastEmu's native class. See Connector.java.
package javax.microedition.io;

public class PushRegistry {
    private PushRegistry() {}

    public static void registerConnection(String connection, String midlet, String filter)
            throws ClassNotFoundException, java.io.IOException { throw new RuntimeException(); }
    public static boolean unregisterConnection(String connection) { throw new RuntimeException(); }
    public static String[] listConnections(boolean available) { throw new RuntimeException(); }
    public static String getMIDlet(String connection) { throw new RuntimeException(); }
    public static String getFilter(String connection) { throw new RuntimeException(); }
    public static long registerAlarm(String midlet, long time)
            throws ClassNotFoundException, ConnectionNotFoundException { throw new RuntimeException(); }
}
