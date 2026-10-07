//! The Shipyard: the stellar's ships for sale, laid out by the interface
//! file's "Shipyard" dialog (`DLOG`/`DITL` 1004) over its background
//! picture.
//!
//! The stock dialog is 765 x 323, centred, and drawn over `PICT` 8501,
//! whose painted frames match its items: the grid of ships (item 5) on the
//! left, the selected ship's description (item 6) beside it, its picture
//! (item 8) and the info box (item 9) on the right, and along the bottom
//! the grid's scroll arrows (items 12 and 13), Info (10), Done (1) and Buy
//! Ship (7). The roles are read from the "Outfit" dialog's numbering and
//! the labels; the screen enables Done, Buy Ship, Info and the arrows
//! itself.
//!
//! The grid shows each ship listed ([`Shipyard`]) in a cell, as the
//! Outfitter's does ([`grid`](super::grid)): its `ShortName`'s lines, and
//! "(current)" in the cell of the class the player flies. With nothing
//! listed, the grid says so (`STR#` 2002 #223).
//!
//! The selected ship's picture is [`ship_picture`]'s: `PICT` 5000 plus its
//! ID less 128, or else that of the lowest-numbered ship sharing its
//! `shän` base image, as the Bible says target pictures are shared, or
//! "No Picture". Its description is `dësc` 13000 plus its ID less 128. The
//! info box shows its price, what the player's ship trades in for, and
//! the final price (#225-227).
//!
//! Buy Ship, clicked or with B, requests the selected ship, and is greyed
//! when it cannot be bought (the original has no words for why). B acts
//! on a press only, never a key repeat, unlike the Outfitter's: a held B
//! would otherwise trade the ship just bought for another of its class,
//! and its outfits with it. The screen only records the request
//! ([`ShipyardScreen::take_request`]): whoever holds the session asks for
//! the new ship's name ([`ShipNaming`]) and opens the "Text Input" prompt
//! with it ([`ShipyardScreen::open_naming`], [`TextInputDialog`]), as the
//! original asks before it buys (`_DoShipyardDialog` @0x5eb6c). The
//! prompt takes every input while it is open; when B asked, the B typed
//! is dropped, as the original flushes it. OK records the order, the ship
//! and the name confirmed ([`ShipyardScreen::take_order`]): whoever holds
//! the session makes it and gives back the shipyard as it now is
//! ([`ShipyardScreen::set_shipyard`]). Cancel records the ship declined
//! ([`ShipyardScreen::take_declined`]), whose roll the session draws
//! again. Without the "Text Input" dialog, the prompt is laid out by
//! [`TextInputDialog::fallback`].
//!
//! Info, clicked or with I, opens the "Shipyard Info" panel (`DLOG`/`DITL`
//! 1005 over `PICT` 8506) over the shipyard: the ship's name in item 3,
//! and in item 5 its speed, acceleration, turn rate, guns, turrets, free
//! space, length, mass, crew and how many jumps its fuel holds
//! (#229-240). Its Done, Return and Escape close the panel alone. Without
//! that dialog, Info is greyed.
//!
//! Done, Return and Escape close the shipyard. Without its dialog, the
//! screen says why, and Return or Escape still closes it.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::hyperspace::max_jumps;
use nova_sim::shipyard::SHIP_NAME_CHARS;
use nova_sim::{ShipId, ShipNaming, ShipRow, ShipSpecs, Shipyard};

use super::catalog::SpaceportCatalog;
use super::grid::{
    self, CellGrid, GREY, INSET, SELECTED_COLOR, Shown, TEXT_COLOR, TEXT_FONT, TEXT_SIZE,
    name_lines, text,
};
use super::layout::DONE_LABEL;
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
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, Role, outline};
use crate::ui::text_input::{TextInputDialog, TextInputOutcome};

/// The "Shipyard" dialog's `DLOG` (and `DITL`) ID.
pub const SHIPYARD_DIALOG: i16 = 1004;
/// The "Shipyard Info" dialog's `DLOG` (and `DITL`) ID.
pub const SHIP_INFO_DIALOG: i16 = 1005;

/// The picture the shipyard is drawn over: "Shipyard", in Nova Graphics.
pub const BACKGROUND: ImageKey = ImageKey::picture(8501);
/// The picture the info panel is drawn over.
pub const INFO_BACKGROUND: ImageKey = ImageKey::picture(8506);

/// Done's item.
pub const DONE_ITEM: usize = 1;
/// The grid's item.
pub const GRID_ITEM: usize = 5;
/// The description's item.
pub const DESCRIPTION_ITEM: usize = 6;
/// Buy Ship's item.
pub const BUY_ITEM: usize = 7;
/// The picture's item.
pub const PICTURE_ITEM: usize = 8;
/// The info box's item.
pub const INFO_BOX_ITEM: usize = 9;
/// Info's item.
pub const INFO_ITEM: usize = 10;
/// The grid's scroll-up arrow's item.
pub const SCROLL_UP_ITEM: usize = 12;
/// The grid's scroll-down arrow's item.
pub const SCROLL_DOWN_ITEM: usize = 13;

/// The info panel's Done.
pub const PANEL_DONE_ITEM: usize = 1;
/// The info panel's title.
pub const PANEL_TITLE_ITEM: usize = 3;
/// The info panel's text.
pub const PANEL_TEXT_ITEM: usize = 5;

/// Buys the selected ship, as Buy Ship does: the original's default.
pub const BUY_KEY: Key = Key::Char('b');
/// Opens the info panel, as Info does.
pub const INFO_KEY: Key = Key::Char('i');

/// The first ship's `PICT`; ship n's is n - 128 after it.
pub const FIRST_SHIP_PICTURE: i16 = 5000;
/// The first `shïp` ID.
const FIRST_SHIP: i16 = 128;

/// `STR#` 150 #4.
pub const BUY_SHIP_LABEL: &str = "Buy Ship";
/// `STR#` 150 #48.
pub const INFO_LABEL: &str = "Info";
/// `STR#` 2002 #223.
pub const NO_SHIPS: &str = "There are no ships available for purchase here.";
/// `STR#` 2002 #225.
pub const PRICE_LABEL: &str = "Ship Price:";
/// `STR#` 2002 #226.
pub const TRADE_IN_LABEL: &str = "Trade-In:";
/// `STR#` 2002 #227.
pub const FINAL_PRICE_LABEL: &str = "Final Price:";
/// `STR#` 2002 #229-237, the info panel's stat labels in order.
pub const STAT_LABELS: [&str; 9] = [
    "Speed:", "Accel:", "Turn:", "Guns:", "Turrets:", "Space:", "Length:", "Mass:", "Crew:",
];
/// `STR#` 2002 #238.
pub const MAXIMUM_OF: &str = "Maximum of";
/// `STR#` 2002 #239.
pub const JUMP: &str = "jump";
/// `STR#` 2002 #240.
pub const JUMPS: &str = "jumps";
/// The mark in the cell of the class the player flies.
pub const CURRENT_MARK: &str = "(current)";

/// `ship`'s picture: its own `PICT` ([`FIRST_SHIP_PICTURE`] + ID - 128)
/// when `exists` says it is there; otherwise that of the lowest-numbered
/// ship in `bases` sharing its `shän` base image whose picture is there;
/// otherwise none. `bases` gives each ship's base image.
#[must_use]
pub fn ship_picture(
    ship: ShipId,
    bases: &[(ShipId, i16)],
    exists: impl Fn(i16) -> bool,
) -> Option<i16> {
    let picture = |id: ShipId| FIRST_SHIP_PICTURE.saturating_add(id.0.saturating_sub(FIRST_SHIP));
    let own = picture(ship);
    if exists(own) {
        return Some(own);
    }
    let (_, base) = bases.iter().find(|(id, _)| *id == ship)?;
    let mut sharers: Vec<ShipId> = bases
        .iter()
        .filter(|(id, other)| other == base && *id != ship)
        .map(|(id, _)| *id)
        .collect();
    sharers.sort();
    sharers.into_iter().map(picture).find(|&id| exists(id))
}

/// The info panel's lines for `row`'s ship.
#[must_use]
pub fn stat_lines(row: &ShipRow) -> Vec<String> {
    spec_lines(row.specs)
}

/// The info panel's lines for a ship of `specs`: its speed,
/// acceleration, turn rate, guns, turrets, free space, length, mass, crew
/// and how many jumps its fuel holds.
#[must_use]
pub fn spec_lines(specs: ShipSpecs) -> Vec<String> {
    let fields = specs.fields;
    let values = [
        fields.speed.to_string(),
        fields.accel.to_string(),
        fields.maneuver.to_string(),
        specs.max_gun.to_string(),
        specs.max_tur.to_string(),
        format!("{} tons", fields.free_mass),
        specs.length.to_string(),
        format!("{} tons", fields.mass),
        specs.crew.to_string(),
    ];
    let jumps = max_jumps(f32::from(fields.fuel));
    let mut lines: Vec<String> = STAT_LABELS
        .iter()
        .zip(values)
        .map(|(label, value)| format!("{label} {value}"))
        .collect();
    let unit = if jumps == 1 { JUMP } else { JUMPS };
    lines.push(format!("{MAXIMUM_OF} {jumps} {unit}"));
    lines
}

/// What the shipyard reads beyond the spaceport's port: each `shïp`'s
/// `shän` base image, raw.
pub trait ShipBaseImages {
    /// Each readable `shän`'s ship (`shän` ID = `shïp` ID) with its
    /// `BaseImageID`; an unreadable one is left out.
    fn ship_base_images(&self) -> Vec<(ShipId, i16)>;
}

/// What the shipyard reads about a ship: its description, through
/// [`DescriptionSource`], whether a picture exists, through
/// [`SpaceportCatalog::picture_exists`], and the ships' base images,
/// through [`ShipBaseImages`]. Anything that is all three is one.
pub trait ShipyardCatalog: SpaceportCatalog + DescriptionSource + ShipBaseImages {}

impl<T: SpaceportCatalog + DescriptionSource + ShipBaseImages + ?Sized> ShipyardCatalog for T {}

/// The catalog, shared, with a `Debug` that shows nothing of it.
#[derive(Clone)]
struct CatalogHandle(Rc<dyn ShipyardCatalog>);

impl std::fmt::Debug for CatalogHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShipyardCatalog")
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

/// The shipyard, laid out.
#[derive(Clone, Debug)]
struct Laid {
    dialog: Dialog,
    metrics: MetricsHandle,
    /// The ship whose picture and description are shown, and them.
    shown: Option<(ShipId, Shown)>,
    /// The info panel's template, if there is one.
    info: Option<DialogTemplate>,
    /// The name prompt's template, if there is one.
    text_input: Option<DialogTemplate>,
}

/// A ship to buy, and the name the player confirmed for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipOrder {
    /// The ship class.
    pub ship: ShipId,
    /// The name, as typed.
    pub name: String,
}

/// The name prompt, open, and the ship it names.
#[derive(Clone, Debug)]
struct Naming {
    ship: ShipId,
    prompt: TextInputDialog,
}

/// The info panel, open: "Shipyard Info" over [`INFO_BACKGROUND`], the
/// ship's name centred in its title and its lines in its text. The hire
/// screen opens it too.
#[derive(Clone, Debug)]
pub(super) struct Panel {
    dialog: Dialog,
    title: String,
}

impl Panel {
    /// The panel laid out by `template`, titled `title` and showing
    /// `lines`, its Done enabled and labelled in `style`, its text
    /// measured by `metrics`.
    pub(super) fn open(
        template: &DialogTemplate,
        title: String,
        lines: &[String],
        metrics: &Rc<dyn TextMetrics>,
        style: ButtonStyle,
    ) -> Self {
        let roles = [
            (PANEL_DONE_ITEM, Role::Button(DONE_LABEL.to_owned())),
            (
                PANEL_TEXT_ITEM,
                Role::ScrollText {
                    text: lines.join("\r"),
                    font: TEXT_FONT,
                    size: TEXT_SIZE,
                    color: TEXT_COLOR,
                },
            ),
        ];
        let mut template = template.clone();
        if let Some(done) = template.items.get_mut(PANEL_DONE_ITEM - 1) {
            done.enabled = true;
        }
        let dialog = Dialog::new(&template, &roles, Rc::clone(metrics))
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(PANEL_DONE_ITEM))
            .with_cancel(Some(PANEL_DONE_ITEM));
        Self { dialog, title }
    }

    /// Its dialog.
    pub(super) fn dialog(&self) -> &Dialog {
        &self.dialog
    }

    /// Its input, its sounds added to `sounds`: whether Done, Return or
    /// Escape closed it.
    pub(super) fn input(&mut self, input: &Input, sounds: &mut Vec<Sound>) -> bool {
        let event = self.dialog.input(input);
        sounds.extend(self.dialog.take_sound().map(Sound::Ui));
        event == Some(DialogEvent::Item(PANEL_DONE_ITEM))
    }

    /// Abandons any click in progress on it.
    pub(super) fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
    }

    /// Draws it, its title measured by `metrics`.
    pub(super) fn draw(&self, list: &mut DrawList, metrics: &dyn TextMetrics) {
        list.picture(INFO_BACKGROUND, self.dialog.bounds().min);
        if let Some(title) = self.dialog.item_bounds(PANEL_TITLE_ITEM) {
            let width = metrics.width(Font::Charcoal, 12.0, &self.title);
            let origin = Point::new(title.center().x - width / 2.0, title.min.y + INSET);
            list.text_in(Font::Charcoal, &self.title, origin, 12.0, None, TEXT_COLOR);
        }
        self.dialog.draw(list);
    }
}

/// The Shipyard of the stellar landed on.
#[derive(Clone, Debug)]
pub struct ShipyardScreen {
    /// The dialog, or why it cannot be shown.
    laid: Result<Laid, String>,
    shipyard: Shipyard,
    catalog: CatalogHandle,
    /// Each ship's base image, read once.
    bases: Vec<(ShipId, i16)>,
    style: ButtonStyle,
    grid: CellGrid,
    /// The info panel, while it is open.
    panel: Option<Panel>,
    /// The ship requested, until it is taken, and whether B asked.
    request: Option<(ShipId, bool)>,
    /// Whether the request last taken was asked by B.
    asked_by_key: bool,
    /// The name prompt, while it is open.
    naming: Option<Naming>,
    /// The ship ordered, named, until it is taken.
    order: Option<ShipOrder>,
    /// The ship declined at its name prompt, until it is taken.
    declined: Option<ShipId>,
    closed: bool,
    sounds: Vec<Sound>,
}

impl ShipyardScreen {
    /// The shipyard `shipyard`, laid out by `layout`: the "Shipyard"
    /// dialog's template and the metrics its text is measured by, or why
    /// there are none; the info panel by `info`, "Shipyard Info" (Info is
    /// greyed without it); and the name prompt by `text_input`, "Text
    /// Input" (or else its fallback). Its buttons are labelled in `style`, and each
    /// ship's picture and description read from `catalog`, which gives
    /// the ships' base images once, now. The first cell is selected.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        info: Result<DialogTemplate, String>,
        text_input: Result<DialogTemplate, String>,
        shipyard: Shipyard,
        catalog: Rc<dyn ShipyardCatalog>,
        style: ButtonStyle,
    ) -> Self {
        let laid = layout.map(|(mut template, metrics)| {
            for item in [
                DONE_ITEM,
                BUY_ITEM,
                INFO_ITEM,
                SCROLL_UP_ITEM,
                SCROLL_DOWN_ITEM,
            ] {
                if let Some(found) = template.items.get_mut(item - 1) {
                    found.enabled = true;
                }
            }
            let roles = [
                (DONE_ITEM, Role::Button(DONE_LABEL.to_owned())),
                (BUY_ITEM, Role::Button(BUY_SHIP_LABEL.to_owned())),
                (INFO_ITEM, Role::Button(INFO_LABEL.to_owned())),
            ];
            let mut dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(DONE_ITEM))
                .with_cancel(Some(DONE_ITEM));
            let info = info.ok();
            dialog.set_greyed(INFO_ITEM, info.is_none());
            Laid {
                dialog,
                metrics: MetricsHandle(metrics),
                shown: None,
                info,
                text_input: text_input.ok(),
            }
        });
        let bases = if laid.is_ok() {
            catalog.ship_base_images()
        } else {
            Vec::new()
        };
        let mut screen = Self {
            laid,
            shipyard,
            catalog: CatalogHandle(catalog),
            bases,
            style,
            grid: CellGrid::default(),
            panel: None,
            request: None,
            asked_by_key: false,
            naming: None,
            order: None,
            declined: None,
            closed: false,
            sounds: Vec::new(),
        };
        screen.select(0);
        screen
    }

    /// The shipyard shown.
    #[must_use]
    pub fn shipyard(&self) -> &Shipyard {
        &self.shipyard
    }

    /// The selected cell's index, if anything is listed.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.grid.selected(self.shipyard.rows.len())
    }

    /// The selected ship's row, if anything is listed.
    fn selected_row(&self) -> Option<&ShipRow> {
        self.selected().map(|index| &self.shipyard.rows[index])
    }

    /// The grid's first row shown.
    #[must_use]
    pub fn top_row(&self) -> usize {
        self.grid.top_row()
    }

    /// The dialog, if it could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.laid.as_ref().ok().map(|laid| &laid.dialog)
    }

    /// The info panel's dialog, while it is open.
    #[must_use]
    pub fn info_panel(&self) -> Option<&Dialog> {
        self.panel.as_ref().map(Panel::dialog)
    }

    /// Why the shipyard cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// Whether Done has been activated.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The ship requested with Buy Ship since it was last taken, once.
    pub fn take_request(&mut self) -> Option<ShipId> {
        let (ship, by_key) = self.request.take()?;
        self.asked_by_key = by_key;
        Some(ship)
    }

    /// Opens the name prompt for `naming`'s ship over the shipyard, laid
    /// out by the "Text Input" dialog or its fallback. When B asked for
    /// it, the B typed is dropped. Nothing opens when the shipyard could
    /// not be laid out.
    pub fn open_naming(&mut self, naming: &ShipNaming) {
        let Ok(laid) = &self.laid else {
            return;
        };
        let metrics = Rc::clone(&laid.metrics.0);
        let built = laid.text_input.as_ref().and_then(|template| {
            TextInputDialog::new(
                template,
                &naming.prompt,
                &naming.default,
                SHIP_NAME_CHARS,
                self.style,
                Rc::clone(&metrics),
            )
            .ok()
        });
        let mut prompt = built.unwrap_or_else(|| {
            TextInputDialog::fallback(
                &naming.prompt,
                &naming.default,
                SHIP_NAME_CHARS,
                self.style,
                metrics,
            )
        });
        if self.asked_by_key {
            prompt.flush_typed_key();
        }
        self.naming = Some(Naming {
            ship: naming.ship,
            prompt,
        });
    }

    /// The name prompt, while it is open.
    #[must_use]
    pub fn naming(&self) -> Option<&TextInputDialog> {
        self.naming.as_ref().map(|naming| &naming.prompt)
    }

    /// The ship ordered at its name prompt since it was last taken, once.
    pub fn take_order(&mut self) -> Option<ShipOrder> {
        self.order.take()
    }

    /// The ship declined at its name prompt since it was last taken, once.
    pub fn take_declined(&mut self) -> Option<ShipId> {
        self.declined.take()
    }

    /// Where cell `index` (counted from the grid's first row shown) is,
    /// if the grid could be laid out.
    #[must_use]
    pub fn cell_bounds(&self, index: usize) -> Option<Bounds> {
        let grid = self.dialog()?.item_bounds(GRID_ITEM)?;
        Some(grid::cell_bounds(grid, index))
    }

    /// Shows `shipyard`, the shipyard after an order, keeping the same
    /// ship selected when it is still listed, or the cell where it was.
    pub fn set_shipyard(&mut self, shipyard: Shipyard) {
        let kept = self
            .selected_row()
            .map(|row| row.id)
            .and_then(|id| shipyard.rows.iter().position(|row| row.id == id));
        self.shipyard = shipyard;
        self.select(kept.unwrap_or(self.grid.selected_raw()));
    }

    /// Selects cell `index`, or the last when there is no such cell, and
    /// scrolls the grid to show it.
    fn select(&mut self, index: usize) {
        self.grid.select(index, self.shipyard.rows.len());
        self.selected_changed();
    }

    /// Greys Buy Ship when the selected ship cannot be bought, and reads
    /// its picture and description if they are not shown already.
    fn selected_changed(&mut self) {
        let buys = self.selected_row().is_some_and(|row| row.buy.is_ok());
        let ship = self.selected_row().map(|row| row.id);
        let catalog = Rc::clone(&self.catalog.0);
        let Ok(laid) = &mut self.laid else {
            return;
        };
        laid.dialog.set_greyed(BUY_ITEM, !buys);
        let Some(ship) = ship else {
            laid.shown = None;
            return;
        };
        if laid.shown.as_ref().is_some_and(|(shown, _)| *shown == ship) {
            return;
        }
        let area = laid
            .dialog
            .item_bounds(DESCRIPTION_ITEM)
            .unwrap_or(Bounds::at(Point::new(0.0, 0.0), 0.0, 0.0));
        let picture = ship_picture(ship, &self.bases, |id| catalog.picture_exists(id));
        let description = nova_data::ship_desc_id(ship);
        let shown = Shown::read(&*catalog, picture, description, area, &laid.metrics.0);
        laid.shown = Some((ship, shown));
    }

    /// Requests the selected ship, if it can be bought, saying whether B
    /// asked.
    fn ask(&mut self, by_key: bool) {
        if let Some(row) = self.selected_row().filter(|row| row.buy.is_ok()) {
            self.request = Some((row.id, by_key));
        }
    }

    /// Opens the info panel on the selected ship, if there is one and a
    /// panel to open.
    fn open_info(&mut self) {
        let Some(row) = self.selected_row() else {
            return;
        };
        let (title, lines) = (row.name.clone(), stat_lines(row));
        let Ok(laid) = &self.laid else {
            return;
        };
        let Some(template) = &laid.info else {
            return;
        };
        self.panel = Some(Panel::open(
            template,
            title,
            &lines,
            &laid.metrics.0,
            self.style,
        ));
    }

    /// Activates dialog item `item`: Done closes, Buy Ship asks, Info
    /// opens the panel, and the arrows scroll. Any other item does
    /// nothing.
    fn activate(&mut self, item: usize) {
        let count = self.shipyard.rows.len();
        match item {
            DONE_ITEM => self.closed = true,
            BUY_ITEM => self.ask(false),
            INFO_ITEM => self.open_info(),
            SCROLL_UP_ITEM => self.grid.scroll(false, count),
            SCROLL_DOWN_ITEM => self.grid.scroll(true, count),
            _ => {}
        }
    }

    /// The cell index under `at` on the grid, if a listed ship is there.
    fn cell_at(&self, at: Point) -> Option<usize> {
        let grid = self.dialog()?.item_bounds(GRID_ITEM)?;
        self.grid.cell_at(grid, at, self.shipyard.rows.len())
    }

    /// The info panel's input: Done, Return and Escape close it.
    fn panel_input(&mut self, input: &Input) {
        let Some(panel) = &mut self.panel else {
            return;
        };
        if panel.input(input, &mut self.sounds) {
            self.panel = None;
        }
    }

    /// The name prompt's input: OK orders the ship, and Cancel declines
    /// it, each closing the prompt.
    fn naming_input(&mut self, input: &Input) {
        let Some(naming) = &mut self.naming else {
            return;
        };
        naming.prompt.input(input);
        self.sounds.extend(naming.prompt.take_sounds());
        match naming.prompt.take_outcome() {
            Some(TextInputOutcome::Confirm(name)) => {
                self.order = Some(ShipOrder {
                    ship: naming.ship,
                    name,
                });
                self.naming = None;
            }
            Some(TextInputOutcome::Cancel) => {
                self.declined = Some(naming.ship);
                self.naming = None;
            }
            None => {}
        }
    }

    fn draw_grid(&self, laid: &Laid, list: &mut DrawList) {
        let line_height = laid.metrics.0.line_height(TEXT_FONT, TEXT_SIZE);
        let Some(area) = laid.dialog.item_bounds(GRID_ITEM) else {
            return;
        };
        if self.shipyard.rows.is_empty() {
            let origin = Point::new(area.min.x + INSET, area.min.y + INSET);
            text(list, NO_SHIPS, origin, TEXT_COLOR);
            return;
        }
        for (index, cell) in self.grid.shown(area, self.shipyard.rows.len()) {
            let row = &self.shipyard.rows[index];
            if Some(index) == self.selected() {
                fill_rect(list, cell, SELECTED_COLOR);
            }
            outline(list, cell, GREY);
            let mut y = cell.min.y + INSET;
            for (line, color) in name_lines(&row.short_name) {
                text(list, line, Point::new(cell.min.x + INSET, y), color);
                y += line_height;
            }
            if row.id == self.shipyard.current {
                let origin = Point::new(cell.min.x + INSET, cell.max.y - INSET - line_height);
                text(list, CURRENT_MARK, origin, TEXT_COLOR);
            }
        }
    }

    fn draw_selected(&self, laid: &Laid, list: &mut DrawList) {
        let Some(row) = self.selected_row() else {
            return;
        };
        if let (Some((_, shown)), Some(area)) = (&laid.shown, laid.dialog.item_bounds(PICTURE_ITEM))
        {
            shown.draw(list, area);
        }
        let Some(info) = laid.dialog.item_bounds(INFO_BOX_ITEM) else {
            return;
        };
        let line_height = laid.metrics.0.line_height(TEXT_FONT, TEXT_SIZE);
        let lines = [
            format!("{PRICE_LABEL} {}", row.price),
            format!("{TRADE_IN_LABEL} {}", self.shipyard.trade_in),
            format!("{FINAL_PRICE_LABEL} {}", self.shipyard.final_price(row)),
        ];
        for (n, line) in lines.iter().enumerate() {
            let origin = Point::new(info.min.x + INSET, info.min.y + line_height * n as f32);
            text(list, line, origin, TEXT_COLOR);
        }
    }

    fn draw_panel(&self, laid: &Laid, list: &mut DrawList) {
        if let Some(panel) = &self.panel {
            panel.draw(list, &*laid.metrics.0);
        }
    }
}

impl Screen for ShipyardScreen {
    /// Every input goes to the shipyard, or to its name prompt or info
    /// panel while that is open; it never quits.
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
        if self.naming.is_some() {
            self.naming_input(input);
            return ScreenAction::None;
        }
        if self.panel.is_some() {
            self.panel_input(input);
            return ScreenAction::None;
        }
        let count = self.shipyard.rows.len();
        let arrow = match *input {
            Input::Key {
                key, pressed: true, ..
            } => self.grid.arrow(key, count),
            _ => false,
        };
        let pressed_once = |wanted: Key| matches!(*input, Input::Key { key, pressed: true, repeat: false } if key == wanted);
        if arrow {
            self.selected_changed();
        } else if pressed_once(BUY_KEY) {
            self.ask(true);
        } else if pressed_once(INFO_KEY) {
            self.open_info();
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
                        "Cannot show the shipyard: {reason}. Press Return or Escape to close it."
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
        let count = self.shipyard.rows.len();
        grid::draw_arrows(
            list,
            laid.dialog.item_bounds(SCROLL_UP_ITEM),
            laid.dialog.item_bounds(SCROLL_DOWN_ITEM),
            self.grid.can_scroll(count),
        );
        laid.dialog.draw(list);
        self.draw_panel(laid, list);
        if let Some(naming) = &self.naming {
            naming.prompt.draw(list);
        }
    }

    fn cancel_pointer(&mut self) {
        if let Some(naming) = &mut self.naming {
            naming.prompt.cancel_pointer();
        } else if let Some(panel) = &mut self.panel {
            panel.cancel_pointer();
        } else if let Ok(laid) = &mut self.laid {
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

    use nova_sim::{ShipFields, ShipRefusal, ShipSpecs};

    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::sound::UiSound;
    use crate::spaceport::catalog::{PortRecord, StellarId};
    use crate::spaceport::grid::NO_PICTURE;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// `items`, each (x, y, w, h, enabled), as user items.
    fn items(items: &[(f32, f32, f32, f32, bool)]) -> Vec<ItemTemplate> {
        items
            .iter()
            .map(|&(x, y, w, h, enabled)| ItemTemplate {
                bounds: rect(x, y, w, h),
                enabled,
                kind: ItemSpec::User,
            })
            .collect()
    }

    /// Stock "Shipyard": `DLOG` 1004, 765 x 323 and centred, and its
    /// thirteen user items, where 2, 3, 4 and 11 are parked outside it.
    fn template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(100.0, 100.0, 765.0, 323.0),
            placement: Placement::Center,
            items: items(&[
                (365.0, 289.0, 109.0, 25.0, true),
                (248.0, 440.0, 68.0, 30.0, false),
                (251.0, 491.0, 29.0, 21.0, false),
                (394.0, 489.0, 99.0, 25.0, false),
                (9.0, 8.0, 333.0, 271.0, true),
                (354.0, 10.0, 192.0, 267.0, false),
                (480.0, 289.0, 109.0, 25.0, false),
                (557.0, 8.0, 200.0, 200.0, false),
                (614.0, 214.0, 143.0, 100.0, false),
                (253.0, 289.0, 89.0, 25.0, false),
                (170.0, 449.0, 32.0, 32.0, false),
                (141.0, 288.0, 25.0, 25.0, false),
                (171.0, 288.0, 25.0, 25.0, false),
            ]),
        }
    }

    /// Stock "Shipyard Info": `DLOG` 1005, 250 x 285 and centred, with
    /// Done (1), the title (3) and the text (5).
    fn info_template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(0.0, 0.0, 250.0, 285.0),
            placement: Placement::Center,
            items: items(&[
                (86.0, 253.0, 74.0, 25.0, false),
                (300.0, 0.0, 1.0, 1.0, false),
                (3.0, 3.0, 240.0, 24.0, false),
                (300.0, 0.0, 1.0, 1.0, false),
                (9.0, 32.0, 234.0, 214.0, false),
            ]),
        }
    }

    /// Where the dialog goes: (1024 - 765) / 2 and (768 - 323) / 2,
    /// floored.
    const ORIGIN: Point = Point::new(129.0, 222.0);

    /// Ship 128's picture (`PICT` 5000) exists, and 5002's; ships 128,
    /// 130 and 131 share base image 1000, ship 129 has its own, 1001.
    /// Every description is "dësc n.". Records what it is asked.
    struct FakeArt {
        pictures: Vec<i16>,
        bases: Vec<(ShipId, i16)>,
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
            Ok(format!("dësc {id}."))
        }

        fn button_style(&self) -> ButtonStyle {
            panic!("the style is given")
        }
    }

    impl ShipBaseImages for FakeArt {
        fn ship_base_images(&self) -> Vec<(ShipId, i16)> {
            self.asked.borrow_mut().push("bases".to_owned());
            self.bases.clone()
        }
    }

    fn art() -> Rc<FakeArt> {
        Rc::new(FakeArt {
            pictures: vec![5000, 5002],
            bases: vec![
                (ShipId(128), 1000),
                (ShipId(129), 1001),
                (ShipId(130), 1000),
                (ShipId(131), 1000),
            ],
            asked: RefCell::default(),
        })
    }

    /// The specs of a ship of speed 300, 12 tons free, mass 25, fuel 300.
    const SPECS: ShipSpecs = ShipSpecs {
        fields: ShipFields {
            speed: 300,
            accel: 350,
            maneuver: 20,
            shield: 0,
            armor: 0,
            fuel: 300,
            fuel_regen: 0,
            holds: 15,
            mass: 25,
            free_mass: 12,
            contribute: 0,
            shield_rech: 0,
            armor_rech: 0,
            flags2: 0,
        },
        max_gun: 2,
        max_tur: 1,
        length: 26,
        crew: 4,
    };

    fn row(id: i16, short_name: &str, price: i64) -> ShipRow {
        ShipRow {
            id: ShipId(id),
            name: format!("Ship {id}"),
            short_name: short_name.to_owned(),
            price,
            specs: SPECS,
            buy: Ok(()),
        }
    }

    /// The Shuttle (flown), a Heavy Shuttle, a Courier too dear, and a
    /// Viper whose `Require` is unmet, with 25,000 credits and a 2,500
    /// trade-in.
    fn shipyard() -> Shipyard {
        Shipyard {
            rows: vec![
                row(128, "Shuttle", 10_000),
                row(129, "Heavy\\nShuttle", 17_500),
                ShipRow {
                    buy: Err(ShipRefusal::CannotAfford),
                    ..row(130, "*Courier", 90_000)
                },
                ShipRow {
                    buy: Err(ShipRefusal::NotForSale),
                    ..row(131, "Viper", 20_000)
                },
            ],
            trade_in: 2500,
            cash: 25_000,
            current: ShipId(128),
        }
    }

    fn layout() -> (DialogTemplate, Rc<dyn TextMetrics>) {
        (template(), Rc::new(MonoMetrics))
    }

    fn screen_with(shipyard: Shipyard, art: &Rc<FakeArt>) -> ShipyardScreen {
        let catalog: Rc<dyn ShipyardCatalog> = Rc::clone(art) as Rc<dyn ShipyardCatalog>;
        ShipyardScreen::new(
            Ok(layout()),
            Ok(info_template()),
            Ok(text_input_template()),
            shipyard,
            catalog,
            ButtonStyle::STOCK,
        )
    }

    fn screen_of(shipyard: Shipyard) -> ShipyardScreen {
        screen_with(shipyard, &art())
    }

    fn screen() -> ShipyardScreen {
        screen_of(shipyard())
    }

    fn drawn(screen: &ShipyardScreen) -> Vec<DrawCommand> {
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

    fn words(screen: &ShipyardScreen) -> Vec<String> {
        texts(&drawn(screen))
            .into_iter()
            .map(|(text, _, _)| text)
            .collect()
    }

    fn item(screen: &ShipyardScreen, number: usize) -> Bounds {
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

    fn press(screen: &mut ShipyardScreen, k: Key) {
        screen.input(&key(k, true, false));
    }

    fn click(screen: &mut ShipyardScreen, at: Point) {
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn click_item(screen: &mut ShipyardScreen, number: usize) {
        let at = item(screen, number).center();
        click(screen, at);
    }

    fn click_cell(screen: &mut ShipyardScreen, index: usize) {
        let at = screen.cell_bounds(index).expect("a cell").center();
        click(screen, at);
    }

    /// Whether `label` draws enabled.
    fn enabled(screen: &ShipyardScreen, label: &str) -> bool {
        texts(&drawn(screen))
            .into_iter()
            .find_map(|(text, _, color)| (text == label).then_some(color))
            .is_some_and(|color| color != ButtonStyle::STOCK.grey)
    }

    /// The lines drawn in the info box.
    fn info_box(screen: &ShipyardScreen) -> Vec<String> {
        let info = item(screen, INFO_BOX_ITEM);
        texts(&drawn(screen))
            .into_iter()
            .filter(|(_, at, _)| info.contains(*at))
            .map(|(text, _, _)| text)
            .collect()
    }

    #[test]
    fn the_named_values() {
        assert_eq!((SHIPYARD_DIALOG, SHIP_INFO_DIALOG), (1004, 1005));
        assert_eq!(BACKGROUND, ImageKey::picture(8501));
        assert_eq!(INFO_BACKGROUND, ImageKey::picture(8506));
        assert_eq!(
            [
                DONE_ITEM,
                GRID_ITEM,
                DESCRIPTION_ITEM,
                BUY_ITEM,
                PICTURE_ITEM,
                INFO_BOX_ITEM,
                INFO_ITEM,
                SCROLL_UP_ITEM,
                SCROLL_DOWN_ITEM
            ],
            [1, 5, 6, 7, 8, 9, 10, 12, 13]
        );
        assert_eq!(
            (PANEL_DONE_ITEM, PANEL_TITLE_ITEM, PANEL_TEXT_ITEM),
            (1, 3, 5)
        );
        assert_eq!((BUY_KEY, INFO_KEY), (Key::Char('b'), Key::Char('i')));
        assert_eq!(FIRST_SHIP_PICTURE, 5000);
        assert_eq!(
            [BUY_SHIP_LABEL, INFO_LABEL, PRICE_LABEL, TRADE_IN_LABEL],
            ["Buy Ship", "Info", "Ship Price:", "Trade-In:"]
        );
        assert_eq!(FINAL_PRICE_LABEL, "Final Price:");
        assert_eq!(NO_SHIPS, "There are no ships available for purchase here.");
        assert_eq!((MAXIMUM_OF, JUMP, JUMPS), ("Maximum of", "jump", "jumps"));
    }

    // The picture.

    /// Pictures 5000 and 5003 exist.
    fn exists(id: i16) -> bool {
        [5000, 5003].contains(&id)
    }

    #[test]
    fn a_ship_with_its_own_picture_shows_it() {
        let bases = [(ShipId(128), 1000), (ShipId(131), 1000)];
        assert_eq!(ship_picture(ShipId(128), &bases, exists), Some(5000));
        assert_eq!(ship_picture(ShipId(131), &bases, exists), Some(5003));
        assert_eq!(ship_picture(ShipId(131), &[], exists), Some(5003));
    }

    #[test]
    fn a_ship_without_one_shows_the_lowest_numbered_sharers_picture() {
        let bases = [
            (ShipId(361), 1000),
            (ShipId(131), 1000),
            (ShipId(128), 1000),
            (ShipId(129), 1001),
        ];
        assert_eq!(ship_picture(ShipId(361), &bases, exists), Some(5000));
    }

    #[test]
    fn a_sharer_without_a_picture_is_skipped() {
        let bases = [
            (ShipId(361), 1000),
            (ShipId(129), 1000),
            (ShipId(131), 1000),
        ];
        assert_eq!(ship_picture(ShipId(361), &bases, exists), Some(5003));
    }

    #[test]
    fn a_ship_with_no_base_image_or_no_sharer_with_a_picture_has_none() {
        let bases = [(ShipId(361), 1000), (ShipId(129), 1000)];
        assert_eq!(ship_picture(ShipId(362), &bases, exists), None);
        assert_eq!(ship_picture(ShipId(361), &bases, exists), None);
        assert_eq!(
            ship_picture(
                ShipId(361),
                &[(ShipId(361), 1000), (ShipId(128), 999)],
                exists
            ),
            None,
            "another base image"
        );
    }

    #[test]
    fn an_impossible_ship_id_does_not_overflow() {
        assert_eq!(ship_picture(ShipId(i16::MIN), &[], |_| false), None);
        assert_eq!(
            ship_picture(ShipId(i16::MAX), &[], |id| id == i16::MAX),
            Some(i16::MAX)
        );
    }

    // The grid.

    #[test]
    fn it_is_centred_with_the_first_cell_selected() {
        let screen = screen();
        assert_eq!(screen.problem(), None);
        assert_eq!(screen.dialog().expect("laid out").bounds().min, ORIGIN);
        assert_eq!(screen.selected(), Some(0));
        assert_eq!(screen.top_row(), 0);
        assert!(!screen.closed());
        assert_eq!(screen.shipyard(), &shipyard());
        assert!(screen.info_panel().is_none());
        assert_eq!(
            screen.cell_bounds(5),
            Some(rect(138.0 + 83.0, 230.0 + 67.0, 83.0, 67.0))
        );
    }

    #[test]
    fn the_grid_shows_the_names_and_marks_the_ship_flown() {
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
        let bottom = screen.cell_bounds(0).expect("a cell");
        let shown = texts(&commands);
        for expected in [
            ("Shuttle".to_owned(), cell(0, INSET), TEXT_COLOR),
            (
                CURRENT_MARK.to_owned(),
                Point::new(bottom.min.x + INSET, bottom.max.y - INSET - 12.0),
                TEXT_COLOR,
            ),
            ("Heavy".to_owned(), cell(1, INSET), TEXT_COLOR),
            ("Shuttle".to_owned(), cell(1, INSET + 12.0), TEXT_COLOR),
            ("*Courier".to_owned(), cell(2, INSET), GREY),
            ("Viper".to_owned(), cell(3, INSET), TEXT_COLOR),
        ] {
            assert!(shown.contains(&expected), "{expected:?}: {shown:?}");
        }
        let marks = shown
            .iter()
            .filter(|(text, _, _)| text == CURRENT_MARK)
            .count();
        assert_eq!(marks, 1, "only the ship flown");
        let mut highlight = DrawList::new();
        fill_rect(
            &mut highlight,
            screen.cell_bounds(0).expect("a cell"),
            SELECTED_COLOR,
        );
        let highlight: Vec<_> = highlight.iter().cloned().collect();
        assert!(commands.windows(highlight.len()).any(|w| w == highlight));
        let mut frame = DrawList::new();
        outline(&mut frame, screen.cell_bounds(3).expect("a cell"), GREY);
        let frame: Vec<_> = frame.iter().cloned().collect();
        assert!(commands.windows(frame.len()).any(|w| w == frame));
        assert!(!words(&screen).contains(&NO_SHIPS.to_owned()));
    }

    /// `count` ships, numbered from 200.
    fn long(count: usize) -> Shipyard {
        Shipyard {
            rows: (0..count)
                .map(|n| {
                    let id = 200 + i16::try_from(n).expect("few");
                    row(id, &format!("Hull {n}"), 10)
                })
                .collect(),
            ..shipyard()
        }
    }

    #[test]
    fn the_selection_moves_by_click_and_arrows_and_the_grid_scrolls() {
        let mut screen = screen_of(long(22));
        press(&mut screen, Key::Right);
        assert_eq!(screen.selected(), Some(1));
        press(&mut screen, Key::Down);
        assert_eq!(screen.selected(), Some(5));
        press(&mut screen, Key::Up);
        press(&mut screen, Key::Left);
        assert_eq!(screen.selected(), Some(0));
        click_cell(&mut screen, 6);
        assert_eq!(screen.selected(), Some(6));
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 2, "never past the end");
        let names: Vec<_> = words(&screen)
            .into_iter()
            .filter(|w| w.starts_with("Hull "))
            .collect();
        assert_eq!(names.first().map(String::as_str), Some("Hull 8"));
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!(screen.top_row(), 1);
        click_cell(&mut screen, 0);
        assert_eq!(screen.selected(), Some(4));
        let grid = item(&screen, GRID_ITEM);
        click(&mut screen, Point::new(grid.min.x - 1.0, grid.min.y + 1.0));
        assert_eq!(screen.selected(), Some(4), "outside the grid");
        let mut short = screen_of(shipyard());
        click_cell(&mut short, 5);
        assert_eq!(short.selected(), Some(0), "an empty cell");
        assert_eq!(short.take_sounds(), [], "cells are silent");
    }

    #[test]
    fn the_arrows_are_white_only_when_the_grid_can_scroll_that_way() {
        let mut screen = screen_of(long(22));
        let up = item(&screen, SCROLL_UP_ITEM);
        let colors = |screen: &ShipyardScreen| -> Vec<Color> {
            drawn(screen)
                .into_iter()
                .filter_map(|command| match command {
                    DrawCommand::Line { from, color, .. } => Some((from, color)),
                    _ => None,
                })
                .filter(|(from, _)| {
                    up.contains(*from) || item(screen, SCROLL_DOWN_ITEM).contains(*from)
                })
                .step_by(3)
                .map(|(_, color)| color)
                .collect()
        };
        assert_eq!(colors(&screen), [GREY, TEXT_COLOR]);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(colors(&screen), [TEXT_COLOR, GREY]);
    }

    #[test]
    fn the_picture_and_description_are_read_as_the_selection_changes() {
        let art = art();
        let mut screen = screen_with(shipyard(), &art);
        let picture = item(&screen, PICTURE_ITEM);
        assert!(drawn(&screen).contains(&DrawCommand::StretchedPicture {
            image: ImageKey::picture(5000),
            top_left: picture.min,
            width: 200.0,
            height: 200.0,
        }));
        assert!(words(&screen).contains(&"dësc 13000.".to_owned()));
        press(&mut screen, Key::Right);
        let shown = texts(&drawn(&screen));
        assert!(shown.contains(&(
            NO_PICTURE.to_owned(),
            Point::new(picture.min.x + INSET, picture.min.y + INSET),
            GREY
        )));
        press(&mut screen, Key::Right);
        assert!(
            drawn(&screen).contains(&DrawCommand::StretchedPicture {
                image: ImageKey::picture(5002),
                top_left: picture.min,
                width: 200.0,
                height: 200.0,
            }),
            "its own"
        );
        press(&mut screen, Key::Right);
        assert!(
            drawn(&screen).contains(&DrawCommand::StretchedPicture {
                image: ImageKey::picture(5000),
                top_left: picture.min,
                width: 200.0,
                height: 200.0,
            }),
            "the Shuttle's, sharing its base image"
        );
        screen.set_shipyard(shipyard());
        assert_eq!(
            *art.asked.borrow(),
            [
                "bases",
                "PICT 5000",
                "dësc 13000",
                "PICT 5001",
                "dësc 13001",
                "PICT 5002",
                "dësc 13002",
                "PICT 5003",
                "PICT 5000",
                "dësc 13003",
            ],
            "the bases once, and each ship once while it stays selected"
        );
    }

    #[test]
    fn the_info_box_shows_the_price_the_trade_in_and_the_final_price() {
        let mut screen = screen();
        assert_eq!(
            info_box(&screen),
            ["Ship Price: 10000", "Trade-In: 2500", "Final Price: 7500"]
        );
        press(&mut screen, Key::Right);
        assert_eq!(
            info_box(&screen),
            ["Ship Price: 17500", "Trade-In: 2500", "Final Price: 15000"]
        );
        let paid = Shipyard {
            trade_in: 40_000,
            ..shipyard()
        };
        assert_eq!(info_box(&screen_of(paid))[2], "Final Price: -30000");
        // A line apart, from the box's top left.
        let info = item(&screen, INFO_BOX_ITEM);
        let at = |n: f32| Point::new(info.min.x + INSET, info.min.y + 12.0 * n);
        let placed: Vec<_> = texts(&drawn(&screen))
            .into_iter()
            .filter(|(text, _, _)| text.contains("Price:") || text.starts_with("Trade-In"))
            .map(|(_, origin, color)| (origin, color))
            .collect();
        assert_eq!(
            placed,
            [
                (at(0.0), TEXT_COLOR),
                (at(1.0), TEXT_COLOR),
                (at(2.0), TEXT_COLOR)
            ]
        );
    }

    #[test]
    fn buy_ship_is_greyed_when_the_ship_cannot_be_bought() {
        let mut screen = screen();
        assert!(enabled(&screen, BUY_SHIP_LABEL));
        press(&mut screen, Key::Right);
        assert!(enabled(&screen, BUY_SHIP_LABEL));
        press(&mut screen, Key::Right);
        assert!(!enabled(&screen, BUY_SHIP_LABEL), "too dear");
        click_item(&mut screen, BUY_ITEM);
        press(&mut screen, BUY_KEY);
        press(&mut screen, Key::Right);
        assert!(!enabled(&screen, BUY_SHIP_LABEL), "not for sale");
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_request(), None, "greyed asks for nothing");
        assert!(enabled(&screen, DONE_LABEL));
        assert!(enabled(&screen, INFO_LABEL));
    }

    #[test]
    fn buy_ship_requests_the_selected_ship_and_b_only_on_a_press() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(screen.take_request(), Some(ShipId(129)));
        assert_eq!(screen.take_request(), None, "once");
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_request(), Some(ShipId(129)));
        for _ in 0..3 {
            screen.input(&key(BUY_KEY, true, true));
            assert_eq!(screen.take_request(), None, "a repeat buys nothing");
        }
        screen.input(&key(BUY_KEY, false, false));
        assert_eq!(screen.take_request(), None, "nor a release");
    }

    #[test]
    fn a_new_shipyard_shows_and_regreys_keeping_the_ship_selected() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        let mut bought = shipyard();
        bought.current = ShipId(129);
        bought.rows[1].buy = Err(ShipRefusal::CannotAfford);
        bought.rows.remove(0);
        screen.set_shipyard(bought.clone());
        assert_eq!(screen.shipyard(), &bought);
        assert_eq!(screen.selected(), Some(0), "followed");
        assert!(!enabled(&screen, BUY_SHIP_LABEL));
        screen.set_shipyard(long(0));
        assert_eq!(screen.selected(), None);
        assert!(!enabled(&screen, BUY_SHIP_LABEL));
    }

    #[test]
    fn with_nothing_listed_the_grid_says_so_and_buy_and_info_do_nothing() {
        let art = art();
        let mut screen = screen_with(long(0), &art);
        assert_eq!(screen.selected(), None);
        let grid = item(&screen, GRID_ITEM);
        assert!(texts(&drawn(&screen)).contains(&(
            NO_SHIPS.to_owned(),
            Point::new(grid.min.x + INSET, grid.min.y + INSET),
            TEXT_COLOR
        )));
        assert!(info_box(&screen).is_empty());
        assert!(!enabled(&screen, BUY_SHIP_LABEL));
        press(&mut screen, BUY_KEY);
        press(&mut screen, INFO_KEY);
        click_item(&mut screen, INFO_ITEM);
        assert_eq!(screen.take_request(), None);
        assert!(screen.info_panel().is_none());
        assert_eq!(*art.asked.borrow(), ["bases"], "nothing else to read");
    }

    // Naming the ship.

    /// Stock "Text Input": `DLOG` 3001, 360 x 138 and centred, with OK
    /// (1), a parked picture (2), the prompt (3), a picture (4), the field
    /// (5) and Cancel (6).
    fn text_input_template() -> DialogTemplate {
        let item = |x, y, w, h, enabled, kind| ItemTemplate {
            bounds: rect(x, y, w, h),
            enabled,
            kind,
        };
        DialogTemplate {
            bounds: rect(0.0, 0.0, 360.0, 138.0),
            placement: Placement::Center,
            items: vec![
                item(
                    252.0,
                    106.0,
                    70.0,
                    20.0,
                    true,
                    ItemSpec::Button("OK".into()),
                ),
                item(7.0, 147.0, 32.0, 32.0, false, ItemSpec::Picture(129)),
                item(
                    52.0,
                    5.0,
                    295.0,
                    50.0,
                    false,
                    ItemSpec::StaticText(String::new()),
                ),
                item(7.0, 5.0, 32.0, 32.0, false, ItemSpec::Picture(130)),
                item(
                    91.0,
                    64.0,
                    200.0,
                    16.0,
                    true,
                    ItemSpec::EditText(String::new()),
                ),
                item(
                    170.0,
                    106.0,
                    70.0,
                    20.0,
                    true,
                    ItemSpec::Button("Cancel".into()),
                ),
            ],
        }
    }

    fn naming() -> ShipNaming {
        ShipNaming {
            ship: ShipId(129),
            prompt: "Please name your new The Heavy Shuttle: ".to_owned(),
            default: "Ship 129 491".to_owned(),
        }
    }

    fn typed(screen: &mut ShipyardScreen, text: &str) {
        for c in text.chars() {
            press(screen, Key::Char(c.to_ascii_lowercase()));
            screen.input(&Input::Text(c));
        }
    }

    /// The screen with B pressed on ship 129 and its prompt open.
    fn naming_by_key() -> ShipyardScreen {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_request(), Some(ShipId(129)));
        screen.open_naming(&naming());
        screen
    }

    fn naming_click(screen: &mut ShipyardScreen, item: usize) {
        let at = screen
            .naming()
            .expect("open")
            .dialog()
            .item_bounds(item)
            .expect("an item")
            .center();
        click(screen, at);
    }

    #[test]
    fn opening_the_naming_routes_input_to_it_and_ok_orders_the_ship_with_the_name() {
        let mut screen = naming_by_key();
        let prompt = screen.naming().expect("open");
        assert_eq!(prompt.field().text(), "Ship 129 491");
        screen.input(&Input::Text('b'));
        assert_eq!(
            screen.naming().expect("open").field().text(),
            "Ship 129 491",
            "the B that asked types nothing"
        );
        typed(&mut screen, "Kestrel");
        press(&mut screen, INFO_KEY);
        assert!(screen.info_panel().is_none(), "the prompt takes the keys");
        assert_eq!(screen.take_order(), None);
        press(&mut screen, Key::Enter);
        assert_eq!(
            screen.take_order(),
            Some(ShipOrder {
                ship: ShipId(129),
                name: "Kestrel".to_owned(),
            })
        );
        assert_eq!(screen.take_order(), None, "once");
        assert!(screen.naming().is_none(), "closed");
        assert!(!screen.closed(), "Return went to the prompt alone");
        assert_eq!(screen.take_declined(), None);
    }

    #[test]
    fn ok_clicked_orders_the_ship_with_its_sounds() {
        let mut screen = naming_by_key();
        screen.take_sounds();
        naming_click(&mut screen, crate::ui::text_input::OK_ITEM);
        assert_eq!(
            screen.take_sounds(),
            [Sound::Ui(UiSound::ButtonDown), Sound::Ui(UiSound::ButtonUp)]
        );
        assert_eq!(
            screen.take_order(),
            Some(ShipOrder {
                ship: ShipId(129),
                name: "Ship 129 491".to_owned(),
            })
        );
    }

    #[test]
    fn a_name_too_long_alerts_and_keeps_the_prompt_open() {
        let mut screen = naming_by_key();
        typed(&mut screen, &"x".repeat(65));
        screen.take_sounds();
        press(&mut screen, Key::Enter);
        assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::Alert)]);
        assert_eq!(screen.take_order(), None);
        assert!(screen.naming().is_some());
    }

    #[test]
    fn cancel_declines_the_ship() {
        let mut screen = naming_by_key();
        press(&mut screen, Key::Escape);
        assert!(screen.naming().is_some(), "Escape does nothing");
        assert!(!screen.closed());
        naming_click(&mut screen, crate::ui::text_input::CANCEL_ITEM);
        assert_eq!(screen.take_declined(), Some(ShipId(129)));
        assert_eq!(screen.take_declined(), None, "once");
        assert_eq!(screen.take_order(), None);
        assert!(screen.naming().is_none(), "closed");
        press(&mut screen, Key::Escape);
        assert!(screen.closed(), "the shipyard takes the keys again");
    }

    #[test]
    fn a_click_on_buy_ship_does_not_flush_text() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(screen.take_request(), Some(ShipId(129)));
        screen.open_naming(&naming());
        screen.input(&Input::Text('K'));
        assert_eq!(screen.naming().expect("open").field().text(), "K");
    }

    #[test]
    fn the_naming_falls_back_without_the_text_input_dialog() {
        let mut screen = ShipyardScreen::new(
            Ok(layout()),
            Ok(info_template()),
            Err("no DLOG 3001".to_owned()),
            shipyard(),
            art(),
            ButtonStyle::STOCK,
        );
        press(&mut screen, Key::Right);
        click_item(&mut screen, BUY_ITEM);
        screen.take_request();
        screen.open_naming(&naming());
        let prompt = screen.naming().expect("open");
        let bounds = prompt.dialog().bounds();
        assert_eq!((bounds.width(), bounds.height()), (360.0, 138.0));
        press(&mut screen, Key::Enter);
        assert_eq!(
            screen.take_order().map(|order| order.name),
            Some("Ship 129 491".to_owned())
        );
    }

    #[test]
    fn the_prompt_is_drawn_over_the_shipyard() {
        let mut screen = screen();
        let under = drawn(&screen);
        screen.open_naming(&naming());
        let commands = drawn(&screen);
        assert_eq!(commands[..under.len()], under[..]);
        let mut prompt = DrawList::new();
        screen.naming().expect("open").draw(&mut prompt);
        assert_eq!(
            commands[under.len()..],
            prompt.iter().cloned().collect::<Vec<_>>()[..]
        );
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_on_the_prompt() {
        let mut screen = naming_by_key();
        let at = screen
            .naming()
            .expect("open")
            .dialog()
            .item_bounds(crate::ui::text_input::OK_ITEM)
            .expect("OK")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.take_order(), None);
        assert!(screen.naming().is_some());
    }

    // The info panel.

    #[test]
    fn the_stat_lines_are_the_records_with_the_jumps_its_fuel_holds() {
        assert_eq!(
            stat_lines(&row(129, "Heavy", 1)),
            [
                "Speed: 300",
                "Accel: 350",
                "Turn: 20",
                "Guns: 2",
                "Turrets: 1",
                "Space: 12 tons",
                "Length: 26",
                "Mass: 25 tons",
                "Crew: 4",
                "Maximum of 3 jumps",
            ]
        );
        let fuel = |fuel| {
            let mut row = row(129, "Heavy", 1);
            row.specs.fields.fuel = fuel;
            stat_lines(&row).pop().expect("a line")
        };
        assert_eq!(fuel(100), "Maximum of 1 jump");
        assert_eq!(fuel(199), "Maximum of 1 jump");
        assert_eq!(fuel(99), "Maximum of 0 jumps");
        assert_eq!(fuel(-100), "Maximum of 0 jumps");
    }

    /// The panel's item `number`.
    fn panel_item(screen: &ShipyardScreen, number: usize) -> Bounds {
        screen
            .info_panel()
            .expect("open")
            .item_bounds(number)
            .expect("an item")
    }

    #[test]
    fn info_and_i_open_the_panel_with_the_ships_name_and_stats() {
        let mut screen = screen();
        press(&mut screen, Key::Right);
        click_item(&mut screen, INFO_ITEM);
        let panel = screen.info_panel().expect("open");
        assert_eq!(
            panel.bounds().min,
            Point::new((1024.0 - 250.0) / 2.0, ((768.0 - 285.0) / 2.0_f32).floor())
        );
        let text = panel.scroll_text().expect("the stats");
        assert_eq!(text.lines()[0], "Speed: 300");
        assert_eq!(text.lines().len(), 10);
        let commands = drawn(&screen);
        let background = commands
            .iter()
            .position(|c| {
                *c == DrawCommand::Picture {
                    image: INFO_BACKGROUND,
                    top_left: panel.bounds().min,
                }
            })
            .expect("drawn");
        let title = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { text, .. } if text == "Ship 129"))
            .expect("the name");
        let done_buy = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { text, .. } if text == BUY_SHIP_LABEL))
            .expect("the shipyard under it");
        assert!(done_buy < background && background < title);
        let DrawCommand::Text {
            origin, font, size, ..
        } = &commands[title]
        else {
            unreachable!()
        };
        let area = panel_item(&screen, PANEL_TITLE_ITEM);
        let width = MonoMetrics.width(Font::Charcoal, 12.0, "Ship 129");
        assert_eq!(
            *origin,
            Point::new(area.center().x - width / 2.0, area.min.y + INSET),
            "centred across the title"
        );
        assert_eq!((*font, *size), (Font::Charcoal, 12.0));
        // Its Done closes it alone.
        let done = panel_item(&screen, PANEL_DONE_ITEM).center();
        click(&mut screen, done);
        assert!(screen.info_panel().is_none());
        assert!(!screen.closed());
        assert_eq!(
            screen.take_sounds(),
            [
                Sound::Ui(UiSound::ButtonDown),
                Sound::Ui(UiSound::ButtonUp),
                Sound::Ui(UiSound::ButtonDown),
                Sound::Ui(UiSound::ButtonUp)
            ]
        );
        press(&mut screen, INFO_KEY);
        assert!(screen.info_panel().is_some(), "I opens it");
        screen.input(&key(INFO_KEY, true, true));
        assert!(screen.info_panel().is_some());
    }

    #[test]
    fn return_and_escape_close_the_panel_alone_and_it_takes_every_input() {
        for k in [Key::Enter, Key::Escape] {
            let mut screen = screen();
            press(&mut screen, INFO_KEY);
            press(&mut screen, BUY_KEY);
            press(&mut screen, Key::Right);
            assert_eq!(screen.take_request(), None, "the panel takes it");
            assert_eq!(screen.selected(), Some(0));
            screen.input(&key(k, true, true));
            assert!(screen.info_panel().is_some(), "not a repeat");
            press(&mut screen, k);
            assert!(screen.info_panel().is_none(), "{k:?}");
            assert!(!screen.closed());
            press(&mut screen, k);
            assert!(screen.closed());
        }
        let mut screen = screen();
        press(&mut screen, INFO_KEY);
        let done = panel_item(&screen, PANEL_DONE_ITEM).center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert!(screen.info_panel().is_some(), "the click was abandoned");
    }

    #[test]
    fn without_the_info_dialog_info_is_greyed() {
        let mut screen = ShipyardScreen::new(
            Ok(layout()),
            Err("no DLOG 1005".to_owned()),
            Ok(text_input_template()),
            shipyard(),
            art(),
            ButtonStyle::STOCK,
        );
        assert!(!enabled(&screen, INFO_LABEL));
        press(&mut screen, INFO_KEY);
        click_item(&mut screen, INFO_ITEM);
        assert!(screen.info_panel().is_none());
    }

    // Closing.

    #[test]
    fn done_return_and_escape_close_it() {
        let mut screen = screen();
        click_item(&mut screen, DONE_ITEM);
        assert!(screen.closed());
        for k in [Key::Enter, Key::Escape] {
            let mut screen = screen_of(shipyard());
            assert_eq!(screen.input(&key(k, true, false)), ScreenAction::None);
            assert!(screen.closed(), "{k:?}");
            assert_eq!(screen.take_request(), None);
        }
        let mut held = screen_of(shipyard());
        held.input(&key(Key::Escape, true, true));
        held.tick(Duration::from_secs(1));
        assert!(!held.closed());
    }

    #[test]
    fn its_buttons_sound_and_cancelling_the_pointer_abandons_a_click() {
        let mut screen = screen();
        let buy = item(&screen, BUY_ITEM).center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: buy,
        };
        screen.input(&button(true));
        assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::ButtonDown)]);
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.take_request(), None);
        press(&mut screen, BUY_KEY);
        press(&mut screen, Key::Enter);
        assert_eq!(screen.take_sounds(), [], "keys are silent");
        let debug = format!("{screen:?}");
        assert!(debug.contains("ShipyardCatalog"), "{debug}");
        assert!(debug.contains("TextMetrics"), "{debug}");
    }

    fn problem(reason: &str) -> DrawCommand {
        DrawCommand::Text {
            text: format!(
                "Cannot show the shipyard: {reason}. Press Return or Escape to close it."
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
            let mut screen = ShipyardScreen::new(
                Err("no DLOG 1004".to_owned()),
                Ok(info_template()),
                Ok(text_input_template()),
                shipyard(),
                Rc::clone(&art) as Rc<dyn ShipyardCatalog>,
                ButtonStyle::STOCK,
            );
            assert_eq!(screen.problem(), Some("no DLOG 1004"));
            assert!(screen.dialog().is_none());
            assert_eq!(screen.cell_bounds(0), None);
            assert_eq!(drawn(&screen), [problem("no DLOG 1004")]);
            for other in [
                key(k, true, true),
                key(k, false, false),
                key(BUY_KEY, true, false),
                key(INFO_KEY, true, false),
            ] {
                screen.input(&other);
            }
            click(&mut screen, Point::new(500.0, 500.0));
            screen.cancel_pointer();
            assert!(!screen.closed());
            assert_eq!(screen.take_request(), None);
            assert!(screen.info_panel().is_none());
            screen.open_naming(&naming());
            assert!(screen.naming().is_none(), "nothing to open it over");
            assert_eq!(*art.asked.borrow(), Vec::<String>::new());
            screen.input(&key(k, true, false));
            assert!(screen.closed(), "{k:?}");
        }
    }
}
