# 7. Sound

The formats games use, how unsupported ones are handled, `playTone`,
`PlayerListener` timing, prefetch latency, and Nokia's and other
vendors' sound APIs. Link bases are as in [note 1](01-suite-loading.md).

## Formats games use

| Format | Magic bytes | Typical source | J2ME Loader | freej2me-plus |
| --- | --- | --- | --- | --- |
| MIDI | `MThd` | almost every game | Sonivox EAS, optional SoundFont synth | Java Sound synth, optional SoundFont |
| WAV PCM and IMA ADPCM | `RIFF` | sound effects | Android player | own WAV player |
| MP3 | `ID3` or an MPEG frame sync (`FF Ex`) | later phones | Android player | JLayer |
| AMR-NB, AMR-WB | `#!AMR\n`, `#!AMR-WB\n` | Sony Ericsson, Nokia | Android player | silent stub |
| Tone sequence | none (`audio/x-tone-seq`) | `ToneControl` | yes | converted to MIDI |
| SMAF / MMF | `MMMD` (sometimes at offset 2) | Samsung, Sony Ericsson, Asian phones | converted to MIDI | own decoder and player |
| MLD / MFi, CMF | `melo`, `cmid` | Japanese phones | no | own decoder |
| iMelody, eMelody | `BEGIN:IMELODY`, `BEGIN:EMELODY` | EMS ringtones | no | converted to MIDI |
| Nokia OTA ringtone | none | Nokia `Sound.FORMAT_TONE` | Sonivox OTA parser | yes |
| Qualcomm QCP | `RIFF` with `QLCMfmt` at offset 8 | CDMA phones | no | silent stub |

Sources: [JL Manager](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/Manager.java#L52),
[JL getSupportedContentTypes](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/Manager.java#L121),
[FP PlatformPlayer](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/PlatformPlayer.java#L104),
[FP getSupportedContentTypes](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/Manager.java#L155). [code]
The QCP check must come before the WAV check because QCP files also
start with `RIFF`. [code comment]

J2ME Loader reports `audio/wav`, `audio/x-wav`, `audio/midi`,
`audio/x-midi`, `audio/mpeg`, `audio/aac`, `audio/amr`, `audio/amr-wb`,
`audio/mp3`, `audio/mp4`, `audio/mmf` and `audio/x-tone-seq` as
supported; freej2me-plus reports `audio/midi`, `audio/x-wav`,
`audio/x-mld`, `audio/amr`, `audio/mpeg`, `audio/x-tone-seq`,
`audio/mmf`, `audio/x-imy` and `audio/basic`. [code]

## Unsupported formats and wrong content types

- J2ME Loader: a type it does not list, or a `null` type with no plugin
  match, gives a `BasePlayer` that accepts every call and plays nothing;
  no exception
  ([createPlayer](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/Manager.java#L101)). [code]
- freej2me-plus: matches the type loosely (contains `mid` or `tone`,
  `wav`, `mp`) and otherwise reads the stream and decides from the magic
  bytes; anything unknown becomes a silent `BasicPlayer`. [code]
  Gameloft's Gangstar Rio and Jurassic Park pass audio streams with no
  content type at all ([freej2me issue 202](https://github.com/hex007/freej2me/issues/202)). [tracker]
- freej2me-plus treats any `createPlayer(String)` locator as a JAR
  resource name ([Manager.createPlayer](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/Manager.java#L89));
  J2ME Loader opens `resource://` and `file://` locators, guesses the
  type from the file extension, and returns a silent player for other
  locators. [code]
- A freej2me-plus comment notes that Sonic Spinball (K800i) relies on an
  exception from the Siemens `createPlayer` with a `null` type as part
  of its logic, so "only a lot of testing" will tell which is right
  ([Manager](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/Manager.java#L75)). [code comment]

**For FastEmu:** decide the format from magic bytes first and the
content type second; give unknown or unsupported data a silent player
that walks through the normal state machine and sends `END_OF_MEDIA`
after a short time, and log the content type and the first bytes at
`LogLevel::Warn`. Accept `resource://`, `file://` and bare resource
paths as locators. Throw `MediaException` only where a game is known to
expect it.

## The player state machine and listeners

- J2ME Loader delivers `PlayerListener` events on one extra thread per
  player (`MidletPlayerCallback`), so they arrive in order but after the
  call that caused them
  ([BasePlayer](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/BasePlayer.java#L32)). [code]
- Event data: `STARTED`, `STOPPED` and `END_OF_MEDIA` carry the media
  time as a `Long`; `VOLUME_CHANGED` carries the `VolumeControl`;
  `CLOSED` carries `null`
  ([commit 3be17a15](https://github.com/nikita36078/J2ME-Loader/commit/3be17a15)). [code]
- At the end of media J2ME Loader sends `END_OF_MEDIA`, then either
  rewinds and goes to `PREFETCHED` (last loop) or restarts and sends
  `STARTED` again ([complete](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/media/BasePlayer.java#L69)). [code]
- Loop counts: `setLoopCount(1)` plays once. freej2me played every sound
  one time too many because Java Sound counts extra loops
  ([freej2me issue 197](https://github.com/hex007/freej2me/issues/197)). [tracker]
- House of the Dead Nightmare polls for `END_OF_MEDIA` before replaying a
  short effect; when the effect ended before the game looked, it played
  it again and again ([freej2me-plus issue 197](https://github.com/TASEmulators/freej2me-plus/issues/197)). [tracker]
  So the state a game reads right after `start()` must be `STARTED`
  even for a very short sample.
- Orcs and Elves calls `stop()` on players that are not playing and
  overflowed the stack through listener notifications in freej2me
  ([freej2me issue 200](https://github.com/hex007/freej2me/issues/200));
  it also crashed J2ME Loader in `deallocate`
  ([issue 560](https://github.com/nikita36078/J2ME-Loader/issues/560)). [tracker]
  `stop()` on a player that is not started must be a silent no-op.
- DOOM II RPG and Orcs and Elves II create a new player for every effect
  and never free the old ones; freej2me-plus capped the number of MIDI
  players and reuses idle ones
  ([freej2me issue 198](https://github.com/hex007/freej2me/issues/198),
  [commit 6d035ea](https://github.com/TASEmulators/freej2me-plus/commit/6d035ea)). [tracker]

**For FastEmu:** run state changes synchronously in the calling thread
(the state after `start()` returns is `STARTED`) and deliver listener
calls on one ordered thread per instance. Release the audio resources of
players that are unreachable or idle beyond a cap, without changing their
visible state.

## Prefetch latency

Games realize and prefetch their effects at load time and call
`start()` when the effect is needed; any decoding left to `start()` shows
as lag. Streets of Rage (Sony Ericsson) stuttered on certain effects
until freej2me-plus reworked its players
([issue 63](https://github.com/TASEmulators/freej2me-plus/issues/63));
freej2me-plus also loads its synthesizers before the game starts
"to minimize ingame stutters". [tracker] [code]

**For FastEmu:** decode WAV, MP3 and AMR completely in `prefetch()`
(or `realize()`), render MIDI with a synthesizer that is created once
per instance, and keep `start()` free of allocation and I/O.

## playTone

freej2me-plus `Manager.playTone`
([playTone](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/media/Manager.java#L169)) [code]:

- note outside 0 to 127: `IllegalArgumentException`;
- duration 0 or less: `IllegalArgumentException`;
- volume clamped to 0 to 100;
- tones shorter than 50 ms are lengthened to 50 ms because Karma Studios
  games use 10 ms tones that were otherwise inaudible;
- a new tone cuts off the previous one;
- the sound is a square wave (General MIDI program 81, index 80), on
  their reasoning that phones beeped.

J2ME Loader plays tones through its `ToneManager`. [code]

**For FastEmu:** same argument checks; synthesize a square wave at
`440 * 2^((note - 69) / 12)` Hz without going through MIDI; a minimum
audible length is reasonable.

## Volume and mixing

- Both emulators report `supports.mixing=true`. [code]
- freej2me-plus has a per-game setting to ignore `VolumeControl`
  changes, added because some games' volume changes sounded wrong
  through Java Sound ([issue 145](https://github.com/TASEmulators/freej2me-plus/issues/145)). [tracker]
- Pausing: freej2me-plus stops the players that were running and starts
  them again on resume. [code] J2ME Loader had games that lost sound or
  froze after a trip to the background with music playing
  ([issue 136](https://github.com/nikita36078/J2ME-Loader/issues/136),
  [issue 678](https://github.com/nikita36078/J2ME-Loader/issues/678)). [tracker]

**For FastEmu:** each instance mixes its own players into one stream
with `InstanceSettings::volume` and `Instance::set_volume` applied after
the game's own volume; pausing an instance pauses its stream without
sending player events.

## Nokia Sound (`com.nokia.mid.sound.Sound`)

From J2ME Loader ([Sound](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/sound/Sound.java)) [code]:

- Formats: `FORMAT_TONE` (1) is Nokia's Smart Messaging ringtone (OTA)
  format; `FORMAT_WAV` (5). `getSupportedFormats()` returns both.
- `Sound(freq, duration)` plays a single tone; frequency 0 crashed once
  ([commit e283cbd8](https://github.com/nikita36078/J2ME-Loader/commit/e283cbd8)).
- OTA data is checked and repaired before playback (`ToneVerifier`), and
  Sonivox's OTA parser needed a fix for "New Skool Skater"
  ([commit ff561bc5](https://github.com/nikita36078/J2ME-Loader/commit/ff561bc5)).
- States: `SOUND_PLAYING` 0, `SOUND_STOPPED` 1, `SOUND_UNINITIALIZED` 3.
  Errors while initializing leave the state at `SOUND_UNINITIALIZED`
  instead of throwing.
- `SoundListener` must receive `SOUND_STOPPED` at the end of playback;
  Harry Potter 3 waits for it
  ([commit 12a05b72](https://github.com/nikita36078/J2ME-Loader/commit/12a05b72)).
- Gain is 0 to 255.

freej2me-plus's Turrican versions for 128x128 "spam Nokia Sound error
messages" ([issue 36](https://github.com/TASEmulators/freej2me-plus/issues/36)),
another reason to keep Nokia `Sound` quiet on errors. [tracker]

## Other vendor sound APIs

Implemented in J2ME Loader and freej2me-plus, by package [code]:

- Siemens `com.siemens.mp.game.Sound`, `Melody`, `MelodyComposer`,
  `com.siemens.mp.media` (a copy of MMAPI under Siemens names).
- Samsung `com.samsung.util.AudioClip`. Games use it off-spec: ClickMan
  sets a loop count of -1, which the documentation says must throw
  ([freej2me-plus issue 122](https://github.com/TASEmulators/freej2me-plus/issues/122));
  a Sonic demo passes a missing file and lost all sound for the session
  ([issue 216](https://github.com/TASEmulators/freej2me-plus/issues/216)). [tracker]
  Treat -1 as "loop forever" and a `null` stream as a silent clip.
- Motorola `com.motorola.midi`, Sprint `com.sprintpcs.media`, Vodafone
  `com.vodafone.v10.sound`, KDDI `com.kddi.media`, JBlend SMAF
  `com.jblend.media.smaf`.

These matter most for Asian and operator-specific builds; Nokia `Sound`,
Siemens `Sound` and Samsung `AudioClip` are the ones European game ports
use. [unverified as a count]
