import javax.microedition.io.PushRegistry;

/**
 * PushRegistry: registrations are checked and kept nowhere, alarms
 * report no earlier alarm. The suite lists one MIDlet, "Game".
 */
public class PushTest {
    public static void main(String[] args) throws Exception {
        PushRegistry.registerConnection("sms://:5000", "Game", "*");
        System.out.println("registered");
        System.out.println("list " + PushRegistry.listConnections(false).length
                + " " + PushRegistry.listConnections(true).length);
        System.out.println("midlet " + PushRegistry.getMIDlet("sms://:5000")
                + " filter " + PushRegistry.getFilter("sms://:5000"));
        System.out.println("unregister " + PushRegistry.unregisterConnection("sms://:5000"));
        System.out.println("alarm " + PushRegistry.registerAlarm("Game", 1000)
                + " " + PushRegistry.registerAlarm("Game", 0));
        try {
            PushRegistry.registerConnection("nocolon", "Game", "*");
        } catch (IllegalArgumentException e) {
            System.out.println("bad " + e.getMessage());
        }
        try {
            PushRegistry.registerConnection("sms://:5000", "Other", "*");
        } catch (ClassNotFoundException e) {
            System.out.println("other " + e.getMessage());
        }
        try {
            PushRegistry.registerAlarm(null, 5);
        } catch (ClassNotFoundException e) {
            System.out.println("null " + e.getMessage());
        }
        System.out.println("done");
    }
}
