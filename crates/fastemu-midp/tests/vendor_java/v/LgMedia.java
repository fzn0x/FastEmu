package v;

import mmpp.media.BackLight;
import mmpp.media.Vibration;

/** LG's mmpp.media backlight and vibration. */
public class LgMedia {
    public static void main(String[] args) {
        BackLight.on(500);
        BackLight.setColor(0x123456);
        BackLight.setColor(0xFF123456);
        System.out.println("backlight: " + Integer.toHexString(BackLight.getColor()) + " of "
                + BackLight.numColors());
        BackLight.off();
        System.out.println("levels: " + Vibration.getLevelNum());
        Vibration.start(3, 250);
        Vibration.stop();
        System.out.println("done");
    }
}
