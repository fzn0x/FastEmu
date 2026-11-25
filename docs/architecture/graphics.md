# Graphics: the MIDP rasterizer

`crates/fastemu-gfx` draws what MIDP 2.0's `javax.microedition.lcdui`
`Graphics`, `Image` and `Font` draw, plus Nokia's `DirectGraphics`. It is a
plain Rust library with no virtual machine in it: the MIDP crate wraps it
in Java classes. This page lists the rules it implements, pixel by pixel,
and how the wrapper is meant to use it.

## Pieces

| Type | MIDP | What it holds |
| --- | --- | --- |
| `Surface` | the screen, `Image` | width, height, `0xAARRGGBB` pixels, kind (screen, mutable, immutable), an "all opaque" flag |
| `Graphics` | `Graphics`, `DirectGraphics` | translation, clip, colour, grey level, stroke style, font, anchor policy |
| `FontLibrary` | `Font.getFont` | the phone's `FontSizes` and text rendering mode; hands out `Font`s |
| `Font` | `Font` | face, style, size and a shared glyph cache |
| `Surface::decode` | `Image.createImage(byte[]...)` | PNG, JPEG, GIF and BMP decoders |
| `nokia` | `com.nokia.mid.ui` | Nokia constants and `DirectGraphics` calls on `Graphics` |

A `Graphics` does not own or borrow its target. Every drawing call takes
the target `&mut Surface`, and images to draw as `&Surface`. The wrapper
keeps a `Graphics` in each Java `Graphics` object and looks up the target
surface per call. Drawing is always clipped to the surface actually
passed, so a wrong target can draw wrongly but never out of bounds.

Pixels are row-major `0xAARRGGBB` words that are not premultiplied, the
format of `fastemu_core::Frame`; `Surface::to_frame` hands the screen to
an app.

## The pixel model

MIDP names a pixel by the grid point at its top-left corner: drawing at
`(x, y)` touches the pixel below and to the right of that point. All
coordinates are translated by `translate(x, y)` (which wraps around like
Java's `int`) and then computed in 64-bit integers, so no Java argument
can overflow them. Only the part of a primitive inside the clip is ever
walked, so a line or circle millions of pixels long costs no more than
the clip's size, except that outlines wider or taller than 65 536 pixels
are not drawn at all.

### Coverage of each primitive

`w` and `h` are the width and height arguments.

| Call | Pixels covered | Nothing drawn when |
| --- | --- | --- |
| `drawLine(x1, y1, x2, y2)` | both end points and the pixels between, see below | never |
| `fillRect` | `[x, x + w) x [y, y + h)`: exactly `w` by `h` | `w <= 0` or `h <= 0` |
| `drawRect` | the outline of `[x, x + w] x [y, y + h]`: `w + 1` by `h + 1`; a zero size gives a line or one pixel | `w < 0` or `h < 0` |
| `drawRoundRect` | the outline over `w + 1` by `h + 1`, corners from an ellipse over `arcWidth + 1` by `arcHeight + 1` | `w < 0` or `h < 0` |
| `fillRoundRect` | `w` by `h`, corners cut by pixel centres against an `arcWidth` by `arcHeight` ellipse | `w <= 0` or `h <= 0` |
| `drawArc` | the part of the outline over `w + 1` by `h + 1` within the angles | `w < 0`, `h < 0` or `arcAngle == 0` |
| `fillArc` | pixels whose centre is in the ellipse filling `w` by `h` and within the angles | `w <= 0`, `h <= 0` or `arcAngle == 0` |
| `fillTriangle` | pixels whose top-left grid point is inside or on the triangle, plus the three sides as `drawLine` draws them | never (three equal points draw one pixel) |

**Lines.** A line steps one pixel at a time along its major axis (x when
it is at least as wide as tall). Pixel `i` has minor offset
`floor((2 i dmin + dmaj) / (2 dmaj))`, Bresenham's rounding, with halves
rounding away from the start. The start is always the end point with the
smaller major coordinate, so `drawLine(a, b)` and `drawLine(b, a)` cover
the same pixels.

**Ellipse outlines** (`drawArc`, `drawRoundRect` corners) use the integer
ellipse-in-a-rectangle algorithm from Alois Zingl's "A Rasterizing
Algorithm for Drawing Curves" (2012): the outline touches all four sides
of its pixel box for every odd or even size, and is symmetric both ways.
A zero width or height gives a straight line.

**Ellipse areas** (`fillArc`, `fillRoundRect` corners) take each pixel
whose centre lies inside or on the ellipse. In doubled coordinates about
the centre a pixel `(c, r)` of a `w` by `h` box has its centre at
`X = 2c + 1 - w`, `Y = 2r + 1 - h` and is inside when
`X^2 h^2 + Y^2 w^2 <= w^2 h^2`; each row's span is solved exactly with an
integer square root. `fillArc(x, y, 1, 1, 0, 360)` fills one pixel;
4 by 4 gives `.##. / #### / #### / .##.`.

**Round rectangles** clamp `arcWidth` to `[0, w]` and `arcHeight` to
`[0, h]`; a zero arc draws `drawRect` or `fillRect`. The outline puts the
left half of the corner ellipse at the left corners and the right half at
the right ones (the middle column goes to both), and joins them with
straight sides. Fills always cover the full height of the band between
the corners.

**Angles.** Degrees, 0 at three o'clock, counter-clockwise; a negative
`arcAngle` sweeps clockwise; `|arcAngle| >= 360` is the whole ellipse.
Angles are measured against the bounding box, so 45 degrees always points
at its top-right corner: a pixel offset `(dx, dy)` from the centre (y
down) has direction `(dx * h, -dy * w)`. A pixel is in the arc when that
direction lies between the start and end directions, both ends included,
tested with cross products and a tolerance of `1e-12` of the vector's
size so end points do not depend on a platform's sine and cosine. The
centre pixel of a pie is always filled.

**Triangles and polygons.** `fillTriangle` must include the lines between
its corners. Filling by pixel centres and then adding the lines leaves
gaps where an edge passes exactly through a centre, so FastEmu fills by
the grid point each pixel is named by: a pixel is inside when its
top-left corner is inside or on the shape. Rows are half-open per edge
(an edge counts on rows from its top end up to, not including, its bottom
end) and horizontal edges never count, so a vertex on a row is counted
once. Crossings are exact integer fractions. The sides' `drawLine` pixels
are then added. Nokia's `fillPolygon` uses the same rule with the
even-odd fill Nokia documents; `drawPolygon` and `drawTriangle` draw only
the sides. A coverage mask makes each pixel of a polygon or outline draw
once, so translucent colours do not stack where pieces meet. No limit
bounds the time one polygon or triangle fill takes: it walks every row
of the shape inside the clip, and every edge on each row.

**Dotted strokes.** MIDP leaves the pattern to the implementation and
lets end points and corners go undrawn. FastEmu draws every other pixel,
starting with the first: along a line from its canonical start, around a
rectangle clockwise from the top-left corner, along each quarter of an
ellipse from its left and right ends, and along each straight side of a
round rectangle from its start. Fills ignore the stroke style.

## Clip and translation

`setClip(x, y, w, h)` replaces the clip and `clipRect` intersects it with
the rectangle, both in translated coordinates, and the result is clamped
to the surface. A negative size empties the clip. `getClipX` and
`getClipY` report the clip in the current translated coordinates, so
they change when `translate` is called after `setClip`. The clip is
stored in surface pixels.

`reset()` restores the state MIDP gives a `Graphics` before each
`paint`: no translation, the whole surface as clip, black, solid strokes
and the default font.

## Colour and blending

- `setColor(rgb)` ignores the top byte; primitives and text draw opaque.
- `setColor(r, g, b)` and `setGrayScale(v)` throw
  `IllegalArgumentException` outside 0 to 255.
- `getGrayScale()` returns the value `setGrayScale` set, or else the
  Rec. 601 luma of the colour, rounded: `(299 r + 587 g + 114 b + 500) /
  1000`, so white is 255.
- `getDisplayColor(c)` keeps 24 bits by default. With
  `Graphics::set_display_colors` it quantizes like the phone's display
  (6-6-6, 5-6-5, 4-4-4, 3-3-2, or grey levels below 256 colours) and
  widens the result by repeating bits.
- Nokia's `setARGBColor` gives primitives an alpha; they then blend.
  `setColor` makes them opaque again.

Blending is Porter-Duff source-over on non-premultiplied colours in 8-bit
fixed point. Over an opaque destination each channel is
`(s a + d (255 - a)) / 255` rounded to nearest, computed for red and blue
in one 32-bit word with the exact `x / 255` identity
`(x + 128 + ((x + 128) >> 8)) >> 8`. The result stays opaque, so the
screen and MIDP's mutable images never gain transparency. Only Nokia's
`DirectUtils.createImage(w, h, argb)` images start translucent; over
those the result alpha is `a + da (255 - a) / 255` and the colour is the
weighted mean. An opaque source replaces the pixel and a fully
transparent one leaves it.

Fast paths: opaque fills use `slice::fill`; translucent fills multiply
the source out once per run; images whose pixels are all opaque copy
whole rows (`copy_from_slice`) when untransformed.

## Images

`Surface` constructors follow MIDP:

- `Surface::new_mutable(w, h)`: `Image.createImage(w, h)`, white and
  opaque; `w` or `h` of zero or less is `IllegalArgumentException`.
- `Surface::from_rgb(rgb, w, h, processAlpha)`: `createRGBImage`; a short
  array is `ArrayIndexOutOfBoundsException`; without `processAlpha` every
  pixel is opaque.
- `Surface::from_region(image, x, y, w, h, transform)`:
  `createImage(image, x, y, w, h, transform)`, immutable, alpha kept.
- `Surface::decode(bytes)` and `decode_range(bytes, offset, length)`:
  `createImage(byte[], ...)`.
- `Surface::new_mutable_filled(w, h, argb)` and `into_kind`: Nokia's
  `DirectUtils.createImage`.

Surfaces over 2^25 pixels (128 MiB) fail with `OutOfMemoryError`, as a
phone would, instead of exhausting the host.

The MIDP wrapper counts an image against the game's heap at 4 bytes a
pixel, and asks the heap before it makes the pixels. A call that makes an
image (`createImage(w, h)`, the copies, `createRGBImage`, the Nokia and
Siemens ones) throws `OutOfMemoryError` when the pixels are more than the
whole heap, or more than twice the heap together with what the heap
counts already. A file is checked by the size its header declares,
`probe_size`, before `Surface::decode` runs. A file of 130 KB that
declares 16384 by 2048 pixels never reaches the decoder on a small heap.
A file whose size cannot be read goes to the decoder, which refuses it.

`Image.createImage(InputStream)` reads at most 16 MiB. A stream that gives
more ends with `IOException`, and so does one whose `read` returns no
bytes 1000 times in a row.

`getRGB(rgbData, offset, scanlength, x, y, w, h)` stores pixel `(i, j)` at
`offset + i + j * scanlength`; a negative `scanlength` stores rows
bottom-up. Its checks run in this order: `|scanlength| < w`, then a
region outside the image or of negative size
(`IllegalArgumentException`), then any index outside the array
(`ArrayIndexOutOfBoundsException`, array unchanged). An empty region
copies nothing.

`drawImage(img, x, y, anchor)` allows `VCENTER` but not `BASELINE`.
`drawRegion` applies one of the eight sprite transforms and then the
anchor, using the transformed size:

| Value | Constant | Pixel `(x, y)` of a `w` by `h` region lands at |
| --- | --- | --- |
| 0 | `TRANS_NONE` | `(x, y)` |
| 1 | `TRANS_MIRROR_ROT180` | `(x, h - 1 - y)` |
| 2 | `TRANS_MIRROR` | `(w - 1 - x, y)` |
| 3 | `TRANS_ROT180` | `(w - 1 - x, h - 1 - y)` |
| 4 | `TRANS_MIRROR_ROT270` | `(y, x)` |
| 5 | `TRANS_ROT90` | `(h - 1 - y, x)` |
| 6 | `TRANS_ROT270` | `(y, w - 1 - x)` |
| 7 | `TRANS_MIRROR_ROT90` | `(h - 1 - y, w - 1 - x)` |

Rotations turn clockwise; mirrored forms mirror first. A region outside
the source or of negative size, a bad transform and a bad anchor are
`IllegalArgumentException`; a zero size draws nothing. MIDP also forbids
drawing a region of the destination itself, which the wrapper detects.

`drawRGB` draws nothing for a size of zero or less; otherwise any row
outside the array is `ArrayIndexOutOfBoundsException`, even if the clip
hides it. `copyArea` refuses the screen (`IllegalStateException`), takes
its source in translated coordinates, copies pixels as they are (no
blending) and handles overlap.

## Anchors

`AnchorPolicy::Strict`, the default, is MIDP's rule: zero means
`TOP | LEFT`; otherwise exactly one of `LEFT`, `HCENTER`, `RIGHT` and one
of `TOP`, `BOTTOM` plus `VCENTER` for images or `BASELINE` for text, and
no other bits. Anything else is `IllegalArgumentException`.
`AnchorPolicy::Lenient` never fails and reads the first bit set (right,
centre, left; bottom, centre, baseline, top), as J2ME Loader does, for
games whose phones accepted sloppy anchors. Centring rounds toward the
top left: `x - w / 2`.

## Text

Fonts are DejaVu LGC Sans (SYSTEM and PROPORTIONAL faces) and DejaVu LGC
Sans Mono (MONOSPACE), regular and bold, version 2.37, unmodified and
compiled in (1.7 MB; licences in `crates/fastemu-gfx/fonts/`). They
cover Latin, Greek and Cyrillic. fontdue rasterizes them.

- **Sizes.** `SIZE_SMALL`, `SIZE_MEDIUM` and `SIZE_LARGE` take their pixel
  heights from the device profile's `FontSizes`; that height is
  `getHeight()`. It is split into whole rows above and below the baseline
  in the font's proportions (`getBaselinePosition()` is the rows above),
  and the em size is the largest whose ascent and descent fit them, so
  glyphs stay inside the line.
- **Widths.** Each glyph advances a whole number of pixels, its advance
  rounded. `charWidth`, `charsWidth`, `stringWidth` and `substringWidth`
  add these, and drawing places each glyph at the sum of the advances
  before it, so measuring and drawing always agree. There is no kerning.
- **Missing characters** (and lone surrogates) draw as a hollow box
  standing on the baseline, `max(3, height / 2)` wide, the same width
  measuring reports. Control characters (U+0000 to U+001F and U+007F to
  U+009F, such as line breaks) draw nothing and take no width.
- **Styles.** Bold uses the bold files. Italic slants glyphs by one pixel
  per four rows above the baseline, without changing widths. Underlined
  text gets a one-pixel line across the measured width, one row below the
  baseline when the descent has room.
- **Anchors.** `LEFT`, `HCENTER` or `RIGHT` with `TOP`, `BASELINE` or
  `BOTTOM`; `VCENTER` is illegal for text.
- **Rendering.** Anti-aliased by default. `TextRendering::Aliased` draws a
  pixel where the glyph covers at least 96/255 of it: half coverage drops
  the thin stems of small unhinted glyphs.
- **Strings** are UTF-16 units (`&[u16]`), as Java holds them, so offsets
  and lengths mean the same. `drawSubstring` and `substringWidth` throw
  `StringIndexOutOfBoundsException` for a bad range, `drawChars` and
  `charsWidth` `ArrayIndexOutOfBoundsException`.
- **Cache.** Glyphs are cached per font file and pixel height for the
  whole process, shared by every running game; Latin-1 glyphs sit in a
  lock-free table. The glyphs above U+00FF of one face and size are
  limited to 4096. When that map is full the next new glyph clears it
  first, so a game that asks for every code unit cannot fill the
  process. Glyphs being drawn keep their own reference.

## Decoding

`Surface::decode` sniffs the format from the first bytes and makes an
immutable image; anything it cannot read is a decode error the wrapper
throws as `IllegalArgumentException` (or `IOException` for
`createImage(String)`).

- **PNG** (all colour types and depths, palettes, `tRNS`, 16-bit,
  Adam7): the file is rebuilt from IHDR, PLTE, tRNS and IDAT in the
  standard order with fresh CRCs and an IEND, so bad CRCs, garbage after
  IEND, a missing IEND and misplaced or malformed ancillary chunks do not
  matter. If the compressed data is cut short or damaged part way, it is
  inflated as far as it goes and padded with zeros, so the rows present
  still show (missing rows are black, or transparent with alpha).
  Gamma and colour profiles are ignored, as phones did.
  - Only the pairs PNG allows are repaired: grey with 1, 2, 4, 8 or 16
    bits, RGB with 8 or 16, indexed with 1, 2, 4 or 8, grey with alpha
    and RGBA with 8 or 16, the standard compression and filter methods
    and interlace method 0 or 1. Any other IHDR is a decode error, as
    the `png` crate says.
  - The repair builds the filtered rows in memory. It refuses rows over
    64 MiB (an 8-bit RGBA image of 16 million pixels) with
    `OutOfMemoryError`, and keeps two copies of them at most. A valid
    file is not repaired and not limited by this.
  - A PLTE chunk that is not 1 to 256 whole colours is left out, as the
    `png` crate panics on one that is not. An indexed picture then fails
    to decode; other pictures do not use the palette.
- **JPEG**, baseline and progressive, through zune-jpeg.
  A file with more than 16 extended XMP segments, which is 1 MB of
  metadata, is refused with a decode error: zune-jpeg joins the unfinished
  segments again after every marker, which took 4.8 s for a 1.6 MB file
  of thousands of them.
- **GIF**: the first frame, placed on the logical screen, transparent
  index clear.
- **BMP**: 1, 4, 8, 16, 24 and 32 bits, bit-field masks, bottom-up or
  top-down; run-length compressed files are refused. 32-bit files whose
  alpha mask is set but all zero are shown opaque.

Images wider or taller than 16 384 pixels, or over the pixel limit, fail
with `OutOfMemoryError` before their pixels are allocated. The wrapper
checks the declared size against the heap first, as the Images section
says, so a PNG that declares 2^25 pixels does not make `decode_stream`
reserve its 128 MiB buffer on a heap of a few MiB.

A panic inside a decoder (png, gif or zune-jpeg) is caught in
`Surface::decode`, and so in `decode_range`, and becomes a decode error
("the image decoder failed"). A panic on a game's thread would stop its
whole virtual machine. It needs `panic = "unwind"`, which every profile
of FastEmu sets. The launcher reads a suite's icon with the `png` crate
itself (`fastemu-ui`, `icon_bgra`), on a background task where a panic
would abort the app. It has the same guard and gives no icon for a file
that makes the decoder panic.

`probe_size(data) -> Option<(u32, u32)>` reads the size a file declares
from its header, without decoding anything, so a caller can check it
against the heap before `Surface::decode` allocates the pixels. It is
exported from the crate root.

The decoders find the header, so the probe cannot take a different one
from the decoder. A header the decoder skips, such as a frame header
inside an APP1 block or a block the `gif` crate skips, is no size here.

- PNG: the first IHDR chunk, found with the walk decoding uses.
- GIF: the logical screen, or the first frame's offset plus its size when
  the screen is empty, read by the `gif` crate up to the frame header
  with the options decoding uses.
- BMP: the width and height of the header, without the sign of the
  height, for the 12 byte and the 40 byte (or longer) headers.
- JPEG: the frame header zune-jpeg takes (SOF0, SOF1 or SOF2), read by
  its header pass, which stops at the start of the image data. It skips
  the other SOF markers by their length, as it does when it decodes.
- `None` when the format is unknown, the header is cut short or the
  picture is empty, and when a crate panics. Decoding such a file fails
  too. The size is only what the file claims. It can be far over the
  decoder's limits, which `Surface::decode` still enforces.

## Nokia DirectGraphics

Constants match Nokia's UI API 1.1 documentation and J2ME Loader's
`DirectGraphics.java`: `FLIP_HORIZONTAL` 0x2000, `FLIP_VERTICAL` 0x4000,
`ROTATE_90/180/270` 90/180/270, `TYPE_BYTE_1_GRAY` 1,
`TYPE_BYTE_1_GRAY_VERTICAL` -1, `TYPE_BYTE_2_GRAY` 2, `TYPE_BYTE_4_GRAY`
4, `TYPE_BYTE_8_GRAY` 8, `TYPE_BYTE_332_RGB` 332, `TYPE_USHORT_4444_ARGB`
4444, `TYPE_USHORT_444_RGB` 444, `TYPE_USHORT_555_RGB` 555,
`TYPE_USHORT_1555_ARGB` 1555, `TYPE_USHORT_565_RGB` 565,
`TYPE_INT_888_RGB` 888, `TYPE_INT_8888_ARGB` 8888.

- **Manipulations** rotate counter-clockwise, then flip vertically, then
  horizontally, as Nokia documents; they map onto sprite transforms
  through J2ME Loader's table. Bits beyond the two flips, or a rotation
  that is not 0, 90, 180 or 270, are `IllegalArgumentException`.
- **drawPixels** converts the block to ARGB, applies the manipulation and
  draws it with its top left at `(x, y)`, translated and clipped. Alpha
  counts for 8888, 4444 and 1555 only when `transparency` is true. For
  byte formats, `offset` and `scanlength` count pixels; pixels pack left
  to right from the high bits (vertical 1-bit: eight pixels of a column
  per byte, top pixel in the low bit, `scanlength` bytes per band of
  eight rows). A set 1-bit pixel is black; deeper grey levels run from
  black at 0 to white at their maximum. The transparency mask packs like
  the pixels, zero transparent; no mask means opaque. Negative sizes, a
  `scanlength` below `width`, a bad manipulation or format are
  `IllegalArgumentException`; a negative offset or a pixel outside the
  array is `ArrayIndexOutOfBoundsException`. A block of more than 4M
  pixels (1 << 22; a 360 by 640 screen is 0.23M) is refused with
  `IllegalArgumentException`, after those checks, because a byte array
  packs up to eight pixels to an element and the block is converted to
  `0xAARRGGBB` words before it is drawn.
- **getPixels** reads at translated coordinates, ignoring the clip, and
  writes only the bits of the pixels read; pixels off the surface leave
  their entries alone. 888 clears alpha. Pixels less than half opaque
  read as zero in byte formats, and the mask gets all ones for opaque
  pixels.
- **Polygons and triangles** take their own ARGB colour and leave the
  `Graphics` colour alone. `nPoints` of zero or less draws nothing; points
  outside the arrays are `ArrayIndexOutOfBoundsException`.
- `getNativePixelFormat()` is `NATIVE_PIXEL_FORMAT`, 5-6-5 like J2ME
  Loader; every format draws, so the wrapper may report another.

## Errors

Every failing call returns `GfxError`, whose `java_class()` names the
exception to throw:

| Variant | Java exception |
| --- | --- |
| `IllegalArgument`, `Decode` | `java/lang/IllegalArgumentException` |
| `ArrayIndexOutOfBounds` | `java/lang/ArrayIndexOutOfBoundsException` |
| `StringIndexOutOfBounds` | `java/lang/StringIndexOutOfBoundsException` |
| `NullPointer` | `java/lang/NullPointerException` |
| `IllegalState` | `java/lang/IllegalStateException` |
| `OutOfMemory` | `java/lang/OutOfMemoryError` |

Hostile input reaches these errors, not a crash: a decoder panic is a
`Decode`, a repair over its cap and an image over the pixel limit are
`OutOfMemory`, and a `drawPixels` block over 4M pixels is
`IllegalArgument`.

Java references reach the library already dereferenced, so the wrapper
throws `NullPointerException` for a null argument before calling, as MIDP
checks nulls first; `non_null(option)` turns a missing reference into
`GfxError::NullPointer` for wrappers that prefer one error path.

## Using it from the MIDP wrapper

- Each instance makes one `FontLibrary` from its profile's `font_sizes`
  and keeps it for `Font.getFont` and `Font.getDefaultFont`.
- The screen is a `Surface::screen(width, height)`. A Canvas's `Graphics`
  is `Graphics::new(&screen, library.default_font())`, `reset()` before
  each `paint`. After `paint` (or `flushGraphics`), `screen.to_frame(n)`
  is the frame for the app.
- `Image.getGraphics()` is `Graphics::new(&image, default_font)`; it
  fails with `IllegalStateException` for immutable images.
- Set `set_display_colors(profile.colors)` so `getDisplayColor` matches
  the profile, and choose `AnchorPolicy::Lenient` for games that need it.
- Java `int[]`, `short[]` and `byte[]` pass as `&[i32]`, `&[i16]` and
  `&[i8]`; `char[]` and `String` contents as `&[u16]`.
- `drawImage` of an image onto itself is undefined in MIDP and cannot be
  expressed with Rust borrows; draw a copy. `drawRegion` of the
  destination itself must throw `IllegalArgumentException`.
- `DirectUtils.getDirectGraphics(g)` wraps the same `Graphics`; its calls
  are the `Graphics` methods in the `nokia` module.
