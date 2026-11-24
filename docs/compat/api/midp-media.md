# javax.microedition.media (MIDP 2.0 subset and JSR-135)

`javax.microedition.media`: sound, and on some phones video and capture.
The classes are those of the Mobile Media API (MMAPI, JSR-135). MIDP 2.0
itself contains only a subset (the MIDP 2.0 Audio Building Block): the
classes `Manager`, `Player`, `PlayerListener`, `Control`, `Controllable`,
`MediaException`, `ToneControl` and `VolumeControl`, minus a few members.
A phone with MMAPI has every member below. The Since column says
`MIDP 2.0` for what the MIDP 2.0 subset has, `MMAPI` for what only MMAPI
adds, and `MMAPI 1.2` for the two permission classes of the 1.2 edition,
which MIDP 2.0 phones do not have (they use `java.security.Permission`,
which CLDC lacks).

Most games play MIDI music and short WAV or AMR effects: they call
`Manager.createPlayer(getClass().getResourceAsStream("/music.mid"),
"audio/midi")`, then `realize`, `prefetch`, `setLoopCount(-1)` and
`start`. Older games use Nokia's `com.nokia.mid.sound.Sound`
([nokia.md](nokia.md)) or vendor players ([vendors.md](vendors.md))
instead or as well.

A `Player` moves through five states: `UNREALIZED` (100) after
`createPlayer`, `REALIZED` (200) after `realize`, `PREFETCHED` (300) after
`prefetch`, `STARTED` (400) while playing, and `CLOSED` (0) for good.
`start` on an unrealized player realizes and prefetches it first; `stop`
returns to `PREFETCHED`; `deallocate` returns to `REALIZED`. Calls on a
closed player throw `IllegalStateException`. The `Player` events go to
each `PlayerListener` with one of the string constants of
`PlayerListener` as the event name.

Content types the `Manager` javadoc names as common: `audio/x-wav`,
`audio/basic`, `audio/mpeg`, `audio/midi`, `audio/x-tone-seq` and
`video/mpeg`. Locators: `device://tone` and `device://midi` (the
`Manager` constants), `capture://audio` and `capture://video` for
recording, `rtp://` for streaming, `http://` and `file://` for remote and
local media. See [system-properties.md](system-properties.md) for the
`supports.*` and `audio.encodings` properties games read to find out what
works.

Column meanings and tier rules: [README](README.md).

## Sources

- [MMAPI javadoc (JSR-135)](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/index.html), hosted by Oracle (this is the MMAPI 1.2 edition).
- [MIDP 2.0 javadoc (JSR-118)](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/index.html), hosted by Oracle: the spec, and the source of every member, throws entry and since tag.
- MicroEmulator 2.0.4 API stub jar [midpapi20](https://repo1.maven.org/maven2/org/microemu/midpapi20/2.0.4/midpapi20-2.0.4.jar) (Maven Central): compiled class files, read for descriptors, flags and `ConstantValue` attributes.
- MicroEmulator 2.0.4 [microemu-jsr-135](https://repo1.maven.org/maven2/org/microemu/microemu-jsr-135/2.0.4/microemu-jsr-135-2.0.4.jar) jar (the JSR-135 classes MIDP 2.0 does not have).
- [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/tree/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java) at commit `e270ed91e67b5cafc18c1991b92ba5249cf41c50` (Apache-2.0): implementation source, checked for constant values and member lists.
- [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/tree/aab3350832c0d82aff1771394d28bff6f66b0aa4/src) at commit `aab3350832c0d82aff1771394d28bff6f66b0aa4` (GPL-3.0): implementation source, checked for constant values and member lists.
- [MicroEmulator](https://github.com/barteo/microemu/tree/e47b656d983deaa67810c776662444483bdbf134/microemulator) at commit `e47b656d983deaa67810c776662444483bdbf134` (LGPL-2.1 or Apache-2.0): implementation source, checked for constant values and member lists.

## Classes

| Class | Kind | Tier | Since | Members |
|---|---|---|---|---|
| [Control](#control) | interface | 1 | MIDP 2.0 | 0 |
| [Controllable](#controllable) | interface | 1 | MIDP 2.0 | 2 |
| [Manager](#manager) | class | 1 | MIDP 2.0 | 9 |
| [MediaException](#mediaexception) | class | 1 | MIDP 2.0 | 2 |
| [Player](#player) | interface | 1 | MIDP 2.0 | 22 |
| [PlayerListener](#playerlistener) | interface | 2 | MIDP 2.0 | 17 |
| [PlayerPermission](#playerpermission) | class | 3 | MMAPI 1.2 | 6 |
| [RTSPProtocolPermission](#rtspprotocolpermission) | class | 3 | MMAPI 1.2 | 6 |
| [TimeBase](#timebase) | interface | 3 | MMAPI | 1 |
| [FramePositioningControl](#framepositioningcontrol) | interface | 3 | MMAPI | 4 |
| [GUIControl](#guicontrol) | interface | 3 | MMAPI | 2 |
| [MIDIControl](#midicontrol) | interface | 3 | MMAPI | 13 |
| [MetaDataControl](#metadatacontrol) | interface | 3 | MMAPI | 6 |
| [PitchControl](#pitchcontrol) | interface | 3 | MMAPI | 4 |
| [RateControl](#ratecontrol) | interface | 3 | MMAPI | 4 |
| [RecordControl](#recordcontrol) | interface | 3 | MMAPI | 8 |
| [StopTimeControl](#stoptimecontrol) | interface | 3 | MMAPI | 3 |
| [TempoControl](#tempocontrol) | interface | 3 | MMAPI | 2 |
| [ToneControl](#tonecontrol) | interface | 2 | MIDP 2.0 | 11 |
| [VideoControl](#videocontrol) | interface | 3 | MMAPI | 13 |
| [VolumeControl](#volumecontrol) | interface | 1 | MIDP 2.0 | 4 |
| [ContentDescriptor](#contentdescriptor) | class | 3 | MMAPI | 2 |
| [DataSource](#datasource) | class | 3 | MMAPI | 10 |
| [SourceStream](#sourcestream) | interface | 3 | MMAPI | 10 |

## javax.microedition.media

### Control

`public interface javax.microedition.media.Control`

Tier 1. Since MIDP 2.0. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/Control.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/Control.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/Control.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/Control.java).

Marker interface for everything `getControl` returns.

Declares no members of its own.

### Controllable

`public interface javax.microedition.media.Controllable`

Tier 1. Since MIDP 2.0. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/Controllable.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/Controllable.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/Controllable.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/Controllable.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `Control[] getControls()` | `getControls()[Ljavax/microedition/media/Control;` |  | MIDP 2.0 | `IllegalStateException` Thrown if getControls is called in a wrong state. See Player and DataSource for more details |  |
| 1 | `Control getControl(String controlType)` | `getControl(Ljava/lang/String;)Ljavax/microedition/media/Control;` |  | MIDP 2.0 | `IllegalArgumentException` Thrown if controlType is null; `IllegalStateException` Thrown if getControl is called in a wrong state. See Player and DataSource for more details | Takes the interface name, with or without the package: `"VolumeControl"` means `"javax.microedition.media.control.VolumeControl"`. Returns `null` when the control is not supported. |

### Manager

`public final class javax.microedition.media.Manager extends java.lang.Object`

Tier 1. Since MIDP 2.0. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/Manager.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/Manager.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/Manager.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/Manager.java).

Static factory for players.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `TONE_DEVICE_LOCATOR` | `Ljava/lang/String;` | `"device://tone"` | static | MIDP 2.0 | A `Player` for it exposes a `ToneControl` to play a tone sequence. |
| 3 | `MIDI_DEVICE_LOCATOR` | `Ljava/lang/String;` | `"device://midi"` | static | MMAPI | A `Player` for it exposes a `MIDIControl` for live MIDI events. |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `String[] getSupportedContentTypes(String protocol)` | `getSupportedContentTypes(Ljava/lang/String;)[Ljava/lang/String;` | static | MIDP 2.0 |  | `null` lists every type the phone plays. Games pick a sound format from the answer. |
| 3 | `String[] getSupportedProtocols(String content_type)` | `getSupportedProtocols(Ljava/lang/String;)[Ljava/lang/String;` | static | MIDP 2.0 |  |  |
| 2 | `Player createPlayer(String locator)` | `createPlayer(Ljava/lang/String;)Ljavax/microedition/media/Player;` | static | MIDP 2.0 | `MediaException` Thrown if a Player cannot be created for the given locator; `IOException` Thrown if there was a problem connecting with the source pointed to by the locator | From a locator (`device://tone`, `http://...`, `capture://audio`). |
| 1 | `Player createPlayer(InputStream stream, String type)` | `createPlayer(Ljava/io/InputStream;Ljava/lang/String;)Ljavax/microedition/media/Player;` | static | MIDP 2.0 | `MediaException` Thrown if a Player cannot be created for the given stream and type; `IOException` Thrown if there was a problem reading data from the InputStream | The usual way games load sound from the JAR. With a `null` type the manager tries to detect the format from the data. J2ME Loader's `getSupportedContentTypes` answers `audio/wav`, `audio/x-wav`, `audio/midi`, `audio/x-midi`, `audio/mpeg`, `audio/aac`, `audio/amr`, `audio/amr-wb`, `audio/mp3`, `audio/mp4`, `audio/mmf` and `audio/x-tone-seq`; accepting the same spellings is a good start. |
| 2 | `void playTone(int note, int duration, int volume)` | `playTone(III)V` | static | MIDP 2.0 | `MediaException` Thrown if the tone cannot be played due to a device-related problem | Note 0 to 127 in MIDI numbering (69 is A at 440 Hz), duration in milliseconds, volume 0 to 100 (clamped). Does not block. |
| 3 | `Player createPlayer(DataSource source)` | `createPlayer(Ljavax/microedition/media/protocol/DataSource;)Ljavax/microedition/media/Player;` | static | MMAPI | `MediaException` Thrown if a Player cannot be created for the given DataSource; `IOException` Thrown if there was a problem connecting with the source |  |
| 3 | `TimeBase getSystemTimeBase()` | `getSystemTimeBase()Ljavax/microedition/media/TimeBase;` | static | MMAPI |  |  |

### MediaException

`public class javax.microedition.media.MediaException extends java.lang.Exception`

Tier 1. Since MIDP 2.0. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/MediaException.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/MediaException.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/MediaException.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/MediaException.java).

Thrown by `createPlayer`, `realize`, `prefetch` and `start` when media cannot be handled.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | MIDP 2.0 |  |  |
| 1 | `(String reason)` | `<init>(Ljava/lang/String;)V` |  | MIDP 2.0 |  |  |

### Player

`public interface javax.microedition.media.Player extends javax.microedition.media.Controllable`

Tier 1. Since MIDP 2.0. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/Player.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/Player.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/Player.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/Player.java).

See the state rules above.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `UNREALIZED` | `I` | `100` | static | MIDP 2.0 |  |
| 2 | `REALIZED` | `I` | `200` | static | MIDP 2.0 |  |
| 2 | `PREFETCHED` | `I` | `300` | static | MIDP 2.0 |  |
| 2 | `STARTED` | `I` | `400` | static | MIDP 2.0 |  |
| 2 | `CLOSED` | `I` | `0` | static | MIDP 2.0 |  |
| 2 | `TIME_UNKNOWN` | `J` | `-1L` | static | MIDP 2.0 | -1, returned by `getDuration` and `getMediaTime` when unknown. |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `void realize()` | `realize()V` |  | MIDP 2.0 | `IllegalStateException` Thrown if the Player is in the CLOSED state; `MediaException` Thrown if the Player cannot be realized; `SecurityException` Thrown if the caller does not have security permission to realize the Player |  |
| 1 | `void prefetch()` | `prefetch()V` |  | MIDP 2.0 | `IllegalStateException` Thrown if the Player is in the CLOSED state; `MediaException` Thrown if the Player cannot be prefetched; `SecurityException` Thrown if the caller does not have security permission to prefetch the Player |  |
| 1 | `void start()` | `start()V` |  | MIDP 2.0 | `IllegalStateException` Thrown if the Player is in the CLOSED state; `MediaException` Thrown if the Player cannot be started; `SecurityException` Thrown if the caller does not have security permission to start the Player | Prefetches first when needed. After the end of media it plays again from the start, so games replay an effect by calling `start` again. Ignored while already `STARTED`. |
| 1 | `void stop()` | `stop()V` |  | MIDP 2.0 | `IllegalStateException` Thrown if the Player is in the CLOSED state; `MediaException` Thrown if the Player cannot be stopped |  |
| 2 | `void deallocate()` | `deallocate()V` |  | MIDP 2.0 | `IllegalStateException` Thrown if the Player is in the CLOSED state |  |
| 1 | `void close()` | `close()V` |  | MIDP 2.0 |  |  |
| 3 | `void setTimeBase(TimeBase master)` | `setTimeBase(Ljavax/microedition/media/TimeBase;)V` |  | MMAPI | `IllegalStateException` Thrown if the Player is in the UNREALIZED, STARTED or CLOSED state; `MediaException` Thrown if the specified TimeBase cannot be set on the Player |  |
| 3 | `TimeBase getTimeBase()` | `getTimeBase()Ljavax/microedition/media/TimeBase;` |  | MMAPI | `IllegalStateException` Thrown if the Player is in the UNREALIZED or CLOSED state |  |
| 2 | `long setMediaTime(long now)` | `setMediaTime(J)J` |  | MIDP 2.0 | `IllegalStateException` Thrown if the Player is in the UNREALIZED or CLOSED state; `MediaException` Thrown if the media time cannot be set | Microseconds. Games rewind with `setMediaTime(0)` before restarting an effect. |
| 2 | `long getMediaTime()` | `getMediaTime()J` |  | MIDP 2.0 | `IllegalStateException` Thrown if the Player is in the CLOSED state | Microseconds. |
| 1 | `int getState()` | `getState()I` |  | MIDP 2.0 |  |  |
| 2 | `long getDuration()` | `getDuration()J` |  | MIDP 2.0 | `IllegalStateException` Thrown if the Player is in the CLOSED state | Microseconds, or `TIME_UNKNOWN`. |
| 3 | `String getContentType()` | `getContentType()Ljava/lang/String;` |  | MIDP 2.0 | `IllegalStateException` Thrown if the Player is in the UNREALIZED or CLOSED state |  |
| 1 | `void setLoopCount(int count)` | `setLoopCount(I)V` |  | MIDP 2.0 | `IllegalArgumentException` Thrown if the given count is invalid; `IllegalStateException` Thrown if the Player is in the STARTED or CLOSED state | `-1` loops forever; 0 is invalid; 1 is the default. Each pass ends with an `END_OF_MEDIA` event. |
| 2 | `void addPlayerListener(PlayerListener playerListener)` | `addPlayerListener(Ljavax/microedition/media/PlayerListener;)V` |  | MIDP 2.0 | `IllegalStateException` Thrown if the Player is in the CLOSED state | Games watch for `END_OF_MEDIA` to start the next track. |
| 3 | `void removePlayerListener(PlayerListener playerListener)` | `removePlayerListener(Ljavax/microedition/media/PlayerListener;)V` |  | MIDP 2.0 | `IllegalStateException` Thrown if the Player is in the CLOSED state |  |

### PlayerListener

`public interface javax.microedition.media.PlayerListener`

Tier 2. Since MIDP 2.0. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/PlayerListener.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/PlayerListener.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/PlayerListener.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/PlayerListener.java).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `STARTED` | `Ljava/lang/String;` | `"started"` | static | MIDP 2.0 |  |
| 2 | `STOPPED` | `Ljava/lang/String;` | `"stopped"` | static | MIDP 2.0 |  |
| 2 | `END_OF_MEDIA` | `Ljava/lang/String;` | `"endOfMedia"` | static | MIDP 2.0 |  |
| 3 | `DURATION_UPDATED` | `Ljava/lang/String;` | `"durationUpdated"` | static | MIDP 2.0 |  |
| 3 | `DEVICE_UNAVAILABLE` | `Ljava/lang/String;` | `"deviceUnavailable"` | static | MIDP 2.0 |  |
| 3 | `DEVICE_AVAILABLE` | `Ljava/lang/String;` | `"deviceAvailable"` | static | MIDP 2.0 |  |
| 3 | `VOLUME_CHANGED` | `Ljava/lang/String;` | `"volumeChanged"` | static | MIDP 2.0 |  |
| 2 | `ERROR` | `Ljava/lang/String;` | `"error"` | static | MIDP 2.0 |  |
| 2 | `CLOSED` | `Ljava/lang/String;` | `"closed"` | static | MIDP 2.0 |  |
| 3 | `STOPPED_AT_TIME` | `Ljava/lang/String;` | `"stoppedAtTime"` | static | MMAPI |  |
| 3 | `SIZE_CHANGED` | `Ljava/lang/String;` | `"sizeChanged"` | static | MMAPI |  |
| 3 | `RECORD_STARTED` | `Ljava/lang/String;` | `"recordStarted"` | static | MMAPI |  |
| 3 | `RECORD_STOPPED` | `Ljava/lang/String;` | `"recordStopped"` | static | MMAPI |  |
| 3 | `RECORD_ERROR` | `Ljava/lang/String;` | `"recordError"` | static | MMAPI |  |
| 3 | `BUFFERING_STARTED` | `Ljava/lang/String;` | `"bufferingStarted"` | static | MMAPI |  |
| 3 | `BUFFERING_STOPPED` | `Ljava/lang/String;` | `"bufferingStopped"` | static | MMAPI |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void playerUpdate(Player player, String event, Object eventData)` | `playerUpdate(Ljavax/microedition/media/Player;Ljava/lang/String;Ljava/lang/Object;)V` |  | MIDP 2.0 |  |  |

### PlayerPermission

`public final class javax.microedition.media.PlayerPermission extends java.security.Permission`

Tier 3. Since MMAPI 1.2. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/PlayerPermission.html).

MMAPI 1.2 only. Not on MIDP 2.0 phones; its supertype `java.security.Permission` is not in CLDC 1.1. FastEmu can leave it out.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(String name, String actions)` | `<init>(Ljava/lang/String;Ljava/lang/String;)V` |  | MMAPI 1.2 | `NullPointerException` if name or actions is null; `IllegalArgumentException` if action is invalid or if name is zero length |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `boolean implies(Permission p)` | `implies(Ljava/security/Permission;)Z` |  | MMAPI 1.2 |  |  |
| 3 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | MMAPI 1.2 |  |  |
| 3 | `int hashCode()` | `hashCode()I` |  | MMAPI 1.2 |  |  |
| 3 | `String getActions()` | `getActions()Ljava/lang/String;` |  | MMAPI 1.2 |  |  |
| 3 | `PermissionCollection newPermissionCollection()` | `newPermissionCollection()Ljava/security/PermissionCollection;` |  | MMAPI 1.2 |  |  |

### RTSPProtocolPermission

`public final class javax.microedition.media.RTSPProtocolPermission extends javax.microedition.media.GCFPermission`

Tier 3. Since MMAPI 1.2. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/RTSPProtocolPermission.html).

MMAPI 1.2 only. Same as `PlayerPermission`.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(String uri)` | `<init>(Ljava/lang/String;)V` |  | MMAPI 1.2 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `boolean implies(Permission p)` | `implies(Ljava/security/Permission;)Z` |  | MMAPI 1.2 |  |  |
| 3 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | MMAPI 1.2 |  |  |
| 3 | `int hashCode()` | `hashCode()I` |  | MMAPI 1.2 |  |  |
| 3 | `String getActions()` | `getActions()Ljava/lang/String;` |  | MMAPI 1.2 |  |  |
| 3 | `PermissionCollection newPermissionCollection()` | `newPermissionCollection()Ljava/security/PermissionCollection;` |  | MMAPI 1.2 |  |  |

### TimeBase

`public interface javax.microedition.media.TimeBase`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/TimeBase.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/TimeBase.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/TimeBase.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/TimeBase.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `long getTime()` | `getTime()J` |  | MMAPI |  |  |

## javax.microedition.media.control

### FramePositioningControl

`public interface javax.microedition.media.control.FramePositioningControl extends javax.microedition.media.Control`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/control/FramePositioningControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/control/FramePositioningControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/control/FramePositioningControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/control/FramePositioningControl.java).

Video.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `int seek(int frameNumber)` | `seek(I)I` |  | MMAPI |  |  |
| 3 | `int skip(int framesToSkip)` | `skip(I)I` |  | MMAPI |  |  |
| 3 | `long mapFrameToTime(int frameNumber)` | `mapFrameToTime(I)J` |  | MMAPI |  |  |
| 3 | `int mapTimeToFrame(long mediaTime)` | `mapTimeToFrame(J)I` |  | MMAPI |  |  |

### GUIControl

`public interface javax.microedition.media.control.GUIControl extends javax.microedition.media.Control`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/control/GUIControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/control/GUIControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/control/GUIControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/control/GUIControl.java).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `USE_GUI_PRIMITIVE` | `I` | `0` | static | MMAPI |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `Object initDisplayMode(int mode, Object arg)` | `initDisplayMode(ILjava/lang/Object;)Ljava/lang/Object;` |  | MMAPI | `IllegalStateException` Thrown if initDisplayMode is called again after it has previously been called successfully; `IllegalArgumentException` Thrown if the mode or arg argument is invalid. mode must be defined by GUIControl or its subclasses; or a custom mode supported by this implementation. arg must conform to the constraints defined by the respective mode definitions. Refer to the mode definitions for the required type of arg |  |

### MIDIControl

`public interface javax.microedition.media.control.MIDIControl extends javax.microedition.media.Control`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/control/MIDIControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/control/MIDIControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/control/MIDIControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/control/MIDIControl.java).

Live MIDI on a `device://midi` player; a few games play their own music with it.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `NOTE_ON` | `I` | `144` | static | MMAPI |  |
| 3 | `CONTROL_CHANGE` | `I` | `176` | static | MMAPI |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `boolean isBankQuerySupported()` | `isBankQuerySupported()Z` |  | MMAPI |  |  |
| 3 | `int[] getProgram(int channel)` | `getProgram(I)[I` |  | MMAPI | `IllegalArgumentException` Thrown if channel is out of range; `IllegalStateException` Thrown if the player has not been prefetched; `MediaException` Thrown if querying of banks is not supported |  |
| 3 | `int getChannelVolume(int channel)` | `getChannelVolume(I)I` |  | MMAPI | `IllegalArgumentException` Thrown if channel is out of range; `IllegalStateException` Thrown if the player has not been prefetched |  |
| 3 | `void setProgram(int channel, int bank, int program)` | `setProgram(III)V` |  | MMAPI | `IllegalArgumentException` Thrown if any of the given parameters is out of range; `IllegalStateException` Thrown if the player has not been prefetched |  |
| 3 | `void setChannelVolume(int channel, int volume)` | `setChannelVolume(II)V` |  | MMAPI | `IllegalArgumentException` Thrown if channel or volume is out of range; `IllegalStateException` Thrown if the player has not been prefetched |  |
| 3 | `int[] getBankList(boolean custom)` | `getBankList(Z)[I` |  | MMAPI | `MediaException` if this device does not support retrieval of banks; `IllegalStateException` Thrown if the player has not been prefetched |  |
| 3 | `int[] getProgramList(int bank)` | `getProgramList(I)[I` |  | MMAPI | `IllegalArgumentException` Thrown if bank is out of range; `MediaException` Thrown if the device does not support retrieval of programs; `IllegalStateException` Thrown if the player has not been prefetched |  |
| 3 | `String getProgramName(int bank, int prog)` | `getProgramName(II)Ljava/lang/String;` |  | MMAPI | `IllegalArgumentException` Thrown if bank or prog is out of range; `MediaException` Thrown if the bank or program is not installed (internal or custom), or if this device does not support retrieval of program names; `IllegalStateException` Thrown if the player has not been prefetched |  |
| 3 | `String getKeyName(int bank, int prog, int key)` | `getKeyName(III)Ljava/lang/String;` |  | MMAPI | `IllegalArgumentException` Thrown if bank, prog or key is out of range; `MediaException` Thrown if the bank or program is not installed (internal or custom), or if this device does not support retrieval of key names; `IllegalStateException` Thrown if the player has not been prefetched |  |
| 3 | `void shortMidiEvent(int type, int data1, int data2)` | `shortMidiEvent(III)V` |  | MMAPI | `IllegalArgumentException` Thrown if one of the parameters is out of range; `IllegalStateException` Thrown if the player has not been prefetched |  |
| 3 | `int longMidiEvent(byte[] data, int offset, int length)` | `longMidiEvent([BII)I` |  | MMAPI | `IllegalArgumentException` Thrown if any one of the given parameters is not valid; `IllegalStateException` Thrown if the player has not been prefetched |  |

### MetaDataControl

`public interface javax.microedition.media.control.MetaDataControl extends javax.microedition.media.Control`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/control/MetaDataControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/control/MetaDataControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/control/MetaDataControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/control/MetaDataControl.java).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `AUTHOR_KEY` | `Ljava/lang/String;` | `"author"` | static | MMAPI |  |
| 3 | `COPYRIGHT_KEY` | `Ljava/lang/String;` | `"copyright"` | static | MMAPI |  |
| 3 | `DATE_KEY` | `Ljava/lang/String;` | `"date"` | static | MMAPI |  |
| 3 | `TITLE_KEY` | `Ljava/lang/String;` | `"title"` | static | MMAPI |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `String[] getKeys()` | `getKeys()[Ljava/lang/String;` |  | MMAPI |  |  |
| 3 | `String getKeyValue(String key)` | `getKeyValue(Ljava/lang/String;)Ljava/lang/String;` |  | MMAPI | `IllegalArgumentException` Thrown if the given key is null or invalid |  |

### PitchControl

`public interface javax.microedition.media.control.PitchControl extends javax.microedition.media.Control`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/control/PitchControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/control/PitchControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/control/PitchControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/control/PitchControl.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `int setPitch(int millisemitones)` | `setPitch(I)I` |  | MMAPI |  |  |
| 3 | `int getPitch()` | `getPitch()I` |  | MMAPI |  |  |
| 3 | `int getMaxPitch()` | `getMaxPitch()I` |  | MMAPI |  |  |
| 3 | `int getMinPitch()` | `getMinPitch()I` |  | MMAPI |  |  |

### RateControl

`public interface javax.microedition.media.control.RateControl extends javax.microedition.media.Control`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/control/RateControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/control/RateControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/control/RateControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/control/RateControl.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `int setRate(int millirate)` | `setRate(I)I` |  | MMAPI |  |  |
| 3 | `int getRate()` | `getRate()I` |  | MMAPI |  |  |
| 3 | `int getMaxRate()` | `getMaxRate()I` |  | MMAPI |  |  |
| 3 | `int getMinRate()` | `getMinRate()I` |  | MMAPI |  |  |

### RecordControl

`public interface javax.microedition.media.control.RecordControl extends javax.microedition.media.Control`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/control/RecordControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/control/RecordControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/control/RecordControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/control/RecordControl.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void setRecordStream(OutputStream stream)` | `setRecordStream(Ljava/io/OutputStream;)V` |  | MMAPI | `IllegalStateException` Thrown if one of the following conditions is true: startRecord has been called and commit has not been called. setRecordLocation has been called and commit has not been called; `IllegalArgumentException` Thrown if stream is null; `SecurityException` Thrown if the caller does not have security permission to set the record stream |  |
| 3 | `void setRecordLocation(String locator)` | `setRecordLocation(Ljava/lang/String;)V` |  | MMAPI | `IllegalStateException` Thrown if one of the following conditions is true: startRecord has been called and commit has not been called. setRecordStream has been called and commit has not been called; `IllegalArgumentException` Thrown if locator is null; `IOException` Thrown if protocol is valid but the media cannot be created at the specified location; `MediaException` Thrown if the locator is not in URL syntax or it specifies a protocol that is not supported; `SecurityException` Thrown if the caller does not have security permission to set the record location |  |
| 3 | `String getContentType()` | `getContentType()Ljava/lang/String;` |  | MMAPI |  |  |
| 3 | `void startRecord()` | `startRecord()V` |  | MMAPI | `IllegalStateException` Thrown if any of the following conditions is true: if setRecordLocation or setRecordStream has not been called for the first time. If commit has been called and setRecordLocation or setRecordStream has not been called |  |
| 3 | `void stopRecord()` | `stopRecord()V` |  | MMAPI |  |  |
| 3 | `void commit()` | `commit()V` |  | MMAPI | `IOException` Thrown if an I/O error occurs during commit. The current recording is not valid. To record again, setRecordLocation or setRecordStream must be called |  |
| 3 | `int setRecordSizeLimit(int size)` | `setRecordSizeLimit(I)I` |  | MMAPI | `IllegalArgumentException` Thrown if the given size is invalid; `MediaException` Thrown if setting the record size limit is not supported |  |
| 3 | `void reset()` | `reset()V` |  | MMAPI | `IOException` Thrown if the current recording cannot be erased. The current recording is not valid. To record again, setRecordLocation or setRecordStream must be called |  |

### StopTimeControl

`public interface javax.microedition.media.control.StopTimeControl extends javax.microedition.media.Control`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/control/StopTimeControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/control/StopTimeControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/control/StopTimeControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/control/StopTimeControl.java).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `RESET` | `J` | `9223372036854775807L` (`0x7fffffffffffffffL`) | static | MMAPI |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void setStopTime(long stopTime)` | `setStopTime(J)V` |  | MMAPI | `IllegalStateException` Thrown if setStopTime is called on a started Player and the media stop-time has already been set |  |
| 3 | `long getStopTime()` | `getStopTime()J` |  | MMAPI |  |  |

### TempoControl

`public interface javax.microedition.media.control.TempoControl extends javax.microedition.media.control.RateControl`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/control/TempoControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/control/TempoControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/control/TempoControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/control/TempoControl.java).

Tempo of a MIDI player, in milli-beats per minute.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `int setTempo(int millitempo)` | `setTempo(I)I` |  | MMAPI |  |  |
| 3 | `int getTempo()` | `getTempo()I` |  | MMAPI |  |  |

### ToneControl

`public interface javax.microedition.media.control.ToneControl extends javax.microedition.media.Control`

Tier 2. Since MIDP 2.0. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/control/ToneControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/control/ToneControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/control/ToneControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/control/ToneControl.java).

Plays a byte sequence of notes: the `VERSION`, `TEMPO`, `RESOLUTION`, `BLOCK_*`, `PLAY_BLOCK`, `SET_VOLUME` and `REPEAT` constants are the command bytes, `C4` is middle C (60) and `SILENCE` is a rest. The content type `audio/x-tone-seq` names the same format in a file.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `VERSION` | `B` | `-2` | static | MIDP 2.0 |  |
| 2 | `TEMPO` | `B` | `-3` | static | MIDP 2.0 |  |
| 2 | `RESOLUTION` | `B` | `-4` | static | MIDP 2.0 |  |
| 2 | `BLOCK_START` | `B` | `-5` | static | MIDP 2.0 |  |
| 2 | `BLOCK_END` | `B` | `-6` | static | MIDP 2.0 |  |
| 2 | `PLAY_BLOCK` | `B` | `-7` | static | MIDP 2.0 |  |
| 2 | `SET_VOLUME` | `B` | `-8` | static | MIDP 2.0 |  |
| 2 | `REPEAT` | `B` | `-9` | static | MIDP 2.0 |  |
| 2 | `C4` | `B` | `60` | static | MIDP 2.0 |  |
| 2 | `SILENCE` | `B` | `-1` | static | MIDP 2.0 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void setSequence(byte[] sequence)` | `setSequence([B)V` |  | MIDP 2.0 | `IllegalArgumentException` Thrown if the sequence is null or invalid; `IllegalStateException` Thrown if the Player that this control belongs to is in the PREFETCHED or STARTED state | Only before the player is prefetched; throws `IllegalArgumentException` for a malformed sequence. |

### VideoControl

`public interface javax.microedition.media.control.VideoControl extends javax.microedition.media.control.GUIControl`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/control/VideoControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/control/VideoControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/control/VideoControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/control/VideoControl.java).

Video and camera viewfinders; rare in games.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `USE_DIRECT_VIDEO` | `I` | `1` | static | MMAPI |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `Object initDisplayMode(int mode, Object arg)` | `initDisplayMode(ILjava/lang/Object;)Ljava/lang/Object;` |  | MMAPI | `IllegalStateException` Thrown if initDisplayMode is called again after it has previously been called successfully; `IllegalArgumentException` Thrown if the mode or arg argument is invalid. mode must be USE_GUI_PRIMITIVE, USE_DIRECT_VIDEO, or a custom mode supported by this implementation. arg must conform to the conditions defined by the respective mode definitions. Refer to the mode definitions for the required type of arg |  |
| 3 | `void setDisplayLocation(int x, int y)` | `setDisplayLocation(II)V` |  | MMAPI | `IllegalStateException` Thrown if initDisplayMode has not been called |  |
| 3 | `int getDisplayX()` | `getDisplayX()I` |  | MMAPI |  |  |
| 3 | `int getDisplayY()` | `getDisplayY()I` |  | MMAPI |  |  |
| 3 | `void setVisible(boolean visible)` | `setVisible(Z)V` |  | MMAPI | `IllegalStateException` Thrown if initDisplayMode has not been called |  |
| 3 | `void setDisplaySize(int width, int height)` | `setDisplaySize(II)V` |  | MMAPI | `IllegalArgumentException` Thrown if the given width and height are non-positive values; `IllegalStateException` Thrown if initDisplayMode has not been called; `MediaException` Thrown if resizing is not supported or the operation failed due to hardware or software limitations |  |
| 3 | `void setDisplayFullScreen(boolean fullScreenMode)` | `setDisplayFullScreen(Z)V` |  | MMAPI | `MediaException` Thrown if resizing to full screen size is not supported; `IllegalStateException` Thrown if initDisplayMode has not been called |  |
| 3 | `int getSourceWidth()` | `getSourceWidth()I` |  | MMAPI |  |  |
| 3 | `int getSourceHeight()` | `getSourceHeight()I` |  | MMAPI |  |  |
| 3 | `int getDisplayWidth()` | `getDisplayWidth()I` |  | MMAPI | `IllegalStateException` Thrown if initDisplayMode has not been called |  |
| 3 | `int getDisplayHeight()` | `getDisplayHeight()I` |  | MMAPI | `IllegalStateException` Thrown if initDisplayMode has not been called |  |
| 3 | `byte[] getSnapshot(String imageType)` | `getSnapshot(Ljava/lang/String;)[B` |  | MMAPI | `IllegalStateException` Thrown if initDisplayMode has not been called; `MediaException` Thrown if the requested format is not supported or the Player does not support snapshots; `SecurityException` Thrown if the caller does not have the security permission to take the snapshot |  |

### VolumeControl

`public interface javax.microedition.media.control.VolumeControl extends javax.microedition.media.Control`

Tier 1. Since MIDP 2.0. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/control/VolumeControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/control/VolumeControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/control/VolumeControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/control/VolumeControl.java).

Games set music and effect volume with it. Levels 0 to 100.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void setMute(boolean mute)` | `setMute(Z)V` |  | MIDP 2.0 |  |  |
| 2 | `boolean isMuted()` | `isMuted()Z` |  | MIDP 2.0 |  |  |
| 1 | `int setLevel(int level)` | `setLevel(I)I` |  | MIDP 2.0 |  | Clamps to 0 to 100 and returns the level set. |
| 2 | `int getLevel()` | `getLevel()I` |  | MIDP 2.0 |  | -1 only while the player is `REALIZED` and no level has been set. |

## javax.microedition.media.protocol

### ContentDescriptor

`public class javax.microedition.media.protocol.ContentDescriptor extends java.lang.Object`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/protocol/ContentDescriptor.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/protocol/ContentDescriptor.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/protocol/ContentDescriptor.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/protocol/ContentDescriptor.java).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(String contentType)` | `<init>(Ljava/lang/String;)V` |  | MMAPI |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `String getContentType()` | `getContentType()Ljava/lang/String;` |  | MMAPI |  |  |

### DataSource

`public abstract class javax.microedition.media.protocol.DataSource extends java.lang.Object implements javax.microedition.media.Controllable`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/protocol/DataSource.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/protocol/DataSource.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/protocol/DataSource.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/protocol/DataSource.java).

For games that feed a `Player` from their own protocol; rare.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(String locator)` | `<init>(Ljava/lang/String;)V` |  | MMAPI |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `String getLocator()` | `getLocator()Ljava/lang/String;` |  | MMAPI |  |  |
| 3 | `String getContentType()` | `getContentType()Ljava/lang/String;` | abstract | MMAPI | `IllegalStateException` Thrown if the source is not connected |  |
| 3 | `void connect()` | `connect()V` | abstract | MMAPI | `IOException` Thrown if there are IO problems when connect is called; `SecurityException` Thrown if the caller does not have security permission to call connect |  |
| 3 | `void disconnect()` | `disconnect()V` | abstract | MMAPI |  |  |
| 3 | `void start()` | `start()V` | abstract | MMAPI | `IllegalStateException` Thrown if the DataSource is not connected; `IOException` Thrown if the DataSource cannot be started due to some IO problems; `SecurityException` Thrown if the caller does not have security permission to call start |  |
| 3 | `void stop()` | `stop()V` | abstract | MMAPI | `IOException` Thrown if the DataSource cannot be stopped due to some IO problems |  |
| 3 | `SourceStream[] getStreams()` | `getStreams()[Ljavax/microedition/media/protocol/SourceStream;` | abstract | MMAPI | `IllegalStateException` Thrown if the source is not connected |  |
| 3 | `Control[] getControls()` | `getControls()[Ljavax/microedition/media/Control;` | abstract | MMAPI |  |  |
| 3 | `Control getControl(String controlType)` | `getControl(Ljava/lang/String;)Ljavax/microedition/media/Control;` | abstract | MMAPI |  |  |

### SourceStream

`public interface javax.microedition.media.protocol.SourceStream extends javax.microedition.media.Controllable`

Tier 3. Since MMAPI. Spec: [jsr135](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/mm/jsr135/javax/microedition/media/protocol/SourceStream.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/protocol/SourceStream.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/protocol/SourceStream.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/protocol/SourceStream.java).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `NOT_SEEKABLE` | `I` | `0` | static | MMAPI |  |
| 3 | `SEEKABLE_TO_START` | `I` | `1` | static | MMAPI |  |
| 3 | `RANDOM_ACCESSIBLE` | `I` | `2` | static | MMAPI |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `ContentDescriptor getContentDescriptor()` | `getContentDescriptor()Ljavax/microedition/media/protocol/ContentDescriptor;` |  | MMAPI |  |  |
| 3 | `long getContentLength()` | `getContentLength()J` |  | MMAPI |  |  |
| 3 | `int read(byte[] b, int off, int len)` | `read([BII)I` |  | MMAPI | `IOException` if an I/O error occurs |  |
| 3 | `int getTransferSize()` | `getTransferSize()I` |  | MMAPI |  |  |
| 3 | `long seek(long where)` | `seek(J)J` |  | MMAPI | `IOException` Thrown if an I/O error occurs |  |
| 3 | `long tell()` | `tell()J` |  | MMAPI |  |  |
| 3 | `int getSeekType()` | `getSeekType()I` |  | MMAPI |  |  |

## Cross-check

Every member above comes from the JSR-135 javadoc. The script that writes this file compared it with the MIDP 2.0 stub jar, MicroEmulator's JSR-135 jar and the implementation sources listed under Sources: constant values, member lists and `static` flags. The midpapi20 stub holds only the MIDP 2.0 subset, so the members it lacks are exactly the ones marked `MMAPI` in the Since column. Member lists are compared only where the parsed source holds the implementation's whole class hierarchy.

- `javax.microedition.media.Manager`
  - midpapi20.jar does not declare `MIDI_DEVICE_LOCATOR`, `createPlayer(Ljavax/microedition/media/protocol/DataSource;)Ljavax/microedition/media/Player;`, `getSystemTimeBase()Ljavax/microedition/media/TimeBase;`
  - [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/Manager.java) does not declare `getSystemTimeBase()Ljavax/microedition/media/TimeBase;`
  - [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/Manager.java) does not declare `MIDI_DEVICE_LOCATOR`, `createPlayer(Ljavax/microedition/media/protocol/DataSource;)Ljavax/microedition/media/Player;`, `getSystemTimeBase()Ljavax/microedition/media/TimeBase;`
- `javax.microedition.media.Player`
  - midpapi20.jar does not declare `getTimeBase()Ljavax/microedition/media/TimeBase;`, `setTimeBase(Ljavax/microedition/media/TimeBase;)V`
  - [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/Player.java) does not declare `getTimeBase()Ljavax/microedition/media/TimeBase;`, `setTimeBase(Ljavax/microedition/media/TimeBase;)V`
- `javax.microedition.media.PlayerListener`
  - midpapi20.jar does not declare `BUFFERING_STARTED`, `BUFFERING_STOPPED`, `RECORD_ERROR`, `RECORD_STARTED`, `RECORD_STOPPED`, `SIZE_CHANGED`, `STOPPED_AT_TIME`
  - [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/PlayerListener.java) does not declare `BUFFERING_STARTED`, `BUFFERING_STOPPED`, `RECORD_ERROR`, `RECORD_STARTED`, `RECORD_STOPPED`, `SIZE_CHANGED`, `STOPPED_AT_TIME`
  - [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-jsr-135/src/main/java/javax/microedition/media/PlayerListener.java) does not declare `BUFFERING_STARTED`, `BUFFERING_STOPPED`, `RECORD_ERROR`, `RECORD_STARTED`, `RECORD_STOPPED`, `SIZE_CHANGED`, `STOPPED_AT_TIME`
