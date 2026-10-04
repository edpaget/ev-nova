//! The parts the Outfitter and the Shipyard share: the grid of cells in
//! their dialog's grid item, with its selection, scrolling and arrows; a
//! `ShortName`'s lines; and the selected item's picture or "No Picture",
//! and its description, as read.
//!
//! The grid shows [`COLUMNS`] cells across and [`GRID_ROWS`] down, each
//! [`CELL_WIDTH`] x [`CELL_HEIGHT`] from the grid item's top left. One cell
//! is selected; the grid scrolls a row at a time, never past either end,
//! and always shows the selected cell when the selection moves.

use std::rc::Rc;

use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::Key;
use crate::text::TextMetrics;
use crate::ui::catalog::DescriptionSource;
use crate::ui::desc::{BODY_COLOR, BODY_SIZE};
use crate::ui::scroll_text::ScrollText;

/// How many cells across the grid shows.
pub const COLUMNS: usize = 4;
/// How many rows of cells the grid shows at once.
pub const GRID_ROWS: usize = 4;
/// A cell's width: four fit the stock grid's 333.
pub const CELL_WIDTH: f32 = 83.0;
/// A cell's height: four fit the stock grid's 271.
pub const CELL_HEIGHT: f32 = 67.0;

/// `STR#` 2002 #213.
pub const NO_PICTURE: &str = "No Picture";

/// The screens' text font.
pub const TEXT_FONT: Font = Font::Geneva;
/// The screens' text size.
pub const TEXT_SIZE: f32 = 10.0;
/// A line that starts with a letter or digit, and the info box.
pub const TEXT_COLOR: Color = Color::WHITE;
/// A line that starts with anything else, the cells' frames and an arrow
/// that cannot scroll.
pub const GREY: Color = Color::DIM;
/// The selected cell's background.
pub const SELECTED_COLOR: Color = Color::rgba(40, 60, 140, 255);
/// How far into a cell or box each line of text starts.
pub const INSET: f32 = 4.0;

/// A `ShortName`'s lines, each with its colour: split on a literal `\n`,
/// white when it starts with a letter or digit, grey otherwise.
#[must_use]
pub fn name_lines(short_name: &str) -> Vec<(&str, Color)> {
    short_name
        .split("\\n")
        .map(|line| {
            let bright = line.chars().next().is_some_and(char::is_alphanumeric);
            (line, if bright { TEXT_COLOR } else { GREY })
        })
        .collect()
}

/// `text` at `origin`, in the screens' font, in `color`.
pub fn text(list: &mut DrawList, text: &str, origin: Point, color: Color) {
    list.text_in(TEXT_FONT, text, origin, TEXT_SIZE, None, color);
}

/// Where cell `index` (counted from the grid's first row shown) is, in a
/// grid item at `grid`.
#[must_use]
pub fn cell_bounds(grid: Bounds, index: usize) -> Bounds {
    let (row, column) = (index / COLUMNS, index % COLUMNS);
    let min = Point::new(
        grid.min.x + CELL_WIDTH * column as f32,
        grid.min.y + CELL_HEIGHT * row as f32,
    );
    Bounds::at(min, CELL_WIDTH, CELL_HEIGHT)
}

/// The grid's selection and scrolling, over a number of cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellGrid {
    /// The selected cell, by index into the cells.
    selected: usize,
    /// The grid's first row shown.
    top_row: usize,
}

impl CellGrid {
    /// The selected cell's index, if there are any of `count` cells.
    #[must_use]
    pub fn selected(&self, count: usize) -> Option<usize> {
        (self.selected < count).then_some(self.selected)
    }

    /// The selected cell's index, whether or not there is such a cell.
    #[must_use]
    pub fn selected_raw(&self) -> usize {
        self.selected
    }

    /// The grid's first row shown.
    #[must_use]
    pub fn top_row(&self) -> usize {
        self.top_row
    }

    /// The grid's last first row, for `count` cells: never scrolled past
    /// its end.
    fn last_top_row(count: usize) -> usize {
        count.div_ceil(COLUMNS).saturating_sub(GRID_ROWS)
    }

    /// Selects cell `index` of `count`, or the last when there is no such
    /// cell, and scrolls the grid to show it.
    pub fn select(&mut self, index: usize, count: usize) {
        self.selected = index.min(count.saturating_sub(1));
        let row = self.selected / COLUMNS;
        self.top_row = self
            .top_row
            .min(Self::last_top_row(count))
            .min(row)
            .max((row + 1).saturating_sub(GRID_ROWS));
    }

    /// Moves the selection for an arrow `key` going down, among `count`
    /// cells, and says whether it was an arrow.
    pub fn arrow(&mut self, key: Key, count: usize) -> bool {
        let index = match key {
            Key::Left => self.selected.saturating_sub(1),
            Key::Right => self.selected + 1,
            Key::Up => self.selected.saturating_sub(COLUMNS),
            Key::Down => self.selected + COLUMNS,
            _ => return false,
        };
        self.select(index, count);
        true
    }

    /// Scrolls the grid of `count` cells a row down, or up, never past
    /// either end.
    pub fn scroll(&mut self, down: bool, count: usize) {
        self.top_row = if down {
            (self.top_row + 1).min(Self::last_top_row(count))
        } else {
            self.top_row.saturating_sub(1)
        };
    }

    /// Whether the grid can scroll up, and down, with `count` cells.
    #[must_use]
    pub fn can_scroll(&self, count: usize) -> (bool, bool) {
        (self.top_row > 0, self.top_row < Self::last_top_row(count))
    }

    /// The index of the cell of `count` under `at` on a grid item at
    /// `grid`, if there is one.
    #[must_use]
    pub fn cell_at(&self, grid: Bounds, at: Point, count: usize) -> Option<usize> {
        if !grid.contains(at) {
            return None;
        }
        let column = ((at.x - grid.min.x) / CELL_WIDTH) as usize;
        let row = ((at.y - grid.min.y) / CELL_HEIGHT) as usize;
        if column >= COLUMNS || row >= GRID_ROWS {
            return None;
        }
        let index = (self.top_row + row) * COLUMNS + column;
        (index < count).then_some(index)
    }

    /// The cells of `count` shown, each index with where its cell is in a
    /// grid item at `grid`.
    #[must_use]
    pub fn shown(&self, grid: Bounds, count: usize) -> Vec<(usize, Bounds)> {
        let first = self.top_row * COLUMNS;
        (first..count.min(first + COLUMNS * GRID_ROWS))
            .map(|index| (index, cell_bounds(grid, index - first)))
            .collect()
    }
}

/// The scroll arrows: a triangle in each of `up` and `down`, white when
/// the grid can scroll that way (`can`), grey otherwise.
pub fn draw_arrows(
    list: &mut DrawList,
    up: Option<Bounds>,
    down: Option<Bounds>,
    (can_up, can_down): (bool, bool),
) {
    for (area, upward, can) in [(up, true, can_up), (down, false, can_down)] {
        let Some(area) = area else {
            continue;
        };
        let color = if can { TEXT_COLOR } else { GREY };
        let (left, right) = (area.min.x + INSET, area.max.x - INSET);
        let (tip, base) = if upward {
            (area.min.y + INSET, area.max.y - INSET)
        } else {
            (area.max.y - INSET, area.min.y + INSET)
        };
        let apex = Point::new(area.center().x, tip);
        list.line(Point::new(left, base), Point::new(right, base), 1.0, color);
        list.line(Point::new(right, base), apex, 1.0, color);
        list.line(apex, Point::new(left, base), 1.0, color);
    }
}

/// The selected item's picture and description, as read.
#[derive(Clone, Debug)]
pub struct Shown {
    /// The `PICT`, when there is one.
    pub picture: Option<i16>,
    /// The description, scrolling in its box.
    pub description: ScrollText,
}

impl Shown {
    /// `picture`, and `dësc` `description` read from `catalog` (none when
    /// it is missing) laid out in `area`.
    pub fn read(
        catalog: &dyn DescriptionSource,
        picture: Option<i16>,
        description: Option<i16>,
        area: Bounds,
        metrics: &Rc<dyn TextMetrics>,
    ) -> Self {
        let text = description
            .and_then(|id| catalog.description(id).ok())
            .unwrap_or_default();
        Self::read_text(&text, picture, area, metrics)
    }

    /// `picture`, and the description `text` laid out in `area`.
    pub fn read_text(
        text: &str,
        picture: Option<i16>,
        area: Bounds,
        metrics: &Rc<dyn TextMetrics>,
    ) -> Self {
        Self {
            picture,
            description: ScrollText::new(text, Font::Geneva, BODY_SIZE, BODY_COLOR, area, metrics),
        }
    }

    /// The picture stretched into `area`, or "No Picture" in it, then the
    /// description.
    pub fn draw(&self, list: &mut DrawList, area: Bounds) {
        if let Some(id) = self.picture {
            list.stretched_picture(ImageKey::picture(id), area.min, area.width(), area.height());
        } else {
            let origin = Point::new(area.min.x + INSET, area.min.y + INSET);
            text(list, NO_PICTURE, origin, GREY);
        }
        self.description.draw(list);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    #[test]
    fn cells_go_across_then_down_from_the_grids_corner() {
        let grid = rect(10.0, 20.0, 333.0, 271.0);
        assert_eq!(cell_bounds(grid, 0), rect(10.0, 20.0, 83.0, 67.0));
        assert_eq!(cell_bounds(grid, 5), rect(93.0, 87.0, 83.0, 67.0));
    }

    #[test]
    fn the_selection_clamps_and_the_grid_follows_it() {
        let mut grid = CellGrid::default();
        assert_eq!(grid.selected(0), None);
        grid.select(30, 22);
        assert_eq!((grid.selected(22), grid.top_row()), (Some(21), 2));
        assert_eq!(grid.selected_raw(), 21);
        assert_eq!(grid.can_scroll(22), (true, false));
        grid.select(0, 22);
        assert_eq!((grid.selected(22), grid.top_row()), (Some(0), 0));
        assert!(grid.arrow(Key::Down, 22));
        assert_eq!(grid.selected(22), Some(4));
        assert!(!grid.arrow(Key::Enter, 22));
        assert_eq!(grid.selected(22), Some(4));
        grid.scroll(true, 22);
        grid.scroll(true, 22);
        grid.scroll(true, 22);
        assert_eq!(grid.top_row(), 2);
        assert_eq!(grid.can_scroll(16), (true, false));
    }

    #[test]
    fn the_cells_shown_are_the_rows_from_the_top_one() {
        let area = rect(0.0, 0.0, 333.0, 271.0);
        let mut grid = CellGrid::default();
        grid.scroll(true, 22);
        let shown = grid.shown(area, 22);
        assert_eq!(shown.len(), 16);
        assert_eq!(shown[0], (4, cell_bounds(area, 0)));
        assert_eq!(shown[15].0, 19);
        assert_eq!(grid.shown(area, 3).len(), 0, "nothing past the end");
        assert_eq!(CellGrid::default().shown(area, 3).len(), 3);
        assert_eq!(
            grid.cell_at(area, Point::new(1.0, 1.0), 22),
            Some(4),
            "under the scroll"
        );
    }
}
