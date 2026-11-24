# Nokia UI API (com.nokia.mid.ui, com.nokia.mid.sound)

Nokia UI API 1.1: Nokia's extensions for games on Series 40 and Series 60
phones, in `com.nokia.mid.ui` and `com.nokia.mid.sound`. Many MIDP 1.0
games need it: before MIDP 2.0 it was the only way to get a full-screen
canvas, transparency, image flips and rotations, sound and vibration on a
Nokia. Later games still use `DirectGraphics` and `DirectUtils`, and
games built for other vendors often probe for these classes with
`Class.forName("com.nokia.mid.ui.FullCanvas")`. FastEmu should offer the
whole API on every device profile, as J2ME Loader and freej2me-plus do.

Most of the API is marked deprecated "as of Nokia UI API 1.1 in devices
with MIDP 2.0 or higher". Deprecated here only means MIDP 2.0 has an
equivalent; games still call these members, so they are not tier 3 for
that reason.

Things that trip up implementations:

- **One graphics context.** `DirectUtils.getDirectGraphics(g)` returns a
  view of the same context: color, clip and translation are shared both
  ways, and `setARGBColor` sets the color `Graphics` draws with,
  including its alpha. `Graphics.setColor` resets the alpha to opaque.
  The javadoc warns that casting `(DirectGraphics) g` "may not work", and
  says the `Graphics` passed to `FullCanvas.paint` is a `DirectGraphics`.
  freej2me-plus makes its graphics class implement `DirectGraphics`, so
  games that cast keep working; FastEmu should do the same.
- **Rotations run counter-clockwise**, unlike MIDP's `Sprite.TRANS_ROT*`,
  which run clockwise: Nokia's `ROTATE_90` is MIDP's `TRANS_ROT270`.
  With several bits set, the rotation is applied first, then the
  vertical flip, then the horizontal flip, and the anchor last.
- **Pixel formats** are named by their constants' values: 4444 is
  `TYPE_USHORT_4444_ARGB`, 565 is `TYPE_USHORT_565_RGB`, 888 and 8888 are
  the int formats. `getNativePixelFormat` tells games which format is
  fastest; J2ME Loader answers `TYPE_USHORT_565_RGB`, MicroEmulator
  `TYPE_BYTE_1_GRAY` and freej2me-plus 0. Games that trust the answer
  build their pixel arrays in that format, so FastEmu should answer a
  format it converts well (`TYPE_USHORT_4444_ARGB` or
  `TYPE_INT_8888_ARGB`) and accept every format in `drawPixels`.
- **Alpha.** A pixel is transparent only when its alpha is 0; any other
  alpha draws (fully or blended).
- **FullCanvas keys.** A `FullCanvas` has no commands: the soft keys
  arrive as key events with codes `KEY_SOFTKEY1` (-6), `KEY_SOFTKEY2`
  (-7) and `KEY_SOFTKEY3` (-5, the select key), matching
  `fastemu-core`'s `KeyCodes::NOKIA`.

Column meanings and tier rules: [README](README.md).

## Sources

- [Nokia UI API 1.1 javadoc](https://nikita36078.github.io/J2ME_Docs/docs/Nokia_UI_API_1_1/overview-summary.html) (Nokia), from [J2ME_Docs](https://github.com/nikita36078/J2ME_Docs/tree/fbfdfed7009369b6bf7a5cce39b8a13f178c2c69/docs) at commit `fbfdfed7009369b6bf7a5cce39b8a13f178c2c69`: a collection of vendor and JSR javadoc sets kept by the J2ME Loader author, browsable at [nikita36078.github.io/J2ME_Docs](https://nikita36078.github.io/J2ME_Docs/). The source of every member, throws entry and since tag. Its `constant-values.html` is empty, so constant values come from the stub and implementations below.
- [Nokia UI API javadoc, later edition](https://nikita36078.github.io/J2ME_Docs/docs/nokiaapi2/overview-summary.html) (the Nokia UI API of Series 40 and Symbian phones with MIDP 2.1): members it adds to the 1.1 classes.
- MicroEmulator 2.0.4 [microemu-nokiaui](https://repo1.maven.org/maven2/org/microemu/microemu-nokiaui/2.0.4/microemu-nokiaui-2.0.4.jar) jar (Maven Central): compiled class files, read for descriptors and `ConstantValue` attributes.
- [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/tree/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java) at commit `e270ed91e67b5cafc18c1991b92ba5249cf41c50` (Apache-2.0): implementation source, checked for constant values and member lists.
- [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/tree/aab3350832c0d82aff1771394d28bff6f66b0aa4/src) at commit `aab3350832c0d82aff1771394d28bff6f66b0aa4` (GPL-3.0): implementation source, checked for constant values and member lists.
- [MicroEmulator](https://github.com/barteo/microemu/tree/e47b656d983deaa67810c776662444483bdbf134/microemulator) at commit `e47b656d983deaa67810c776662444483bdbf134` (LGPL-2.1 or Apache-2.0): implementation source, checked for constant values and member lists.

## Classes

| Class | Kind | Tier | Since | Members |
|---|---|---|---|---|
| [DeviceControl](#devicecontrol) | class | 2 | Nokia UI 1.0 | 4 |
| [DirectGraphics](#directgraphics) | interface | 1 | Nokia UI 1.0 | 32 |
| [DirectUtils](#directutils) | class | 1 | Nokia UI 1.0 | 3 |
| [FullCanvas](#fullcanvas) | class | 1 | Nokia UI 1.0 | 12 |
| [Sound](#sound) | class | 1 | Nokia UI 1.0 | 19 |
| [SoundListener](#soundlistener) | interface | 2 | Nokia UI 1.0 | 1 |

## com.nokia.mid.ui

### DeviceControl

`public class com.nokia.mid.ui.DeviceControl extends java.lang.Object`

Tier 2. Since Nokia UI 1.0. Javadoc: [Nokia_UI_API_1_1](https://nikita36078.github.io/J2ME_Docs/docs/Nokia_UI_API_1_1/com/nokia/mid/ui/DeviceControl.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/ui/DeviceControl.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/nokia/mid/ui/DeviceControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-nokiaui/src/main/java/com/nokia/mid/ui/DeviceControl.java).

Backlight and vibration.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void setLights(int num, int level)` | `setLights(II)V` | static | 1.0 | `IllegalArgumentException` if light num is not supported or level is not between 0-100 | Light 0 is the display backlight; level 0 to 100. Throws `IllegalArgumentException` for other lights or levels. Some games call it often to keep the screen on, so it must be cheap. |
| 3 | `void flashLights(long duration)` | `flashLights(J)V` | static deprecated | 1.0 | `IllegalArgumentException` if duration < 0 |  |
| 2 | `void startVibra(int freq, long duration)` | `startVibra(IJ)V` | static deprecated | 1.0 | `IllegalStateException` For freq values 1-100: the use of vibration is not allowed or the system does not have vibration support. For frequency value 0: the device does not have vibration; `IllegalArgumentException` if duration or freq is 100 | Frequency 0 to 100 and duration in milliseconds. FastEmu can show vibration in its window or ignore it. |
| 2 | `void stopVibra()` | `stopVibra()V` | static deprecated | 1.0 |  |  |

Added by a later edition ([nokiaapi2](https://nikita36078.github.io/J2ME_Docs/docs/nokiaapi2/com/nokia/mid/ui/DeviceControl.html)):

| T | Member | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `KEYMAT_DEFAULT` = `0` | `I` | static |  |  |  |
| 3 | `KEYMAT_NUMERIC` = `1` | `I` | static |  |  |  |
| 3 | `KEYMAT_ALPHANUMERIC` = `2` | `I` | static |  |  |  |
| 3 | `KEYMAT_OFF` = `3` | `I` | static |  |  |  |
| 3 | `int getUserInactivityTime()` | `getUserInactivityTime()I` | static | 1.4 |  |  |
| 3 | `void resetUserInactivityTime()` | `resetUserInactivityTime()V` | static | 1.4 |  |  |

### DirectGraphics

`public interface com.nokia.mid.ui.DirectGraphics`

Tier 1. Since Nokia UI 1.0. Javadoc: [Nokia_UI_API_1_1](https://nikita36078.github.io/J2ME_Docs/docs/Nokia_UI_API_1_1/com/nokia/mid/ui/DirectGraphics.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/ui/DirectGraphics.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/nokia/mid/ui/DirectGraphics.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-nokiaui/src/main/java/com/nokia/mid/ui/DirectGraphics.java).

Interface: what `DirectUtils.getDirectGraphics` returns. See the notes above on shared state, rotation order and alpha.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 1 | `FLIP_HORIZONTAL` | `I` | `8192` (`0x2000`) | static | 1.0 |  |
| 1 | `FLIP_VERTICAL` | `I` | `16384` (`0x4000`) | static | 1.0 |  |
| 1 | `ROTATE_90` | `I` | `90` | static | 1.0 |  |
| 1 | `ROTATE_180` | `I` | `180` | static | 1.0 |  |
| 1 | `ROTATE_270` | `I` | `270` | static | 1.0 |  |
| 2 | `TYPE_BYTE_1_GRAY` | `I` | `1` | static | 1.0 |  |
| 3 | `TYPE_BYTE_1_GRAY_VERTICAL` | `I` | `-1` | static | 1.0 |  |
| 3 | `TYPE_BYTE_2_GRAY` | `I` | `2` | static | 1.0 |  |
| 3 | `TYPE_BYTE_4_GRAY` | `I` | `4` | static | 1.0 |  |
| 3 | `TYPE_BYTE_8_GRAY` | `I` | `8` | static | 1.0 |  |
| 2 | `TYPE_BYTE_332_RGB` | `I` | `332` | static | 1.0 |  |
| 1 | `TYPE_USHORT_4444_ARGB` | `I` | `4444` | static | 1.0 |  |
| 2 | `TYPE_USHORT_444_RGB` | `I` | `444` | static | 1.0 |  |
| 2 | `TYPE_USHORT_555_RGB` | `I` | `555` | static | 1.0 |  |
| 2 | `TYPE_USHORT_1555_ARGB` | `I` | `1555` | static | 1.0 |  |
| 1 | `TYPE_USHORT_565_RGB` | `I` | `565` | static | 1.0 |  |
| 2 | `TYPE_INT_888_RGB` | `I` | `888` | static | 1.0 |  |
| 1 | `TYPE_INT_8888_ARGB` | `I` | `8888` | static | 1.0 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `void setARGBColor(int argbColor)` | `setARGBColor(I)V` |  | 1.0 |  | `0xAARRGGBB`; the alpha also applies to `Graphics` drawing until the next `setColor`. |
| 1 | `void drawImage(Image img, int x, int y, int anchor, int manipulation)` | `drawImage(Ljavax/microedition/lcdui/Image;IIII)V` | deprecated | 1.0 | `IllegalArgumentException` if anchor is not a legal value or manipulation is not supported; `NullPointerException` if img is null | `Graphics.drawImage` with a flip and rotate `manipulation` (0 for none). |
| 2 | `void drawTriangle(int x1, int y1, int x2, int y2, int x3, int y3, int argbColor)` | `drawTriangle(IIIIIII)V` |  | 1.0 |  |  |
| 1 | `void fillTriangle(int x1, int y1, int x2, int y2, int x3, int y3, int argbColor)` | `fillTriangle(IIIIIII)V` |  | 1.0 |  |  |
| 2 | `void drawPolygon(int[] xPoints, int xOffset, int[] yPoints, int yOffset, int nPoints, int argbColor)` | `drawPolygon([II[IIII)V` |  | 1.0 | `NullPointerException` if xPoints or yPoints array is null; `ArrayIndexOutOfBoundsException` if requested to access xPoints or yPoints beyond the length of the arrays or with a negative index | Arrays plus offsets: point `i` is `xPoints[xOffset + i]`, `yPoints[yOffset + i]`. |
| 1 | `void fillPolygon(int[] xPoints, int xOffset, int[] yPoints, int yOffset, int nPoints, int argbColor)` | `fillPolygon([II[IIII)V` |  | 1.0 | `NullPointerException` if xPoints or yPoints array is null; `ArrayIndexOutOfBoundsException` if requested to access xPoints or yPoints beyond the length of the arrays or with a negative index |  |
| 1 | `void drawPixels(int[] pixels, boolean transparency, int offset, int scanlength, int x, int y, int width, int height, int manipulation, int format)` | `drawPixels([IZIIIIIIII)V` |  | 1.0 | `NullPointerException` if pixels is null; `ArrayIndexOutOfBoundsException` if requested to access pixels beyond the length of array or with negative index; `IllegalArgumentException` if manipulation is unknown, width or height is negative, or the format is wrong or unsupported | Int formats only (`TYPE_INT_888_RGB`, `TYPE_INT_8888_ARGB`); `transparency` true means alpha is honored. Subject to clip and translation. |
| 2 | `void drawPixels(byte[] pixels, byte[] transparencyMask, int offset, int scanlength, int x, int y, int width, int height, int manipulation, int format)` | `drawPixels([B[BIIIIIIII)V` |  | 1.0 | `NullPointerException` if pixels is null; `ArrayIndexOutOfBoundsException` if requested to access beyond the length of array or with negative index; `IllegalArgumentException` if manipulation is unknown, width or height is negative, or the format is wrong or unsupported | Byte formats; the second array is an optional transparency mask. |
| 1 | `void drawPixels(short[] pixels, boolean transparency, int offset, int scanlength, int x, int y, int width, int height, int manipulation, int format)` | `drawPixels([SZIIIIIIII)V` |  | 1.0 | `NullPointerException` if pixels is null; `ArrayIndexOutOfBoundsException` if requested to access beyond the length of array or with negative index; `IllegalArgumentException` if manipulation is unknown, width or height is negative, or the format is wrong or unsupported | Short formats such as `TYPE_USHORT_4444_ARGB` and `TYPE_USHORT_565_RGB`. |
| 2 | `void getPixels(int[] pixels, int offset, int scanlength, int x, int y, int width, int height, int format)` | `getPixels([IIIIIIII)V` |  | 1.0 | `NullPointerException` if pixels is null; `ArrayIndexOutOfBoundsException` if array size is too small for the image pixels or negative index access is attempted, contents of the array remain unchanged; `IllegalArgumentException` if x, y, width or height is negative, or the format is wrong or unsupported | On-screen contexts report every pixel as opaque. |
| 3 | `void getPixels(byte[] pixels, byte[] transparencyMask, int offset, int scanlength, int x, int y, int width, int height, int format)` | `getPixels([B[BIIIIIII)V` |  | 1.0 | `NullPointerException` if pixels is null; `ArrayIndexOutOfBoundsException` if array size is too small for the image pixels or negative index access is attempted, contents of the array remain unchanged; `IllegalArgumentException` if x, y, width or height is negative, or the format is wrong or unsupported |  |
| 2 | `void getPixels(short[] pixels, int offset, int scanlength, int x, int y, int width, int height, int format)` | `getPixels([SIIIIIII)V` |  | 1.0 | `NullPointerException` if pixels is null; `ArrayIndexOutOfBoundsException` if array size is too small for the image pixels or negative index access is attempted, contents of the array remain unchanged; `IllegalArgumentException` if x, y, width or height is negative, or the format is wrong or unsupported |  |
| 1 | `int getNativePixelFormat()` | `getNativePixelFormat()I` |  | 1.0 |  | See the notes above. |
| 2 | `int getAlphaComponent()` | `getAlphaComponent()I` |  | 1.0 |  |  |

Added by a later edition ([nokiaapi2](https://nikita36078.github.io/J2ME_Docs/docs/nokiaapi2/com/nokia/mid/ui/DirectGraphics.html)):

| T | Member | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `FORM_ITEM_BORDER_TYPE` = `1` | `I` | static |  |  |  |
| 3 | `FORM_TEXT_FIELD_BORDER_TYPE` = `2` | `I` | static |  |  |  |
| 3 | `FORM_CHOICE_HIGHLIGHT_BORDER_TYPE` = `3` | `I` | static |  |  |  |
| 3 | `void drawElementBorder(int x, int y, int w, int h, int borderType, boolean withFocus)` | `drawElementBorder(IIIIIZ)V` |  | 1.0 | `IllegalArgumentException` if the specified border type is invalid |  |

### DirectUtils

`public class com.nokia.mid.ui.DirectUtils extends java.lang.Object`

Tier 1. Since Nokia UI 1.0. Javadoc: [Nokia_UI_API_1_1](https://nikita36078.github.io/J2ME_Docs/docs/Nokia_UI_API_1_1/com/nokia/mid/ui/DirectUtils.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/ui/DirectUtils.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/nokia/mid/ui/DirectUtils.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-nokiaui/src/main/java/com/nokia/mid/ui/DirectUtils.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `DirectGraphics getDirectGraphics(Graphics g)` | `getDirectGraphics(Ljavax/microedition/lcdui/Graphics;)Lcom/nokia/mid/ui/DirectGraphics;` | static | 1.0 |  | Returns a view of the same graphics context, not necessarily the same object. |
| 2 | `Image createImage(byte[] imageData, int imageOffset, int imageLength)` | `createImage([BII)Ljavax/microedition/lcdui/Image;` | static | 1.0 | `ArrayIndexOutOfBoundsException` if imageOffset and imageLength specify an invalid range; `NullPointerException` if imageData is null; `IllegalArgumentException` if imageData is incorrectly formatted or otherwise cannot be decoded | Like `Image.createImage(byte[], int, int)`, but the result is mutable. |
| 1 | `Image createImage(int width, int height, int ARGBcolor)` | `createImage(III)Ljavax/microedition/lcdui/Image;` | static | 1.0 | `IllegalArgumentException` if either width or height is zero or less | A mutable image filled with an ARGB color; `0x00000000` gives a fully transparent mutable image, which MIDP itself cannot make. Games draw sprites into it. |

Added by a later edition ([nokiaapi2](https://nikita36078.github.io/J2ME_Docs/docs/nokiaapi2/com/nokia/mid/ui/DirectUtils.html)):

| T | Member | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `STYLE_LIGHT` = `64` | `I` | static |  |  |  |
| 3 | `Font getFont(int face, int style, int height)` | `getFont(III)Ljavax/microedition/lcdui/Font;` | static |  | `IllegalArgumentException` if height is negative, if face or style are not legal values |  |
| 3 | `boolean setHeader(Displayable displayable, String headerText, Image headerImage, int headerTextColor, int headerBgColor, int headerDividerColor)` | `setHeader(Ljavax/microedition/lcdui/Displayable;Ljava/lang/String;Ljavax/microedition/lcdui/Image;III)Z` | static |  | `NullPointerException` if displayable is null; `IllegalArgumentException` if displayable is not a Canvas or Form |  |

### FullCanvas

`public abstract class com.nokia.mid.ui.FullCanvas extends javax.microedition.lcdui.Canvas`

Tier 1. Since Nokia UI 1.0. Javadoc: [Nokia_UI_API_1_1](https://nikita36078.github.io/J2ME_Docs/docs/Nokia_UI_API_1_1/com/nokia/mid/ui/FullCanvas.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/ui/FullCanvas.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/nokia/mid/ui/FullCanvas.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-nokiaui/src/main/java/com/nokia/mid/ui/FullCanvas.java).

A `Canvas` that always fills the screen, for MIDP 1.0 Nokia phones. Its constructor is deprecated, but MIDP 1.0 games subclass it.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 1 | `KEY_SOFTKEY1` | `I` | `-6` | static | 1.0 |  |
| 1 | `KEY_SOFTKEY2` | `I` | `-7` | static | 1.0 |  |
| 2 | `KEY_SEND` | `I` | `-10` | static | 1.0 |  |
| 2 | `KEY_END` | `I` | `-11` | static | 1.0 |  |
| 1 | `KEY_SOFTKEY3` | `I` | `-5` | static | 1.0 |  |
| 1 | `KEY_UP_ARROW` | `I` | `-1` | static | 1.0 |  |
| 1 | `KEY_DOWN_ARROW` | `I` | `-2` | static | 1.0 |  |
| 1 | `KEY_LEFT_ARROW` | `I` | `-3` | static | 1.0 |  |
| 1 | `KEY_RIGHT_ARROW` | `I` | `-4` | static | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` | protected deprecated | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void addCommand(Command cmd)` | `addCommand(Ljavax/microedition/lcdui/Command;)V` | deprecated | 1.0 | `IllegalStateException` always thrown | Commands are not allowed on a `FullCanvas`. |
| 3 | `void setCommandListener(CommandListener l)` | `setCommandListener(Ljavax/microedition/lcdui/CommandListener;)V` | deprecated | 1.0 | `IllegalStateException` always thrown |  |

Added by a later edition ([nokiaapi2](https://nikita36078.github.io/J2ME_Docs/docs/nokiaapi2/com/nokia/mid/ui/FullCanvas.html)):

| T | Member | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(boolean transparency)` | `<init>(Z)V` | protected |  |  |  |
| 3 | `boolean getTransparency()` | `getTransparency()Z` |  |  |  |  |

## com.nokia.mid.sound

### Sound

`public class com.nokia.mid.sound.Sound extends java.lang.Object`

Tier 1. Since Nokia UI 1.0. Javadoc: [Nokia_UI_API_1_1](https://nikita36078.github.io/J2ME_Docs/docs/Nokia_UI_API_1_1/com/nokia/mid/sound/Sound.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/sound/Sound.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/nokia/mid/sound/Sound.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-nokiaui/src/main/java/com/nokia/mid/sound/Sound.java).

Nokia's sound API from before MMAPI. Every phone plays tones and Nokia Smart Messaging ringtones (`FORMAT_TONE`); some also play WAV (`FORMAT_WAV`). Many MIDP 1.0 games use it for all their sound.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 1 | `FORMAT_TONE` | `I` | `1` | static deprecated | 1.0 |  |
| 1 | `FORMAT_WAV` | `I` | `5` | static deprecated | 1.0 |  |
| 1 | `SOUND_PLAYING` | `I` | `0` | static deprecated | 1.0 |  |
| 1 | `SOUND_STOPPED` | `I` | `1` | static deprecated | 1.0 |  |
| 2 | `SOUND_UNINITIALIZED` | `I` | `3` | static deprecated | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `(byte[] data, int type)` | `<init>([BI)V` | deprecated | 1.0 | `IllegalArgumentException` if the data can not be recognized as a given type or the type is unsupported or unknown; `NullPointerException` if the data is null |  |
| 2 | `(int freq, long duration)` | `<init>(IJ)V` | deprecated | 1.0 | `IllegalArgumentException` if parameter values are illegal, freq is not in given range or duration is negative or zero | A beep of the given frequency in Hz and duration in milliseconds. |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void init(int freq, long duration)` | `init(IJ)V` | deprecated | 1.0 | `IllegalArgumentException` if parameter values are illegal, freq is not in given range or duration is negative or zero |  |
| 2 | `void init(byte[] data, int type)` | `init([BI)V` | deprecated | 1.0 | `IllegalArgumentException` if the data can not be recognized as a given type or the type is unsupported or unknown; `NullPointerException` if the data is null |  |
| 1 | `int getState()` | `getState()I` | deprecated | 1.0 |  |  |
| 1 | `void play(int loop)` | `play(I)V` | deprecated | 1.0 | `IllegalArgumentException` if the loop value is negative, or if sound values/data is illegal or corrupt | Starts from the beginning; `loop` 0 loops forever, 1 plays once, and 255 or more may not play at all. Restarting a playing sound stops it first. |
| 1 | `void stop()` | `stop()V` | deprecated | 1.0 |  |  |
| 2 | `void resume()` | `resume()V` | deprecated | 1.0 |  |  |
| 2 | `void release()` | `release()V` | deprecated | 1.0 |  |  |
| 2 | `void setGain(int gain)` | `setGain(I)V` | deprecated | 1.0 |  | 0 to 255, clamped; 0 is silent. |
| 2 | `int getGain()` | `getGain()I` | deprecated | 1.0 |  |  |
| 2 | `int getConcurrentSoundCount(int type)` | `getConcurrentSoundCount(I)I` | static deprecated | 1.0 | `IllegalArgumentException` if the type is unsupported or unknown |  |
| 2 | `int[] getSupportedFormats()` | `getSupportedFormats()[I` | static deprecated | 1.0 |  | The format constants the phone plays; games use WAV only when 5 is in it. |
| 2 | `void setSoundListener(SoundListener listener)` | `setSoundListener(Lcom/nokia/mid/sound/SoundListener;)V` | deprecated | 1.0 |  |  |

### SoundListener

`public interface com.nokia.mid.sound.SoundListener`

Tier 2. Since Nokia UI 1.0. Javadoc: [Nokia_UI_API_1_1](https://nikita36078.github.io/J2ME_Docs/docs/Nokia_UI_API_1_1/com/nokia/mid/sound/SoundListener.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/sound/SoundListener.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/nokia/mid/sound/SoundListener.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-nokiaui/src/main/java/com/nokia/mid/sound/SoundListener.java).

`soundStateChanged` reports `SOUND_PLAYING`, `SOUND_STOPPED` and `SOUND_UNINITIALIZED`.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void soundStateChanged(Sound sound, int event)` | `soundStateChanged(Lcom/nokia/mid/sound/Sound;I)V` | deprecated | 1.0 |  |  |

## Later Nokia classes

The later Nokia UI API javadoc
([nokiaapi2](https://nikita36078.github.io/J2ME_Docs/docs/nokiaapi2/overview-summary.html))
adds many classes for Series 40 touch phones and Symbian, for example
`TextEditor`, `CanvasGraphicsItem`, `TactileFeedback`,
`com.nokia.mid.ui.orientation.Orientation`,
`com.nokia.mid.ui.multipointtouch.MultipointTouch` and
`com.nokia.mid.ui.gestures.*`. A few late touch games use them; they are
tier 3 and left for a later FastEmu version. J2ME Loader also implements
`com.nokia.mid.ui.Clipboard`, `SoftNotification`, `com.nokia.mid.m3d`
and `com.nokia.mid.payment` (in-app purchase); see its
[com/nokia](https://github.com/nikita36078/J2ME-Loader/tree/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia)
tree.

Nokia system properties (`com.nokia.*`) are in
[system-properties.md](system-properties.md).

## Cross-check

Every member above comes from the javadoc named first under Sources. The script that writes this file compared it with the stub and implementation sources listed under Sources: constant values, member lists and `static` flags. Member lists are compared only where the parsed source holds the implementation's whole class hierarchy. What differs is listed here; a class not listed matched everywhere it was compared.

- `com.nokia.mid.ui.DeviceControl`
  - microemu-nokiaui.jar also declares `<init>()V`
- `com.nokia.mid.ui.DirectUtils`
  - microemu-nokiaui.jar also declares `<init>()V`
- `com.nokia.mid.ui.FullCanvas`
  - microemu-nokiaui.jar also declares `getHeight()I`, `getWidth()I`
