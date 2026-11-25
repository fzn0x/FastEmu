/*
 * A signature-only stand-in for compiling MeBench, the MicroEmulator
 * wrapper of `cargo xtask bench`, where MicroEmulator is not installed,
 * such as CI. It holds no MicroEmulator code: only the names and types
 * MeBench uses, with empty bodies. The bench never runs it: before it
 * measures, it compiles MeBench again against the real microemulator.jar.
 */
package org.microemu;

import java.io.InputStream;

import org.microemu.device.DeviceDisplay;
import org.microemu.device.FontManager;
import org.microemu.device.InputMethod;

public interface EmulatorContext {
    DisplayComponent getDisplayComponent();

    InputMethod getDeviceInputMethod();

    DeviceDisplay getDeviceDisplay();

    FontManager getDeviceFontManager();

    InputStream getResourceAsStream(String name);

    boolean platformRequest(String url);
}
