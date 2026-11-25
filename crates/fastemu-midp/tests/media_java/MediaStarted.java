import javax.microedition.media.MediaException;
import javax.microedition.media.Player;

/** A game starts every player it made: the 33rd cannot start. */
public class MediaStarted extends MediaUtil {
    /** Makes a player and tries to start it; true when the start was refused. */
    static boolean refused() throws Exception {
        Player p = player("/long.wav", "audio/x-wav");
        try {
            p.start();
            return false;
        } catch (MediaException e) {
            return true;
        }
    }

    public static void main(String[] args) throws Exception {
        Player[] players = new Player[34];
        for (int i = 0; i < players.length; i++) {
            players[i] = player("/long.wav", "audio/x-wav");
        }
        for (int i = 0; i < 32; i++) {
            players[i].start();
        }
        say("the 32nd is started", players[31].getState());
        try {
            players[32].start();
            say("the 33rd start", "no exception");
        } catch (Exception e) {
            threw("the 33rd start", e);
        }
        say("the 33rd stays", players[32].getState());
        // A stop gives its place to the next.
        players[0].stop();
        players[32].start();
        say("after a stop", players[32].getState());
        try {
            players[33].start();
            say("the 34th start", "no exception");
        } catch (Exception e) {
            threw("the 34th start", e);
        }
        // A player that ended gives its place back too.
        Pump.advance(1500);
        say("after the end", players[33].getState());
        players[33].start();
        say("started after the end", players[33].getState());

        // A start that failed leaves nothing holding the player.
        for (int i = 0; i < players.length; i++) {
            players[i].close();
        }
        for (int i = 0; i < 32; i++) {
            players[i] = player("/long.wav", "audio/x-wav");
            players[i].start();
        }
        int before = Pump.players();
        say("one refused", refused());
        System.gc();
        say("and collected", Pump.players() == before);
        System.out.println("done");
    }
}
