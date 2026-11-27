/*
 * A signature-only stand-in for compiling MeBench, the MicroEmulator
 * wrapper of `cargo xtask bench`, where MicroEmulator is not installed,
 * such as CI. It holds no MicroEmulator code: only the names and types
 * MeBench uses, with empty bodies. The bench never runs it: before it
 * measures, it compiles MeBench again against the real microemulator.jar.
 */
package javax.microedition.lcdui;

public class Image {
    public int getWidth() {
        throw new UnsupportedOperationException();
    }

    public int getHeight() {
        throw new UnsupportedOperationException();
    }

    public void getRGB(int[] rgbData, int offset, int scanlength, int x, int y, int width, int height) {
        throw new UnsupportedOperationException();
    }
}
