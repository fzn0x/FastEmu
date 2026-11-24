# javax.microedition.lcdui (MIDP 2.0)

`javax.microedition.lcdui`: the screen, keys and drawing. Nearly every
game draws everything itself on a `Canvas` (or a `GameCanvas`, see
[midp-lcdui-game.md](midp-lcdui-game.md)) with `Graphics`, `Image` and
`Font`, and uses the high-level screens (`Form`, `List`, `Alert`,
`TextBox`) only for menus, name entry or error messages, if at all.

Rules from the spec that every implementer of this package needs:

- **Event delivery is serial.** `showNotify`, `hideNotify`, the key and
  pointer callbacks, `paint`, `commandAction`, `itemStateChanged` and the
  `run` of `Display.callSerially` are never called while another of them
  is running. `Canvas.serviceRepaints` is the one exception: it runs
  `paint` and waits for it, even from inside a callback.
- **Repaints are asynchronous and coalesced.** `repaint` only marks a
  region; `paint` runs later with the clip covering at least the union of
  the marked regions. A game thread that calls `repaint` and then
  `serviceRepaints` blocks until `paint` returns.
- **The `Graphics` passed to `paint`** has its origin at the top left of
  the canvas, color black, the default font, `SOLID` stroke and a clip
  covering the area to redraw. The implementation does not clear it
  first, and it must not be used after `paint` returns.
- **Key codes.** `KEY_NUM0` to `KEY_NUM9`, `KEY_STAR` and `KEY_POUND` are
  the character codes of those keys on every phone. Every other key has a
  negative, vendor-specific code; `fastemu-core`'s `KeyCodes` holds the
  presets (Nokia: soft keys -6 and -7, arrows -1 to -4, fire -5).
  `getGameAction` maps a key code to `UP`, `DOWN`, `LEFT`, `RIGHT`,
  `FIRE` or `GAME_A` to `GAME_D`, and the mapping never changes while the
  game runs. J2ME Loader's
  [KeyMapper](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/keyboard/KeyMapper.java)
  maps the arrows and fire key, and also `2` `UP`, `4` `LEFT`, `6`
  `RIGHT`, `8` `DOWN`, `5` `FIRE`, `7` `GAME_A`, `9` `GAME_B`, `*`
  `GAME_C` and `#` `GAME_D`; `getKeyCode` returns the arrow and fire codes
  for the directions and `7`, `9`, `*`, `#` for `GAME_A` to `GAME_D`.
  FastEmu should map the same way.
- **Soft keys.** While a `Canvas` is not in full-screen mode the phone
  shows its `Command`s on the soft keys and does not deliver those keys
  to `keyPressed`. Many games call `setFullScreenMode(true)`, add no
  commands, and read the soft keys as raw key codes (-6 and -7 on Nokia),
  so FastEmu must deliver them in that case.
- **Colors** are `0x00RRGGBB` everywhere except the ARGB methods
  (`drawRGB`, `createRGBImage`, `getRGB`), which use `0xAARRGGBB`.

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
| [Alert](#alert) | class | 2 | MIDP 1.0 | 18 |
| [AlertType](#alerttype) | class | 2 | MIDP 1.0 | 7 |
| [Canvas](#canvas) | class | 1 | MIDP 1.0 | 43 |
| [Choice](#choice) | interface | 2 | MIDP 1.0 | 24 |
| [ChoiceGroup](#choicegroup) | class | 3 | MIDP 1.0 | 19 |
| [Command](#command) | class | 1 | MIDP 1.0 | 14 |
| [CommandListener](#commandlistener) | interface | 1 | MIDP 1.0 | 1 |
| [CustomItem](#customitem) | class | 3 | MIDP 2.0 | 31 |
| [DateField](#datefield) | class | 3 | MIDP 1.0 | 9 |
| [Display](#display) | class | 1 | MIDP 1.0 | 24 |
| [Displayable](#displayable) | class | 1 | MIDP 1.0 | 11 |
| [Font](#font) | class | 1 | MIDP 1.0 | 28 |
| [Form](#form) | class | 2 | MIDP 1.0 | 14 |
| [Gauge](#gauge) | class | 3 | MIDP 1.0 | 17 |
| [Graphics](#graphics) | class | 1 | MIDP 1.0 | 47 |
| [Image](#image) | class | 1 | MIDP 1.0 | 12 |
| [ImageItem](#imageitem) | class | 3 | MIDP 1.0 | 15 |
| [Item](#item) | class | 2 | MIDP 1.0 | 31 |
| [ItemCommandListener](#itemcommandlistener) | interface | 3 | MIDP 2.0 | 1 |
| [ItemStateListener](#itemstatelistener) | interface | 3 | MIDP 1.0 | 1 |
| [List](#list) | class | 2 | MIDP 1.0 | 24 |
| [Screen](#screen) | class | 2 | MIDP 1.0 | 0 |
| [Spacer](#spacer) | class | 3 | MIDP 2.0 | 5 |
| [StringItem](#stringitem) | class | 2 | MIDP 1.0 | 8 |
| [TextBox](#textbox) | class | 2 | MIDP 1.0 | 17 |
| [TextField](#textfield) | class | 3 | MIDP 1.0 | 28 |
| [Ticker](#ticker) | class | 3 | MIDP 1.0 | 3 |

## javax.microedition.lcdui

### Alert

`public class javax.microedition.lcdui.Alert extends javax.microedition.lcdui.Screen`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Alert.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Alert.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Alert.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Alert.java).

Games show one for errors and "Game over" messages, often before any `Canvas`.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `FOREVER` | `I` | `-2` (`0xfffffffe`) | static | 1.0 | `setTimeout(FOREVER)` keeps the alert until the user dismisses it. |
| 3 | `DISMISS_COMMAND` | `Ljavax/microedition/lcdui/Command;` | (not a constant) | static | 2.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(String title)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 2 | `(String title, String alertText, Image alertImage, AlertType alertType)` | `<init>(Ljava/lang/String;Ljava/lang/String;Ljavax/microedition/lcdui/Image;Ljavax/microedition/lcdui/AlertType;)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `int getDefaultTimeout()` | `getDefaultTimeout()I` |  | 1.0 |  |  |
| 2 | `int getTimeout()` | `getTimeout()I` |  | 1.0 |  |  |
| 2 | `void setTimeout(int time)` | `setTimeout(I)V` |  | 1.0 | `IllegalArgumentException` if time is not positive and is not FOREVER |  |
| 3 | `AlertType getType()` | `getType()Ljavax/microedition/lcdui/AlertType;` |  | 1.0 |  |  |
| 3 | `void setType(AlertType type)` | `setType(Ljavax/microedition/lcdui/AlertType;)V` |  | 1.0 |  |  |
| 2 | `String getString()` | `getString()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `void setString(String str)` | `setString(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 3 | `Image getImage()` | `getImage()Ljavax/microedition/lcdui/Image;` |  | 1.0 |  |  |
| 3 | `void setImage(Image img)` | `setImage(Ljavax/microedition/lcdui/Image;)V` |  | 1.0 |  |  |
| 3 | `void setIndicator(Gauge indicator)` | `setIndicator(Ljavax/microedition/lcdui/Gauge;)V` |  | 2.0 | `IllegalArgumentException` if indicator does not meet the restrictions for its use in an Alert |  |
| 3 | `Gauge getIndicator()` | `getIndicator()Ljavax/microedition/lcdui/Gauge;` |  | 2.0 |  |  |
| 2 | `void addCommand(Command cmd)` | `addCommand(Ljavax/microedition/lcdui/Command;)V` |  | 1.0 | `NullPointerException` if cmd is null |  |
| 3 | `void removeCommand(Command cmd)` | `removeCommand(Ljavax/microedition/lcdui/Command;)V` |  | 1.0 |  |  |
| 2 | `void setCommandListener(CommandListener l)` | `setCommandListener(Ljavax/microedition/lcdui/CommandListener;)V` |  | 1.0 |  |  |

### AlertType

`public class javax.microedition.lcdui.AlertType extends java.lang.Object`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/AlertType.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/AlertType.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/AlertType.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/AlertType.java).

The five constants are instances, not ints. `playSound` plays the type's system sound and returns whether it did.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `INFO` | `Ljavax/microedition/lcdui/AlertType;` | (not a constant) | static | 1.0 |  |
| 2 | `WARNING` | `Ljavax/microedition/lcdui/AlertType;` | (not a constant) | static | 1.0 |  |
| 2 | `ERROR` | `Ljavax/microedition/lcdui/AlertType;` | (not a constant) | static | 1.0 |  |
| 2 | `ALARM` | `Ljavax/microedition/lcdui/AlertType;` | (not a constant) | static | 1.0 |  |
| 2 | `CONFIRMATION` | `Ljavax/microedition/lcdui/AlertType;` | (not a constant) | static | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` | protected | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `boolean playSound(Display display)` | `playSound(Ljavax/microedition/lcdui/Display;)Z` |  | 1.0 | `NullPointerException` if display is null | Some games call `AlertType.ALARM.playSound(display)` as a cheap beep. |

### Canvas

`public abstract class javax.microedition.lcdui.Canvas extends javax.microedition.lcdui.Displayable`

Tier 1. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Canvas.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Canvas.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Canvas.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Canvas.java).

The base of every game screen. A game subclasses it, implements `paint`, and overrides the key and pointer callbacks it needs.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 1 | `UP` | `I` | `1` | static | 1.0 |  |
| 1 | `DOWN` | `I` | `6` | static | 1.0 |  |
| 1 | `LEFT` | `I` | `2` | static | 1.0 |  |
| 1 | `RIGHT` | `I` | `5` | static | 1.0 |  |
| 1 | `FIRE` | `I` | `8` | static | 1.0 |  |
| 2 | `GAME_A` | `I` | `9` | static | 1.0 |  |
| 2 | `GAME_B` | `I` | `10` | static | 1.0 |  |
| 2 | `GAME_C` | `I` | `11` | static | 1.0 |  |
| 2 | `GAME_D` | `I` | `12` | static | 1.0 |  |
| 1 | `KEY_NUM0` | `I` | `48` | static | 1.0 |  |
| 1 | `KEY_NUM1` | `I` | `49` | static | 1.0 |  |
| 1 | `KEY_NUM2` | `I` | `50` | static | 1.0 |  |
| 1 | `KEY_NUM3` | `I` | `51` | static | 1.0 |  |
| 1 | `KEY_NUM4` | `I` | `52` | static | 1.0 |  |
| 1 | `KEY_NUM5` | `I` | `53` | static | 1.0 |  |
| 1 | `KEY_NUM6` | `I` | `54` | static | 1.0 |  |
| 1 | `KEY_NUM7` | `I` | `55` | static | 1.0 |  |
| 1 | `KEY_NUM8` | `I` | `56` | static | 1.0 |  |
| 1 | `KEY_NUM9` | `I` | `57` | static | 1.0 |  |
| 1 | `KEY_STAR` | `I` | `42` | static | 1.0 |  |
| 1 | `KEY_POUND` | `I` | `35` | static | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` | protected | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `boolean isDoubleBuffered()` | `isDoubleBuffered()Z` |  | 1.0 |  | FastEmu should answer `true`. Games that see `false` draw into their own off-screen `Image` first. |
| 2 | `boolean hasPointerEvents()` | `hasPointerEvents()Z` |  | 1.0 |  | From the device profile (`DeviceProfile.touch`). Touch-aware games switch their controls on it. |
| 3 | `boolean hasPointerMotionEvents()` | `hasPointerMotionEvents()Z` |  | 1.0 |  |  |
| 2 | `boolean hasRepeatEvents()` | `hasRepeatEvents()Z` |  | 1.0 |  | `true` when `keyRepeated` is delivered for held keys. |
| 2 | `int getKeyCode(int gameAction)` | `getKeyCode(I)I` |  | 1.0 | `IllegalArgumentException` if gameAction is not a valid game action |  |
| 3 | `String getKeyName(int keyCode)` | `getKeyName(I)Ljava/lang/String;` |  | 1.0 | `IllegalArgumentException` if keyCode is not a valid key code | A label such as `"SELECT"` or `"5"`; non-empty for every valid code. |
| 1 | `int getGameAction(int keyCode)` | `getGameAction(I)I` |  | 1.0 | `IllegalArgumentException` if keyCode is not a valid key code | Throws `IllegalArgumentException` for codes that are not valid keys; games call it with every code they get, so every code FastEmu sends must be valid. Returns 0 for keys with no action. |
| 1 | `void setFullScreenMode(boolean mode)` | `setFullScreenMode(Z)V` |  | 2.0 |  | Hides the title, ticker and command bar and gives the canvas the whole screen; `sizeChanged` follows when the size changes. |
| 1 | `void keyPressed(int keyCode)` | `keyPressed(I)V` | protected | 1.0 |  | Empty in `Canvas`; games override it. |
| 2 | `void keyRepeated(int keyCode)` | `keyRepeated(I)V` | protected | 1.0 |  | Called while a key is held, if `hasRepeatEvents` is `true`. |
| 1 | `void keyReleased(int keyCode)` | `keyReleased(I)V` | protected | 1.0 |  |  |
| 2 | `void pointerPressed(int x, int y)` | `pointerPressed(II)V` | protected | 1.0 |  | Coordinates are relative to the canvas. |
| 2 | `void pointerReleased(int x, int y)` | `pointerReleased(II)V` | protected | 1.0 |  |  |
| 2 | `void pointerDragged(int x, int y)` | `pointerDragged(II)V` | protected | 1.0 |  |  |
| 1 | `void repaint(int x, int y, int width, int height)` | `repaint(IIII)V` | final | 1.0 |  | Does nothing for an empty rectangle or a hidden canvas. |
| 1 | `void repaint()` | `repaint()V` | final | 1.0 |  | Repaints the whole canvas. |
| 1 | `void serviceRepaints()` | `serviceRepaints()V` | final | 1.0 |  | Blocks until `paint` returns, or returns at once when nothing is pending or the canvas is hidden. Game loops call `repaint()` then `serviceRepaints()` every frame, so this pair sets the frame rate. |
| 1 | `void showNotify()` | `showNotify()V` | protected | 1.0 |  | Called before the canvas becomes visible; a full repaint follows. |
| 1 | `void hideNotify()` | `hideNotify()V` | protected | 1.0 |  | Called after the canvas leaves the screen. Games pause here. |
| 1 | `void paint(Graphics g)` | `paint(Ljavax/microedition/lcdui/Graphics;)V` | protected abstract | 1.0 |  | Called only by the implementation. |
| 2 | `void sizeChanged(int w, int h)` | `sizeChanged(II)V` | protected | 2.0 |  | Inherited from `Displayable` and overridden here; called at least once before the canvas is first shown. |

### Choice

`public interface javax.microedition.lcdui.Choice`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Choice.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Choice.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Choice.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Choice.java).

Interface shared by `List` and `ChoiceGroup`.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `EXCLUSIVE` | `I` | `1` | static | 1.0 |  |
| 2 | `MULTIPLE` | `I` | `2` | static | 1.0 |  |
| 2 | `IMPLICIT` | `I` | `3` | static | 1.0 |  |
| 3 | `POPUP` | `I` | `4` | static | 2.0 |  |
| 3 | `TEXT_WRAP_DEFAULT` | `I` | `0` | static | 2.0 |  |
| 3 | `TEXT_WRAP_ON` | `I` | `1` | static | 2.0 |  |
| 3 | `TEXT_WRAP_OFF` | `I` | `2` | static | 2.0 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `int size()` | `size()I` |  | 1.0 |  |  |
| 2 | `String getString(int elementNum)` | `getString(I)Ljava/lang/String;` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 2 | `Image getImage(int elementNum)` | `getImage(I)Ljavax/microedition/lcdui/Image;` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 2 | `int append(String stringPart, Image imagePart)` | `append(Ljava/lang/String;Ljavax/microedition/lcdui/Image;)I` |  | 1.0 | `NullPointerException` if stringPart is null |  |
| 2 | `void insert(int elementNum, String stringPart, Image imagePart)` | `insert(ILjava/lang/String;Ljavax/microedition/lcdui/Image;)V` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid; `NullPointerException` if stringPart is null |  |
| 2 | `void delete(int elementNum)` | `delete(I)V` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 2 | `void deleteAll()` | `deleteAll()V` |  | 2.0 |  |  |
| 2 | `void set(int elementNum, String stringPart, Image imagePart)` | `set(ILjava/lang/String;Ljavax/microedition/lcdui/Image;)V` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid; `NullPointerException` if stringPart is null |  |
| 2 | `boolean isSelected(int elementNum)` | `isSelected(I)Z` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 2 | `int getSelectedIndex()` | `getSelectedIndex()I` |  | 1.0 |  |  |
| 3 | `int getSelectedFlags(boolean[] selectedArray_return)` | `getSelectedFlags([Z)I` |  | 1.0 | `IllegalArgumentException` if selectedArray_return is shorter than the size of the Choice; `NullPointerException` if selectedArray_return is null |  |
| 2 | `void setSelectedIndex(int elementNum, boolean selected)` | `setSelectedIndex(IZ)V` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 3 | `void setSelectedFlags(boolean[] selectedArray)` | `setSelectedFlags([Z)V` |  | 1.0 | `IllegalArgumentException` if selectedArray is shorter than the size of the Choice; `NullPointerException` if selectedArray is null |  |
| 3 | `void setFitPolicy(int fitPolicy)` | `setFitPolicy(I)V` |  | 2.0 | `IllegalArgumentException` if fitPolicy is invalid |  |
| 3 | `int getFitPolicy()` | `getFitPolicy()I` |  | 2.0 |  |  |
| 3 | `void setFont(int elementNum, Font font)` | `setFont(ILjavax/microedition/lcdui/Font;)V` |  | 2.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 3 | `Font getFont(int elementNum)` | `getFont(I)Ljavax/microedition/lcdui/Font;` |  | 2.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |

### ChoiceGroup

`public class javax.microedition.lcdui.ChoiceGroup extends javax.microedition.lcdui.Item implements javax.microedition.lcdui.Choice`

Tier 3. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/ChoiceGroup.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/ChoiceGroup.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/ChoiceGroup.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/ChoiceGroup.java).

A `Choice` inside a `Form`, used by option screens.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(String label, int choiceType)` | `<init>(Ljava/lang/String;I)V` |  | 1.0 | `IllegalArgumentException` if choiceType is not one of EXCLUSIVE, MULTIPLE, or POPUP |  |
| 3 | `(String label, int choiceType, String[] stringElements, Image[] imageElements)` | `<init>(Ljava/lang/String;I[Ljava/lang/String;[Ljavax/microedition/lcdui/Image;)V` |  | 1.0 | `NullPointerException` if stringElements is null; `NullPointerException` if the stringElements array contains any null elements; `IllegalArgumentException` if the imageElements array is non-null and has a different length from the stringElements array; `IllegalArgumentException` if choiceType is not one of EXCLUSIVE, MULTIPLE, or POPUP |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `int size()` | `size()I` |  | 1.0 |  |  |
| 3 | `String getString(int elementNum)` | `getString(I)Ljava/lang/String;` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 3 | `Image getImage(int elementNum)` | `getImage(I)Ljavax/microedition/lcdui/Image;` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 3 | `int append(String stringPart, Image imagePart)` | `append(Ljava/lang/String;Ljavax/microedition/lcdui/Image;)I` |  | 1.0 | `NullPointerException` if stringPart is null |  |
| 3 | `void insert(int elementNum, String stringPart, Image imagePart)` | `insert(ILjava/lang/String;Ljavax/microedition/lcdui/Image;)V` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid; `NullPointerException` if stringPart is null |  |
| 3 | `void delete(int elementNum)` | `delete(I)V` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 3 | `void deleteAll()` | `deleteAll()V` |  | 1.0 |  |  |
| 3 | `void set(int elementNum, String stringPart, Image imagePart)` | `set(ILjava/lang/String;Ljavax/microedition/lcdui/Image;)V` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid; `NullPointerException` if stringPart is null |  |
| 3 | `boolean isSelected(int elementNum)` | `isSelected(I)Z` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 3 | `int getSelectedIndex()` | `getSelectedIndex()I` |  | 1.0 |  |  |
| 3 | `int getSelectedFlags(boolean[] selectedArray_return)` | `getSelectedFlags([Z)I` |  | 1.0 | `IllegalArgumentException` if selectedArray_return is shorter than the size of the ChoiceGroup; `NullPointerException` if selectedArray_return is null |  |
| 3 | `void setSelectedIndex(int elementNum, boolean selected)` | `setSelectedIndex(IZ)V` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 3 | `void setSelectedFlags(boolean[] selectedArray)` | `setSelectedFlags([Z)V` |  | 1.0 | `IllegalArgumentException` if selectedArray is shorter than the size of the ChoiceGroup; `NullPointerException` if the selectedArray is null |  |
| 3 | `void setFitPolicy(int fitPolicy)` | `setFitPolicy(I)V` |  | 2.0 | `IllegalArgumentException` if fitPolicy is invalid |  |
| 3 | `int getFitPolicy()` | `getFitPolicy()I` |  | 2.0 |  |  |
| 3 | `void setFont(int elementNum, Font font)` | `setFont(ILjavax/microedition/lcdui/Font;)V` |  | 2.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 3 | `Font getFont(int elementNum)` | `getFont(I)Ljavax/microedition/lcdui/Font;` |  | 2.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |

### Command

`public class javax.microedition.lcdui.Command extends java.lang.Object`

Tier 1. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Command.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Command.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Command.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Command.java).

Soft-key actions. Games add `Command`s to a `Canvas` for "Menu" and "Back", or to their screens.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `SCREEN` | `I` | `1` | static | 1.0 |  |
| 2 | `BACK` | `I` | `2` | static | 1.0 |  |
| 2 | `CANCEL` | `I` | `3` | static | 1.0 |  |
| 2 | `OK` | `I` | `4` | static | 1.0 |  |
| 3 | `HELP` | `I` | `5` | static | 1.0 |  |
| 3 | `STOP` | `I` | `6` | static | 1.0 |  |
| 2 | `EXIT` | `I` | `7` | static | 1.0 |  |
| 3 | `ITEM` | `I` | `8` | static | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `(String label, int commandType, int priority)` | `<init>(Ljava/lang/String;II)V` |  | 1.0 | `NullPointerException` if label is null; `IllegalArgumentException` if the commandType is an invalid type | Label, type (`SCREEN`, `BACK`, `OK`, ...) and priority (lower is more important). The phone picks soft keys by type and priority. |
| 3 | `(String shortLabel, String longLabel, int commandType, int priority)` | `<init>(Ljava/lang/String;Ljava/lang/String;II)V` |  | 2.0 | `NullPointerException` if shortLabel is null; `IllegalArgumentException` if the commandType is an invalid type |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `String getLabel()` | `getLabel()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `String getLongLabel()` | `getLongLabel()Ljava/lang/String;` |  | 2.0 |  |  |
| 2 | `int getCommandType()` | `getCommandType()I` |  | 1.0 |  |  |
| 3 | `int getPriority()` | `getPriority()I` |  | 1.0 |  |  |

### CommandListener

`public interface javax.microedition.lcdui.CommandListener`

Tier 1. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/CommandListener.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/CommandListener.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/CommandListener.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/CommandListener.java).

`commandAction` runs on the event thread, serialized with the other callbacks.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `void commandAction(Command c, Displayable d)` | `commandAction(Ljavax/microedition/lcdui/Command;Ljavax/microedition/lcdui/Displayable;)V` |  | 1.0 |  |  |

### CustomItem

`public abstract class javax.microedition.lcdui.CustomItem extends javax.microedition.lcdui.Item`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/CustomItem.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/CustomItem.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/CustomItem.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/CustomItem.java).

A game-drawn `Item` inside a `Form`; rare in games.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `TRAVERSE_HORIZONTAL` | `I` | `1` | protected static | 2.0 |  |
| 3 | `TRAVERSE_VERTICAL` | `I` | `2` | protected static | 2.0 |  |
| 3 | `KEY_PRESS` | `I` | `4` | protected static | 2.0 |  |
| 3 | `KEY_RELEASE` | `I` | `8` | protected static | 2.0 |  |
| 3 | `KEY_REPEAT` | `I` | `16` | protected static | 2.0 |  |
| 3 | `POINTER_PRESS` | `I` | `32` | protected static | 2.0 |  |
| 3 | `POINTER_RELEASE` | `I` | `64` | protected static | 2.0 |  |
| 3 | `POINTER_DRAG` | `I` | `128` | protected static | 2.0 |  |
| 3 | `NONE` | `I` | `0` | protected static | 2.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(String label)` | `<init>(Ljava/lang/String;)V` | protected | 2.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `int getGameAction(int keyCode)` | `getGameAction(I)I` |  | 2.0 | `IllegalArgumentException` if keyCode is not a valid key code |  |
| 3 | `int getInteractionModes()` | `getInteractionModes()I` | protected final | 2.0 |  |  |
| 3 | `int getMinContentWidth()` | `getMinContentWidth()I` | protected abstract | 2.0 |  |  |
| 3 | `int getMinContentHeight()` | `getMinContentHeight()I` | protected abstract | 2.0 |  |  |
| 3 | `int getPrefContentWidth(int height)` | `getPrefContentWidth(I)I` | protected abstract | 2.0 |  |  |
| 3 | `int getPrefContentHeight(int width)` | `getPrefContentHeight(I)I` | protected abstract | 2.0 |  |  |
| 3 | `void sizeChanged(int w, int h)` | `sizeChanged(II)V` | protected | 2.0 |  |  |
| 3 | `void invalidate()` | `invalidate()V` | protected final | 2.0 |  |  |
| 3 | `void paint(Graphics g, int w, int h)` | `paint(Ljavax/microedition/lcdui/Graphics;II)V` | protected abstract | 2.0 |  |  |
| 3 | `void repaint()` | `repaint()V` | protected final | 2.0 |  |  |
| 3 | `void repaint(int x, int y, int w, int h)` | `repaint(IIII)V` | protected final | 2.0 |  |  |
| 3 | `boolean traverse(int dir, int viewportWidth, int viewportHeight, int[] visRect_inout)` | `traverse(III[I)Z` | protected | 2.0 |  |  |
| 3 | `void traverseOut()` | `traverseOut()V` | protected | 2.0 |  |  |
| 3 | `void keyPressed(int keyCode)` | `keyPressed(I)V` | protected | 2.0 |  |  |
| 3 | `void keyReleased(int keyCode)` | `keyReleased(I)V` | protected | 2.0 |  |  |
| 3 | `void keyRepeated(int keyCode)` | `keyRepeated(I)V` | protected | 2.0 |  |  |
| 3 | `void pointerPressed(int x, int y)` | `pointerPressed(II)V` | protected | 2.0 |  |  |
| 3 | `void pointerReleased(int x, int y)` | `pointerReleased(II)V` | protected | 2.0 |  |  |
| 3 | `void pointerDragged(int x, int y)` | `pointerDragged(II)V` | protected | 2.0 |  |  |
| 3 | `void showNotify()` | `showNotify()V` | protected | 2.0 |  |  |
| 3 | `void hideNotify()` | `hideNotify()V` | protected | 2.0 |  |  |

### DateField

`public class javax.microedition.lcdui.DateField extends javax.microedition.lcdui.Item`

Tier 3. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/DateField.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/DateField.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/DateField.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/DateField.java).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `DATE` | `I` | `1` | static | 1.0 |  |
| 3 | `TIME` | `I` | `2` | static | 1.0 |  |
| 3 | `DATE_TIME` | `I` | `3` | static | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(String label, int mode)` | `<init>(Ljava/lang/String;I)V` |  | 1.0 | `IllegalArgumentException` if the input mode's value is invalid |  |
| 3 | `(String label, int mode, TimeZone timeZone)` | `<init>(Ljava/lang/String;ILjava/util/TimeZone;)V` |  | 1.0 | `IllegalArgumentException` if the input mode's value is invalid |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `Date getDate()` | `getDate()Ljava/util/Date;` |  | 1.0 |  |  |
| 3 | `void setDate(Date date)` | `setDate(Ljava/util/Date;)V` |  | 1.0 |  |  |
| 3 | `int getInputMode()` | `getInputMode()I` |  | 1.0 |  |  |
| 3 | `void setInputMode(int mode)` | `setInputMode(I)V` |  | 1.0 | `IllegalArgumentException` if an invalid value is specified |  |

### Display

`public class javax.microedition.lcdui.Display extends java.lang.Object`

Tier 1. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Display.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Display.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Display.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Display.java).

One per MIDlet.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `LIST_ELEMENT` | `I` | `1` | static | 2.0 |  |
| 3 | `CHOICE_GROUP_ELEMENT` | `I` | `2` | static | 2.0 |  |
| 3 | `ALERT` | `I` | `3` | static | 2.0 |  |
| 3 | `COLOR_BACKGROUND` | `I` | `0` | static | 2.0 |  |
| 3 | `COLOR_FOREGROUND` | `I` | `1` | static | 2.0 |  |
| 3 | `COLOR_HIGHLIGHTED_BACKGROUND` | `I` | `2` | static | 2.0 |  |
| 3 | `COLOR_HIGHLIGHTED_FOREGROUND` | `I` | `3` | static | 2.0 |  |
| 3 | `COLOR_BORDER` | `I` | `4` | static | 2.0 |  |
| 3 | `COLOR_HIGHLIGHTED_BORDER` | `I` | `5` | static | 2.0 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `Display getDisplay(MIDlet m)` | `getDisplay(Ljavax/microedition/midlet/MIDlet;)Ljavax/microedition/lcdui/Display;` | static | 1.0 | `NullPointerException` if m is null | Always returns the same object for a MIDlet. |
| 3 | `int getColor(int colorSpecifier)` | `getColor(I)I` |  | 2.0 | `IllegalArgumentException` if colorSpecifier is not a valid color specifier |  |
| 3 | `int getBorderStyle(boolean highlighted)` | `getBorderStyle(Z)I` |  | 2.0 |  |  |
| 2 | `boolean isColor()` | `isColor()Z` |  | 1.0 |  |  |
| 2 | `int numColors()` | `numColors()I` |  | 1.0 |  | From the device profile (`DeviceProfile.colors`). |
| 2 | `int numAlphaLevels()` | `numAlphaLevels()I` |  | 2.0 |  | At least 2; more means alpha blending works. From `DeviceProfile.alpha_levels`. |
| 1 | `Displayable getCurrent()` | `getCurrent()Ljavax/microedition/lcdui/Displayable;` |  | 1.0 |  | May not yet return what `setCurrent` was just given, since the switch is asynchronous. |
| 1 | `void setCurrent(Displayable nextDisplayable)` | `setCurrent(Ljavax/microedition/lcdui/Displayable;)V` |  | 1.0 |  | Asynchronous: the switch happens later, between event callbacks, and a newer call replaces a pending one. `null` keeps the current screen (and may mean "send me to the background"). |
| 2 | `void setCurrent(Alert alert, Displayable nextDisplayable)` | `setCurrent(Ljavax/microedition/lcdui/Alert;Ljavax/microedition/lcdui/Displayable;)V` |  | 1.0 | `NullPointerException` if alert or nextDisplayable is null; `IllegalArgumentException` if nextDisplayable is an Alert | Shows the alert, then the given screen when it is dismissed. |
| 3 | `void setCurrentItem(Item item)` | `setCurrentItem(Ljavax/microedition/lcdui/Item;)V` |  | 2.0 | `IllegalStateException` if the item is not owned by a container; `IllegalStateException` if the item is owned by an Alert; `NullPointerException` if item is null |  |
| 1 | `void callSerially(Runnable r)` | `callSerially(Ljava/lang/Runnable;)V` |  | 1.0 |  | Queues `r.run()` on the event thread after any pending repaint has been painted. Never blocks. Some game loops are built on it: `run` draws a frame and calls `callSerially(this)` again. |
| 2 | `boolean flashBacklight(int duration)` | `flashBacklight(I)Z` |  | 2.0 | `IllegalArgumentException` if duration is negative | Same contract as `vibrate`. |
| 2 | `boolean vibrate(int duration)` | `vibrate(I)Z` |  | 2.0 | `IllegalArgumentException` if duration is negative | Returns `false` when there is no vibrator or the MIDlet is in the background; a new call replaces the running duration. Throws `IllegalArgumentException` for a negative duration. |
| 3 | `int getBestImageWidth(int imageType)` | `getBestImageWidth(I)I` |  | 2.0 | `IllegalArgumentException` if imageType is illegal |  |
| 3 | `int getBestImageHeight(int imageType)` | `getBestImageHeight(I)I` |  | 2.0 | `IllegalArgumentException` if imageType is illegal |  |

### Displayable

`public abstract class javax.microedition.lcdui.Displayable extends java.lang.Object`

Tier 1. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Displayable.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Displayable.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Displayable.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Displayable.java).

Base of `Canvas` and `Screen`. Has no public constructor, so games cannot subclass it directly.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `String getTitle()` | `getTitle()Ljava/lang/String;` |  | 2.0 |  |  |
| 2 | `void setTitle(String s)` | `setTitle(Ljava/lang/String;)V` |  | 2.0 |  |  |
| 3 | `Ticker getTicker()` | `getTicker()Ljavax/microedition/lcdui/Ticker;` |  | 2.0 |  |  |
| 3 | `void setTicker(Ticker ticker)` | `setTicker(Ljavax/microedition/lcdui/Ticker;)V` |  | 2.0 |  |  |
| 2 | `boolean isShown()` | `isShown()Z` |  | 1.0 |  |  |
| 1 | `void addCommand(Command cmd)` | `addCommand(Ljavax/microedition/lcdui/Command;)V` |  | 1.0 | `NullPointerException` if cmd is null |  |
| 2 | `void removeCommand(Command cmd)` | `removeCommand(Ljavax/microedition/lcdui/Command;)V` |  | 1.0 |  |  |
| 1 | `void setCommandListener(CommandListener l)` | `setCommandListener(Ljavax/microedition/lcdui/CommandListener;)V` |  | 1.0 |  |  |
| 1 | `int getWidth()` | `getWidth()I` |  | 2.0 |  | MIDP 2.0 declares it here; MIDP 1.0 declared `getWidth` and `getHeight` on `Canvas`. Old class files that call `Canvas.getWidth()I` still resolve, through the superclass. The value depends on full-screen mode, title, ticker and commands. |
| 1 | `int getHeight()` | `getHeight()I` |  | 2.0 |  | See `getWidth`. |
| 2 | `void sizeChanged(int w, int h)` | `sizeChanged(II)V` | protected | 2.0 |  |  |

### Font

`public final class javax.microedition.lcdui.Font extends java.lang.Object`

Tier 1. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Font.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Font.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Font.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Font.java).

Three faces, four styles and three sizes. Pixel sizes come from the device profile (`DeviceProfile.font_sizes`); games lay out text with `getHeight` and `stringWidth`, so the metrics must match what FastEmu draws.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 1 | `STYLE_PLAIN` | `I` | `0` | static | 1.0 |  |
| 1 | `STYLE_BOLD` | `I` | `1` | static | 1.0 |  |
| 1 | `STYLE_ITALIC` | `I` | `2` | static | 1.0 |  |
| 2 | `STYLE_UNDERLINED` | `I` | `4` | static | 1.0 |  |
| 1 | `SIZE_SMALL` | `I` | `8` | static | 1.0 |  |
| 1 | `SIZE_MEDIUM` | `I` | `0` | static | 1.0 |  |
| 1 | `SIZE_LARGE` | `I` | `16` | static | 1.0 |  |
| 1 | `FACE_SYSTEM` | `I` | `0` | static | 1.0 |  |
| 2 | `FACE_MONOSPACE` | `I` | `32` | static | 1.0 |  |
| 2 | `FACE_PROPORTIONAL` | `I` | `64` | static | 1.0 |  |
| 3 | `FONT_STATIC_TEXT` | `I` | `0` | static | 2.0 |  |
| 3 | `FONT_INPUT_TEXT` | `I` | `1` | static | 2.0 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `Font getFont(int fontSpecifier)` | `getFont(I)Ljavax/microedition/lcdui/Font;` | static | 2.0 | `IllegalArgumentException` if fontSpecifier is not a valid fontSpecifier | `FONT_STATIC_TEXT` or `FONT_INPUT_TEXT`. |
| 1 | `Font getDefaultFont()` | `getDefaultFont()Ljavax/microedition/lcdui/Font;` | static | 1.0 |  |  |
| 1 | `Font getFont(int face, int style, int size)` | `getFont(III)Ljavax/microedition/lcdui/Font;` | static | 1.0 | `IllegalArgumentException` if face, style, or size are not legal values | Never returns null: the nearest font if no exact match. Throws `IllegalArgumentException` for values that are not face, style or size constants. |
| 3 | `int getStyle()` | `getStyle()I` |  | 1.0 |  |  |
| 3 | `int getSize()` | `getSize()I` |  | 1.0 |  |  |
| 3 | `int getFace()` | `getFace()I` |  | 1.0 |  |  |
| 3 | `boolean isPlain()` | `isPlain()Z` |  | 1.0 |  |  |
| 3 | `boolean isBold()` | `isBold()Z` |  | 1.0 |  |  |
| 3 | `boolean isItalic()` | `isItalic()Z` |  | 1.0 |  |  |
| 3 | `boolean isUnderlined()` | `isUnderlined()Z` |  | 1.0 |  |  |
| 1 | `int getHeight()` | `getHeight()I` |  | 1.0 |  | Line height including the leading below the text. |
| 2 | `int getBaselinePosition()` | `getBaselinePosition()I` |  | 1.0 |  | Distance from the top of the text to its baseline; `drawString` with `BASELINE` anchors here. |
| 1 | `int charWidth(char ch)` | `charWidth(C)I` |  | 1.0 |  |  |
| 2 | `int charsWidth(char[] ch, int offset, int length)` | `charsWidth([CII)I` |  | 1.0 | `ArrayIndexOutOfBoundsException` if offset and length specify an invalid range; `NullPointerException` if ch is null |  |
| 1 | `int stringWidth(String str)` | `stringWidth(Ljava/lang/String;)I` |  | 1.0 | `NullPointerException` if str is null |  |
| 2 | `int substringWidth(String str, int offset, int len)` | `substringWidth(Ljava/lang/String;II)I` |  | 1.0 | `StringIndexOutOfBoundsException` if offset and length specify an invalid range; `NullPointerException` if str is null |  |

### Form

`public class javax.microedition.lcdui.Form extends javax.microedition.lcdui.Screen`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Form.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Form.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Form.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Form.java).

Used for option, help and score screens.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(String title)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 3 | `(String title, Item[] items)` | `<init>(Ljava/lang/String;[Ljavax/microedition/lcdui/Item;)V` |  | 1.0 | `IllegalStateException` if one of the items is already owned by another container; `NullPointerException` if an element of the items array is null |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `int append(Item item)` | `append(Ljavax/microedition/lcdui/Item;)I` |  | 1.0 | `IllegalStateException` if the item is already owned by a container; `NullPointerException` if item is null |  |
| 2 | `int append(String str)` | `append(Ljava/lang/String;)I` |  | 1.0 | `NullPointerException` if str is null |  |
| 3 | `int append(Image img)` | `append(Ljavax/microedition/lcdui/Image;)I` |  | 1.0 | `NullPointerException` if img is null |  |
| 3 | `void insert(int itemNum, Item item)` | `insert(ILjavax/microedition/lcdui/Item;)V` |  | 1.0 | `IndexOutOfBoundsException` if itemNum is invalid; `IllegalStateException` if the item is already owned by a container; `NullPointerException` if item is null |  |
| 2 | `void delete(int itemNum)` | `delete(I)V` |  | 1.0 | `IndexOutOfBoundsException` if itemNum is invalid |  |
| 2 | `void deleteAll()` | `deleteAll()V` |  | 2.0 |  |  |
| 3 | `void set(int itemNum, Item item)` | `set(ILjavax/microedition/lcdui/Item;)V` |  | 1.0 | `IndexOutOfBoundsException` if itemNum is invalid; `IllegalStateException` if the item is already owned by a container; `NullPointerException` if item is null |  |
| 2 | `Item get(int itemNum)` | `get(I)Ljavax/microedition/lcdui/Item;` |  | 1.0 | `IndexOutOfBoundsException` if itemNum is invalid |  |
| 2 | `void setItemStateListener(ItemStateListener iListener)` | `setItemStateListener(Ljavax/microedition/lcdui/ItemStateListener;)V` |  | 1.0 |  |  |
| 2 | `int size()` | `size()I` |  | 1.0 |  |  |
| 3 | `int getWidth()` | `getWidth()I` |  | 2.0 |  |  |
| 3 | `int getHeight()` | `getHeight()I` |  | 2.0 |  |  |

### Gauge

`public class javax.microedition.lcdui.Gauge extends javax.microedition.lcdui.Item`

Tier 3. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Gauge.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Gauge.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Gauge.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Gauge.java).

Loading bars on `Form`s and `Alert`s.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `INDEFINITE` | `I` | `-1` | static | 2.0 |  |
| 3 | `CONTINUOUS_IDLE` | `I` | `0` | static | 2.0 |  |
| 3 | `INCREMENTAL_IDLE` | `I` | `1` | static | 2.0 |  |
| 3 | `CONTINUOUS_RUNNING` | `I` | `2` | static | 2.0 |  |
| 3 | `INCREMENTAL_UPDATING` | `I` | `3` | static | 2.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(String label, boolean interactive, int maxValue, int initialValue)` | `<init>(Ljava/lang/String;ZII)V` |  | 1.0 | `IllegalArgumentException` if maxValue is not positive for interactive gauges; `IllegalArgumentException` if maxValue is neither positive nor INDEFINITE for non-interactive gauges; `IllegalArgumentException` if initialValue is not one of CONTINUOUS_IDLE, INCREMENTAL_IDLE, CONTINUOUS_RUNNING, or INCREMENTAL_UPDATING for a non-interactive gauge with indefinite range |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void setLabel(String label)` | `setLabel(Ljava/lang/String;)V` |  | 1.0 | `IllegalStateException` if this Item is contained within an Alert |  |
| 3 | `void setLayout(int layout)` | `setLayout(I)V` |  | 2.0 | `IllegalArgumentException` if the value of layout is not a valid combination of layout directives; `IllegalStateException` if this Item is contained within an Alert |  |
| 3 | `void addCommand(Command cmd)` | `addCommand(Ljavax/microedition/lcdui/Command;)V` |  | 2.0 | `IllegalStateException` if this Item is contained within an Alert; `NullPointerException` if cmd is null |  |
| 3 | `void setItemCommandListener(ItemCommandListener l)` | `setItemCommandListener(Ljavax/microedition/lcdui/ItemCommandListener;)V` |  | 2.0 | `IllegalStateException` if this Item is contained within an Alert |  |
| 3 | `void setPreferredSize(int width, int height)` | `setPreferredSize(II)V` |  | 2.0 | `IllegalArgumentException` if width or height is less than -1; `IllegalStateException` if this Item is contained within an Alert |  |
| 3 | `void setDefaultCommand(Command cmd)` | `setDefaultCommand(Ljavax/microedition/lcdui/Command;)V` |  | 2.0 | `IllegalStateException` if this Item is contained within an Alert |  |
| 3 | `void setValue(int value)` | `setValue(I)V` |  | 1.0 | `IllegalArgumentException` if value is not one of CONTINUOUS_IDLE, INCREMENTAL_IDLE, CONTINUOUS_RUNNING, or INCREMENTAL_UPDATING for non-interactive gauges with indefinite range |  |
| 3 | `int getValue()` | `getValue()I` |  | 1.0 |  |  |
| 3 | `void setMaxValue(int maxValue)` | `setMaxValue(I)V` |  | 1.0 | `IllegalArgumentException` if maxValue is invalid |  |
| 3 | `int getMaxValue()` | `getMaxValue()I` |  | 1.0 |  |  |
| 3 | `boolean isInteractive()` | `isInteractive()Z` |  | 1.0 |  |  |

### Graphics

`public class javax.microedition.lcdui.Graphics extends java.lang.Object`

Tier 1. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Graphics.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Graphics.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Graphics.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Graphics.java).

The drawing API. A `Graphics` comes from `paint` or from `Image.getGraphics()`; it has no public constructor.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 1 | `HCENTER` | `I` | `1` | static | 1.0 |  |
| 1 | `VCENTER` | `I` | `2` | static | 1.0 |  |
| 1 | `LEFT` | `I` | `4` | static | 1.0 |  |
| 1 | `RIGHT` | `I` | `8` | static | 1.0 |  |
| 1 | `TOP` | `I` | `16` | static | 1.0 |  |
| 1 | `BOTTOM` | `I` | `32` | static | 1.0 |  |
| 1 | `BASELINE` | `I` | `64` | static | 1.0 |  |
| 2 | `SOLID` | `I` | `0` | static | 1.0 |  |
| 2 | `DOTTED` | `I` | `1` | static | 1.0 | Stroke for lines, rectangles and arcs; the dot pattern is up to the device. |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `void translate(int x, int y)` | `translate(II)V` |  | 1.0 |  | Adds to the current translation. |
| 2 | `int getTranslateX()` | `getTranslateX()I` |  | 1.0 |  |  |
| 2 | `int getTranslateY()` | `getTranslateY()I` |  | 1.0 |  |  |
| 2 | `int getColor()` | `getColor()I` |  | 1.0 |  |  |
| 3 | `int getRedComponent()` | `getRedComponent()I` |  | 1.0 |  |  |
| 3 | `int getGreenComponent()` | `getGreenComponent()I` |  | 1.0 |  |  |
| 3 | `int getBlueComponent()` | `getBlueComponent()I` |  | 1.0 |  |  |
| 3 | `int getGrayScale()` | `getGrayScale()I` |  | 1.0 |  |  |
| 1 | `void setColor(int red, int green, int blue)` | `setColor(III)V` |  | 1.0 | `IllegalArgumentException` if any of the color components are outside of range 0-255 | Throws `IllegalArgumentException` for a component outside 0 to 255. |
| 1 | `void setColor(int RGB)` | `setColor(I)V` |  | 1.0 |  | `0x00RRGGBB`; the high byte is ignored, so drawing is always opaque. |
| 3 | `void setGrayScale(int value)` | `setGrayScale(I)V` |  | 1.0 | `IllegalArgumentException` if the gray value is out of range |  |
| 2 | `Font getFont()` | `getFont()Ljavax/microedition/lcdui/Font;` |  | 1.0 |  |  |
| 2 | `void setStrokeStyle(int style)` | `setStrokeStyle(I)V` |  | 1.0 | `IllegalArgumentException` if the style is illegal |  |
| 3 | `int getStrokeStyle()` | `getStrokeStyle()I` |  | 1.0 |  |  |
| 1 | `void setFont(Font font)` | `setFont(Ljavax/microedition/lcdui/Font;)V` |  | 1.0 |  | `null` selects the default font. |
| 2 | `int getClipX()` | `getClipX()I` |  | 1.0 |  |  |
| 2 | `int getClipY()` | `getClipY()I` |  | 1.0 |  |  |
| 2 | `int getClipWidth()` | `getClipWidth()I` |  | 1.0 |  |  |
| 2 | `int getClipHeight()` | `getClipHeight()I` |  | 1.0 |  |  |
| 1 | `void clipRect(int x, int y, int width, int height)` | `clipRect(IIII)V` |  | 1.0 |  | Intersects the clip with the rectangle. |
| 1 | `void setClip(int x, int y, int width, int height)` | `setClip(IIII)V` |  | 1.0 |  | Replaces the clip, in translated coordinates. Games set it per sprite cell to draw frames out of a strip image. |
| 1 | `void drawLine(int x1, int y1, int x2, int y2)` | `drawLine(IIII)V` |  | 1.0 |  | Both end points are drawn. |
| 1 | `void fillRect(int x, int y, int width, int height)` | `fillRect(IIII)V` |  | 1.0 |  | Nothing for zero or negative width or height. |
| 1 | `void drawRect(int x, int y, int width, int height)` | `drawRect(IIII)V` |  | 1.0 |  | Covers `width + 1` by `height + 1` pixels. |
| 2 | `void drawRoundRect(int x, int y, int width, int height, int arcWidth, int arcHeight)` | `drawRoundRect(IIIIII)V` |  | 1.0 |  |  |
| 2 | `void fillRoundRect(int x, int y, int width, int height, int arcWidth, int arcHeight)` | `fillRoundRect(IIIIII)V` |  | 1.0 |  |  |
| 2 | `void fillArc(int x, int y, int width, int height, int startAngle, int arcAngle)` | `fillArc(IIIIII)V` |  | 1.0 |  | Angles in degrees, counter-clockwise from 3 o'clock. |
| 2 | `void drawArc(int x, int y, int width, int height, int startAngle, int arcAngle)` | `drawArc(IIIIII)V` |  | 1.0 |  | Angles in degrees, counter-clockwise from 3 o'clock. |
| 1 | `void drawString(String str, int x, int y, int anchor)` | `drawString(Ljava/lang/String;III)V` |  | 1.0 | `NullPointerException` if str is null; `IllegalArgumentException` if anchor is not a legal value | Anchor is one of `LEFT`, `HCENTER`, `RIGHT` ORed with one of `TOP`, `BASELINE`, `BOTTOM`; 0 means `TOP \| LEFT`. `VCENTER` is not legal for text. |
| 2 | `void drawSubstring(String str, int offset, int len, int x, int y, int anchor)` | `drawSubstring(Ljava/lang/String;IIIII)V` |  | 1.0 | `StringIndexOutOfBoundsException` if offset and length do not specify a valid range within the String str; `IllegalArgumentException` if anchor is not a legal value; `NullPointerException` if str is null |  |
| 2 | `void drawChar(char character, int x, int y, int anchor)` | `drawChar(CIII)V` |  | 1.0 | `IllegalArgumentException` if anchor is not a legal value |  |
| 2 | `void drawChars(char[] data, int offset, int length, int x, int y, int anchor)` | `drawChars([CIIIII)V` |  | 1.0 | `ArrayIndexOutOfBoundsException` if offset and length do not specify a valid range within the data array; `IllegalArgumentException` if anchor is not a legal value; `NullPointerException` if data is null |  |
| 1 | `void drawImage(Image img, int x, int y, int anchor)` | `drawImage(Ljavax/microedition/lcdui/Image;III)V` |  | 1.0 | `IllegalArgumentException` if anchor is not a legal value; `NullPointerException` if img is null | Anchor is a horizontal constant ORed with `TOP`, `VCENTER` or `BOTTOM`; `BASELINE` is not legal for images. Transparent pixels leave the destination untouched; semi-transparent pixels blend. |
| 1 | `void drawRegion(Image src, int x_src, int y_src, int width, int height, int transform, int x_dest, int y_dest, int anchor)` | `drawRegion(Ljavax/microedition/lcdui/Image;IIIIIIII)V` |  | 2.0 | `IllegalArgumentException` if src is the same image as the destination of this Graphics object; `NullPointerException` if src is null; `IllegalArgumentException` if transform is invalid; `IllegalArgumentException` if anchor is invalid; `IllegalArgumentException` if the region to be copied exceeds the bounds of the source image | MIDP 2.0. Draws a region of an image with one of the eight `Sprite.TRANS_*` transforms; the anchor applies to the transformed region. |
| 3 | `void copyArea(int x_src, int y_src, int width, int height, int x_dest, int y_dest, int anchor)` | `copyArea(IIIIIII)V` |  | 2.0 | `IllegalStateException` if the destination of this Graphics object is the display device; `IllegalArgumentException` if the region to be copied exceeds the bounds of the source image | MIDP 2.0. Only works on the graphics of a mutable image, never on the display's. |
| 2 | `void fillTriangle(int x1, int y1, int x2, int y2, int x3, int y3)` | `fillTriangle(IIIIII)V` |  | 2.0 |  |  |
| 1 | `void drawRGB(int[] rgbData, int offset, int scanlength, int x, int y, int width, int height, boolean processAlpha)` | `drawRGB([IIIIIIIZ)V` |  | 2.0 | `ArrayIndexOutOfBoundsException` if the requested operation will attempt to access an element of rgbData whose index is either negative or beyond its length; `NullPointerException` if rgbData is null | Draws `0xAARRGGBB` pixels from an `int[]`; `processAlpha` false means every pixel is opaque. Subject to clip and translation. Games draw fades and generated sprites with it. |
| 3 | `int getDisplayColor(int color)` | `getDisplayColor(I)I` |  | 2.0 |  |  |

### Image

`public class javax.microedition.lcdui.Image extends java.lang.Object`

Tier 1. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Image.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Image.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Image.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Image.java).

Mutable images (from `createImage(int, int)`) can be drawn on through `getGraphics`; all others are immutable. MIDP 2.0 requires PNG with every color type and bit depth: alpha 0 is transparent, full alpha opaque, and values between blend when `numAlphaLevels` is above 2 (otherwise they become transparent). Other formats are optional.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `Image createImage(int width, int height)` | `createImage(II)Ljavax/microedition/lcdui/Image;` | static | 1.0 | `IllegalArgumentException` if either width or height is zero or less | A mutable image with every pixel white. Throws `IllegalArgumentException` for a zero or negative size. |
| 2 | `Image createImage(Image source)` | `createImage(Ljavax/microedition/lcdui/Image;)Ljavax/microedition/lcdui/Image;` | static | 1.0 | `NullPointerException` if source is null | An immutable copy; may return the same object for an immutable source. |
| 1 | `Image createImage(String name)` | `createImage(Ljava/lang/String;)Ljavax/microedition/lcdui/Image;` | static | 1.0 | `NullPointerException` if name is null; `IOException` if the resource does not exist, the data cannot be loaded, or the image data cannot be decoded | Loads a resource from the JAR by the rules of `Class.getResourceAsStream`. Throws `IOException` when it is missing or cannot be decoded. |
| 1 | `Image createImage(byte[] imageData, int imageOffset, int imageLength)` | `createImage([BII)Ljavax/microedition/lcdui/Image;` | static | 1.0 | `ArrayIndexOutOfBoundsException` if imageOffset and imageLength specify an invalid range; `NullPointerException` if imageData is null; `IllegalArgumentException` if imageData is incorrectly formatted or otherwise cannot be decoded | Decodes PNG (or another supported format) from part of an array. Games that pack images into one data file use it. |
| 2 | `Image createImage(Image image, int x, int y, int width, int height, int transform)` | `createImage(Ljavax/microedition/lcdui/Image;IIIII)Ljavax/microedition/lcdui/Image;` | static | 2.0 | `NullPointerException` if image is null; `IllegalArgumentException` if the region to be copied exceeds the bounds of the source image; `IllegalArgumentException` if either width or height is zero or less; `IllegalArgumentException` if the transform is not valid | A transformed region as a new immutable image. |
| 1 | `Graphics getGraphics()` | `getGraphics()Ljavax/microedition/lcdui/Graphics;` |  | 1.0 | `IllegalStateException` if the image is immutable | A new `Graphics` each call, with clip covering the image, color black, default font. Throws `IllegalStateException` for an immutable image. |
| 1 | `int getWidth()` | `getWidth()I` |  | 1.0 |  |  |
| 1 | `int getHeight()` | `getHeight()I` |  | 1.0 |  |  |
| 3 | `boolean isMutable()` | `isMutable()Z` |  | 1.0 |  |  |
| 2 | `Image createImage(InputStream stream)` | `createImage(Ljava/io/InputStream;)Ljavax/microedition/lcdui/Image;` | static | 2.0 | `NullPointerException` if stream is null; `IOException` if an I/O error occurs, if the image data cannot be loaded, or if the image data cannot be decoded | Leaves the stream open at an undefined position. |
| 1 | `Image createRGBImage(int[] rgb, int width, int height, boolean processAlpha)` | `createRGBImage([IIIZ)Ljavax/microedition/lcdui/Image;` | static | 2.0 | `NullPointerException` if rgb is null; `IllegalArgumentException` if either width or height is zero or less; `ArrayIndexOutOfBoundsException` if the length of rgb is less than width * height | An immutable image from `0xAARRGGBB` pixels. |
| 1 | `void getRGB(int[] rgbData, int offset, int scanlength, int x, int y, int width, int height)` | `getRGB([IIIIIII)V` |  | 2.0 | `ArrayIndexOutOfBoundsException` if the requested operation would attempt to access an element in the rgbData array whose index is either negative or beyond its length (the contents of the array are unchanged); `IllegalArgumentException` if the area being retrieved exceeds the bounds of the source image; `IllegalArgumentException` if the absolute value of scanlength is less than width; `NullPointerException` if rgbData is null | Games read pixels back to recolor or to test collisions; values may be resampled to the device's color depth. |

### ImageItem

`public class javax.microedition.lcdui.ImageItem extends javax.microedition.lcdui.Item`

Tier 3. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/ImageItem.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/ImageItem.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/ImageItem.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/ImageItem.java).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `LAYOUT_DEFAULT` | `I` | `0` | static | 1.0 |  |
| 3 | `LAYOUT_LEFT` | `I` | `1` | static | 1.0 |  |
| 3 | `LAYOUT_RIGHT` | `I` | `2` | static | 1.0 |  |
| 3 | `LAYOUT_CENTER` | `I` | `3` | static | 1.0 |  |
| 3 | `LAYOUT_NEWLINE_BEFORE` | `I` | `256` (`0x100`) | static | 1.0 |  |
| 3 | `LAYOUT_NEWLINE_AFTER` | `I` | `512` (`0x200`) | static | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(String label, Image img, int layout, String altText)` | `<init>(Ljava/lang/String;Ljavax/microedition/lcdui/Image;ILjava/lang/String;)V` |  | 1.0 | `IllegalArgumentException` if the layout value is not a legal combination of directives |  |
| 3 | `(String label, Image image, int layout, String altText, int appearanceMode)` | `<init>(Ljava/lang/String;Ljavax/microedition/lcdui/Image;ILjava/lang/String;I)V` |  | 2.0 | `IllegalArgumentException` if the layout value is not a legal combination of directives; `IllegalArgumentException` if appearanceMode invalid |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `Image getImage()` | `getImage()Ljavax/microedition/lcdui/Image;` |  | 1.0 |  |  |
| 3 | `void setImage(Image img)` | `setImage(Ljavax/microedition/lcdui/Image;)V` |  | 1.0 |  |  |
| 3 | `String getAltText()` | `getAltText()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `void setAltText(String text)` | `setAltText(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 3 | `int getLayout()` | `getLayout()I` |  | 1.0 |  |  |
| 3 | `void setLayout(int layout)` | `setLayout(I)V` |  | 1.0 | `IllegalArgumentException` if the value of layout is not a valid combination of layout directives |  |
| 3 | `int getAppearanceMode()` | `getAppearanceMode()I` |  | 2.0 |  |  |

### Item

`public abstract class javax.microedition.lcdui.Item extends java.lang.Object`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Item.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Item.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Item.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Item.java).

Base of the `Form` items.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `LAYOUT_DEFAULT` | `I` | `0` | static | 2.0 |  |
| 3 | `LAYOUT_LEFT` | `I` | `1` | static | 2.0 |  |
| 3 | `LAYOUT_RIGHT` | `I` | `2` | static | 2.0 |  |
| 2 | `LAYOUT_CENTER` | `I` | `3` | static | 2.0 |  |
| 3 | `LAYOUT_TOP` | `I` | `16` | static | 2.0 |  |
| 3 | `LAYOUT_BOTTOM` | `I` | `32` | static | 2.0 |  |
| 3 | `LAYOUT_VCENTER` | `I` | `48` | static | 2.0 |  |
| 2 | `LAYOUT_NEWLINE_BEFORE` | `I` | `256` (`0x100`) | static | 2.0 |  |
| 2 | `LAYOUT_NEWLINE_AFTER` | `I` | `512` (`0x200`) | static | 2.0 |  |
| 3 | `LAYOUT_SHRINK` | `I` | `1024` (`0x400`) | static | 2.0 |  |
| 3 | `LAYOUT_EXPAND` | `I` | `2048` (`0x800`) | static | 2.0 |  |
| 3 | `LAYOUT_VSHRINK` | `I` | `4096` (`0x1000`) | static | 2.0 |  |
| 3 | `LAYOUT_VEXPAND` | `I` | `8192` (`0x2000`) | static | 2.0 |  |
| 3 | `LAYOUT_2` | `I` | `16384` (`0x4000`) | static | 2.0 | MIDP 2.0 layout rules apply only when this bit is set. |
| 3 | `PLAIN` | `I` | `0` | static | 2.0 |  |
| 3 | `HYPERLINK` | `I` | `1` | static | 2.0 |  |
| 3 | `BUTTON` | `I` | `2` | static | 2.0 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void setLabel(String label)` | `setLabel(Ljava/lang/String;)V` |  | 1.0 | `IllegalStateException` if this Item is contained within an Alert |  |
| 2 | `String getLabel()` | `getLabel()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `int getLayout()` | `getLayout()I` |  | 2.0 |  |  |
| 3 | `void setLayout(int layout)` | `setLayout(I)V` |  | 2.0 | `IllegalArgumentException` if the value of layout is not a bit-wise OR combination of layout directives; `IllegalStateException` if this Item is contained within an Alert |  |
| 3 | `void addCommand(Command cmd)` | `addCommand(Ljavax/microedition/lcdui/Command;)V` |  | 2.0 | `IllegalStateException` if this Item is contained within an Alert; `NullPointerException` if cmd is null |  |
| 3 | `void removeCommand(Command cmd)` | `removeCommand(Ljavax/microedition/lcdui/Command;)V` |  | 2.0 |  |  |
| 3 | `void setItemCommandListener(ItemCommandListener l)` | `setItemCommandListener(Ljavax/microedition/lcdui/ItemCommandListener;)V` |  | 2.0 | `IllegalStateException` if this Item is contained within an Alert |  |
| 3 | `int getPreferredWidth()` | `getPreferredWidth()I` |  | 2.0 |  |  |
| 3 | `int getPreferredHeight()` | `getPreferredHeight()I` |  | 2.0 |  |  |
| 3 | `void setPreferredSize(int width, int height)` | `setPreferredSize(II)V` |  | 2.0 | `IllegalArgumentException` if width or height is less than -1; `IllegalStateException` if this Item is contained within an Alert |  |
| 3 | `int getMinimumWidth()` | `getMinimumWidth()I` |  | 2.0 |  |  |
| 3 | `int getMinimumHeight()` | `getMinimumHeight()I` |  | 2.0 |  |  |
| 3 | `void setDefaultCommand(Command cmd)` | `setDefaultCommand(Ljavax/microedition/lcdui/Command;)V` |  | 2.0 | `IllegalStateException` if this Item is contained within an Alert |  |
| 3 | `void notifyStateChanged()` | `notifyStateChanged()V` |  | 2.0 | `IllegalStateException` if the Item is not owned by a Form |  |

### ItemCommandListener

`public interface javax.microedition.lcdui.ItemCommandListener`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/ItemCommandListener.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/ItemCommandListener.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/ItemCommandListener.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/ItemCommandListener.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void commandAction(Command c, Item item)` | `commandAction(Ljavax/microedition/lcdui/Command;Ljavax/microedition/lcdui/Item;)V` |  | 2.0 |  |  |

### ItemStateListener

`public interface javax.microedition.lcdui.ItemStateListener`

Tier 3. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/ItemStateListener.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/ItemStateListener.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/ItemStateListener.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/ItemStateListener.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void itemStateChanged(Item item)` | `itemStateChanged(Ljavax/microedition/lcdui/Item;)V` |  | 1.0 |  |  |

### List

`public class javax.microedition.lcdui.List extends javax.microedition.lcdui.Screen implements javax.microedition.lcdui.Choice`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/List.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/List.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/List.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/List.java).

Menus in games that do not draw their own. `IMPLICIT` lists send `SELECT_COMMAND` to the listener when the user picks an element.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `SELECT_COMMAND` | `Ljavax/microedition/lcdui/Command;` | (not a constant) | static | 1.0 | The `Command` an `IMPLICIT` list sends on selection. |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(String title, int listType)` | `<init>(Ljava/lang/String;I)V` |  | 1.0 | `IllegalArgumentException` if listType is not one of IMPLICIT, EXCLUSIVE, or MULTIPLE |  |
| 2 | `(String title, int listType, String[] stringElements, Image[] imageElements)` | `<init>(Ljava/lang/String;I[Ljava/lang/String;[Ljavax/microedition/lcdui/Image;)V` |  | 1.0 | `NullPointerException` if stringElements is null; `NullPointerException` if the stringElements array contains any null elements; `IllegalArgumentException` if the imageElements array is non-null and has a different length from the stringElements array; `IllegalArgumentException` if listType is not one of IMPLICIT, EXCLUSIVE, or MULTIPLE |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void setTicker(Ticker ticker)` | `setTicker(Ljavax/microedition/lcdui/Ticker;)V` |  | 2.0 |  |  |
| 2 | `void setTitle(String s)` | `setTitle(Ljava/lang/String;)V` |  | 2.0 |  |  |
| 2 | `int size()` | `size()I` |  | 1.0 |  |  |
| 2 | `String getString(int elementNum)` | `getString(I)Ljava/lang/String;` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 2 | `Image getImage(int elementNum)` | `getImage(I)Ljavax/microedition/lcdui/Image;` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 2 | `int append(String stringPart, Image imagePart)` | `append(Ljava/lang/String;Ljavax/microedition/lcdui/Image;)I` |  | 1.0 | `NullPointerException` if stringPart is null |  |
| 2 | `void insert(int elementNum, String stringPart, Image imagePart)` | `insert(ILjava/lang/String;Ljavax/microedition/lcdui/Image;)V` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid; `NullPointerException` if stringPart is null |  |
| 2 | `void delete(int elementNum)` | `delete(I)V` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 2 | `void deleteAll()` | `deleteAll()V` |  | 1.0 |  |  |
| 2 | `void set(int elementNum, String stringPart, Image imagePart)` | `set(ILjava/lang/String;Ljavax/microedition/lcdui/Image;)V` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid; `NullPointerException` if stringPart is null |  |
| 2 | `boolean isSelected(int elementNum)` | `isSelected(I)Z` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 2 | `int getSelectedIndex()` | `getSelectedIndex()I` |  | 1.0 |  |  |
| 3 | `int getSelectedFlags(boolean[] selectedArray_return)` | `getSelectedFlags([Z)I` |  | 1.0 | `IllegalArgumentException` if selectedArray_return is shorter than the size of the List; `NullPointerException` if selectedArray_return is null |  |
| 2 | `void setSelectedIndex(int elementNum, boolean selected)` | `setSelectedIndex(IZ)V` |  | 1.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 3 | `void setSelectedFlags(boolean[] selectedArray)` | `setSelectedFlags([Z)V` |  | 1.0 | `IllegalArgumentException` if selectedArray is shorter than the size of the List; `NullPointerException` if selectedArray is null |  |
| 2 | `void removeCommand(Command cmd)` | `removeCommand(Ljavax/microedition/lcdui/Command;)V` |  | 2.0 |  |  |
| 3 | `void setSelectCommand(Command command)` | `setSelectCommand(Ljavax/microedition/lcdui/Command;)V` |  | 2.0 |  |  |
| 3 | `void setFitPolicy(int fitPolicy)` | `setFitPolicy(I)V` |  | 2.0 | `IllegalArgumentException` if fitPolicy is invalid |  |
| 3 | `int getFitPolicy()` | `getFitPolicy()I` |  | 2.0 |  |  |
| 3 | `void setFont(int elementNum, Font font)` | `setFont(ILjavax/microedition/lcdui/Font;)V` |  | 2.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |
| 3 | `Font getFont(int elementNum)` | `getFont(I)Ljavax/microedition/lcdui/Font;` |  | 2.0 | `IndexOutOfBoundsException` if elementNum is invalid |  |

### Screen

`public abstract class javax.microedition.lcdui.Screen extends javax.microedition.lcdui.Displayable`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Screen.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Screen.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Screen.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Screen.java).

Abstract base of `Alert`, `Form`, `List` and `TextBox`. In MIDP 2.0 it declares nothing of its own; MIDP 1.0 declared `getTitle`, `setTitle`, `getTicker` and `setTicker` here, which now resolve to `Displayable`.

Declares no members of its own.

### Spacer

`public class javax.microedition.lcdui.Spacer extends javax.microedition.lcdui.Item`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Spacer.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Spacer.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Spacer.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Spacer.java).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(int minWidth, int minHeight)` | `<init>(II)V` |  | 2.0 | `IllegalArgumentException` if either minWidth or minHeight is less than zero |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void setMinimumSize(int minWidth, int minHeight)` | `setMinimumSize(II)V` |  | 2.0 | `IllegalArgumentException` if either minWidth or minHeight is less than zero |  |
| 3 | `void addCommand(Command cmd)` | `addCommand(Ljavax/microedition/lcdui/Command;)V` |  | 2.0 | `IllegalStateException` always |  |
| 3 | `void setDefaultCommand(Command cmd)` | `setDefaultCommand(Ljavax/microedition/lcdui/Command;)V` |  | 2.0 | `IllegalStateException` always |  |
| 3 | `void setLabel(String label)` | `setLabel(Ljava/lang/String;)V` |  | 2.0 | `IllegalStateException` always |  |

### StringItem

`public class javax.microedition.lcdui.StringItem extends javax.microedition.lcdui.Item`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/StringItem.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/StringItem.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/StringItem.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/StringItem.java).

Text in a `Form`.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(String label, String text)` | `<init>(Ljava/lang/String;Ljava/lang/String;)V` |  | 1.0 |  |  |
| 3 | `(String label, String text, int appearanceMode)` | `<init>(Ljava/lang/String;Ljava/lang/String;I)V` |  | 2.0 | `IllegalArgumentException` if appearanceMode invalid |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `String getText()` | `getText()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `void setText(String text)` | `setText(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 3 | `int getAppearanceMode()` | `getAppearanceMode()I` |  | 2.0 |  |  |
| 3 | `void setFont(Font font)` | `setFont(Ljavax/microedition/lcdui/Font;)V` |  | 2.0 |  |  |
| 3 | `Font getFont()` | `getFont()Ljavax/microedition/lcdui/Font;` |  | 2.0 |  |  |
| 3 | `void setPreferredSize(int width, int height)` | `setPreferredSize(II)V` |  | 2.0 | `IllegalArgumentException` if width or height is less than -1; `IllegalStateException` if this Item is contained within an Alert |  |

### TextBox

`public class javax.microedition.lcdui.TextBox extends javax.microedition.lcdui.Screen`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/TextBox.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/TextBox.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/TextBox.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/TextBox.java).

Name entry for high scores. FastEmu feeds host keyboard text into it.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(String title, String text, int maxSize, int constraints)` | `<init>(Ljava/lang/String;Ljava/lang/String;II)V` |  | 1.0 | `IllegalArgumentException` if maxSize is zero or less; `IllegalArgumentException` if the constraints parameter is invalid; `IllegalArgumentException` if text is illegal for the specified constraints; `IllegalArgumentException` if the length of the string exceeds the requested maximum capacity |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `String getString()` | `getString()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `void setString(String text)` | `setString(Ljava/lang/String;)V` |  | 1.0 | `IllegalArgumentException` if text is illegal for the current input constraints; `IllegalArgumentException` if the text would exceed the current maximum capacity |  |
| 3 | `int getChars(char[] data)` | `getChars([C)I` |  | 1.0 | `ArrayIndexOutOfBoundsException` if the array is too short for the contents; `NullPointerException` if data is null |  |
| 3 | `void setChars(char[] data, int offset, int length)` | `setChars([CII)V` |  | 1.0 | `ArrayIndexOutOfBoundsException` if offset and length do not specify a valid range within the data array; `IllegalArgumentException` if data is illegal for the current input constraints; `IllegalArgumentException` if the text would exceed the current maximum capacity |  |
| 3 | `void insert(String src, int position)` | `insert(Ljava/lang/String;I)V` |  | 1.0 | `IllegalArgumentException` if the resulting contents would be illegal for the current input constraints; `IllegalArgumentException` if the insertion would exceed the current maximum capacity; `NullPointerException` if src is null |  |
| 3 | `void insert(char[] data, int offset, int length, int position)` | `insert([CIII)V` |  | 1.0 | `ArrayIndexOutOfBoundsException` if offset and length do not specify a valid range within the data array; `IllegalArgumentException` if the resulting contents would be illegal for the current input constraints; `IllegalArgumentException` if the insertion would exceed the current maximum capacity; `NullPointerException` if data is null |  |
| 3 | `void delete(int offset, int length)` | `delete(II)V` |  | 1.0 | `IllegalArgumentException` if the resulting contents would be illegal for the current input constraints; `StringIndexOutOfBoundsException` if offset and length do not specify a valid range within the contents of the TextBox |  |
| 2 | `int getMaxSize()` | `getMaxSize()I` |  | 1.0 |  |  |
| 3 | `int setMaxSize(int maxSize)` | `setMaxSize(I)I` |  | 1.0 | `IllegalArgumentException` if maxSize is zero or less; `IllegalArgumentException` if the contents after truncation would be illegal for the current input constraints |  |
| 2 | `int size()` | `size()I` |  | 1.0 |  |  |
| 3 | `int getCaretPosition()` | `getCaretPosition()I` |  | 1.0 |  |  |
| 3 | `void setConstraints(int constraints)` | `setConstraints(I)V` |  | 1.0 | `IllegalArgumentException` if the value of the constraints parameter is invalid |  |
| 3 | `int getConstraints()` | `getConstraints()I` |  | 1.0 |  |  |
| 3 | `void setInitialInputMode(String characterSubset)` | `setInitialInputMode(Ljava/lang/String;)V` |  | 2.0 |  |  |
| 2 | `void setTitle(String s)` | `setTitle(Ljava/lang/String;)V` |  | 2.0 |  |  |
| 3 | `void setTicker(Ticker ticker)` | `setTicker(Ljavax/microedition/lcdui/Ticker;)V` |  | 2.0 |  |  |

### TextField

`public class javax.microedition.lcdui.TextField extends javax.microedition.lcdui.Item`

Tier 3. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/TextField.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/TextField.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/TextField.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/TextField.java).

A text input in a `Form`. The constraint constants are shared with `TextBox`.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `ANY` | `I` | `0` | static | 1.0 |  |
| 3 | `EMAILADDR` | `I` | `1` | static | 1.0 |  |
| 2 | `NUMERIC` | `I` | `2` | static | 1.0 |  |
| 3 | `PHONENUMBER` | `I` | `3` | static | 1.0 |  |
| 3 | `URL` | `I` | `4` | static | 1.0 |  |
| 3 | `DECIMAL` | `I` | `5` | static | 2.0 |  |
| 2 | `PASSWORD` | `I` | `65536` (`0x10000`) | static | 1.0 |  |
| 3 | `UNEDITABLE` | `I` | `131072` (`0x20000`) | static | 2.0 |  |
| 3 | `SENSITIVE` | `I` | `262144` (`0x40000`) | static | 2.0 |  |
| 3 | `NON_PREDICTIVE` | `I` | `524288` (`0x80000`) | static | 2.0 |  |
| 3 | `INITIAL_CAPS_WORD` | `I` | `1048576` (`0x100000`) | static | 2.0 |  |
| 3 | `INITIAL_CAPS_SENTENCE` | `I` | `2097152` (`0x200000`) | static | 2.0 |  |
| 2 | `CONSTRAINT_MASK` | `I` | `65535` (`0xffff`) | static | 1.0 | `(constraints & CONSTRAINT_MASK)` is one of `ANY` to `DECIMAL`; the modifiers (`PASSWORD` and up) are the high bits. |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(String label, String text, int maxSize, int constraints)` | `<init>(Ljava/lang/String;Ljava/lang/String;II)V` |  | 1.0 | `IllegalArgumentException` if maxSize is zero or less; `IllegalArgumentException` if the value of the constraints parameter is invalid; `IllegalArgumentException` if text is illegal for the specified constraints; `IllegalArgumentException` if the length of the string exceeds the requested maximum capacity |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `String getString()` | `getString()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `void setString(String text)` | `setString(Ljava/lang/String;)V` |  | 1.0 | `IllegalArgumentException` if text is illegal for the current input constraints; `IllegalArgumentException` if the text would exceed the current maximum capacity |  |
| 3 | `int getChars(char[] data)` | `getChars([C)I` |  | 1.0 | `ArrayIndexOutOfBoundsException` if the array is too short for the contents; `NullPointerException` if data is null |  |
| 3 | `void setChars(char[] data, int offset, int length)` | `setChars([CII)V` |  | 1.0 | `ArrayIndexOutOfBoundsException` if offset and length do not specify a valid range within the data array; `IllegalArgumentException` if data is illegal for the current input constraints; `IllegalArgumentException` if the text would exceed the current maximum capacity |  |
| 3 | `void insert(String src, int position)` | `insert(Ljava/lang/String;I)V` |  | 1.0 | `IllegalArgumentException` if the resulting contents would be illegal for the current input constraints; `IllegalArgumentException` if the insertion would exceed the current maximum capacity; `NullPointerException` if src is null |  |
| 3 | `void insert(char[] data, int offset, int length, int position)` | `insert([CIII)V` |  | 1.0 | `ArrayIndexOutOfBoundsException` if offset and length do not specify a valid range within the data array; `IllegalArgumentException` if the resulting contents would be illegal for the current input constraints; `IllegalArgumentException` if the insertion would exceed the current maximum capacity; `NullPointerException` if data is null |  |
| 3 | `void delete(int offset, int length)` | `delete(II)V` |  | 1.0 | `IllegalArgumentException` if the resulting contents would be illegal for the current input constraints; `StringIndexOutOfBoundsException` if offset and length do not specify a valid range within the contents of the TextField |  |
| 3 | `int getMaxSize()` | `getMaxSize()I` |  | 1.0 |  |  |
| 3 | `int setMaxSize(int maxSize)` | `setMaxSize(I)I` |  | 1.0 | `IllegalArgumentException` if maxSize is zero or less; `IllegalArgumentException` if the contents after truncation would be illegal for the current input constraints |  |
| 3 | `int size()` | `size()I` |  | 1.0 |  |  |
| 3 | `int getCaretPosition()` | `getCaretPosition()I` |  | 1.0 |  |  |
| 3 | `void setConstraints(int constraints)` | `setConstraints(I)V` |  | 1.0 | `IllegalArgumentException` if constraints is not any of the ones specified in input constraints |  |
| 3 | `int getConstraints()` | `getConstraints()I` |  | 1.0 |  |  |
| 3 | `void setInitialInputMode(String characterSubset)` | `setInitialInputMode(Ljava/lang/String;)V` |  | 2.0 |  |  |

### Ticker

`public class javax.microedition.lcdui.Ticker extends java.lang.Object`

Tier 3. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Ticker.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Ticker.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Ticker.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Ticker.java).

Scrolling text across the top of a screen.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(String str)` | `<init>(Ljava/lang/String;)V` |  | 1.0 | `NullPointerException` if str is null |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void setString(String str)` | `setString(Ljava/lang/String;)V` |  | 1.0 | `NullPointerException` if str is null |  |
| 3 | `String getString()` | `getString()Ljava/lang/String;` |  | 1.0 |  |  |

## Cross-check

Every member above comes from the spec javadoc. The script that writes this file compared it with the stub and implementation sources listed under Sources: constant values, member lists and `static` flags. Member lists are compared only where the parsed source holds the implementation's whole class hierarchy. What differs is listed here; a class not listed matched everywhere it was compared.

- `javax.microedition.lcdui.CustomItem`
  - [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/CustomItem.java) does not declare `keyPressed(I)V`
