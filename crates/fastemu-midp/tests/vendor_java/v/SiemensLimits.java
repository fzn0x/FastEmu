package v;

import com.siemens.mp.game.ExtendedImage;
import com.siemens.mp.game.GraphicObjectManager;
import com.siemens.mp.game.MelodyComposer;
import com.siemens.mp.game.TiledBackground;
import javax.microedition.lcdui.Image;

/**
 * Siemens calls with extreme arguments: each must return at once, without
 * looping over a region far larger than its arrays and image, and without
 * an arithmetic overflow.
 */
public class SiemensLimits {
    public static void main(String[] args) throws Exception {
        ExtendedImage e = new ExtendedImage(Image.createImage(8, 2));
        byte[] bytes = new byte[2];
        e.getPixelBytes(bytes, 2147483640, 0, 16, 1);
        System.out.println("pixel bytes far right: " + bytes[0] + " " + bytes[1]);
        e.getPixelBytes(bytes, 0, 2147483647, 8, 2);
        System.out.println("pixel bytes far down: " + bytes[0] + " " + bytes[1]);
        e.getPixelBytes(bytes, 0, 0, 2147483640, 2147483647);
        System.out.println("pixel bytes huge: " + bytes[0] + " " + bytes[1]);
        e.setPixels(new byte[] {(byte) 0xFF}, 0, 0, 2147483640, 2147483647);
        System.out.println("set pixels huge: " + e.getPixel(0, 0) + " " + e.getPixel(0, 1));

        byte[] map = {3};
        byte[] tile = {(byte) 0xFF, 0, 0, 0, 0, 0, 0, 0};
        TiledBackground bg = new TiledBackground(tile, null, map, 1, 1);
        GraphicObjectManager m = new GraphicObjectManager();
        m.addObject(bg);
        Image target = Image.createImage(8, 8);
        m.paint(target, -2147483648, -2147483648);
        int[] row = new int[1];
        target.getRGB(row, 0, 1, 0, 0, 1, 1);
        System.out.println("far background: " + Integer.toHexString(row[0]));

        // TONE_MARK (64), then TONE_REPEAT_MARK (65) of nothing.
        MelodyComposer c = new MelodyComposer(new int[] {64, 0, 65, 2147483647}, 60);
        c.getMelody();
        c.resetMelody();
        c.appendNote(MelodyComposer.TONE_REPEAT, 2147483647);
        c.getMelody();
        System.out.println("empty repeats: " + c.length());
        System.out.println("done");
    }
}
