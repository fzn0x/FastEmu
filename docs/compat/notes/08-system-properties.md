# 8. System properties

The properties games read with `System.getProperty`, what the emulators
answer, and what FastEmu should answer. Link bases are as in
[note 1](01-suite-loading.md).

## Where the values come from

- **J2ME Loader** keeps a per-game list, seeded from
  [`defaults/system.props`](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/assets/defaults/system.props),
  and sets the locale and file paths at launch
  ([setProperties](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/shell/MicroLoader.java#L219)).
  The game's `System.getProperty` calls are rewritten to a helper that
  answers from that list and otherwise asks Android's own `System`
  ([MidletSystem.getProperty](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/shell/MidletSystem.java#L23)). [code]
- **freej2me-plus** sets a fixed list as host Java system properties and
  as app properties ([MIDletLoader constructor](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L190));
  anything else comes from the host JVM. [code]
- **MicroEmulator** sets four values and falls back to the host JVM
  ([MIDletSystemProperties.getProperty](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-javase/src/main/java/org/microemu/app/util/MIDletSystemProperties.java#L103)). [code]
- **wie** sets `microedition.platform` to `wie` and exposes app properties
  under a `wie.appProperty.` prefix. [code]

All three Java emulators leak host properties such as `os.name`. That
breaks games, see [below](#host-properties-must-not-leak).

## Values

| Property | J2ME Loader | freej2me-plus | MicroEmulator |
| --- | --- | --- | --- |
| `microedition.platform` | `Nokia6233/05.10` | `FreeJ2ME-Plus, a Cross-Platform J2ME Emulator.` | `MicroEmulator` |
| `microedition.configuration` | `CLDC-1.1` | `CLDC-1.1` | `CLDC-1.1` |
| `microedition.profiles` | `MIDP-2.0` | `MIDP-2.0` | `MIDP-2.0` |
| `microedition.encoding` | `ISO-8859-1` (per game) | host `file.encoding` | host `file.encoding` |
| `microedition.locale` | host language and country, `en-US` style | `en-US` | host language only, `en` |
| `microedition.media.version` | `1.0` | `1.1` | |
| `microedition.m3g.version` | `1.1` | `1.1` | |
| `microedition.pim.version` | `1.0` | `1.0` | |
| `microedition.io.file.FileConnection.version` | `1.0` | `1.0` | |
| `microedition.sensor.version` | `1` | `1.0` | |
| `microedition.location.version` | `1.0` | | |
| `microedition.amms.version` | | `1.1` | |
| `microedition.jtwi.version` | | `1.0` | |
| `supports.mixing` | `true` | `true` | |
| `supports.audio.capture`, `supports.video.capture`, `supports.recording` | `true` | `true` | |
| `audio.samplerates` | | `8000 11025 12000 16000 22050 24000 32000 44100 48000` | |
| `video.snapshot.encodings` | `encoding=png encoding=jpeg ...` | | |
| `fileconn.dir.photos`, `.music`, `.private`, `.cache`, `.tones`, `.memorycard` | photos, music, private, cache | photos, music, private, tones, memorycard | |
| `device.imei` | `000000000000000` | `000000000000000` | |
| `com.siemens.IMEI` | `000000000000000` | `000000000005152` | |
| `com.siemens.OSVersion` | `11` | `11` | |
| `com.sonyericsson.imei` | `IMEI 00460101-501594-5-00` | `IMEI9 00460101-501594-5-00` | |
| `com.nokia.mid.ui.softnotification` | `true` | | |
| `com.nokia.pointer.number` | index of the pointer during a callback | | |
| `wireless.messaging.sms.smsc` | `+8613800010000` | `+8613800010000` | |
| `bluetooth.api.version` | | `1.0` | |
| `com.xce.wipi.version`, `m.SK_VM`, `m.VENDER` and other SK Telecom keys | | set | |

J2ME Loader changed `microedition.platform` to a Nokia string on purpose
([commit c265feb3](https://github.com/nikita36078/J2ME-Loader/commit/c265feb3))
and added `microedition.location.version` when it implemented JSR 179
([commit 475116e5](https://github.com/nikita36078/J2ME-Loader/commit/475116e5)). [code]

## Games that check the platform string

- Harry Potter: Find Scabbers checks whether `microedition.platform` is
  `Nokia6230` ([freej2me-plus issue 18](https://github.com/TASEmulators/freej2me-plus/issues/18)). [tracker]
- freej2me-plus therefore scans string constants in game code: in a
  method that mentions `microedition.platform`, any constant containing
  a known vendor or model name (`nokia`, `samsung`, `siemens`,
  `sonyericsson`, `motorola`, `sagem`, `lg`, `n95`, `k800` and others) is
  replaced by its own platform string, so the comparison succeeds
  ([knownModelStrings](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L105),
  [commit 2c8b7f0](https://github.com/TASEmulators/freej2me-plus/commit/2c8b7f0)).
  It also lets a `Nokia-Platform` manifest attribute set the platform.
  The rewrite did not help Nokia 3250 Tunes Studio
  ([issue 143](https://github.com/TASEmulators/freej2me-plus/issues/143)). [code] [tracker]

**For FastEmu:** the platform string comes from the device profile
(`DeviceProfile::platform`) and per-game overrides
(`InstanceSettings::properties`). That is simpler and more honest than
rewriting game constants: pick the profile of the phone the game was
built for. A launcher could suggest a profile from a `Nokia-Platform` or
`MicroEdition-Profile` attribute.

## Host properties must not leak

- A Chinese game reads `os.name`, takes `substring(0, 7)` and checks for
  `Windows` to detect emulators; on Android `os.name` is `Linux`, five
  characters, so the game crashed with
  `StringIndexOutOfBoundsException` ([J2ME Loader issue 863](https://github.com/nikita36078/J2ME-Loader/issues/863),
  [issue 872](https://github.com/nikita36078/J2ME-Loader/issues/872)). [tracker]
  On a phone the property is absent and the game's `null` check skips
  the test. The Great Condor Hero does the same
  ([freej2me-plus issue 158](https://github.com/TASEmulators/freej2me-plus/issues/158)). [tracker]
- J2ME Loader's fix lets the user set any property per game
  ([commit b480414c](https://github.com/nikita36078/J2ME-Loader/commit/b480414c)),
  but an empty value still falls back to the host. [code]

**For FastEmu:** answer only from the profile, the settings and the
table below. Every other key returns `null`, including `os.name`,
`os.arch`, `java.version`, `java.vendor`, `user.dir`, `user.name`,
`file.separator` and `line.separator`. J2ME Loader sets `user.home` for
its own file system code; FastEmu does not need it.

## Advertise only what is implemented

Games read the `*.version` properties to choose a code path: a game
that sees `microedition.m3g.version` uses 3D and crashes if M3G is
missing, while without the property it may use a 2D fallback.
[unverified as a tracker count; it is the documented purpose of these
properties] J2ME Loader and freej2me-plus advertise M3G, PIM, sensors,
capture and more because they implement them, at least partly. [code]

**For FastEmu**, a starting table:

| Property | Value |
| --- | --- |
| `microedition.platform` | `DeviceProfile::platform` |
| `microedition.configuration` | `CLDC-1.1` (or `CLDC-1.0` for profiles that say so) |
| `microedition.profiles` | `MIDP-2.0` (or what the profile's platform string says) |
| `microedition.encoding` | `InstanceSettings::encoding` |
| `microedition.locale` | `InstanceSettings::locale`, `xx-YY` form |
| `microedition.media.version` | `1.1` once MMAPI players exist |
| `supports.mixing` | `true` |
| `supports.audio.capture`, `supports.video.capture`, `supports.recording` | `false` |
| `audio.encodings`, `video.encodings`, `video.snapshot.encodings` | `null` |
| `microedition.m3g.version`, `microedition.pim.version`, `microedition.io.file.FileConnection.version`, `microedition.location.version`, `microedition.sensor.version`, `bluetooth.api.version`, `wireless.messaging.*` | only once the API exists |
| vendor keys (`com.nokia.*`, `com.siemens.*`, `com.sonyericsson.*`) | only on that vendor's profiles |
| `device.imei` and vendor IMEI keys | a fixed fake IMEI, only on profiles whose vendor had the key |

## Memory reports

Not properties, but read for the same reasons:

- `Runtime.totalMemory()` is `DeviceProfile::heap_bytes`, and
  `freeMemory()` must stay below it. Detection code in Chinese games
  treats `totalMemory() == 8000000` as an emulator (8,000,000 bytes,
  not 8 MiB) and rejects totals that are far too large for a phone
  ([freej2me-plus issue 158](https://github.com/TASEmulators/freej2me-plus/issues/158),
  [J2ME Loader issue 872](https://github.com/nikita36078/J2ME-Loader/issues/872)). [tracker]
  FastEmu's presets use powers of two in MiB (2, 4, 8, 16), which pass
  both checks. Keep it that way.
