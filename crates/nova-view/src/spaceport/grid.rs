//! The parts the Outfitter and the Shipyard share: the grid of cells in
//! their dialog's grid item, with its selection, scrolling and arrows; a
//! `ShortName`'s lines; and the selected item's picture, or "No Picture"
//! and "Available" centred on its item, and its description, as read.
//!
//! The grid shows [`COLUMNS`] cells across and [`GRID_ROWS`] down, the 20
//! slots of `_SetupListRects` (@0x51a2c-0x51af8): each a [`CELL_WIDTH`] x
//! [`CELL_HEIGHT`] frame, set [`CELL_PITCH_X`] x [`CELL_PITCH_Y`] apart from
//! the grid item's top left, so neighbouring frames share a line. Every
//! listed cell is framed in the `cölr` `GridDim` colour and the selected one
//! framed again, last, in `GridBright` ([`GridColors`]); no cell is filled. A
//! press is hit-tested on the frame's size, not the pitch. One cell is
//! selected, or none once cleared (the outfitter's, the shipyard's and
//! the hire screen's, by the engine); the grid scrolls a row at a time,
//! never past either end, dropping a selection it scrolls out of view
//! when asked ([`CellGrid::scroll_dropping_hidden`]), and always shows
//! the selected cell when the selection moves. The
//! arrow keys move the selection as
//! [`grid_arrows`](super::grid_arrows) decides, by
//! [`RuleKey::GridArrows`](nova_sim::RuleKey::GridArrows).
//!
//! A cell shows its item's picture, stretched to [`ICON_SIZE`] square,
//! centred across the cell near its top; its `ShortName` centred across
//! the bottom; and a note in a top corner: the original's placement
//! (`_SetupListRects` @0x51a2c, `_RetrieveMenuIcon` @0x4f425, and the cell
//! loops of `_OutfitDialogUpdate` @0x572cb and `_ShipyardDialogUpdate`
//! @0x58a10). A missing picture is a black square, as the original paints
//! it.

use std::rc::Rc;

use super::grid_arrows::{ArrowMoves, Placed, arrow_move};
use crate::color::Color;
use crate::draw::{DrawList, fill_rect, frame_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::Key;
use crate::text::TextMetrics;
use crate::ui::catalog::DescriptionSource;
use crate::ui::desc::{BODY_COLOR, BODY_SIZE};
use crate::ui::text_box::TextBox;

/// How many cells across the grid shows.
pub const COLUMNS: usize = 4;
/// How many rows of cells the grid shows at once: `_SetupListRects` lays
/// out 20 slots, and the dialog updates draw a slot only while its index
/// less the scroll is at most 0x13 (`_OutfitDialogUpdate` @0x573ef,
/// `_ShipyardDialogUpdate` @0x58b30).
pub const GRID_ROWS: usize = 5;
/// A cell's frame's width, 0x54 (`_SetupListRects` @0x51a99), and the
/// width a press's column is divided by (`_OutfitFilter` @0x585b1,
/// `_ShipyardFilter` @0x609dc).
pub const CELL_WIDTH: f32 = 84.0;
/// A cell's frame's height, 0x37 (`_SetupListRects` @0x51a99), and the
/// height a press's row is divided by (`_OutfitFilter` @0x585b1,
/// `_ShipyardFilter` @0x609fe).
pub const CELL_HEIGHT: f32 = 55.0;
/// The step from one cell's left to the next's, 0x53: one less than the
/// width, so neighbouring frames share a line, and four fill the stock
/// grid's 333 with the last frame's line.
pub const CELL_PITCH_X: f32 = 83.0;
/// The step from one row's top to the next's, 0x36: one less than the
/// height, and five fill the stock grid's 271.
pub const CELL_PITCH_Y: f32 = 54.0;

/// `STR#` 2002 #213: the first line drawn on a missing picture.
pub const NO_PICTURE: &str = "No Picture";
/// `STR#` 2002 #214: the second line drawn on a missing picture.
pub const PICTURE_AVAILABLE: &str = "Available";
/// How far [`NO_PICTURE`]'s baseline is above, and [`PICTURE_AVAILABLE`]'s
/// below, the picture item's vertical middle (`_OutfitDialogUpdate`
/// @0x57a2a-0x57a43, @0x57a75-0x57a8e).
pub const NO_PICTURE_OFFSET: f32 = 6.0;

/// The screens' text font.
pub const TEXT_FONT: Font = Font::Geneva;
/// The screens' text size.
pub const TEXT_SIZE: f32 = 10.0;
/// A line that starts with a letter or digit, and the info box.
pub const TEXT_COLOR: Color = Color::WHITE;
/// A line that starts with anything else, and an arrow that cannot
/// scroll.
pub const GREY: Color = Color::DIM;

/// The grid's two frame colours, `_shopGridColor`: `cölr`'s `GridBright`
/// then `GridDim` (@0x57848-0x578c7, @0x58f44-0x58fbf).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridColors {
    /// Every listed cell's frame, `_shopGridColor[1]`.
    pub dim: Color,
    /// The selected cell's frame, drawn over the dim ones,
    /// `_shopGridColor[0]`.
    pub bright: Color,
}

impl GridColors {
    /// Stock `cölr` 128's: `GridDim` 0x404040 and `GridBright` 0xFF0000.
    pub const STOCK: Self = Self {
        dim: Color::from_rgb24(0x0040_4040),
        bright: Color::from_rgb24(0x00FF_0000),
    };
}

/// The grid's look.
pub trait GridLooks {
    /// The grid's colours, from `cölr` 128, or [`GridColors::STOCK`] when
    /// it is missing or does not decode.
    fn grid_colors(&self) -> GridColors;
}

/// Distinct non-stock grid colours, for test fakes to tell the frames
/// apart.
#[cfg(test)]
pub(crate) const TEST_GRID_COLORS: GridColors = GridColors {
    dim: Color::rgba(1, 2, 3, 255),
    bright: Color::rgba(4, 5, 6, 255),
};

/// Frames `cell` in `color`, as the dialog updates `FrameRect` each listed
/// cell and then the selected one.
fn frame_cell(list: &mut DrawList, cell: Bounds, color: Color) {
    frame_rect(list, cell, color);
}

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

/// A cell's picture's width and height: the original stretches the whole
/// `PICT` into a 32 x 32 slot (`_RetrieveMenuIcon` @0x4f425, its slots set
/// by `_SetupListRects` @0x51afe-0x51b36).
pub const ICON_SIZE: f32 = 32.0;
/// How far a cell's picture's top is above the cell's whole-number centre
/// (`_SetupListRects` @0x51a9e-0x51af0): 3 below a 55-tall cell's top.
pub const ICON_RISE: f32 = 24.0;
/// The corner text's (the owned count's or the mark's) baseline, below the
/// cell's top (`_OutfitDialogUpdate` @0x57471-0x5752d).
pub const CORNER_BASELINE: f32 = 12.0;
/// How far the corner text sits inside the cell's side.
pub const CORNER_INSET: f32 = 3.0;
/// A two-line name's baselines, above the cell's bottom
/// (`_OutfitDialogUpdate` @0x5769f-0x5781d, `_ShipyardDialogUpdate`
/// @0x58d86-0x58f33).
pub const NAME_BASELINES: [f32; 2] = [14.0, 3.0];
/// A one-line name's baseline, above the cell's bottom.
pub const ONE_LINE_BASELINE: f32 = 6.0;
/// The gap between a name's baselines, for a name of more than two lines
/// (the original splits into at most two).
pub const NAME_LINE_STEP: f32 = NAME_BASELINES[0] - NAME_BASELINES[1];

/// Where a cell's picture goes: [`ICON_SIZE`] square, centred across the
/// cell, its top [`ICON_RISE`] above the cell's centre. The centre is
/// halved to a whole number on each axis, as `_SetupListRects` truncates
/// (left + right) / 2 and (top + bottom) / 2 (@0x51a9e-0x51ad0).
#[must_use]
pub fn icon_bounds(cell: Bounds) -> Bounds {
    let c = cell.center();
    let (x, y) = (c.x.floor(), c.y.floor());
    Bounds::at(
        Point::new(x - ICON_SIZE / 2.0, y - ICON_RISE),
        ICON_SIZE,
        ICON_SIZE,
    )
}

/// `PICT` `picture` stretched into `cell`'s icon, or, with none, the icon
/// painted black, as `_RetrieveMenuIcon` paints a missing `PICT`'s slot
/// (@0x4f56f-0x4f582).
pub fn draw_cell_picture(list: &mut DrawList, cell: Bounds, picture: Option<i16>) {
    let icon = icon_bounds(cell);
    match picture {
        Some(id) => {
            list.stretched_picture(ImageKey::picture(id), icon.min, ICON_SIZE, ICON_SIZE);
        }
        None => fill_rect(list, icon, Color::BLACK),
    }
}

/// The top of a line of the screens' text whose baseline is `baseline`.
pub(crate) fn baseline_top(baseline: f32) -> f32 {
    baseline - TEXT_SIZE
}

/// How a run of text is set: its font, size and colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    /// The font.
    pub font: Font,
    /// The size.
    pub size: f32,
    /// The colour.
    pub color: Color,
}

/// The screens' text: [`TEXT_FONT`] at [`TEXT_SIZE`] in [`TEXT_COLOR`].
pub const SCREEN_TEXT: TextStyle = TextStyle {
    font: TEXT_FONT,
    size: TEXT_SIZE,
    color: TEXT_COLOR,
};

/// `words` in white, centred across `area` on `baseline`, as
/// `_DrawCenteredText` (@0x263d) draws it: from the floor of the middle of
/// `area`'s sides, less half the text's width.
pub fn draw_centred_text(
    list: &mut DrawList,
    metrics: &dyn TextMetrics,
    area: Bounds,
    baseline: f32,
    words: &str,
) {
    draw_centred_text_in(list, metrics, SCREEN_TEXT, area, baseline, words);
}

/// `words` set in `style`, centred across `area` on `baseline`, as
/// `_DrawCenteredText` (@0x263d) draws it: from the floor of the middle of
/// `area`'s sides, less half the text's width, its top the size above
/// the baseline.
pub fn draw_centred_text_in(
    list: &mut DrawList,
    metrics: &dyn TextMetrics,
    style: TextStyle,
    area: Bounds,
    baseline: f32,
    words: &str,
) {
    let middle = f32::midpoint(area.min.x, area.max.x).floor();
    let x = middle - metrics.width(style.font, style.size, words) / 2.0;
    list.text_in(
        style.font,
        words,
        Point::new(x, baseline - style.size),
        style.size,
        None,
        style.color,
    );
}

/// `short_name`'s lines, each in its colour and centred across `cell`: a
/// lone line [`ONE_LINE_BASELINE`] above its bottom, otherwise the last
/// line 3 above it and each earlier one [`NAME_LINE_STEP`] higher.
pub fn draw_cell_name(
    list: &mut DrawList,
    metrics: &dyn TextMetrics,
    cell: Bounds,
    short_name: &str,
) {
    let lines = name_lines(short_name);
    let final_line = lines.len() - 1;
    for (n, (line, color)) in lines.into_iter().enumerate() {
        let rise = if final_line == 0 {
            ONE_LINE_BASELINE
        } else {
            NAME_BASELINES[1] + NAME_LINE_STEP * (final_line - n) as f32
        };
        let x = cell.center().x - metrics.width(TEXT_FONT, TEXT_SIZE, line) / 2.0;
        text(
            list,
            line,
            Point::new(x, baseline_top(cell.max.y - rise)),
            color,
        );
    }
}

/// `words` in white in a top corner of `cell`, [`CORNER_INSET`] inside its
/// side: ending there on the `right`, starting there otherwise.
pub fn draw_corner_text(
    list: &mut DrawList,
    metrics: &dyn TextMetrics,
    cell: Bounds,
    words: &str,
    right: bool,
) {
    let x = if right {
        cell.max.x - CORNER_INSET - metrics.width(TEXT_FONT, TEXT_SIZE, words)
    } else {
        cell.min.x + CORNER_INSET
    };
    let top = baseline_top(cell.min.y + CORNER_BASELINE);
    text(list, words, Point::new(x, top), TEXT_COLOR);
}

/// Where cell `index` (counted from the grid's first row shown) is, in a
/// grid item at `grid`: a [`CELL_WIDTH`] x [`CELL_HEIGHT`] frame,
/// [`CELL_PITCH_X`] x [`CELL_PITCH_Y`] a step, as `_SetupListRects` sets
/// each slot (@0x51a2c-0x51a99).
#[must_use]
pub fn cell_bounds(grid: Bounds, index: usize) -> Bounds {
    let (row, column) = (index / COLUMNS, index % COLUMNS);
    let min = Point::new(
        grid.min.x + CELL_PITCH_X * column as f32,
        grid.min.y + CELL_PITCH_Y * row as f32,
    );
    Bounds::at(min, CELL_WIDTH, CELL_HEIGHT)
}

/// What [`CellGrid::scroll_dropping_hidden`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scrolled {
    /// The grid could not scroll that way: nothing changed.
    Stayed,
    /// The grid scrolled and the highlighted cell is still shown.
    Kept,
    /// The grid scrolled and no cell is highlighted: the one that was is
    /// out of view, or there was none, and the caller lets go of any item
    /// it held unhighlighted.
    Dropped,
}

/// The grid's selection and scrolling, over a number of cells.
///
/// By default cell 0 is selected; [`CellGrid::clear`] leaves no cell
/// highlighted, as `_ShipyardFilter` sets its cell to -1 (the shipyard and
/// the hire screen under
/// [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection)'s engine
/// reading), until a cell is selected or an arrow picks one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellGrid {
    /// The selected cell, by index into the cells, while
    /// [`CellGrid::highlighted`]; the last one selected otherwise.
    selected: usize,
    /// Whether a cell is highlighted at all.
    highlighted: bool,
    /// The grid's first row shown.
    top_row: usize,
}

impl Default for CellGrid {
    /// Cell 0 selected, the grid at its top.
    fn default() -> Self {
        Self {
            selected: 0,
            highlighted: true,
            top_row: 0,
        }
    }
}

impl CellGrid {
    /// The selected cell's index, if one is highlighted and there are any
    /// of `count` cells.
    #[must_use]
    pub fn selected(&self, count: usize) -> Option<usize> {
        (self.highlighted && self.selected < count).then_some(self.selected)
    }

    /// Whether a cell is highlighted: false once [`CellGrid::clear`]ed,
    /// until a cell is selected.
    #[must_use]
    pub fn highlighted(&self) -> bool {
        self.highlighted
    }

    /// Leaves no cell highlighted, the scroll as it is.
    pub fn clear(&mut self) {
        self.highlighted = false;
    }

    /// Where listed cell `index` is among the cells shown: its visible
    /// position, 0-19 while it is shown (`column + 4·row`), as
    /// `_ShipyardFilter` keeps its cell (`*0xded3c`, `*0xdecb4`).
    #[must_use]
    pub fn visible_position(&self, index: usize) -> usize {
        index.saturating_sub(self.top_row * COLUMNS)
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
        let placed = Self::place(index, self.top_row, count);
        self.highlighted = true;
        self.selected = placed.selected;
        self.top_row = placed.top_row;
    }

    /// Where selecting cell `index` of `count`, or the last when there is
    /// no such cell, leaves a grid whose first row shown is `top_row`: that
    /// cell, with the grid scrolled the least that shows it and never past
    /// its end. The one placement [`CellGrid::select`] and the arrows'
    /// moves ([`arrow_move`]) share.
    #[must_use]
    pub(super) fn place(index: usize, top_row: usize, count: usize) -> Placed {
        let selected = index.min(count.saturating_sub(1));
        let row = selected / COLUMNS;
        let top_row = top_row
            .min(Self::last_top_row(count))
            .min(row)
            .max((row + 1).saturating_sub(GRID_ROWS));
        Placed { selected, top_row }
    }

    /// Moves the selection for an arrow `key` going down, among `count`
    /// cells, by `moves`, the reading of
    /// [`RuleKey::GridArrows`](nova_sim::RuleKey::GridArrows), and says
    /// whether it was an arrow. Page Up and Page Down move as Up and Down.
    /// With no cell highlighted ([`CellGrid::clear`]), an arrow highlights
    /// one without scrolling: Right and Down the first cell shown, and Left
    /// and Up the last cell shown, or the last listed when fewer are. See
    /// [`arrow_move`] for the moves.
    pub fn arrow(&mut self, key: Key, count: usize, moves: ArrowMoves) -> bool {
        let Some(placed) = arrow_move(
            key,
            self.highlighted,
            self.selected,
            self.top_row,
            count,
            moves,
        ) else {
            return false;
        };
        self.highlighted = true;
        self.selected = placed.selected;
        self.top_row = placed.top_row;
        true
    }

    /// Scrolls the grid of `count` cells a row down, or up, never past
    /// either end, and says whether it moved.
    pub fn scroll(&mut self, down: bool, count: usize) -> bool {
        let before = self.top_row;
        self.top_row = if down {
            (self.top_row + 1).min(Self::last_top_row(count))
        } else {
            self.top_row.saturating_sub(1)
        };
        self.top_row != before
    }

    /// Scrolls the grid of `count` cells a row down, or up, as
    /// [`CellGrid::scroll`] does, and when it moved, drops the highlight
    /// unless the selected cell is still among the cells shown, as the
    /// outfitter's scroll arrows do (`_DoOutfitDialog` item 10
    /// @0x5d351-0x5d3d6, item 11 @0x5d6b0-0x5d759): Up keeps visible
    /// positions 0-15 and Down 4-19, and either clears the cell from any
    /// other, or from none.
    pub fn scroll_dropping_hidden(&mut self, down: bool, count: usize) -> Scrolled {
        if !self.scroll(down, count) {
            return Scrolled::Stayed;
        }
        if self.highlighted && self.shown_range(count).contains(&self.selected) {
            Scrolled::Kept
        } else {
            self.clear();
            Scrolled::Dropped
        }
    }

    /// Whether the grid can scroll up, and down, with `count` cells.
    #[must_use]
    pub fn can_scroll(&self, count: usize) -> (bool, bool) {
        (self.top_row > 0, self.top_row < Self::last_top_row(count))
    }

    /// The index, counted with the scroll, of the cell position under
    /// `at` on a grid item at `grid`, listed or not: `None` outside the
    /// item, which `PtInRect` tests half-open (its right and bottom edges
    /// out), or past its [`COLUMNS`] x [`GRID_ROWS`] cells. The column is
    /// the press's offset divided by [`CELL_WIDTH`] and the row by
    /// [`CELL_HEIGHT`], the frame's size rather than the drawing pitch, as
    /// `_OutfitFilter` (@0x585a4-0x58607) and `_ShipyardFilter`, which
    /// also runs the hire screen (@0x609cf-0x60a32), divide it.
    #[must_use]
    pub fn cell_under(&self, grid: Bounds, at: Point) -> Option<usize> {
        if !grid.contains_half_open(at) {
            return None;
        }
        let column = ((at.x - grid.min.x) / CELL_WIDTH) as usize;
        let row = ((at.y - grid.min.y) / CELL_HEIGHT) as usize;
        if column >= COLUMNS || row >= GRID_ROWS {
            return None;
        }
        Some((self.top_row + row) * COLUMNS + column)
    }

    /// The index of the cell of `count` under `at` on a grid item at
    /// `grid`, if there is one.
    #[must_use]
    pub fn cell_at(&self, grid: Bounds, at: Point, count: usize) -> Option<usize> {
        self.cell_under(grid, at).filter(|&index| index < count)
    }

    /// Where listed cell `index` of `count` is in a grid item at `grid`,
    /// when it is shown.
    #[must_use]
    pub fn shown_bounds(&self, grid: Bounds, index: usize, count: usize) -> Option<Bounds> {
        let first = self.top_row * COLUMNS;
        self.shown_range(count)
            .contains(&index)
            .then(|| cell_bounds(grid, index - first))
    }

    /// The indexes of the cells of `count` shown: the grid's rows from its
    /// first shown.
    fn shown_range(&self, count: usize) -> std::ops::Range<usize> {
        let first = self.top_row * COLUMNS;
        first..count.min(first + COLUMNS * GRID_ROWS)
    }

    /// The cells of `count` shown, each index with where its cell is in a
    /// grid item at `grid`.
    #[must_use]
    pub fn shown(&self, grid: Bounds, count: usize) -> Vec<(usize, Bounds)> {
        let first = self.top_row * COLUMNS;
        self.shown_range(count)
            .map(|index| (index, cell_bounds(grid, index - first)))
            .collect()
    }

    /// Draws the cells of `count` shown in a grid item at `grid`: each
    /// one's contents (`draw_contents`, given its index and bounds), then
    /// its frame in `colors.dim`; then, after every dim frame, the
    /// `selected` cell, when it is shown, framed again in `colors.bright`
    /// over its neighbours' shared lines. No cell is filled, as
    /// `_OutfitDialogUpdate` (@0x573ef-0x57895, @0x578a1-0x578c7) and
    /// `_ShipyardDialogUpdate`, which also draws the hire screen's grid
    /// (@0x58b30-0x58f91, @0x58f9d-0x58fbf), draw them.
    pub fn draw_cells(
        &self,
        list: &mut DrawList,
        grid: Bounds,
        count: usize,
        selected: Option<usize>,
        colors: GridColors,
        mut draw_contents: impl FnMut(&mut DrawList, usize, Bounds),
    ) {
        for (index, cell) in self.shown(grid, count) {
            draw_contents(list, index, cell);
            frame_cell(list, cell, colors.dim);
        }
        if let Some(cell) = selected.and_then(|index| self.shown_bounds(grid, index, count)) {
            frame_cell(list, cell, colors.bright);
        }
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
    /// The description in its box, drawn as `TETextBox` draws it
    /// (`_OutfitDialogUpdate` @0x57f2b, `_ShipyardDialogUpdate` @0x597ac):
    /// static, a line straddling the box's bottom clipped to it.
    pub description: TextBox,
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

    /// The same picture with the description `text`, laid out in `area`
    /// in place of the one shown: the description loaded
    /// ([`FlightView::load_description`](crate::flight::FlightView::load_description))
    /// once the raw read has shown.
    pub fn retext(&mut self, text: &str, area: Bounds, metrics: &Rc<dyn TextMetrics>) {
        *self = Self::read_text(text, self.picture, area, metrics);
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
            description: TextBox::new(text, Font::Geneva, BODY_SIZE, BODY_COLOR, area, metrics),
        }
    }

    /// The picture stretched into `area`, or, with none, "No Picture" and
    /// "Available" centred across it in white, [`NO_PICTURE_OFFSET`] above
    /// and below the floor of its vertical middle (`_OutfitDialogUpdate`
    /// @0x579e0-0x57aa3, `_ShipyardDialogUpdate` @0x59164-0x59227); then
    /// the description's box filled black and the description in white
    /// over it, as `TETextBox` then `InvertRect` show it (@0x57f2b-0x57f39,
    /// @0x597ac-0x597ba).
    pub fn draw(&self, list: &mut DrawList, area: Bounds, metrics: &dyn TextMetrics) {
        if let Some(id) = self.picture {
            list.stretched_picture(ImageKey::picture(id), area.min, area.width(), area.height());
        } else {
            let middle = f32::midpoint(area.min.y, area.max.y).floor();
            for (words, baseline) in [
                (NO_PICTURE, middle - NO_PICTURE_OFFSET),
                (PICTURE_AVAILABLE, middle + NO_PICTURE_OFFSET),
            ] {
                draw_centred_text(list, metrics, area, baseline, words);
            }
        }
        fill_rect(list, self.description.rect(), Color::BLACK);
        self.description.draw(list);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    use crate::draw::DrawCommand;
    use crate::text::fixture::MonoMetrics;

    fn commands(list: &DrawList) -> Vec<DrawCommand> {
        list.iter().cloned().collect()
    }

    #[test]
    fn centred_text_takes_its_font_size_and_colour() {
        let mut list = DrawList::new();
        let style = TextStyle {
            font: Font::Times,
            size: 18.0,
            color: Color::WHITE,
        };
        let area = rect(7.0, 0.0, 600.0, 24.0);
        draw_centred_text_in(&mut list, &MonoMetrics, style, area, 18.0, "Kestrel");
        let width = MonoMetrics.width(Font::Times, 18.0, "Kestrel");
        assert_eq!(
            commands(&list),
            [DrawCommand::Text {
                text: "Kestrel".to_owned(),
                font: Font::Times,
                origin: Point::new(307.0 - width / 2.0, 0.0),
                size: 18.0,
                wrap_width: None,
                color: Color::WHITE,
            }]
        );
        // The middle is floored: 7..608 is 307.5, drawn from 307.
        let mut odd = DrawList::new();
        let grey = TextStyle {
            color: Color::DIM,
            ..style
        };
        draw_centred_text_in(
            &mut odd,
            &MonoMetrics,
            grey,
            rect(7.0, 10.0, 601.0, 24.0),
            40.0,
            "Kestrel",
        );
        let [DrawCommand::Text { origin, color, .. }] = &commands(&odd)[..] else {
            panic!("one text: {:?}", commands(&odd));
        };
        assert_eq!(
            (*origin, *color),
            (Point::new(307.0 - width / 2.0, 22.0), Color::DIM)
        );
        assert_eq!(
            SCREEN_TEXT,
            TextStyle {
                font: TEXT_FONT,
                size: TEXT_SIZE,
                color: TEXT_COLOR
            }
        );
    }

    fn texts(list: &DrawList) -> Vec<(String, Point, Color)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    font,
                    size,
                    ..
                } => {
                    assert_eq!((*font, *size), (TEXT_FONT, TEXT_SIZE), "{text}");
                    Some((text.clone(), *origin, *color))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_cells_picture_is_stretched_into_its_icon() {
        let cell = rect(10.0, 20.0, 83.0, 67.0);
        let mut list = DrawList::new();
        draw_cell_picture(&mut list, cell, Some(6000));
        assert_eq!(
            commands(&list),
            [DrawCommand::StretchedPicture {
                image: ImageKey::picture(6000),
                top_left: icon_bounds(cell).min,
                width: 32.0,
                height: 32.0,
            }]
        );
    }

    #[test]
    fn a_missing_picture_is_a_black_square_in_the_icon() {
        let cell = rect(10.0, 20.0, 83.0, 67.0);
        let mut list = DrawList::new();
        draw_cell_picture(&mut list, cell, None);
        let mut expected = DrawList::new();
        fill_rect(&mut expected, icon_bounds(cell), Color::BLACK);
        assert_eq!(commands(&list), commands(&expected));
    }

    #[test]
    fn two_name_lines_are_centred_14_and_3_above_the_bottom() {
        let cell = rect(10.0, 20.0, 83.0, 67.0);
        let mut list = DrawList::new();
        draw_cell_name(&mut list, &MonoMetrics, cell, "Light\\nBlaster");
        let centre = cell.center().x;
        assert_eq!(
            texts(&list),
            [
                (
                    "Light".to_owned(),
                    Point::new(centre - 12.5, 87.0 - 14.0 - 10.0),
                    TEXT_COLOR
                ),
                (
                    "Blaster".to_owned(),
                    Point::new(centre - 17.5, 87.0 - 3.0 - 10.0),
                    TEXT_COLOR
                ),
            ]
        );
    }

    #[test]
    fn each_name_line_keeps_its_colour() {
        let cell = rect(0.0, 0.0, 83.0, 67.0);
        let mut list = DrawList::new();
        draw_cell_name(&mut list, &MonoMetrics, cell, "*Ammo\\nPack");
        let colors: Vec<Color> = texts(&list).into_iter().map(|(_, _, c)| c).collect();
        assert_eq!(colors, [GREY, TEXT_COLOR]);
    }

    #[test]
    fn a_lone_name_line_is_6_above_the_bottom() {
        let cell = rect(10.0, 20.0, 83.0, 67.0);
        let mut list = DrawList::new();
        draw_cell_name(&mut list, &MonoMetrics, cell, "Map");
        assert_eq!(
            texts(&list),
            [(
                "Map".to_owned(),
                Point::new(cell.center().x - 7.5, 87.0 - 6.0 - 10.0),
                TEXT_COLOR
            )]
        );
    }

    #[test]
    fn more_name_lines_step_up_11_from_3_above_the_bottom() {
        let cell = rect(10.0, 20.0, 83.0, 67.0);
        let mut list = DrawList::new();
        draw_cell_name(&mut list, &MonoMetrics, cell, "a\\nb\\nc");
        let tops: Vec<f32> = texts(&list).into_iter().map(|(_, at, _)| at.y).collect();
        assert_eq!(
            tops,
            [87.0 - 25.0 - 10.0, 87.0 - 14.0 - 10.0, 87.0 - 3.0 - 10.0]
        );
    }

    #[test]
    fn corner_text_sits_3_inside_a_side_on_a_baseline_12_below_the_top() {
        let cell = rect(10.0, 20.0, 83.0, 67.0);
        let mut list = DrawList::new();
        draw_corner_text(&mut list, &MonoMetrics, cell, "12", true);
        draw_corner_text(&mut list, &MonoMetrics, cell, "(current)", false);
        assert_eq!(
            texts(&list),
            [
                (
                    "12".to_owned(),
                    Point::new(93.0 - 3.0 - 10.0, 20.0 + 12.0 - 10.0),
                    TEXT_COLOR
                ),
                (
                    "(current)".to_owned(),
                    Point::new(10.0 + 3.0, 20.0 + 12.0 - 10.0),
                    TEXT_COLOR
                ),
            ]
        );
    }

    #[test]
    fn the_description_is_white_on_black_and_clips_its_straddling_line() {
        // Geneva 10, 12 a line, in a box 30 high: the third line's top, 24,
        // is inside it and its bottom, 36, past it; the fourth's top is
        // below it.
        let picture_area = rect(100.0, 40.0, 200.0, 200.0);
        let boxed = rect(320.0, 40.0, 100.0, 30.0);
        let metrics: Rc<dyn TextMetrics> = Rc::new(MonoMetrics);
        let shown = Shown::read_text("l0\rl1\rl2\rl3", Some(5000), boxed, &metrics);
        let mut list = DrawList::new();
        shown.draw(&mut list, picture_area, &MonoMetrics);
        let line = |text: &str, row: f32| DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin: Point::new(320.0, 40.0 + 12.0 * row),
            size: BODY_SIZE,
            wrap_width: None,
            color: Color::WHITE,
        };
        let mut black = DrawList::new();
        fill_rect(&mut black, boxed, Color::BLACK);
        let mut expected = vec![DrawCommand::StretchedPicture {
            image: ImageKey::picture(5000),
            top_left: picture_area.min,
            width: 200.0,
            height: 200.0,
        }];
        expected.extend(commands(&black));
        expected.extend([
            line("l0", 0.0),
            line("l1", 1.0),
            DrawCommand::Clipped {
                clip: boxed,
                command: Box::new(line("l2", 2.0)),
            },
        ]);
        assert_eq!(commands(&list), expected);
    }

    fn shown(picture: Option<i16>, area: Bounds) -> Shown {
        let metrics: Rc<dyn TextMetrics> = Rc::new(MonoMetrics);
        Shown::read_text("", picture, area, &metrics)
    }

    #[test]
    fn a_missing_picture_draws_no_picture_and_available_centred_6_either_side_of_the_middle() {
        let area = rect(100.0, 40.0, 200.0, 200.0);
        let mut list = DrawList::new();
        shown(None, area).draw(&mut list, area, &MonoMetrics);
        assert_eq!(
            texts(&list),
            [
                (
                    "No Picture".to_owned(),
                    Point::new(200.0 - 25.0, baseline_top(140.0 - 6.0)),
                    TEXT_COLOR
                ),
                (
                    "Available".to_owned(),
                    Point::new(200.0 - 22.5, baseline_top(140.0 + 6.0)),
                    TEXT_COLOR
                ),
            ]
        );
    }

    #[test]
    fn the_middle_is_halved_to_a_whole_number_and_the_width_exactly() {
        let area = rect(10.0, 20.0, 201.0, 101.0);
        let mut list = DrawList::new();
        shown(None, area).draw(&mut list, area, &MonoMetrics);
        assert_eq!(
            texts(&list),
            [
                (
                    "No Picture".to_owned(),
                    Point::new(110.0 - 25.0, baseline_top(70.0 - 6.0)),
                    TEXT_COLOR
                ),
                (
                    "Available".to_owned(),
                    Point::new(110.0 - 22.5, baseline_top(70.0 + 6.0)),
                    TEXT_COLOR
                ),
            ]
        );
    }

    #[test]
    fn a_picture_draws_no_text() {
        let area = rect(10.0, 20.0, 201.0, 101.0);
        let mut list = DrawList::new();
        shown(Some(6000), area).draw(&mut list, area, &MonoMetrics);
        let mut expected = vec![DrawCommand::StretchedPicture {
            image: ImageKey::picture(6000),
            top_left: area.min,
            width: 201.0,
            height: 101.0,
        }];
        let mut black = DrawList::new();
        fill_rect(&mut black, area, Color::BLACK);
        expected.extend(commands(&black));
        assert_eq!(
            commands(&list),
            expected,
            "only the empty description's box"
        );
    }

    #[test]
    fn cells_are_84_by_55_frames_on_an_83_by_54_pitch_from_the_grids_corner() {
        let grid = rect(10.0, 20.0, 333.0, 271.0);
        assert_eq!(cell_bounds(grid, 0), rect(10.0, 20.0, 84.0, 55.0));
        assert_eq!(cell_bounds(grid, 1), rect(93.0, 20.0, 84.0, 55.0));
        assert_eq!(cell_bounds(grid, 4), rect(10.0, 74.0, 84.0, 55.0));
        assert_eq!(cell_bounds(grid, 5), rect(93.0, 74.0, 84.0, 55.0));
        let last = cell_bounds(grid, 19);
        assert_eq!(last, rect(10.0 + 249.0, 20.0 + 216.0, 84.0, 55.0));
        assert_eq!(
            last.max, grid.max,
            "the last frame ends on the stock item's corner"
        );
    }

    #[test]
    fn the_icons_top_is_from_the_cells_whole_centre() {
        // `_SetupListRects` halves top + bottom to a whole number: a 55-tall
        // cell's centre is 27 below its top, so the icon's top is 3 below it.
        assert_eq!(
            icon_bounds(rect(0.0, 0.0, 84.0, 55.0)),
            rect(26.0, 3.0, 32.0, 32.0)
        );
        assert_eq!(
            icon_bounds(rect(10.0, 20.0, 83.0, 67.0)),
            rect(35.0, 29.0, 32.0, 32.0),
            "both axes floored"
        );
    }

    #[test]
    fn the_selection_clamps_and_the_grid_follows_it() {
        let mut grid = CellGrid::default();
        assert_eq!(grid.selected(0), None);
        // 22 cells are 6 rows: the grid's last first row is 1.
        grid.select(30, 22);
        assert_eq!((grid.selected(22), grid.top_row()), (Some(21), 1));
        assert_eq!(grid.selected_raw(), 21);
        assert_eq!(grid.can_scroll(22), (true, false));
        grid.select(0, 22);
        assert_eq!((grid.selected(22), grid.top_row()), (Some(0), 0));
        assert_eq!(grid.can_scroll(22), (false, true));
        grid.select(19, 22);
        assert_eq!(grid.top_row(), 0, "the fifth row shows without scrolling");
        assert!(grid.arrow(Key::Down, 22, ArrowMoves::Filter));
        assert_eq!((grid.selected(22), grid.top_row()), (Some(21), 1));
        grid.select(0, 22);
        assert!(grid.arrow(Key::Down, 22, ArrowMoves::Filter));
        assert_eq!(grid.selected(22), Some(4));
        assert!(!grid.arrow(Key::Enter, 22, ArrowMoves::Filter));
        assert_eq!(grid.selected(22), Some(4));
        grid.scroll(true, 22);
        grid.scroll(true, 22);
        assert_eq!(grid.top_row(), 1, "never past the end");
        assert_eq!(grid.can_scroll(20), (true, false));
        assert_eq!(CellGrid::default().can_scroll(20), (false, false));
        assert_eq!(CellGrid::default().can_scroll(21), (false, true));
    }

    #[test]
    fn the_cells_shown_are_the_rows_from_the_top_one() {
        let area = rect(0.0, 0.0, 333.0, 271.0);
        let top = CellGrid::default().shown(area, 22);
        assert_eq!(top.len(), 20, "4 x 5 cells");
        assert_eq!(top[19], (19, cell_bounds(area, 19)));
        let mut grid = CellGrid::default();
        grid.scroll(true, 22);
        let shown = grid.shown(area, 22);
        assert_eq!(shown.len(), 18);
        assert_eq!(shown[0], (4, cell_bounds(area, 0)));
        assert_eq!(shown[17], (21, cell_bounds(area, 17)));
        assert_eq!(grid.shown(area, 3).len(), 0, "nothing past the end");
        assert_eq!(CellGrid::default().shown(area, 3).len(), 3);
        assert_eq!(
            grid.cell_at(area, Point::new(1.0, 1.0), 22),
            Some(4),
            "under the scroll"
        );
    }

    #[test]
    fn a_listed_cell_has_bounds_only_while_shown() {
        let area = rect(10.0, 20.0, 333.0, 271.0);
        let mut grid = CellGrid::default();
        assert_eq!(grid.shown_bounds(area, 0, 22), Some(cell_bounds(area, 0)));
        assert_eq!(grid.shown_bounds(area, 19, 22), Some(cell_bounds(area, 19)));
        assert_eq!(grid.shown_bounds(area, 20, 22), None, "below the fifth row");
        assert_eq!(grid.shown_bounds(area, 5, 5), None, "not listed");
        grid.scroll(true, 22);
        assert_eq!(grid.shown_bounds(area, 3, 22), None, "scrolled off the top");
        assert_eq!(grid.shown_bounds(area, 4, 22), Some(cell_bounds(area, 0)));
        assert_eq!(grid.shown_bounds(area, 21, 22), Some(cell_bounds(area, 17)));
        assert_eq!(grid.shown_bounds(area, 22, 22), None);
    }

    #[test]
    fn a_press_is_divided_by_84_across_and_55_down_as_the_filters_divide_it() {
        let area = rect(10.0, 20.0, 333.0, 271.0);
        let grid = CellGrid::default();
        let across = |dx: f32| grid.cell_under(area, Point::new(10.0 + dx, 21.0));
        assert_eq!(across(0.0), Some(0), "the left edge is in");
        assert_eq!(across(83.0), Some(0), "not on the 83 pitch");
        assert_eq!(across(84.0), Some(1));
        assert_eq!(across(332.0), Some(3));
        assert_eq!(across(333.0), None, "the right edge is out");
        let down = |dy: f32| grid.cell_under(area, Point::new(11.0, 20.0 + dy));
        assert_eq!(down(0.0), Some(0), "the top edge is in");
        assert_eq!(down(54.0), Some(0), "not on the 54 pitch");
        assert_eq!(down(55.0), Some(4));
        assert_eq!(down(270.0), Some(16));
        assert_eq!(down(271.0), None, "the bottom edge is out");
        let mut scrolled = CellGrid::default();
        scrolled.scroll(true, 22);
        assert_eq!(
            scrolled.cell_under(area, Point::new(10.0 + 84.0, 20.0 + 55.0)),
            Some(4 + 5),
            "a row down from the scroll"
        );
    }

    /// What [`CellGrid::draw_cells`] draws, with each cell's contents a
    /// one-colour text naming its index.
    fn drawn_cells(
        grid: &CellGrid,
        area: Bounds,
        count: usize,
        selected: Option<usize>,
    ) -> DrawList {
        let mut list = DrawList::new();
        grid.draw_cells(
            &mut list,
            area,
            count,
            selected,
            TEST_GRID_COLORS,
            |list, index, cell| {
                text(list, &index.to_string(), cell.min, TEXT_COLOR);
            },
        );
        list
    }

    #[test]
    fn each_shown_cell_is_framed_dim_after_its_contents_and_the_selected_one_bright_last() {
        let area = rect(10.0, 20.0, 333.0, 271.0);
        let mut grid = CellGrid::default();
        grid.scroll(true, 22);
        let mut expected = DrawList::new();
        for (index, cell) in grid.shown(area, 22) {
            text(&mut expected, &index.to_string(), cell.min, TEXT_COLOR);
            frame_rect(&mut expected, cell, TEST_GRID_COLORS.dim);
        }
        let selected = grid.shown_bounds(area, 9, 22).expect("shown");
        frame_rect(&mut expected, selected, TEST_GRID_COLORS.bright);
        assert_eq!(
            commands(&drawn_cells(&grid, area, 22, Some(9))),
            commands(&expected),
            "rows 1 to 5, each framed after its contents, then cell 9 bright"
        );
    }

    #[test]
    fn a_selection_not_shown_or_none_is_not_framed_bright() {
        let area = rect(10.0, 20.0, 333.0, 271.0);
        let mut grid = CellGrid::default();
        grid.scroll(true, 22);
        let mut unselected = DrawList::new();
        for (index, cell) in grid.shown(area, 22) {
            text(&mut unselected, &index.to_string(), cell.min, TEXT_COLOR);
            frame_rect(&mut unselected, cell, TEST_GRID_COLORS.dim);
        }
        let unselected = commands(&unselected);
        assert_eq!(commands(&drawn_cells(&grid, area, 22, None)), unselected);
        assert_eq!(
            commands(&drawn_cells(&grid, area, 22, Some(2))),
            unselected,
            "scrolled off the top"
        );
        assert_eq!(
            commands(&drawn_cells(&CellGrid::default(), area, 0, Some(0))),
            [],
            "nothing listed"
        );
    }

    #[test]
    fn a_press_past_the_four_by_five_cells_of_a_bigger_item_is_under_none() {
        // A plug-in grid item bigger than 4 x 84 by 5 x 55: the press is in
        // the item, but past the cells the filters divide it into.
        let area = rect(10.0, 20.0, 400.0, 330.0);
        let grid = CellGrid::default();
        let across = Point::new(10.0 + 335.0, 21.0);
        assert_eq!(grid.cell_under(area, across), Some(3), "the last column");
        let past_across = Point::new(10.0 + 336.0, 21.0);
        assert_eq!(grid.cell_under(area, past_across), None, "column 4");
        assert_eq!(grid.cell_at(area, past_across, 40), None);
        let down = Point::new(11.0, 20.0 + 274.0);
        assert_eq!(grid.cell_under(area, down), Some(16), "the last row");
        let past_down = Point::new(11.0, 20.0 + 275.0);
        assert_eq!(grid.cell_under(area, past_down), None, "row 5");
        assert_eq!(grid.cell_at(area, past_down, 40), None);
    }

    #[test]
    fn a_point_on_an_unlisted_cell_is_under_a_cell_but_at_none() {
        let area = rect(10.0, 20.0, 333.0, 271.0);
        let grid = CellGrid::default();
        let listed = Point::new(100.0, 30.0);
        assert_eq!(grid.cell_under(area, listed), Some(1));
        assert_eq!(grid.cell_at(area, listed, 6), Some(1));
        let empty = Point::new(100.0, 140.0);
        assert_eq!(grid.cell_under(area, empty), Some(9), "an empty cell");
        assert_eq!(grid.cell_at(area, empty, 6), None);
        let next = Point::new(190.0, 90.0);
        assert_eq!(grid.cell_under(area, next), Some(6));
        assert_eq!(grid.cell_at(area, next, 7), Some(6), "the last listed");
        assert_eq!(grid.cell_at(area, next, 6), None, "the first empty");
        for outside in [Point::new(9.5, 30.0), Point::new(20.0, 19.5)] {
            assert_eq!(grid.cell_under(area, outside), None, "outside");
            assert_eq!(grid.cell_at(area, outside, 22), None);
        }

        let mut scrolled = CellGrid::default();
        scrolled.scroll(true, 22);
        assert_eq!(scrolled.cell_under(area, listed), Some(5), "scrolled");
        assert_eq!(scrolled.cell_at(area, listed, 22), Some(5));
        let empty = Point::new(10.0 + 84.0 * 3.5, 20.0 + 55.0 * 4.5);
        assert_eq!(scrolled.cell_under(area, empty), Some(23));
        assert_eq!(scrolled.cell_at(area, empty, 22), None);
    }

    #[test]
    fn a_cleared_grid_selects_nothing_until_a_cell_is_selected() {
        let mut grid = CellGrid::default();
        assert_eq!(grid.selected(22), Some(0), "the default selects cell 0");
        grid.select(5, 22);
        grid.clear();
        assert_eq!(grid.selected(22), None);
        assert!(!grid.highlighted());
        grid.select(6, 22);
        assert_eq!(grid.selected(22), Some(6));
        assert!(grid.highlighted());
    }

    #[test]
    fn a_scroll_says_whether_it_moved() {
        let mut grid = CellGrid::default();
        assert!(!grid.scroll(false, 30), "at the top");
        assert!(grid.scroll(true, 30));
        assert!(grid.scroll(true, 30));
        assert!(grid.scroll(true, 30));
        assert_eq!(grid.top_row(), 3, "30 listed: the last first row is 3");
        assert!(!grid.scroll(true, 30), "at the bottom");
        assert!(grid.scroll(false, 30));
        assert!(!CellGrid::default().scroll(true, 20), "nothing to scroll");
    }

    #[test]
    fn an_arrow_from_no_highlight_picks_the_first_or_the_last_cell_shown() {
        let cleared = |top_rows: usize, count: usize| {
            let mut grid = CellGrid::default();
            for _ in 0..top_rows {
                grid.scroll(true, count);
            }
            grid.clear();
            grid
        };
        for (key, top_rows, count, expected) in [
            (Key::Right, 0, 30, 0),
            (Key::Down, 0, 30, 0),
            (Key::Left, 0, 30, 19),
            (Key::Up, 0, 30, 19),
            (Key::Right, 1, 30, 4),
            (Key::Down, 1, 30, 4),
            (Key::Left, 1, 30, 23),
            (Key::Up, 1, 30, 23),
            (Key::Left, 0, 7, 6),
            (Key::Up, 0, 7, 6),
            (Key::Left, 1, 22, 21),
            (Key::Right, 0, 7, 0),
            (Key::PageDown, 1, 30, 4),
            (Key::PageUp, 1, 30, 23),
        ] {
            let mut grid = cleared(top_rows, count);
            assert!(grid.arrow(key, count, ArrowMoves::Filter), "{key:?}");
            assert!(grid.highlighted(), "{key:?} highlights a cell");
            assert_eq!(
                grid.selected(count),
                Some(expected),
                "{key:?} from top row {top_rows} of {count}"
            );
            assert_eq!(grid.top_row(), top_rows, "{key:?} does not scroll");
        }
        let mut grid = cleared(0, 30);
        assert!(!grid.arrow(Key::Enter, 30, ArrowMoves::Filter));
        assert_eq!(grid.selected(30), None, "not an arrow");
    }

    #[test]
    fn an_arrow_moves_the_selection_by_the_reading_given() {
        // Ten listed: Down from 6 stays and Up from 2 stays by the filter,
        // and go to the ends by the clamped reading.
        for (moves, down, up) in [(ArrowMoves::Filter, 6, 2), (ArrowMoves::Clamped, 9, 0)] {
            let mut grid = CellGrid::default();
            grid.select(6, 10);
            assert!(grid.arrow(Key::Down, 10, moves));
            assert_eq!(grid.selected(10), Some(down), "{moves:?} Down");
            grid.select(2, 10);
            assert!(grid.arrow(Key::Up, 10, moves));
            assert_eq!(grid.selected(10), Some(up), "{moves:?} Up");
        }
        let mut grid = CellGrid::default();
        assert!(grid.arrow(Key::Right, 30, ArrowMoves::Filter));
        assert_eq!(grid.selected(30), Some(1));
        assert!(grid.arrow(Key::PageDown, 30, ArrowMoves::Filter));
        assert_eq!(grid.selected(30), Some(5));
        // Index 19 to the next row scrolls the grid with it.
        grid.select(19, 30);
        assert!(grid.arrow(Key::Right, 30, ArrowMoves::Filter));
        assert_eq!((grid.selected(30), grid.top_row()), (Some(20), 1));
    }

    /// A grid of `count` cells scrolled to `top_row`, with `index`
    /// selected, or none.
    fn scrolled_to(top_row: usize, index: Option<usize>, count: usize) -> CellGrid {
        let mut grid = CellGrid::default();
        for _ in 0..top_row {
            grid.scroll(true, count);
        }
        match index {
            Some(index) => {
                grid.select(index, count);
                assert_eq!(grid.top_row(), top_row, "index {index} is shown");
            }
            None => grid.clear(),
        }
        grid
    }

    #[test]
    fn a_scroll_dropping_hidden_keeps_a_cell_still_shown() {
        // Earth's 26: 7 rows, the last first row 2.
        for (top_row, index, down, after) in [
            (0, 9, true, 1),
            (1, 9, true, 2),
            (2, 9, false, 1),
            (1, 19, false, 0),
            (1, 8, true, 2),
        ] {
            let mut grid = scrolled_to(top_row, Some(index), 26);
            assert_eq!(
                grid.scroll_dropping_hidden(down, 26),
                Scrolled::Kept,
                "{index} from top row {top_row}, down {down}"
            );
            assert_eq!(
                (grid.selected(26), grid.top_row()),
                (Some(index), after),
                "{index} from top row {top_row}, down {down}"
            );
        }
    }

    #[test]
    fn a_scroll_dropping_hidden_drops_a_cell_scrolled_out_of_view() {
        for (top_row, index, down, after) in [(0, 3, true, 1), (1, 20, false, 0), (1, 4, true, 2)] {
            let mut grid = scrolled_to(top_row, Some(index), 26);
            assert_eq!(
                grid.scroll_dropping_hidden(down, 26),
                Scrolled::Dropped,
                "{index} from top row {top_row}, down {down}"
            );
            assert!(!grid.highlighted());
            assert_eq!((grid.selected(26), grid.top_row()), (None, after));
        }
    }

    #[test]
    fn a_scroll_dropping_hidden_from_no_highlight_drops_whenever_it_moves() {
        for (top_row, down, after) in [(0, true, 1), (2, false, 1)] {
            let mut grid = scrolled_to(top_row, None, 26);
            assert_eq!(
                grid.scroll_dropping_hidden(down, 26),
                Scrolled::Dropped,
                "from top row {top_row}, down {down}"
            );
            assert_eq!((grid.highlighted(), grid.top_row()), (false, after));
        }
    }

    #[test]
    fn a_scroll_dropping_hidden_that_cannot_move_stays() {
        for (top_row, index, down, count) in [
            (2, Some(9), true, 26),
            (0, Some(3), false, 26),
            (0, None, true, 20),
            (0, None, false, 26),
        ] {
            let mut grid = scrolled_to(top_row, index, count);
            let before = grid;
            assert_eq!(
                grid.scroll_dropping_hidden(down, count),
                Scrolled::Stayed,
                "{index:?} from top row {top_row} of {count}, down {down}"
            );
            assert_eq!(grid, before, "unchanged, the highlight included");
        }
    }

    #[test]
    fn a_cells_visible_position_is_its_index_less_the_scroll() {
        let mut grid = CellGrid::default();
        assert_eq!(grid.visible_position(9), 9);
        grid.scroll(true, 30);
        assert_eq!(grid.visible_position(9), 5);
        assert_eq!(grid.visible_position(4), 0);
    }
}
