//! A tiled layer: a grid of cells, each empty (0), a static tile from the
//! image (1 and up) or an animated tile (-1 and down) that stands for a
//! static tile the game can change.

use std::ops::Range;

use fastemu_gfx::anchor::{LEFT, TOP};
use fastemu_gfx::{GfxError, Graphics, Surface};

use super::{Area, Base, Cells, GameError};

/// The most cells one tiled layer may hold; a larger grid is the
/// `OutOfMemoryError` a phone would throw, rather than a host allocation
/// of gigabytes.
pub const MAX_CELLS: u64 = 1 << 24;

/// The state of one `TiledLayer`, without its image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TiledLayer {
    pub base: Base,
    columns: i32,
    rows: i32,
    tiles: Cells,
    /// Row by row.
    cells: Vec<i32>,
    /// The static tile of each animated tile: entry `k` is tile `-(k + 1)`.
    animated: Vec<i32>,
}

impl Default for TiledLayer {
    /// One empty cell of one pixel: what a `TiledLayer` holds before its
    /// constructor runs.
    fn default() -> Self {
        Self {
            base: Base::default(),
            columns: 1,
            rows: 1,
            tiles: Cells {
                width: 1,
                height: 1,
                columns: 1,
                count: 1,
            },
            cells: vec![0],
            animated: Vec::new(),
        }
    }
}

/// The width or height of the grid in pixels; past Java's `int` range is
/// the `IllegalArgumentException` a negative layer size gives.
fn extent(cells: i32, cell: i32) -> Result<i32, GameError> {
    i32::try_from(i64::from(cells) * i64::from(cell))
        .map_err(|_| GameError::IllegalArgument("the layer is too large"))
}

impl TiledLayer {
    /// `TiledLayer(columns, rows, image, tileWidth, tileHeight)` for an
    /// image of the given size. Every cell starts empty.
    pub fn new(
        columns: i32,
        rows: i32,
        image_width: u32,
        image_height: u32,
        tile_width: i32,
        tile_height: i32,
    ) -> Result<Self, GameError> {
        if columns < 1 || rows < 1 {
            return Err(GameError::IllegalArgument(
                "the number of rows or columns is less than 1",
            ));
        }
        let tiles = Cells::cut(image_width, image_height, tile_width, tile_height)?;
        extent(columns, tile_width)?;
        extent(rows, tile_height)?;
        let count = columns as u64 * rows as u64;
        if count > MAX_CELLS {
            return Err(GameError::OutOfMemory);
        }
        Ok(Self {
            base: Base::default(),
            columns,
            rows,
            tiles,
            cells: vec![0; count as usize],
            animated: Vec::new(),
        })
    }

    /// `getColumns()`.
    pub fn columns(&self) -> i32 {
        self.columns
    }

    /// `getRows()`.
    pub fn rows(&self) -> i32 {
        self.rows
    }

    /// `getCellWidth()`.
    pub fn cell_width(&self) -> i32 {
        self.tiles.width
    }

    /// `getCellHeight()`.
    pub fn cell_height(&self) -> i32 {
        self.tiles.height
    }

    /// `getWidth()`: the columns times the cell width.
    pub fn width(&self) -> i32 {
        self.columns * self.tiles.width
    }

    /// `getHeight()`: the rows times the cell height.
    pub fn height(&self) -> i32 {
        self.rows * self.tiles.height
    }

    /// The layer on the layer plane.
    pub fn bounds(&self) -> Area {
        Area::new(
            self.base.x.into(),
            self.base.y.into(),
            self.width().into(),
            self.height().into(),
        )
    }

    /// The memory the grid and the animated tiles take.
    pub fn heap_bytes(&self) -> usize {
        (self.cells.len() + self.animated.len()) * 4
    }

    /// `setStaticTileSet(image, tileWidth, tileHeight)`. With at least as
    /// many tiles as before the cells and animated tiles stay; with fewer
    /// every cell is emptied and the animated tiles are deleted.
    pub fn set_static_tile_set(
        &mut self,
        image_width: u32,
        image_height: u32,
        tile_width: i32,
        tile_height: i32,
    ) -> Result<(), GameError> {
        let tiles = Cells::cut(image_width, image_height, tile_width, tile_height)?;
        extent(self.columns, tile_width)?;
        extent(self.rows, tile_height)?;
        if tiles.count < self.tiles.count {
            self.cells.fill(0);
            self.animated.clear();
        }
        self.tiles = tiles;
        Ok(())
    }

    /// Whether `tile` names a static tile, 0 included.
    fn is_static(&self, tile: i32) -> bool {
        (0..=self.tiles.count).contains(&tile)
    }

    /// The entry of animated tile `tile` (-1 and down), if it exists.
    fn animated_slot(&self, tile: i32) -> Option<usize> {
        let k = usize::try_from(-i64::from(tile) - 1).ok()?;
        (k < self.animated.len()).then_some(k)
    }

    /// Whether a cell may hold `tile`: 0, a static tile or an existing
    /// animated tile.
    fn check_tile(&self, tile: i32) -> Result<(), GameError> {
        if self.is_static(tile) || self.animated_slot(tile).is_some() {
            Ok(())
        } else {
            Err(GameError::IndexOutOfBounds(
                "there is no tile with this index",
            ))
        }
    }

    /// `createAnimatedTile(staticTileIndex)`: returns -1 for the first,
    /// then -2 and so on.
    pub fn create_animated_tile(&mut self, static_tile: i32) -> Result<i32, GameError> {
        if !self.is_static(static_tile) {
            return Err(GameError::IndexOutOfBounds("invalid static tile index"));
        }
        self.animated.push(static_tile);
        Ok(-(self.animated.len() as i32))
    }

    /// `setAnimatedTile(animatedTileIndex, staticTileIndex)`.
    pub fn set_animated_tile(&mut self, tile: i32, static_tile: i32) -> Result<(), GameError> {
        if !self.is_static(static_tile) {
            return Err(GameError::IndexOutOfBounds("invalid static tile index"));
        }
        let k = self
            .animated_slot(tile)
            .ok_or(GameError::IndexOutOfBounds("invalid animated tile index"))?;
        self.animated[k] = static_tile;
        Ok(())
    }

    /// `getAnimatedTile(animatedTileIndex)`.
    pub fn animated_tile(&self, tile: i32) -> Result<i32, GameError> {
        self.animated_slot(tile)
            .map(|k| self.animated[k])
            .ok_or(GameError::IndexOutOfBounds("invalid animated tile index"))
    }

    /// The index of cell `(column, row)` in `cells`, if it is in the grid.
    fn slot(&self, column: i32, row: i32) -> Option<usize> {
        let inside = (0..self.columns).contains(&column) && (0..self.rows).contains(&row);
        inside.then(|| row as usize * self.columns as usize + column as usize)
    }

    fn checked_slot(&self, column: i32, row: i32) -> Result<usize, GameError> {
        self.slot(column, row)
            .ok_or(GameError::IndexOutOfBounds("the cell is outside the grid"))
    }

    /// `setCell(col, row, tileIndex)`.
    pub fn set_cell(&mut self, column: i32, row: i32, tile: i32) -> Result<(), GameError> {
        let slot = self.checked_slot(column, row)?;
        self.check_tile(tile)?;
        self.cells[slot] = tile;
        Ok(())
    }

    /// `getCell(col, row)`.
    pub fn cell(&self, column: i32, row: i32) -> Result<i32, GameError> {
        Ok(self.cells[self.checked_slot(column, row)?])
    }

    /// `fillCells(col, row, numCols, numRows, tileIndex)`. A count of zero
    /// changes nothing, but the first cell must still be in the grid.
    pub fn fill_cells(
        &mut self,
        column: i32,
        row: i32,
        columns: i32,
        rows: i32,
        tile: i32,
    ) -> Result<(), GameError> {
        if columns < 0 || rows < 0 {
            return Err(GameError::IllegalArgument(
                "the number of columns or rows is negative",
            ));
        }
        self.checked_slot(column, row)?;
        let fits = i64::from(column) + i64::from(columns) <= i64::from(self.columns)
            && i64::from(row) + i64::from(rows) <= i64::from(self.rows);
        if !fits {
            return Err(GameError::IndexOutOfBounds(
                "the region extends beyond the grid",
            ));
        }
        self.check_tile(tile)?;
        let width = self.columns as usize;
        for r in row as usize..(row + rows) as usize {
            let start = r * width + column as usize;
            self.cells[start..start + columns as usize].fill(tile);
        }
        Ok(())
    }

    /// What cell `(column, row)` holds, 0 outside the grid.
    pub fn cell_value(&self, column: i32, row: i32) -> i32 {
        self.slot(column, row).map_or(0, |slot| self.cells[slot])
    }

    /// The static tile cell `(column, row)` shows now, an animated tile
    /// read through; 0 for none.
    pub fn tile_at(&self, column: i32, row: i32) -> i32 {
        match self.cell_value(column, row) {
            tile if tile < 0 => self.animated_slot(tile).map_or(0, |k| self.animated[k]),
            tile => tile,
        }
    }

    /// The part of the image static tile `tile` (1 and up) covers.
    pub fn tile_region(&self, tile: i32) -> Area {
        self.tiles.region(tile - 1)
    }

    /// The columns and rows of the cells `area` touches; `area` lies
    /// inside the layer.
    pub fn cells_in(&self, area: Area) -> (Range<i32>, Range<i32>) {
        let layer = self.bounds();
        let (cw, ch) = (i64::from(self.tiles.width), i64::from(self.tiles.height));
        let span = |from: i64, to: i64, size: i64, count: i32| {
            let first = from.div_euclid(size).max(0);
            let last = (to - 1).div_euclid(size).min(i64::from(count) - 1);
            first as i32..(last + 1).max(first) as i32
        };
        (
            span(
                area.left - layer.left,
                area.right - layer.left,
                cw,
                self.columns,
            ),
            span(area.top - layer.top, area.bottom - layer.top, ch, self.rows),
        )
    }

    /// `paint(g)`: the tiles of the cells inside the clip, each through
    /// `drawRegion`. Empty cells and animated tiles showing 0 draw nothing,
    /// nor does an invisible layer.
    pub fn paint(
        &self,
        g: &Graphics,
        target: &mut Surface,
        image: &Surface,
    ) -> Result<(), GfxError> {
        if !self.base.visible {
            return Ok(());
        }
        let clip = Area::new(
            g.clip_x().into(),
            g.clip_y().into(),
            g.clip_width().into(),
            g.clip_height().into(),
        )
        .intersect(self.bounds());
        if clip.is_empty() {
            return Ok(());
        }
        let (columns, rows) = self.cells_in(clip);
        let (cw, ch) = (self.tiles.width, self.tiles.height);
        for row in rows {
            for column in columns.clone() {
                let tile = self.tile_at(column, row);
                if tile == 0 {
                    continue;
                }
                let source = self.tile_region(tile);
                let x = i64::from(self.base.x) + i64::from(column) * i64::from(cw);
                let y = i64::from(self.base.y) + i64::from(row) * i64::from(ch);
                g.draw_region(
                    target,
                    image,
                    source.left as i32,
                    source.top as i32,
                    cw,
                    ch,
                    0,
                    x as i32,
                    y as i32,
                    TOP | LEFT,
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{numbered, screen};
    use super::*;

    fn layer() -> TiledLayer {
        // A 4 by 2 image of 2 by 1 tiles: four static tiles.
        TiledLayer::new(5, 3, 4, 2, 2, 1).unwrap()
    }

    #[test]
    fn constructor_rules() {
        let iae =
            |r: Result<TiledLayer, GameError>| matches!(r, Err(GameError::IllegalArgument(_)));
        assert!(iae(TiledLayer::new(0, 1, 4, 2, 2, 1)));
        assert!(iae(TiledLayer::new(1, 0, 4, 2, 2, 1)));
        assert!(iae(TiledLayer::new(1, 1, 4, 2, 0, 1)));
        assert!(iae(TiledLayer::new(1, 1, 4, 2, 3, 1)));
        assert!(iae(TiledLayer::new(1, 1, 4, 2, 2, 3)));
        assert!(
            iae(TiledLayer::new(i32::MAX, 1, 4, 2, 2, 1)),
            "wider than an int"
        );
        assert_eq!(
            TiledLayer::new(1 << 13, 1 << 13, 4, 2, 2, 1),
            Err(GameError::OutOfMemory)
        );
        let t = layer();
        assert_eq!(
            (t.columns(), t.rows(), t.cell_width(), t.cell_height()),
            (5, 3, 2, 1)
        );
        assert_eq!((t.width(), t.height()), (10, 3));
        assert!((0..3).all(|r| (0..5).all(|c| t.cell(c, r) == Ok(0))));
    }

    #[test]
    fn cells_take_static_and_animated_tiles() {
        let mut t = layer();
        t.set_cell(4, 2, 4).unwrap();
        assert_eq!(t.cell(4, 2), Ok(4));
        let ioobe = |r: Result<(), GameError>| matches!(r, Err(GameError::IndexOutOfBounds(_)));
        assert!(ioobe(t.set_cell(0, 0, 5)), "no tile 5");
        assert!(ioobe(t.set_cell(0, 0, -1)), "no animated tile yet");
        assert!(ioobe(t.set_cell(5, 0, 1)));
        assert!(ioobe(t.set_cell(0, -1, 1)));
        assert!(t.cell(0, 3).is_err());
        assert_eq!(t.create_animated_tile(2), Ok(-1));
        assert_eq!(t.create_animated_tile(0), Ok(-2), "tile 0 is allowed");
        assert!(matches!(
            t.create_animated_tile(5),
            Err(GameError::IndexOutOfBounds(_))
        ));
        assert!(matches!(
            t.create_animated_tile(-1),
            Err(GameError::IndexOutOfBounds(_))
        ));
        t.set_cell(1, 1, -2).unwrap();
        assert!(ioobe(t.set_cell(1, 1, -3)));
        assert!(ioobe(t.set_cell(1, 1, i32::MIN)));
        assert_eq!(t.tile_at(1, 1), 0);
        t.set_animated_tile(-2, 3).unwrap();
        assert_eq!((t.animated_tile(-2), t.tile_at(1, 1)), (Ok(3), 3));
        assert!(ioobe(t.set_animated_tile(-3, 1)));
        assert!(ioobe(t.set_animated_tile(0, 1)));
        assert!(ioobe(t.set_animated_tile(-1, 5)));
        assert!(t.animated_tile(1).is_err());
    }

    #[test]
    fn fill_cells_checks_the_whole_region() {
        let mut t = layer();
        t.fill_cells(1, 1, 3, 2, 2).unwrap();
        let grid: Vec<i32> = (0..3)
            .flat_map(|r| (0..5).map(move |c| (c, r)))
            .map(|(c, r)| t.cell(c, r).unwrap())
            .collect();
        assert_eq!(grid, [0, 0, 0, 0, 0, 0, 2, 2, 2, 0, 0, 2, 2, 2, 0]);
        assert!(matches!(
            t.fill_cells(0, 0, -1, 1, 1),
            Err(GameError::IllegalArgument(_))
        ));
        assert!(matches!(
            t.fill_cells(3, 0, 3, 1, 1),
            Err(GameError::IndexOutOfBounds(_))
        ));
        assert!(matches!(
            t.fill_cells(0, 0, 1, 1, 9),
            Err(GameError::IndexOutOfBounds(_))
        ));
        assert!(matches!(
            t.fill_cells(5, 0, 0, 0, 1),
            Err(GameError::IndexOutOfBounds(_))
        ));
        t.fill_cells(4, 2, 0, 0, 1).unwrap();
        t.fill_cells(0, 0, 5, 3, 0).unwrap();
        assert!((0..3).all(|r| (0..5).all(|c| t.cell(c, r) == Ok(0))));
    }

    #[test]
    fn static_tile_sets_keep_or_clear_the_grid() {
        let mut t = layer();
        t.set_cell(0, 0, 4).unwrap();
        let a = t.create_animated_tile(1).unwrap();
        t.set_static_tile_set(8, 2, 2, 2).unwrap();
        assert_eq!((t.cell_width(), t.cell_height(), t.width()), (2, 2, 10));
        assert_eq!(t.cell(0, 0), Ok(4), "as many tiles keep the cells");
        assert_eq!(t.animated_tile(a), Ok(1));
        t.set_static_tile_set(2, 2, 2, 2).unwrap();
        assert_eq!(t.cell(0, 0), Ok(0), "fewer tiles clear the cells");
        assert!(t.animated_tile(a).is_err(), "and delete the animated tiles");
        assert_eq!(t.create_animated_tile(1), Ok(-1));
        assert!(matches!(
            t.set_static_tile_set(2, 2, 3, 2),
            Err(GameError::IllegalArgument(_))
        ));
    }

    #[test]
    fn paint_draws_each_cell_with_its_tile() {
        let image = numbered(4, 2);
        let mut t = layer();
        t.base.x = 3;
        t.base.y = 4;
        t.set_cell(0, 0, 1).unwrap();
        t.set_cell(1, 0, 4).unwrap();
        let a = t.create_animated_tile(2).unwrap();
        t.set_cell(4, 2, a).unwrap();
        let (g, mut target) = screen(20, 20);
        t.paint(&g, &mut target, &image).unwrap();
        let at = |x: u32, y: u32| target.pixel(x, y);
        assert_eq!((at(3, 4), at(4, 4)), (image.pixel(0, 0), image.pixel(1, 0)));
        assert_eq!((at(5, 4), at(6, 4)), (image.pixel(2, 1), image.pixel(3, 1)));
        assert_eq!(
            (at(11, 6), at(12, 6)),
            (image.pixel(2, 0), image.pixel(3, 0))
        );
        assert_eq!(at(7, 4), Some(0xFFFF_FFFF), "an empty cell");
        assert_eq!(at(3, 5), Some(0xFFFF_FFFF));
        t.set_animated_tile(a, 3).unwrap();
        t.paint(&g, &mut target, &image).unwrap();
        assert_eq!(
            target.pixel(11, 6),
            image.pixel(0, 1),
            "the animated tile changed"
        );
        t.set_animated_tile(a, 0).unwrap();
        let (g, mut target) = screen(20, 20);
        t.paint(&g, &mut target, &image).unwrap();
        assert_eq!(
            target.pixel(11, 6),
            Some(0xFFFF_FFFF),
            "it shows nothing now"
        );
    }

    #[test]
    fn paint_draws_only_the_cells_inside_the_clip() {
        let image = numbered(4, 2);
        let mut t = layer();
        t.fill_cells(0, 0, 5, 3, 1).unwrap();
        let (mut g, mut target) = screen(20, 20);
        g.set_clip(3, 1, 4, 1);
        t.paint(&g, &mut target, &image).unwrap();
        let drawn: Vec<(u32, u32)> = (0..20)
            .flat_map(|y| (0..20).map(move |x| (x, y)))
            .filter(|&(x, y)| target.pixel(x, y) != Some(0xFFFF_FFFF))
            .collect();
        assert_eq!(drawn, [(3, 1), (4, 1), (5, 1), (6, 1)]);
        let cells = t.cells_in(Area::new(3, 1, 4, 1));
        assert_eq!(cells, (1..4, 1..2), "the three cells the clip touches");
        g.translate(-100, 0);
        let (_, mut untouched) = screen(20, 20);
        t.paint(&g, &mut untouched, &image).unwrap();
        assert!(
            untouched.pixels().iter().all(|&p| p == 0xFFFF_FFFF),
            "moved out"
        );
    }
}
