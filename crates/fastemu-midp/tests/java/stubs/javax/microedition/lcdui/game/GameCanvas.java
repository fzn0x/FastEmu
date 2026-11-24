// Compile-time stand-ins for the runtime's game classes. Signatures and
// constant values follow docs/compat/api/midp-lcdui-game.md, since javac
// inlines the constants.
package javax.microedition.lcdui.game;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Graphics;

public abstract class GameCanvas extends Canvas {
    public static final int UP_PRESSED = 2;
    public static final int DOWN_PRESSED = 64;
    public static final int LEFT_PRESSED = 4;
    public static final int RIGHT_PRESSED = 32;
    public static final int FIRE_PRESSED = 256;
    public static final int GAME_A_PRESSED = 512;
    public static final int GAME_B_PRESSED = 1024;
    public static final int GAME_C_PRESSED = 2048;
    public static final int GAME_D_PRESSED = 4096;

    protected GameCanvas(boolean suppressKeyEvents) {}

    protected Graphics getGraphics() {
        return null;
    }

    public int getKeyStates() {
        return 0;
    }

    public void paint(Graphics g) {}

    public void flushGraphics(int x, int y, int width, int height) {}

    public void flushGraphics() {}
}
