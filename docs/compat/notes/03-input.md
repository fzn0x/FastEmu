# 3. Input

Key codes per vendor, game actions, key repeat, `GameCanvas` key
states, pointers, soft keys with Commands, and full-screen mode. Link
bases are as in [note 1](01-suite-loading.md).

## Vendor key codes

The twelve ITU-T keys send their character code on every phone
(`KEY_NUM0` is 48, `KEY_STAR` 42, `KEY_POUND` 35). [code in all
emulators] The other keys are vendor-specific. What the emulators
define, verified in their source:

| Layout | Up | Down | Left | Right | Fire | Left soft | Right soft | Clear | Send | End | Source |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Nokia (and most others) | -1 | -2 | -3 | -4 | -5 | -6 | -7 | -8 | -10 | -11 | JL, FP, ME |
| Sony Ericsson | as Nokia | | | | | | | | | | JL |
| Siemens | -59 | -60 | -61 | -62 | -26 | -1 | -4 | | | | FP |
| Siemens (J2ME Loader) | -59 | -60 | -61 | -62 | -5 | -1 | -4 | | | | JL |
| Motorola | -1 | -6 | -2 | -5 | -20 | -21 | -22 | | | | JL, FP |
| Motorola V8 | -1 | -2 | -3 | -4 | -5 | -21 | -22 | | | | FP |
| Motorola "triplets" | 1 | 6 | 2 | 5 | 20 | 21 | 22 | | | | FP |
| Motorola A1000 | -1 | -2 | -3 | -4 | 13 | -10 | -11 | | | | FP |
| LG | -1 | -2 | -3 | -4 | -5 | -202 | -203 | -204 | | | FP |
| Sagem | -1 | -2 | -3 | -4 | -5 | -7 | -6 | | | | FP |
| BlackBerry 89xx | 1 | 6 | 2 | 5 | 100 | 113 | 112 | -88 | | | FP |
| KDDI (Japan) | 1 | 6 | 2 | 5 | 8 | 20 | 21 | 8 | | | FP |

Extra keys:

- **Sony Ericsson gaming keys** A and B send -13 and -14; J2ME Loader's
  on-screen keypad has buttons for them
  ([KeyMapper](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/keyboard/KeyMapper.java)). [code]
- **Nokia E61/E62 QWERTY**: the number keys share letter keys, so the
  phone sends the letters' codes: 0 = 109 (`m`), 1 = 114 (`r`),
  2 = 116 (`t`), 3 = 121 (`y`), 4 = 102 (`f`), 5 = 103 (`g`),
  6 = 104 (`h`), 7 = 118 (`v`), 8 = 98 (`b`), 9 = 110 (`n`), `*` = 117
  (`u`), `#` = 106 (`j`). J2ME Loader and freej2me-plus both have this
  layout. [code]
- **SK Telecom (Korea)** uses positive codes: up 141, left 142, right
  145, down 146, fire 148, clear 8; freej2me-plus and wie agree
  ([FP Canvas](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Canvas.java),
  [WIE MIDPKeyCode](https://github.com/dlunch/wie/blob/eb5b809d693ee4f0023f06acd0ad6393006578fc/wie-midp/src/classes/net/wie/event_queue.rs#L61)). [code]
  wie uses these SKVM codes for its J2ME games too, so its J2ME key
  handling does not match Nokia-targeted games.

Sources: [FP Mobile.java key constants](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/Mobile.java#L249),
[JL Canvas constants](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Canvas.java),
[JL KeyMapper.remapKeys](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/keyboard/KeyMapper.java#L110),
[JL FullCanvas](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/ui/FullCanvas.java),
[ME ButtonDetaultDeviceKeyCodes](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-javase/src/main/java/org/microemu/device/impl/ButtonDetaultDeviceKeyCodes.java#L83). [code]

freej2me-plus once had a "Sharp" preset; it is now renamed to Motorola
triplets because the two were identical
([Config](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/freej2me/Config.java#L238),
[issue 214](https://github.com/TASEmulators/freej2me-plus/issues/214)). [code]

## Checking the presets in `fastemu-core`

### Nokia

`KeyCodes::NOKIA` (soft -6/-7, arrows -1 to -4, fire -5, clear -8,
send -10, end -11) matches J2ME Loader, freej2me-plus, MicroEmulator and
Nokia's own `FullCanvas` constants. **Correct.** The doc comment says
Sony Ericsson and Samsung copied these codes. Sony Ericsson: J2ME Loader
uses the Nokia layout for it. Samsung: none of the emulators has a
Samsung layout, so the claim is unverified.

### Siemens

`KeyCodes::SIEMENS` has soft -1/-4, arrows -59 to -62, fire -26: these
match freej2me-plus exactly. J2ME Loader's Siemens layout remaps only
the arrows and soft keys and leaves fire at -5. **Correct as far as the
sources go.**

`clear: -12`, `send: -11` and `end: -12` appear in none of the studied
emulators. Clear and end sharing one code means a game cannot tell them
apart, and the round-trip test in `device.rs` already skips Siemens for
that reason. **Unverified; needs a source** (a Siemens SDK device file
or a real phone) before games rely on it. If no source turns up, keep
the Nokia values for these three keys, which is what J2ME Loader's
Siemens layout effectively sends.

### Motorola

`KeyCodes::MOTOROLA` has soft -21/-22, up -1, down -6, left -2,
right -5, fire -20: these match J2ME Loader and freej2me-plus.
**Correct.** `clear: -8`, `send: -10` and `end: -11` are copied from
Nokia; neither emulator defines them for Motorola. **Unverified.**

Motorola shipped other layouts as well (V8, "triplets" with positive
codes, A1000, see the table). The `motorazrV3` platform string of the
preset fits the -1/-6/-2/-5 layout according to the freej2me-plus
naming, but that pairing is unverified.

### Presets to add

With sources in the table above: `LG` (soft -202/-203, clear -204),
`SAGEM` (Nokia with the soft keys swapped), `MOTOROLA_V8`,
`MOTOROLA_TRIPLETS`, `MOTOROLA_A1000`, and a Nokia E61/E62 QWERTY layout
that changes the ITU-T keys. The current `KeyCodes` type cannot express
the E62 layout, because `Key::itu_code` fixes the digits to their
characters; that needs either an override table in `KeyCodes` or
`InputEvent::RawKey`. Sony Ericsson's -13/-14 gaming keys can be sent as
`InputEvent::RawKey` without a type change.

## getGameAction

| Key | J2ME Loader | freej2me-plus | MicroEmulator | wie |
| --- | --- | --- | --- | --- |
| Arrows, fire | UP, DOWN, LEFT, RIGHT, FIRE | same | same | same |
| 2, 4, 6, 8, 5 | UP, LEFT, RIGHT, DOWN, FIRE | same | 0 | 0 |
| 1 | 0 | GAME_A | GAME_A | GAME_A |
| 3 | 0 | GAME_B | GAME_B | GAME_B |
| 7 | GAME_A | GAME_C | GAME_C | GAME_C |
| 9 | GAME_B | GAME_D | GAME_D | GAME_D |
| `*` | GAME_C | 42 (bug) | 0 | 0 |
| `#` | GAME_D | 35 (bug) | 0 | 0 |
| 0 | 0 | 48 (bug) | 0 | 0 |
| Soft keys | 0 | 0 on Nokia; GAME_A and GAME_B on KDDI, LG and Motorola layouts | 0 | 0 |
| Unknown code | 0 | 0 | 0 | 0 |

Sources: [JL KeyMapper static block](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/keyboard/KeyMapper.java#L75),
[FP Mobile.getGameAction](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/Mobile.java#L605),
[ME J2SEInputMethod.getGameAction](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-javase-swing/src/main/java/org/microemu/device/j2se/J2SEInputMethod.java#L105),
[WIE Canvas.get_game_action](https://github.com/dlunch/wie/blob/eb5b809d693ee4f0023f06acd0ad6393006578fc/wie-midp/src/classes/javax/microedition/lcdui/canvas.rs#L157). [code]

Notes:

- freej2me-plus returns the key code itself for 0, `*` and `#`; a game
  that compares the result with `Canvas.FIRE` (8) or `GAME_A` (9) is
  unaffected, but any game that switches on the value can misread 48.
  It is a bug to avoid.
- freej2me-plus maps the soft keys to GAME_A and GAME_B on KDDI, LG and
  Motorola layouts because "KDDI's documentation specifies that this is
  what they should return" and a Motorola 176x220 game expects it
  ([commit 0443389](https://github.com/TASEmulators/freej2me-plus/commit/0443389)). [tracker]
- J2ME Loader used to throw `IllegalArgumentException` for codes with no
  mapping; it now returns 0 ([commit 4866947d](https://github.com/nikita36078/J2ME-Loader/commit/4866947d)).
  Games call `getGameAction` on every key, soft keys included. [code]
- Which digits mean GAME_A to GAME_D on real phones differs by vendor.
  1/3/7/9 is what three of the four emulators use. [unverified on
  hardware]

**For FastEmu:** put the game-action table in the device profile. Default:
arrows and 2/4/6/8/5 to the directions and FIRE, 1/3/7/9 to GAME_A to
GAME_D, everything else 0; never throw. `getKeyCode(action)` returns the
arrow or fire code of the profile, and the digit for GAME_A to GAME_D;
throw `IllegalArgumentException` only for a value that is not a game
action. Keep `getGameAction` constant for the life of the game.

## keyRepeated

- J2ME Loader: `hasRepeatEvents()` is `true`. Hardware keys turn Android
  repeat events into `keyRepeated`. The on-screen keypad repeats after
  400 ms, then after 200, 400, 128, 128, 128, 128, 128 ms, then every
  80 ms ([VirtualKeyboard](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/keyboard/VirtualKeyboard.java#L87)). [code]
- freej2me-plus: `hasRepeatEvents()` is `true`; its gamepad reader
  generates repeats itself and does not send one at the moment of the
  press ([commit b979ca3](https://github.com/TASEmulators/freej2me-plus/commit/b979ca3)). [code]
- freej2me passed the host's key autorepeat to the game as extra
  `keyPressed` calls, which made held keys act as many presses
  ([freej2me issue 130](https://github.com/hex007/freej2me/issues/130)). [tracker]

`fastemu-core` already says apps send one press and one release per
stroke and the runtime makes the repeats. **For FastEmu:** first repeat
after about 400 ms, then every 100 to 130 ms; repeats go to `keyRepeated`,
never to `keyPressed`; no repeats for keys swallowed by the soft key
bar; filter host autorepeat in the apps.

## GameCanvas key states

JSR 118 [spec]: a bit is 1 "if the key is currently down or has been
pressed at least once since the last time this method was called";
reading clears the latched bits; the method "returns 0 unless the
GameCanvas is currently visible"; on becoming visible all keys read as
up, and a key held while the canvas is shown must be released and
pressed again. `suppressKeyEvents` suppresses `keyPressed`,
`keyRepeated` and `keyReleased` "only for the defined game keys";
other keys always get events.

- **J2ME Loader** does exactly this: a latched mask and a current mask,
  both cleared on `showNotify`; suppression only for keys with a game
  action ([GameCanvas](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/game/GameCanvas.java#L110)).
  An old bug suppressed every key because the game-action test was
  wrong ([commit 35c4ed4e](https://github.com/nikita36078/J2ME-Loader/commit/35c4ed4e)). [code]
- **freej2me-plus** keeps one global mask with no latching and clears a
  released key's bit with XOR, which sets the bit again when two keys
  with the same action are released
  ([updateKeyState](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MobilePlatform.java#L383)). [code]
  Rayman Raving Rabbids dropped jump inputs and stopped walking when
  jumping on "freej2me and all other emulators"
  ([issue 11](https://github.com/TASEmulators/freej2me-plus/issues/11)). [tracker]

**For FastEmu:** copy J2ME Loader's two masks; derive the bit from the
profile's game action of the key; clear both masks on `showNotify` and
ignore releases of keys that were down when the canvas appeared.

## Pointer events

- J2ME Loader reports `hasPointerEvents()` and `hasPointerMotionEvents()`
  from a per-game "touch input" setting. It drops drag events that do
  not move in game pixels after scaling
  ([commit b52e7740](https://github.com/nikita36078/J2ME-Loader/commit/b52e7740)).
  Multi-touch follows Nokia's convention: enabled when the
  `Nokia-UI-Enhancement` attribute contains
  `EnableMultiPointTouchEvents`, with the pointer index read from the
  `com.nokia.pointer.number` system property during the callback
  ([Display.getDisplay](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Display.java#L65),
  [MidletSystem.getProperty](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/shell/MidletSystem.java#L23)). [code]
- freej2me-plus always reports pointer events and never motion events. [code]
- Touch-capable games often draw their own on-screen buttons when
  `hasPointerEvents()` is true and hide them otherwise; J2ME Loader has
  an open request to honour `Nokia-MIDlet-On-Screen-Keypad: no`
  ([issue 847](https://github.com/nikita36078/J2ME-Loader/issues/847)). [tracker]

**For FastEmu:** `DeviceProfile::touch` drives both `hasPointer*`
methods. Convert host coordinates to game pixels before deciding whether
a drag moved. Deliver pointer events only inside the Canvas area and
send a release for every press, even when the host pointer leaves the
window.

## Soft keys and Commands on a Canvas

J2ME Loader's soft key bar
([AbstractSoftKeysBar.onCommandsChanged](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/commands/AbstractSoftKeysBar.java#L77),
[fireLeftSoft](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Canvas.java#L1321)) [code]:

1. Commands are sorted by type, then priority, and the first `BACK` or
   `EXIT` command moves to the front of the list.
2. One command: its label is on the left, and both soft keys fire it.
3. Two commands: the front command is on the right soft key, the other
   on the left.
4. More than two: the right soft key fires the front command and the
   left soft key opens a menu of all of them.
5. A soft key press that fires a command or opens the menu is **not**
   passed to the game, and neither is its release.
6. With no commands, soft keys reach `keyPressed` as -6 and -7.
7. In full-screen mode no labels are drawn. With one command, the left
   soft key opens the menu and the right soft key goes to the game; with
   more, both open the menu. Without a `CommandListener` the keys go to
   the game.
8. A non-full-screen Canvas with commands is shorter by the bar's height.

freej2me-plus draws the command bar over the game, never shrinks the
Canvas (except for KDDI), and fades the bar after three seconds
([Canvas.getHeight](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Canvas.java#L403)). [code]

Nokia's `FullCanvas` is always full screen and throws
`IllegalStateException` from `addCommand` and `setCommandListener`
([JL FullCanvas](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/ui/FullCanvas.java#L59)). [code]

**For FastEmu:** follow J2ME Loader's rules 1 to 7. Whether a
non-full-screen Canvas loses height to the bar should be a profile
setting, with "force full screen" as a per-game override, because games
were tuned to their target phone's layout.

## Full-screen mode

- `setFullScreenMode` changes the size, then sends `sizeChanged` and a
  full repaint if the Canvas is visible. [code, JL]
- J2ME Loader creates the `GameCanvas` buffer at the full-screen height
  so a later switch to full screen does not lose pixels. [code]
- `Nokia-MIDlet-Original-Display-Size: 180,320` tells S60 phones to scale
  a game made for a smaller screen ([freej2me-plus issue 9](https://github.com/TASEmulators/freej2me-plus/issues/9)). [tracker]
  FastEmu could read it to choose the profile size for a game.

## Key names

`getKeyName` in J2ME Loader returns `UP`, `DOWN`, `LEFT`, `RIGHT`,
`SELECT`, `SOFT1`, `SOFT2`, `CLEAR`, `SEND`, `END`, otherwise the
character for the code, and throws for codes with neither. freej2me-plus
returns `-` for anything it does not know. [code] Games print key names
in their control menus; return a name for every code the profile can
send.
