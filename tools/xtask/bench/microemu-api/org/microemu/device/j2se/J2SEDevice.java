/*
 * A signature-only stand-in for compiling MeBench, the MicroEmulator
 * wrapper of `cargo xtask bench`, where MicroEmulator is not installed,
 * such as CI. It holds no MicroEmulator code: only the names and types
 * MeBench uses, with empty bodies. The bench never runs it: before it
 * measures, it compiles MeBench again against the real microemulator.jar.
 */
package org.microemu.device.j2se;

import org.microemu.device.impl.DeviceImpl;

public class J2SEDevice extends DeviceImpl {
}
