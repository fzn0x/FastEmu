/*
 * A signature-only stand-in for compiling MeBench, the MicroEmulator
 * wrapper of `cargo xtask bench`, where MicroEmulator is not installed,
 * such as CI. It holds no MicroEmulator code: only the names and types
 * MeBench uses, with empty bodies. The bench never runs it: before it
 * measures, it compiles MeBench again against the real microemulator.jar.
 */
package org.microemu.log;

public class LoggingEvent {
    public static final int DEBUG = 1;

    public static final int INFO = 2;

    public static final int WARN = 3;

    public static final int ERROR = 4;

    public int getLevel() {
        throw new UnsupportedOperationException();
    }

    public String getMessage() {
        throw new UnsupportedOperationException();
    }
}
