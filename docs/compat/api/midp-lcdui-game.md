# javax.microedition.lcdui.game (MIDP 2.0)

`javax.microedition.lcdui.game`, new in MIDP 2.0: a canvas with its own
off-screen buffer and polled key state, and sprites and tile maps that
draw themselves. Most MIDP 2.0 action games use `GameCanvas`; many use
`Sprite`, `TiledLayer` and `LayerManager` as well. Games for MIDP 1.0
phones, and for Siemens and Nokia APIs, wrote their own equivalents (see
[vendors.md](vendors.md) for Siemens' older `com.siemens.mp.game`).

The eight `Sprite.TRANS_*` constants are also the `transform` argument of
`Graphics.drawRegion` and `Image.createImage(Image, int, int, int, int,
int)`. A transform with `ROT90` or `ROT270` swaps the width and height.

Pixel-level collision (`collidesWith(..., true)`) compares opaque pixels
inside both collision rectangles, after transforms. The spec does not say
what partly transparent pixels do; both
[J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/game/Sprite.java)
and
[freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/game/Sprite.java)
count a pixel as opaque only when its alpha is `0xFF`, and FastEmu should
too.

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
| [GameCanvas](#gamecanvas) | class | 1 | MIDP 2.0 | 15 |
| [Layer](#layer) | class | 1 | MIDP 2.0 | 9 |
| [LayerManager](#layermanager) | class | 2 | MIDP 2.0 | 8 |
| [Sprite](#sprite) | class | 1 | MIDP 2.0 | 29 |
| [TiledLayer](#tiledlayer) | class | 1 | MIDP 2.0 | 13 |

## javax.microedition.lcdui.game

### GameCanvas

`public abstract class javax.microedition.lcdui.game.GameCanvas extends javax.microedition.lcdui.Canvas`

Tier 1. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/game/GameCanvas.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/game/GameCanvas.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/game/GameCanvas.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/game/GameCanvas.java).

A `Canvas` that owns an off-screen buffer of its full size, initially white. The usual loop is: read `getKeyStates()`, draw with the `Graphics` from `getGraphics()`, call `flushGraphics()`, sleep.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 1 | `UP_PRESSED` | `I` | `2` | static | 2.0 | `1 << Canvas.UP`. Every `*_PRESSED` bit is 1 shifted left by its game action. |
| 1 | `DOWN_PRESSED` | `I` | `64` | static | 2.0 |  |
| 1 | `LEFT_PRESSED` | `I` | `4` | static | 2.0 |  |
| 1 | `RIGHT_PRESSED` | `I` | `32` | static | 2.0 |  |
| 1 | `FIRE_PRESSED` | `I` | `256` (`0x100`) | static | 2.0 |  |
| 2 | `GAME_A_PRESSED` | `I` | `512` (`0x200`) | static | 2.0 |  |
| 2 | `GAME_B_PRESSED` | `I` | `1024` (`0x400`) | static | 2.0 |  |
| 2 | `GAME_C_PRESSED` | `I` | `2048` (`0x800`) | static | 2.0 |  |
| 2 | `GAME_D_PRESSED` | `I` | `4096` (`0x1000`) | static | 2.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `(boolean suppressKeyEvents)` | `<init>(Z)V` | protected | 2.0 |  | `protected`. `true` suppresses `keyPressed`, `keyRepeated` and `keyReleased` for game-action keys while this canvas is shown; other keys still arrive. |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `Graphics getGraphics()` | `getGraphics()Ljavax/microedition/lcdui/Graphics;` | protected | 2.0 |  | `protected`. A new `Graphics` on the buffer at every call; drawing shows only after `flushGraphics`. |
| 1 | `int getKeyStates()` | `getKeyStates()I` |  | 2.0 |  | One bit per game action (`*_PRESSED`). A bit is set if the key is down now or was pressed since the previous call, and the call clears the latched bits, so a quick tap between two polls is never lost. Returns 0 while the canvas is hidden. |
| 2 | `void paint(Graphics g)` | `paint(Ljavax/microedition/lcdui/Graphics;)V` |  | 2.0 | `NullPointerException` if g is null | Draws the buffer. A subclass that overrides it and still wants the buffer must call `super.paint(g)`. |
| 2 | `void flushGraphics(int x, int y, int width, int height)` | `flushGraphics(IIII)V` |  | 2.0 |  | Copies only the given region. |
| 1 | `void flushGraphics()` | `flushGraphics()V` |  | 2.0 |  | Copies the whole buffer to the screen and returns only when done; the buffer keeps its contents. Does nothing when the canvas is hidden. |

### Layer

`public abstract class javax.microedition.lcdui.game.Layer extends java.lang.Object`

Tier 1. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/game/Layer.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/game/Layer.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/game/Layer.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/game/Layer.java).

Abstract base of `Sprite` and `TiledLayer`; no public constructor.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `void setPosition(int x, int y)` | `setPosition(II)V` |  | 2.0 |  |  |
| 1 | `void move(int dx, int dy)` | `move(II)V` |  | 2.0 |  |  |
| 1 | `int getX()` | `getX()I` | final | 2.0 |  |  |
| 1 | `int getY()` | `getY()I` | final | 2.0 |  |  |
| 1 | `int getWidth()` | `getWidth()I` | final | 2.0 |  |  |
| 1 | `int getHeight()` | `getHeight()I` | final | 2.0 |  |  |
| 2 | `void setVisible(boolean visible)` | `setVisible(Z)V` |  | 2.0 |  |  |
| 2 | `boolean isVisible()` | `isVisible()Z` | final | 2.0 |  |  |
| 1 | `void paint(Graphics g)` | `paint(Ljavax/microedition/lcdui/Graphics;)V` | abstract | 2.0 | `NullPointerException` if g is null | `abstract`. Draws at the layer's position, relative to the graphics translation; invisible layers draw nothing. |

### LayerManager

`public class javax.microedition.lcdui.game.LayerManager extends java.lang.Object`

Tier 2. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/game/LayerManager.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/game/LayerManager.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/game/LayerManager.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/game/LayerManager.java).

Draws layers back to front: index 0 is drawn last, on top.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 2.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void append(Layer l)` | `append(Ljavax/microedition/lcdui/game/Layer;)V` |  | 2.0 | `NullPointerException` if the Layer is null |  |
| 3 | `void insert(Layer l, int index)` | `insert(Ljavax/microedition/lcdui/game/Layer;I)V` |  | 2.0 | `NullPointerException` if the Layer is null; `IndexOutOfBoundsException` if the index is less than 0 or greater than the number of Layers already added to the this LayerManager |  |
| 3 | `Layer getLayerAt(int index)` | `getLayerAt(I)Ljavax/microedition/lcdui/game/Layer;` |  | 2.0 | `IndexOutOfBoundsException` if the specified index is less than zero, or if it is equal to or greater than the number of Layers added to the this LayerManager |  |
| 3 | `int getSize()` | `getSize()I` |  | 2.0 |  |  |
| 2 | `void remove(Layer l)` | `remove(Ljavax/microedition/lcdui/game/Layer;)V` |  | 2.0 | `NullPointerException` if the specified Layer is null | Removing a layer that is not present does nothing. |
| 2 | `void paint(Graphics g, int x, int y)` | `paint(Ljavax/microedition/lcdui/Graphics;II)V` |  | 2.0 | `NullPointerException` if g is null |  |
| 2 | `void setViewWindow(int x, int y, int width, int height)` | `setViewWindow(IIII)V` |  | 2.0 | `IllegalArgumentException` if the width or height is less than 0 | The part of the layer world `paint` shows; `paint(g, x, y)` puts that window's top left at `(x, y)` on the graphics. |

### Sprite

`public class javax.microedition.lcdui.game.Sprite extends javax.microedition.lcdui.game.Layer`

Tier 1. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/game/Sprite.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/game/Sprite.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/game/Sprite.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/game/Sprite.java).

Frames are equal-sized cells of one image, read left to right, top to bottom. The image width and height must be whole multiples of the frame size.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 1 | `TRANS_NONE` | `I` | `0` | static | 2.0 |  |
| 2 | `TRANS_ROT90` | `I` | `5` | static | 2.0 |  |
| 2 | `TRANS_ROT180` | `I` | `3` | static | 2.0 |  |
| 2 | `TRANS_ROT270` | `I` | `6` | static | 2.0 |  |
| 1 | `TRANS_MIRROR` | `I` | `2` | static | 2.0 |  |
| 2 | `TRANS_MIRROR_ROT90` | `I` | `7` | static | 2.0 |  |
| 2 | `TRANS_MIRROR_ROT180` | `I` | `1` | static | 2.0 |  |
| 2 | `TRANS_MIRROR_ROT270` | `I` | `4` | static | 2.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(Image image)` | `<init>(Ljavax/microedition/lcdui/Image;)V` |  | 2.0 | `NullPointerException` if img is null | One frame: the whole image. |
| 1 | `(Image image, int frameWidth, int frameHeight)` | `<init>(Ljavax/microedition/lcdui/Image;II)V` |  | 2.0 | `NullPointerException` if img is null; `IllegalArgumentException` if frameHeight or frameWidth is less than 1; `IllegalArgumentException` if the image width is not an integer multiple of the frameWidth; `IllegalArgumentException` if the image height is not an integer multiple of the frameHeight |  |
| 3 | `(Sprite s)` | `<init>(Ljavax/microedition/lcdui/game/Sprite;)V` |  | 2.0 | `NullPointerException` if s is null | Copy constructor. |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void defineReferencePixel(int x, int y)` | `defineReferencePixel(II)V` |  | 2.0 |  | Relative to the untransformed frame; does not move the sprite. |
| 2 | `void setRefPixelPosition(int x, int y)` | `setRefPixelPosition(II)V` |  | 2.0 |  |  |
| 2 | `int getRefPixelX()` | `getRefPixelX()I` |  | 2.0 |  |  |
| 2 | `int getRefPixelY()` | `getRefPixelY()I` |  | 2.0 |  |  |
| 1 | `void setFrame(int sequenceIndex)` | `setFrame(I)V` |  | 2.0 | `IndexOutOfBoundsException` if frameIndex is less than0; `IndexOutOfBoundsException` if frameIndex is equal to or greater than the length of the current frame sequence (or the number of raw frames for the default sequence) |  |
| 1 | `int getFrame()` | `getFrame()I` | final | 2.0 |  |  |
| 2 | `int getRawFrameCount()` | `getRawFrameCount()I` |  | 2.0 |  |  |
| 2 | `int getFrameSequenceLength()` | `getFrameSequenceLength()I` |  | 2.0 |  |  |
| 1 | `void nextFrame()` | `nextFrame()V` |  | 2.0 |  | Wraps to the start of the sequence. |
| 2 | `void prevFrame()` | `prevFrame()V` |  | 2.0 |  |  |
| 1 | `void paint(Graphics g)` | `paint(Ljavax/microedition/lcdui/Graphics;)V` | final | 2.0 | `NullPointerException` if g is null |  |
| 2 | `void setFrameSequence(int[] sequence)` | `setFrameSequence([I)V` |  | 2.0 | `ArrayIndexOutOfBoundsException` if seq is non-null and any member of the array has a value less than 0 or greater than or equal to the number of frames as reported by getRawFrameCount(); `IllegalArgumentException` if the array has less than 1 element | Copies the array; `null` restores the default order. Resets the index to 0. |
| 2 | `void setImage(Image img, int frameWidth, int frameHeight)` | `setImage(Ljavax/microedition/lcdui/Image;II)V` |  | 2.0 | `NullPointerException` if img is null; `IllegalArgumentException` if frameHeight or frameWidth is less than 1; `IllegalArgumentException` if the image width is not an integer multiple of the frameWidth; `IllegalArgumentException` if the image height is not an integer multiple of the frameHeight |  |
| 2 | `void defineCollisionRectangle(int x, int y, int width, int height)` | `defineCollisionRectangle(IIII)V` |  | 2.0 | `IllegalArgumentException` if the specified width or height is less than 0 | Relative to the untransformed frame; defaults to the whole frame. |
| 1 | `void setTransform(int transform)` | `setTransform(I)V` |  | 2.0 | `IllegalArgumentException` if the requested transform is invalid | Not cumulative. Keeps the reference pixel where it is on screen, so the sprite's `getX` and `getY` may change. |
| 1 | `boolean collidesWith(Sprite s, boolean pixelLevel)` | `collidesWith(Ljavax/microedition/lcdui/game/Sprite;Z)Z` | final | 2.0 | `NullPointerException` if Sprite s is null | Both sprites must be visible. |
| 2 | `boolean collidesWith(TiledLayer t, boolean pixelLevel)` | `collidesWith(Ljavax/microedition/lcdui/game/TiledLayer;Z)Z` | final | 2.0 | `NullPointerException` if t is null | Only non-empty cells (index not 0) can collide. |
| 3 | `boolean collidesWith(Image image, int x, int y, boolean pixelLevel)` | `collidesWith(Ljavax/microedition/lcdui/Image;IIZ)Z` | final | 2.0 | `NullPointerException` if image is null |  |

### TiledLayer

`public class javax.microedition.lcdui.game.TiledLayer extends javax.microedition.lcdui.game.Layer`

Tier 1. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/game/TiledLayer.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/game/TiledLayer.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/game/TiledLayer.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/game/TiledLayer.java).

A grid of cells. Cell value 0 is empty, positive values are static tiles (1-based, read from the image like sprite frames), negative values are animated tiles from `createAnimatedTile`.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `(int columns, int rows, Image image, int tileWidth, int tileHeight)` | `<init>(IILjavax/microedition/lcdui/Image;II)V` |  | 2.0 | `NullPointerException` if image is null; `IllegalArgumentException` if the number of rows or columns is less than 1; `IllegalArgumentException` if tileHeight or tileWidth is less than 1; `IllegalArgumentException` if the image width is not an integer multiple of the tileWidth; `IllegalArgumentException` if the image height is not an integer multiple of the tileHeight |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `int createAnimatedTile(int staticTileIndex)` | `createAnimatedTile(I)I` |  | 2.0 | `IndexOutOfBoundsException` if the staticTileIndex is invalid | Returns -1 for the first animated tile, then -2, and so on. |
| 2 | `void setAnimatedTile(int animatedTileIndex, int staticTileIndex)` | `setAnimatedTile(II)V` |  | 2.0 | `IndexOutOfBoundsException` if the staticTileIndex is invalid; `IndexOutOfBoundsException` if the animated tile index is invalid |  |
| 2 | `int getAnimatedTile(int animatedTileIndex)` | `getAnimatedTile(I)I` |  | 2.0 | `IndexOutOfBoundsException` if the animated tile index is invalid |  |
| 1 | `void setCell(int col, int row, int tileIndex)` | `setCell(III)V` |  | 2.0 | `IndexOutOfBoundsException` if there is no tile with index tileIndex; `IndexOutOfBoundsException` if row or col is outside the bounds of the TiledLayer grid |  |
| 1 | `int getCell(int col, int row)` | `getCell(II)I` |  | 2.0 | `IndexOutOfBoundsException` if row or col is outside the bounds of the TiledLayer grid |  |
| 2 | `void fillCells(int col, int row, int numCols, int numRows, int tileIndex)` | `fillCells(IIIII)V` |  | 2.0 | `IndexOutOfBoundsException` if the rectangular region defined by the parameters extends beyond the bounds of the TiledLayer grid; `IllegalArgumentException` if numCols is less than zero; `IllegalArgumentException` if numRows is less than zero; `IndexOutOfBoundsException` if there is no tile with index tileIndex |  |
| 2 | `int getCellWidth()` | `getCellWidth()I` | final | 2.0 |  |  |
| 2 | `int getCellHeight()` | `getCellHeight()I` | final | 2.0 |  |  |
| 2 | `int getColumns()` | `getColumns()I` | final | 2.0 |  |  |
| 2 | `int getRows()` | `getRows()I` | final | 2.0 |  |  |
| 3 | `void setStaticTileSet(Image image, int tileWidth, int tileHeight)` | `setStaticTileSet(Ljavax/microedition/lcdui/Image;II)V` |  | 2.0 | `NullPointerException` if image is null; `IllegalArgumentException` if tileHeight or tileWidth is less than 1; `IllegalArgumentException` if the image width is not an integer multiple of the tileWidth; `IllegalArgumentException` if the image height is not an integer multiple of the tileHeight | Keeps the cells when the new set has at least as many tiles; otherwise clears every cell to 0 and drops the animated tiles. |
| 1 | `void paint(Graphics g)` | `paint(Ljavax/microedition/lcdui/Graphics;)V` | final | 2.0 | `NullPointerException` if g is null |  |

## Cross-check

Every member above comes from the spec javadoc. The script that writes this file compared it with the stub and implementation sources listed under Sources: constant values, member lists and `static` flags. Member lists are compared only where the parsed source holds the implementation's whole class hierarchy. What differs is listed here; a class not listed matched everywhere it was compared.

No differences.
