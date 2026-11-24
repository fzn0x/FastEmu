package v;

import com.motorola.multimedia.Lighting;

/** Motorola's backlight. */
public class MotorolaLighting {
    public static void main(String[] args) {
        Lighting.backlightOn();
        Lighting.backlightOff();
        System.out.println("done");
    }
}
