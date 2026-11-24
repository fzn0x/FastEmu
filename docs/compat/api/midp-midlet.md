# javax.microedition.midlet (MIDP 2.0)

`javax.microedition.midlet`: the entry point of every game. The runtime
loads the class named by `MIDlet-<n>` in the JAD or manifest, calls its
public no-argument constructor, then drives its life cycle through the
three abstract methods. A MIDlet is in one of three states: Paused (after
construction), Active (after `startApp` returns) and Destroyed.

Column meanings and tier rules: [README](README.md).

## Sources

- [MIDP 2.0 javadoc (JSR-118)](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/index.html), hosted by Oracle: the spec, and the source of every member, throws entry and since tag.
- MicroEmulator 2.0.4 API stub jar [midpapi20](https://repo1.maven.org/maven2/org/microemu/midpapi20/2.0.4/midpapi20-2.0.4.jar) (Maven Central): compiled class files, read for descriptors, flags and `ConstantValue` attributes.
- [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/tree/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java) at commit `e270ed91e67b5cafc18c1991b92ba5249cf41c50` (Apache-2.0): implementation source, checked for constant values and member lists.
- [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/tree/aab3350832c0d82aff1771394d28bff6f66b0aa4/src) at commit `aab3350832c0d82aff1771394d28bff6f66b0aa4` (GPL-3.0): implementation source, checked for constant values and member lists.
- [MicroEmulator](https://github.com/barteo/microemu/tree/e47b656d983deaa67810c776662444483bdbf134/microemulator) at commit `e47b656d983deaa67810c776662444483bdbf134` (LGPL-2.1 or Apache-2.0): implementation source, checked for constant values and member lists.

## Classes

| Class | Kind | Tier | Since | Members |
|---|---|---|---|---|
| [MIDlet](#midlet) | class | 1 | MIDP 1.0 | 10 |
| [MIDletStateChangeException](#midletstatechangeexception) | class | 2 | MIDP 1.0 | 2 |

## javax.microedition.midlet

### MIDlet

`public abstract class javax.microedition.midlet.MIDlet extends java.lang.Object`

Tier 1. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/midlet/MIDlet.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/midlet/MIDlet.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/midlet/MIDlet.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/midlet/MIDlet.java).

Every game subclasses it. The runtime, not the game, must be the only caller of the constructor.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` | protected | 1.0 | `SecurityException` unless the application management software is creating the MIDlet | Games never call it directly; their subclass constructor runs it when the runtime creates the MIDlet. |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `void startApp()` | `startApp()V` | protected abstract | 1.0 | `MIDletStateChangeException` is thrown if the MIDlet cannot start now but might be able to start at a later time | Called once after construction and again after every resume. |
| 1 | `void pauseApp()` | `pauseApp()V` | protected abstract | 1.0 |  | Called when the phone takes the screen away, for example for an incoming call. |
| 1 | `void destroyApp(boolean unconditional)` | `destroyApp(Z)V` | protected abstract | 1.0 | `MIDletStateChangeException` is thrown if the MIDlet wishes to continue to execute (Not enter the Destroyed state). This exception is ignored if unconditional is equal to true | Games save their state here. |
| 1 | `void notifyDestroyed()` | `notifyDestroyed()V` | final | 1.0 |  | How games quit. The runtime must stop the instance without calling `destroyApp`. |
| 2 | `void notifyPaused()` | `notifyPaused()V` | final | 1.0 |  |  |
| 1 | `String getAppProperty(String key)` | `getAppProperty(Ljava/lang/String;)Ljava/lang/String;` | final | 1.0 | `NullPointerException` is thrown if key is null | Returns `null` for unknown keys. See Suite attributes below. |
| 2 | `void resumeRequest()` | `resumeRequest()V` | final | 1.0 |  |  |
| 2 | `boolean platformRequest(String URL)` | `platformRequest(Ljava/lang/String;)Z` | final | 2.0 | `ConnectionNotFoundException` if the platform cannot handle the URL requested | Games open web pages (`http://`) or start calls (`tel:`) with it. It never blocks. Returns `true` when the MIDlet suite must exit before the request can run. FastEmu can hand `http` and `https` URLs to the host browser after asking the user, and return `false`. |
| 3 | `int checkPermission(String permission)` | `checkPermission(Ljava/lang/String;)I` | final | 2.0 |  | Returns 1 allowed, 0 denied, -1 unknown. Permission names unknown to the device are denied. |

### MIDletStateChangeException

`public class javax.microedition.midlet.MIDletStateChangeException extends java.lang.Exception`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/midlet/MIDletStateChangeException.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/midlet/MIDletStateChangeException.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/midlet/MIDletStateChangeException.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/midlet/MIDletStateChangeException.java).

Checked exception declared by `startApp` and `destroyApp`.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

## Life cycle as the spec states it

- The runtime constructs the MIDlet (Paused), then calls `startApp`. A
  `RuntimeException` from `startApp` destroys the MIDlet at once and calls
  `destroyApp`. A `MIDletStateChangeException` from `startApp` means a
  transient failure: the MIDlet stays Paused.
- `pauseApp` is called only in the Active state. A `RuntimeException`
  from it destroys the MIDlet. After a pause the runtime calls
  `startApp` again to resume: `startApp` can run many times in one
  life, and FastEmu must not assume it runs once.
- `destroyApp(true)` must end the MIDlet whatever it throws.
  `destroyApp(false)` may be refused with `MIDletStateChangeException`.
  Runtime exceptions from `destroyApp` are ignored.
- `notifyDestroyed` tells the runtime the MIDlet is done; the runtime does
  not call `destroyApp` after it. This is how games quit, since
  `System.exit` throws `SecurityException` in MIDP.
- `notifyPaused` moves an Active MIDlet to Paused; `resumeRequest` asks
  to be made Active again, which the runtime does by calling `startApp`.

For FastEmu, closing an instance window maps to `destroyApp(true)`, and
pausing an instance (or Android sending the app to the background) maps
to `pauseApp` followed by `startApp` on resume.

## Suite attributes

`getAppProperty` returns attributes from the JAD and the manifest. For an
untrusted suite (every suite FastEmu runs) a JAD attribute overrides the
manifest attribute of the same name. Names are case-sensitive. The JAD is
UTF-8 by default; each line is `name: value` with spaces and tabs around
the value ignored. The attributes MIDP 2.0 defines, from the
[package description](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/midlet/package-summary.html):

| Attribute | Where | Meaning |
|---|---|---|
| `MIDlet-Name` | manifest and JAD, required | Suite name shown to the user. |
| `MIDlet-Version` | manifest and JAD, required | `Major.Minor[.Micro]`, each part 0 to 99. |
| `MIDlet-Vendor` | manifest and JAD, required | Suite vendor. |
| `MIDlet-<n>` | manifest or JAD, required | `name, icon, class` of the nth MIDlet; `n` starts at 1 and the first gap ends the list. The icon may be empty. |
| `MicroEdition-Profile` | manifest or JAD, required | Profiles needed, such as `MIDP-2.0`, space separated. |
| `MicroEdition-Configuration` | manifest or JAD, required | Configuration needed, such as `CLDC-1.0`. |
| `MIDlet-Jar-URL` | JAD, required | Where the JAR is; may be relative to the JAD. |
| `MIDlet-Jar-Size` | JAD, required | JAR size in bytes. |
| `MIDlet-Icon` | optional | Absolute path of a PNG in the JAR for the suite. |
| `MIDlet-Description` | optional | Description of the suite. |
| `MIDlet-Info-URL` | optional | URL with more information. |
| `MIDlet-Data-Size` | optional | Minimum bytes of record store space the suite needs; default 0. |
| `MIDlet-Permissions` | optional | Permissions the suite cannot work without. |
| `MIDlet-Permissions-Opt` | optional | Permissions the suite can work without. |
| `MIDlet-Push-<n>` | optional | Push registration: see `PushRegistry` in [midp-io.md](midp-io.md). |
| `MIDlet-Install-Notify`, `MIDlet-Delete-Notify`, `MIDlet-Delete-Confirm` | optional | Over-the-air install hooks; FastEmu can ignore them. |

Any other attribute that does not start with `MIDlet-` or
`MicroEdition-` belongs to the game. Games read their own settings this
way, and vendor attributes such as `Nokia-MIDlet-Category` or
`Nokia-UI-Enhancement` (which J2ME Loader's
[Descriptor](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/ru/woesss/j2me/jar/Descriptor.java)
also knows) are passed through like any other.

## Cross-check

Every member above comes from the spec javadoc. The script that writes this file compared it with the stub and implementation sources listed under Sources: constant values, member lists and `static` flags. Member lists are compared only where the parsed source holds the implementation's whole class hierarchy. What differs is listed here; a class not listed matched everywhere it was compared.

No differences.
