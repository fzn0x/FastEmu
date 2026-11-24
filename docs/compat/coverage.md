# API coverage

How much of the [API inventory](api/README.md) FastEmu registers today.
This page is written by `cargo xtask coverage`; run it again after
changing the runtime's classes or the inventory, and commit the result.

A class counts as present when the runtime registers it. A member counts
as present when a game's reference to it links: a field or method
through the class, its superclasses and its interfaces, as the JVM
resolves it, and a constructor in the class itself. The classes of
`jsr-later.md` that the inventory lists without their members (PIM,
Bluetooth, Location, SVG) count as classes only. Tiers are the
inventory's: 1 for what most games need, 2 for what many need, 3 for the
rest.

## Summary

| Package | Tier 1 classes | Tier 1 members | Tier 2 classes | Tier 2 members | Tier 3 classes | Tier 3 members |
|---|---|---|---|---|---|---|
| [`java.lang`](#javalang) | 26 of 26 | 93 of 93 | 13 of 13 | 145 of 145 | 3 of 3 | 95 of 95 |
| [`java.lang.ref`](#javalangref) | none | none | none | none | 2 of 2 | 5 of 5 |
| [`java.util`](#javautil) | 4 of 4 | 31 of 31 | 6 of 6 | 66 of 66 | 2 of 2 | 44 of 44 |
| [`java.io`](#javaio) | 7 of 7 | 51 of 51 | 7 of 7 | 74 of 74 | 4 of 4 | 61 of 61 |
| [`javax.microedition.midlet`](#javaxmicroeditionmidlet) | 1 of 1 | 6 of 6 | 1 of 1 | 5 of 5 | none | 1 of 1 |
| [`javax.microedition.lcdui`](#javaxmicroeditionlcdui) | 8 of 8 | 78 of 78 | 9 of 9 | 130 of 130 | 10 of 10 | 244 of 244 |
| [`javax.microedition.lcdui.game`](#javaxmicroeditionlcduigame) | 4 of 4 | 29 of 29 | 1 of 1 | 39 of 39 | none | 6 of 6 |
| [`javax.microedition.rms`](#javaxmicroeditionrms) | 3 of 3 | 11 of 11 | 4 of 4 | 18 of 18 | 3 of 3 | 26 of 26 |
| [`javax.microedition.io`](#javaxmicroeditionio) | 3 of 3 | 4 of 4 | 5 of 5 | 20 of 20 | 11 of 11 | 106 of 106 |
| [`javax.microedition.media`](#javaxmicroeditionmedia) | 5 of 5 | 11 of 11 | 1 of 1 | 22 of 22 | 1 of 3 | 20 of 32 |
| [`javax.microedition.media.control`](#javaxmicroeditionmediacontrol) | 1 of 1 | 1 of 1 | 1 of 1 | 14 of 14 | 10 of 10 | 59 of 59 |
| [`javax.microedition.media.protocol`](#javaxmicroeditionmediaprotocol) | none | none | none | none | 3 of 3 | 22 of 22 |
| [`javax.microedition.pki`](#javaxmicroeditionpki) | none | none | none | none | 0 of 2 | 0 of 26 |
| [`com.nokia.mid.ui`](#comnokiamidui) | 3 of 3 | 25 of 25 | 1 of 1 | 18 of 18 | none | 14 of 23 |
| [`com.nokia.mid.sound`](#comnokiamidsound) | 1 of 1 | 8 of 8 | 1 of 1 | 12 of 12 | none | none |
| [`com.siemens.mp.game`](#comsiemensmpgame) | none | none | 10 of 10 | 52 of 52 | none | 85 of 86 |
| [`com.siemens.mp.color_game`](#comsiemensmpcolor_game) | none | none | 5 of 5 | 72 of 72 | none | none |
| [`com.siemens.mp`](#comsiemensmp) | none | none | none | none | 0 of 2 | 0 of 8 |
| [`com.siemens.mp.gsm`](#comsiemensmpgsm) | none | none | none | none | 0 of 3 | 0 of 6 |
| [`com.siemens.mp.io`](#comsiemensmpio) | none | none | none | none | 0 of 3 | 0 of 26 |
| [`com.siemens.mp.lcdui`](#comsiemensmplcdui) | none | none | none | none | 0 of 1 | 0 of 5 |
| [`com.siemens.mp.m55`](#comsiemensmpm55) | none | none | none | none | 0 of 1 | 0 of 19 |
| [`com.siemens.mp.ui`](#comsiemensmpui) | none | none | 1 of 1 | 20 of 20 | none | none |
| [`com.siemens.mp.media`](#comsiemensmpmedia) | none | none | none | none | 0 of 7 | 0 of 53 |
| [`com.siemens.mp.media.control`](#comsiemensmpmediacontrol) | none | none | none | none | 0 of 2 | 0 of 15 |
| [`com.samsung.util`](#comsamsungutil) | none | none | none | none | 0 of 5 | 0 of 26 |
| [`com.motorola.game`](#commotorolagame) | none | none | none | none | 0 of 8 | 0 of 97 |
| [`com.motorola.funlight`](#commotorolafunlight) | none | none | none | none | 0 of 3 | 0 of 30 |
| [`com.motorola.multimedia`](#commotorolamultimedia) | none | none | none | none | 1 of 2 | 2 of 17 |
| [`com.sonyericsson.capuchin`](#comsonyericssoncapuchin) | none | none | none | none | 0 of 8 | 0 of 33 |
| [`com.velox`](#comvelox) | none | none | none | none | 0 of 1 | 0 of 12 |
| [`mmpp.lang`](#mmpplang) | none | none | none | none | 0 of 1 | 0 of 28 |
| [`mmpp.media`](#mmppmedia) | none | none | none | none | 0 of 5 | 0 of 63 |
| [`mmpp.media.phrase`](#mmppmediaphrase) | none | none | none | none | 0 of 4 | 0 of 46 |
| [`mmpp.microedition.lcdui`](#mmppmicroeditionlcdui) | none | none | none | none | 0 of 2 | 0 of 34 |
| [`mmpp.phone`](#mmppphone) | none | none | none | none | 0 of 2 | 0 of 19 |
| [`javax.microedition.pim`](#javaxmicroeditionpim) | none | none | none | none | 0 of 14 | none |
| [`org.w3c.dom`](#orgw3cdom) | none | none | none | none | 0 of 4 | none |
| [`org.w3c.dom.events`](#orgw3cdomevents) | none | none | none | none | 0 of 3 | none |
| [`javax.microedition.m3g`](#javaxmicroeditionm3g) | none | none | 0 of 23 | 0 of 296 | 0 of 7 | 0 of 91 |
| [`com.mascotcapsule.micro3d.v3`](#commascotcapsulemicro3dv3) | none | none | 0 of 10 | 0 of 200 | none | none |
| [`javax.microedition.io.file`](#javaxmicroeditioniofile) | none | none | none | none | 0 of 5 | 0 of 41 |
| [`javax.wireless.messaging`](#javaxwirelessmessaging) | none | none | none | none | 5 of 5 | 16 of 16 |
| [`javax.bluetooth`](#javaxbluetooth) | none | none | none | none | 0 of 13 | none |
| [`javax.obex`](#javaxobex) | none | none | none | none | 0 of 8 | none |
| [`javax.microedition.location`](#javaxmicroeditionlocation) | none | none | none | none | 0 of 13 | none |
| [`javax.microedition.m2g`](#javaxmicroeditionm2g) | none | none | none | none | 0 of 6 | none |
| [`org.w3c.dom.svg`](#orgw3cdomsvg) | none | none | none | none | 0 of 10 | none |
| All | 66 of 66 | 348 of 348 | 66 of 99 | 707 of 1203 | 55 of 201 | 806 of 1521 |

## Differences

Classes and members FastEmu has that differ from the inventory. An
override that is protected in a game and public here links and runs;
the other kinds can break a game. The stubs that test MIDlets compile
against (`cargo xtask stubs`) follow FastEmu, so a test MIDlet declares
such an override public.

- `javax/microedition/midlet/MIDlet.startApp()V` (tier 1): public, the inventory says protected
- `javax/microedition/midlet/MIDlet.pauseApp()V` (tier 1): public, the inventory says protected
- `javax/microedition/midlet/MIDlet.destroyApp(Z)V` (tier 1): public, the inventory says protected
- `javax/microedition/lcdui/Canvas.paint(Ljavax/microedition/lcdui/Graphics;)V` (tier 1): public, the inventory says protected
- `javax/microedition/lcdui/CustomItem.TRAVERSE_HORIZONTAL:I` (tier 3): public, the inventory says protected
- `javax/microedition/lcdui/CustomItem.TRAVERSE_VERTICAL:I` (tier 3): public, the inventory says protected
- `javax/microedition/lcdui/CustomItem.KEY_PRESS:I` (tier 3): public, the inventory says protected
- `javax/microedition/lcdui/CustomItem.KEY_RELEASE:I` (tier 3): public, the inventory says protected
- `javax/microedition/lcdui/CustomItem.KEY_REPEAT:I` (tier 3): public, the inventory says protected
- `javax/microedition/lcdui/CustomItem.POINTER_PRESS:I` (tier 3): public, the inventory says protected
- `javax/microedition/lcdui/CustomItem.POINTER_RELEASE:I` (tier 3): public, the inventory says protected
- `javax/microedition/lcdui/CustomItem.POINTER_DRAG:I` (tier 3): public, the inventory says protected
- `javax/microedition/lcdui/CustomItem.NONE:I` (tier 3): public, the inventory says protected

## By package

### java.lang

From [cldc-java-lang.md](api/cldc-java-lang.md). Tier 1: 26 of 26 classes, 93 of 93 members. Tier 2: 13 of 13 classes, 145 of 145 members. Tier 3: 3 of 3 classes, 95 of 95 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `ArithmeticException` | 1 | yes | 2 of 2 | none |
| `ArrayIndexOutOfBoundsException` | 1 | yes | 3 of 3 | none |
| `ArrayStoreException` | 2 | yes | none | 2 of 2 |
| `Boolean` | 2 | yes | none | 7 of 7 |
| `Character` | 2 | yes | none | 11 of 11 |
| `Class` | 1 | yes | 2 of 2 | 2 of 2 |
| `ClassCastException` | 1 | yes | 2 of 2 | none |
| `ClassNotFoundException` | 1 | yes | 2 of 2 | none |
| `Double` | 2 | yes | none | 11 of 11 |
| `Error` | 2 | yes | none | 2 of 2 |
| `Exception` | 1 | yes | 2 of 2 | none |
| `Float` | 2 | yes | none | 10 of 10 |
| `IllegalAccessException` | 2 | yes | none | 2 of 2 |
| `IllegalArgumentException` | 1 | yes | 2 of 2 | none |
| `IllegalMonitorStateException` | 2 | yes | none | 2 of 2 |
| `IndexOutOfBoundsException` | 2 | yes | none | 2 of 2 |
| `InstantiationException` | 2 | yes | none | 2 of 2 |
| `Integer` | 1 | yes | 7 of 7 | 8 of 8 |
| `InterruptedException` | 1 | yes | 2 of 2 | none |
| `Long` | 2 | yes | none | 9 of 9 |
| `Math` | 1 | yes | 3 of 3 | 15 of 15 |
| `NegativeArraySizeException` | 1 | yes | 2 of 2 | none |
| `NoClassDefFoundError` | 1 | yes | 2 of 2 | none |
| `NullPointerException` | 1 | yes | 2 of 2 | none |
| `NumberFormatException` | 1 | yes | 2 of 2 | none |
| `Object` | 1 | yes | 5 of 5 | 4 of 4 |
| `OutOfMemoryError` | 1 | yes | 2 of 2 | none |
| `Runnable` | 1 | yes | 1 of 1 | none |
| `Runtime` | 1 | yes | 3 of 3 | 1 of 1 |
| `RuntimeException` | 1 | yes | 2 of 2 | none |
| `SecurityException` | 2 | yes | none | 2 of 2 |
| `String` | 1 | yes | 18 of 18 | 26 of 26 |
| `StringBuffer` | 1 | yes | 11 of 11 | 12 of 12 |
| `StringIndexOutOfBoundsException` | 1 | yes | 3 of 3 | none |
| `System` | 1 | yes | 4 of 4 | 1 of 1 |
| `Thread` | 1 | yes | 4 of 4 | 10 of 10 |
| `Throwable` | 1 | yes | 3 of 3 | 2 of 2 |
| `VirtualMachineError` | 2 | yes | none | 2 of 2 |
| `IllegalStateException` | 1 | yes | 2 of 2 | none |

### java.lang.ref

From [cldc-java-lang.md](api/cldc-java-lang.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 2 of 2 classes, 5 of 5 members.

### java.util

From [cldc-java-util.md](api/cldc-java-util.md). Tier 1: 4 of 4 classes, 31 of 31 members. Tier 2: 6 of 6 classes, 66 of 66 members. Tier 3: 2 of 2 classes, 44 of 44 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `Calendar` | 2 | yes | none | 32 of 32 |
| `Date` | 2 | yes | none | 4 of 4 |
| `Enumeration` | 1 | yes | 2 of 2 | none |
| `Hashtable` | 1 | yes | 10 of 10 | 2 of 2 |
| `NoSuchElementException` | 2 | yes | none | 2 of 2 |
| `Random` | 1 | yes | 4 of 4 | 4 of 4 |
| `Stack` | 2 | yes | none | 5 of 5 |
| `TimeZone` | 3 | yes | none | 2 of 2 |
| `Vector` | 1 | yes | 15 of 15 | 7 of 7 |
| `Timer` | 2 | yes | none | 5 of 5 |
| `TimerTask` | 2 | yes | none | 3 of 3 |

### java.io

From [cldc-java-io.md](api/cldc-java-io.md). Tier 1: 7 of 7 classes, 51 of 51 members. Tier 2: 7 of 7 classes, 74 of 74 members. Tier 3: 4 of 4 classes, 61 of 61 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `ByteArrayInputStream` | 1 | yes | 6 of 6 | 2 of 2 |
| `ByteArrayOutputStream` | 1 | yes | 7 of 7 | 1 of 1 |
| `DataInput` | 2 | yes | none | 14 of 14 |
| `DataInputStream` | 1 | yes | 14 of 14 | 7 of 7 |
| `DataOutput` | 2 | yes | none | 13 of 13 |
| `DataOutputStream` | 1 | yes | 10 of 10 | 4 of 4 |
| `EOFException` | 2 | yes | none | 2 of 2 |
| `IOException` | 1 | yes | 2 of 2 | none |
| `InputStream` | 1 | yes | 6 of 6 | 1 of 1 |
| `InputStreamReader` | 2 | yes | none | 5 of 5 |
| `OutputStream` | 1 | yes | 6 of 6 | none |
| `PrintStream` | 2 | yes | none | 18 of 18 |
| `Reader` | 2 | yes | none | 5 of 5 |
| `UnsupportedEncodingException` | 2 | yes | none | 2 of 2 |

### javax.microedition.midlet

From [midp-midlet.md](api/midp-midlet.md). Tier 1: 1 of 1 classes, 6 of 6 members. Tier 2: 1 of 1 classes, 5 of 5 members. Tier 3: none classes, 1 of 1 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `MIDlet` | 1 | yes | 6 of 6 | 3 of 3 |
| `MIDletStateChangeException` | 2 | yes | none | 2 of 2 |

### javax.microedition.lcdui

From [midp-lcdui.md](api/midp-lcdui.md). Tier 1: 8 of 8 classes, 78 of 78 members. Tier 2: 9 of 9 classes, 130 of 130 members. Tier 3: 10 of 10 classes, 244 of 244 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `Alert` | 2 | yes | none | 9 of 9 |
| `AlertType` | 2 | yes | none | 6 of 6 |
| `Canvas` | 1 | yes | 28 of 28 | 13 of 13 |
| `Choice` | 2 | yes | none | 14 of 14 |
| `Command` | 1 | yes | 1 of 1 | 7 of 7 |
| `CommandListener` | 1 | yes | 1 of 1 | none |
| `Display` | 1 | yes | 4 of 4 | 6 of 6 |
| `Displayable` | 1 | yes | 4 of 4 | 4 of 4 |
| `Font` | 1 | yes | 12 of 12 | 6 of 6 |
| `Form` | 2 | yes | none | 8 of 8 |
| `Graphics` | 1 | yes | 20 of 20 | 19 of 19 |
| `Image` | 1 | yes | 8 of 8 | 3 of 3 |
| `Item` | 2 | yes | none | 6 of 6 |
| `List` | 2 | yes | none | 16 of 16 |
| `Screen` | 2 | yes | none | none |
| `StringItem` | 2 | yes | none | 3 of 3 |
| `TextBox` | 2 | yes | none | 6 of 6 |
| `TextField` | 3 | yes | none | 4 of 4 |

### javax.microedition.lcdui.game

From [midp-lcdui-game.md](api/midp-lcdui-game.md). Tier 1: 4 of 4 classes, 29 of 29 members. Tier 2: 1 of 1 classes, 39 of 39 members. Tier 3: none classes, 6 of 6 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `GameCanvas` | 1 | yes | 9 of 9 | 6 of 6 |
| `Layer` | 1 | yes | 7 of 7 | 2 of 2 |
| `LayerManager` | 2 | yes | none | 5 of 5 |
| `Sprite` | 1 | yes | 9 of 9 | 18 of 18 |
| `TiledLayer` | 1 | yes | 4 of 4 | 8 of 8 |

### javax.microedition.rms

From [midp-rms.md](api/midp-rms.md). Tier 1: 3 of 3 classes, 11 of 11 members. Tier 2: 4 of 4 classes, 18 of 18 members. Tier 3: 3 of 3 classes, 26 of 26 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `InvalidRecordIDException` | 2 | yes | none | 2 of 2 |
| `RecordEnumeration` | 2 | yes | none | 5 of 5 |
| `RecordStore` | 1 | yes | 7 of 7 | 7 of 7 |
| `RecordStoreException` | 1 | yes | 2 of 2 | none |
| `RecordStoreFullException` | 2 | yes | none | 2 of 2 |
| `RecordStoreNotFoundException` | 1 | yes | 2 of 2 | none |
| `RecordStoreNotOpenException` | 2 | yes | none | 2 of 2 |

### javax.microedition.io

From [midp-io.md](api/midp-io.md). Tier 1: 3 of 3 classes, 4 of 4 members. Tier 2: 5 of 5 classes, 20 of 20 members. Tier 3: 11 of 11 classes, 106 of 106 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `Connection` | 1 | yes | 1 of 1 | none |
| `ConnectionNotFoundException` | 1 | yes | 2 of 2 | none |
| `Connector` | 1 | yes | 1 of 1 | 6 of 6 |
| `ContentConnection` | 2 | yes | none | 3 of 3 |
| `InputConnection` | 2 | yes | none | 2 of 2 |
| `OutputConnection` | 2 | yes | none | 2 of 2 |
| `StreamConnection` | 2 | yes | none | none |
| `HttpConnection` | 2 | yes | none | 7 of 7 |

### javax.microedition.media

From [midp-media.md](api/midp-media.md). Tier 1: 5 of 5 classes, 11 of 11 members. Tier 2: 1 of 1 classes, 22 of 22 members. Tier 3: 1 of 3 classes, 20 of 32 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `Control` | 1 | yes | none | none |
| `Controllable` | 1 | yes | 1 of 1 | 1 of 1 |
| `Manager` | 1 | yes | 1 of 1 | 4 of 4 |
| `MediaException` | 1 | yes | 2 of 2 | none |
| `Player` | 1 | yes | 7 of 7 | 11 of 11 |
| `PlayerListener` | 2 | yes | none | 6 of 6 |

### javax.microedition.media.control

From [midp-media.md](api/midp-media.md). Tier 1: 1 of 1 classes, 1 of 1 members. Tier 2: 1 of 1 classes, 14 of 14 members. Tier 3: 10 of 10 classes, 59 of 59 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `ToneControl` | 2 | yes | none | 11 of 11 |
| `VolumeControl` | 1 | yes | 1 of 1 | 3 of 3 |

### javax.microedition.media.protocol

From [midp-media.md](api/midp-media.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 3 of 3 classes, 22 of 22 members.

### javax.microedition.pki

From [midp-pki.md](api/midp-pki.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 2 classes, 0 of 26 members.

### com.nokia.mid.ui

From [nokia.md](api/nokia.md). Tier 1: 3 of 3 classes, 25 of 25 members. Tier 2: 1 of 1 classes, 18 of 18 members. Tier 3: none classes, 14 of 23 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `DeviceControl` | 2 | yes | none | 3 of 3 |
| `DirectGraphics` | 1 | yes | 15 of 15 | 12 of 12 |
| `DirectUtils` | 1 | yes | 2 of 2 | 1 of 1 |
| `FullCanvas` | 1 | yes | 8 of 8 | 2 of 2 |

### com.nokia.mid.sound

From [nokia.md](api/nokia.md). Tier 1: 1 of 1 classes, 8 of 8 members. Tier 2: 1 of 1 classes, 12 of 12 members. Tier 3: none classes, none members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `Sound` | 1 | yes | 8 of 8 | 11 of 11 |
| `SoundListener` | 2 | yes | none | 1 of 1 |

### com.siemens.mp.game

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: 10 of 10 classes, 52 of 52 members. Tier 3: none classes, 85 of 86 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `ExtendedImage` | 2 | yes | none | 8 of 8 |
| `GraphicObject` | 2 | yes | none | 3 of 3 |
| `GraphicObjectManager` | 2 | yes | none | 9 of 9 |
| `Light` | 2 | yes | none | 3 of 3 |
| `Melody` | 2 | yes | none | 2 of 2 |
| `MelodyComposer` | 2 | yes | none | 6 of 6 |
| `Sound` | 2 | yes | none | 2 of 2 |
| `Sprite` | 2 | yes | none | 11 of 11 |
| `TiledBackground` | 2 | yes | none | 4 of 4 |
| `Vibrator` | 2 | yes | none | 4 of 4 |

### com.siemens.mp.color_game

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: 5 of 5 classes, 72 of 72 members. Tier 3: none classes, none members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `GameCanvas` | 2 | yes | none | 17 of 17 |
| `Layer` | 2 | yes | none | 17 of 17 |
| `LayerManager` | 2 | yes | none | 10 of 10 |
| `Sprite` | 2 | yes | none | 15 of 15 |
| `TiledLayer` | 2 | yes | none | 13 of 13 |

### com.siemens.mp

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 2 classes, 0 of 8 members.

### com.siemens.mp.gsm

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 3 classes, 0 of 6 members.

### com.siemens.mp.io

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 3 classes, 0 of 26 members.

### com.siemens.mp.lcdui

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 1 classes, 0 of 5 members.

### com.siemens.mp.m55

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 1 classes, 0 of 19 members.

### com.siemens.mp.ui

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: 1 of 1 classes, 20 of 20 members. Tier 3: none classes, none members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `Image` | 2 | yes | none | 20 of 20 |

### com.siemens.mp.media

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 7 classes, 0 of 53 members.

### com.siemens.mp.media.control

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 2 classes, 0 of 15 members.

### com.samsung.util

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 5 classes, 0 of 26 members.

### com.motorola.game

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 8 classes, 0 of 97 members.

### com.motorola.funlight

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 3 classes, 0 of 30 members.

### com.motorola.multimedia

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 1 of 2 classes, 2 of 17 members.

### com.sonyericsson.capuchin

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 8 classes, 0 of 33 members.

### com.velox

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 1 classes, 0 of 12 members.

### mmpp.lang

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 1 classes, 0 of 28 members.

### mmpp.media

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 5 classes, 0 of 63 members.

### mmpp.media.phrase

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 4 classes, 0 of 46 members.

### mmpp.microedition.lcdui

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 2 classes, 0 of 34 members.

### mmpp.phone

From [vendors.md](api/vendors.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 2 classes, 0 of 19 members.

### javax.microedition.pim

From [jsr-later.md](api/jsr-later.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 14 classes, none members.

### org.w3c.dom

From [jsr-later.md](api/jsr-later.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 4 classes, none members.

### org.w3c.dom.events

From [jsr-later.md](api/jsr-later.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 3 classes, none members.

### javax.microedition.m3g

From [jsr-later.md](api/jsr-later.md). Tier 1: none classes, none members. Tier 2: 0 of 23 classes, 0 of 296 members. Tier 3: 0 of 7 classes, 0 of 91 members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `Appearance` | 2 | missing | none | 0 of 13 |
| `Background` | 2 | missing | none | 0 of 19 |
| `Camera` | 2 | missing | none | 0 of 9 |
| `CompositingMode` | 2 | missing | none | 0 of 21 |
| `Graphics3D` | 2 | missing | none | 0 of 32 |
| `Group` | 2 | missing | none | 0 of 7 |
| `Image2D` | 2 | missing | none | 0 of 14 |
| `IndexBuffer` | 2 | missing | none | 0 of 2 |
| `Light` | 2 | missing | none | 0 of 19 |
| `Loader` | 2 | missing | none | 0 of 2 |
| `Material` | 2 | missing | none | 0 of 11 |
| `Mesh` | 2 | missing | none | 0 of 7 |
| `Node` | 2 | missing | none | 0 of 19 |
| `Object3D` | 2 | missing | none | 0 of 12 |
| `PolygonMode` | 2 | missing | none | 0 of 20 |
| `Sprite3D` | 2 | missing | none | 0 of 11 |
| `Texture2D` | 2 | missing | none | 0 of 23 |
| `Transform` | 2 | missing | none | 0 of 15 |
| `Transformable` | 2 | missing | none | 0 of 13 |
| `TriangleStripArray` | 2 | missing | none | 0 of 2 |
| `VertexArray` | 2 | missing | none | 0 of 8 |
| `VertexBuffer` | 2 | missing | none | 0 of 12 |
| `World` | 2 | missing | none | 0 of 5 |

### com.mascotcapsule.micro3d.v3

From [jsr-later.md](api/jsr-later.md). Tier 1: none classes, none members. Tier 2: 0 of 10 classes, 0 of 200 members. Tier 3: none classes, none members.

| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |
|---|---|---|---|---|
| `ActionTable` | 2 | missing | none | 0 of 7 |
| `AffineTrans` | 2 | missing | none | 0 of 42 |
| `Effect3D` | 2 | missing | none | 0 of 26 |
| `Figure` | 2 | missing | none | 0 of 11 |
| `FigureLayout` | 2 | missing | none | 0 of 18 |
| `Graphics3D` | 2 | missing | none | 0 of 57 |
| `Light` | 2 | missing | none | 0 of 14 |
| `Texture` | 2 | missing | none | 0 of 3 |
| `Util3D` | 2 | missing | none | 0 of 3 |
| `Vector3D` | 2 | missing | none | 0 of 19 |

### javax.microedition.io.file

From [jsr-later.md](api/jsr-later.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 5 classes, 0 of 41 members.

### javax.wireless.messaging

From [jsr-later.md](api/jsr-later.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 5 of 5 classes, 16 of 16 members.

### javax.bluetooth

From [jsr-later.md](api/jsr-later.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 13 classes, none members.

### javax.obex

From [jsr-later.md](api/jsr-later.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 8 classes, none members.

### javax.microedition.location

From [jsr-later.md](api/jsr-later.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 13 classes, none members.

### javax.microedition.m2g

From [jsr-later.md](api/jsr-later.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 6 classes, none members.

### org.w3c.dom.svg

From [jsr-later.md](api/jsr-later.md). Tier 1: none classes, none members. Tier 2: none classes, none members. Tier 3: 0 of 10 classes, none members.
