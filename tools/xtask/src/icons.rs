//! The app's icons, rendered from `assets/icon.svg`.
//!
//! `cargo xtask icons` writes every icon the release builds and the Android
//! app use, so that a change to the SVG reaches them all at once:
//!
//! * `packaging/icons/fastemu-<size>.png`, 16 to 1024 pixels, for Linux's
//!   icon theme and anything else that wants a bitmap;
//! * `packaging/icons/fastemu.ico`, which `crates/fastemu/build.rs` embeds
//!   in `fastemu.exe`;
//! * `packaging/icons/fastemu.icns`, the macOS app's icon, drawn inside
//!   Apple's icon grid so it sits in the Dock as other apps do;
//! * the Android launcher icons in `android/app/src/main/res`: square and
//!   round bitmaps for each density, and the foreground, monochrome and
//!   background of the adaptive icon.
//!
//! The SVG's first shape that covers its whole canvas is the background;
//! the rest is the glyph. The Android icons place the glyph on their own
//! backgrounds, which the launcher masks to its shape.

use std::fs;
use std::path::Path;

use resvg::tiny_skia::{Pixmap, PremultipliedColorU8, Transform};
use resvg::usvg::{self, Node, Paint};

/// The sizes of `packaging/icons/fastemu-<size>.png`.
const PNG_SIZES: [u32; 9] = [16, 24, 32, 48, 64, 128, 256, 512, 1024];

/// The sizes `fastemu.ico` holds: what Windows shows from a list view to a
/// large tile.
const ICO_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

/// The entries of `fastemu.icns`: each PNG's type and size in pixels. The
/// `@2x` types (ic11 to ic14, ic10) are the Retina versions of the others.
const ICNS_TYPES: [(&[u8; 4], u32); 11] = [
    (b"icp4", 16),
    (b"icp5", 32),
    (b"icp6", 64),
    (b"ic07", 128),
    (b"ic08", 256),
    (b"ic09", 512),
    (b"ic10", 1024),
    (b"ic11", 32),
    (b"ic12", 64),
    (b"ic13", 256),
    (b"ic14", 512),
];

/// Apple's icon grid: on a 1024-pixel canvas the icon's body is 824 pixels
/// wide, centred.
const MACOS_BODY: f32 = 824.0 / 1024.0;

/// Android's densities and the size of a 48dp launcher icon in each.
const ANDROID_DENSITIES: [(&str, u32); 5] = [
    ("mdpi", 48),
    ("hdpi", 72),
    ("xhdpi", 96),
    ("xxhdpi", 144),
    ("xxxhdpi", 192),
];

/// An adaptive icon's layers are 108dp square; the launcher shows the middle
/// 72dp, and its mask always keeps a 66dp circle. The SVG's canvas maps onto
/// that 66dp, so no mask cuts into the glyph.
const ADAPTIVE_LAYER_DP: f32 = 108.0;
const ADAPTIVE_SAFE_DP: f32 = 66.0;
const ADAPTIVE_VISIBLE_DP: f32 = 72.0;

/// Writes every icon for the repository at `root`.
pub fn run(root: &Path) -> Result<(), String> {
    let svg_path = root.join("assets/icon.svg");
    let svg = fs::read(&svg_path).map_err(|e| format!("{}: {e}", svg_path.display()))?;
    let icon = Icon::parse(&svg)?;
    let mut written = 0;

    let icons = root.join("packaging/icons");
    for size in PNG_SIZES {
        write(
            &icons.join(format!("fastemu-{size}.png")),
            &png(&icon.full(size)?)?,
        )?;
        written += 1;
    }
    let ico_images = ICO_SIZES
        .iter()
        .map(|&size| icon.full(size))
        .collect::<Result<Vec<_>, _>>()?;
    write(&icons.join("fastemu.ico"), &ico(&ico_images)?)?;
    let icns_images = ICNS_TYPES
        .iter()
        .map(|&(kind, size)| Ok((*kind, png(&icon.macos(size)?)?)))
        .collect::<Result<Vec<_>, String>>()?;
    write(&icons.join("fastemu.icns"), &icns(&icns_images))?;
    written += 2;

    let res = root.join("android/app/src/main/res");
    for (density, size) in ANDROID_DENSITIES {
        let dir = res.join(format!("mipmap-{density}"));
        let layer = (size as f32 * ADAPTIVE_LAYER_DP / 48.0).round() as u32;
        let foreground = icon.adaptive_foreground(layer)?;
        write(&dir.join("ic_launcher.png"), &png(&icon.full(size)?)?)?;
        write(
            &dir.join("ic_launcher_round.png"),
            &png(&icon.round(size)?)?,
        )?;
        write(&dir.join("ic_launcher_foreground.png"), &png(&foreground)?)?;
        write(
            &dir.join("ic_launcher_monochrome.png"),
            &png(&monochrome(&foreground, icon.background))?,
        )?;
        written += 4;
    }
    let [r, g, b] = icon.background;
    let colour = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <!-- Written by `cargo xtask icons` from assets/icon.svg. -->\n\
         <resources>\n    \
         <color name=\"ic_launcher_background\">#{r:02X}{g:02X}{b:02X}</color>\n\
         </resources>\n"
    );
    write(
        &res.join("values/ic_launcher_background.xml"),
        colour.as_bytes(),
    )?;
    written += 1;

    println!("wrote {written} icons from assets/icon.svg");
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// The parsed SVG and what the Android icons take from it.
struct Icon {
    tree: usvg::Tree,
    /// The width and height of the SVG's canvas.
    canvas: f32,
    /// The background shape's colour.
    background: [u8; 3],
    /// The index, in the root's children, of the background shape.
    background_index: usize,
}

impl Icon {
    fn parse(svg: &[u8]) -> Result<Self, String> {
        let tree = usvg::Tree::from_data(svg, &usvg::Options::default())
            .map_err(|e| format!("assets/icon.svg: {e}"))?;
        let size = tree.size();
        if (size.width() - size.height()).abs() > f32::EPSILON {
            return Err("assets/icon.svg: an icon's canvas is square".to_string());
        }
        let canvas = size.width();
        let (background_index, background) = tree
            .root()
            .children()
            .iter()
            .enumerate()
            .find_map(|(i, node)| {
                let Node::Path(path) = node else { return None };
                let bounds = path.abs_bounding_box();
                let covers = bounds.left() <= 0.0
                    && bounds.top() <= 0.0
                    && bounds.right() >= canvas
                    && bounds.bottom() >= canvas;
                match path.fill().map(usvg::Fill::paint) {
                    Some(Paint::Color(c)) if covers => Some((i, [c.red, c.green, c.blue])),
                    _ => None,
                }
            })
            .ok_or(
                "assets/icon.svg: its first full-canvas shape must be a plain-coloured background",
            )?;
        Ok(Self {
            tree,
            canvas,
            background,
            background_index,
        })
    }

    /// The whole icon, `size` pixels square.
    fn full(&self, size: u32) -> Result<Pixmap, String> {
        let mut pixmap = new_pixmap(size)?;
        let scale = size as f32 / self.canvas;
        resvg::render(
            &self.tree,
            Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        Ok(pixmap)
    }

    /// The whole icon inside Apple's icon grid, `size` pixels square.
    fn macos(&self, size: u32) -> Result<Pixmap, String> {
        let mut pixmap = new_pixmap(size)?;
        let body = size as f32 * MACOS_BODY;
        let margin = (size as f32 - body) / 2.0;
        let scale = body / self.canvas;
        resvg::render(
            &self.tree,
            Transform::from_row(scale, 0.0, 0.0, scale, margin, margin),
            &mut pixmap.as_mut(),
        );
        Ok(pixmap)
    }

    /// The glyph alone, its canvas scaled to `body` pixels and centred on a
    /// transparent `size`-pixel square.
    fn glyph(&self, size: u32, body: f32) -> Result<Pixmap, String> {
        let mut pixmap = new_pixmap(size)?;
        let scale = body / self.canvas;
        let margin = (size as f32 - body) / 2.0;
        let transform = Transform::from_row(scale, 0.0, 0.0, scale, margin, margin);
        for (i, node) in self.tree.root().children().iter().enumerate() {
            if i == self.background_index {
                continue;
            }
            // render_node draws a node relative to its own bounding box;
            // undoing that shift keeps it where the SVG puts it.
            if let Some(bounds) = node.abs_layer_bounding_box() {
                resvg::render_node(
                    node,
                    transform.pre_translate(bounds.x(), bounds.y()),
                    &mut pixmap.as_mut(),
                );
            }
        }
        Ok(pixmap)
    }

    /// The adaptive icon's foreground layer, `size` pixels square: the glyph
    /// inside the 66dp safe zone of the 108dp layer.
    fn adaptive_foreground(&self, size: u32) -> Result<Pixmap, String> {
        self.glyph(size, size as f32 * ADAPTIVE_SAFE_DP / ADAPTIVE_LAYER_DP)
    }

    /// A round launcher icon for launchers that ask for one: the background
    /// colour as a circle and the glyph at the size the adaptive icon shows.
    fn round(&self, size: u32) -> Result<Pixmap, String> {
        let mut pixmap = new_pixmap(size)?;
        let [r, g, b] = self.background;
        let radius = size as f32 / 2.0;
        let circle = resvg::tiny_skia::PathBuilder::from_circle(radius, radius, radius)
            .ok_or("an icon needs a size")?;
        let mut paint = resvg::tiny_skia::Paint::default();
        paint.set_color_rgba8(r, g, b, 255);
        paint.anti_alias = true;
        pixmap.fill_path(
            &circle,
            &paint,
            resvg::tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
        let glyph = self.glyph(size, size as f32 * ADAPTIVE_SAFE_DP / ADAPTIVE_VISIBLE_DP)?;
        pixmap.draw_pixmap(
            0,
            0,
            glyph.as_ref(),
            &resvg::tiny_skia::PixmapPaint::default(),
            Transform::identity(),
            None,
        );
        Ok(pixmap)
    }
}

fn new_pixmap(size: u32) -> Result<Pixmap, String> {
    Pixmap::new(size, size).ok_or_else(|| format!("cannot make a {size}-pixel icon"))
}

fn png(pixmap: &Pixmap) -> Result<Vec<u8>, String> {
    pixmap.encode_png().map_err(|e| e.to_string())
}

/// The monochrome layer of an adaptive icon, from its foreground: Android
/// tints it and keeps only its alpha. Parts of the glyph in the background's
/// colour, such as the phone's screen, show the background through it in
/// colour, so here they become holes.
fn monochrome(foreground: &Pixmap, background: [u8; 3]) -> Pixmap {
    let distance = |c: [u8; 3]| -> f32 {
        c.iter()
            .zip(background)
            .map(|(&a, b)| (f32::from(a) - f32::from(b)).powi(2))
            .sum::<f32>()
            .sqrt()
    };
    let colour = |p: &PremultipliedColorU8| {
        let c = p.demultiply();
        [c.red(), c.green(), c.blue()]
    };
    // The glyph's colour farthest from the background counts as solid.
    let far = foreground
        .pixels()
        .iter()
        .filter(|p| p.alpha() == 255)
        .map(|p| distance(colour(p)))
        .fold(0.0_f32, f32::max)
        .max(1.0);
    let mut out = foreground.clone();
    for pixel in out.pixels_mut() {
        let solid = (distance(colour(pixel)) / far).min(1.0);
        let alpha = (f32::from(pixel.alpha()) * solid).round() as u8;
        *pixel = PremultipliedColorU8::from_rgba(alpha, alpha, alpha, alpha)
            .unwrap_or(PremultipliedColorU8::TRANSPARENT);
    }
    out
}

/// An ICO file of `images`, each square and at most 256 pixels: the 256-pixel
/// one as PNG, the smaller ones as 32-bit bitmaps, which every Windows tool
/// reads.
fn ico(images: &[Pixmap]) -> Result<Vec<u8>, String> {
    let mut entries = Vec::new();
    let mut data = Vec::new();
    let count = u16::try_from(images.len()).map_err(|_| "too many icons")?;
    let mut offset = 6 + 16 * u32::from(count);
    for image in images {
        let size = image.width();
        if size != image.height() || size > 256 {
            return Err(format!(
                "an ICO image is square and at most 256 pixels, not {size}"
            ));
        }
        let bytes = if size == 256 { png(image)? } else { dib(image) };
        let length = u32::try_from(bytes.len()).map_err(|_| "an icon too large")?;
        // 256 is written as 0.
        let side = (size % 256) as u8;
        entries.extend_from_slice(&[side, side, 0, 0]);
        entries.extend_from_slice(&1u16.to_le_bytes()); // planes
        entries.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        entries.extend_from_slice(&length.to_le_bytes());
        entries.extend_from_slice(&offset.to_le_bytes());
        offset += length;
        data.extend(bytes);
    }
    let mut out = Vec::with_capacity(6 + entries.len() + data.len());
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // an icon, not a cursor
    out.extend_from_slice(&count.to_le_bytes());
    out.extend(entries);
    out.extend(data);
    Ok(out)
}

/// An icon's bitmap as an ICO stores it: a BITMAPINFOHEADER of twice the
/// height, the BGRA rows bottom up, then the 1-bit mask of transparent
/// pixels, each row padded to four bytes.
fn dib(image: &Pixmap) -> Vec<u8> {
    let (width, height) = (image.width(), image.height());
    let mask_row = width.div_ceil(32) as usize * 4;
    let pixels = image.pixels();
    let mut out = Vec::new();
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&(height * 2).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // no compression
    let image_size = width * height * 4 + mask_row as u32 * height;
    out.extend_from_slice(&image_size.to_le_bytes());
    out.extend_from_slice(&[0; 16]); // resolution and palette: unused
    for y in (0..height).rev() {
        for x in 0..width {
            let c = pixels[(y * width + x) as usize].demultiply();
            out.extend_from_slice(&[c.blue(), c.green(), c.red(), c.alpha()]);
        }
    }
    for y in (0..height).rev() {
        let mut row = vec![0u8; mask_row];
        for x in 0..width {
            if pixels[(y * width + x) as usize].alpha() == 0 {
                row[x as usize / 8] |= 0x80 >> (x % 8);
            }
        }
        out.extend(row);
    }
    out
}

/// An ICNS file of PNG `entries`, each with its type.
fn icns(entries: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let total: usize = 8 + entries.iter().map(|(_, png)| 8 + png.len()).sum::<usize>();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&(total as u32).to_be_bytes());
    for (kind, png) in entries {
        out.extend_from_slice(kind);
        out.extend_from_slice(&((8 + png.len()) as u32).to_be_bytes());
        out.extend_from_slice(png);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="32" height="32">
  <rect width="32" height="32" rx="8" fill="#5eead4"/>
  <rect x="8" y="8" width="16" height="16" fill="#0a0a0a"/>
  <rect x="12" y="12" width="8" height="8" fill="#5eead4"/>
</svg>"##;

    fn at(pixmap: &Pixmap, x: u32, y: u32) -> PremultipliedColorU8 {
        pixmap.pixels()[(y * pixmap.width() + x) as usize]
    }

    #[test]
    fn the_background_is_the_first_full_canvas_shape() {
        let icon = Icon::parse(SVG.as_bytes()).unwrap();
        assert_eq!(icon.background, [0x5e, 0xea, 0xd4]);
        assert_eq!(icon.background_index, 0);
        assert!(
            Icon::parse(br#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"/>"#)
                .is_err()
        );
    }

    #[test]
    fn the_full_icon_keeps_its_rounded_corners() {
        let icon = Icon::parse(SVG.as_bytes()).unwrap();
        let full = icon.full(64).unwrap();
        assert_eq!(at(&full, 0, 0).alpha(), 0);
        assert_eq!(at(&full, 32, 4).demultiply().green(), 0xea);
        assert_eq!(at(&full, 20, 20).demultiply().red(), 0x0a);
    }

    #[test]
    fn the_glyph_leaves_out_the_background() {
        let icon = Icon::parse(SVG.as_bytes()).unwrap();
        let glyph = icon.glyph(64, 64.0).unwrap();
        assert_eq!(at(&glyph, 4, 32).alpha(), 0);
        assert_eq!(at(&glyph, 20, 20).alpha(), 255);
        assert_eq!(at(&glyph, 20, 20).red(), 0x0a);
        // Scaled to half and centred, the dark square starts at 24.
        let small = icon.glyph(64, 32.0).unwrap();
        assert_eq!(at(&small, 23, 32).alpha(), 0);
        assert_eq!(at(&small, 25, 25).alpha(), 255);
    }

    #[test]
    fn monochrome_cuts_out_what_has_the_background_colour() {
        let icon = Icon::parse(SVG.as_bytes()).unwrap();
        let mono = monochrome(&icon.glyph(64, 64.0).unwrap(), icon.background);
        assert_eq!(at(&mono, 20, 20).alpha(), 255);
        assert_eq!(at(&mono, 32, 32).alpha(), 0);
        assert_eq!(at(&mono, 4, 4).alpha(), 0);
    }

    #[test]
    fn round_icons_are_circles() {
        let icon = Icon::parse(SVG.as_bytes()).unwrap();
        let round = icon.round(48).unwrap();
        assert_eq!(at(&round, 1, 1).alpha(), 0);
        assert_eq!(at(&round, 24, 2).alpha(), 255);
    }

    #[test]
    fn ico_has_a_directory_entry_per_image() {
        let images: Vec<Pixmap> = [16, 256].iter().map(|&s| new_pixmap(s).unwrap()).collect();
        let file = ico(&images).unwrap();
        assert_eq!(&file[..6], &[0, 0, 1, 0, 2, 0]);
        // 16 pixels: a bitmap header of height 32.
        let offset = u32::from_le_bytes(file[18..22].try_into().unwrap()) as usize;
        assert_eq!(
            u32::from_le_bytes(file[offset + 8..offset + 12].try_into().unwrap()),
            32
        );
        let length = u32::from_le_bytes(file[14..18].try_into().unwrap()) as usize;
        assert_eq!(length, 40 + 16 * 16 * 4 + 4 * 16);
        // 256 pixels: written as 0, stored as PNG.
        assert_eq!(file[22], 0);
        let offset = u32::from_le_bytes(file[34..38].try_into().unwrap()) as usize;
        assert_eq!(&file[offset..offset + 4], b"\x89PNG");
        assert!(ico(&[new_pixmap(512).unwrap()]).is_err());
    }

    #[test]
    fn transparent_pixels_are_masked_in_bitmaps() {
        let image = new_pixmap(16).unwrap();
        let bytes = dib(&image);
        assert!(
            bytes[40 + 16 * 16 * 4..]
                .chunks(4)
                .all(|row| row == [0xff, 0xff, 0, 0])
        );
    }

    #[test]
    fn icns_lengths_add_up() {
        let file = icns(&[(*b"ic07", vec![1, 2, 3]), (*b"ic08", vec![4])]);
        assert_eq!(&file[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(file[4..8].try_into().unwrap()) as usize,
            file.len()
        );
        assert_eq!(&file[8..12], b"ic07");
        assert_eq!(u32::from_be_bytes(file[12..16].try_into().unwrap()), 11);
        assert_eq!(&file[19..23], b"ic08");
    }
}
