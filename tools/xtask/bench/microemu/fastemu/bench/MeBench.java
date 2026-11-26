package fastemu.bench;

import java.awt.image.BufferedImage;
import java.io.File;
import java.io.InputStream;
import java.util.ArrayList;
import java.util.List;

import javax.imageio.ImageIO;
import javax.microedition.lcdui.Displayable;
import javax.microedition.midlet.MIDlet;

import org.microemu.DisplayAccess;
import org.microemu.DisplayComponent;
import org.microemu.EmulatorContext;
import org.microemu.MIDletAccess;
import org.microemu.MIDletBridge;
import org.microemu.app.Common;
import org.microemu.app.launcher.Launcher;
import org.microemu.app.ui.DisplayRepaintListener;
import org.microemu.app.ui.noui.NoUiDisplayComponent;
import org.microemu.app.util.DeviceEntry;
import org.microemu.device.DeviceDisplay;
import org.microemu.device.FontManager;
import org.microemu.device.InputMethod;
import org.microemu.device.MutableImage;
import org.microemu.device.impl.DeviceImpl;
import org.microemu.device.j2se.J2SEDevice;
import org.microemu.device.j2se.J2SEDeviceDisplay;
import org.microemu.device.j2se.J2SEFontManager;
import org.microemu.device.j2se.J2SEInputMethod;
import org.microemu.device.ui.EventDispatcher;
import org.microemu.log.Logger;
import org.microemu.log.LoggerAppender;
import org.microemu.log.LoggingEvent;

/**
 * MicroEmulator's own headless runner (org.microemu.app.Headless), set up
 * the same way, plus what `cargo xtask bench` reads: the time of the
 * game's first painted frame, a count of painted frames, the size of the
 * screen the game draws on, the game's log, key presses from a timeline,
 * a screenshot and an end. It opens no window: NoUiDisplayComponent
 * paints into an image in memory.
 *
 * Usage: MeBench --version
 *        MeBench --seconds S [--events T:CODE:p|r,...] [--max-fps N]
 *                [--shot-at S --shot FILE] [MicroEmulator options] SUITE
 *
 * Prints, as FastEmu's `xtask bench-run` does:
 *   RUN first game=0 ms=N
 *   RUN frames game=0 n=N ms=N w=W h=H   (every second; W and H are the
 *                                         current screen's getWidth and
 *                                         getHeight, -1 when none)
 *   RUN shot game=0 w=W h=H
 *   RUN end game=0 status=S frames=N errors=N ms=N
 *   log 0 LEVEL TEXT                     (MicroEmulator's log, where the
 *                                         game's System.out goes)
 */
public final class MeBench {
    private static final long START = System.currentTimeMillis();
    private static final Object LOCK = new Object();
    private static long frames;
    private static long firstAt = -1;
    private static int errors;
    private static boolean ended;

    private MeBench() {}

    static long now() {
        return System.currentTimeMillis() - START;
    }

    static synchronized void out(String line) {
        System.out.println(line);
        System.out.flush();
    }

    public static void main(String[] args) throws Exception {
        double seconds = 30;
        String events = "";
        double shotAt = -1;
        String shot = null;
        List<String> params = new ArrayList<String>();
        // Saves go to files under user.home, which the bench gives each
        // process on its own, as FastEmu writes them to its data folder.
        params.add("--rms");
        params.add("file");
        params.add("--quit");
        // A MIDlet started from a JAR gets strict class loading, which
        // never reaches the Nokia and Siemens APIs on the class path, so a
        // game built on FullCanvas ends at once. Relaxed is what
        // MicroEmulator uses for a MIDlet started by its class name.
        params.add("--appclassloader");
        params.add("relaxed");
        for (int i = 0; i < args.length; i++) {
            if ("--version".equals(args[i])) {
                out("RUN version microemulator=" + Common.class.getPackage().getImplementationVersion());
                return;
            } else if ("--seconds".equals(args[i])) {
                seconds = Double.parseDouble(args[++i]);
            } else if ("--events".equals(args[i])) {
                events = args[++i];
            } else if ("--max-fps".equals(args[i])) {
                // MicroEmulator paints as often as a game asks unless
                // this is set; only its applet sets it.
                EventDispatcher.maxFps = Integer.parseInt(args[++i]);
            } else if ("--shot-at".equals(args[i])) {
                shotAt = Double.parseDouble(args[++i]);
            } else if ("--shot".equals(args[i])) {
                shot = args[++i];
            } else {
                params.add(args[i]);
            }
        }
        Logger.removeAllAppenders();
        Logger.setLocationEnabled(false);
        Logger.addAppender(new LoggerAppender() {
            public void append(LoggingEvent event) {
                String level = "info";
                if (event.getLevel() == LoggingEvent.ERROR) {
                    level = "err";
                    synchronized (LOCK) {
                        errors++;
                    }
                } else if (event.getLevel() == LoggingEvent.WARN) {
                    level = "warn";
                } else if (event.getLevel() == LoggingEvent.DEBUG) {
                    return;
                }
                String text = String.valueOf(event.getMessage()).replace('\n', ' ').replace('\r', ' ');
                out("log 0 " + level + " " + text);
            }
        });
        Runtime.getRuntime().addShutdownHook(new Thread() {
            public void run() {
                end("exited");
            }
        });
        final NoUiDisplayComponent display = new NoUiDisplayComponent();
        display.addDisplayRepaintListener(new DisplayRepaintListener() {
            public void repaintInvoked(MutableImage image) {
                MIDlet current = MIDletBridge.getCurrentMIDlet();
                if (current == null || current instanceof Launcher) {
                    return; // MicroEmulator's own list of MIDlets, not the game
                }
                synchronized (LOCK) {
                    frames++;
                    if (firstAt < 0) {
                        firstAt = now();
                        out("RUN first game=0 ms=" + firstAt);
                        LOCK.notifyAll();
                    }
                }
            }
        });
        EmulatorContext context = new EmulatorContext() {
            private final InputMethod input = new J2SEInputMethod();
            private final DeviceDisplay deviceDisplay = new J2SEDeviceDisplay(this);
            private final FontManager fonts = new J2SEFontManager();

            public DisplayComponent getDisplayComponent() {
                return display;
            }

            public InputMethod getDeviceInputMethod() {
                return input;
            }

            public DeviceDisplay getDeviceDisplay() {
                return deviceDisplay;
            }

            public FontManager getDeviceFontManager() {
                return fonts;
            }

            public InputStream getResourceAsStream(String name) {
                return MIDletBridge.getCurrentMIDlet().getClass().getResourceAsStream(name);
            }

            public boolean platformRequest(String url) {
                return false;
            }
        };
        Common emulator = new Common(context);
        DeviceEntry device = new DeviceEntry("Default device", null, DeviceImpl.DEFAULT_LOCATION, true, false);
        emulator.initParams(params, device, J2SEDevice.class);
        final String[] steps = events.length() == 0 ? new String[0] : events.split(",");
        final long endAt = (long) (seconds * 1000);
        final long shotMs = shotAt < 0 ? -1 : (long) (shotAt * 1000);
        final String shotFile = shot;
        Thread driver = new Thread(new Runnable() {
            public void run() {
                drive(steps, endAt, shotMs, shotFile, display);
            }
        }, "MeBench");
        driver.setDaemon(true);
        driver.start();
        emulator.initMIDlet(true);
    }

    static void end(String status) {
        synchronized (LOCK) {
            if (ended) {
                return;
            }
            ended = true;
            out("RUN end game=0 status=" + status + " frames=" + frames + " errors=" + errors + " ms=" + now());
        }
    }

    static long at(String step) {
        return Long.parseLong(step.substring(0, step.indexOf(':')));
    }

    /** The current screen's size as "w=W h=H", -1 when there is none. */
    static String size() {
        int w = -1;
        int h = -1;
        try {
            MIDletAccess access = MIDletBridge.getMIDletAccess();
            DisplayAccess display = access == null ? null : access.getDisplayAccess();
            Displayable current = display == null ? null : display.getCurrent();
            if (current != null) {
                w = current.getWidth();
                h = current.getHeight();
            }
        } catch (RuntimeException e) {
            // The game is between screens; the next second tells.
        }
        return "w=" + w + " h=" + h;
    }

    /**
     * Presses the timeline's keys from the first frame, reports each
     * second, saves the screenshot, and ends the run.
     */
    static void drive(String[] steps, long endAt, long shotMs, String shotFile, DisplayComponent display) {
        int next = 0;
        long tick = 1000;
        boolean shotTaken = shotMs < 0 || shotFile == null;
        try {
            while (true) {
                long first;
                synchronized (LOCK) {
                    first = firstAt;
                }
                long t = now();
                while (first >= 0 && next < steps.length && first + at(steps[next]) <= t) {
                    String[] p = steps[next].split(":");
                    key(Integer.parseInt(p[1]), "p".equals(p[2]));
                    next++;
                }
                if (t >= tick) {
                    String size = size();
                    synchronized (LOCK) {
                        out("RUN frames game=0 n=" + frames + " ms=" + t + " " + size);
                    }
                    tick += 1000;
                }
                if (!shotTaken && t >= shotMs) {
                    shotTaken = true;
                    screenshot(display, shotFile);
                }
                if (t >= endAt) {
                    break;
                }
                long wake = Math.min(tick, endAt);
                if (!shotTaken) {
                    wake = Math.min(wake, shotMs);
                }
                if (first >= 0 && next < steps.length) {
                    wake = Math.min(wake, first + at(steps[next]));
                }
                synchronized (LOCK) {
                    // The first frame wakes the wait early, to start the keys.
                    long wait = wake - now();
                    if (wait > 0 && firstAt == first) {
                        LOCK.wait(wait);
                    }
                }
            }
        } catch (InterruptedException e) {
            return;
        }
        end("running");
        System.exit(0);
    }

    /** Saves the last painted picture as a PNG. */
    static void screenshot(DisplayComponent display, String file) {
        try {
            MutableImage image = display.getDisplayImage();
            if (image == null) {
                out("RUN shot game=0 w=-1 h=-1");
                return;
            }
            int w = image.getWidth();
            int h = image.getHeight();
            int[] pixels = new int[w * h];
            image.getRGB(pixels, 0, w, 0, 0, w, h);
            BufferedImage picture = new BufferedImage(w, h, BufferedImage.TYPE_INT_ARGB);
            picture.setRGB(0, 0, w, h, pixels, 0, w);
            ImageIO.write(picture, "png", new File(file));
            out("RUN shot game=0 w=" + w + " h=" + h);
        } catch (Exception e) {
            out("log 0 warn the screenshot failed: " + e);
        }
    }

    static void key(int code, boolean press) {
        MIDletAccess access = MIDletBridge.getMIDletAccess();
        DisplayAccess display = access == null ? null : access.getDisplayAccess();
        if (display == null) {
            return;
        }
        if (press) {
            display.keyPressed(code);
        } else {
            display.keyReleased(code);
        }
    }
}
