/*
 * A signature-only stand-in for compiling MeBench, the MicroEmulator
 * wrapper of `cargo xtask bench`, where MicroEmulator is not installed,
 * such as CI. It holds no MicroEmulator code: only the names and types
 * MeBench uses, with empty bodies. The bench never runs it: before it
 * measures, it compiles MeBench again against the real microemulator.jar.
 */
package org.microemu.app.ui.noui;

import org.microemu.DisplayComponent;
import org.microemu.app.ui.DisplayRepaintListener;
import org.microemu.device.MutableImage;

public class NoUiDisplayComponent implements DisplayComponent {
    public void addDisplayRepaintListener(DisplayRepaintListener l) {
        throw new UnsupportedOperationException();
    }

    public void removeDisplayRepaintListener(DisplayRepaintListener l) {
        throw new UnsupportedOperationException();
    }

    public MutableImage getDisplayImage() {
        throw new UnsupportedOperationException();
    }

    public void repaintRequest(int x, int y, int width, int height) {
        throw new UnsupportedOperationException();
    }
}
