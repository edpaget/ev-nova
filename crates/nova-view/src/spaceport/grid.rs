//! The parts the Outfitter and the Shipyard share: the grid of cells in
//! their dialog's grid item, with its selection, scrolling and arrows; a
//! `ShortName`'s lines; and the selected item's picture, or "No Picture"
//! and "Available" centred on its item, and its description, as read.
//!
//! The grid shows [`COLUMNS`] cells across and [`GRID_ROWS`] down, each
//! [`CELL_WIDTH`] x [`CELL_HEIGHT`] from the grid item's top left. One cell
//! is selected; the grid scrolls a row at a time, never past either end,
//! and always shows the selected cell when the selection moves.
//!
//! A cell shows its item's picture, stretched to [`ICON_SIZE`] square,
//! centred across the cell near its top; its `ShortName` centred across
//! the bottom; and a note in a top corner: the original's placement
//! (`_SetupListRects` @0x51a2c, `_RetrieveMenuIcon` @0x4f425, and the cell
//! loops of `_OutfitDialogUpdate` @0x572cb and `_ShipyardDialogUpdate`
//! @0x58a10). A missing picture is a black square, as the original paints
//! it.

use std::rc::Rc;

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
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
/// How many rows of cells the grid shows at once.
pub const GRID_ROWS: usize = 4;
/// A cell's width: four fit the stock grid's 333.
pub const CELL_WIDTH: f32 = 83.0;
/// A cell's height: four fit the stock grid's 271.
pub const CELL_HEIGHT: f32 = 67.0;

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

/// A cell's picture's width and height: the original stretches the whole
/// `PICT` into a 32 x 32 slot (`_RetrieveMenuIcon` @0x4f425, its slots set
/// by `_SetupListRects` @0x51afe-0x51b36).
pub const ICON_SIZE: f32 = 32.0;
/// How far a cell's picture's top is above the cell's centre
/// (`_SetupListRects` @0x51a9e-0x51af0).
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
/// cell, its top [`ICON_RISE`] above the cell's centre.
#[must_use]
pub fn icon_bounds(cell: Bounds) -> Bounds {
    let c = cell.center();
    Bounds::at(
        Point::new(c.x - ICON_SIZE / 2.0, c.y - ICON_RISE),
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

    /// The index, counted with the scroll, of the cell position under
    /// `at` on a grid item at `grid`, listed or not: `None` outside the
    /// item or past its [`COLUMNS`] x [`GRID_ROWS`] cells, as the
    /// original bounds the column and row (`_OutfitFilter`
    /// @0x585ea-0x58607).
    #[must_use]
    pub fn cell_under(&self, grid: Bounds, at: Point) -> Option<usize> {
        if !grid.contains(at) {
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
    fn the_icon_is_32_square_centred_across_and_24_above_the_cells_centre() {
        assert_eq!(
            icon_bounds(rect(10.0, 20.0, 83.0, 67.0)),
            rect(35.5, 29.5, 32.0, 32.0)
        );
        assert_eq!(
            icon_bounds(rect(0.0, 0.0, 84.0, 55.0)),
            rect(26.0, 3.5, 32.0, 32.0),
            "3 below the top of the original's 55-tall cell"
        );
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

    #[test]
    fn a_point_on_an_unlisted_cell_is_under_a_cell_but_at_none() {
        let area = rect(10.0, 20.0, 333.0, 271.0);
        let grid = CellGrid::default();
        let listed = Point::new(100.0, 30.0);
        assert_eq!(grid.cell_under(area, listed), Some(1));
        assert_eq!(grid.cell_at(area, listed, 6), Some(1));
        let empty = Point::new(100.0, 160.0);
        assert_eq!(grid.cell_under(area, empty), Some(9), "an empty cell");
        assert_eq!(grid.cell_at(area, empty, 6), None);
        let next = Point::new(190.0, 100.0);
        assert_eq!(grid.cell_under(area, next), Some(6));
        assert_eq!(grid.cell_at(area, next, 7), Some(6), "the last listed");
        assert_eq!(grid.cell_at(area, next, 6), None, "the first empty");
        let last = Point::new(10.0 + 83.0 * 4.0 - 0.5, 20.0 + 67.0 * 4.0 - 0.5);
        assert_eq!(grid.cell_under(area, last), Some(15), "the last cell");
        for past in [
            Point::new(10.0 + 83.0 * 4.0 + 0.5, 30.0),
            Point::new(20.0, 20.0 + 67.0 * 4.0 + 0.5),
        ] {
            assert_eq!(grid.cell_under(area, past), None, "past the cells");
            assert_eq!(grid.cell_at(area, past, 22), None);
        }
        for outside in [Point::new(9.5, 30.0), Point::new(20.0, 19.5)] {
            assert_eq!(grid.cell_under(area, outside), None, "outside");
            assert_eq!(grid.cell_at(area, outside, 22), None);
        }

        let mut scrolled = CellGrid::default();
        scrolled.scroll(true, 22);
        scrolled.scroll(true, 22);
        assert_eq!(scrolled.cell_under(area, listed), Some(9), "scrolled");
        assert_eq!(scrolled.cell_at(area, listed, 22), Some(9));
        let empty = Point::new(10.0 + 83.0 * 3.5, 20.0 + 67.0 * 3.5);
        assert_eq!(scrolled.cell_under(area, empty), Some(23));
        assert_eq!(scrolled.cell_at(area, empty, 22), None);
    }
}
