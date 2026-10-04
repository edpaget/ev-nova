//! The Outfitter: the stellar's outfits for sale, laid out by the
//! interface file's "Outfit" dialog (`DLOG`/`DITL` 1002) over its
//! background picture.
//!
//! The stock dialog is 765 x 321, centred, and drawn over `PICT` 8502,
//! whose painted frames match its items: the grid of outfits (item 5) on
//! the left, the selected outfit's description (item 6) beside it, its
//! picture (item 8) and the info box (item 9) on the right, and along the
//! bottom the grid's scroll arrows (items 10 and 11), Buy (7), Sell (4) and
//! Done (1). The stock template disables Sell, as the original enables it
//! as it greys it, so the screen enables Done, Buy, Sell and the arrows
//! itself.
//!
//! The grid shows each outfit listed ([`Outfitter`]) in a cell,
//! [`COLUMNS`] across and [`GRID_ROWS`] down: its `ShortName`, split into
//! lines on a literal `\n`, each line white when it starts with a letter
//! or digit and grey otherwise (the Bible), and how many the player has.
//! One cell is selected, by a click on it or with the arrow keys, and
//! highlighted; the grid scrolls a row at a time with the arrows below it,
//! or with the selection. With nothing listed, the grid says so (`STR#`
//! 2002 #206).
//!
//! The selected outfit's picture is `PICT` 6000 plus its ID less 128, or
//! "No Picture" (#213) when the data has none, and its description is
//! `dësc` 3000 plus its ID less 128, both read through the
//! [`OutfitterCatalog`] as the selection changes. The info box shows its price, its mass, how many the player
//! has and the free mass (#215-218), and why it cannot be bought (#219-222)
//! or sold (#207), when the original has words for it.
//!
//! Buy and Sell, clicked or with B and S (key repeats too, so holding a
//! key keeps going, but only on the outfit the key went down on: when an
//! order removes its cell, a held key stops until it is pressed again),
//! ask for one of the selected outfit, and each is
//! greyed when one could not be bought or sold. The screen only records
//! the order ([`OutfitterScreen::take_order`]): whoever holds the session
//! makes it and gives back the outfitter as it now is
//! ([`OutfitterScreen::set_outfitter`]).
//!
//! Done, Return and Escape close it. Without the dialog, the screen says
//! why, and Return or Escape still closes it.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::{Direction, OutfitId, OutfitOrder, OutfitRefusal, Outfitter};

use super::catalog::SpaceportCatalog;
use super::layout::DONE_LABEL;
use super::trade::{BUY_LABEL, SELL_LABEL};
use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::catalog::DescriptionSource;
use crate::ui::desc::{BODY_COLOR, BODY_SIZE};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, Role, outline};
use crate::ui::scroll_text::ScrollText;

/// The "Outfit" dialog's `DLOG` (and `DITL`) ID.
pub const OUTFIT_DIALOG: i16 = 1002;

/// The picture the dialog is drawn over: "Outfit", in Nova Graphics.
pub const BACKGROUND: ImageKey = ImageKey::picture(8502);

/// Done's item.
pub const DONE_ITEM: usize = 1;
/// Sell's item.
pub const SELL_ITEM: usize = 4;
/// The grid's item.
pub const GRID_ITEM: usize = 5;
/// The description's item.
pub const DESCRIPTION_ITEM: usize = 6;
/// Buy's item.
pub const BUY_ITEM: usize = 7;
/// The picture's item.
pub const PICTURE_ITEM: usize = 8;
/// The info box's item.
pub const INFO_ITEM: usize = 9;
/// The grid's scroll-up arrow's item.
pub const SCROLL_UP_ITEM: usize = 10;
/// The grid's scroll-down arrow's item.
pub const SCROLL_DOWN_ITEM: usize = 11;

/// How many cells across the grid shows.
pub const COLUMNS: usize = 4;
/// How many rows of cells the grid shows at once.
pub const GRID_ROWS: usize = 4;
/// A cell's width: four fit the stock grid's 333.
pub const CELL_WIDTH: f32 = 83.0;
/// A cell's height: four fit the stock grid's 271.
pub const CELL_HEIGHT: f32 = 67.0;

/// Buys one, as Buy does: the original's default.
pub const BUY_KEY: Key = Key::Char('b');
/// Sells one, as Sell does: the original's default.
pub const SELL_KEY: Key = Key::Char('s');

/// The first outfit's `PICT`; outfit n's is n - 128 after it.
pub const FIRST_PICTURE: i16 = 6000;
/// The first outfit's `dësc`; outfit n's is n - 128 after it.
pub const FIRST_DESCRIPTION: i16 = 3000;
/// The first `oütf` ID.
const FIRST_OUTFIT: i16 = 128;

/// `STR#` 2002 #206.
pub const NO_ITEMS: &str = "There are no items available for purchase here.";
/// `STR#` 2002 #207.
pub const NEGATIVE_FREE_MASS: &str =
    "Can't sell that item, because your ship would have negative free mass afterwards.";
/// `STR#` 2002 #213.
pub const NO_PICTURE: &str = "No Picture";
/// `STR#` 2002 #215.
pub const PRICE_LABEL: &str = "Item Price:";
/// `STR#` 2002 #216.
pub const MASS_LABEL: &str = "Item Mass:";
/// `STR#` 2002 #217.
pub const OWNED_LABEL: &str = "You Have:";
/// `STR#` 2002 #218.
pub const AVAILABLE_LABEL: &str = "Available:";
/// `STR#` 2002 #219.
pub const MAX_OWNED: &str = "Can't have any more!";
/// `STR#` 2002 #220.
pub const NONE_ALLOWED: &str = "Can't have any of this item!";
/// `STR#` 2002 #221.
pub const NO_SPACE: &str = "Can't hold any more!";
/// `STR#` 2002 #222.
pub const NO_SPACE_FOR_ANY: &str = "Can't hold any of this item!";

/// The screen's text font.
pub const TEXT_FONT: Font = Font::Geneva;
/// The screen's text size.
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

/// The original's words for why one cannot be bought or sold, if it has
/// any.
#[must_use]
pub fn refusal_text(refusal: OutfitRefusal) -> Option<&'static str> {
    match refusal {
        OutfitRefusal::MaxOwned => Some(MAX_OWNED),
        OutfitRefusal::NoneAllowed => Some(NONE_ALLOWED),
        OutfitRefusal::NoSpace => Some(NO_SPACE),
        OutfitRefusal::NoSpaceForAny => Some(NO_SPACE_FOR_ANY),
        OutfitRefusal::NegativeFreeMass => Some(NEGATIVE_FREE_MASS),
        _ => None,
    }
}

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

/// What the outfitter reads about an outfit: its description, through
/// [`DescriptionSource`], and whether its picture exists, through
/// [`SpaceportCatalog::picture_exists`]. Anything that is both is one.
pub trait OutfitterCatalog: SpaceportCatalog + DescriptionSource {}

impl<T: SpaceportCatalog + DescriptionSource + ?Sized> OutfitterCatalog for T {}

/// The catalog, shared, with a `Debug` that shows nothing of it.
#[derive(Clone)]
struct CatalogHandle(Rc<dyn OutfitterCatalog>);

impl std::fmt::Debug for CatalogHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OutfitterCatalog")
    }
}

/// The metrics, shared, with a `Debug` that shows nothing of them.
#[derive(Clone)]
struct MetricsHandle(Rc<dyn TextMetrics>);

impl std::fmt::Debug for MetricsHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextMetrics")
    }
}

/// The selected outfit's picture and description, as read.
#[derive(Clone, Debug)]
struct Shown {
    outfit: OutfitId,
    /// Its `PICT`, when the data has it.
    picture: Option<i16>,
    description: ScrollText,
}

/// The outfitter, laid out.
#[derive(Clone, Debug)]
struct Laid {
    dialog: Dialog,
    metrics: MetricsHandle,
    shown: Option<Shown>,
}

/// The Outfitter of the stellar landed on.
#[derive(Clone, Debug)]
pub struct OutfitterScreen {
    /// The dialog, or why it cannot be shown.
    laid: Result<Laid, String>,
    outfitter: Outfitter,
    catalog: CatalogHandle,
    /// The selected cell, by index into the outfitter's rows.
    selected: usize,
    /// The grid's first row shown.
    top_row: usize,
    /// The order asked for, until it is taken.
    order: Option<OutfitOrder>,
    /// The key B or S last went down on (a press, not a repeat), and the
    /// outfit selected then, if any: its repeats act on that outfit
    /// alone.
    held: Option<(Key, Option<OutfitId>)>,
    closed: bool,
    sounds: Vec<Sound>,
}

impl OutfitterScreen {
    /// The outfitter `outfitter`, laid out by `layout`: the "Outfit"
    /// dialog's template and the metrics its text is measured by, or why
    /// there are none. Its buttons are labelled in `style`, and each
    /// outfit's picture and description are read from `catalog`. The
    /// first cell is selected.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        outfitter: Outfitter,
        catalog: Rc<dyn OutfitterCatalog>,
        style: ButtonStyle,
    ) -> Self {
        let laid = layout.map(|(mut template, metrics)| {
            for item in [
                DONE_ITEM,
                SELL_ITEM,
                BUY_ITEM,
                SCROLL_UP_ITEM,
                SCROLL_DOWN_ITEM,
            ] {
                if let Some(found) = template.items.get_mut(item - 1) {
                    found.enabled = true;
                }
            }
            let roles = [
                (DONE_ITEM, Role::Button(DONE_LABEL.to_owned())),
                (BUY_ITEM, Role::Button(BUY_LABEL.to_owned())),
                (SELL_ITEM, Role::Button(SELL_LABEL.to_owned())),
            ];
            let dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(DONE_ITEM))
                .with_cancel(Some(DONE_ITEM));
            Laid {
                dialog,
                metrics: MetricsHandle(metrics),
                shown: None,
            }
        });
        let mut screen = Self {
            laid,
            outfitter,
            catalog: CatalogHandle(catalog),
            selected: 0,
            top_row: 0,
            order: None,
            held: None,
            closed: false,
            sounds: Vec::new(),
        };
        screen.select(0);
        screen
    }

    /// The outfitter shown.
    #[must_use]
    pub fn outfitter(&self) -> &Outfitter {
        &self.outfitter
    }

    /// The selected cell's index, if anything is listed.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        (self.selected < self.outfitter.rows.len()).then_some(self.selected)
    }

    /// The grid's first row shown.
    #[must_use]
    pub fn top_row(&self) -> usize {
        self.top_row
    }

    /// The dialog, if it could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.laid.as_ref().ok().map(|laid| &laid.dialog)
    }

    /// Why the outfitter cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// Whether Done has been activated.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The order asked for since it was last taken, once.
    pub fn take_order(&mut self) -> Option<OutfitOrder> {
        self.order.take()
    }

    /// Where cell `index` (counted from the grid's first row shown) is,
    /// if the grid could be laid out.
    #[must_use]
    pub fn cell_bounds(&self, index: usize) -> Option<Bounds> {
        let grid = self.dialog()?.item_bounds(GRID_ITEM)?;
        let (row, column) = (index / COLUMNS, index % COLUMNS);
        let min = Point::new(
            grid.min.x + CELL_WIDTH * column as f32,
            grid.min.y + CELL_HEIGHT * row as f32,
        );
        Some(Bounds::at(min, CELL_WIDTH, CELL_HEIGHT))
    }

    /// Shows `outfitter`, the outfitter after an order, keeping the same
    /// outfit selected when it is still listed, or the cell where it was.
    pub fn set_outfitter(&mut self, outfitter: Outfitter) {
        let kept = self
            .selected()
            .map(|index| self.outfitter.rows[index].id)
            .and_then(|id| outfitter.rows.iter().position(|row| row.id == id));
        self.outfitter = outfitter;
        self.select(kept.unwrap_or(self.selected));
    }

    /// How many rows of cells the outfits fill.
    fn rows_filled(&self) -> usize {
        self.outfitter.rows.len().div_ceil(COLUMNS)
    }

    /// The grid's last first row: never scrolled past its end.
    fn last_top_row(&self) -> usize {
        self.rows_filled().saturating_sub(GRID_ROWS)
    }

    /// Selects cell `index`, or the last when there is no such cell,
    /// scrolls the grid to show it, and reads its picture and description
    /// if they are not shown already.
    fn select(&mut self, index: usize) {
        let count = self.outfitter.rows.len();
        self.selected = index.min(count.saturating_sub(1));
        let row = self.selected / COLUMNS;
        self.top_row = self
            .top_row
            .min(self.last_top_row())
            .min(row)
            .max((row + 1).saturating_sub(GRID_ROWS));
        self.regrey();
        self.show();
    }

    /// Reads the selected outfit's picture and description, unless they
    /// are shown already; with nothing selected, shows none.
    fn show(&mut self) {
        let outfit = self.selected().map(|index| self.outfitter.rows[index].id);
        let catalog = Rc::clone(&self.catalog.0);
        let Ok(laid) = &mut self.laid else {
            return;
        };
        let Some(outfit) = outfit else {
            laid.shown = None;
            return;
        };
        if laid
            .shown
            .as_ref()
            .is_some_and(|shown| shown.outfit == outfit)
        {
            return;
        }
        let offset = outfit.0.saturating_sub(FIRST_OUTFIT);
        let picture = FIRST_PICTURE.saturating_add(offset);
        let text = catalog
            .description(FIRST_DESCRIPTION.saturating_add(offset))
            .unwrap_or_default();
        let area = laid
            .dialog
            .item_bounds(DESCRIPTION_ITEM)
            .unwrap_or(Bounds::at(Point::new(0.0, 0.0), 0.0, 0.0));
        laid.shown = Some(Shown {
            outfit,
            picture: catalog.picture_exists(picture).then_some(picture),
            description: ScrollText::new(
                &text,
                Font::Geneva,
                BODY_SIZE,
                BODY_COLOR,
                area,
                &laid.metrics.0,
            ),
        });
    }

    /// Greys Buy and Sell when one of the selected outfit could not be
    /// bought or sold.
    fn regrey(&mut self) {
        let buys = self.allows(Direction::Buy);
        let sells = self.allows(Direction::Sell);
        if let Ok(laid) = &mut self.laid {
            laid.dialog.set_greyed(BUY_ITEM, !buys);
            laid.dialog.set_greyed(SELL_ITEM, !sells);
        }
    }

    /// Whether one of the selected outfit can go `direction`.
    fn allows(&self, direction: Direction) -> bool {
        self.selected().is_some_and(|index| {
            let row = &self.outfitter.rows[index];
            match direction {
                Direction::Buy => row.buy.is_ok(),
                Direction::Sell => row.sell.is_ok(),
            }
        })
    }

    /// Asks to buy or sell one of the selected outfit, if it can go that
    /// way.
    fn ask(&mut self, direction: Direction) {
        let Some(index) = self.selected().filter(|_| self.allows(direction)) else {
            return;
        };
        self.order = Some(OutfitOrder {
            outfit: self.outfitter.rows[index].id,
            direction,
        });
    }

    /// Asks to go `direction` for `key` going down: a press asks for the
    /// selected outfit and remembers it; a repeat asks only while that
    /// same outfit is still selected and the press was this key's. So a
    /// key held while an order removes the outfit's cell never acts on
    /// the outfit that takes the cell.
    fn ask_by_key(&mut self, key: Key, input: &Input, direction: Direction) {
        let selected = self.selected().map(|index| self.outfitter.rows[index].id);
        let repeat = matches!(*input, Input::Key { repeat: true, .. });
        if !repeat {
            self.held = Some((key, selected));
        } else if self.held != Some((key, selected)) {
            return;
        }
        self.ask(direction);
    }

    /// Scrolls the grid a row down, or up, never past either end.
    fn scroll(&mut self, down: bool) {
        self.top_row = if down {
            (self.top_row + 1).min(self.last_top_row())
        } else {
            self.top_row.saturating_sub(1)
        };
    }

    /// Activates dialog item `item`: Done closes, Buy and Sell ask, and the
    /// arrows scroll. Any other item does nothing.
    fn activate(&mut self, item: usize) {
        match item {
            DONE_ITEM => self.closed = true,
            BUY_ITEM => self.ask(Direction::Buy),
            SELL_ITEM => self.ask(Direction::Sell),
            SCROLL_UP_ITEM => self.scroll(false),
            SCROLL_DOWN_ITEM => self.scroll(true),
            _ => {}
        }
    }

    /// The cell index under `at` on the grid, if a listed outfit is there.
    fn cell_at(&self, at: Point) -> Option<usize> {
        let grid = self.dialog()?.item_bounds(GRID_ITEM)?;
        if !grid.contains(at) {
            return None;
        }
        let column = ((at.x - grid.min.x) / CELL_WIDTH) as usize;
        let row = ((at.y - grid.min.y) / CELL_HEIGHT) as usize;
        if column >= COLUMNS || row >= GRID_ROWS {
            return None;
        }
        let index = (self.top_row + row) * COLUMNS + column;
        (index < self.outfitter.rows.len()).then_some(index)
    }

    /// `text` at `origin`, in the screen's font, in `color`.
    fn text(list: &mut DrawList, text: &str, origin: Point, color: Color) {
        list.text_in(TEXT_FONT, text, origin, TEXT_SIZE, None, color);
    }

    fn draw_grid(&self, laid: &Laid, list: &mut DrawList) {
        let line_height = laid.metrics.0.line_height(TEXT_FONT, TEXT_SIZE);
        if self.outfitter.rows.is_empty() {
            if let Some(grid) = laid.dialog.item_bounds(GRID_ITEM) {
                let origin = Point::new(grid.min.x + INSET, grid.min.y + INSET);
                Self::text(list, NO_ITEMS, origin, TEXT_COLOR);
            }
            return;
        }
        let first = self.top_row * COLUMNS;
        let shown = self
            .outfitter
            .rows
            .iter()
            .enumerate()
            .skip(first)
            .take(COLUMNS * GRID_ROWS);
        for (index, row) in shown {
            let Some(cell) = self.cell_bounds(index - first) else {
                continue;
            };
            if Some(index) == self.selected() {
                fill_rect(list, cell, SELECTED_COLOR);
            }
            outline(list, cell, GREY);
            let mut y = cell.min.y + INSET;
            for (line, color) in name_lines(&row.short_name) {
                Self::text(list, line, Point::new(cell.min.x + INSET, y), color);
                y += line_height;
            }
            if row.owned > 0 {
                let origin = Point::new(cell.min.x + INSET, cell.max.y - INSET - line_height);
                Self::text(list, &row.owned.to_string(), origin, TEXT_COLOR);
            }
        }
    }

    fn draw_selected(&self, laid: &Laid, list: &mut DrawList) {
        let Some(index) = self.selected() else {
            return;
        };
        let row = &self.outfitter.rows[index];
        if let (Some(shown), Some(area)) = (&laid.shown, laid.dialog.item_bounds(PICTURE_ITEM)) {
            if let Some(id) = shown.picture {
                list.stretched_picture(
                    ImageKey::picture(id),
                    area.min,
                    area.width(),
                    area.height(),
                );
            } else {
                let origin = Point::new(area.min.x + INSET, area.min.y + INSET);
                Self::text(list, NO_PICTURE, origin, GREY);
            }
            shown.description.draw(list);
        }
        let Some(info) = laid.dialog.item_bounds(INFO_ITEM) else {
            return;
        };
        let line_height = laid.metrics.0.line_height(TEXT_FONT, TEXT_SIZE);
        let mut lines = vec![
            format!("{PRICE_LABEL} {}", row.price),
            format!("{MASS_LABEL} {} tons", row.mass),
            format!("{OWNED_LABEL} {}", row.owned),
            format!("{AVAILABLE_LABEL} {} tons", self.outfitter.free_mass),
        ];
        let refusals = [row.buy.err(), row.sell.err()];
        lines.extend(
            refusals
                .into_iter()
                .flatten()
                .filter_map(refusal_text)
                .map(str::to_owned),
        );
        for (n, line) in lines.iter().enumerate() {
            let origin = Point::new(info.min.x + INSET, info.min.y + line_height * n as f32);
            Self::text(list, line, origin, TEXT_COLOR);
        }
    }

    /// The scroll arrows: a triangle in each item, white when the grid can
    /// scroll that way, grey otherwise.
    fn draw_arrows(&self, laid: &Laid, list: &mut DrawList) {
        for (item, up, can) in [
            (SCROLL_UP_ITEM, true, self.top_row > 0),
            (SCROLL_DOWN_ITEM, false, self.top_row < self.last_top_row()),
        ] {
            let Some(area) = laid.dialog.item_bounds(item) else {
                continue;
            };
            let color = if can { TEXT_COLOR } else { GREY };
            let (left, right) = (area.min.x + INSET, area.max.x - INSET);
            let (tip, base) = if up {
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
}

impl Screen for OutfitterScreen {
    /// Every input goes to the outfitter; it never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if self.laid.is_err() {
            if let Input::Key {
                key: Key::Enter | Key::Escape,
                pressed: true,
                repeat: false,
            } = *input
            {
                self.closed = true;
            }
            return ScreenAction::None;
        }
        let pressed =
            |wanted: Key| matches!(*input, Input::Key { key, pressed: true, .. } if key == wanted);
        if pressed(Key::Left) {
            self.select(self.selected.saturating_sub(1));
        } else if pressed(Key::Right) {
            self.select(self.selected + 1);
        } else if pressed(Key::Up) {
            self.select(self.selected.saturating_sub(COLUMNS));
        } else if pressed(Key::Down) {
            self.select(self.selected + COLUMNS);
        } else if pressed(BUY_KEY) {
            self.ask_by_key(BUY_KEY, input, Direction::Buy);
        } else if pressed(SELL_KEY) {
            self.ask_by_key(SELL_KEY, input, Direction::Sell);
        } else if let Some(index) = match *input {
            Input::PointerButton {
                pressed: true, at, ..
            } => self.cell_at(at),
            _ => None,
        } {
            self.select(index);
        } else if let Ok(laid) = &mut self.laid {
            let event = laid.dialog.input(input);
            self.sounds.extend(laid.dialog.take_sound().map(Sound::Ui));
            if let Some(DialogEvent::Item(item)) = event {
                self.activate(item);
            }
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        let laid = match &self.laid {
            Ok(laid) => laid,
            Err(reason) => {
                list.text(
                    format!(
                        "Cannot show the outfitter: {reason}. Press Return or Escape to close it."
                    ),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
                return;
            }
        };
        list.picture(BACKGROUND, laid.dialog.bounds().min);
        self.draw_grid(laid, list);
        self.draw_selected(laid, list);
        self.draw_arrows(laid, list);
        laid.dialog.draw(list);
    }

    fn cancel_pointer(&mut self) {
        if let Ok(laid) = &mut self.laid {
            laid.dialog.cancel_pointer();
        }
    }

    /// The buttons' sounds, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use nova_sim::OutfitRow;

    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::sound::UiSound;
    use crate::spaceport::catalog::{PortRecord, StellarId};
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock "Outfit": `DLOG` 1002, 765 x 321 and centred, and its thirteen
    /// user items, where 2, 3, 4, 6, 8, 9, 12 and 13 are disabled and 2,
    /// 12 and 13 parked outside it.
    fn template() -> DialogTemplate {
        let items = [
            (500.0, 289.0, 99.0, 25.0, true),
            (248.0, 440.0, 68.0, 30.0, false),
            (251.0, 291.0, 29.0, 21.0, false),
            (394.0, 289.0, 99.0, 25.0, false),
            (9.0, 8.0, 333.0, 271.0, true),
            (354.0, 10.0, 192.0, 267.0, false),
            (288.0, 289.0, 99.0, 25.0, true),
            (557.0, 8.0, 200.0, 200.0, false),
            (618.0, 214.0, 135.0, 100.0, false),
            (148.0, 288.0, 25.0, 25.0, true),
            (178.0, 288.0, 25.0, 25.0, true),
            (170.0, 449.0, 32.0, 32.0, false),
            (356.0, 422.0, 68.0, 30.0, false),
        ];
        DialogTemplate {
            bounds: rect(100.0, 100.0, 765.0, 321.0),
            placement: Placement::Center,
            items: items
                .into_iter()
                .map(|(x, y, w, h, enabled)| ItemTemplate {
                    bounds: rect(x, y, w, h),
                    enabled,
                    kind: ItemSpec::User,
                })
                .collect(),
        }
    }

    /// Where the dialog goes: (1024 - 765) / 2 and (768 - 321) / 2,
    /// floored.
    const ORIGIN: Point = Point::new(129.0, 223.0);

    /// Outfit 128's picture exists; every description is "dësc n.", and
    /// 3002's is missing. Records what it is asked.
    struct FakeArt {
        pictures: Vec<i16>,
        asked: RefCell<Vec<String>>,
    }

    impl SpaceportCatalog for FakeArt {
        fn stellar_port(&self, id: StellarId) -> Result<PortRecord, String> {
            panic!("asked for spöb {}", id.0)
        }

        fn picture_exists(&self, id: i16) -> bool {
            self.asked.borrow_mut().push(format!("PICT {id}"));
            self.pictures.contains(&id)
        }
    }

    impl DescriptionSource for FakeArt {
        fn description(&self, id: i16) -> Result<String, String> {
            self.asked.borrow_mut().push(format!("dësc {id}"));
            if id == 3002 {
                return Err("no dësc 3002".to_owned());
            }
            Ok(format!("dësc {id}."))
        }

        fn button_style(&self) -> ButtonStyle {
            panic!("the style is given")
        }
    }

    fn art() -> Rc<FakeArt> {
        Rc::new(FakeArt {
            pictures: vec![6000],
            asked: RefCell::default(),
        })
    }

    fn row(id: i16, short_name: &str, price: i64, mass: i64, owned: u16) -> OutfitRow {
        OutfitRow {
            id: OutfitId(id),
            name: format!("Outfit {id}"),
            short_name: short_name.to_owned(),
            price,
            mass,
            owned,
            max: 10,
            buy: Ok(()),
            sell: if owned > 0 {
                Ok(())
            } else {
                Err(OutfitRefusal::NoneOwned)
            },
        }
    }

    /// A light blaster (2 owned), ammunition, a gun too heavy to hold, and
    /// a map that can only be sold here (1 owned), for a player with 5000
    /// credits and 5 tons free.
    fn outfitter() -> Outfitter {
        Outfitter {
            rows: vec![
                row(128, "Light\\nBlaster", 1000, 1, 2),
                row(129, "*Ammo\\nPack", 50, 0, 0),
                OutfitRow {
                    buy: Err(OutfitRefusal::NoSpaceForAny),
                    ..row(130, "Big Gun", 9000, 10, 0)
                },
                OutfitRow {
                    buy: Err(OutfitRefusal::NotForSale),
                    ..row(131, "Map", 0, 0, 1)
                },
            ],
            cash: 5000,
            free_mass: 5,
        }
    }

    fn layout() -> (DialogTemplate, Rc<dyn TextMetrics>) {
        (template(), Rc::new(MonoMetrics))
    }

    fn screen_with(outfitter: Outfitter, art: &Rc<FakeArt>) -> OutfitterScreen {
        let catalog: Rc<dyn OutfitterCatalog> = Rc::clone(art) as Rc<dyn OutfitterCatalog>;
        OutfitterScreen::new(Ok(layout()), outfitter, catalog, ButtonStyle::STOCK)
    }

    fn screen_of(outfitter: Outfitter) -> OutfitterScreen {
        screen_with(outfitter, &art())
    }

    fn screen() -> OutfitterScreen {
        screen_of(outfitter())
    }

    fn drawn(screen: &OutfitterScreen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(commands: &[DrawCommand]) -> Vec<(String, Point, Color)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    ..
                } => Some((text.clone(), *origin, *color)),
                _ => None,
            })
            .collect()
    }

    fn words(screen: &OutfitterScreen) -> Vec<String> {
        texts(&drawn(screen))
            .into_iter()
            .map(|(text, _, _)| text)
            .collect()
    }

    fn item(screen: &OutfitterScreen, number: usize) -> Bounds {
        screen
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
    }

    fn key(key: Key, pressed: bool, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat,
        }
    }

    fn press(screen: &mut OutfitterScreen, k: Key) {
        screen.input(&key(k, true, false));
    }

    fn click(screen: &mut OutfitterScreen, at: Point) {
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn click_item(screen: &mut OutfitterScreen, number: usize) {
        let at = item(screen, number).center();
        click(screen, at);
    }

    fn click_cell(screen: &mut OutfitterScreen, index: usize) {
        let at = screen.cell_bounds(index).expect("a cell").center();
        click(screen, at);
    }

    fn order(id: i16, direction: Direction) -> OutfitOrder {
        OutfitOrder {
            outfit: OutfitId(id),
            direction,
        }
    }

    /// Whether Buy and Sell draw enabled.
    fn enabled(screen: &OutfitterScreen) -> (bool, bool) {
        let label_color = |label: &str| {
            texts(&drawn(screen))
                .into_iter()
                .find_map(|(text, _, color)| (text == label).then_some(color))
        };
        let grey = ButtonStyle::STOCK.grey;
        (
            label_color(BUY_LABEL) != Some(grey),
            label_color(SELL_LABEL) != Some(grey),
        )
    }

    #[test]
    fn the_named_values() {
        assert_eq!(OUTFIT_DIALOG, 1002);
        assert_eq!(BACKGROUND, ImageKey::picture(8502));
        assert_eq!(
            [
                DONE_ITEM,
                SELL_ITEM,
                GRID_ITEM,
                DESCRIPTION_ITEM,
                BUY_ITEM,
                PICTURE_ITEM,
                INFO_ITEM,
                SCROLL_UP_ITEM,
                SCROLL_DOWN_ITEM
            ],
            [1, 4, 5, 6, 7, 8, 9, 10, 11]
        );
        assert_eq!((COLUMNS, GRID_ROWS), (4, 4));
        assert!(CELL_WIDTH * COLUMNS as f32 <= 333.0);
        assert!(CELL_HEIGHT * GRID_ROWS as f32 <= 271.0);
        assert_eq!((BUY_KEY, SELL_KEY), (Key::Char('b'), Key::Char('s')));
        assert_eq!((FIRST_PICTURE, FIRST_DESCRIPTION), (6000, 3000));
        assert_eq!(
            [
                PRICE_LABEL,
                MASS_LABEL,
                OWNED_LABEL,
                AVAILABLE_LABEL,
                NO_PICTURE
            ],
            [
                "Item Price:",
                "Item Mass:",
                "You Have:",
                "Available:",
                "No Picture"
            ]
        );
    }

    #[test]
    fn each_refusal_the_original_words_has_its_words() {
        assert_eq!(
            refusal_text(OutfitRefusal::MaxOwned),
            Some("Can't have any more!")
        );
        assert_eq!(
            refusal_text(OutfitRefusal::NoneAllowed),
            Some("Can't have any of this item!")
        );
        assert_eq!(
            refusal_text(OutfitRefusal::NoSpace),
            Some("Can't hold any more!")
        );
        assert_eq!(
            refusal_text(OutfitRefusal::NoSpaceForAny),
            Some("Can't hold any of this item!")
        );
        assert_eq!(
            refusal_text(OutfitRefusal::NegativeFreeMass),
            Some(
                "Can't sell that item, because your ship would have negative free mass afterwards."
            )
        );
        for silent in [
            OutfitRefusal::NoOutfitter,
            OutfitRefusal::NotListed,
            OutfitRefusal::NotForSale,
            OutfitRefusal::NoExpansion,
            OutfitRefusal::CannotAfford,
            OutfitRefusal::NoneOwned,
            OutfitRefusal::CannotSell,
            OutfitRefusal::NotBoughtHere,
        ] {
            assert_eq!(refusal_text(silent), None, "{silent:?}");
        }
    }

    #[test]
    fn a_short_name_splits_on_backslash_n_and_greys_lines_not_starting_with_a_letter_or_digit() {
        assert_eq!(
            name_lines("Real Big\\nScary Gun"),
            [("Real Big", TEXT_COLOR), ("Scary Gun", TEXT_COLOR)]
        );
        assert_eq!(
            name_lines("*Ammo\\n9mm\\n(x10)"),
            [("*Ammo", GREY), ("9mm", TEXT_COLOR), ("(x10)", GREY)]
        );
        assert_eq!(name_lines(""), [("", GREY)]);
        assert_eq!(name_lines("Élan"), [("Élan", TEXT_COLOR)]);
    }

    #[test]
    fn it_is_centred_with_the_first_cell_selected() {
        let screen = screen();
        assert_eq!(screen.problem(), None);
        assert_eq!(screen.dialog().expect("laid out").bounds().min, ORIGIN);
        assert_eq!(screen.selected(), Some(0));
        assert_eq!(screen.top_row(), 0);
        assert!(!screen.closed());
        assert_eq!(screen.outfitter(), &outfitter());
        assert_eq!(item(&screen, GRID_ITEM), rect(138.0, 231.0, 333.0, 271.0));
        assert_eq!(
            screen.cell_bounds(5),
            Some(rect(138.0 + 83.0, 231.0 + 67.0, 83.0, 67.0))
        );
    }

    #[test]
    fn the_stock_templates_disabled_sell_and_arrows_work() {
        let mut template = template();
        for number in [SCROLL_UP_ITEM, SCROLL_DOWN_ITEM, DONE_ITEM, BUY_ITEM] {
            template.items[number - 1].enabled = false;
        }
        let mut screen = OutfitterScreen::new(
            Ok((template, Rc::new(MonoMetrics))),
            outfitter(),
            art(),
            ButtonStyle::STOCK,
        );
        click_item(&mut screen, SELL_ITEM);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
        click_item(&mut screen, DONE_ITEM);
        assert!(screen.closed());
        let short = DialogTemplate {
            items: template_items(3),
            ..super::tests::template()
        };
        let screen = OutfitterScreen::new(
            Ok((short, Rc::new(MonoMetrics))),
            outfitter(),
            art(),
            ButtonStyle::STOCK,
        );
        assert_eq!(screen.cell_bounds(0), None, "no grid");
        assert!(drawn(&screen).len() > 1);
    }

    /// The stock template's first `count` items.
    fn template_items(count: usize) -> Vec<ItemTemplate> {
        template().items.into_iter().take(count).collect()
    }

    #[test]
    fn it_draws_the_background_the_grid_the_selection_the_arrows_then_the_dialog() {
        let screen = screen();
        let commands = drawn(&screen);
        assert_eq!(
            commands[0],
            DrawCommand::Picture {
                image: BACKGROUND,
                top_left: ORIGIN,
            }
        );
        let cell = |index: usize, dy: f32| {
            let bounds = screen.cell_bounds(index).expect("a cell");
            Point::new(bounds.min.x + INSET, bounds.min.y + dy)
        };
        let bottom = |index: usize| {
            let bounds = screen.cell_bounds(index).expect("a cell");
            Point::new(bounds.min.x + INSET, bounds.max.y - INSET - 12.0)
        };
        let info = item(&screen, INFO_ITEM);
        let line = |n: f32| Point::new(info.min.x + INSET, info.min.y + 12.0 * n);
        let description = item(&screen, DESCRIPTION_ITEM).min;
        let mut expected = vec![
            ("Light".to_owned(), cell(0, INSET), TEXT_COLOR),
            ("Blaster".to_owned(), cell(0, INSET + 12.0), TEXT_COLOR),
            ("2".to_owned(), bottom(0), TEXT_COLOR),
            ("*Ammo".to_owned(), cell(1, INSET), GREY),
            ("Pack".to_owned(), cell(1, INSET + 12.0), TEXT_COLOR),
            ("Big Gun".to_owned(), cell(2, INSET), TEXT_COLOR),
            ("Map".to_owned(), cell(3, INSET), TEXT_COLOR),
            ("1".to_owned(), bottom(3), TEXT_COLOR),
            ("dësc 3000.".to_owned(), description, Color::WHITE),
            ("Item Price: 1000".to_owned(), line(0.0), TEXT_COLOR),
            ("Item Mass: 1 tons".to_owned(), line(1.0), TEXT_COLOR),
            ("You Have: 2".to_owned(), line(2.0), TEXT_COLOR),
            ("Available: 5 tons".to_owned(), line(3.0), TEXT_COLOR),
        ];
        let mut dialog = DrawList::new();
        screen.dialog().expect("laid out").draw(&mut dialog);
        let buttons: Vec<DrawCommand> = dialog.iter().cloned().collect();
        expected.extend(texts(&buttons));
        assert_eq!(texts(&commands), expected);
        assert_eq!(commands[commands.len() - buttons.len()..], buttons[..]);
        let labels: Vec<_> = texts(&buttons).into_iter().map(|(t, _, _)| t).collect();
        assert_eq!(labels, ["Done", "Sell", "Buy"]);
        let picture = item(&screen, PICTURE_ITEM);
        assert!(commands.contains(&DrawCommand::StretchedPicture {
            image: ImageKey::picture(6000),
            top_left: picture.min,
            width: 200.0,
            height: 200.0,
        }));
    }

    #[test]
    fn the_text_is_geneva_10() {
        let commands = drawn(&screen());
        for command in &commands {
            if let DrawCommand::Text {
                text,
                font,
                size,
                wrap_width,
                ..
            } = command
                && !["Done", "Sell", "Buy"].contains(&text.as_str())
            {
                assert_eq!(
                    (*font, *size, *wrap_width),
                    (Font::Geneva, 10.0, None),
                    "{text}"
                );
            }
        }
    }

    /// The selected cell's highlight: the cell filled.
    fn highlight(screen: &OutfitterScreen, index: usize) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        fill_rect(
            &mut list,
            screen.cell_bounds(index).expect("a cell"),
            SELECTED_COLOR,
        );
        list.iter().cloned().collect()
    }

    /// A cell's frame.
    fn frame(screen: &OutfitterScreen, index: usize) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        outline(&mut list, screen.cell_bounds(index).expect("a cell"), GREY);
        list.iter().cloned().collect()
    }

    fn contains(commands: &[DrawCommand], part: &[DrawCommand]) -> bool {
        commands.windows(part.len()).any(|window| window == part)
    }

    #[test]
    fn the_selected_cell_is_highlighted_and_every_listed_cell_framed() {
        let mut screen = screen();
        let commands = drawn(&screen);
        let first = highlight(&screen, 0);
        let at = commands
            .windows(first.len())
            .position(|window| window == first)
            .expect("highlighted");
        let light = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { text, .. } if text == "Light"))
            .expect("drawn");
        assert!(at < light, "under the text");
        for index in 0..4 {
            assert!(contains(&commands, &frame(&screen, index)), "{index}");
        }
        assert!(!contains(&commands, &frame(&screen, 4)), "nothing there");
        press(&mut screen, Key::Right);
        let commands = drawn(&screen);
        assert!(!contains(&commands, &first));
        assert!(contains(&commands, &highlight(&screen, 1)));
    }

    #[test]
    fn the_arrow_keys_move_the_selection_and_stop_at_the_ends() {
        let mut screen = screen_of(long(10));
        press(&mut screen, Key::Left);
        assert_eq!(screen.selected(), Some(0));
        press(&mut screen, Key::Up);
        assert_eq!(screen.selected(), Some(0));
        press(&mut screen, Key::Right);
        assert_eq!(screen.selected(), Some(1));
        screen.input(&key(Key::Down, true, true));
        assert_eq!(screen.selected(), Some(5), "a row down, repeats too");
        press(&mut screen, Key::Down);
        assert_eq!(screen.selected(), Some(9), "the last");
        press(&mut screen, Key::Right);
        assert_eq!(screen.selected(), Some(9));
        screen.input(&key(Key::Up, false, false));
        assert_eq!(screen.selected(), Some(9), "not on release");
        press(&mut screen, Key::Up);
        assert_eq!(screen.selected(), Some(5));
        press(&mut screen, Key::Left);
        assert_eq!(screen.selected(), Some(4));
        assert_eq!(screen.take_sounds(), [], "keys are silent");
    }

    #[test]
    fn a_click_on_a_cell_selects_it_and_on_an_empty_one_nothing() {
        let mut screen = screen();
        click_cell(&mut screen, 2);
        assert_eq!(screen.selected(), Some(2));
        click_cell(&mut screen, 4);
        assert_eq!(screen.selected(), Some(2), "the first empty cell");
        let grid = item(&screen, GRID_ITEM);
        let mut full = screen_of(long(20));
        click(&mut full, Point::new(grid.max.x - 0.5, grid.min.y + 1.0));
        assert_eq!(full.selected(), Some(0), "past the last column");
        click(&mut full, Point::new(grid.min.x + 1.0, grid.max.y - 0.5));
        assert_eq!(full.selected(), Some(0), "past the last row");
        click(&mut screen, Point::new(grid.min.x + 1.0, grid.min.y + 1.0));
        assert_eq!(screen.selected(), Some(0));
        click(&mut screen, Point::new(grid.min.x - 1.0, grid.min.y + 1.0));
        assert_eq!(screen.selected(), Some(0), "outside the grid");
        assert_eq!(screen.take_sounds(), [], "cells are silent");
        assert_eq!(screen.take_order(), None);
    }

    /// `count` outfits, numbered from 200, each 10 credits and a ton.
    fn long(count: usize) -> Outfitter {
        Outfitter {
            rows: (0..count)
                .map(|n| {
                    let id = 200 + i16::try_from(n).expect("few");
                    row(id, &format!("Good {n}"), 10, 1, 0)
                })
                .collect(),
            ..outfitter()
        }
    }

    fn names(screen: &OutfitterScreen) -> Vec<String> {
        words(screen)
            .into_iter()
            .filter(|text| text.starts_with("Good "))
            .collect()
    }

    #[test]
    fn the_grid_scrolls_a_row_at_a_time_with_the_selection_or_its_arrows() {
        let mut screen = screen_of(long(22));
        let first: Vec<_> = (0..16).map(|n| format!("Good {n}")).collect();
        assert_eq!(names(&screen), first);
        for _ in 0..3 {
            press(&mut screen, Key::Down);
        }
        assert_eq!((screen.selected(), screen.top_row()), (Some(12), 0));
        press(&mut screen, Key::Down);
        assert_eq!((screen.selected(), screen.top_row()), (Some(16), 1));
        assert_eq!(names(&screen)[0], "Good 4");
        assert!(contains(&drawn(&screen), &highlight(&screen, 12)));
        press(&mut screen, Key::Down);
        assert_eq!((screen.selected(), screen.top_row()), (Some(20), 2));
        assert_eq!(names(&screen).last().map(String::as_str), Some("Good 21"));
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 2, "the last row shows");
        click_item(&mut screen, SCROLL_UP_ITEM);
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!(screen.top_row(), 0);
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!(screen.top_row(), 0, "the first row shows");
        assert_eq!(screen.selected(), Some(20), "scrolling keeps the selection");
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 1);
        click_cell(&mut screen, 0);
        assert_eq!((screen.selected(), screen.top_row()), (Some(4), 1));
        press(&mut screen, Key::Up);
        assert_eq!((screen.selected(), screen.top_row()), (Some(0), 0));
    }

    /// The lines drawn from inside the scroll arrows' items.
    fn arrow_lines(screen: &OutfitterScreen) -> Vec<(Point, Point, Color)> {
        let up = item(screen, SCROLL_UP_ITEM);
        let down = item(screen, SCROLL_DOWN_ITEM);
        drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Line {
                    from, to, color, ..
                } => Some((from, to, color)),
                _ => None,
            })
            .filter(|(from, _, _)| up.contains(*from) || down.contains(*from))
            .collect()
    }

    /// The arrows' colours, up then down.
    fn arrows(screen: &OutfitterScreen) -> Vec<Color> {
        arrow_lines(screen)
            .into_iter()
            .step_by(3)
            .map(|(_, _, color)| color)
            .collect()
    }

    #[test]
    fn the_arrows_are_white_only_when_the_grid_can_scroll_that_way() {
        let mut screen = screen_of(long(22));
        let up = item(&screen, SCROLL_UP_ITEM);
        let down = item(&screen, SCROLL_DOWN_ITEM);
        let commands = drawn(&screen);
        let lines: Vec<_> = commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Line {
                    from, to, color, ..
                } => Some((*from, *to, *color)),
                _ => None,
            })
            .filter(|(from, _, _)| up.contains(*from) || down.contains(*from))
            .collect();
        let apex_up = Point::new(up.center().x, up.min.y + INSET);
        let apex_down = Point::new(down.center().x, down.max.y - INSET);
        assert_eq!(
            lines,
            [
                (
                    Point::new(up.min.x + INSET, up.max.y - INSET),
                    Point::new(up.max.x - INSET, up.max.y - INSET),
                    GREY
                ),
                (
                    Point::new(up.max.x - INSET, up.max.y - INSET),
                    apex_up,
                    GREY
                ),
                (
                    apex_up,
                    Point::new(up.min.x + INSET, up.max.y - INSET),
                    GREY
                ),
                (
                    Point::new(down.min.x + INSET, down.min.y + INSET),
                    Point::new(down.max.x - INSET, down.min.y + INSET),
                    TEXT_COLOR
                ),
                (
                    Point::new(down.max.x - INSET, down.min.y + INSET),
                    apex_down,
                    TEXT_COLOR
                ),
                (
                    apex_down,
                    Point::new(down.min.x + INSET, down.min.y + INSET),
                    TEXT_COLOR
                ),
            ]
        );
        assert_eq!(arrows(&screen), [GREY, TEXT_COLOR]);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(arrows(&screen), [TEXT_COLOR, GREY]);
        assert_eq!(arrows(&screen_of(long(16))), [GREY, GREY]);
    }

    #[test]
    fn the_picture_and_description_are_read_as_the_selection_changes() {
        let art = art();
        let mut screen = screen_with(outfitter(), &art);
        assert_eq!(*art.asked.borrow(), ["dësc 3000", "PICT 6000"]);
        press(&mut screen, Key::Right);
        let shown = words(&screen);
        assert!(shown.contains(&NO_PICTURE.to_owned()), "{shown:?}");
        assert!(shown.contains(&"dësc 3001.".to_owned()));
        press(&mut screen, Key::Right);
        let shown = words(&screen);
        assert!(
            !shown.iter().any(|text| text.starts_with("dësc")),
            "{shown:?}"
        );
        press(&mut screen, Key::Right);
        press(&mut screen, Key::Right);
        screen.set_outfitter(outfitter());
        assert_eq!(
            *art.asked.borrow(),
            [
                "dësc 3000",
                "PICT 6000",
                "dësc 3001",
                "PICT 6001",
                "dësc 3002",
                "PICT 6002",
                "dësc 3003",
                "PICT 6003",
            ],
            "once each, and not again while it stays selected"
        );
        let picture = item(&screen, PICTURE_ITEM);
        let no_picture = texts(&drawn(&screen))
            .into_iter()
            .find(|(text, _, _)| text == NO_PICTURE)
            .expect("drawn");
        assert_eq!(
            (no_picture.1, no_picture.2),
            (
                Point::new(picture.min.x + INSET, picture.min.y + INSET),
                GREY
            )
        );
    }

    #[test]
    fn an_outfit_with_an_impossible_id_does_not_overflow() {
        let art = art();
        let odd = Outfitter {
            rows: vec![row(i16::MIN, "Odd", 1, 1, 0), row(i16::MAX, "Big", 1, 1, 0)],
            ..outfitter()
        };
        let mut screen = screen_with(odd, &art);
        press(&mut screen, Key::Right);
        assert_eq!(
            *art.asked.borrow(),
            ["dësc -29768", "PICT -26768", "dësc 32767", "PICT 32767"]
        );
    }

    #[test]
    fn the_info_box_says_why_buy_or_sell_is_greyed_when_the_original_words_it() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        press(&mut screen, Key::Right);
        let shown = words(&screen);
        assert!(shown.contains(&NO_SPACE_FOR_ANY.to_owned()), "{shown:?}");
        assert!(shown.contains(&"Item Price: 9000".to_owned()));
        assert!(shown.contains(&"Item Mass: 10 tons".to_owned()));
        press(&mut screen, Key::Right);
        let shown = words(&screen);
        assert!(
            !shown.iter().any(|text| text.starts_with("Can't")),
            "{shown:?}"
        );
        let overloaded = Outfitter {
            rows: vec![OutfitRow {
                buy: Err(OutfitRefusal::MaxOwned),
                sell: Err(OutfitRefusal::NegativeFreeMass),
                ..row(128, "Expansion", 100, -5, 1)
            }],
            ..outfitter()
        };
        let screen = screen_of(overloaded);
        let info = item(&screen, INFO_ITEM);
        let lines: Vec<_> = texts(&drawn(&screen))
            .into_iter()
            .filter(|(_, at, _)| info.contains(*at))
            .map(|(text, _, _)| text)
            .collect();
        assert_eq!(
            lines,
            [
                "Item Price: 100",
                "Item Mass: -5 tons",
                "You Have: 1",
                "Available: 5 tons",
                MAX_OWNED,
                NEGATIVE_FREE_MASS,
            ]
        );
    }

    #[test]
    fn with_nothing_listed_the_grid_says_so_and_both_are_greyed() {
        let art = art();
        let mut screen = screen_with(
            Outfitter {
                rows: Vec::new(),
                ..outfitter()
            },
            &art,
        );
        assert_eq!(screen.selected(), None);
        let grid = item(&screen, GRID_ITEM);
        let shown = texts(&drawn(&screen));
        assert!(shown.contains(&(
            NO_ITEMS.to_owned(),
            Point::new(grid.min.x + INSET, grid.min.y + INSET),
            TEXT_COLOR
        )));
        assert!(!shown.iter().any(|(text, _, _)| text.starts_with("Item")));
        assert_eq!(enabled(&screen), (false, false));
        press(&mut screen, BUY_KEY);
        press(&mut screen, SELL_KEY);
        press(&mut screen, Key::Down);
        assert_eq!(screen.take_order(), None);
        assert_eq!(*art.asked.borrow(), Vec::<String>::new(), "nothing to read");
        assert!(!words(&screen_of(outfitter())).contains(&NO_ITEMS.to_owned()));
    }

    #[test]
    fn buy_and_sell_ask_for_one_of_the_selected_outfit() {
        let mut screen = screen();
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
        assert_eq!(screen.take_order(), None, "once");
        click_item(&mut screen, SELL_ITEM);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        press(&mut screen, Key::Right);
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_order(), Some(order(129, Direction::Buy)));
        screen.input(&key(BUY_KEY, true, true));
        assert_eq!(
            screen.take_order(),
            Some(order(129, Direction::Buy)),
            "repeats too"
        );
        press(&mut screen, Key::Left);
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        screen.input(&key(SELL_KEY, true, true));
        assert_eq!(screen.take_order(), Some(order(128, Direction::Sell)));
        screen.input(&key(BUY_KEY, false, false));
        assert_eq!(screen.take_order(), None, "not on release");
    }

    #[test]
    fn buy_and_sell_are_greyed_when_one_could_not_go_that_way() {
        let mut screen = screen();
        assert_eq!(enabled(&screen), (true, true), "the blaster");
        press(&mut screen, Key::Right);
        assert_eq!(enabled(&screen), (true, false), "ammunition, none owned");
        press(&mut screen, Key::Right);
        assert_eq!(enabled(&screen), (false, false), "too heavy, none owned");
        click_item(&mut screen, BUY_ITEM);
        press(&mut screen, BUY_KEY);
        click_item(&mut screen, SELL_ITEM);
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), None, "greyed asks for nothing");
    }

    #[test]
    fn a_sell_only_row_can_be_selected_and_sold_but_not_bought() {
        let mut screen = screen();
        click_cell(&mut screen, 3);
        assert_eq!(screen.selected(), Some(3));
        assert_eq!(enabled(&screen), (false, true));
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_order(), None);
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), Some(order(131, Direction::Sell)));
    }

    #[test]
    fn a_held_key_never_acts_on_an_outfit_it_did_not_start_on() {
        // A sell-only map (1 owned) next to a sellable blaster (2 owned).
        let both = || Outfitter {
            rows: vec![
                OutfitRow {
                    buy: Err(OutfitRefusal::NotForSale),
                    ..row(131, "Map", 0, 0, 1)
                },
                row(128, "Light\\nBlaster", 1000, 1, 2),
            ],
            ..outfitter()
        };
        let mut screen = screen_of(both());
        screen.input(&key(SELL_KEY, true, false));
        assert_eq!(screen.take_order(), Some(order(131, Direction::Sell)));
        // Sold, the map's row is gone: the blaster takes its cell.
        let mut sold = both();
        sold.rows.remove(0);
        screen.set_outfitter(sold);
        assert_eq!(screen.selected(), Some(0));
        for _ in 0..3 {
            screen.input(&key(SELL_KEY, true, true));
            assert_eq!(screen.take_order(), None, "the key was held for the map");
        }
        screen.input(&key(SELL_KEY, false, false));
        screen.input(&key(SELL_KEY, true, false));
        assert_eq!(
            screen.take_order(),
            Some(order(128, Direction::Sell)),
            "a fresh press acts on the new selection"
        );
        screen.input(&key(SELL_KEY, true, true));
        assert_eq!(
            screen.take_order(),
            Some(order(128, Direction::Sell)),
            "and its repeats too, while the blaster stays selected"
        );
    }

    #[test]
    fn a_repeat_acts_only_for_the_key_pressed_on_the_outfit_selected() {
        let mut screen = screen();
        screen.input(&key(BUY_KEY, true, true));
        assert_eq!(screen.take_order(), None, "no press before it");
        screen.input(&key(BUY_KEY, true, false));
        assert_eq!(screen.take_order(), Some(order(128, Direction::Buy)));
        screen.input(&key(SELL_KEY, true, true));
        assert_eq!(screen.take_order(), None, "S was never pressed");
        press(&mut screen, Key::Right);
        screen.input(&key(BUY_KEY, true, true));
        assert_eq!(screen.take_order(), None, "pressed on another outfit");
        press(&mut screen, Key::Left);
        screen.input(&key(BUY_KEY, true, true));
        assert_eq!(
            screen.take_order(),
            Some(order(128, Direction::Buy)),
            "back on the outfit it was pressed on"
        );
    }

    #[test]
    fn a_new_outfitter_shows_and_regreys_keeping_the_outfit_selected() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        let mut bought = outfitter();
        bought.rows[1].owned = 3;
        bought.rows[1].sell = Ok(());
        bought.rows[1].buy = Err(OutfitRefusal::CannotAfford);
        bought.cash = 10;
        screen.set_outfitter(bought.clone());
        assert_eq!(screen.outfitter(), &bought);
        assert_eq!(screen.selected(), Some(1));
        assert_eq!(enabled(&screen), (false, true));
        assert!(words(&screen).contains(&"You Have: 3".to_owned()));
        // Sold out of a sell-only map: the next cell along is selected.
        click_cell(&mut screen, 3);
        let mut sold = outfitter();
        sold.rows.remove(3);
        sold.rows.insert(0, row(127, "First", 1, 1, 0));
        screen.set_outfitter(sold.clone());
        assert_eq!(screen.selected(), Some(3), "the map's place, now the gun");
        sold.rows.remove(3);
        screen.set_outfitter(sold);
        assert_eq!(screen.selected(), Some(2), "clamped");
        let mut kept = screen_of(long(10));
        press(&mut kept, Key::Right);
        let mut moved = long(10);
        moved.rows.rotate_right(1);
        kept.set_outfitter(moved);
        assert_eq!(kept.selected(), Some(2), "followed");
        kept.set_outfitter(long(0));
        assert_eq!(kept.selected(), None);
        kept.set_outfitter(long(5));
        assert_eq!(kept.selected(), Some(0));
    }

    #[test]
    fn done_return_and_escape_close_it() {
        let mut screen = screen();
        click_item(&mut screen, DONE_ITEM);
        assert!(screen.closed());
        for k in [Key::Enter, Key::Escape] {
            let mut screen = screen_of(outfitter());
            assert_eq!(screen.input(&key(k, true, false)), ScreenAction::None);
            assert!(screen.closed(), "{k:?}");
            assert_eq!(screen.take_order(), None);
        }
        let mut held = screen_of(outfitter());
        held.input(&key(Key::Escape, true, true));
        held.input(&key(Key::Space, true, false));
        held.tick(Duration::from_secs(1));
        assert!(!held.closed());
    }

    #[test]
    fn its_buttons_sound_as_they_are_clicked() {
        let mut screen = screen();
        let buy = item(&screen, BUY_ITEM).center();
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: buy,
        });
        assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::ButtonDown)]);
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: buy,
        });
        assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::ButtonUp)]);
        press(&mut screen, BUY_KEY);
        press(&mut screen, Key::Enter);
        assert_eq!(screen.take_sounds(), []);
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click() {
        let mut screen = screen();
        let buy = item(&screen, BUY_ITEM).center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: buy,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.take_order(), None);
        let debug = format!("{screen:?}");
        assert!(debug.contains("TextMetrics"), "{debug}");
        assert!(debug.contains("OutfitterCatalog"), "{debug}");
    }

    fn problem(reason: &str) -> DrawCommand {
        DrawCommand::Text {
            text: format!(
                "Cannot show the outfitter: {reason}. Press Return or Escape to close it."
            ),
            font: Font::Geneva,
            origin: PROBLEM_AT,
            size: PROBLEM_SIZE,
            wrap_width: None,
            color: Color::ERROR,
        }
    }

    #[test]
    fn without_the_dialog_it_says_why_and_return_or_escape_closes_it() {
        for k in [Key::Enter, Key::Escape] {
            let art = art();
            let mut screen = OutfitterScreen::new(
                Err("no DLOG 1002".to_owned()),
                outfitter(),
                Rc::clone(&art) as Rc<dyn OutfitterCatalog>,
                ButtonStyle::STOCK,
            );
            assert_eq!(screen.problem(), Some("no DLOG 1002"));
            assert!(screen.dialog().is_none());
            assert_eq!(screen.cell_bounds(0), None);
            assert_eq!(drawn(&screen), [problem("no DLOG 1002")]);
            for other in [
                key(k, true, true),
                key(k, false, false),
                key(BUY_KEY, true, false),
                key(Key::Down, true, false),
            ] {
                screen.input(&other);
            }
            click(&mut screen, Point::new(500.0, 500.0));
            screen.cancel_pointer();
            assert!(!screen.closed());
            assert_eq!(screen.take_order(), None);
            assert_eq!(*art.asked.borrow(), Vec::<String>::new());
            screen.input(&key(k, true, false));
            assert!(screen.closed(), "{k:?}");
        }
    }
}
