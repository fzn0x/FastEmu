# System properties games read

`System.getProperty(key)` returns a string the phone defines, or `null`
for keys it does not know. Games read these keys to find out what the
phone is (`microedition.platform`), which optional APIs it has (the
`*.version` keys), and where its files are (`fileconn.dir.*`). They are
not the same as `MIDlet.getAppProperty`, which reads the JAD and
manifest ([midp-midlet.md](midp-midlet.md)).

Rules every key follows:

- A key for an optional API that the phone lacks must return `null`, not
  an empty string: games test `System.getProperty("microedition.m3g.version")
  != null` to decide whether to use M3G. FastEmu must define a `*.version`
  key only when it really implements that API.
- `getProperty` throws `NullPointerException` for a null key and
  `IllegalArgumentException` for an empty one (CLDC 1.1 javadoc).
- In FastEmu, `InstanceSettings::encoding` and `InstanceSettings::locale`
  give `microedition.encoding` and `microedition.locale`,
  `DeviceProfile::platform` gives `microedition.platform`, and
  `InstanceSettings::properties` adds or overrides any other key
  (`fastemu-core`).

Column meanings and tier rules: [README](README.md). A value in `code`
is fixed by the defining document; a value in plain text describes the
format.

## Sources

- [MIDP 2.0 javadoc](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/java/lang/package-summary.html), `java.lang` package description: `microedition.locale` and `microedition.profiles`; [`CommConnection`](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/io/CommConnection.html): `microedition.commports`; [`SocketConnection`](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/io/SocketConnection.html): `microedition.hostname`; [`javax.microedition.midlet`](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/midlet/package-summary.html): the format of `microedition.configuration` and `microedition.profiles`.
- [CLDC 1.1 javadoc, `System.getProperty`](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/System.html): the null and empty key exceptions.
- [MIDP 3.0 javadoc](https://nikita36078.github.io/J2ME_Docs/docs/midp-3_0-fr-javadoc/java/lang/package-summary.html), `java.lang` package description: the descriptions of `microedition.platform`, `microedition.commports` and `microedition.hostname`, and the MIDP 3.0 device ID keys.
- [JSR-135 overview](https://nikita36078.github.io/J2ME_Docs/docs/jsr135/overview-summary.html): the MMAPI keys.
- [JSR-75 FileConnection overview](https://nikita36078.github.io/J2ME_Docs/docs/jsr75/overview-summary.html) and [package description](https://nikita36078.github.io/J2ME_Docs/docs/jsr75/javax/microedition/io/file/package-summary.html); [JSR-75 PIM overview](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/overview-summary.html).
- [JSR-82 `LocalDevice.getProperty`](https://nikita36078.github.io/J2ME_Docs/docs/jsr82_1.1.1_javadoc/javax/bluetooth/LocalDevice.html), [JSR-205 SMS adapter](https://nikita36078.github.io/J2ME_Docs/docs/jsr205/sms_adapter.html), [JSR-179 `javax.microedition.location`](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/package-summary.html), [JSR-184 `javax.microedition.m3g`](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/package-summary.html), [JSR-226 overview](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/overview-summary.html), [JSR-234 overview](https://nikita36078.github.io/J2ME_Docs/docs/jsr234-1_1_1-mrel-javadoc/overview-summary.html), [JSR-256 overview](https://nikita36078.github.io/J2ME_Docs/docs/jsr256/overview-summary.html).
- [Nokia UI API overview](https://nikita36078.github.io/J2ME_Docs/docs/nokiaapi2/overview-summary.html) (later edition): the `com.nokia.mid.ui.*` keys.
- [BlackBerry JDE 7.1 `java.lang.System`](https://nikita36078.github.io/J2ME_Docs/docs/BlackBerry_API_7_1_0/java/lang/System.html): one vendor's list of every key it supports, grouped by API; used to confirm the key names of APIs whose own javadoc is not in the collection (JTWI, SATSA, CHAPI, JAX-RPC, JAXP).
- All javadoc above except Oracle's comes from [J2ME_Docs](https://github.com/nikita36078/J2ME_Docs/tree/fbfdfed7009369b6bf7a5cce39b8a13f178c2c69/docs) at commit `fbfdfed7009369b6bf7a5cce39b8a13f178c2c69`.
- What emulators report: J2ME Loader's [`system.props`](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/assets/defaults/system.props) defaults and [`MicroLoader.setProperties`](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/shell/MicroLoader.java), freej2me-plus' [`MIDletLoader`](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java), MicroEmulator's [`MIDletSystemProperties`](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-javase/src/main/java/org/microemu/app/util/MIDletSystemProperties.java).

## CLDC and MIDP

| Key | T | Defined by | Value |
|---|---|---|---|
| `microedition.platform` | 1 | CLDC | Name of the host platform or device. Phones put the vendor and model first, often with firmware, profile and configuration after: J2ME Loader reports `Nokia6233/05.10`; FastEmu's built-in profiles use strings such as `Nokia6300/2.0 (05.00) Profile/MIDP-2.0 Configuration/CLDC-1.1`, `SonyEricssonK700i/R2AE Profile/MIDP-2.0 Configuration/CLDC-1.1` and `SIEMENS-CX65/2.0 Profile/MIDP-2.0 Configuration/CLDC-1.1` ([device.rs](../../../crates/fastemu-core/src/device.rs)). Games search it for a vendor name (`Nokia`, `SonyEricsson`, `SIEMENS`, `SAMSUNG`, `MOT`, `LG`) to pick key codes and vendor APIs, so a profile must start with the vendor it imitates. |
| `microedition.encoding` | 1 | CLDC | The default character encoding of `String(byte[])`, `getBytes()` and `InputStreamReader`. J2ME Loader defaults to `ISO-8859-1`; freej2me-plus and MicroEmulator use the host's `file.encoding`; FastEmu defaults to `UTF-8` and lets the user pick per game (some games expect `windows-1251` or another regional code page). |
| `microedition.configuration` | 1 | CLDC | `CLDC-1.0` or `CLDC-1.1`, the same format as the `MicroEdition-Configuration` attribute. All three emulators report `CLDC-1.1`. |
| `microedition.profiles` | 1 | MIDP 2.0 | Space-separated profiles; MUST contain `MIDP-2.0` on a MIDP 2.0 phone. Same format as `MicroEdition-Profile`. |
| `microedition.locale` | 1 | MIDP 2.0 | Language, optionally `-` and country, such as `en-US` or `fr-FR`: ISO-639 lower-case language and ISO-3166 upper-case country, separated by a hyphen (not the underscore of Java SE). May be `null` in MIDP 2.0. Games choose their text language from it. |
| `microedition.commports` | 3 | MIDP 2.0 | Comma-separated serial port names for `comm:` URLs. FastEmu has none: `null`. |
| `microedition.hostname` | 3 | MIDP 2.0 | The local host name, if available. |
| `microedition.jtwi.version` | 3 | JSR-185 (JTWI), per the BlackBerry list | The JTWI version, such as `1.0`, on phones that claim JTWI. |
| `microedition.deviceid.imei` and the other `microedition.deviceid.*` and `microedition.subscriberid.*` keys | 3 | MIDP 3.0 | Device and subscriber IDs; MIDP 2.0 phones do not have them. |

## Multimedia (JSR-135, JSR-234)

| Key | T | Defined by | Value |
|---|---|---|---|
| `microedition.media.version` | 2 | JSR-135 | `1.1` or `1.2` on a phone with MMAPI; MUST be `1.2` where `java.security.Permission` exists, and may be either on CLDC, which has no `java.security.Permission`. `null` without MMAPI (a MIDP 2.0 phone with only the audio subset). J2ME Loader reports `1.0`, freej2me-plus `1.1`. |
| `supports.mixing` | 2 | JSR-135 | `true` or `false`. `true` promises two simultaneous `playTone` tones, a tone during playback, and two `Player`s playing at once. Games that see `false` play only one sound at a time, so FastEmu should answer `true`. |
| `supports.audio.capture` | 3 | JSR-135 | `true` or `false`; when `true`, `audio.encodings` must not be empty. FastEmu has no microphone: `false`. |
| `supports.video.capture` | 3 | JSR-135 | `true` or `false`; FastEmu: `false`. |
| `supports.recording` | 3 | JSR-135 | `true` or `false`; FastEmu: `false`. |
| `audio.encodings` | 3 | JSR-135 | Space-separated capture formats in the MMAPI encoding syntax, such as `encoding=pcm&rate=8000`; `null` without audio capture. |
| `video.encodings` | 3 | JSR-135 | Space-separated video capture formats; `null` without video capture. |
| `video.snapshot.encodings` | 3 | JSR-135 | Snapshot formats for `VideoControl.getSnapshot`, first is the default, such as `encoding=png encoding=jpeg`; `null` without snapshots. |
| `streamable.contents` | 3 | JSR-135 | Space-separated content types a `Player` streams while downloading. |
| `microedition.amms.version` | 3 | JSR-234 | `1.1` for AMMS 1.1. |
| `supports.mediacapabilities` | 3 | JSR-234 | At least one of `music`, `audio3d`, `imageencoding`, `imagepostprocessing`, `camera`, `tuner`, space-separated. |
| `tuner.modulations`, `audio.samplerates`, `audio3d.simultaneouslocations`, `camera.orientations`, `camera.resolutions` | 3 | JSR-234 | AMMS capability strings; `null` when the capability is missing. |

## Other optional APIs

| Key | T | Defined by | Value |
|---|---|---|---|
| `microedition.m3g.version` | 2 | JSR-184 | `1.0` or `1.1` when M3G is present, `null` otherwise. 3D games test it. |
| `microedition.io.file.FileConnection.version` | 2 | JSR-75 | `1.0` when FileConnection is present, `null` otherwise. |
| `file.separator` | 3 | JSR-75 | The platform's path separator for display only; FileConnection URLs always use `/`. |
| `fileconn.dir.photos`, `fileconn.dir.videos`, `fileconn.dir.tones`, `fileconn.dir.music`, `fileconn.dir.memorycard`, `fileconn.dir.memorycard.photos`, `.videos`, `.tones`, `.music`, `fileconn.dir.private`, `fileconn.dir.cache`, each with a `.name` variant | 2 | Vendors (used in Nokia's `FileSelect` example, listed by BlackBerry, set by J2ME Loader and freej2me-plus; not in the JSR-75 javadoc) | `file:///` URLs of standard folders, ending in `/`; the `.name` keys give display names. J2ME Loader uses `file:///sys/<app>/private/` and `file:///sys/<app>/cache/`. |
| `microedition.pim.version` | 3 | JSR-75 | `1.0` when PIM is present. |
| `bluetooth.api.version`, `obex.api.version`, `bluetooth.l2cap.receiveMTU.max`, `bluetooth.connected.devices.max`, `bluetooth.connected.inquiry`, `bluetooth.connected.page`, `bluetooth.connected.inquiry.scan`, `bluetooth.connected.page.scan`, `bluetooth.master.switch`, `bluetooth.sd.trans.max`, `bluetooth.sd.attr.retrievable.max` | 3 | JSR-82 | Read through `LocalDevice.getProperty`, not `System.getProperty` (some phones answer both). `bluetooth.api.version` is `1.1.1` for that edition. |
| `microedition.bluetooth.version` | 3 | BlackBerry list | Not in the JSR-82 javadoc; BlackBerry lists it under Bluetooth. |
| `wireless.messaging.sms.smsc` | 3 | JSR-120, JSR-205 | The SMS center number, `+` and digits. |
| `wireless.messaging.mms.mmsc` | 3 | JSR-205 | The MMS center URL. |
| `microedition.location.version` | 3 | JSR-179 | `1.0` or `1.1` (the 1.1 javadoc says `1.1`). |
| `microedition.m2g.version` | 3 | JSR-226 | `1.0`. |
| `microedition.m2g.svg.baseProfile` | 3 | JSR-226 | `tiny`. |
| `microedition.m2g.svg.version` | 3 | JSR-226 | The SVG Tiny version, such as `1.1`. |
| `microedition.sensor.version` | 3 | JSR-256 | `1.2` for the 1.2 javadoc; `null` without the Mobile Sensor API. J2ME Loader reports `1`. |
| `microedition.satsa.version`, `microedition.smartcardslots` | 3 | JSR-177 (`microedition.smartcardslots` in its `APDUConnection` javadoc; the version key per the BlackBerry list) | Security and smart card API. |
| `microedition.chapi.version` | 3 | JSR-211 | Content handler API. |
| `microedition.global.version` | 3 | JSR-238 | Internationalization API. |
| `microedition.sip.version` | 3 | JSR-180 | SIP API. |
| `microedition.contactless.version` | 3 | JSR-257 | Contactless (NFC) API. |
| `xml.jaxp.subset.version`, `xml.rpc.subset.version` | 3 | JSR-172, per the BlackBerry list | XML parsing and web services. |

## Nokia

| Key | T | Defined by | Value |
|---|---|---|---|
| `com.nokia.mid.ui.version` | 2 | Nokia UI API (later edition) | Dot-separated version of the Nokia UI API, such as `1.2`. Absent on Nokia UI API 1.1 phones. |
| `com.nokia.mid.ui.softnotification`, `com.nokia.mid.ui.customfontsize`, `com.nokia.mid.ui.tactilefeedback`, `com.nokia.mid.ui.screensaverprevention` | 3 | Nokia UI API (later edition) | `true` when the feature exists. J2ME Loader sets `com.nokia.mid.ui.softnotification` to `true`. |
| `com.nokia.mid.ui.orientation.version`, `com.nokia.mid.ui.multipointtouch.version`, `com.nokia.mid.ui.locale.version`, `com.nokia.mid.ui.frameanimator.fps`, `com.nokia.mid.ui.frameanimator.pps`, `com.nokia.mid.ui.joystick` | 3 | Nokia UI API (later edition), package and class pages | Versions and capabilities of the later Nokia UI packages. |
| `com.nokia.pointer.number` | 3 | J2ME Loader | The number of touch points; J2ME Loader answers it on multi-touch screens. |
| `com.nokia.mid.cmdline`, `com.nokia.mid.cmdline.instance` | 3 | J2ME Loader | Launch arguments; J2ME Loader sets them when started with arguments. |
| `com.nokia.mid.imei` | 3 | Not verified | Widely quoted as Nokia's IMEI key, but no Nokia document in the sources defines it. If FastEmu answers it, it must be a fake IMEI. |

## Other vendors

| Key | T | Defined by | Value |
|---|---|---|---|
| `com.siemens.IMEI` | 3 | J2ME Loader, freej2me-plus | A fake IMEI (`000000000000000` in J2ME Loader). |
| `com.siemens.OSVersion` | 3 | J2ME Loader, freej2me-plus | `11` in both. |
| `com.siemens.mp.systemfolder.ringingtone`, `com.siemens.mp.systemfolder.pictures` | 3 | J2ME Loader | Paths for `com.siemens.mp.io.File` (`fs/MyStuff/Ringtones`, `fs/MyStuff/Pictures`). |
| `com.sonyericsson.imei` | 3 | J2ME Loader, freej2me-plus | `IMEI 00460101-501594-5-00` in J2ME Loader; the same with `IMEI9` in freej2me-plus. |
| `device.imei` | 3 | J2ME Loader, freej2me-plus | `000000000000000`; freej2me-plus' J-Phone `DeviceControl` reads it. |
| `m.SK_VM`, `m.VENDER`, `m.MODEL`, `m.CARRIER`, `m.COLOR`, `m.MIN`, `MIN`, `com.xce.wipi.version` | 3 | freej2me-plus | Korean SKT and WIPI phones; only games from that market read them. |

Games read IMEI-style keys for licence checks and online IDs. FastEmu
should answer them only with fixed fake values, never with anything from
the host machine.

## What the open emulators report

For reference when choosing FastEmu's defaults:

| Key | J2ME Loader | freej2me-plus | MicroEmulator |
|---|---|---|---|
| `microedition.platform` | `Nokia6233/05.10` | `FreeJ2ME-Plus, a Cross-Platform J2ME Emulator.` | `MicroEmulator` |
| `microedition.configuration` | `CLDC-1.1` | `CLDC-1.1` | `CLDC-1.1` |
| `microedition.profiles` | `MIDP-2.0` | `MIDP-2.0` | `MIDP-2.0` |
| `microedition.encoding` | `ISO-8859-1` | host `file.encoding` | host `file.encoding` |
| `microedition.locale` | host locale as `ll-CC` | `en-US` | host language only, such as `en` |
| `microedition.media.version` | `1.0` | `1.1` | not set |
| `microedition.m3g.version` | `1.1` | `1.1` | not set |
| `microedition.io.file.FileConnection.version` | `1.0` | `1.0` | not set |
| `microedition.pim.version` | `1.0` | `1.0` | not set |
| `microedition.location.version` | `1.0` | not set | not set |
| `microedition.sensor.version` | `1` | `1.0` | not set |
| `supports.mixing` | `true` | `true` | not set |
| `supports.audio.capture`, `supports.video.capture`, `supports.recording` | `true` | `true` | not set |
| `video.snapshot.encodings` | `encoding=png encoding=jpeg encoding=image/png encoding=image/jpeg encoding=image/jpg` | not set | not set |
| `wireless.messaging.sms.smsc` | `+8613800010000` | `+8613800010000` | not set |
| `com.nokia.mid.ui.DirectGraphics.PIXEL_FORMAT` | `565` (J2ME Loader's own setting for `getNativePixelFormat`) | not set | not set |
