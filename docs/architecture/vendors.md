# Vendor APIs: Nokia, Siemens, Samsung, Motorola, LG

Phone makers shipped their own Java classes beside MIDP: Nokia's UI API
for full-screen canvases, pixel formats and transparency, Siemens' game
packages, Samsung's sound clips, and lights and vibration on every brand.
Games written for one maker's phones link against its classes and stop
without them. This page is for the people who work on those classes in
`fastemu-midp` (`src/nokia/` and `src/vendor/`): which profile gets which
classes, how they sit on the rest of the runtime, and where FastEmu chose
between the javadoc and what other emulators do. The member lists, with
descriptors, constants and tiers, are in `docs/compat/api/nokia.md` and
`docs/compat/api/vendors.md`.

## Which profile gets which classes

Games probe for vendor classes with `Class.forName` to learn which phone
they run on, then pick key codes and code paths from what they find
(`docs/compat/emulator-notes.md`, rule 13). So a vendor's classes exist
only on profiles of that vendor, and a probe for another maker's class
throws `ClassNotFoundException`.

`fastemu_midp::classes(&profile)` builds the class list of one virtual
machine. `vendor::Vendor::of` reads the maker from the first word of the
profile id (`siemens-132x176` is a Siemens; `sonyericsson` or `se`,
`motorola` or `moto`, `samsung`, `lg`, `nokia`); any other word, such as
`generic`, is `Vendor::Generic`. `fastemu-core` needs no field for it.

| Vendor | Classes |
| --- | --- |
| Nokia, Sony Ericsson, generic | Nokia's UI API (`com.nokia.mid.ui`) and sound (`com.nokia.mid.sound`) |
| Siemens | `com.siemens.mp.game`, `com.siemens.mp.color_game`, `com.siemens.mp.ui.Image`, `com.siemens.mp.misc.NativeMem` |
| Samsung | `com.samsung.util`: `AudioClip`, `LCDLight`, `Vibration`, `SM`, `SMS` |
| Motorola | `com.motorola.multimedia.Lighting` |
| LG | `mmpp.media.BackLight`, `mmpp.media.Vibration` |

**Who gets Nokia's UI API.** `nokia.md` suggests offering it on every
profile, as J2ME Loader and freej2me-plus do, but those emulators have no
per-vendor profiles. The notes give one fact to decide by: Nokia's key
codes were copied by Sony Ericsson (J2ME Loader uses the Nokia layout for
it), and a game that finds `FullCanvas` assumes those codes. So Nokia's
API goes with Nokia's key codes: Nokia, Sony Ericsson and the generic
profiles, which use `KeyCodes::NOKIA`. Siemens and Motorola have key codes
of their own and do not get it. No source in `docs/compat/notes` shows
Samsung or LG phones with it, so they do not get it either, though the
Samsung profile has Nokia's key codes, as Samsung phones copied them. A
test checks that every built-in profile with the API has Nokia's key
codes. Sony Ericsson's own Capuchin (Flash) API is rare in games and not
offered.

## Nokia

- **`FullCanvas`** extends `Canvas` and sets the displayable's full-screen
  flag in its constructor. It has no commands: `addCommand` and
  `setCommandListener` throw `IllegalStateException`, as on the phones and
  in J2ME Loader. The runtime passes the soft keys of a canvas without a
  `CommandListener` to the game, so they arrive as `KEY_SOFTKEY1` (-6),
  `KEY_SOFTKEY2` (-7) and `KEY_SOFTKEY3` (-5). The later edition's
  `FullCanvas(boolean transparency)` asks to show what lies beneath the
  canvas; nothing does in FastEmu, so it draws as any other and only
  `getTransparency` reports the request. `DeviceControl` has that
  edition's `KEYMAT_*` constants.
- **`DirectGraphics` is MIDP's `Graphics`.** `lib.rs` passes the
  `Graphics` class through `nokia::direct_graphics::implement`, which adds
  the interface and its methods, so games that cast `(DirectGraphics) g`
  keep working (as in freej2me-plus), and `DirectUtils.getDirectGraphics(g)`
  returns `g` itself. Colour, clip and translation are shared because there
  is only one object. Every call draws through `fastemu-gfx`'s Nokia
  functions (`docs/architecture/graphics.md`): all thirteen pixel formats
  in `drawPixels` and `getPixels`, the manipulations, polygons and
  triangles in their own ARGB colour.
- **`getNativePixelFormat`** answers `TYPE_USHORT_565_RGB`, like J2ME
  Loader. `nokia.md` suggests 4444 or 8888, but note 04 records that
  Nokia's Virtual Me crashed with 8888 because it passes the native format
  to the `short[]` `getPixels`; 565 is a short format every game handles,
  and every format draws anyway.
- **`getPixels(byte[], byte[], ...)`** reads both arrays into copies and
  writes them back only when the call succeeds, since two Java arrays
  cannot be borrowed mutably at once.
- **`DirectUtils.createImage(byte[], ...)`** decodes like
  `Image.createImage(byte[], ...)` and returns a mutable image with its
  alpha; `createImage(w, h, argb)` fills a mutable image that may be
  translucent.
- **`DeviceControl`** never throws: lights and vibration are logged (see
  below). The javadoc throws for lights a phone lacks and levels outside 0
  to 100, but Nokia models differed in their lights and freej2me-plus
  ignores the calls; FastEmu clamps the level.

## Lights and vibration

`vendor::device` is shared by every maker: `set_light`, `flash_lights`,
`vibrate` (for a while or until stopped) and `stop_vibrating`. The
emulated phone has a backlight, other lights and a motor, so nothing
fails. `fastemu-core` has no notification for them, so each request is
logged at `LogLevel::Info`, as MIDP's `Display.vibrate` is. Games set the
backlight every frame to keep the screen lit, so a light is logged only
when its level changes, and stopping the motor only when it ran without an
end.

## Siemens

- **`com.siemens.mp.game`.** `Light` and `Vibrator` use the device helper.
  `Sound.playTone` plays the MIDI note nearest the frequency.
  `MelodyComposer` keeps the notes and makes a standard MIDI file when
  asked for a `Melody`, following freej2me-plus: `TONE_C0` is MIDI note 24,
  a quarter note is a beat at the composer's tempo, `TONE_REPEAT` repeats
  what came before it and `TONE_REPEAT_MARK` what followed the last
  `TONE_MARK`, `TONE_STOP` ends the melody; `TONE_REPEV` and `TONE_REPON`,
  which no source explains, are skipped. One melody plays at a time.
- **Bitmaps** (`vendor/siemens/bitmap.rs`) are packed rows, the left pixel
  in the high bit: one bit per pixel (set is black) with an optional mask
  (clear is transparent), or two bits per pixel (0 transparent, 1 white, 2
  and 3 black). A mask given as a MIDP image is transparent where white.
  `ui.Image.createRGBImage` takes `RRRGGGBB` bytes with `0xC0`
  transparent, as freej2me-plus found games use it; only `COLOR_BMP_8BIT`
  is accepted, as the javadoc says. `mirrorImage*` mirror the image in
  place. The scaling constructors and `createImageWithScaling` load the
  picture at its own size.
- **`ExtendedImage`** wraps the game's image when it is mutable, so drawing
  on either shows in both, and a mutable copy otherwise. An image with
  transparent pixels is two bits per pixel, any other one bit, as in
  freej2me-plus. `blitToScreen` draws onto a copy of the shown canvas's back
  buffer and flushes that region, so it reaches the screen at once.
- **Sprites and tiled backgrounds** keep their frames and tiles as
  pictures; a background keeps the game's map array, so changing it
  changes what is drawn, and the map repeats over the drawing area as the
  javadoc says. `GraphicObjectManager` holds an object once, so
  `addObject` of one it has changes nothing, and counts its list against
  the heap; so do the frames and tiles a sprite or background is made of.
  `GraphicObjectManager.paint` draws visible objects first to
  last, offset by its `x` and `y`, and refuses targets with transparent
  pixels. Collisions compare collision rectangles, as J2ME Loader does.
- **`com.siemens.mp.color_game`** is Siemens' copy of MIDP 2.0's game
  package. `GameCanvas` uses the runtime's key states and `canvas::flush`
  (`docs/architecture/midp.md`); its off-screen buffer is a full-screen
  mutable image in a private field, copied before each flush so no image
  lock is held while the flush waits for its frame slot. `Layer` keeps its
  state in a payload shared by `Sprite` and `TiledLayer`, and has protected
  constructors, so games may subclass it; `LayerManager` paints every layer
  through its `paint`, last to first, inside the view window, and gives the
  `Graphics` its translation and clip back. Sprites have no reference
  pixel or transforms, as Siemens' javadoc lists none. This package does
  not share code with `javax.microedition.lcdui.game`, which was not there
  when it was written; the two could share it later.

## Samsung, Motorola, LG

- **`AudioClip`** follows freej2me-plus: a type outside 1 to 3 is taken
  for MMF, the bytes' signature decides the format, `play` starts from the
  beginning, a loop count of 0 or 255 loops for ever, and the volume runs
  0 to 5 (0 to 100 for MMF). FastEmu cannot play MMF (Yamaha SMAF), so
  those clips stay silent with a warning, as any sound the engine cannot
  play does.
- **Text messages are never sent.** `SM` keeps its fields and checks them
  as the javadoc says (a phone number of digits, `+`, `*` and `#`, at most
  80 characters of text). `SMS.isSupported()` is false and `SMS.send` logs
  the attempt and throws the `IllegalStateException` the javadoc names for
  phones without messaging.
- **Motorola and LG** get their backlight and vibration classes on the
  device helper. LG's backlight takes any colour (logged when it changes)
  and its vibration has five levels, as in freej2me-plus.

## Sound

`vendor::clip` opens a player for a vendor sound in the instance's group,
with the VM lock released, and logs a warning when the engine refuses the
data. Without a group (the headless runner, tests) there is no player and
the vendor classes behave as if the sound played.

## Testing

- Unit tests beside the code: the vendor of each profile id, bitmap
  unpacking, MIDI writing, melody expansion, collision shapes, the device
  log.
- `tests/vendor_support` compiles the programs in `tests/vendor_java`
  against the stubs in `tests/vendor_stubs` and runs them on a chosen
  profile, with or without an offline sound engine, or packs them into a
  suite for the launcher.
- `tests/nokia_ui.rs`: `DirectGraphics` in every pixel format, checked
  pixel by pixel, and a `FullCanvas` MIDlet's size, commands and key codes
  on three profiles.
- `tests/vendor_siemens.rs`, `tests/vendor_others.rs`: each maker's
  classes, their exceptions and their device log.
- `tests/vendor_probe.rs`: a MIDlet probing with `Class.forName` on every
  built-in profile and on Samsung and LG ids.

## Not done yet

- Tier 3 classes: Siemens' `MIDlet`, `gsm`, `io`, `media`, `m55` and
  `lcdui.Image`, Samsung's extras, Motorola's game and FunLight packages,
  Sony Ericsson's Capuchin, LG's other `mmpp` classes, and the later Nokia
  UI classes (`TextEditor`, gestures, orientation).
- Telling the app about lights and vibration, which only reach the log.
- Playing MMF clips.
