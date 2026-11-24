# 4. Graphics

Fonts, anti-aliasing, drawing rules where implementations differ, image
decoding, `getRGB` and `drawRGB`, mutable images and Nokia
`DirectGraphics`. Link bases are as in [note 1](01-suite-loading.md).

## Font sizes

What each emulator picks for `SIZE_SMALL`, `SIZE_MEDIUM` and
`SIZE_LARGE`:

| Emulator | Chosen by | Unit | Values (small / medium / large) |
| --- | --- | --- | --- |
| J2ME Loader | larger screen side | pixel line height (`getHeight`) | up to 128: 9 / 13 / 15; up to 176: 13 / 15 / 20; up to 220: 15 / 18 / 22; above: 18 / 22 / 26 |
| J2ME Loader default profile (240x320) | per-game setting | pixel line height | 18 / 22 / 26 |
| freej2me-plus | smaller screen side | Java2D point size; `getHeight` comes from the font metrics and is larger | under 128: 8 / 9 / 11; under 176: 11 / 13 / 14; under 220: 12 / 13 / 15; otherwise 13 / 15 / 17; plus a per-game offset |
| MicroEmulator, 176x220 device | device XML | Java2D point size | 9 / 11 / 13 |
| MicroEmulator, 240x320 device | device XML | Java2D point size | 10 / 12 / 14 |
| FastEmu `fastemu-core` | profile preset | "pixel heights" | small screens 10 / 12 / 14; medium 12 / 14 / 16; large 14 / 16 / 20 |

Sources: [JL Font](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Font.java#L50),
[JL ProfileModel defaults](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/ru/playsoftware/j2meloader/config/ProfileModel.java#L191),
[FP PlatformFont](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/PlatformFont.java#L34),
[ME default device.xml](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-javase/src/main/resources/org/microemu/device/default/device.xml#L69). [code]

J2ME Loader sizes the Android font so that its line spacing equals the
chosen height exactly; `getBaselinePosition` is the rounded-up ascent and
widths are rounded up. [code] freej2me-plus takes height, ascent and
descent straight from Java2D. [code] None of the emulators has measured
values from real phones; all of these numbers are choices. [unverified]

What goes wrong with fonts in the trackers: text overflows menus when
fonts are too large (Harry Potter: Find Scabbers,
[freej2me-plus issue 18](https://github.com/TASEmulators/freej2me-plus/issues/18);
Townsmen text wrapping, [issue 67](https://github.com/TASEmulators/freej2me-plus/issues/67)),
which is why freej2me-plus has a per-game font size offset and J2ME
Loader per-game font sizes. [tracker] [code]
Games wrap text with `stringWidth` and size their boxes with
`getHeight`, so a wrong size is a layout bug, not only a look.

Smaller details:

- `Font.getFont(int fontSpecifier)` (`FONT_STATIC_TEXT`, `FONT_INPUT_TEXT`)
  returns the default font in J2ME Loader. [code]
- J2ME Loader's `isBold()` and `isItalic()` compare the style with `==`,
  so a bold italic font answers `false` to both. Compare bits. [code]
- freej2me-plus validates face, style and size and throws
  `IllegalArgumentException` for anything else. [code]
- `drawString` with `\n`: freej2me-plus added line breaks
  ([commit 98585bf](https://github.com/TASEmulators/freej2me-plus/commit/98585bf)). [tracker]
  Whether phones broke lines is unverified.

**For FastEmu:** document `FontSizes` as the pixel value of
`Font.getHeight()`. Keep per-profile values and add a per-game size
offset. Make `getBaselinePosition`, `charWidth`, `stringWidth` and
`substringWidth` exact integers from the same rasterizer that draws, so
measuring and drawing agree.

## Anti-aliasing

- J2ME Loader turns anti-aliasing **off** for lines and shapes
  ([Graphics constructor](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Graphics.java#L83))
  and makes text anti-aliasing a per-game setting, on by default. [code]
- freej2me-plus enables text anti-aliasing and leaves Java2D's shape
  anti-aliasing at its default, off. [code]
- MicroEmulator's device files ask for anti-aliased fonts. [code]

**For FastEmu:** never anti-alias shapes, lines or image scaling inside
the game's frame; games rely on exact pixels for borders and joins. Text
anti-aliasing can be a per-game option. Smoothing the scaled-up frame on
the host is a separate, app-side choice.

## Drawing rules where implementations differ

J2ME Loader has an instrumented test that pins several rules
([GraphicsTest](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/androidTest/java/javax/microedition/lcdui/GraphicsTest.java)). [code]
It is a good model for FastEmu's own tests:

- `drawLine(10, 4, 10, 15)` sets both end points.
- `drawImage(img, x, y, 0)`: anchor 0 means `TOP | LEFT`.
- `drawRGB(..., processAlpha = false)` stores every pixel with alpha
  0xFF; with `true`, a 0x75 alpha red over white blends to `0xFF8A8A`.
- `drawRegion(img, 1, 3, 5, 7, TRANS_MIRROR_ROT270, 9, 11, LEFT | VCENTER)`
  lands where the transformed region's width and height say.
- `setClip` then `clipRect` intersects.
- A negative clip width or height clips everything; it must not mirror
  the rectangle ([commit f7cfec50](https://github.com/nikita36078/J2ME-Loader/commit/f7cfec50)).

Other differences:

- **`setColor` and alpha.** `setColor(int)` ignores the top byte in J2ME
  Loader ([setColor](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Graphics.java#L127));
  only Nokia's `DirectGraphics.setARGBColor` draws with alpha. [code]
- **Text anchors.** J2ME Loader accepts any anchor for text, including
  `VCENTER`, and treats a missing horizontal or vertical part as `LEFT`
  or `TOP` ([commit 1d90eaf4](https://github.com/nikita36078/J2ME-Loader/commit/1d90eaf4)). [code]
  Throwing `IllegalArgumentException` for odd anchors crashes games
  written for lenient phones; accept them.
- **`drawRegion` checks.** freej2me-plus throws
  `IllegalArgumentException` when the source region leaves the image,
  and returns early for a zero-size region
  ([drawRegion](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/PlatformGraphics.java#L472));
  J2ME Loader throws for an unknown transform. [code]
- **`copyArea` outside the image.** Both throw
  `IllegalArgumentException`. Battle City crashed with freej2me-plus's
  message "Requested copy area exceeds the bounds of the image"
  ([issue 181](https://github.com/TASEmulators/freej2me-plus/issues/181),
  closed without a recorded fix). [code] [tracker]
- **Translation after `reset`.** freej2me-plus restores the translation
  that existed before the first reset of a Graphics object, unless the
  "translate to origin on reset" compatibility flag is set
  ([reset](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/PlatformGraphics.java#L202)). [code]
  The Graphics passed to `paint` should start at origin 0, 0 every time.
- **Per-game drawing flags in freej2me-plus**: "Siemens-friendly
  drawing" adds back a negative translation in `drawRegion`; "don't
  translate drawRGB" exists for the Sony Ericsson versions of Peggle;
  the "Fantasy Zone fix" adjusts the clip around Nokia
  `DirectGraphics.drawImage` ([PlatformGraphics](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/PlatformGraphics.java#L1308)). [code]
  They show that vendors differed in how translation applies to these
  calls. FastEmu should implement the spec and keep such flags as
  per-game settings, added only with a named game that needs them.

## Image creation and decoding

From the JSR 118 `Image` javadoc [spec]:

- `createImage(String)` throws `IOException` "if the resource does not
  exist, the data cannot be loaded, or the image data cannot be decoded".
- `createImage(byte[], int, int)` throws `IllegalArgumentException` if the
  data "cannot be decoded".
- `createImage(int, int)`: "Every pixel within the newly created image is
  white."
- PNG: all critical chunks, all bit depths, both interlace methods and
  palettes must be supported; "All implementations must support the tRNS
  chunk"; other ancillary chunks are silently ignored.

What breaks in practice:

- **Wrong exception for a missing image.** Blue Sphere Games titles
  (Mr. Bean Mini Racer, Burning Skies) load `0.png`, `1.png`, ... until
  `IOException`; freej2me-plus threw `NullPointerException`, which the
  games do not catch, so they never booted
  ([issue 119](https://github.com/TASEmulators/freej2me-plus/issues/119)). [tracker]
  J2ME Loader throws `IOException` for a missing resource and for an
  undecodable one ([createImage](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Image.java#L61)). [code]
- **Bad PNG checksums.** J2ME Loader decodes PNGs with pngj and CRC
  checking switched off ([PNGUtils.fixPNG](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/ru/playsoftware/j2meloader/util/PNGUtils.java#L66));
  before that it recalculated checksums of invalid images
  ([commit f1db198d](https://github.com/nikita36078/J2ME-Loader/commit/f1db198d),
  [commit 5b305369](https://github.com/nikita36078/J2ME-Loader/commit/5b305369)). [code]
  Phones did not check the CRC. [unverified on hardware]
- **tRNS on grayscale and truecolor PNGs.** Old Java decoders dropped the
  colour key, so freej2me-plus applies it itself; Sonic 1 Part 1
  (low end) drew a white box around a sprite without it
  ([PNGUtility](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/PNGUtility.java#L75),
  [issue 250](https://github.com/TASEmulators/freej2me-plus/issues/250)).
  J2ME Loader converts palette, gray, 8-bit and 16-bit rows to ARGB
  itself and applies tRNS for each type. [code]
- **Low bit depths.** 1, 2 and 4-bit images need scaling up to 8 bits
  per sample ([commit a126433f](https://github.com/nikita36078/J2ME-Loader/commit/a126433f)). [code]
- **Other formats.** When the data is not a PNG, J2ME Loader passes it to
  Android's decoder, which reads JPEG, GIF and BMP; freej2me-plus uses
  Java ImageIO and adds an IFF/ILBM reader. [code] Vendors accepted
  JPEG and often GIF and BMP. [unverified per vendor]
- **`createImage(Image)`** of an immutable image may return the same
  object (J2ME Loader does). [code]

**For FastEmu:** decode PNGs with CRC checks off; support every colour
type, bit depth and interlace mode; apply tRNS everywhere; fall back to
JPEG, GIF and BMP decoders by magic bytes; tolerate a missing `IEND` if
the pixel data is complete [unverified that games need it]; throw
`IOException` from `createImage(String)` and `createImage(InputStream)`,
`IllegalArgumentException` from `createImage(byte[], ...)`.

## getRGB and drawRGB

- JSR 118: "Every pixel within a mutable image is always fully opaque."
  [spec] J2ME Loader's test checks that `drawRGB` into a mutable image
  leaves alpha 0xFF. [code]
- `drawRGB` with `processAlpha = false` must ignore the alpha byte;
  J2ME Loader ORs in 0xFF000000 ([drawRGB](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Graphics.java#L500)). [code]
- Bounds: J2ME Loader throws `ArrayIndexOutOfBoundsException` when
  `offset` and `scanlength` reach outside the array, for positive and
  negative `scanlength`; it logs and continues for a negative width or
  height. freej2me-plus throws for a bad offset and clips the
  destination to the clip rectangle. [code]
- Colour distortion in `drawRGB` was a J2ME Loader bug once
  ([commit 59330387](https://github.com/nikita36078/J2ME-Loader/commit/59330387)). [code]
- `getRGB` returns the stored ARGB value; for immutable images with
  partial alpha, JSR 118 allows the alpha to be resampled. [spec]

**For FastEmu:** store immutable images as ARGB 8888 and mutable images
as opaque RGB with alpha forced to 0xFF on every write. Use straight
(not premultiplied) alpha so `getRGB` returns what `createRGBImage`
received.

## Mutable images

- `createImage(w, h)` is white and opaque. J2ME Loader fills with white;
  freej2me-plus fills with white and has a speed setting that stores
  blank images as RGB without alpha
  ([PlatformImage](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/PlatformImage.java#L80),
  [commit e5befdd](https://github.com/TASEmulators/freej2me-plus/commit/e5befdd)). [code]
- Nokia's `DirectUtils.createImage(w, h, argb)` makes a mutable image
  that **can** be transparent; J2ME Loader fills it with the given ARGB
  value ([DirectUtils](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/ui/DirectUtils.java)). [code]
  Games use it for sprites built at run time.
- `getGraphics()` on an immutable image throws `IllegalStateException`
  (freej2me-plus). [code]

## Nokia DirectGraphics

- **Manipulation flags.** `ROTATE_90` turns counter-clockwise, so it
  maps to `Sprite.TRANS_ROT270`; `FLIP_HORIZONTAL` is 0x2000 and
  `FLIP_VERTICAL` 0x4000, and combinations map as in J2ME Loader's table
  ([MANIPULATION2TRANSFORM](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/ui/DirectGraphicsImp.java#L43)).
  An angle that is not a multiple of 90 throws
  `IllegalArgumentException`. [code] freej2me-plus had several rounds of
  fixes for the 90 and 270 degree mirror cases
  ([commit 4528ba8](https://github.com/TASEmulators/freej2me-plus/commit/4528ba8)). [code]
- **Pixel formats.** `TYPE_BYTE_1_GRAY` (1), `TYPE_BYTE_1_GRAY_VERTICAL`
  (-1), `TYPE_BYTE_2_GRAY`, `TYPE_BYTE_4_GRAY`, `TYPE_BYTE_8_GRAY`,
  `TYPE_BYTE_332_RGB`, `TYPE_USHORT_4444_ARGB`, `TYPE_USHORT_444_RGB`,
  `TYPE_USHORT_555_RGB`, `TYPE_USHORT_1555_ARGB`, `TYPE_USHORT_565_RGB`,
  `TYPE_INT_888_RGB`, `TYPE_INT_8888_ARGB`, with the numeric values of
  their names. [code] In the 1-bit gray format a set bit is **black**,
  and in the transparency mask a set bit is opaque
  ([DirectGraphicsImp.getPixel](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/ui/DirectGraphicsImp.java#L56)). [code]
- **`getNativePixelFormat`.** J2ME Loader returns 565 by default and lets
  the user change it with the system property
  `com.nokia.mid.ui.DirectGraphics.PIXEL_FORMAT`
  ([PIXEL_FORMAT](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/com/nokia/mid/ui/DirectGraphicsImp.java#L41)).
  It used to return 8888, and Nokia's Virtual Me then crashed with
  "Illegal format: 8888" because it passes the native format to the
  `short[]` variant of `getPixels`
  ([issue 802](https://github.com/nikita36078/J2ME-Loader/issues/802)). [code] [tracker]
  freej2me-plus returns 0, which is not a valid format
  ([getNativePixelFormat](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/PlatformGraphics.java#L1308)). [code]
- **Alpha.** `setARGBColor` and the colour arguments of `fillTriangle`
  and the polygon calls carry alpha; `getAlphaComponent` returns it. [code]

**For FastEmu:** add the native pixel format to `DeviceProfile` (444 for
4096-colour Series 40 phones, 565 for 65k-colour phones [unverified per
model]), defaulting to 565, never 8888 or 0. Support every format in
both `drawPixels` and `getPixels`, for `byte[]`, `short[]` and `int[]`.

## Display colour reports

`Display.numColors()` returns `Integer.MAX_VALUE` in J2ME Loader and
freej2me-plus; `isColor()` is `true`; `numAlphaLevels()` is 256. [code]
`getBestImageWidth` and `getBestImageHeight` return 0 in J2ME Loader
and screen-derived sizes in freej2me-plus. [code] FastEmu's profile
already carries `colors` and `alpha_levels`; report them, and return
real sizes from the `getBestImage*` methods.
