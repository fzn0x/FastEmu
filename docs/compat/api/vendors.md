# Vendor APIs: Siemens, Samsung, Motorola, Sony Ericsson, LG

APIs that phone makers other than Nokia shipped for games, from their own
javadoc. No vendor API is needed by most games, so nothing here is tier
1. A game written for one vendor's phones usually needs that vendor's
classes to start at all (it links against them directly), and many
multi-vendor games probe with `Class.forName` and use what they find.
FastEmu should offer these classes on every profile, as J2ME Loader and
freej2me-plus do, and report the matching vendor in
`microedition.platform` when a profile imitates that vendor's phone.

Tiers in this file: 2 for the APIs that whole families of games were
built on (Siemens' game and color game packages and its sound), 3 for
everything else.

Vendor key codes (soft keys and arrows) are in `fastemu-core`'s
`KeyCodes` presets; vendor system properties are in
[system-properties.md](system-properties.md). Nokia is in
[nokia.md](nokia.md), Mascot Capsule 3D (used by Sony Ericsson and others)
in [jsr-later.md](jsr-later.md).

Siemens' javadoc names two classes outside its own set:
`com.siemens.mp.misc.NativeMem`, the superclass of `ExtendedImage`,
`GraphicObjectManager`, `Melody` and `com.siemens.mp.ui.Image`, which
declares nothing a game calls (J2ME Loader and MicroEmulator define it as
an empty public class); and the MIDP `Image` that
`com.siemens.mp.lcdui.Image` extends.

Column meanings and tier rules: [README](README.md).

## Sources

- [J2ME_Docs](https://github.com/nikita36078/J2ME_Docs/tree/fbfdfed7009369b6bf7a5cce39b8a13f178c2c69/docs) at commit `fbfdfed7009369b6bf7a5cce39b8a13f178c2c69`: a collection of vendor and JSR javadoc sets kept by the J2ME Loader author, browsable at [nikita36078.github.io/J2ME_Docs](https://nikita36078.github.io/J2ME_Docs/). The vendor javadoc sets used here: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/overview-summary.html), [Samsung_API_1_0](https://nikita36078.github.io/J2ME_Docs/docs/Samsung_API_1_0/overview-summary.html), [Motorola_Gaming_API](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_Gaming_API/index.html), [Motorola_FunLights_API_1_0](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_FunLights_API_1_0/index.html), [VibeLight](https://nikita36078.github.io/J2ME_Docs/docs/VibeLight/index.html) (Motorola `com.motorola.multimedia`), [Capuchin](https://nikita36078.github.io/J2ME_Docs/docs/Capuchin/index.html) (Sony Ericsson) and [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/overview-summary.html). Every member, throws entry and since tag comes from them.
- MicroEmulator 2.0.4 [microemu-siemensapi](https://repo1.maven.org/maven2/org/microemu/microemu-siemensapi/2.0.4/microemu-siemensapi-2.0.4.jar) jar (Maven Central): compiled class files, read for descriptors and `ConstantValue` attributes.
- [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/tree/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java) at commit `e270ed91e67b5cafc18c1991b92ba5249cf41c50` (Apache-2.0): implementation source, checked for constant values and member lists.
- [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/tree/aab3350832c0d82aff1771394d28bff6f66b0aa4/src) at commit `aab3350832c0d82aff1771394d28bff6f66b0aa4` (GPL-3.0): implementation source, checked for constant values and member lists.
- [MicroEmulator](https://github.com/barteo/microemu/tree/e47b656d983deaa67810c776662444483bdbf134/microemulator) at commit `e47b656d983deaa67810c776662444483bdbf134` (LGPL-2.1 or Apache-2.0): implementation source, checked for constant values and member lists.

## Classes

| Class | Kind | Tier | Since | Members |
|---|---|---|---|---|
| [ExtendedImage](#extendedimage) | class | 2 |  | 8 |
| [GraphicObject](#graphicobject) | class | 2 |  | 3 |
| [GraphicObjectManager](#graphicobjectmanager) | class | 2 |  | 10 |
| [Light](#light) | class | 2 |  | 3 |
| [Melody](#melody) | class | 2 |  | 2 |
| [MelodyComposer](#melodycomposer) | class | 2 |  | 91 |
| [Sound](#sound) | class | 2 |  | 2 |
| [com.siemens.mp.game.Sprite](#comsiemensmpgamesprite) | class | 2 |  | 11 |
| [TiledBackground](#tiledbackground) | class | 2 |  | 4 |
| [com.siemens.mp.game.Vibrator](#comsiemensmpgamevibrator) | class | 2 |  | 4 |
| [GameCanvas](#gamecanvas) | class | 2 |  | 17 |
| [Layer](#layer) | class | 2 |  | 17 |
| [LayerManager](#layermanager) | class | 2 |  | 10 |
| [com.siemens.mp.color_game.Sprite](#comsiemensmpcolor_gamesprite) | class | 2 |  | 15 |
| [TiledLayer](#tiledlayer) | class | 2 |  | 13 |
| [MIDlet](#midlet) | class | 3 |  | 6 |
| [NotAllowedException](#notallowedexception) | class | 3 |  | 2 |
| [Call](#call) | class | 3 |  | 2 |
| [PhoneBook](#phonebook) | class | 3 |  | 2 |
| [com.siemens.mp.gsm.SMS](#comsiemensmpgsmsms) | class | 3 |  | 2 |
| [Connection](#connection) | class | 3 |  | 4 |
| [ConnectionListener](#connectionlistener) | interface | 3 |  | 1 |
| [File](#file) | class | 3 |  | 21 |
| [com.siemens.mp.lcdui.Image](#comsiemensmplcduiimage) | class | 3 |  | 5 |
| [Ledcontrol](#ledcontrol) | class | 3 |  | 19 |
| [com.siemens.mp.ui.Image](#comsiemensmpuiimage) | class | 2 |  | 20 |
| [Control](#control) | interface | 3 |  | 0 |
| [Controllable](#controllable) | interface | 3 |  | 2 |
| [Manager](#manager) | class | 3 |  | 9 |
| [MediaException](#mediaexception) | class | 3 |  | 2 |
| [Player](#player) | interface | 3 |  | 22 |
| [PlayerListener](#playerlistener) | interface | 3 |  | 17 |
| [TimeBase](#timebase) | interface | 3 |  | 1 |
| [ToneControl](#tonecontrol) | interface | 3 |  | 11 |
| [VolumeControl](#volumecontrol) | interface | 3 |  | 4 |
| [AudioClip](#audioclip) | class | 3 |  | 10 |
| [LCDLight](#lcdlight) | class | 3 |  | 3 |
| [SM](#sm) | class | 3 |  | 8 |
| [com.samsung.util.SMS](#comsamsungutilsms) | class | 3 |  | 2 |
| [com.samsung.util.Vibration](#comsamsungutilvibration) | class | 3 |  | 3 |
| [BackgroundMusic](#backgroundmusic) | class | 3 |  | 1 |
| [FileFormatNotSupportedException](#fileformatnotsupportedexception) | class | 3 |  | 3 |
| [GameScreen](#gamescreen) | class | 3 |  | 27 |
| [ImageUtil](#imageutil) | class | 3 |  | 8 |
| [PaletteImage](#paletteimage) | class | 3 |  | 10 |
| [PlayField](#playfield) | class | 3 |  | 24 |
| [SoundEffect](#soundeffect) | class | 3 |  | 1 |
| [com.motorola.game.Sprite](#commotorolagamesprite) | class | 3 |  | 23 |
| [FunLight](#funlight) | class | 3 |  | 21 |
| [FunLightException](#funlightexception) | class | 3 |  | 2 |
| [Region](#region) | interface | 3 |  | 7 |
| [Lighting](#lighting) | class | 3 |  | 2 |
| [com.motorola.multimedia.Vibrator](#commotorolamultimediavibrator) | class | 3 |  | 15 |
| [ExternalResourceHandler](#externalresourcehandler) | interface | 3 |  | 1 |
| [FlashCanvas](#flashcanvas) | class | 3 |  | 2 |
| [FlashDataRequest](#flashdatarequest) | class | 3 |  | 4 |
| [FlashDataRequestListener](#flashdatarequestlistener) | interface | 3 |  | 1 |
| [FlashEventListener](#flasheventlistener) | class | 3 |  | 1 |
| [FlashEventManager](#flasheventmanager) | interface | 3 |  | 1 |
| [FlashImage](#flashimage) | class | 3 |  | 18 |
| [FlashPlayer](#flashplayer) | class | 3 |  | 5 |
| [SISAniImage](#sisaniimage) | class | 3 |  | 12 |
| [MathFP](#mathfp) | class | 3 | 1.0 | 28 |
| [BackLight](#backlight) | class | 3 |  | 5 |
| [Beep](#beep) | class | 3 |  | 39 |
| [LED](#led) | class | 3 |  | 4 |
| [MediaPlayer](#mediaplayer) | class | 3 |  | 12 |
| [mmpp.media.Vibration](#mmppmediavibration) | class | 3 |  | 3 |
| [PhraseData](#phrasedata) | class | 3 |  | 4 |
| [PhrasePlayer](#phraseplayer) | class | 3 |  | 9 |
| [PhraseTrack](#phrasetrack) | class | 3 |  | 32 |
| [PhraseTrackListener](#phrasetracklistener) | interface | 3 |  | 1 |
| [GraphicsX](#graphicsx) | class | 3 |  | 10 |
| [TextFieldX](#textfieldx) | class | 3 |  | 24 |
| [ContentsManager](#contentsmanager) | class | 3 |  | 10 |
| [Phone](#phone) | class | 3 |  | 9 |

## Siemens: com.siemens.mp.game

### ExtendedImage

`public class com.siemens.mp.game.ExtendedImage extends com.siemens.mp.misc.NativeMem`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/game/ExtendedImage.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/game/ExtendedImage.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/game/ExtendedImage.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/game/ExtendedImage.java).

Wraps a MIDP `Image` (passed to the constructor) to give pixel access to black and white, or black, white and transparent, bitmaps, and to draw straight to the display without `paint`.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `(Image image)` | `<init>(Ljavax/microedition/lcdui/Image;)V` |  | `IllegalArgumentException` if the width of the image is not a multiple of 8 |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `void setPixel(int x, int y, byte color)` | `setPixel(IIB)V` |  |  |  |
| 2 | `void setPixels(byte[] pixels, int x, int y, int width, int height)` | `setPixels([BIIII)V` |  |  |  |
| 2 | `int getPixel(int x, int y)` | `getPixel(II)I` |  |  |  |
| 2 | `void getPixelBytes(byte[] pixels, int x, int y, int width, int height)` | `getPixelBytes([BIIII)V` |  |  |  |
| 2 | `void clear(byte color)` | `clear(B)V` |  |  |  |
| 2 | `void blitToScreen(int x, int y)` | `blitToScreen(II)V` |  |  | Draws to the display at once, outside `paint`. |
| 2 | `Image getImage()` | `getImage()Ljavax/microedition/lcdui/Image;` |  |  |  |

### GraphicObject

`public class com.siemens.mp.game.GraphicObject extends com.siemens.mp.misc.NativeMem`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/game/GraphicObject.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/game/GraphicObject.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/game/GraphicObject.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/game/GraphicObject.java).

Abstract base of `Sprite` and `TiledBackground`; only `GraphicObjectManager` draws them.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` | protected |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `void setVisible(boolean visible)` | `setVisible(Z)V` |  |  |  |
| 2 | `boolean getVisible()` | `getVisible()Z` |  |  |  |

### GraphicObjectManager

`public class com.siemens.mp.game.GraphicObjectManager extends com.siemens.mp.misc.NativeMem`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/game/GraphicObjectManager.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/game/GraphicObjectManager.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/game/GraphicObjectManager.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/game/GraphicObjectManager.java).

Holds `Sprite`s and `TiledBackground`s and paints them in order, first to last, onto an `Image` or `ExtendedImage`.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `void addObject(GraphicObject gobject)` | `addObject(Lcom/siemens/mp/game/GraphicObject;)V` |  |  |  |
| 2 | `void insertObject(GraphicObject gobject, int position)` | `insertObject(Lcom/siemens/mp/game/GraphicObject;I)V` |  |  |  |
| 2 | `int getObjectPosition(GraphicObject gobject)` | `getObjectPosition(Lcom/siemens/mp/game/GraphicObject;)I` |  |  |  |
| 2 | `GraphicObject getObjectAt(int index)` | `getObjectAt(I)Lcom/siemens/mp/game/GraphicObject;` |  |  |  |
| 2 | `void deleteObject(GraphicObject gobject)` | `deleteObject(Lcom/siemens/mp/game/GraphicObject;)V` |  |  |  |
| 2 | `void deleteObject(int position)` | `deleteObject(I)V` |  |  |  |
| 2 | `void paint(Image image, int x, int y)` | `paint(Ljavax/microedition/lcdui/Image;II)V` |  | `IllegalArgumentException` if an image with transparent color is used Example |  |
| 2 | `void paint(ExtendedImage eimage, int x, int y)` | `paint(Lcom/siemens/mp/game/ExtendedImage;II)V` |  | `IllegalArgumentException` if an image with transparent color is used Example |  |
| 3 | `byte[] createTextureBits(int width, int height, byte[] texture)` | `createTextureBits(II[B)[B` | static |  |  |

### Light

`public class com.siemens.mp.game.Light extends java.lang.Object`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/game/Light.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/game/Light.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/game/Light.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/game/Light.java).

Backlight on and off.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `void setLightOn()` | `setLightOn()V` | static |  |  |
| 2 | `void setLightOff()` | `setLightOff()V` | static |  |  |

### Melody

`public class com.siemens.mp.game.Melody extends com.siemens.mp.misc.NativeMem`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/game/Melody.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/game/Melody.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/game/Melody.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/game/Melody.java).

A melody built with `MelodyComposer`; not created directly.

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `void play()` | `play()V` |  |  |  |
| 2 | `void stop()` | `stop()V` | static |  |  |

### MelodyComposer

`public class com.siemens.mp.game.MelodyComposer extends java.lang.Object`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/game/MelodyComposer.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/game/MelodyComposer.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/game/MelodyComposer.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/game/MelodyComposer.java).

Builds a melody from tone and length constants and a tempo. The many `TONE_*` and `TONELENGTH_*` constants are the composer's note codes.

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `TONE_C0` | `I` | `0` | static |  |
| 3 | `TONE_CIS0` | `I` | `1` | static |  |
| 3 | `TONE_D0` | `I` | `2` | static |  |
| 3 | `TONE_DIS0` | `I` | `3` | static |  |
| 3 | `TONE_E0` | `I` | `4` | static |  |
| 3 | `TONE_F0` | `I` | `5` | static |  |
| 3 | `TONE_FIS0` | `I` | `6` | static |  |
| 3 | `TONE_G0` | `I` | `7` | static |  |
| 3 | `TONE_GIS0` | `I` | `8` | static |  |
| 3 | `TONE_A0` | `I` | `9` | static |  |
| 3 | `TONE_AIS0` | `I` | `10` | static |  |
| 3 | `TONE_H0` | `I` | `11` | static |  |
| 3 | `TONE_C1` | `I` | `12` | static |  |
| 3 | `TONE_CIS1` | `I` | `13` | static |  |
| 3 | `TONE_D1` | `I` | `14` | static |  |
| 3 | `TONE_DIS1` | `I` | `15` | static |  |
| 3 | `TONE_E1` | `I` | `16` | static |  |
| 3 | `TONE_F1` | `I` | `17` | static |  |
| 3 | `TONE_FIS1` | `I` | `18` | static |  |
| 3 | `TONE_G1` | `I` | `19` | static |  |
| 3 | `TONE_GIS1` | `I` | `20` | static |  |
| 3 | `TONE_A1` | `I` | `21` | static |  |
| 3 | `TONE_AIS1` | `I` | `22` | static |  |
| 3 | `TONE_H1` | `I` | `23` | static |  |
| 3 | `TONE_C2` | `I` | `24` | static |  |
| 3 | `TONE_CIS2` | `I` | `25` | static |  |
| 3 | `TONE_D2` | `I` | `26` | static |  |
| 3 | `TONE_DIS2` | `I` | `27` | static |  |
| 3 | `TONE_E2` | `I` | `28` | static |  |
| 3 | `TONE_F2` | `I` | `29` | static |  |
| 3 | `TONE_FIS2` | `I` | `30` | static |  |
| 3 | `TONE_G2` | `I` | `31` | static |  |
| 3 | `TONE_GIS2` | `I` | `32` | static |  |
| 3 | `TONE_A2` | `I` | `33` | static |  |
| 3 | `TONE_AIS2` | `I` | `34` | static |  |
| 3 | `TONE_H2` | `I` | `35` | static |  |
| 3 | `TONE_C3` | `I` | `36` | static |  |
| 3 | `TONE_CIS3` | `I` | `37` | static |  |
| 3 | `TONE_D3` | `I` | `38` | static |  |
| 3 | `TONE_DIS3` | `I` | `39` | static |  |
| 3 | `TONE_E3` | `I` | `40` | static |  |
| 3 | `TONE_F3` | `I` | `41` | static |  |
| 3 | `TONE_FIS3` | `I` | `42` | static |  |
| 3 | `TONE_G3` | `I` | `43` | static |  |
| 3 | `TONE_GIS3` | `I` | `44` | static |  |
| 3 | `TONE_A3` | `I` | `45` | static |  |
| 3 | `TONE_AIS3` | `I` | `46` | static |  |
| 3 | `TONE_H3` | `I` | `47` | static |  |
| 3 | `TONE_C4` | `I` | `48` | static |  |
| 3 | `TONE_CIS4` | `I` | `49` | static |  |
| 3 | `TONE_D4` | `I` | `50` | static |  |
| 3 | `TONE_DIS4` | `I` | `51` | static |  |
| 3 | `TONE_E4` | `I` | `52` | static |  |
| 3 | `TONE_F4` | `I` | `53` | static |  |
| 3 | `TONE_FIS4` | `I` | `54` | static |  |
| 3 | `TONE_G4` | `I` | `55` | static |  |
| 3 | `TONE_GIS4` | `I` | `56` | static |  |
| 3 | `TONE_A4` | `I` | `57` | static |  |
| 3 | `TONE_PAUSE` | `I` | `58` | static |  |
| 3 | `NO_TONE` | `I` | `59` | static |  |
| 3 | `TONE_STOP` | `I` | `60` | static |  |
| 3 | `TONE_REPEAT` | `I` | `61` | static |  |
| 3 | `TONE_REPEV` | `I` | `62` | static |  |
| 3 | `TONE_REPON` | `I` | `63` | static |  |
| 3 | `TONE_MARK` | `I` | `64` | static |  |
| 3 | `TONE_REPEAT_MARK` | `I` | `65` | static |  |
| 3 | `TONE_REPEV_MARK` | `I` | `66` | static |  |
| 3 | `TONE_REPON_MARK` | `I` | `67` | static |  |
| 3 | `TONELENGTH_1_1` | `I` | `0` | static |  |
| 3 | `TONELENGTH_1_2` | `I` | `1` | static |  |
| 3 | `TONELENGTH_1_4` | `I` | `2` | static |  |
| 3 | `TONELENGTH_1_8` | `I` | `3` | static |  |
| 3 | `TONELENGTH_1_16` | `I` | `4` | static |  |
| 3 | `TONELENGTH_1_32` | `I` | `5` | static |  |
| 3 | `TONELENGTH_1_64` | `I` | `6` | static |  |
| 3 | `TONELENGTH_DOTTED_1_1` | `I` | `7` | static |  |
| 3 | `TONELENGTH_DOTTED_1_2` | `I` | `8` | static |  |
| 3 | `TONELENGTH_DOTTED_1_4` | `I` | `9` | static |  |
| 3 | `TONELENGTH_DOTTED_1_8` | `I` | `10` | static |  |
| 3 | `TONELENGTH_DOTTED_1_16` | `I` | `11` | static |  |
| 3 | `TONELENGTH_DOTTED_1_32` | `I` | `12` | static |  |
| 3 | `TONELENGTH_DOTTED_1_64` | `I` | `13` | static |  |
| 3 | `BPM` | `I` | `60` | static |  |

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  |  |  |
| 2 | `(int[] notes, int bpm)` | `<init>([II)V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `void setBPM(int bpm)` | `setBPM(I)V` |  |  |  |
| 2 | `void appendNote(int note, int length)` | `appendNote(II)V` |  | `Exception` |  |
| 2 | `Melody getMelody()` | `getMelody()Lcom/siemens/mp/game/Melody;` |  |  |  |
| 2 | `void resetMelody()` | `resetMelody()V` |  |  |  |
| 3 | `int length()` | `length()I` |  |  |  |
| 3 | `int maxLength()` | `maxLength()I` | static |  |  |

### Sound

`public class com.siemens.mp.game.Sound extends java.lang.Object`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/game/Sound.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/game/Sound.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/game/Sound.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/game/Sound.java).

A beep: `playTone(frequency, milliseconds)`.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `void playTone(int tone_freq, int tone_time)` | `playTone(II)V` | static |  |  |

### com.siemens.mp.game.Sprite

`public class com.siemens.mp.game.Sprite extends com.siemens.mp.game.GraphicObject`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/game/Sprite.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/game/Sprite.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/game/Sprite.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/game/Sprite.java).

Frames are bitmaps with a mask; position, frame and collision rectangle. Drawn by `GraphicObjectManager`.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `(Image pixels, Image mask, int numFrames)` | `<init>(Ljavax/microedition/lcdui/Image;Ljavax/microedition/lcdui/Image;I)V` |  | `IllegalArgumentException` if the width of any image is not a multiple of 8, if there is a wrong number of frames, if any javax.microedition.lcdui.Image with transparent color is used, if the image and the mask do not have the same size, if the height of the image is not divisible by the number of frames Example |  |
| 2 | `(ExtendedImage pixels, ExtendedImage mask, int numFrames)` | `<init>(Lcom/siemens/mp/game/ExtendedImage;Lcom/siemens/mp/game/ExtendedImage;I)V` |  | `IllegalArgumentException` if the width of any image is not a multiple of 8, if there is a wrong number of frames, if any com.siemens.mp.game.ExtendedImage with transparent color is used, if the image and the mask do not have the same size, if the height of the image is not divisible by the number of frames Example |  |
| 2 | `(byte[] pixels, int pixel_offset, int width, int height, byte[] mask, int mask_offset, int numFrames)` | `<init>([BIII[BII)V` |  | `IllegalArgumentException` if the width is not a multiple of 8 or if there is a wrong number of frames or if the size of the pixelarray or maskarray is not equals (width*height*numFrames)/8 |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `void setPosition(int x, int y)` | `setPosition(II)V` |  |  |  |
| 2 | `int getXPosition()` | `getXPosition()I` |  |  |  |
| 2 | `int getYPosition()` | `getYPosition()I` |  |  |  |
| 2 | `void setCollisionRectangle(int x, int y, int width, int height)` | `setCollisionRectangle(IIII)V` |  |  |  |
| 2 | `boolean isCollidingWith(Sprite other)` | `isCollidingWith(Lcom/siemens/mp/game/Sprite;)Z` |  |  |  |
| 2 | `boolean isCollidingWithPos(int xpos, int ypos)` | `isCollidingWithPos(II)Z` |  |  |  |
| 2 | `void setFrame(int framenumber)` | `setFrame(I)V` |  |  |  |
| 2 | `int getFrame()` | `getFrame()I` |  |  |  |

### TiledBackground

`public class com.siemens.mp.game.TiledBackground extends com.siemens.mp.game.GraphicObject`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/game/TiledBackground.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/game/TiledBackground.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/game/TiledBackground.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/game/TiledBackground.java).

8 by 8 pixel tiles; map index 0 is transparent, 1 white, 2 black, and user tiles start at 3. A map smaller than the screen repeats.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `(Image tilePixels, Image tileMask, byte[] map, int widthInTiles, int heightInTiles)` | `<init>(Ljavax/microedition/lcdui/Image;Ljavax/microedition/lcdui/Image;[BII)V` |  | `IllegalArgumentException` if the width of the image/mask is not 8 if transparent color is used in the image/mask if there is a wrong tile number in the map Example |  |
| 2 | `(ExtendedImage tilePixels, ExtendedImage tileMask, byte[] map, int widthInTiles, int heightInTiles)` | `<init>(Lcom/siemens/mp/game/ExtendedImage;Lcom/siemens/mp/game/ExtendedImage;[BII)V` |  | `IllegalArgumentException` if the width of the image/mask is not 8 if transparent color is used in the image/mask if there is a wrong tile number in the map Example |  |
| 2 | `(byte[] tilePixels, byte[] tileMask, byte[] map, int widthInTiles, int heightInTiles)` | `<init>([B[B[BII)V` |  | `IllegalArgumentException` if the size of the tiles/mask array is not a multiple of 8 if there is a wrong tile number in the map |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `void setPositionInMap(int x, int y)` | `setPositionInMap(II)V` |  |  |  |

### com.siemens.mp.game.Vibrator

`public class com.siemens.mp.game.Vibrator extends java.lang.Object`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/game/Vibrator.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/game/Vibrator.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/game/Vibrator.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/game/Vibrator.java).

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `void triggerVibrator(int duration)` | `triggerVibrator(I)V` | static |  |  |
| 2 | `void startVibrator()` | `startVibrator()V` | static |  |  |
| 2 | `void stopVibrator()` | `stopVibrator()V` | static |  |  |

## Siemens: com.siemens.mp.color_game

### GameCanvas

`public abstract class com.siemens.mp.color_game.GameCanvas extends javax.microedition.lcdui.Canvas`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/color_game/GameCanvas.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/color_game/GameCanvas.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/color_game/GameCanvas.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/color_game/GameCanvas.java).

Siemens' copy of MIDP 2.0's `GameCanvas` for its MIDP 1.0 color phones. Behaves like `javax.microedition.lcdui.game.GameCanvas` ([midp-lcdui-game.md](midp-lcdui-game.md)); an implementation can share the code.

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 2 | `UP_PRESSED` | `I` | `2` | static |  |
| 2 | `DOWN_PRESSED` | `I` | `64` | static |  |
| 2 | `LEFT_PRESSED` | `I` | `4` | static |  |
| 2 | `RIGHT_PRESSED` | `I` | `32` | static |  |
| 2 | `FIRE_PRESSED` | `I` | `256` (`0x100`) | static |  |
| 2 | `GAME_A_PRESSED` | `I` | `512` (`0x200`) | static |  |
| 2 | `GAME_B_PRESSED` | `I` | `1024` (`0x400`) | static |  |
| 2 | `GAME_C_PRESSED` | `I` | `2048` (`0x800`) | static |  |
| 2 | `GAME_D_PRESSED` | `I` | `4096` (`0x1000`) | static |  |

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `(boolean suppressKeyEvents)` | `<init>(Z)V` | protected | `OutOfMemoryError` if there is insufficient memory available to create this GameCanvas and its off-screen buffer |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `void flushGraphics()` | `flushGraphics()V` |  |  |  |
| 2 | `void flushGraphics(int x, int y, int width, int height)` | `flushGraphics(IIII)V` |  |  |  |
| 2 | `Graphics getGraphics()` | `getGraphics()Ljavax/microedition/lcdui/Graphics;` | protected |  |  |
| 2 | `int getKeyStates()` | `getKeyStates()I` |  |  |  |
| 2 | `void keyPressed(int keyCode)` | `keyPressed(I)V` | protected |  |  |
| 2 | `void keyRepeated(int keyCode)` | `keyRepeated(I)V` | protected |  |  |
| 2 | `void paint(Graphics g)` | `paint(Ljavax/microedition/lcdui/Graphics;)V` |  |  |  |

### Layer

`public abstract class com.siemens.mp.color_game.Layer extends java.lang.Object`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/color_game/Layer.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/color_game/Layer.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/color_game/Layer.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/color_game/Layer.java).

Like MIDP 2.0's `Layer`, with a few extra members of Siemens' own.

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 2 | `NativeMemoryTable` | `Lcom/siemens/mp/misc/NativeMem;` | (not a constant) | protected not final |  |

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `(int width, int height)` | `<init>(II)V` | protected | `IllegalArgumentException` if width or height is less than zero |  |
| 2 | `(Image image)` | `<init>(Ljavax/microedition/lcdui/Image;)V` | protected | `NullPointerException` if the image is NULL |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `Image getLayerImage()` | `getLayerImage()Ljavax/microedition/lcdui/Image;` | protected |  |  |
| 2 | `void setLayerImage(Image image)` | `setLayerImage(Ljavax/microedition/lcdui/Image;)V` | protected | `NullPointerException` |  |
| 2 | `void copyAllLayerVariables(Layer l)` | `copyAllLayerVariables(Lcom/siemens/mp/color_game/Layer;)V` | protected |  |  |
| 2 | `int getHeight()` | `getHeight()I` | final |  |  |
| 2 | `int getWidth()` | `getWidth()I` | final |  |  |
| 2 | `int getX()` | `getX()I` | final |  |  |
| 2 | `int getY()` | `getY()I` | final |  |  |
| 2 | `boolean isVisible()` | `isVisible()Z` | final |  |  |
| 2 | `void move(int dx, int dy)` | `move(II)V` |  |  |  |
| 2 | `void paint(Graphics g)` | `paint(Ljavax/microedition/lcdui/Graphics;)V` | abstract | `NullPointerException` if g is null |  |
| 2 | `void setHeight(int height)` | `setHeight(I)V` |  | `IllegalArgumentException` if the specified height is less than 0 or greater than original image |  |
| 2 | `void setPosition(int x, int y)` | `setPosition(II)V` |  |  |  |
| 2 | `void setVisible(boolean visible)` | `setVisible(Z)V` |  |  |  |
| 2 | `void setWidth(int width)` | `setWidth(I)V` |  | `IllegalArgumentException` if the specified width is less than 0 or greater than original image |  |

### LayerManager

`public class com.siemens.mp.color_game.LayerManager extends java.lang.Object`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/color_game/LayerManager.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/color_game/LayerManager.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/color_game/LayerManager.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/color_game/LayerManager.java).

Like MIDP 2.0's `LayerManager`.

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 2 | `NativeMemoryTable` | `Lcom/siemens/mp/misc/NativeMem;` | (not a constant) | protected not final |  |

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `void append(Layer l)` | `append(Lcom/siemens/mp/color_game/Layer;)V` |  | `NullPointerException` if the Layer l is null |  |
| 2 | `Layer getLayerAt(int index)` | `getLayerAt(I)Lcom/siemens/mp/color_game/Layer;` |  | `IndexOutOfBoundsException` if the specified index is less than zero, or if it is equal to or greater than the number of Layers added to the this LayerManager |  |
| 2 | `int getSize()` | `getSize()I` |  |  |  |
| 2 | `void insert(Layer l, int index)` | `insert(Lcom/siemens/mp/color_game/Layer;I)V` |  | `NullPointerException` if the Layer is null; `IndexOutOfBoundsException` if the index is less than 0 or greater than the number of Layers already added to the this LayerManager |  |
| 2 | `void paint(Graphics g, int x, int y)` | `paint(Ljavax/microedition/lcdui/Graphics;II)V` |  | `NullPointerException` if g is null |  |
| 2 | `void remove(Layer l)` | `remove(Lcom/siemens/mp/color_game/Layer;)V` |  |  |  |
| 2 | `void removeLayer(NativeMem NativeMemoryTable, Layer l)` | `removeLayer(Lcom/siemens/mp/misc/NativeMem;Lcom/siemens/mp/color_game/Layer;)V` |  |  |  |
| 2 | `void setViewWindow(int x, int y, int width, int height)` | `setViewWindow(IIII)V` |  | `IllegalArgumentException` if the width or height is less than 0 |  |

### com.siemens.mp.color_game.Sprite

`public class com.siemens.mp.color_game.Sprite extends com.siemens.mp.color_game.Layer`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/color_game/Sprite.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/color_game/Sprite.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/color_game/Sprite.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/color_game/Sprite.java).

Like MIDP 2.0's `Sprite`, without the reference pixel and transforms.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `(Image image)` | `<init>(Ljavax/microedition/lcdui/Image;)V` |  | `NullPointerException` if image is null |  |
| 2 | `(Image image, int frameWidth, int frameHeight)` | `<init>(Ljavax/microedition/lcdui/Image;II)V` |  | `NullPointerException` if image is null; `IllegalArgumentException` if frameHeight or frameWidth is less than 1; `IllegalArgumentException` if the image width is not an integer multiple of the frameWidth; `IllegalArgumentException` if the image height is not an integer multiple of the frameHeight |  |
| 2 | `(Sprite s)` | `<init>(Lcom/siemens/mp/color_game/Sprite;)V` |  | `NullPointerException` if s is null |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `boolean collidesWith(Sprite s, boolean pixelLevel)` | `collidesWith(Lcom/siemens/mp/color_game/Sprite;Z)Z` |  |  |  |
| 2 | `boolean collidesWith(Image image, int x, int y, boolean pixelLevel)` | `collidesWith(Ljavax/microedition/lcdui/Image;IIZ)Z` |  |  |  |
| 2 | `int getFrame()` | `getFrame()I` | final |  |  |
| 2 | `int getFrameSequenceLength()` | `getFrameSequenceLength()I` |  |  |  |
| 2 | `int getRawFrameCount()` | `getRawFrameCount()I` |  |  |  |
| 2 | `void nextFrame()` | `nextFrame()V` |  |  |  |
| 2 | `void paint(Graphics g)` | `paint(Ljavax/microedition/lcdui/Graphics;)V` | final | `NullPointerException` if g is null |  |
| 2 | `void prevFrame()` | `prevFrame()V` |  |  |  |
| 2 | `void setCollisionRectangle(int x, int y, int width, int height)` | `setCollisionRectangle(IIII)V` |  | `IllegalArgumentException` if the specified width or height is less than 0 |  |
| 2 | `void setFrame(int sequenceIndex)` | `setFrame(I)V` |  | `IndexOutOfBoundsException` if frameIndex is less than 0; `IndexOutOfBoundsException` if frameIndex is equal to or greater than the length of the current frame sequence (or the number of raw frames for the default sequence) |  |
| 2 | `void setFrameSequence(int[] sequence)` | `setFrameSequence([I)V` |  | `ArrayIndexOutOfBoundsException` if seq is non-null and any member of the array has a value less than 0 or greater than or equal to the number of frames as reported by getRawFrameCount(); `IllegalArgumentException` if the array has less than 1 element |  |
| 2 | `void setImage(Image image, int frameWidth, int frameHeight)` | `setImage(Ljavax/microedition/lcdui/Image;II)V` |  | `NullPointerException` if image is null; `IllegalArgumentException` if frameHeight or frameWidth is less than 1; `IllegalArgumentException` if the image width is not an integer multiple of the frameWidth; `IllegalArgumentException` if the image height is not an integer multiple of the frameHeight |  |

### TiledLayer

`public class com.siemens.mp.color_game.TiledLayer extends com.siemens.mp.color_game.Layer`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/color_game/TiledLayer.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/color_game/TiledLayer.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/color_game/TiledLayer.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/color_game/TiledLayer.java).

Like MIDP 2.0's `TiledLayer`.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `(int columns, int rows, Image image, int tileWidth, int tileHeight)` | `<init>(IILjavax/microedition/lcdui/Image;II)V` |  | `NullPointerException` if image is null; `IllegalArgumentException` if the number of rows or columns is less than 1; `IllegalArgumentException` if tileHeight or tileWidth is less than 1; `IllegalArgumentException` if the image width is not an integer multiple of the tileWidth; `IllegalArgumentException` if the image height is not an integer multiple of the tileHeight; `IllegalArgumentException` if the resulting number if tiles exceeds 32,767; `OutOfMemoryError` if the request exceeds the resource availability of the device |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `int createAnimatedTile(int staticTileIndex)` | `createAnimatedTile(I)I` |  | `IndexOutOfBoundsException` If the staticTileIndex is invalid; `IllegalArgumentException` if the request will result in more than 32,767 animated tiles |  |
| 2 | `void fillCells(int col, int row, int width, int height, int tileIndex)` | `fillCells(IIIII)V` |  | `IndexOutOfBoundsException` if the rectangular region defined by the parameters extends beyond the bounds of the TiledLayer grid; `IndexOutOfBoundsException` if there is no tile with index tileIndex |  |
| 2 | `int getAnimatedTile(int animatedTileIndex)` | `getAnimatedTile(I)I` |  | `IndexOutOfBoundsException` If the animated tile index is invalid |  |
| 2 | `int getCell(int col, int row)` | `getCell(II)I` |  | `IndexOutOfBoundsException` if row or col is outside the bounds of the TiledLayer grid |  |
| 2 | `int getCellHeight()` | `getCellHeight()I` | final |  |  |
| 2 | `int getCellWidth()` | `getCellWidth()I` | final |  |  |
| 2 | `int getColumns()` | `getColumns()I` | final |  |  |
| 2 | `int getRows()` | `getRows()I` | final |  |  |
| 2 | `void paint(Graphics g)` | `paint(Ljavax/microedition/lcdui/Graphics;)V` | final | `NullPointerException` if g is null |  |
| 2 | `void setAnimatedTile(int animatedTileIndex, int staticTileIndex)` | `setAnimatedTile(II)V` |  |  |  |
| 2 | `void setCell(int col, int row, int tileIndex)` | `setCell(III)V` |  | `IndexOutOfBoundsException` if there is no tile with index tileIndex; `IndexOutOfBoundsException` if row or col is outside the bounds of the TiledLayer grid |  |
| 2 | `void setStaticTileSet(Image image, int tileWidth, int tileHeight)` | `setStaticTileSet(Ljavax/microedition/lcdui/Image;II)V` |  | `NullPointerException` if image is null; `IllegalArgumentException` if tileHeight or tileWidth is less than 1; `IllegalArgumentException` if the image width is not an integer multiple of the tileWidth; `IllegalArgumentException` if the image height is not an integer multiple of the tileHeight; `IllegalArgumentException` if the resulting number if tiles exceeds 32,767 |  |

## Siemens: other packages

### MIDlet

`public class com.siemens.mp.MIDlet extends java.lang.Object`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/MIDlet.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/MIDlet.java).

The javadoc calls it a wrapper for a native widget implementation, only on the MC60.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void notifyDestroyed()` | `notifyDestroyed()V` | static |  |  |
| 3 | `void notifyPaused()` | `notifyPaused()V` | static |  |  |
| 3 | `String getAppProperty(String key)` | `getAppProperty(Ljava/lang/String;)Ljava/lang/String;` | static |  |  |
| 3 | `boolean platformRequest(String locator)` | `platformRequest(Ljava/lang/String;)Z` | static final | `ConnectionNotFoundException`; `NotAllowedException` |  |
| 3 | `String[] getSupportedProtocols()` | `getSupportedProtocols()[Ljava/lang/String;` | static |  |  |

### NotAllowedException

`public class com.siemens.mp.NotAllowedException extends java.lang.Exception`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/NotAllowedException.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/NotAllowedException.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/NotAllowedException.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/NotAllowedException.java).

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  |  |  |
| 3 | `(String msg)` | `<init>(Ljava/lang/String;)V` |  |  |  |

### Call

`public class com.siemens.mp.gsm.Call extends java.lang.Object`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/gsm/Call.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/gsm/Call.java).

Starts a phone call.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void start(String number)` | `start(Ljava/lang/String;)V` | static | `IllegalArgumentException` if a phone number is entered in the wrong format or is missing in method call; `NotAllowedException` if the call is not allowed Example |  |

### PhoneBook

`public class com.siemens.mp.gsm.PhoneBook extends java.lang.Object`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/gsm/PhoneBook.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/gsm/PhoneBook.java).

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `String[] getMDN()` | `getMDN()[Ljava/lang/String;` | static | `NotAllowedException` if memory allocation is not possible Example |  |

### com.siemens.mp.gsm.SMS

`public class com.siemens.mp.gsm.SMS extends java.lang.Object`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/gsm/SMS.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/gsm/SMS.java).

Sends a text message. FastEmu must never send anything; returning as if sent, or failing, are both safer.

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `int send(String number, String data)` | `send(Ljava/lang/String;Ljava/lang/String;)I` | static | `IllegalArgumentException` the phone number or the text message is missing in method call, or format of the number is incorrect; `NotAllowedException` if the transfer is not allowed; `IOException` if the network is not available Example |  |
| 3 | `int send(String number, byte[] data)` | `send(Ljava/lang/String;[B)I` | static | `IllegalArgumentException` the phone number or the data is missing in method call, or format of the number is incorrect; `NotAllowedException` if the transfer is not allowed; `IOException` if the network is not available Example |  |

### Connection

`public class com.siemens.mp.io.Connection extends com.siemens.mp.misc.NativeMem`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/io/Connection.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/io/Connection.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/io/Connection.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/io/Connection.java).

Data over SMS.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `(String connectTo)` | `<init>(Ljava/lang/String;)V` |  | `IllegalArgumentException` if SMS connection is used, and the number is not a correct phone-number Example |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void send(byte[] data)` | `send([B)V` |  | `NotAllowedException` if SMS connection is used and the call is not acknowledged |  |
| 3 | `void setListener(ConnectionListener listener)` | `setListener(Lcom/siemens/mp/io/ConnectionListener;)V` |  |  |  |
| 3 | `ConnectionListener getListener()` | `getListener()Lcom/siemens/mp/io/ConnectionListener;` |  |  |  |

### ConnectionListener

`public interface com.siemens.mp.io.ConnectionListener`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/io/ConnectionListener.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/io/ConnectionListener.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/io/ConnectionListener.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/io/ConnectionListener.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void receiveData(byte[] data)` | `receiveData([B)V` |  |  |  |

### File

`public class com.siemens.mp.io.File extends java.lang.Object`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/io/File.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/io/File.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/io/File.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/io/File.java).

Files in the MIDlet's own storage folder on the phone; some games keep saves here instead of in RMS. Paths are relative to that folder and `..` is forbidden; FastEmu maps the folder into the instance's data directory.

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `OUTSIDE_STORAGE_PATH` | `I` | `0` | static |  |
| 3 | `INSIDE_STORAGE_PATH` | `I` | `1` | static |  |
| 3 | `STORAGE_DRIVE` | `Ljava/lang/String;` | `"a:"` | static |  |

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `int checkFileName(String fileName)` | `checkFileName(Ljava/lang/String;)I` | static | `IllegalArgumentException` if specified file name violates security policies |  |
| 3 | `int debugWrite(String fileName, String infoString)` | `debugWrite(Ljava/lang/String;Ljava/lang/String;)I` | static | `IOException` if some other kind of I/O error occurs. Example |  |
| 3 | `int open(String fileName)` | `open(Ljava/lang/String;)I` |  | `IllegalArgumentException` if the path is invalid; `IOException` if some other kind of I/O error occurs. Example |  |
| 3 | `int exists(String fileName)` | `exists(Ljava/lang/String;)I` | static | `IllegalArgumentException` if the path is invalid |  |
| 3 | `int seek(int fileDescriptor, int seekpos)` | `seek(II)I` |  | `IOException` if some kind of I/O error occurs |  |
| 3 | `int length(int fileDescriptor)` | `length(I)I` |  | `IOException` |  |
| 3 | `int close(int fileDescriptor)` | `close(I)I` |  | `IOException` if some kind of I/O error occurs |  |
| 3 | `int write(int fileDescriptor, byte[] buf, int offset, int numBytes)` | `write(I[BII)I` |  | `IOException` if some kind of I/O error occurs |  |
| 3 | `int read(int fileDescriptor, byte[] buf, int offset, int numBytes)` | `read(I[BII)I` |  | `IOException` if some kind of I/O error occurs |  |
| 3 | `int delete(String fileName)` | `delete(Ljava/lang/String;)I` | static | `IllegalArgumentException` if the path is invalid; `IOException` if some other kind of I/O error occurs |  |
| 3 | `int spaceAvailable()` | `spaceAvailable()I` | static | `IOException` if some kind of I/O error occurs. Example |  |
| 3 | `int rename(String source, String dest)` | `rename(Ljava/lang/String;Ljava/lang/String;)I` | static | `IllegalArgumentException` if the path is invalid; `IOException` if some other kind of I/O error occurs |  |
| 3 | `int copy(String source, String dest)` | `copy(Ljava/lang/String;Ljava/lang/String;)I` | static | `IllegalArgumentException` if the path is invalid; `IOException` if some other kind of I/O error occurs |  |
| 3 | `boolean isDirectory(String pathName)` | `isDirectory(Ljava/lang/String;)Z` | static | `IllegalArgumentException` if the path is invalid; `IOException` if some kind of I/O error occurs |  |
| 3 | `String[] list(String pathName)` | `list(Ljava/lang/String;)[Ljava/lang/String;` | static | `IllegalArgumentException` if the path is invalid; `IOException` if some kind of I/O error occurs |  |
| 3 | `String buildPath(String fileName)` | `buildPath(Ljava/lang/String;)Ljava/lang/String;` | static |  |  |
| 3 | `void truncate(int fileDescriptor, int size)` | `truncate(II)V` | static | `IOException` if some kind of I/O error occurs |  |

### com.siemens.mp.lcdui.Image

`public class com.siemens.mp.lcdui.Image extends com.siemens.mp.ui.Image`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/lcdui/Image.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/lcdui/Image.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/lcdui/Image.java).

MIDP `Image` with file and pixel helpers; the javadoc says only the MC60 has it and other phones use `com.siemens.mp.ui.Image`.

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void writeBmpToFile(Image image, String filename)` | `writeBmpToFile(Ljavax/microedition/lcdui/Image;Ljava/lang/String;)V` | static | `IOException` in case of I/O problems |  |
| 3 | `Image createImageFromFile(String filename, boolean ScaleToFullScreen)` | `createImageFromFile(Ljava/lang/String;Z)Ljavax/microedition/lcdui/Image;` | static | `IOException` in case of I/O problems |  |
| 3 | `Image createImageFromFile(String filename, int ScaleToWidth, int ScaleToHeight)` | `createImageFromFile(Ljava/lang/String;II)Ljavax/microedition/lcdui/Image;` | static | `IOException` in case of I/O problems |  |
| 3 | `int getPixelColor(Image image, int x, int y)` | `getPixelColor(Ljavax/microedition/lcdui/Image;II)I` | static | `IllegalArgumentException` if x or y are out of the image |  |
| 3 | `void setPixelColor(Image image, int x, int y, int color)` | `setPixelColor(Ljavax/microedition/lcdui/Image;III)V` | static | `IllegalArgumentException` if x or y are out of the image |  |

### Ledcontrol

`public class com.siemens.mp.m55.Ledcontrol extends java.lang.Object`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/m55/Ledcontrol.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/m55/Ledcontrol.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/m55/Ledcontrol.java).

The M55's colored LEDs.

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `LED_TOP` | `I` | `0` | static |  |
| 3 | `LED_BOTTOM` | `I` | `1` | static |  |
| 3 | `P_SPEED` | `I` | `0` | static |  |
| 3 | `P_LIGHTHOUSE` | `I` | `1` | static |  |
| 3 | `P_WAVE` | `I` | `2` | static |  |
| 3 | `P_BEAT` | `I` | `3` | static |  |
| 3 | `P_RUNWAY` | `I` | `4` | static |  |
| 3 | `P_TRANCE` | `I` | `5` | static |  |
| 3 | `P_ETERNITY` | `I` | `6` | static |  |
| 3 | `P_STROBO` | `I` | `7` | static |  |
| 3 | `P_LIMELIGHT` | `I` | `8` | static |  |
| 3 | `P_NORMAL_BLINKING` | `I` | `9` | static |  |
| 3 | `P_PULSATING` | `I` | `10` | static |  |
| 3 | `P_CONSTANLY_LITEUP` | `I` | `11` | static |  |
| 3 | `P_IDLE` | `I` | `12` | static |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void switchON(int led)` | `switchON(I)V` | static |  |  |
| 3 | `void playPattern(int pattern)` | `playPattern(I)V` | static |  |  |
| 3 | `void stopPattern()` | `stopPattern()V` | static |  |  |
| 3 | `void switchOFF(int led)` | `switchOFF(I)V` | static |  |  |

### com.siemens.mp.ui.Image

`public class com.siemens.mp.ui.Image extends com.siemens.mp.misc.NativeMem`

Tier 2. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/ui/Image.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/ui/Image.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/ui/Image.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/ui/Image.java).

Static helpers that create MIDP images from Siemens bitmaps, scale and mirror them.

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 2 | `COLOR_BMP_8BIT` | `I` | `5` | static |  |

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` | protected |  |  |
| 2 | `(int imageWidth, int imageHeight)` | `<init>(II)V` |  |  |  |
| 2 | `(Image image)` | `<init>(Lcom/siemens/mp/ui/Image;)V` |  |  |  |
| 2 | `(byte[] bytes, int imageWidth, int imageHeight)` | `<init>([BII)V` |  |  |  |
| 2 | `(byte[] bytes, int imageWidth, int imageHeight, boolean transparent)` | `<init>([BIIZ)V` |  |  |  |
| 2 | `(String name, boolean doScale)` | `<init>(Ljava/lang/String;Z)V` |  | `IOException` |  |
| 2 | `(byte[] imageData)` | `<init>([B)V` |  |  |  |
| 2 | `(byte[] bytes, int imageWidth, int imageHeight, int BitmapType)` | `<init>([BIII)V` |  | `IOException` if the image type is not supported |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 2 | `int getHeight()` | `getHeight()I` |  |  |  |
| 2 | `int getWidth()` | `getWidth()I` |  |  |  |
| 2 | `Image createImageWithScaling(String name)` | `createImageWithScaling(Ljava/lang/String;)Ljavax/microedition/lcdui/Image;` | static | `IOException` if the resource does not exist, the data cannot be loaded, or the image data cannot be decoded |  |
| 2 | `Image createImageWithoutScaling(String name)` | `createImageWithoutScaling(Ljava/lang/String;)Ljavax/microedition/lcdui/Image;` | static | `IOException` if the resource does not exist, the data cannot be loaded, or the image data cannot be decoded |  |
| 2 | `Image createImageFromBitmap(byte[] imageData, int imageWidth, int imageHeight)` | `createImageFromBitmap([BII)Ljavax/microedition/lcdui/Image;` | static |  |  |
| 2 | `Image createRGBImage(byte[] imageData, int imageWidth, int imageHeight, int BitmapType)` | `createRGBImage([BIII)Ljavax/microedition/lcdui/Image;` | static | `IOException` if the image type is not supported; `ArrayIndexOutOfBoundsException` if width or height is equal or less than zero |  |
| 2 | `Image createTransparentImageFromBitmap(byte[] bytes, int width, int height)` | `createTransparentImageFromBitmap([BII)Ljavax/microedition/lcdui/Image;` | static |  |  |
| 2 | `void mirrorImageHorizontally(Image image)` | `mirrorImageHorizontally(Ljavax/microedition/lcdui/Image;)V` | static |  |  |
| 2 | `void mirrorImageVertically(Image image)` | `mirrorImageVertically(Ljavax/microedition/lcdui/Image;)V` | static |  |  |
| 2 | `void setNativeImage(Image img, Image simg)` | `setNativeImage(Ljavax/microedition/lcdui/Image;Lcom/siemens/mp/ui/Image;)V` | protected static |  |  |
| 2 | `Image getNativeImage(Image img)` | `getNativeImage(Ljavax/microedition/lcdui/Image;)Lcom/siemens/mp/ui/Image;` | static |  |  |

## Siemens: com.siemens.mp.media

### Control

`public interface com.siemens.mp.media.Control`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/media/Control.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/media/Control.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/media/Control.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/media/Control.java).

Siemens' copy of the MMAPI player classes in its own package; the members follow MMAPI ([midp-media.md](midp-media.md)).

Declares no members of its own.

### Controllable

`public interface com.siemens.mp.media.Controllable`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/media/Controllable.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/media/Controllable.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/media/Controllable.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/media/Controllable.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `Control[] getControls()` | `getControls()[Lcom/siemens/mp/media/Control;` |  | `IllegalStateException` Thrown if getControls is called in a wrong state. See Player for more details |  |
| 3 | `Control getControl(String controlType)` | `getControl(Ljava/lang/String;)Lcom/siemens/mp/media/Control;` |  | `IllegalStateException` Thrown if getControl is called in a wrong state. See Player for more details |  |

### Manager

`public final class com.siemens.mp.media.Manager extends java.lang.Object`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/media/Manager.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/media/Manager.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/media/Manager.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/media/Manager.java).

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `TONE_DEVICE_LOCATOR` | `Ljava/lang/String;` | `"device://tone"` | static |  |
| 3 | `MIDI_DEVICE_LOCATOR` | `Ljava/lang/String;` | `"device://midi"` | static |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `String[] getSupportedContentTypes(String protocol)` | `getSupportedContentTypes(Ljava/lang/String;)[Ljava/lang/String;` | static |  |  |
| 3 | `String[] getSupportedProtocols(String content_type)` | `getSupportedProtocols(Ljava/lang/String;)[Ljava/lang/String;` | static |  |  |
| 3 | `Player createPlayer(String locator)` | `createPlayer(Ljava/lang/String;)Lcom/siemens/mp/media/Player;` | static | `MediaException` Thrown if a Player cannot be created for the given locator; `IOException` Thrown if there was a problem opening the media; `SecurityException` Thrown if the caller does not have security permission to create the Player |  |
| 3 | `Player createPlayer(InputStream stream, String type)` | `createPlayer(Ljava/io/InputStream;Ljava/lang/String;)Lcom/siemens/mp/media/Player;` | static | `MediaException` Thrown if a Player cannot be created for the given stream and type; `IOException` Thrown if there was a problem reading data from the InputStream; `SecurityException` Thrown if the caller does not have security permission to create the Player |  |
| 3 | `Player createPlayer(DataSource source)` | `createPlayer(Lcom/siemens/mp/media/protocol/DataSource;)Lcom/siemens/mp/media/Player;` | static | `IllegalArgumentException` Thrown if source is null; `MediaException` Thrown if a Player cannot be created for the given DataSource; `IOException` Thrown if there was a problem connecting with the source; `SecurityException` Thrown if the caller does not have security permission to create the Player |  |
| 3 | `void playTone(int note, int duration, int volume)` | `playTone(III)V` | static | `IllegalArgumentException` Thrown if the given note or duration is out of range; `MediaException` Thrown if the tone cannot be played due to a device-related problem |  |
| 3 | `TimeBase getSystemTimeBase()` | `getSystemTimeBase()Lcom/siemens/mp/media/TimeBase;` | static |  |  |

### MediaException

`public class com.siemens.mp.media.MediaException extends java.lang.Exception`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/media/MediaException.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/media/MediaException.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/media/MediaException.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/media/MediaException.java).

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  |  |  |
| 3 | `(String reason)` | `<init>(Ljava/lang/String;)V` |  |  |  |

### Player

`public interface com.siemens.mp.media.Player extends com.siemens.mp.media.Controllable`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/media/Player.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/media/Player.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/media/Player.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/media/Player.java).

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `UNREALIZED` | `I` | `100` | static |  |
| 3 | `REALIZED` | `I` | `200` | static |  |
| 3 | `PREFETCHED` | `I` | `300` | static |  |
| 3 | `STARTED` | `I` | `400` | static |  |
| 3 | `CLOSED` | `I` | `0` | static |  |
| 3 | `TIME_UNKNOWN` | `J` | `-1L` | static |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void realize()` | `realize()V` |  | `IllegalStateException` Thrown if the Player is in the CLOSED state; `MediaException` Thrown if the Player cannot be realized; `SecurityException` Thrown if the caller does not have security permission to realize the Player |  |
| 3 | `void prefetch()` | `prefetch()V` |  | `IllegalStateException` Thrown if the Player is in the CLOSED state; `MediaException` Thrown if the Player cannot be prefetched; `SecurityException` Thrown if the caller does not have security permission to prefetch the Player |  |
| 3 | `void start()` | `start()V` |  | `IllegalStateException` Thrown if the Player is in the CLOSED state; `MediaException` Thrown if the Player cannot be started; `SecurityException` Thrown if the caller does not have security permission to start the Player |  |
| 3 | `void stop()` | `stop()V` |  | `IllegalStateException` Thrown if the Player is in the CLOSED state; `MediaException` Thrown if the Player cannot be stopped |  |
| 3 | `void deallocate()` | `deallocate()V` |  | `IllegalStateException` Thrown if the Player is in the CLOSED state |  |
| 3 | `void close()` | `close()V` |  |  |  |
| 3 | `void setTimeBase(TimeBase master)` | `setTimeBase(Lcom/siemens/mp/media/TimeBase;)V` |  | `IllegalStateException` Thrown if the Player is in the UNREALIZED, STARTED or CLOSED state; `MediaException` Thrown if the specified TimeBase cannot be set on the Player |  |
| 3 | `TimeBase getTimeBase()` | `getTimeBase()Lcom/siemens/mp/media/TimeBase;` |  | `IllegalStateException` Thrown if the Player is in the UNREALIZED or CLOSED state |  |
| 3 | `long setMediaTime(long now)` | `setMediaTime(J)J` |  | `IllegalStateException` Thrown if the Player is in the UNREALIZED or CLOSED state; `MediaException` Thrown if the media time cannot be set |  |
| 3 | `long getMediaTime()` | `getMediaTime()J` |  | `IllegalStateException` Thrown if the Player is in the CLOSED state |  |
| 3 | `int getState()` | `getState()I` |  |  |  |
| 3 | `long getDuration()` | `getDuration()J` |  | `IllegalStateException` Thrown if the Player is in the CLOSED state |  |
| 3 | `String getContentType()` | `getContentType()Ljava/lang/String;` |  | `IllegalStateException` Thrown if the Player is in the UNREALIZED or CLOSED state |  |
| 3 | `void setLoopCount(int count)` | `setLoopCount(I)V` |  | `IllegalArgumentException` Thrown if the given count is invalid; `IllegalStateException` Thrown if the Player is in the STARTED or CLOSED state |  |
| 3 | `void addPlayerListener(PlayerListener playerListener)` | `addPlayerListener(Lcom/siemens/mp/media/PlayerListener;)V` |  | `IllegalStateException` Thrown if the Player is in the CLOSED state |  |
| 3 | `void removePlayerListener(PlayerListener playerListener)` | `removePlayerListener(Lcom/siemens/mp/media/PlayerListener;)V` |  | `IllegalStateException` Thrown if the Player is in the CLOSED state |  |

### PlayerListener

`public interface com.siemens.mp.media.PlayerListener`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/media/PlayerListener.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/media/PlayerListener.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/media/PlayerListener.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/media/PlayerListener.java).

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `STARTED` | `Ljava/lang/String;` | `"started"` | static |  |
| 3 | `STOPPED` | `Ljava/lang/String;` | `"stopped"` | static |  |
| 3 | `STOPPED_AT_TIME` | `Ljava/lang/String;` | `"stoppedAtTime"` | static |  |
| 3 | `END_OF_MEDIA` | `Ljava/lang/String;` | `"endOfMedia"` | static |  |
| 3 | `DURATION_UPDATED` | `Ljava/lang/String;` | `"durationUpdated"` | static |  |
| 3 | `DEVICE_UNAVAILABLE` | `Ljava/lang/String;` | `"deviceUnavailable"` | static |  |
| 3 | `DEVICE_AVAILABLE` | `Ljava/lang/String;` | `"deviceAvailable"` | static |  |
| 3 | `VOLUME_CHANGED` | `Ljava/lang/String;` | `"volumeChanged"` | static |  |
| 3 | `SIZE_CHANGED` | `Ljava/lang/String;` | `"sizeChanged"` | static |  |
| 3 | `ERROR` | `Ljava/lang/String;` | `"error"` | static |  |
| 3 | `CLOSED` | `Ljava/lang/String;` | `"closed"` | static |  |
| 3 | `RECORD_STARTED` | `Ljava/lang/String;` | `"recordStarted"` | static |  |
| 3 | `RECORD_STOPPED` | `Ljava/lang/String;` | `"recordStopped"` | static |  |
| 3 | `RECORD_ERROR` | `Ljava/lang/String;` | `"recordError"` | static |  |
| 3 | `BUFFERING_STARTED` | `Ljava/lang/String;` | `"bufferingStarted"` | static |  |
| 3 | `BUFFERING_STOPPED` | `Ljava/lang/String;` | `"bufferingStopped"` | static |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void playerUpdate(Player player, String event, Object eventData)` | `playerUpdate(Lcom/siemens/mp/media/Player;Ljava/lang/String;Ljava/lang/Object;)V` |  |  |  |

### TimeBase

`public interface com.siemens.mp.media.TimeBase`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/media/TimeBase.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/media/TimeBase.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/media/TimeBase.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `long getTime()` | `getTime()J` |  |  |  |

### ToneControl

`public interface com.siemens.mp.media.control.ToneControl extends com.siemens.mp.media.Control`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/media/control/ToneControl.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/media/control/ToneControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/media/control/ToneControl.java).

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `VERSION` | `B` | `-2` | static |  |
| 3 | `TEMPO` | `B` | `-3` | static |  |
| 3 | `RESOLUTION` | `B` | `-4` | static |  |
| 3 | `BLOCK_START` | `B` | `-5` | static |  |
| 3 | `BLOCK_END` | `B` | `-6` | static |  |
| 3 | `PLAY_BLOCK` | `B` | `-7` | static |  |
| 3 | `SET_VOLUME` | `B` | `-8` | static |  |
| 3 | `REPEAT` | `B` | `-9` | static |  |
| 3 | `C4` | `B` | `60` | static |  |
| 3 | `SILENCE` | `B` | `-1` | static |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void setSequence(byte[] sequence)` | `setSequence([B)V` |  | `IllegalArgumentException` Thrown if the sequence is null or invalid; `IllegalStateException` Thrown if the Player that this control belongs to is in the PREFETCHED or STARTED state |  |

### VolumeControl

`public interface com.siemens.mp.media.control.VolumeControl extends com.siemens.mp.media.Control`

Tier 3. Javadoc: [Siemens_API](https://nikita36078.github.io/J2ME_Docs/docs/Siemens_API/com/siemens/mp/media/control/VolumeControl.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/media/control/VolumeControl.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/media/control/VolumeControl.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void setMute(boolean mute)` | `setMute(Z)V` |  |  |  |
| 3 | `boolean isMuted()` | `isMuted()Z` |  |  |  |
| 3 | `int setLevel(int level)` | `setLevel(I)I` |  |  |  |
| 3 | `int getLevel()` | `getLevel()I` |  |  |  |

## Samsung: com.samsung.util

### AudioClip

`public class com.samsung.util.AudioClip extends java.lang.Object`

Tier 3. Javadoc: [Samsung_API_1_0](https://nikita36078.github.io/J2ME_Docs/docs/Samsung_API_1_0/com/samsung/util/AudioClip.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/samsung/util/AudioClip.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/samsung/util/AudioClip.java).

Plays sound files, typically MMF, from the JAR or a buffer. Check `isSupported()` first, as the javadoc example does.

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `TYPE_MMF` | `I` | `1` | static |  |
| 3 | `TYPE_MP3` | `I` | `2` | static |  |
| 3 | `TYPE_MIDI` | `I` | `3` | static |  |

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `(int type, byte[] audioData, int audioOffset, int audioLength)` | `<init>(I[BII)V` |  | `IllegalArgumentException` if type is not legal values; `NullPointerException` if audioData is null; `ArrayIndexOutOfBoundsException` if audioOffset or audioLength specify an invalid range |  |
| 3 | `(int type, String filename)` | `<init>(ILjava/lang/String;)V` |  | `IllegalArgumentException` if type is not legal values; `NullPointerException` if filename is null; `IOException` if the resource does not exist |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void play(int loop, int volume)` | `play(II)V` |  | `IllegalArgumentException` Thrown if loop, or volume are invalid value; `IllegalStateException` Thrown if the device does not support AudioClip |  |
| 3 | `void stop()` | `stop()V` |  | `IllegalStateException` Thrown if the device does not support AudioClip |  |
| 3 | `void pause()` | `pause()V` |  | `IllegalStateException` Thrown if the device does not support AudioClip |  |
| 3 | `void resume()` | `resume()V` |  | `IllegalStateException` Thrown if the device does not support AudioClip |  |
| 3 | `boolean isSupported()` | `isSupported()Z` | static |  |  |

### LCDLight

`public final class com.samsung.util.LCDLight extends java.lang.Object`

Tier 3. Javadoc: [Samsung_API_1_0](https://nikita36078.github.io/J2ME_Docs/docs/Samsung_API_1_0/com/samsung/util/LCDLight.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/samsung/util/LCDLight.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/samsung/util/LCDLight.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `boolean isSupported()` | `isSupported()Z` | static |  |  |
| 3 | `void on(int duration)` | `on(I)V` | static | `IllegalStateException` Thrown if the device does not support LCDLight; `IllegalArgumentException` Thrown if duration is in invalid range |  |
| 3 | `void off()` | `off()V` | static | `IllegalStateException` Thrown if the device does not support LCDLight |  |

### SM

`public class com.samsung.util.SM extends java.lang.Object`

Tier 3. Javadoc: [Samsung_API_1_0](https://nikita36078.github.io/J2ME_Docs/docs/Samsung_API_1_0/com/samsung/util/SM.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/samsung/util/SM.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/samsung/util/SM.java).

A text message. FastEmu must never send it.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  |  |  |
| 3 | `(String dest, String callback, String textMessage)` | `<init>(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V` |  | `IllegalArgumentException` if the length of textMessage exceeds 80. or dest is invalid phone number |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void setDestAddress(String address)` | `setDestAddress(Ljava/lang/String;)V` |  | `IllegalArgumentException` if address is invalid phone number |  |
| 3 | `void setCallbackAddress(String address)` | `setCallbackAddress(Ljava/lang/String;)V` |  |  |  |
| 3 | `void setData(String textMessage)` | `setData(Ljava/lang/String;)V` |  | `IllegalArgumentException` if the length of textMessage exceeds 80 |  |
| 3 | `String getDestAddress()` | `getDestAddress()Ljava/lang/String;` |  |  |  |
| 3 | `String getCallbackAddress()` | `getCallbackAddress()Ljava/lang/String;` |  |  |  |
| 3 | `String getData()` | `getData()Ljava/lang/String;` |  |  |  |

### com.samsung.util.SMS

`public class com.samsung.util.SMS extends java.lang.Object`

Tier 3. Javadoc: [Samsung_API_1_0](https://nikita36078.github.io/J2ME_Docs/docs/Samsung_API_1_0/com/samsung/util/SMS.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/samsung/util/SMS.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/samsung/util/SMS.java).

FastEmu must never send anything.

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `boolean isSupported()` | `isSupported()Z` | static |  |  |
| 3 | `void send(SM sm)` | `send(Lcom/samsung/util/SM;)V` | static | `IllegalStateException` Thrown if the device does not support SMS |  |

### com.samsung.util.Vibration

`public final class com.samsung.util.Vibration extends java.lang.Object`

Tier 3. Javadoc: [Samsung_API_1_0](https://nikita36078.github.io/J2ME_Docs/docs/Samsung_API_1_0/com/samsung/util/Vibration.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/samsung/util/Vibration.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/samsung/util/Vibration.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `boolean isSupported()` | `isSupported()Z` | static |  |  |
| 3 | `void start(int duration, int strength)` | `start(II)V` | static | `IllegalStateException` Thrown if the device does not support Vibration; `IllegalArgumentException` Thrown if duration or strength is in invalid range |  |
| 3 | `void stop()` | `stop()V` | static | `IllegalStateException` Thrown if the device does not support Vibration |  |

## Motorola: com.motorola.game

### BackgroundMusic

`public abstract class com.motorola.game.BackgroundMusic extends java.lang.Object`

Tier 3. Javadoc: [Motorola_Gaming_API](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_Gaming_API/com/motorola/game/BackgroundMusic.html).

Motorola's game API for its MIDP 1.0 phones; MIDP 2.0's game package covers the same ground.

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `BackgroundMusic createBackgroundMusic(String name)` | `createBackgroundMusic(Ljava/lang/String;)Lcom/motorola/game/BackgroundMusic;` | static | `FileFormatNotSupportedException` if the specified resource is not a recognized format |  |

### FileFormatNotSupportedException

`public class com.motorola.game.FileFormatNotSupportedException extends java.lang.RuntimeException`

Tier 3. Javadoc: [Motorola_Gaming_API](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_Gaming_API/com/motorola/game/FileFormatNotSupportedException.html).

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `(Exception e)` | `<init>(Ljava/lang/Exception;)V` |  |  |  |
| 3 | `(String info)` | `<init>(Ljava/lang/String;)V` |  |  |  |
| 3 | `(String info, Exception e)` | `<init>(Ljava/lang/String;Ljava/lang/Exception;)V` |  |  |  |

### GameScreen

`public abstract class com.motorola.game.GameScreen extends java.lang.Object`

Tier 3. Javadoc: [Motorola_Gaming_API](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_Gaming_API/com/motorola/game/GameScreen.html).

A canvas with an off-screen buffer, key states and sound, like MIDP 2.0's `GameCanvas`.

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `UP_KEY` | `I` | `1` | static |  |
| 3 | `DOWN_KEY` | `I` | `2` | static |  |
| 3 | `LEFT_KEY` | `I` | `4` | static |  |
| 3 | `RIGHT_KEY` | `I` | `8` | static |  |
| 3 | `FIRE_KEY` | `I` | `16` | static |  |
| 3 | `GAME_A_KEY` | `I` | `32` | static |  |
| 3 | `GAME_B_KEY` | `I` | `64` | static |  |
| 3 | `GAME_C_KEY` | `I` | `128` | static |  |
| 3 | `GAME_D_KEY` | `I` | `256` (`0x100`) | static |  |
| 3 | `VOLUME_MAX` | `I` | `100` | static |  |
| 3 | `VOLUME_MIN` | `I` | `0` | static |  |
| 3 | `PRIORITY_MAX` | `I` | `100` | static |  |
| 3 | `PRIORITY_MIN` | `I` | `0` | static |  |

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `Graphics getGraphics()` | `getGraphics()Ljavax/microedition/lcdui/Graphics;` | protected |  |  |
| 3 | `int getKeyStates()` | `getKeyStates()I` |  |  |  |
| 3 | `void enableKeyEvents(boolean enabled)` | `enableKeyEvents(Z)V` |  |  |  |
| 3 | `void paint(Graphics g)` | `paint(Ljavax/microedition/lcdui/Graphics;)V` |  |  |  |
| 3 | `void flushGraphics(int x, int y, int width, int height)` | `flushGraphics(IIII)V` |  |  |  |
| 3 | `void flushGraphics()` | `flushGraphics()V` |  |  |  |
| 3 | `int getDisplayColor(int color)` | `getDisplayColor(I)I` | static | `IllegalArgumentException` if color is not between 0x000000 and 0xFFFFFF |  |
| 3 | `void playSoundEffect(SoundEffect se, int volume, int priority)` | `playSoundEffect(Lcom/motorola/game/SoundEffect;II)V` |  |  |  |
| 3 | `boolean soundEffectsSupported()` | `soundEffectsSupported()Z` |  |  |  |
| 3 | `boolean backgroundMusicSupported()` | `backgroundMusicSupported()Z` |  |  |  |
| 3 | `int getMaxSoundsSupported()` | `getMaxSoundsSupported()I` |  |  |  |
| 3 | `void stopAllSoundEffects()` | `stopAllSoundEffects()V` |  |  |  |
| 3 | `void playBackgroundMusic(BackgroundMusic bgm, boolean loop)` | `playBackgroundMusic(Lcom/motorola/game/BackgroundMusic;Z)V` |  |  |  |

### ImageUtil

`public class com.motorola.game.ImageUtil extends java.lang.Object`

Tier 3. Javadoc: [Motorola_Gaming_API](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_Gaming_API/com/motorola/game/ImageUtil.html).

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `SCALE_REPLICATE` | `I` | `0` | static |  |
| 3 | `SCALE_AREA` | `I` | `1` | static |  |
| 3 | `SCALE_SMOOTH` | `I` | `2` | static |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void getPixels(Image src, int x, int y, int width, int height, int[] rgbData)` | `getPixels(Ljavax/microedition/lcdui/Image;IIII[I)V` | static | `ArrayIndexOutOfBoundsException` if the length of rgbData is less than width * height |  |
| 3 | `void getPixels(Image src, int[] rgbData)` | `getPixels(Ljavax/microedition/lcdui/Image;[I)V` | static | `ArrayIndexOutOfBoundsException` if the length of rgbData is not sufficient (i.e. less than the number of pixels in the source image) |  |
| 3 | `void setPixels(Image dest, int x, int y, int width, int height, int[] rgbData)` | `setPixels(Ljavax/microedition/lcdui/Image;IIII[I)V` | static | `ArrayIndexOutOfBoundsException` if the length of rgbData is less than width * height; `IllegalArgumentException` if the destination image is immutable |  |
| 3 | `void setPixels(Image dest, int[] rgbData)` | `setPixels(Ljavax/microedition/lcdui/Image;[I)V` | static | `ArrayIndexOutOfBoundsException` if the length of rgbData is not sufficient (i.e. less than the number of pixels in the destination image); `IllegalArgumentException` if the destination image is immutable |  |
| 3 | `Image getScaleImage(Image src, int width, int height, int method)` | `getScaleImage(Ljavax/microedition/lcdui/Image;III)Ljavax/microedition/lcdui/Image;` | static | `IllegalArgumentException` if the width or height is less than 1, or if the specified method is not a valid value (SCALE_REPLICATE, SCALE_AREA, or SCALE_SMOOTH) |  |

### PaletteImage

`public class com.motorola.game.PaletteImage extends java.lang.Object`

Tier 3. Javadoc: [Motorola_Gaming_API](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_Gaming_API/com/motorola/game/PaletteImage.html).

An image whose palette can be changed after loading.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `(byte[] data, int offset, int length)` | `<init>([BII)V` |  | `IOException` if the data is corrupt, cannot be read, or does not contain valid palette-based image data in a supported format |  |
| 3 | `(String name)` | `<init>(Ljava/lang/String;)V` |  | `IOException` if the resource is corrupt, cannot be read, or does not contain valid palette-based image data in a supported format |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `Image getImage()` | `getImage()Ljavax/microedition/lcdui/Image;` |  |  |  |
| 3 | `int getTransparentIndex()` | `getTransparentIndex()I` |  |  |  |
| 3 | `void setTransparentIndex(int index)` | `setTransparentIndex(I)V` |  | `IndexOutOfBoundsException` if the index is invalid |  |
| 3 | `int getPaletteSize()` | `getPaletteSize()I` |  |  |  |
| 3 | `int getPaletteEntry(int index)` | `getPaletteEntry(I)I` |  | `IndexOutOfBoundsException` if the index is invalid |  |
| 3 | `void setPaletteEntry(int index, int color)` | `setPaletteEntry(II)V` |  | `IndexOutOfBoundsException` if the index is invalid |  |
| 3 | `int[] getPalette()` | `getPalette()[I` |  |  |  |
| 3 | `void setPalette(int[] newPalette)` | `setPalette([I)V` |  | `ArrayIndexOutOfBoundsException` if the palette data array is too small; `NullPointerException` if the palette data array is null; `IllegalArgumentException` if any of the required entries are not between 0x000000 and 0xFFFFFF |  |

### PlayField

`public class com.motorola.game.PlayField extends java.lang.Object`

Tier 3. Javadoc: [Motorola_Gaming_API](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_Gaming_API/com/motorola/game/PlayField.html).

A tile grid with sprites, like `TiledLayer` and `LayerManager` together.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `(int columns, int rows, Image img, int tWidth, int tHeight)` | `<init>(IILjavax/microedition/lcdui/Image;II)V` |  | `NullPointerException` if img is null; `IllegalArgumentException` if `rows < 1 \|\| columns < 1 \|\| tWidth < 1 \|\| tHeight < 1 \|\| img.getWidth() % tWidth != 0 \|\| img.getHeight() % tHeight != 0` |  |
| 3 | `(int columns, int rows, int cellWidth, int cellHeight)` | `<init>(IIII)V` |  | `IllegalArgumentException` if `rows < 1 \|\| columns < 1 \|\| cellWidth < 1 \|\| cellHeight < 1` |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void addSprite(Sprite s)` | `addSprite(Lcom/motorola/game/Sprite;)V` |  | `NullPointerException` if s is null |  |
| 3 | `void removeSprite(Sprite s)` | `removeSprite(Lcom/motorola/game/Sprite;)V` |  | `NullPointerException` if s is null; `RuntimeException` if Sprite s is not associated with the PlayField |  |
| 3 | `void removeAllSprites()` | `removeAllSprites()V` |  |  |  |
| 3 | `int createAnimatedTile(int staticTileIdx)` | `createAnimatedTile(I)I` |  | `IndexOutOfBoundsException` if there is no static tile with index staticTileIdx |  |
| 3 | `void setAnimatedTileImage(int animTileIdx, int staticTileIdx)` | `setAnimatedTileImage(II)V` |  | `IndexOutOfBoundsException` if there is no static tile with index staticTileIdx or no animated tile with index animTileIdx |  |
| 3 | `int getAnimatedTileImage(int animTileIdx)` | `getAnimatedTileImage(I)I` |  | `IndexOutOfBoundsException` if there is no animated tile with index animTileIdx |  |
| 3 | `void setCell(int col, int row, int tileIdx)` | `setCell(III)V` |  | `IndexOutOfBoundsException` if there is no tile with index tileIdx; `ArrayIndexOutOfBoundsException` if row or col is outside the bounds of the PlayField grid |  |
| 3 | `int getCell(int col, int row)` | `getCell(II)I` |  | `ArrayIndexOutOfBoundsException` if row or col is outside the bounds of the PlayField grid |  |
| 3 | `void moveTiles(int dstCol, int dstRow, int srcCol, int srcRow, int width, int height)` | `moveTiles(IIIIII)V` |  | `ArrayIndexOutOfBoundsException` if either of the rectangular regions defined by the parameters extends beyond the bounds of the PlayField grid |  |
| 3 | `void fillCells(int col, int row, int width, int height, int tileIdx)` | `fillCells(IIIII)V` |  | `ArrayIndexOutOfBoundsException` if the rectangular region defined by the parameters extends beyond the bounds of the PlayField grid; `IndexOutOfBoundsException` if there is no tile with index tileIdx |  |
| 3 | `void draw(Graphics g, int x, int y)` | `draw(Ljavax/microedition/lcdui/Graphics;II)V` |  | `NullPointerException` if g is null |  |
| 3 | `int getCellWidth()` | `getCellWidth()I` |  |  |  |
| 3 | `int getCellHeight()` | `getCellHeight()I` |  |  |  |
| 3 | `int getGridWidth()` | `getGridWidth()I` |  |  |  |
| 3 | `int getGridHeight()` | `getGridHeight()I` |  |  |  |
| 3 | `boolean anyCollisions()` | `anyCollisions()Z` |  |  |  |
| 3 | `boolean collidesWithSprites(Sprite s)` | `collidesWithSprites(Lcom/motorola/game/Sprite;)Z` |  | `NullPointerException` if s is null |  |
| 3 | `boolean collidesWithAnyTile(Sprite s)` | `collidesWithAnyTile(Lcom/motorola/game/Sprite;)Z` |  | `NullPointerException` if s is null |  |
| 3 | `boolean collidesWithTiles(int col, int row, int width, int height, Sprite s, boolean pixelLevel)` | `collidesWithTiles(IIIILcom/motorola/game/Sprite;Z)Z` |  | `NullPointerException` if s is null; `ArrayIndexOutOfBoundsException` if the rectangular region defined by the parameters extends beyond the bounds of the PlayField grid |  |
| 3 | `void setStaticTileImage(int staticTileIdx, Image img, int x, int y)` | `setStaticTileImage(ILjavax/microedition/lcdui/Image;II)V` |  | `IndexOutOfBoundsException` if there is no static tile with index staticTileIdx or if the rectangular region required for the tile extends beyond the bounds of the Image; `NullPointerException` if img is null |  |
| 3 | `void setStaticTileSet(Image img, int tWidth, int tHeight)` | `setStaticTileSet(Ljavax/microedition/lcdui/Image;II)V` |  | `NullPointerException` if img is null; `IllegalArgumentException` if `tWidth < 1 \|\| tHeight < 1 \|\| img.getWidth() % tWidth != 0 \|\| img.getHeight() % tHeight != 0` |  |
| 3 | `void setViewWindow(int x, int y, int width, int height)` | `setViewWindow(IIII)V` |  |  |  |

### SoundEffect

`public class com.motorola.game.SoundEffect extends java.lang.Object`

Tier 3. Javadoc: [Motorola_Gaming_API](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_Gaming_API/com/motorola/game/SoundEffect.html).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `SoundEffect createSoundEffect(String resource)` | `createSoundEffect(Ljava/lang/String;)Lcom/motorola/game/SoundEffect;` | static | `FileFormatNotSupportedException` if the specified resource is not a recognized format or the File size is too large for the internal buffers |  |

### com.motorola.game.Sprite

`public class com.motorola.game.Sprite extends java.lang.Object`

Tier 3. Javadoc: [Motorola_Gaming_API](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_Gaming_API/com/motorola/game/Sprite.html).

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `(Image img)` | `<init>(Ljavax/microedition/lcdui/Image;)V` |  | `NullPointerException` if img is null |  |
| 3 | `(Image img, int fWidth, int fHeight)` | `<init>(Ljavax/microedition/lcdui/Image;II)V` |  | `NullPointerException` if img is null; `IllegalArgumentException` if `fWidth < 1 \|\| fHeight < 1 \|\| img.getWidth() % fWidth != 0 \|\| img.getHeight() % fHeight != 0` |  |
| 3 | `(Sprite s)` | `<init>(Lcom/motorola/game/Sprite;)V` |  | `NullPointerException` if s is null |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void setPosition(int x, int y)` | `setPosition(II)V` |  |  |  |
| 3 | `void setDepth(int d)` | `setDepth(I)V` |  |  |  |
| 3 | `void move(int dx, int dy)` | `move(II)V` |  |  |  |
| 3 | `int getX()` | `getX()I` |  |  |  |
| 3 | `int getY()` | `getY()I` |  |  |  |
| 3 | `int getDepth()` | `getDepth()I` |  |  |  |
| 3 | `int getHeight()` | `getHeight()I` |  |  |  |
| 3 | `int getWidth()` | `getWidth()I` |  |  |  |
| 3 | `boolean collidesWith(Sprite s, boolean pixelLevel)` | `collidesWith(Lcom/motorola/game/Sprite;Z)Z` |  | `NullPointerException` if s is null |  |
| 3 | `void setFrame(int f)` | `setFrame(I)V` |  | `ArrayIndexOutOfBoundsException` if `f < 0 \|\| f >= getFrameSequence().length` |  |
| 3 | `int getFrame()` | `getFrame()I` |  |  |  |
| 3 | `int getNumRawFrames()` | `getNumRawFrames()I` |  |  |  |
| 3 | `void nextFrame()` | `nextFrame()V` |  |  |  |
| 3 | `void prevFrame()` | `prevFrame()V` |  |  |  |
| 3 | `void setVisible(boolean visible)` | `setVisible(Z)V` |  |  |  |
| 3 | `boolean isVisible()` | `isVisible()Z` |  |  |  |
| 3 | `void draw(Graphics g)` | `draw(Ljavax/microedition/lcdui/Graphics;)V` | final | `NullPointerException` if g is null |  |
| 3 | `void setFrameSequence(int[] seq)` | `setFrameSequence([I)V` |  | `ArrayIndexOutOfBoundsException` if seq is non-null and any member of the array has a value less than 0 or greater than or equal to getNumRawFrames() |  |
| 3 | `int[] getFrameSequence()` | `getFrameSequence()[I` |  |  |  |
| 3 | `void setImage(Image img, int fWidth, int fHeight)` | `setImage(Ljavax/microedition/lcdui/Image;II)V` |  | `NullPointerException` if img is null; `IllegalArgumentException` if `fWidth < 1 \|\| fHeight < 1 \|\| img.getWidth() % fWidth != 0 \|\| img.getHeight() % fHeight != 0` |  |

## Motorola: com.motorola.funlight

### FunLight

`public class com.motorola.funlight.FunLight extends java.lang.Object`

Tier 3. Javadoc: [Motorola_FunLights_API_1_0](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_FunLights_API_1_0/com/motorola/funlight/FunLight.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/motorola/funlight/FunLight.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/motorola/funlight/FunLight.java).

The colored lights of some Motorola phones. FastEmu can accept and ignore every call.

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `BLACK` | `I` | `0` | static |  |
| 3 | `BLANK` | `I` | `0` | static |  |
| 3 | `BLUE` | `I` | `255` | static |  |
| 3 | `CYAN` | `I` | `65535` (`0xffff`) | static |  |
| 3 | `GREEN` | `I` | `65280` | static |  |
| 3 | `MAGENTA` | `I` | `16711935` (`0xff00ff`) | static |  |
| 3 | `OFF` | `I` | `0` | static |  |
| 3 | `ON` | `I` | `16777215` (`0xffffff`) | static |  |
| 3 | `RED` | `I` | `16711680` (`0xff0000`) | static |  |
| 3 | `WHITE` | `I` | `16777215` (`0xffffff`) | static |  |
| 3 | `YELLOW` | `I` | `16776960` (`0xffff00`) | static |  |
| 3 | `QUEUED` | `I` | `1` | static |  |
| 3 | `SUCCESS` | `I` | `0` | static |  |
| 3 | `IGNORED` | `I` | `2` | static |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `int getControl()` | `getControl()I` | static |  |  |
| 3 | `Region getRegion(int ID)` | `getRegion(I)Lcom/motorola/funlight/Region;` | static |  |  |
| 3 | `int[] getRegionsIDs()` | `getRegionsIDs()[I` | static |  |  |
| 3 | `Region[] getRegions()` | `getRegions()[Lcom/motorola/funlight/Region;` | static |  |  |
| 3 | `void releaseControl()` | `releaseControl()V` | static |  |  |
| 3 | `int setColor(int color)` | `setColor(I)I` | static |  |  |
| 3 | `int setColor(byte red, byte green, byte blue)` | `setColor(BBB)I` | static |  |  |

### FunLightException

`public class com.motorola.funlight.FunLightException extends java.lang.RuntimeException`

Tier 3. Javadoc: [Motorola_FunLights_API_1_0](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_FunLights_API_1_0/com/motorola/funlight/FunLightException.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/motorola/funlight/FunLightException.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/motorola/funlight/FunLightException.java).

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `(String s)` | `<init>(Ljava/lang/String;)V` |  |  |  |
| 3 | `()` | `<init>()V` |  |  |  |

### Region

`public interface com.motorola.funlight.Region`

Tier 3. Javadoc: [Motorola_FunLights_API_1_0](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_FunLights_API_1_0/com/motorola/funlight/Region.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/motorola/funlight/Region.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/motorola/funlight/Region.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `int getColor()` | `getColor()I` |  | `FunLightException` if setColor method was not called before calling this method except Blank region |  |
| 3 | `int getControl()` | `getControl()I` |  |  |  |
| 3 | `int getID()` | `getID()I` |  |  |  |
| 3 | `void releaseControl()` | `releaseControl()V` |  |  |  |
| 3 | `int setColor(int color)` | `setColor(I)I` |  |  |  |
| 3 | `int setColor(byte red, byte green, byte blue)` | `setColor(BBB)I` |  |  |  |
| 3 | `String toString()` | `toString()Ljava/lang/String;` |  |  |  |

## Motorola: com.motorola.multimedia

### Lighting

`public class com.motorola.multimedia.Lighting extends java.lang.Object`

Tier 3. Javadoc: [VibeLight](https://nikita36078.github.io/J2ME_Docs/docs/VibeLight/com/motorola/multimedia/Lighting.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/motorola/multimedia/Lighting.java).

Backlight on and off.

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void backlightOff()` | `backlightOff()V` | static |  |  |
| 3 | `void backlightOn()` | `backlightOn()V` | static |  |  |

### com.motorola.multimedia.Vibrator

`public class com.motorola.multimedia.Vibrator extends java.util.TimerTask`

Tier 3. Javadoc: [VibeLight](https://nikita36078.github.io/J2ME_Docs/docs/VibeLight/com/motorola/multimedia/Vibrator.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/motorola/multimedia/Vibrator.java).

Extends `java.util.TimerTask`.

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `MAX_VIBRATE_TIME` | `I` | (value not in the sources) | static |  |
| 3 | `MIN_PAUSE_TIME` | `I` | `2000` (from freej2me-plus only) | static |  |
| 3 | `VIBRATE_SILENT` | `I` | `0` (from freej2me-plus only) | static |  |
| 3 | `VIBRATE_SHORT` | `I` | `4` (from freej2me-plus only) | static |  |
| 3 | `VIBRATE_LONG` | `I` | `2` (from freej2me-plus only) | static |  |
| 3 | `VIBRATE_2SHORT` | `I` | `1` (from freej2me-plus only) | static |  |
| 3 | `VIBRATE_SHORT_LONG` | `I` | `5` (from freej2me-plus only) | static |  |
| 3 | `VIBRATE_PULSE` | `I` | `3` (from freej2me-plus only) | static |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void run()` | `run()V` |  |  |  |
| 3 | `void vibrateFor(int timeInMs)` | `vibrateFor(I)V` | static |  |  |
| 3 | `void vibratePeriodically(int timeInMs)` | `vibratePeriodically(I)V` | static |  |  |
| 3 | `void vibratePeriodically(int timeOnInMs, int timeOffInMs)` | `vibratePeriodically(II)V` | static |  |  |
| 3 | `void vibratorOff()` | `vibratorOff()V` | static |  |  |
| 3 | `void vibratorOn()` | `vibratorOn()V` | static |  |  |
| 3 | `void setVibrateTone(int tone)` | `setVibrateTone(I)V` | static |  |  |

## Sony Ericsson: com.sonyericsson.capuchin

### ExternalResourceHandler

`public interface com.sonyericsson.capuchin.ExternalResourceHandler`

Tier 3. Javadoc: [Capuchin](https://nikita36078.github.io/J2ME_Docs/docs/Capuchin/com/sonyericsson/capuchin/ExternalResourceHandler.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/sonyericsson/capuchin/ExternalResourceHandler.java).

Capuchin plays Flash Lite content inside a Java application; rare in games.

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void requestResource(FlashImage image, String URI)` | `requestResource(Lcom/sonyericsson/capuchin/FlashImage;Ljava/lang/String;)V` |  |  |  |

### FlashCanvas

`public class com.sonyericsson.capuchin.FlashCanvas extends javax.microedition.lcdui.Canvas`

Tier 3. Javadoc: [Capuchin](https://nikita36078.github.io/J2ME_Docs/docs/Capuchin/com/sonyericsson/capuchin/FlashCanvas.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/sonyericsson/capuchin/FlashCanvas.java).

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `(FlashImage image)` | `<init>(Lcom/sonyericsson/capuchin/FlashImage;)V` |  | `NullPointerException` if image is null |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `int getDelay()` | `getDelay()I` |  |  |  |

### FlashDataRequest

`public final class com.sonyericsson.capuchin.FlashDataRequest extends java.lang.Object`

Tier 3. Javadoc: [Capuchin](https://nikita36078.github.io/J2ME_Docs/docs/Capuchin/com/sonyericsson/capuchin/FlashDataRequest.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/sonyericsson/capuchin/FlashDataRequest.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `String[] getArgs()` | `getArgs()[Ljava/lang/String;` |  |  |  |
| 3 | `void setProperty(String name, String value)` | `setProperty(Ljava/lang/String;Ljava/lang/String;)V` |  | `NullPointerException` if any argument is null; `IllegalStateException` if this request has been completed, by calling {link complete() See com.sonyericsson.capuchin.FlashDataRequestListener for code example using this method |  |
| 3 | `void setProperty(String name, int value)` | `setProperty(Ljava/lang/String;I)V` |  | `NullPointerException` if any argument is null; `IllegalStateException` if this request has been completed after {link complete() has been called. See com.sonyericsson.capuchin.FlashDataRequestListener for code example using this method |  |
| 3 | `void complete()` | `complete()V` |  |  |  |

### FlashDataRequestListener

`public interface com.sonyericsson.capuchin.FlashDataRequestListener`

Tier 3. Javadoc: [Capuchin](https://nikita36078.github.io/J2ME_Docs/docs/Capuchin/com/sonyericsson/capuchin/FlashDataRequestListener.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/sonyericsson/capuchin/FlashDataRequestListener.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void dataRequested(FlashDataRequest flashDataRequest)` | `dataRequested(Lcom/sonyericsson/capuchin/FlashDataRequest;)V` |  |  |  |

### FlashEventListener

`public final class com.sonyericsson.capuchin.FlashEventListener extends java.lang.Object`

Tier 3. Javadoc: [Capuchin](https://nikita36078.github.io/J2ME_Docs/docs/Capuchin/com/sonyericsson/capuchin/FlashEventListener.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/sonyericsson/capuchin/FlashEventListener.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void handleEvent(String[] eventData)` | `handleEvent([Ljava/lang/String;)V` |  | `NullPointerException` if eventData is null. See com.sonyericsson.capuchin.FlashEventManager for code example using this method |  |

### FlashEventManager

`public interface com.sonyericsson.capuchin.FlashEventManager`

Tier 3. Javadoc: [Capuchin](https://nikita36078.github.io/J2ME_Docs/docs/Capuchin/com/sonyericsson/capuchin/FlashEventManager.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/sonyericsson/capuchin/FlashEventManager.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void addFlashEventListener(String eventListenerName, FlashEventListener flashEventListener)` | `addFlashEventListener(Ljava/lang/String;Lcom/sonyericsson/capuchin/FlashEventListener;)V` |  |  |  |

### FlashImage

`public class com.sonyericsson.capuchin.FlashImage extends java.lang.Object`

Tier 3. Javadoc: [Capuchin](https://nikita36078.github.io/J2ME_Docs/docs/Capuchin/com/sonyericsson/capuchin/FlashImage.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/sonyericsson/capuchin/FlashImage.java).

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `KEY_PRESSED` | `I` | `0` | static |  |
| 3 | `KEY_RELEASED` | `I` | `3` | static |  |
| 3 | `KEY_REPEATED` | `I` | `1` | static |  |
| 3 | `COMMAND_PLAY` | `I` | `1` | static |  |
| 3 | `COMMAND_STOP` | `I` | `2` | static |  |
| 3 | `COMMAND_PAUSE` | `I` | `3` | static |  |
| 3 | `COMMAND_RESUME` | `I` | `4` | static |  |
| 3 | `POINTER_DRAGGED` | `I` | `1` | static |  |
| 3 | `POINTER_PRESSED` | `I` | `2` | static |  |
| 3 | `POINTER_RELEASED` | `I` | `3` | static |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `FlashImage createImage(InputStream flashContent, ExternalResourceHandler externalRH)` | `createImage(Ljava/io/InputStream;Lcom/sonyericsson/capuchin/ExternalResourceHandler;)Lcom/sonyericsson/capuchin/FlashImage;` | static | `IOException` if any error when reading the content; `NullPointerException` if flashContent is null |  |
| 3 | `void dispatchCommand(int command)` | `dispatchCommand(I)V` |  |  |  |
| 3 | `void dispatchKeyEvent(int keyCode, int type)` | `dispatchKeyEvent(II)V` |  |  |  |
| 3 | `void dispatchPointerEvent(int x, int y, int type)` | `dispatchPointerEvent(III)V` |  |  |  |
| 3 | `int render(Graphics g, int x, int y, int width, int height)` | `render(Ljavax/microedition/lcdui/Graphics;IIII)I` |  | `NullPointerException` if g is null; `IllegalArgumentException` if width or height are negative; `OutOfMemoryError` if there is not enough available memory for rendering the Flash content |  |
| 3 | `void setFlashDataRequestListener(FlashDataRequestListener requestListener)` | `setFlashDataRequestListener(Lcom/sonyericsson/capuchin/FlashDataRequestListener;)V` |  |  |  |
| 3 | `void setFlashEventManager(FlashEventManager eventManager)` | `setFlashEventManager(Lcom/sonyericsson/capuchin/FlashEventManager;)V` |  |  |  |
| 3 | `void resourceAvailable(String URI, InputStream resourceData)` | `resourceAvailable(Ljava/lang/String;Ljava/io/InputStream;)V` |  | `IOException` if an error occurs while loading the resource; `NullPointerException` if URI is null |  |

### FlashPlayer

`public class com.sonyericsson.capuchin.FlashPlayer extends java.lang.Object`

Tier 3. Javadoc: [Capuchin](https://nikita36078.github.io/J2ME_Docs/docs/Capuchin/com/sonyericsson/capuchin/FlashPlayer.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/sonyericsson/capuchin/FlashPlayer.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `FlashPlayer createFlashPlayer(FlashImage flashImage, FlashCanvas flashCanvas)` | `createFlashPlayer(Lcom/sonyericsson/capuchin/FlashImage;Lcom/sonyericsson/capuchin/FlashCanvas;)Lcom/sonyericsson/capuchin/FlashPlayer;` | static | `NullPointerException` if any parameter is null |  |
| 3 | `void play()` | `play()V` |  | `IllegalStateException` if the FlashPlayer is not currently in the stopped or paused state |  |
| 3 | `void pause()` | `pause()V` |  | `IllegalStateException` if the FlashPlayer is not currently in the playing state |  |
| 3 | `void stop()` | `stop()V` |  | `IllegalStateException` if the FlashPlayer is not currently in the playing or paused state |  |
| 3 | `Displayable getDisplayable()` | `getDisplayable()Ljavax/microedition/lcdui/Displayable;` |  |  |  |

## LG: mmpp

### SISAniImage

`public class com.velox.SISAniImage extends javax.microedition.lcdui.Image`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/com/velox/SISAniImage.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/velox/SISAniImage.java).

LG: animated images.

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `Image createImage(String name)` | `createImage(Ljava/lang/String;)Ljavax/microedition/lcdui/Image;` | static | `IOException` |  |
| 3 | `Image createImage(byte[] imagedata, int imageoffset, int imagelength)` | `createImage([BII)Ljavax/microedition/lcdui/Image;` | static | `ArrayIndexOutOfBoundsException` if imageoffset or imagelength is smaller than 0, or length of imagedata is smaller than sum of imgaeoffset and imagelength |  |
| 3 | `Image get(int index)` | `get(I)Ljavax/microedition/lcdui/Image;` |  | `IllegalArgumentException` if index is smaller than 0 or index is bigger or same than size |  |
| 3 | `Image getType(int index)` | `getType(I)Ljavax/microedition/lcdui/Image;` |  | `IllegalArgumentException` if index is smaller than 0 or index is bigger or same than maxType |  |
| 3 | `boolean isLooped()` | `isLooped()Z` |  |  |  |
| 3 | `int size()` | `size()I` |  |  |  |
| 3 | `int[] getDelay()` | `getDelay()[I` |  |  |  |
| 3 | `int bestIndex()` | `bestIndex()I` |  |  |  |
| 3 | `int numTypes()` | `numTypes()I` |  |  |  |
| 3 | `void set(int index)` | `set(I)V` |  | `IllegalArgumentException` if index is smaller than 0 or index is bigger or same than size |  |
| 3 | `void setBufferSize(int size)` | `setBufferSize(I)V` | static |  |  |
| 3 | `void deallocate()` | `deallocate()V` | static |  |  |

### MathFP

`public final class mmpp.lang.MathFP extends java.lang.Object`

Tier 3. Since 1.0. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/lang/MathFP.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/mmpp/lang/MathFP.java).

LG's fixed-point math. Its javadoc (Korean) describes 20.12 fixed point: 20 integer bits and 12 fraction bits, range -524288 to 524287.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `E` | `I` | (value not in the sources) | static | 1.0 |  |
| 3 | `PI` | `I` | (value not in the sources) | static | 1.0 |  |
| 3 | `MAX_VALUE` | `I` | `524287` (`0x7ffff`) (from freej2me-plus only) | static | 1.0 |  |
| 3 | `MIN_VALUE` | `I` | `-524288` (`0xfff80000`) (from freej2me-plus only) | static | 1.0 |  |
| 3 | `MAX_VALUE_INT` | `I` | (value not in the sources) | static | 1.0 |  |
| 3 | `MIN_VALUE_INT` | `I` | (value not in the sources) | static | 1.0 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `int abs(int i)` | `abs(I)I` | static | 1.0 |  |  |
| 3 | `int round(int i)` | `round(I)I` | static | 1.0 |  |  |
| 3 | `int pow(int b, int e)` | `pow(II)I` | static | 1.0 | `IllegalArgumentException`; `ArithmeticException` |  |
| 3 | `int sqrt(int i)` | `sqrt(I)I` | static | 1.0 | `IllegalArgumentException`; `ArithmeticException` |  |
| 3 | `int log(int f)` | `log(I)I` | static | 1.0 | `IllegalArgumentException`; `ArithmeticException` |  |
| 3 | `int exp(int f)` | `exp(I)I` | static | 1.0 | `ArithmeticException` |  |
| 3 | `int max(int a, int b)` | `max(II)I` | static | 1.0 |  |  |
| 3 | `int min(int a, int b)` | `min(II)I` | static | 1.0 |  |  |
| 3 | `int add(int i, int j)` | `add(II)I` | static | 1.0 |  |  |
| 3 | `int sub(int i, int j)` | `sub(II)I` | static | 1.0 |  |  |
| 3 | `int multiply(int i, int j)` | `multiply(II)I` | static | 1.0 | `ArithmeticException` |  |
| 3 | `int divide(int i, int j)` | `divide(II)I` | static | 1.0 | `ArithmeticException` |  |
| 3 | `int parseFP(int f)` | `parseFP(I)I` | static | 1.0 | `NumberFormatException` |  |
| 3 | `int parseFP(String s)` | `parseFP(Ljava/lang/String;)I` | static | 1.0 | `NumberFormatException` |  |
| 3 | `int toInt(int i)` | `toInt(I)I` | static | 1.0 |  |  |
| 3 | `String toString(int f)` | `toString(I)Ljava/lang/String;` | static | 1.0 |  |  |
| 3 | `int cos(int r)` | `cos(I)I` | static | 1.0 |  |  |
| 3 | `int sin(int r)` | `sin(I)I` | static | 1.0 |  |  |
| 3 | `int tan(int r)` | `tan(I)I` | static | 1.0 |  |  |
| 3 | `int acos(int r)` | `acos(I)I` | static | 1.0 | `IllegalArgumentException` |  |
| 3 | `int asin(int r)` | `asin(I)I` | static | 1.0 | `IllegalArgumentException` |  |
| 3 | `int atan(int r)` | `atan(I)I` | static | 1.0 |  |  |

### BackLight

`public final class mmpp.media.BackLight extends java.lang.Object`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/media/BackLight.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/mmpp/media/BackLight.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/mmpp/media/BackLight.java).

LG MMPP API (Korean javadoc): backlight and its color.

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void on(int timeout)` | `on(I)V` | static |  |  |
| 3 | `void off()` | `off()V` | static |  |  |
| 3 | `int numColors()` | `numColors()I` | static |  |  |
| 3 | `void setColor(int RGB)` | `setColor(I)V` | static |  |  |
| 3 | `int getColor()` | `getColor()I` | static |  |  |

### Beep

`public final class mmpp.media.Beep extends java.lang.Object`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/media/Beep.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/mmpp/media/Beep.java).

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `SND_4OCT_A` | `I` | `0` (from freej2me-plus only) | static |  |
| 3 | `SND_4OCT_AS` | `I` | `1` (from freej2me-plus only) | static |  |
| 3 | `SND_4OCT_B` | `I` | `2` (from freej2me-plus only) | static |  |
| 3 | `SND_5OCT_C` | `I` | `6` (from freej2me-plus only) | static |  |
| 3 | `SND_5OCT_CS` | `I` | `7` (from freej2me-plus only) | static |  |
| 3 | `SND_5OCT_D` | `I` | `8` (from freej2me-plus only) | static |  |
| 3 | `SND_5OCT_DS` | `I` | `9` (from freej2me-plus only) | static |  |
| 3 | `SND_5OCT_E` | `I` | `10` (from freej2me-plus only) | static |  |
| 3 | `SND_5OCT_F` | `I` | `11` (from freej2me-plus only) | static |  |
| 3 | `SND_5OCT_FS` | `I` | `12` (from freej2me-plus only) | static |  |
| 3 | `SND_5OCT_G` | `I` | `13` (from freej2me-plus only) | static |  |
| 3 | `SND_5OCT_GS` | `I` | `14` (from freej2me-plus only) | static |  |
| 3 | `SND_5OCT_A` | `I` | `3` (from freej2me-plus only) | static |  |
| 3 | `SND_5OCT_AS` | `I` | `4` (from freej2me-plus only) | static |  |
| 3 | `SND_5OCT_B` | `I` | `5` (from freej2me-plus only) | static |  |
| 3 | `SND_6OCT_C` | `I` | `18` (from freej2me-plus only) | static |  |
| 3 | `SND_6OCT_CS` | `I` | `19` (from freej2me-plus only) | static |  |
| 3 | `SND_6OCT_D` | `I` | `20` (from freej2me-plus only) | static |  |
| 3 | `SND_6OCT_DS` | `I` | `21` (from freej2me-plus only) | static |  |
| 3 | `SND_6OCT_E` | `I` | `22` (from freej2me-plus only) | static |  |
| 3 | `SND_6OCT_F` | `I` | `23` (from freej2me-plus only) | static |  |
| 3 | `SND_6OCT_FS` | `I` | `24` (from freej2me-plus only) | static |  |
| 3 | `SND_6OCT_G` | `I` | `25` (from freej2me-plus only) | static |  |
| 3 | `SND_6OCT_GS` | `I` | `26` (from freej2me-plus only) | static |  |
| 3 | `SND_6OCT_A` | `I` | `15` (from freej2me-plus only) | static |  |
| 3 | `SND_6OCT_AS` | `I` | `16` (from freej2me-plus only) | static |  |
| 3 | `SND_6OCT_B` | `I` | `17` (from freej2me-plus only) | static |  |
| 3 | `SND_7OCT_C` | `I` | `30` (from freej2me-plus only) | static |  |
| 3 | `SND_7OCT_CS` | `I` | `31` (from freej2me-plus only) | static |  |
| 3 | `SND_7OCT_D` | `I` | `32` (from freej2me-plus only) | static |  |
| 3 | `SND_7OCT_DS` | `I` | `33` (from freej2me-plus only) | static |  |
| 3 | `SND_7OCT_E` | `I` | `34` (from freej2me-plus only) | static |  |
| 3 | `SND_7OCT_F` | `I` | `35` (from freej2me-plus only) | static |  |
| 3 | `SND_7OCT_FS` | `I` | `36` (from freej2me-plus only) | static |  |
| 3 | `SND_7OCT_G` | `I` | `37` (from freej2me-plus only) | static |  |
| 3 | `SND_7OCT_GS` | `I` | `38` (from freej2me-plus only) | static |  |
| 3 | `SND_7OCT_A` | `I` | `27` (from freej2me-plus only) | static |  |

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void playBeep(int tone, int duration)` | `playBeep(II)V` | static |  |  |

### LED

`public final class mmpp.media.LED extends java.lang.Object`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/media/LED.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/mmpp/media/LED.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `int getColorDepth()` | `getColorDepth()I` | static |  |  |
| 3 | `int getNumber()` | `getNumber()I` | static |  |  |
| 3 | `int getColor(int LEDnumber)` | `getColor(I)I` | static |  |  |
| 3 | `void setColor(int LEDnumber, int RGBcolor)` | `setColor(II)V` | static |  |  |

### MediaPlayer

`public class mmpp.media.MediaPlayer extends java.lang.Object`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/media/MediaPlayer.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/mmpp/media/MediaPlayer.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/mmpp/media/MediaPlayer.java).

LG's sound player, used by games written for LG phones.

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void setMediaLocation(String location)` | `setMediaLocation(Ljava/lang/String;)V` |  | `IOException` |  |
| 3 | `void setMediaSource(byte[] buffer)` | `setMediaSource([B)V` |  |  |  |
| 3 | `void setMediaSource(byte[] buffer, int offset, int length)` | `setMediaSource([BII)V` |  |  |  |
| 3 | `boolean isPlayBackLoop()` | `isPlayBackLoop()Z` |  |  |  |
| 3 | `void setPlayBackLoop(boolean val)` | `setPlayBackLoop(Z)V` |  |  |  |
| 3 | `String getVolumeLevel()` | `getVolumeLevel()Ljava/lang/String;` |  |  |  |
| 3 | `void setVolumeLevel(String volumeString)` | `setVolumeLevel(Ljava/lang/String;)V` |  |  |  |
| 3 | `void start()` | `start()V` |  |  |  |
| 3 | `void stop()` | `stop()V` |  |  |  |
| 3 | `void pause()` | `pause()V` |  |  |  |
| 3 | `void resume()` | `resume()V` |  |  |  |

### mmpp.media.Vibration

`public final class mmpp.media.Vibration extends java.lang.Object`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/media/Vibration.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/mmpp/media/Vibration.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/mmpp/media/Vibration.java).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `int getLevelNum()` | `getLevelNum()I` | static |  |  |
| 3 | `void start(int level, int timeout)` | `start(II)V` | static |  |  |
| 3 | `void stop()` | `stop()V` | static |  |  |

### PhraseData

`public class mmpp.media.phrase.PhraseData extends java.lang.Object`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/media/phrase/PhraseData.html).

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `(byte[] phraseData)` | `<init>([B)V` |  | `NullPointerException`; `IllegalArgumentException` |  |
| 3 | `(String url)` | `<init>(Ljava/lang/String;)V` |  | `IOException`; `IllegalArgumentException` |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `int getSize()` | `getSize()I` |  |  |  |
| 3 | `int getUseTracks()` | `getUseTracks()I` |  |  |  |

### PhrasePlayer

`public class mmpp.media.phrase.PhrasePlayer extends java.lang.Object`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/media/phrase/PhrasePlayer.html).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `PhrasePlayer getPlayer()` | `getPlayer()Lmmpp/media/phrase/PhrasePlayer;` | static | `RuntimeException` |  |
| 3 | `void closePlayer()` | `closePlayer()V` |  | `RuntimeException` |  |
| 3 | `void closeTrack(PhraseTrack track)` | `closeTrack(Lmmpp/media/phrase/PhraseTrack;)V` |  |  |  |
| 3 | `PhraseTrack getTrack()` | `getTrack()Lmmpp/media/phrase/PhraseTrack;` |  |  |  |
| 3 | `int getTrackCount()` | `getTrackCount()I` |  |  |  |
| 3 | `void kill()` | `kill()V` |  | `RuntimeException` |  |
| 3 | `void pause()` | `pause()V` |  |  |  |
| 3 | `void resume()` | `resume()V` |  |  |  |
| 3 | `void handleEvent(int ch, int mode)` | `handleEvent(II)V` | static |  |  |

### PhraseTrack

`public class mmpp.media.phrase.PhraseTrack extends java.lang.Object`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/media/phrase/PhraseTrack.html).

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `DEFAULT_PANPOT` | `I` | `64` | static |  |
| 3 | `DEFAULT_VOLUME` | `I` | `100` | static |  |
| 3 | `NO_DATA` | `I` | `1` | static |  |
| 3 | `READY` | `I` | `2` | static |  |
| 3 | `PLAY` | `I` | `3` | static |  |
| 3 | `ENDING` | `I` | `4` | static |  |
| 3 | `PAUSE` | `I` | `5` | static |  |
| 3 | `MASTER` | `I` | `0` | static |  |
| 3 | `SLAVE` | `I` | `1` | static |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void setPhraseData(PhraseData data)` | `setPhraseData(Lmmpp/media/phrase/PhraseData;)V` |  | `NullPointerException`; `IllegalArgumentException`; `RuntimeException` |  |
| 3 | `PhraseData getPhraseData()` | `getPhraseData()Lmmpp/media/phrase/PhraseData;` |  |  |  |
| 3 | `void removePhraseData()` | `removePhraseData()V` |  | `RuntimeException` |  |
| 3 | `void setMaster(PhraseTrack master)` | `setMaster(Lmmpp/media/phrase/PhraseTrack;)V` |  | `RuntimeException` |  |
| 3 | `PhraseTrack getMaster()` | `getMaster()Lmmpp/media/phrase/PhraseTrack;` |  |  |  |
| 3 | `void setPhraseTrackListener(PhraseTrackListener listener)` | `setPhraseTrackListener(Lmmpp/media/phrase/PhraseTrackListener;)V` |  |  |  |
| 3 | `int getNumber()` | `getNumber()I` |  |  |  |
| 3 | `int getPanpot()` | `getPanpot()I` |  | `RuntimeException` |  |
| 3 | `void setPanpot(int panpot)` | `setPanpot(I)V` |  | `IllegalArgumentException`; `RuntimeException` |  |
| 3 | `int getVolume()` | `getVolume()I` |  | `RuntimeException` |  |
| 3 | `void setVolume(int volume)` | `setVolume(I)V` |  | `IllegalArgumentException`; `RuntimeException` |  |
| 3 | `boolean getMute()` | `getMute()Z` |  |  |  |
| 3 | `void setMute(boolean mute)` | `setMute(Z)V` |  |  |  |
| 3 | `int getState()` | `getState()I` |  |  |  |
| 3 | `void play(int loop)` | `play(I)V` |  | `IlligamArgumentException`; `RuntimeException` |  |
| 3 | `void play()` | `play()V` |  | `RuntimeException`; `IllegalArgumentException` |  |
| 3 | `void pause()` | `pause()V` |  | `RuntimeException` |  |
| 3 | `void resume()` | `resume()V` |  | `RuntimeException` |  |
| 3 | `void stop()` | `stop()V` |  | `RuntimeException` |  |
| 3 | `int getPlayingTime()` | `getPlayingTime()I` |  |  |  |
| 3 | `int getPosition()` | `getPosition()I` |  |  |  |
| 3 | `void seek(int pos)` | `seek(I)V` |  | `RuntimeException` |  |
| 3 | `int getIsMaster()` | `getIsMaster()I` |  |  |  |

### PhraseTrackListener

`public interface mmpp.media.phrase.PhraseTrackListener`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/media/phrase/PhraseTrackListener.html).

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void eventNotify(int trackNumber, int event)` | `eventNotify(II)V` |  |  |  |

### GraphicsX

`public class mmpp.microedition.lcdui.GraphicsX extends javax.microedition.lcdui.Graphics`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/microedition/lcdui/GraphicsX.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/mmpp/microedition/lcdui/GraphicsX.java).

Extends `Graphics`. The javadoc says every `Graphics` object on LG phones is a `GraphicsX`, and games get one by casting `(GraphicsX) g`; FastEmu's `Graphics` would have to be a `GraphicsX` for those games.

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `DEFAULT_ALPHA` | `I` | (not a constant) | static not final |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `Image capture(int x, int y, int width, int height)` | `capture(IIII)Ljavax/microedition/lcdui/Image;` |  | `IllegalArgumentExcepton` |  |
| 3 | `void setXORMode(int RGB)` | `setXORMode(I)V` |  |  |  |
| 3 | `void setPaintMode()` | `setPaintMode()V` |  |  |  |
| 3 | `void setAlpha(int alpha)` | `setAlpha(I)V` |  | `IllegalArgumentException` |  |
| 3 | `int getPixel(int x, int y)` | `getPixel(II)I` |  | `IllegalArgumentException` |  |
| 3 | `void setPixel(int x, int y, int RGB)` | `setPixel(III)V` |  |  |  |
| 3 | `void drawPolyline(int[] xPoints, int[] yPoints, int nPoints)` | `drawPolyline([I[II)V` |  |  |  |
| 3 | `void drawPolygon(int[] xPoints, int[] yPoints, int nPoints)` | `drawPolygon([I[II)V` |  |  |  |
| 3 | `void fillPolygon(int[] xPoints, int[] yPoints, int nPoints)` | `fillPolygon([I[II)V` |  |  |  |

### TextFieldX

`public class mmpp.microedition.lcdui.TextFieldX extends java.lang.Object`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/microedition/lcdui/TextFieldX.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/mmpp/microedition/lcdui/TextFieldX.java).

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `IM_NONE` | `I` | (value not in the sources) | static |  |
| 3 | `IM_ROMAN_CAPS` | `I` | (value not in the sources) | static |  |
| 3 | `IM_ROMAN_SMALL` | `I` | (value not in the sources) | static |  |
| 3 | `IM_NUMERIC` | `I` | (value not in the sources) | static |  |
| 3 | `IM_SYMBOL` | `I` | (value not in the sources) | static |  |
| 3 | `IM_KOREAN` | `I` | (value not in the sources) | static |  |
| 3 | `eT` | `Lmmpp/microedition/lcdui/ET;` | (not a constant) | protected not final |  |

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `(String label, String text, int maxSize, int constraints)` | `<init>(Ljava/lang/String;Ljava/lang/String;II)V` |  | `IllegalArgumentException`; `IllegalArgumentException`; `IllegalArgumentException`; `IllegalArgumentException` |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `void setOwner(Canvas owner)` | `setOwner(Ljavax/microedition/lcdui/Canvas;)V` |  |  |  |
| 3 | `Canvas getOwner()` | `getOwner()Ljavax/microedition/lcdui/Canvas;` |  |  |  |
| 3 | `int getWidth()` | `getWidth()I` |  |  |  |
| 3 | `void setWidth(int width)` | `setWidth(I)V` |  | `IllegalArgumentException` |  |
| 3 | `int getHeight()` | `getHeight()I` |  |  |  |
| 3 | `int getMaxRow()` | `getMaxRow()I` |  |  |  |
| 3 | `void setMaxRow(int maxRow)` | `setMaxRow(I)V` |  | `IllegalArgumentException` |  |
| 3 | `Font getFont()` | `getFont()Ljavax/microedition/lcdui/Font;` |  |  |  |
| 3 | `void setFont(Font newFont)` | `setFont(Ljavax/microedition/lcdui/Font;)V` |  |  |  |
| 3 | `int nextInputMode()` | `nextInputMode()I` |  |  |  |
| 3 | `int getInputMode()` | `getInputMode()I` |  |  |  |
| 3 | `String getString()` | `getString()Ljava/lang/String;` |  |  |  |
| 3 | `void setString(String text)` | `setString(Ljava/lang/String;)V` |  | `IllegalArgumentException`; `IllegalArgumentException` |  |
| 3 | `int getChars(char[] data)` | `getChars([C)I` |  | `ArrayIndexOutOfBoundsException` |  |
| 3 | `void setChars(char[] data, int offset, int length)` | `setChars([CII)V` |  | `ArrayIndexOutOfBoundsException`; `IllegalArgumentException`; `IllegalArgumentException` |  |
| 3 | `void insert(String src, int position)` | `insert(Ljava/lang/String;I)V` |  | `IllegalArgumentException`; `IllegalArgumentException` |  |

### ContentsManager

`public class mmpp.phone.ContentsManager extends java.lang.Object`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/phone/ContentsManager.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/mmpp/phone/ContentsManager.java).

| T | Field | Type | Value | Flags | Notes |
|---|---|---|---|---|---|
| 3 | `SUCCESS` | `I` | (not a constant) | static not final |  |
| 3 | `CONTENTS_FULL` | `I` | (not a constant) | static not final |  |
| 3 | `FAILURE` | `I` | (not a constant) | static not final |  |

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `int setCurrentMelody(String name, byte[] contents, int offset, int len)` | `setCurrentMelody(Ljava/lang/String;[BII)I` | static | `IllegalArgumentException` |  |
| 3 | `int setCurrentDesktop(String name, byte[] contents, int offset, int len)` | `setCurrentDesktop(Ljava/lang/String;[BII)I` | static | `IllegalArgumentException` |  |
| 3 | `String[] getListOfMelody()` | `getListOfMelody()[Ljava/lang/String;` | static |  |  |
| 3 | `String[] getListOfDesktop()` | `getListOfDesktop()[Ljava/lang/String;` | static |  |  |
| 3 | `int getNumberOfMelody()` | `getNumberOfMelody()I` | static |  |  |
| 3 | `int getNumberOfDesktop()` | `getNumberOfDesktop()I` | static |  |  |

### Phone

`public final class mmpp.phone.Phone extends java.lang.Object`

Tier 3. Javadoc: [LG_MMPP_API](https://nikita36078.github.io/J2ME_Docs/docs/LG_MMPP_API/mmpp/phone/Phone.html). Implementations: [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/mmpp/phone/Phone.java).

| T | Constructor | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  |  |  |

| T | Method | Descriptor | Flags | Throws | Notes |
|---|---|---|---|---|---|
| 3 | `String getProperty(String key)` | `getProperty(Ljava/lang/String;)Ljava/lang/String;` | static |  |  |
| 3 | `void placeCall(String phoneNumber)` | `placeCall(Ljava/lang/String;)V` | static |  |  |
| 3 | `void invokeWAPBrowser(String url)` | `invokeWAPBrowser(Ljava/lang/String;)V` | static |  |  |
| 3 | `int getNID()` | `getNID()I` | static |  |  |
| 3 | `int getSID()` | `getSID()I` | static |  |  |
| 3 | `int getBASEID()` | `getBASEID()I` | static |  |  |
| 3 | `int getBASELAT()` | `getBASELAT()I` | static |  |  |
| 3 | `int getBASELONG()` | `getBASELONG()I` | static |  |  |

## Other vendor APIs

These exist, and J2ME Loader or freej2me-plus implement some of them,
but they are outside FastEmu's first versions:

- **Motorola iDEN** (`com.motorola.iden.*`, `com.mot.iden.*`): APIs of
  Motorola's iDEN network phones; javadoc in
  [J2ME_Docs Motorola_iDEN_API](https://nikita36078.github.io/J2ME_Docs/docs/Motorola_iDEN_API/overview-summary.html).
- **Motorola 3D** (`com.motorola.graphics.j3d`): a Mascot Capsule V3
  clone; see [jsr-later.md](jsr-later.md).
- **Samsung extras** in freej2me-plus (`com.samsung.util.System`,
  `Muglet`) that the Samsung 1.0 javadoc does not list.
- **Sony Ericsson** shipped mostly standard JSRs; its own
  `com.sonyericsson.accelerometer` and Capuchin (Flash, above) are rare in
  games.
- **Japanese and Korean carrier APIs**: Vodafone and J-Phone
  (`com.vodafone.*`, `com.j_phone.*`), KDDI (`com.kddi.*`), NTT DoCoMo
  DoJa (`com.nttdocomo.*`, a different application model, not MIDP), SKT
  and KTF WIPI. freej2me-plus implements several; they matter only for
  games from those markets.

## Cross-check

Every member above comes from the javadoc named first under Sources. The script that writes this file compared it with the stub and implementation sources listed under Sources: constant values, member lists and `static` flags. Member lists are compared only where the parsed source holds the implementation's whole class hierarchy. What differs is listed here; a class not listed matched everywhere it was compared.

- `com.siemens.mp.game.ExtendedImage`
  - [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/game/ExtendedImage.java) does not declare `blitToScreen(II)V`
- `com.siemens.mp.game.GraphicObject`
  - microemu-siemensapi.jar also declares `paint(Ljavax/microedition/lcdui/Graphics;)V`
- `com.siemens.mp.game.Melody`
  - microemu-siemensapi.jar also declares `<init>()V`
- `com.siemens.mp.game.Sprite`
  - microemu-siemensapi.jar also declares `paint(Ljavax/microedition/lcdui/Graphics;)V`
- `com.siemens.mp.game.TiledBackground`
  - microemu-siemensapi.jar also declares `paint(Ljavax/microedition/lcdui/Graphics;)V`
- `com.siemens.mp.color_game.GameCanvas`
  - microemu-siemensapi.jar also declares `hideNotify()V`, `keyReleased(I)V`
- `com.siemens.mp.color_game.Layer`
  - microemu-siemensapi.jar does not declare `NativeMemoryTable`, `setHeight(I)V`, `setWidth(I)V`
  - [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/color_game/Layer.java) does not declare `NativeMemoryTable`, `copyAllLayerVariables(Lcom/siemens/mp/color_game/Layer;)V`, `getLayerImage()Ljavax/microedition/lcdui/Image;`, `setLayerImage(Ljavax/microedition/lcdui/Image;)V`
  - [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/color_game/Layer.java) does not declare `NativeMemoryTable`, `getLayerImage()Ljavax/microedition/lcdui/Image;`, `setLayerImage(Ljavax/microedition/lcdui/Image;)V`
  - [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/color_game/Layer.java) does not declare `NativeMemoryTable`, `setHeight(I)V`, `setWidth(I)V`
- `com.siemens.mp.color_game.LayerManager`
  - microemu-siemensapi.jar does not declare `NativeMemoryTable`, `removeLayer(Lcom/siemens/mp/misc/NativeMem;Lcom/siemens/mp/color_game/Layer;)V`
  - [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/color_game/LayerManager.java) does not declare `NativeMemoryTable`, `removeLayer(Lcom/siemens/mp/misc/NativeMem;Lcom/siemens/mp/color_game/Layer;)V`
  - [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/color_game/LayerManager.java) does not declare `NativeMemoryTable`
  - [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/color_game/LayerManager.java) does not declare `NativeMemoryTable`, `removeLayer(Lcom/siemens/mp/misc/NativeMem;Lcom/siemens/mp/color_game/Layer;)V`
- `com.siemens.mp.color_game.Sprite`
  - microemu-siemensapi.jar also declares `TRANS_MIRROR`, `TRANS_MIRROR_ROT180`, `TRANS_MIRROR_ROT270`, `TRANS_MIRROR_ROT90`, `TRANS_NONE`, `TRANS_ROT180`, `TRANS_ROT270`, `TRANS_ROT90`, `collidesWith(Lcom/siemens/mp/color_game/TiledLayer;Z)Z`, `defineCollisionRectangle(IIII)V`, `defineReferencePixel(II)V`, `getRefPixelX()I`, `getRefPixelY()I`, `setRefPixelPosition(II)V`, `setTransform(I)V`
  - microemu-siemensapi.jar does not declare `setCollisionRectangle(IIII)V`
  - [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/color_game/Sprite.java) does not declare `setCollisionRectangle(IIII)V`
- `com.siemens.mp.MIDlet`
  - freej2me-plus differs on `static` for `getAppProperty(Ljava/lang/String;)Ljava/lang/String;`
  - freej2me-plus differs on `static` for `notifyDestroyed()V`
  - freej2me-plus differs on `static` for `notifyPaused()V`
  - freej2me-plus differs on `static` for `platformRequest(Ljava/lang/String;)Z`
- `com.siemens.mp.NotAllowedException`
  - microemu-siemensapi.jar does not declare `<init>(Ljava/lang/String;)V`
- `com.siemens.mp.io.Connection`
  - freej2me-plus differs on `static` for `setListener(Lcom/siemens/mp/io/ConnectionListener;)V`
- `com.siemens.mp.io.File`
  - microemu-siemensapi.jar does not declare `INSIDE_STORAGE_PATH`, `OUTSIDE_STORAGE_PATH`, `STORAGE_DRIVE`, `buildPath(Ljava/lang/String;)Ljava/lang/String;`, `checkFileName(Ljava/lang/String;)I`
  - [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/io/File.java) does not declare `INSIDE_STORAGE_PATH`, `OUTSIDE_STORAGE_PATH`, `STORAGE_DRIVE`, `buildPath(Ljava/lang/String;)Ljava/lang/String;`, `checkFileName(Ljava/lang/String;)I`
- `com.siemens.mp.lcdui.Image`
  - [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/lcdui/Image.java) does not declare `createImageFromFile(Ljava/lang/String;II)Ljavax/microedition/lcdui/Image;`, `getPixelColor(Ljavax/microedition/lcdui/Image;II)I`, `writeBmpToFile(Ljavax/microedition/lcdui/Image;Ljava/lang/String;)V`
- `com.siemens.mp.m55.Ledcontrol`
  - `LED_TOP`: freej2me-plus has `1`, javadoc has `0`
  - `LED_BOTTOM`: freej2me-plus has `0`, javadoc has `1`
  - `P_SPEED`: freej2me-plus has `9`, javadoc has `0`
  - `P_LIGHTHOUSE`: freej2me-plus has `4`, javadoc has `1`
  - `P_WAVE`: freej2me-plus has `12`, javadoc has `2`
  - `P_BEAT`: freej2me-plus has `0`, javadoc has `3`
  - `P_RUNWAY`: freej2me-plus has `8`, javadoc has `4`
  - `P_TRANCE`: freej2me-plus has `11`, javadoc has `5`
  - `P_ETERNITY`: freej2me-plus has `2`, javadoc has `6`
  - `P_STROBO`: freej2me-plus has `10`, javadoc has `7`
  - `P_LIMELIGHT`: freej2me-plus has `5`, javadoc has `8`
  - `P_NORMAL_BLINKING`: freej2me-plus has `6`, javadoc has `9`
  - `P_PULSATING`: freej2me-plus has `7`, javadoc has `10`
  - `P_CONSTANLY_LITEUP`: freej2me-plus has `1`, javadoc has `11`
  - `P_IDLE`: freej2me-plus has `3`, javadoc has `12`
- `com.siemens.mp.ui.Image`
  - microemu-siemensapi.jar also declares `createImageFromBitmap([B[BII)Ljavax/microedition/lcdui/Image;`
  - microemu-siemensapi.jar does not declare `<init>()V`, `<init>([BIII)V`, `COLOR_BMP_8BIT`, `createImageWithScaling(Ljava/lang/String;)Ljavax/microedition/lcdui/Image;`, `createRGBImage([BIII)Ljavax/microedition/lcdui/Image;`, `mirrorImageHorizontally(Ljavax/microedition/lcdui/Image;)V`, `mirrorImageVertically(Ljavax/microedition/lcdui/Image;)V`, `setNativeImage(Ljavax/microedition/lcdui/Image;Lcom/siemens/mp/ui/Image;)V`
  - [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/ui/Image.java) does not declare `COLOR_BMP_8BIT`, `createImageWithScaling(Ljava/lang/String;)Ljavax/microedition/lcdui/Image;`, `createRGBImage([BIII)Ljavax/microedition/lcdui/Image;`, `getHeight()I`, `getNativeImage(Ljavax/microedition/lcdui/Image;)Lcom/siemens/mp/ui/Image;`, `getWidth()I`, `mirrorImageHorizontally(Ljavax/microedition/lcdui/Image;)V`, `mirrorImageVertically(Ljavax/microedition/lcdui/Image;)V`, `setNativeImage(Ljavax/microedition/lcdui/Image;Lcom/siemens/mp/ui/Image;)V`
  - [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/ui/Image.java) does not declare `COLOR_BMP_8BIT`, `createImageWithScaling(Ljava/lang/String;)Ljavax/microedition/lcdui/Image;`, `createRGBImage([BIII)Ljavax/microedition/lcdui/Image;`, `mirrorImageHorizontally(Ljavax/microedition/lcdui/Image;)V`, `mirrorImageVertically(Ljavax/microedition/lcdui/Image;)V`, `setNativeImage(Ljavax/microedition/lcdui/Image;Lcom/siemens/mp/ui/Image;)V`
- `com.siemens.mp.media.Manager`
  - microemu-siemensapi.jar does not declare `MIDI_DEVICE_LOCATOR`, `TONE_DEVICE_LOCATOR`
  - [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/media/Manager.java) does not declare `MIDI_DEVICE_LOCATOR`, `TONE_DEVICE_LOCATOR`, `getSystemTimeBase()Lcom/siemens/mp/media/TimeBase;`
  - [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/media/Manager.java) does not declare `getSystemTimeBase()Lcom/siemens/mp/media/TimeBase;`
  - [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/media/Manager.java) does not declare `MIDI_DEVICE_LOCATOR`, `TONE_DEVICE_LOCATOR`
- `com.siemens.mp.media.Player`
  - [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/siemens/mp/media/Player.java) does not declare `getTimeBase()Lcom/siemens/mp/media/TimeBase;`, `setTimeBase(Lcom/siemens/mp/media/TimeBase;)V`
  - [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/com/siemens/mp/media/Player.java) does not declare `getTimeBase()Lcom/siemens/mp/media/TimeBase;`, `setTimeBase(Lcom/siemens/mp/media/TimeBase;)V`
- `com.siemens.mp.media.PlayerListener`
  - `STARTED`: freej2me-plus has `"STARTED"`, javadoc has `"started"`
  - `STOPPED`: freej2me-plus has `"STOPPED"`, javadoc has `"stopped"`
  - `STOPPED_AT_TIME`: freej2me-plus has `"STOPPEDATTIME"`, javadoc has `"stoppedAtTime"`
  - `END_OF_MEDIA`: freej2me-plus has `"ENDOFMEDIA"`, javadoc has `"endOfMedia"`
  - `DURATION_UPDATED`: freej2me-plus has `"DURATIONUPDATED"`, javadoc has `"durationUpdated"`
  - `DEVICE_UNAVAILABLE`: freej2me-plus has `"DEVICEUNAVAILABLE"`, javadoc has `"deviceUnavailable"`
  - `DEVICE_AVAILABLE`: freej2me-plus has `"DEVICEAVAILABLE"`, javadoc has `"deviceAvailable"`
  - `VOLUME_CHANGED`: freej2me-plus has `"VOLUMECHANGED"`, javadoc has `"volumeChanged"`
  - `SIZE_CHANGED`: freej2me-plus has `"SIZECHANGED"`, javadoc has `"sizeChanged"`
  - `ERROR`: freej2me-plus has `"ERROR"`, javadoc has `"error"`
  - `CLOSED`: freej2me-plus has `"CLOSED"`, javadoc has `"closed"`
  - `RECORD_STARTED`: freej2me-plus has `"RECORDSTARTED"`, javadoc has `"recordStarted"`
  - `RECORD_STOPPED`: freej2me-plus has `"RECORDSTOPPED"`, javadoc has `"recordStopped"`
  - `RECORD_ERROR`: freej2me-plus has `"RECORDERROR"`, javadoc has `"recordError"`
  - `BUFFERING_STARTED`: freej2me-plus has `"BUFFERINGSTARTED"`, javadoc has `"bufferingStarted"`
  - `BUFFERING_STOPPED`: freej2me-plus has `"BUFFERINGSTOPPED"`, javadoc has `"bufferingStopped"`
- `com.siemens.mp.media.control.VolumeControl`
  - microemu-siemensapi.jar does not declare `getLevel()I`, `isMuted()Z`, `setLevel(I)I`, `setMute(Z)V`
  - [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-extensions/microemu-siemensapi/src/main/java/com/siemens/mp/media/control/VolumeControl.java) does not declare `getLevel()I`, `isMuted()Z`, `setLevel(I)I`, `setMute(Z)V`
- `mmpp.media.MediaPlayer`
  - [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/mmpp/media/MediaPlayer.java) does not declare `isPlayBackLoop()Z`, `setMediaSource([B)V`
- `mmpp.microedition.lcdui.TextFieldX`
  - [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/mmpp/microedition/lcdui/TextFieldX.java) does not declare `IM_KOREAN`, `IM_NONE`, `IM_NUMERIC`, `IM_ROMAN_CAPS`, `IM_ROMAN_SMALL`, `IM_SYMBOL`, `eT`, `getFont()Ljavax/microedition/lcdui/Font;`, `getHeight()I`, `getInputMode()I`, `getMaxRow()I`, `getOwner()Ljavax/microedition/lcdui/Canvas;`, `getWidth()I`, `nextInputMode()I`, `setFont(Ljavax/microedition/lcdui/Font;)V`, `setMaxRow(I)V`, `setOwner(Ljavax/microedition/lcdui/Canvas;)V`, `setWidth(I)V`
