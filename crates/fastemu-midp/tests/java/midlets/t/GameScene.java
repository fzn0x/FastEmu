package t;

import javax.microedition.lcdui.Display;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;
import javax.microedition.lcdui.game.GameCanvas;
import javax.microedition.lcdui.game.LayerManager;
import javax.microedition.lcdui.game.Sprite;
import javax.microedition.lcdui.game.TiledLayer;
import javax.microedition.midlet.MIDlet;

/**
 * A GameCanvas drawing a LayerManager: a tiled background with an
 * animated row and a sprite with a clear corner. Fire turns the sprite,
 * changes the animated tile and moves the view window.
 */
public class GameScene extends MIDlet {
    protected void startApp() {
        GameSceneCanvas canvas = new GameSceneCanvas();
        Display.getDisplay(this).setCurrent(canvas);
        new Thread(canvas).start();
    }

    protected void pauseApp() {}

    protected void destroyApp(boolean unconditional) {}
}

class GameSceneCanvas extends GameCanvas implements Runnable {
    private final LayerManager manager = new LayerManager();
    private TiledLayer background;
    private Sprite player;
    private Image dot;

    GameSceneCanvas() {
        super(true);
        setFullScreenMode(true);
    }

    private static Image solid(int w, int h, int color, int clear) {
        int[] rgb = new int[w * h];
        for (int y = 0; y < h; y++) {
            for (int x = 0; x < w; x++) {
                boolean corner = x < clear && y < clear;
                rgb[y * w + x] = corner ? 0 : 0xFF000000 | color;
            }
        }
        return Image.createRGBImage(rgb, w, h, true);
    }

    private void build() {
        int[] rgb = new int[32 * 16];
        for (int y = 0; y < 16; y++) {
            for (int x = 0; x < 32; x++) {
                rgb[y * 32 + x] = x < 16 ? 0xFF0000FF : 0xFF00FF00;
            }
        }
        background = new TiledLayer(14, 20, Image.createRGBImage(rgb, 32, 16, false), 16, 16);
        background.fillCells(0, 0, 14, 20, 1);
        int animated = background.createAnimatedTile(2);
        background.fillCells(0, 5, 14, 1, animated);
        player = new Sprite(solid(16, 16, 0xFF0000, 4));
        player.defineReferencePixel(8, 8);
        player.setPosition(100, 100);
        dot = solid(1, 1, 0xFFFFFF, 0);
        manager.append(player);
        manager.append(background);
        manager.setViewWindow(0, 0, getWidth(), getHeight());
    }

    private void draw(Graphics g) {
        g.setColor(0);
        g.fillRect(0, 0, getWidth(), getHeight());
        manager.paint(g, 0, 0);
        flushGraphics();
    }

    private String collisions() {
        return "tiles " + player.collidesWith(background, false) + " "
                + player.collidesWith(background, true) + " dot "
                + player.collidesWith(dot, 101, 101, true) + " "
                + player.collidesWith(dot, 114, 101, true);
    }

    private void waitForFire() {
        while ((getKeyStates() & FIRE_PRESSED) == 0) {
            try {
                Thread.sleep(20);
            } catch (InterruptedException e) {
                return;
            }
        }
    }

    public void run() {
        build();
        Graphics g = getGraphics();
        draw(g);
        System.out.println("scene 1 " + collisions());
        waitForFire();
        player.setTransform(Sprite.TRANS_ROT90);
        background.setAnimatedTile(-1, 1);
        manager.setViewWindow(16, 0, getWidth(), getHeight());
        draw(g);
        System.out.println("scene 2 at " + player.getX() + "," + player.getY() + " "
                + collisions());
    }
}
