/*
 * A signature-only stand-in for compiling MeBench, the MicroEmulator
 * wrapper of `cargo xtask bench`, where MicroEmulator is not installed,
 * such as CI. It holds no MicroEmulator code: only the names and types
 * MeBench uses, with empty bodies. The bench never runs it: before it
 * measures, it compiles MeBench again against the real microemulator.jar.
 */
package org.microemu.app;

import java.util.List;

import org.microemu.EmulatorContext;
import org.microemu.app.util.DeviceEntry;

public class Common {
    public Common(EmulatorContext context) {
        throw new UnsupportedOperationException();
    }

    public boolean initParams(List params, DeviceEntry defaultDevice, Class defaultDeviceClass) {
        throw new UnsupportedOperationException();
    }

    public void initMIDlet(boolean startMidlet) {
        throw new UnsupportedOperationException();
    }
}
