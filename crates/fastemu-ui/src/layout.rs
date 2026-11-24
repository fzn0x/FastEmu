//! The arithmetic behind the workspace: how large each game's screen is
//! drawn, where its keypad goes, and how tiles share the window.
//!
//! Sizes are logical pixels (GPUI's `Pixels`) unless a name says device
//! pixels. Screens scale by whole numbers of device pixels so every phone
//! pixel becomes a crisp square.

/// A width and a height in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Extent {
    pub width: f32,
    pub height: f32,
}

impl Extent {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    fn shrink(self, width: f32, height: f32) -> Self {
        Self::new(self.width - width, self.height - height)
    }

    fn area(self) -> f32 {
        self.width.max(0.0) * self.height.max(0.0)
    }
}

/// The height of a tile's title bar.
pub const TITLE_BAR_HEIGHT: f32 = 34.0;
/// Space between a tile's border and its contents.
pub const TILE_PADDING: f32 = 8.0;
/// The narrowest a tile gets, so its title bar buttons fit.
pub const MIN_TILE_WIDTH: f32 = 236.0;
/// Space between the screen and the keypad, and between tiles.
pub const GAP: f32 = 10.0;
/// The largest side of a screen texture, to keep atlas uploads bounded.
pub const MAX_TEXTURE_SIDE: u32 = 4096;

/// One keypad button.
pub const KEY_WIDTH: f32 = 40.0;
pub const KEY_HEIGHT: f32 = 30.0;
pub const KEY_GAP: f32 = 4.0;
/// Space between the D-pad cluster and the number pad.
pub const CLUSTER_GAP: f32 = 12.0;

/// The D-pad cluster: three rows of three keys.
pub const NAV_CLUSTER: Extent = Extent::new(
    3.0 * KEY_WIDTH + 2.0 * KEY_GAP,
    3.0 * KEY_HEIGHT + 2.0 * KEY_GAP,
);
/// The number pad: four rows of three keys.
pub const NUM_CLUSTER: Extent = Extent::new(
    3.0 * KEY_WIDTH + 2.0 * KEY_GAP,
    4.0 * KEY_HEIGHT + 3.0 * KEY_GAP,
);

/// How a screen is drawn in the space it has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenFit {
    /// Each phone pixel becomes this many texture pixels a side, copied
    /// with nearest-neighbour sampling before upload. Always at least 1.
    pub texture_factor: u32,
    /// The size to draw the screen at.
    pub size: Extent,
}

/// Fits a `screen` of phone pixels into `space`, on a display with
/// `scale_factor` device pixels per logical pixel.
///
/// The screen gets the largest whole number of device pixels per phone
/// pixel that fits. When not even one fits, it shrinks smoothly instead:
/// small thumbnails favour seeing the whole picture over sharpness.
pub fn fit_screen((width, height): (u32, u32), space: Extent, scale_factor: f32) -> ScreenFit {
    let (w, h) = (width.max(1) as f32, height.max(1) as f32);
    let scale_factor = if scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };
    let device = (space.width * scale_factor, space.height * scale_factor);
    let whole = (device.0 / w).min(device.1 / h).floor();
    let cap = (MAX_TEXTURE_SIDE / width.max(height).max(1)).max(1);
    if whole >= 1.0 {
        let factor = (whole as u32).min(cap);
        let f = factor as f32 / scale_factor;
        ScreenFit {
            texture_factor: factor,
            size: Extent::new(w * f, h * f),
        }
    } else {
        let s = (space.width / w).min(space.height / h).max(0.0);
        ScreenFit {
            texture_factor: 1,
            size: Extent::new(w * s, h * s),
        }
    }
}

/// Where a tile's on-screen keypad sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeypadPlacement {
    /// Under the screen, the two clusters side by side.
    Below,
    /// Right of the screen, the clusters stacked.
    Beside,
}

impl KeypadPlacement {
    /// The room the keypad takes in this placement.
    pub fn extent(self) -> Extent {
        match self {
            Self::Below => Extent::new(
                NAV_CLUSTER.width + CLUSTER_GAP + NUM_CLUSTER.width,
                NUM_CLUSTER.height.max(NAV_CLUSTER.height),
            ),
            Self::Beside => Extent::new(
                NAV_CLUSTER.width.max(NUM_CLUSTER.width),
                NAV_CLUSTER.height + CLUSTER_GAP + NUM_CLUSTER.height,
            ),
        }
    }
}

/// The parts of one tile and their sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileLayout {
    pub screen: ScreenFit,
    pub keypad: Option<KeypadPlacement>,
    /// The tile's outer size, which may be smaller than its cell: tiles
    /// hug their contents and sit centred in the cell.
    pub frame: Extent,
}

/// Lays out a tile that has `cell` to itself.
pub fn tile_layout(
    cell: Extent,
    screen: (u32, u32),
    keypad: bool,
    scale_factor: f32,
) -> TileLayout {
    let inner = cell.shrink(2.0 * TILE_PADDING, 2.0 * TILE_PADDING + TITLE_BAR_HEIGHT);
    let mut best = (fit_screen(screen, inner, scale_factor), None);
    if keypad {
        let mut candidates = Vec::with_capacity(2);
        for placement in [KeypadPlacement::Below, KeypadPlacement::Beside] {
            let pad = placement.extent();
            let space = match placement {
                KeypadPlacement::Below => inner.shrink(0.0, pad.height + GAP),
                KeypadPlacement::Beside => inner.shrink(pad.width + GAP, 0.0),
            };
            let fits_pad = match placement {
                KeypadPlacement::Below => inner.width >= pad.width,
                KeypadPlacement::Beside => inner.height >= pad.height,
            };
            if fits_pad && space.width > 0.0 && space.height > 0.0 {
                candidates.push((fit_screen(screen, space, scale_factor), Some(placement)));
            }
        }
        if let Some(choice) = candidates.into_iter().reduce(|a, b| {
            if b.0.size.area() > a.0.size.area() {
                b
            } else {
                a
            }
        }) {
            best = choice;
        }
    }
    let (fit, placement) = best;
    let content = match placement {
        None => fit.size,
        Some(p @ KeypadPlacement::Below) => Extent::new(
            fit.size.width.max(p.extent().width),
            fit.size.height + GAP + p.extent().height,
        ),
        Some(p @ KeypadPlacement::Beside) => Extent::new(
            fit.size.width + GAP + p.extent().width,
            fit.size.height.max(p.extent().height),
        ),
    };
    let frame = Extent::new(
        (content.width + 2.0 * TILE_PADDING)
            .max(MIN_TILE_WIDTH)
            .min(cell.width.max(0.0)),
        (content.height + 2.0 * TILE_PADDING + TITLE_BAR_HEIGHT).min(cell.height.max(0.0)),
    );
    TileLayout {
        screen: fit,
        keypad: placement,
        frame,
    }
}

/// An arrangement of equal cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub columns: usize,
    pub rows: usize,
    pub cell: Extent,
}

/// Picks the grid for `count` tiles in `area` that draws the screens
/// largest. `screen` is the largest screen among the tiles. Ties go to the
/// grid with fewer columns.
pub fn grid(
    count: usize,
    area: Extent,
    screen: (u32, u32),
    keypad: bool,
    scale_factor: f32,
) -> Grid {
    let count = count.max(1);
    let mut best: Option<(f32, Grid)> = None;
    for columns in 1..=count {
        let rows = count.div_ceil(columns);
        let cell = Extent::new(
            (area.width - GAP * (columns - 1) as f32) / columns as f32,
            (area.height - GAP * (rows - 1) as f32) / rows as f32,
        );
        if cell.width <= 0.0 || cell.height <= 0.0 {
            continue;
        }
        let score = tile_layout(cell, screen, keypad, scale_factor)
            .screen
            .size
            .area();
        if best.is_none_or(|(s, _)| score > s) {
            best = Some((
                score,
                Grid {
                    columns,
                    rows,
                    cell,
                },
            ));
        }
    }
    best.map_or(
        Grid {
            columns: 1,
            rows: count,
            cell: Extent::new(area.width.max(0.0), area.height.max(0.0) / count as f32),
        },
        |(_, grid)| grid,
    )
}

/// Which edge the thumbnails of the focus layout line up along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StripSide {
    Right,
    Bottom,
}

/// The focus layout: one large tile and a strip of thumbnails.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocusLayout {
    pub main: Extent,
    /// The strip, when there is more than one game.
    pub strip: Option<(StripSide, Extent)>,
    /// The size of each thumbnail in the strip.
    pub thumb: Extent,
}

/// The height of a thumbnail's title row.
pub const THUMB_TITLE_HEIGHT: f32 = 24.0;

/// Splits `area` between the focused tile and `count - 1` thumbnails. The
/// strip goes along the longer side; it scrolls when the thumbnails do not
/// fit.
pub fn focus_layout(count: usize, area: Extent) -> FocusLayout {
    if count <= 1 {
        return FocusLayout {
            main: area,
            strip: None,
            thumb: Extent::default(),
        };
    }
    if area.width >= area.height {
        let width = (area.width * 0.2).clamp(140.0, 240.0);
        let thumb = Extent::new(width, width * 4.0 / 3.0 + THUMB_TITLE_HEIGHT);
        FocusLayout {
            main: Extent::new(area.width - width - GAP, area.height),
            strip: Some((StripSide::Right, Extent::new(width, area.height))),
            thumb,
        }
    } else {
        let height = (area.height * 0.22).clamp(140.0, 260.0);
        let thumb = Extent::new((height - THUMB_TITLE_HEIGHT) * 0.75 + 16.0, height);
        FocusLayout {
            main: Extent::new(area.width, area.height - height - GAP),
            strip: Some((StripSide::Bottom, Extent::new(area.width, height))),
            thumb,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn screens_scale_by_whole_device_pixels() {
        let fit = fit_screen((240, 320), Extent::new(800.0, 1000.0), 1.0);
        assert_eq!(fit.texture_factor, 3);
        assert_eq!(fit.size, Extent::new(720.0, 960.0));
    }

    #[test]
    fn high_dpi_displays_count_device_pixels() {
        // 500 logical pixels at 1.5x are 750 device pixels: three phone
        // pixels wide, drawn at 480 logical pixels.
        let fit = fit_screen((240, 320), Extent::new(500.0, 1000.0), 1.5);
        assert_eq!(fit.texture_factor, 3);
        assert!(close(fit.size.width, 480.0) && close(fit.size.height, 640.0));
    }

    #[test]
    fn too_little_room_shrinks_smoothly() {
        let fit = fit_screen((240, 320), Extent::new(120.0, 400.0), 1.0);
        assert_eq!(fit.texture_factor, 1);
        assert!(close(fit.size.width, 120.0) && close(fit.size.height, 160.0));
        let none = fit_screen((240, 320), Extent::new(-5.0, 10.0), 1.0);
        assert_eq!(none.size, Extent::new(0.0, 0.0));
    }

    #[test]
    fn texture_size_is_capped() {
        let fit = fit_screen((128, 160), Extent::new(100_000.0, 100_000.0), 1.0);
        assert_eq!(fit.texture_factor, MAX_TEXTURE_SIDE / 160);
    }

    #[test]
    fn the_keypad_goes_where_the_screen_stays_largest() {
        let tall = tile_layout(Extent::new(500.0, 900.0), (240, 320), true, 1.0);
        assert_eq!(tall.keypad, Some(KeypadPlacement::Below));
        let wide = tile_layout(Extent::new(900.0, 500.0), (240, 320), true, 1.0);
        assert_eq!(wide.keypad, Some(KeypadPlacement::Beside));
        let none = tile_layout(Extent::new(900.0, 500.0), (240, 320), false, 1.0);
        assert_eq!(none.keypad, None);
        assert!(none.screen.size.width >= wide.screen.size.width);
    }

    #[test]
    fn tiles_hug_their_contents() {
        let layout = tile_layout(Extent::new(2000.0, 2000.0), (128, 160), false, 1.0);
        let factor = layout.screen.texture_factor as f32;
        assert!(close(
            layout.frame.height,
            160.0 * factor + 2.0 * TILE_PADDING + TITLE_BAR_HEIGHT
        ));
        let tiny = tile_layout(Extent::new(100.0, 100.0), (128, 160), false, 1.0);
        assert!(tiny.frame.width <= 100.0 && tiny.frame.height <= 100.0);
    }

    #[test]
    fn grids_follow_the_window_shape() {
        let wide = grid(2, Extent::new(1600.0, 700.0), (240, 320), false, 1.0);
        assert_eq!((wide.columns, wide.rows), (2, 1));
        let tall = grid(2, Extent::new(700.0, 1600.0), (240, 320), false, 1.0);
        assert_eq!((tall.columns, tall.rows), (1, 2));
        let four = grid(4, Extent::new(1200.0, 1000.0), (240, 320), false, 1.0);
        assert_eq!((four.columns, four.rows), (2, 2));
        let one = grid(1, Extent::new(800.0, 600.0), (240, 320), true, 1.0);
        assert_eq!((one.columns, one.rows), (1, 1));
        assert_eq!(one.cell, Extent::new(800.0, 600.0));
    }

    #[test]
    fn many_games_still_get_a_grid() {
        let g = grid(9, Extent::new(1500.0, 1000.0), (240, 320), false, 1.0);
        assert!(g.columns * g.rows >= 9);
        assert!(g.columns >= 3);
        let cramped = grid(3, Extent::new(10.0, 10.0), (240, 320), true, 1.0);
        assert!(cramped.columns * cramped.rows >= 3);
    }

    #[test]
    fn focus_puts_thumbnails_along_the_longer_side() {
        let solo = focus_layout(1, Extent::new(1000.0, 800.0));
        assert_eq!(solo.strip, None);
        assert_eq!(solo.main, Extent::new(1000.0, 800.0));
        let wide = focus_layout(3, Extent::new(1200.0, 800.0));
        let (side, strip) = wide.strip.expect("a strip");
        assert_eq!(side, StripSide::Right);
        assert!(close(wide.main.width + strip.width + GAP, 1200.0));
        let tall = focus_layout(3, Extent::new(600.0, 1000.0));
        assert_eq!(tall.strip.map(|s| s.0), Some(StripSide::Bottom));
        assert!(tall.thumb.height <= tall.strip.expect("a strip").1.height);
    }
}
