//! The hire screen: the ships for hire in the bar, laid out by the
//! interface file's "Shipyard" dialog (`DLOG`/`DITL` 1004) over its
//! background picture, as the original opens its shipyard dialog in hire
//! mode (`_DoShipyardDialog` with `_shipBuyMode` 1).
//!
//! It is the shipyard screen's twin, built from the same pieces: the grid
//! of ships ([`grid`](super::grid)), each cell drawn as the shipyard's:
//! the ship's picture in its icon slot ([`grid::draw_cell_picture`],
//! `_RetrieveMenuIcon` @0x58bc0-0x58bf3) and its `ShortName` split in two
//! lines and centred under it ([`grid::draw_cell_name`], @0x58d11-0x58f33),
//! since `_ShipyardDialogUpdate`'s cell loop (@0x58b30-0x58f91) never reads
//! `_shipBuyMode`; the
//! selected ship's picture ([`ship_picture`]); its description, the
//! pilot's, `dësc` 14000 plus its ID less 128 (the Bible: "Ship pilot
//! descriptions, shown in the hire-escort dialog"); and the Info panel
//! ("Shipyard Info", [`spec_lines`](super::shipyard::spec_lines)), whose
//! variant ([`InfoVariant`](super::shipyard::InfoVariant)) follows the
//! pilot's `dësc` 14000+ `Graphic`, as `_ShipyardFilter` loads it in
//! hire mode (@0x60c01-0x60c33). Its info box draws two
//! rows in white, as `_ShipyardDialogUpdate`'s hire mode does
//! (@0x595a0-0x596f6): "Hiring Price:" (`STR#` 2002 #228) and the fee at
//! baseline 12, and "You Have:" (#217) and the cash at baseline 36, each
//! label at the box's left and each amount at its left + 70, drawn by
//! `_DrawCommaNumber` with " cr" after it ([`credits_text`]). The original
//! shows an escort's daily pay only once it is hired, in the escort dialog,
//! which the port's comm dialog stands in for.
//!
//! Its grid's selection follows
//! [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection) as the shipyard's does (see
//! [`shipyard`](super::shipyard)): by the engine it opens with nothing
//! selected, and an empty cell pressed or a scroll that moves selects
//! nothing; by the other reading it opens on the first cell and keeps
//! it. Its arrow keys move the selection by
//! [`RuleKey::GridArrows`](nova_sim::RuleKey::GridArrows) as the
//! shipyard's do: by the engine, Up on the first row and Down onto an
//! empty cell stay, and Page Up and Page Down move as Up and Down. By both, a second press on a cell within 15 Mac ticks opens Info,
//! as `_ShipyardFilter` does in hire mode too (@0x60cd7-0x60d12).
//!
//! Hire Escort (item 1, `STR#` 150 #13), clicked or with H, asks for the
//! selected ship, and is greyed when it cannot be hired; H with no ship
//! selected, or one that cannot be hired, beeps
//! ([`UiSound::BuyRefused`], @0x5f22a-0x5f256). H acts on a
//! press only, never a key repeat, so a held H hires one ship. The screen
//! only records the order ([`HireScreen::take_order`]): whoever holds the
//! session hires it, and the spaceport closes the screen back to the bar
//! (`_DoShipyardDialog` @0x5f221).
//!
//! Done, Return and Escape close it. Without its dialog, the screen says
//! why, and Return or Escape still closes it.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::hire::{HIRE_ESCORT, HIRING_PRICE};
use nova_sim::{HireList, HireRow, ShipId};

use super::credits::{self, credits_text, draw_amount_rows};
use super::grid::{self, GridColors, Shown};
use super::layout::DONE_LABEL;
use super::shipyard::{
    BACKGROUND, BUY_ITEM, DESCRIPTION_ITEM, DONE_ITEM, GRID_ITEM, INFO_BOX_ITEM, INFO_ITEM,
    INFO_KEY, INFO_LABEL, InfoTemplates, PICTURE_ITEM, Panel, SCROLL_DOWN_ITEM, SCROLL_UP_ITEM,
    ShipyardCatalog, open_panel, ship_picture,
};
use super::shipyard_grid::{PressOutcome, ShipyardGrid};
use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::DrawList;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::{Sound, UiSound};
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, Role};

/// Hire Escort's item: the shipyard's Buy Ship.
pub const HIRE_ITEM: usize = BUY_ITEM;
/// Hires the selected ship, as Hire Escort does (`_ShipyardFilter`
/// @0x60503).
pub const HIRE_KEY: Key = Key::Char('h');
/// The first pilot description's `dësc`: ship n's is n - 128 after it.
pub const FIRST_PILOT_DESC: i16 = 14_000;
/// The first `shïp` ID.
const FIRST_SHIP: i16 = 128;

/// The `dësc` of the pilot of `ship`, for hire: [`FIRST_PILOT_DESC`] + its
/// ID - 128.
#[must_use]
pub fn pilot_desc_id(ship: ShipId) -> i16 {
    FIRST_PILOT_DESC.saturating_add(ship.0.saturating_sub(FIRST_SHIP))
}

/// The info box's rows' baselines below its top: Hiring Price and You
/// Have (`_ShipyardDialogUpdate` @0x595ac, @0x59683).
pub const HIRE_ROWS: [f32; 2] = [12.0, 36.0];

/// The info box's rows for `row`, with `cash` in hand, as
/// `_ShipyardDialogUpdate`'s hire mode draws them (@0x595a0-0x596f6): each
/// row's baseline, its label and its amount, the fee and then the cash.
/// There is no Pay line.
#[must_use]
pub fn hire_lines(row: &HireRow, cash: i64) -> [(f32, &'static str, String); 2] {
    [
        (HIRE_ROWS[0], HIRING_PRICE, credits_text(row.fee)),
        (HIRE_ROWS[1], credits::YOU_HAVE, credits_text(cash)),
    ]
}

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

/// The screen, laid out.
#[derive(Clone, Debug)]
struct Laid {
    dialog: Dialog,
    metrics: MetricsHandle,
    /// The ship whose picture and description are shown, and them.
    shown: Option<(ShipId, Shown)>,
    /// The info panel's templates.
    info: InfoTemplates,
}

/// The ships for hire in the bar.
#[derive(Clone, Debug)]
pub struct HireScreen {
    /// The dialog, or why it cannot be shown.
    laid: Result<Laid, String>,
    list: HireList,
    catalog: CatalogHandle,
    /// The grid's frame colours, read from the catalog once.
    colors: GridColors,
    /// Each listed ship's cell picture, read once when there is a dialog.
    pictures: Vec<Option<i16>>,
    style: ButtonStyle,
    /// The grid, its selection as `_ShipyardFilter` keeps it.
    grid: ShipyardGrid,
    /// The info panel, while it is open.
    panel: Option<Panel>,
    /// The ship asked for, until it is taken.
    order: Option<ShipId>,
    closed: bool,
    /// The `dësc` shown since this was last taken, to be loaded.
    description_request: Option<i16>,
    sounds: Vec<Sound>,
}

impl HireScreen {
    /// The ships `list` for hire, laid out by `layout`: the "Shipyard"
    /// dialog's template and the metrics its text is measured by, or why
    /// there are none; the info panel by `info`, "Shipyard Info" (Info is
    /// greyed without it). Its buttons are labelled in `style`, and each
    /// ship's picture and description read from `catalog`, each listed
    /// ship's cell picture once, now. By the engine nothing is
    /// selected (`_DoShipyardDialog` @0x5e6d8-0x5e705); by the other
    /// reading of [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection), read from `list`'s rules,
    /// the first cell is.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        info: InfoTemplates,
        list: HireList,
        catalog: Rc<dyn ShipyardCatalog>,
        style: ButtonStyle,
    ) -> Self {
        let laid = layout.map(|(mut template, metrics)| {
            for item in [
                DONE_ITEM,
                HIRE_ITEM,
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
                (HIRE_ITEM, Role::Button(HIRE_ESCORT.to_owned())),
                (INFO_ITEM, Role::Button(INFO_LABEL.to_owned())),
            ];
            let mut dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(DONE_ITEM))
                .with_cancel(Some(DONE_ITEM));
            dialog.set_greyed(INFO_ITEM, info.plain.is_err());
            Laid {
                dialog,
                metrics: MetricsHandle(metrics),
                shown: None,
                info,
            }
        });
        let pictures = if laid.is_ok() {
            let bases = catalog.ship_base_images();
            list.rows
                .iter()
                .map(|row| ship_picture(row.id, &bases, |id| catalog.picture_exists(id)))
                .collect()
        } else {
            Vec::new()
        };
        let grid = ShipyardGrid::open(&list.rules, list.rows.len());
        let mut screen = Self {
            laid,
            list,
            colors: catalog.grid_colors(),
            catalog: CatalogHandle(catalog),
            pictures,
            style,
            grid,
            panel: None,
            order: None,
            closed: false,
            description_request: None,
            sounds: Vec::new(),
        };
        screen.selected_changed();
        screen
    }

    /// The ships shown.
    #[must_use]
    pub fn list(&self) -> &HireList {
        &self.list
    }

    /// The selected cell's index: none when nothing is listed, or, by
    /// [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection)'s
    /// engine reading, when nothing is selected (on opening, after a press
    /// on an empty cell or a scroll that moved).
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.grid.cells().selected(self.list.rows.len())
    }

    /// The selected ship's row, if one is selected
    /// ([`HireScreen::selected`]).
    fn selected_row(&self) -> Option<&HireRow> {
        self.selected().map(|index| &self.list.rows[index])
    }

    /// The grid's first row shown.
    #[must_use]
    pub fn top_row(&self) -> usize {
        self.grid.cells().top_row()
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

    /// Why the screen cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// Whether Done has been activated.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The `dësc` of the ship the hire screen has shown since this was last
    /// taken, once, read raw until whoever flies the ship loads it as
    /// `_ShipyardFilter` does (with `_hiringEscort` set, @0x607e1, @0x60c0c) and answers with
    /// `show_description`, in the same input.
    pub fn take_description_request(&mut self) -> Option<i16> {
        self.description_request.take()
    }

    /// Shows `text`, `dësc` `id` loaded, as the description of the ship
    /// shown, while it is the ship whose `dësc` that is.
    pub fn show_description(&mut self, id: i16, text: &str) {
        let Ok(Laid {
            dialog,
            metrics,
            shown: Some((ship, shown)),
            ..
        }) = &mut self.laid
        else {
            return;
        };
        if pilot_desc_id(*ship) == id {
            let area = dialog.item_bounds(DESCRIPTION_ITEM).unwrap_or(Bounds::at(
                Point::new(0.0, 0.0),
                0.0,
                0.0,
            ));
            shown.retext(text, area, &metrics.0);
        }
    }

    /// The ship asked for since it was last taken, once.
    pub fn take_order(&mut self) -> Option<ShipId> {
        self.order.take()
    }

    /// Where cell `index` (counted from the grid's first row shown) is,
    /// if the grid could be laid out.
    #[must_use]
    pub fn cell_bounds(&self, index: usize) -> Option<Bounds> {
        let grid = self.dialog()?.item_bounds(GRID_ITEM)?;
        Some(grid::cell_bounds(grid, index))
    }

    /// Greys Hire Escort when the selected ship cannot be hired, and Info
    /// with none selected (`_DrawShipyardButtons` @0x2a3f7-0x2a405) or
    /// without its panel, and reads the ship's picture and description if
    /// they are not shown already. With none selected, none is shown
    /// (`_ShipyardDialogUpdate` @0x58fd0-0x58fda).
    fn selected_changed(&mut self) {
        let hires = self.selected_row().is_some_and(|row| row.hire.is_ok());
        let picture = self
            .selected()
            .and_then(|index| self.pictures.get(index).copied().flatten());
        let ship = self.selected_row().map(|row| row.id);
        let catalog = Rc::clone(&self.catalog.0);
        let Ok(laid) = &mut self.laid else {
            return;
        };
        laid.dialog.set_greyed(HIRE_ITEM, !hires);
        laid.dialog
            .set_greyed(INFO_ITEM, laid.info.plain.is_err() || ship.is_none());
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
        let description = Some(pilot_desc_id(ship));
        self.description_request = description;
        let shown = Shown::read(&*catalog, picture, description, area, &laid.metrics.0);
        laid.shown = Some((ship, shown));
    }

    /// Asks for the selected ship, if it can be hired; otherwise beeps
    /// ([`UiSound::BuyRefused`]), as `_DoShipyardDialog`'s item 1 does in
    /// hire mode with no ship or one it cannot hire (@0x5e9bb-0x5e9d6,
    /// @0x5f22a-0x5f256).
    fn ask(&mut self) {
        if let Some(row) = self.selected_row().filter(|row| row.hire.is_ok()) {
            self.order = Some(row.id);
        } else {
            self.sounds.push(Sound::Ui(UiSound::BuyRefused));
        }
    }

    /// Opens the info panel on the selected ship, if there is one and a
    /// panel to open.
    fn open_info(&mut self) {
        let Some(row) = self.selected_row() else {
            return;
        };
        let Ok(laid) = &self.laid else {
            return;
        };
        if laid.info.plain.is_err() {
            return;
        }
        self.panel = open_panel(
            &laid.info,
            &*self.catalog.0,
            Some(pilot_desc_id(row.id)),
            &row.name,
            &row.specs,
            &laid.metrics.0,
            self.style,
        );
    }

    /// Activates dialog item `item`: Done closes, Hire Escort asks, Info
    /// opens the panel, and the arrows scroll. Any other item does
    /// nothing.
    fn activate(&mut self, item: usize) {
        match item {
            DONE_ITEM => self.closed = true,
            HIRE_ITEM => self.ask(),
            INFO_ITEM => self.open_info(),
            SCROLL_UP_ITEM => self.scroll(false),
            SCROLL_DOWN_ITEM => self.scroll(true),
            _ => {}
        }
    }

    /// Scrolls the grid a row down, or up, never past either end. By the
    /// engine, a scroll that moves selects nothing, as the shipyard's
    /// does (`_DoShipyardDialog` @0x5f580-0x5f5be, @0x5f60b-0x5f670).
    fn scroll(&mut self, down: bool) {
        if self.grid.scroll(down, self.list.rows.len()) {
            self.selected_changed();
        }
    }

    /// A press on the grid at list index `index`, listed or not, and says
    /// whether the grid took it, as the shipyard's does
    /// ([`ShipyardGrid::press`]): a listed cell is selected, and a
    /// double-click on it opens Info; an empty one selects nothing by the
    /// engine and is left to the dialog by the other reading.
    fn press_cell(&mut self, index: usize) -> bool {
        match self.grid.press(index, self.list.rows.len()) {
            PressOutcome::Selected { double } => {
                self.selected_changed();
                if double {
                    self.open_info();
                }
            }
            PressOutcome::Cleared => self.selected_changed(),
            PressOutcome::FallThrough => return false,
        }
        true
    }

    /// The list index under `at` on the grid, listed or not
    /// ([`CellGrid::cell_under`](grid::CellGrid::cell_under)).
    fn cell_under(&self, at: Point) -> Option<usize> {
        let grid = self.dialog()?.item_bounds(GRID_ITEM)?;
        self.grid.cells().cell_under(grid, at)
    }

    /// The shown cells, each drawn as the shipyard's: the ship's picture in
    /// its icon slot ([`grid::draw_cell_picture`]) and its two-line
    /// `ShortName` centred under it ([`grid::draw_cell_name`]), then its
    /// frame in `GridDim`;
    /// then, last, the selected cell framed again in `GridBright`. No cell
    /// is filled (`_ShipyardDialogUpdate` @0x58f44-0x58f69,
    /// @0x58f9d-0x58fbf, which draws hire mode's grid as the shipyard's).
    fn draw_grid(&self, laid: &Laid, list: &mut DrawList) {
        let metrics = &*laid.metrics.0;
        let Some(area) = laid.dialog.item_bounds(GRID_ITEM) else {
            return;
        };
        let count = self.list.rows.len();
        let selected = self.selected();
        self.grid.cells().draw_cells(
            list,
            area,
            count,
            selected,
            self.colors,
            |list, index, cell| {
                let row = &self.list.rows[index];
                grid::draw_cell_picture(list, cell, self.pictures[index]);
                grid::draw_cell_name(list, metrics, cell, &row.short_name);
            },
        );
    }

    fn draw_selected(&self, laid: &Laid, list: &mut DrawList) {
        let Some(row) = self.selected_row() else {
            return;
        };
        if let (Some((_, shown)), Some(area)) = (&laid.shown, laid.dialog.item_bounds(PICTURE_ITEM))
        {
            shown.draw(list, area, &*laid.metrics.0);
        }
        let Some(info) = laid.dialog.item_bounds(INFO_BOX_ITEM) else {
            return;
        };
        draw_amount_rows(list, info, hire_lines(row, self.list.cash));
    }
}

impl Screen for HireScreen {
    /// Every input goes to the screen, or to its info panel while that is
    /// open; it never quits.
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
        if let Some(panel) = &mut self.panel {
            if panel.input(input) {
                self.panel = None;
            }
            return ScreenAction::None;
        }
        let count = self.list.rows.len();
        let arrow = match *input {
            Input::Key {
                key, pressed: true, ..
            } => self.grid.arrow(key, count),
            _ => false,
        };
        let pressed_once = |wanted: Key| matches!(*input, Input::Key { key, pressed: true, repeat: false } if key == wanted);
        let on_grid = match *input {
            Input::PointerButton {
                pressed: true, at, ..
            } => self
                .cell_under(at)
                .is_some_and(|index| self.press_cell(index)),
            _ => false,
        };
        if arrow {
            self.selected_changed();
        } else if pressed_once(HIRE_KEY) {
            self.ask();
        } else if pressed_once(INFO_KEY) {
            self.open_info();
        } else if on_grid {
            // The grid took the press.
        } else if let Ok(laid) = &mut self.laid {
            let event = laid.dialog.input(input);
            if let Some(DialogEvent::Item(item)) = event {
                self.activate(item);
            }
        }
        ScreenAction::None
    }

    /// The time since the last press on the grid runs on.
    fn tick(&mut self, dt: Duration) {
        self.grid.advance(dt);
    }

    fn draw(&self, list: &mut DrawList) {
        let laid = match &self.laid {
            Ok(laid) => laid,
            Err(reason) => {
                list.text(
                    format!(
                        "Cannot show the ships for hire: {reason}. Press Return or Escape to \
                         close it."
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
        let count = self.list.rows.len();
        grid::draw_arrows(
            list,
            laid.dialog.item_bounds(SCROLL_UP_ITEM),
            laid.dialog.item_bounds(SCROLL_DOWN_ITEM),
            self.grid.cells().can_scroll(count),
        );
        laid.dialog.draw(list);
        if let Some(panel) = &self.panel {
            panel.draw(list, &*laid.metrics.0);
        }
    }

    fn cancel_pointer(&mut self) {
        if let Some(panel) = &mut self.panel {
            panel.cancel_pointer();
        } else if let Ok(laid) = &mut self.laid {
            laid.dialog.cancel_pointer();
        }
    }

    /// The refused hire's beep ([`UiSound::BuyRefused`]); the buttons are
    /// silent.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use nova_sim::{HireRefusal, ShipFields, ShipSpecs, StockLine};

    use super::*;
    use crate::draw::{DrawCommand, frame_rect};
    use crate::font::Font;
    use crate::image::ImageKey;
    use crate::input::MouseButton;
    use crate::spaceport::catalog::{PortRecord, SpaceportCatalog, StellarId};
    use crate::spaceport::grid::TEXT_COLOR;
    use crate::spaceport::shipyard::{
        INFO_BACKGROUND, PANEL_GRAPHIC_ITEM, PICTURE_INFO_BACKGROUND, ShipBaseImages,
    };
    use crate::text::fixture::MonoMetrics;
    use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};
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
    /// thirteen items as `DITL` 1004 has them.
    fn template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(100.0, 100.0, 765.0, 323.0),
            placement: Placement::Center,
            items: items(&[
                (365.0, 289.0, 109.0, 25.0, true),
                (248.0, 440.0, 68.0, 30.0, false),
                (201.0, 380.0, 102.0, 18.0, false),
                (144.0, 438.0, 69.0, 22.0, false),
                (9.0, 8.0, 333.0, 271.0, true),
                (354.0, 10.0, 192.0, 267.0, false),
                (480.0, 289.0, 109.0, 25.0, true),
                (557.0, 8.0, 200.0, 200.0, false),
                (614.0, 214.0, 143.0, 100.0, false),
                (253.0, 289.0, 89.0, 25.0, false),
                (365.0, 431.0, 69.0, 22.0, false),
                (141.0, 288.0, 25.0, 25.0, true),
                (171.0, 288.0, 25.0, 25.0, true),
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
                (86.0, 253.0, 74.0, 25.0, true),
                (202.0, 368.0, 68.0, 30.0, false),
                (3.0, 3.0, 240.0, 24.0, false),
                (27.0, 334.0, 55.0, 31.0, false),
                (9.0, 32.0, 234.0, 214.0, false),
            ]),
        }
    }

    /// Ship 128's picture (`PICT` 5000) exists; every description is
    /// "dësc n.". Ship 141 shares 128's base image, so its own missing
    /// picture falls back to 5000. Records what it is asked.
    struct FakeArt {
        /// Each `dësc` with a `Graphic`, and it.
        graphics: Vec<(i16, i16)>,
        asked: RefCell<Vec<String>>,
    }

    impl SpaceportCatalog for FakeArt {
        fn stellar_port(&self, id: StellarId) -> Result<PortRecord, String> {
            panic!("asked for spöb {}", id.0)
        }

        fn picture_exists(&self, id: i16) -> bool {
            id == 5000
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

    impl crate::spaceport::grid::GridLooks for FakeArt {
        fn grid_colors(&self) -> crate::spaceport::grid::GridColors {
            crate::spaceport::grid::TEST_GRID_COLORS
        }
    }

    impl ShipBaseImages for FakeArt {
        fn ship_base_images(&self) -> Vec<(ShipId, i16)> {
            vec![(ShipId(128), 128), (ShipId(141), 128)]
        }
    }

    impl DescriptionGraphics for FakeArt {
        fn description_graphic(&self, id: i16) -> Option<i16> {
            self.asked.borrow_mut().push(format!("graphic {id}"));
            self.graphics
                .iter()
                .find(|(desc, _)| *desc == id)
                .map(|(_, graphic)| *graphic)
        }

        /// Names no movie: these screens never play one.
        fn description_movie(&self, _id: i16) -> Option<nova_sim::DescMovie> {
            None
        }
    }

    fn art() -> Rc<FakeArt> {
        art_with(Vec::new())
    }

    /// [`art`], with these `dësc` graphics.
    fn art_with(graphics: Vec<(i16, i16)>) -> Rc<FakeArt> {
        Rc::new(FakeArt {
            graphics,
            asked: RefCell::default(),
        })
    }

    /// Stock "Shipyard Info" with a picture: `DLOG` 1019, 614 x 537 and
    /// centred, with Done (1), the title (3), the text (5), the graphic
    /// (7) and the weapons (8); items 2, 4 and 6 parked outside it.
    fn picture_info_template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(0.0, 0.0, 614.0, 537.0),
            placement: Placement::Center,
            items: items(&[
                (503.0, 507.0, 99.0, 25.0, true),
                (700.0, 0.0, 1.0, 1.0, false),
                (7.0, 413.0, 600.0, 24.0, false),
                (700.0, 0.0, 1.0, 1.0, false),
                (11.0, 442.0, 256.0, 92.0, false),
                (700.0, 0.0, 1.0, 1.0, false),
                (7.0, 6.0, 600.0, 400.0, false),
                (266.0, 442.0, 342.0, 59.0, false),
            ]),
        }
    }

    /// Both info templates, stock.
    fn info_templates() -> InfoTemplates {
        InfoTemplates {
            plain: Ok(info_template()),
            picture: Ok(picture_info_template()),
        }
    }

    /// Ship `id`'s row, at `fee` and `wage`, hired or refused.
    fn row(id: i16, fee: i64, wage: i64, hire: Result<(), HireRefusal>) -> HireRow {
        HireRow {
            id: ShipId(id),
            name: format!("Ship {id}"),
            short_name: format!("Ship\\n{id}"),
            fee,
            wage,
            specs: ShipSpecs {
                fields: ShipFields {
                    speed: 300,
                    fuel: 300,
                    ..ShipFields::default()
                },
                max_gun: 2,
                max_tur: 1,
                length: 20,
                crew: 3,
                space: 0,
                rounding: nova_sim::RuleSource::Engine,
                weapons: Vec::new(),
                long_name: String::new(),
            },
            hire,
        }
    }

    /// The Shuttle at Viking's fee and wage, hireable, and a dear ship
    /// the cash does not cover; 25,000 credits in hand.
    fn list() -> HireList {
        HireList {
            rules: nova_sim::Rulebook::default(),
            rows: vec![
                row(128, 970, 100, Ok(())),
                row(140, 50_000, 1, Err(HireRefusal::CannotAfford)),
            ],
            cash: 25_000,
            room: true,
        }
    }

    /// The hire screen as it opens, with nothing selected by the engine.
    fn opened_of(art: &Rc<FakeArt>, list: HireList) -> HireScreen {
        let catalog: Rc<dyn ShipyardCatalog> = Rc::clone(art) as Rc<dyn ShipyardCatalog>;
        HireScreen::new(
            Ok((template(), Rc::new(MonoMetrics))),
            info_templates(),
            list,
            catalog,
            ButtonStyle::STOCK,
        )
    }

    /// The hire screen opened, then the first cell selected, as Right
    /// picks it from no selection (an arrow, so no press is remembered
    /// for a double-click).
    fn screen_of(art: &Rc<FakeArt>, list: HireList) -> HireScreen {
        let mut screen = opened_of(art, list);
        if screen.selected().is_none() {
            screen.input(&key(Key::Right, false));
        }
        screen
    }

    fn screen() -> HireScreen {
        screen_of(&art(), list())
    }

    fn drawn(screen: &HireScreen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(screen: &HireScreen) -> Vec<(String, Point, Color)> {
        drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    ..
                } => Some((text, origin, color)),
                _ => None,
            })
            .collect()
    }

    fn item(screen: &HireScreen, number: usize) -> Bounds {
        screen
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
    }

    fn key(key: Key, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat,
        }
    }

    fn click(screen: &mut HireScreen, at: Point) {
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    /// Whether `label` draws enabled.
    fn enabled(screen: &HireScreen, label: &str) -> bool {
        texts(screen)
            .into_iter()
            .find_map(|(text, _, color)| (text == label).then_some(color))
            .is_some_and(|color| color != ButtonStyle::STOCK.grey)
    }

    /// The lines drawn in the info box.
    fn info_box(screen: &HireScreen) -> Vec<String> {
        let info = item(screen, INFO_BOX_ITEM);
        texts(screen)
            .into_iter()
            .filter(|(_, at, _)| info.contains(*at))
            .map(|(text, _, _)| text)
            .collect()
    }

    /// The picture item's two missing-picture lines as drawn: centred
    /// across it (`MonoMetrics` sets Geneva 10 at 5 a character), on
    /// baselines 6 either side of the floor of its middle, in white.
    fn no_picture_lines(picture: Bounds) -> Vec<(String, Point, Color)> {
        let mid_x = f32::midpoint(picture.min.x, picture.max.x).floor();
        let mid_y = f32::midpoint(picture.min.y, picture.max.y).floor();
        vec![
            (
                "No Picture".to_owned(),
                Point::new(mid_x - 25.0, grid::baseline_top(mid_y - 6.0)),
                grid::TEXT_COLOR,
            ),
            (
                "Available".to_owned(),
                Point::new(mid_x - 22.5, grid::baseline_top(mid_y + 6.0)),
                grid::TEXT_COLOR,
            ),
        ]
    }

    #[test]
    fn a_missing_picture_shows_no_picture_and_available_centred_in_white() {
        let mut screen = screen();
        screen.input(&key(Key::Right, false));
        let picture = item(&screen, PICTURE_ITEM);
        let in_picture: Vec<_> = texts(&screen)
            .into_iter()
            .filter(|(_, at, _)| picture.contains(*at))
            .collect();
        assert_eq!(in_picture, no_picture_lines(picture));
    }

    #[test]
    fn the_named_values() {
        assert_eq!((HIRE_ITEM, HIRE_KEY), (1, Key::Char('h')));
        assert_eq!(pilot_desc_id(ShipId(128)), 14_000);
        assert_eq!(pilot_desc_id(ShipId(129)), 14_001);
        assert_eq!(pilot_desc_id(ShipId(895)), 14_767);
        assert_eq!(HIRE_ROWS, [12.0, 36.0]);
    }

    /// The texts drawn in the info box, with their origins and colours.
    fn info_box_texts(screen: &HireScreen) -> Vec<(String, Point, Color)> {
        let info = item(screen, INFO_BOX_ITEM);
        texts(screen)
            .into_iter()
            .filter(|(_, at, _)| info.contains(*at))
            .collect()
    }

    /// The four texts the info box should draw, `fee` and `cash` as
    /// given, relative to `info`'s top left.
    fn expected_box(info: Bounds, fee: &str, cash: &str) -> Vec<(String, Point, Color)> {
        let at = |x: f32, y: f32| Point::new(info.min.x + x, info.min.y + y);
        vec![
            ("Hiring Price:".to_owned(), at(0.0, 2.0), TEXT_COLOR),
            (fee.to_owned(), at(70.0, 2.0), TEXT_COLOR),
            ("You Have:".to_owned(), at(0.0, 26.0), TEXT_COLOR),
            (cash.to_owned(), at(70.0, 26.0), TEXT_COLOR),
        ]
    }

    #[test]
    fn the_info_box_draws_the_fee_and_the_cash_at_the_originals_rows_and_columns() {
        let mut screen = screen();
        let info = item(&screen, INFO_BOX_ITEM);
        assert_eq!(
            info_box_texts(&screen),
            expected_box(info, "970 cr", "25,000 cr")
        );
        assert!(
            texts(&screen)
                .iter()
                .all(|(text, _, _)| !text.starts_with("Pay:") && !text.contains("per day")),
            "no Pay line"
        );
        screen.input(&key(Key::Right, false));
        assert_eq!(
            screen.selected(),
            Some(1),
            "the row that cannot be afforded"
        );
        assert_eq!(
            info_box_texts(&screen),
            expected_box(info, "50,000 cr", "25,000 cr")
        );
    }

    #[test]
    fn the_amounts_draw_as_draw_comma_number_does() {
        let line =
            |baseline: f32, label: &'static str, amount: &str| (baseline, label, amount.to_owned());
        assert_eq!(
            hire_lines(&row(128, 1_234_567, 1, Ok(())), -1_500),
            [
                line(12.0, "Hiring Price:", "1.23M cr"),
                line(36.0, "You Have:", "-1500 cr")
            ]
        );
        assert_eq!(
            hire_lines(&row(128, 999, 1, Ok(())), 1_000),
            [
                line(12.0, "Hiring Price:", "999 cr"),
                line(36.0, "You Have:", "1,000 cr")
            ]
        );
        assert_eq!(
            hire_lines(&row(128, 0, 0, Ok(())), 0),
            [
                line(12.0, "Hiring Price:", "0 cr"),
                line(36.0, "You Have:", "0 cr")
            ]
        );
        let screen = screen_of(
            &art(),
            HireList {
                rules: nova_sim::Rulebook::default(),
                rows: vec![row(128, 2_500_000, 100, Ok(()))],
                cash: -1_500,
                room: true,
            },
        );
        let info = item(&screen, INFO_BOX_ITEM);
        assert_eq!(
            info_box_texts(&screen),
            expected_box(info, "2.50M cr", "-1500 cr")
        );
    }

    #[test]
    fn hire_escort_is_greyed_for_a_ship_that_cannot_be_hired() {
        let mut screen = screen();
        assert!(enabled(&screen, HIRE_ESCORT));
        let second = screen.cell_bounds(1).expect("a cell").center();
        click(&mut screen, second);
        assert_eq!(screen.selected(), Some(1));
        assert!(!enabled(&screen, HIRE_ESCORT));
        screen.input(&key(HIRE_KEY, false));
        assert_eq!(screen.take_order(), None, "refused");
        let hire = item(&screen, HIRE_ITEM).center();
        click(&mut screen, hire);
        assert_eq!(screen.take_order(), None);
    }

    #[test]
    fn h_or_hire_escort_asks_for_the_selected_ship_on_a_press_only() {
        let mut screen = screen();
        screen.input(&key(HIRE_KEY, true));
        assert_eq!(screen.take_order(), None, "a repeat");
        screen.input(&key(HIRE_KEY, false));
        assert_eq!(screen.take_order(), Some(ShipId(128)));
        assert_eq!(screen.take_order(), None, "once");
        let hire = item(&screen, HIRE_ITEM).center();
        click(&mut screen, hire);
        assert_eq!(screen.take_order(), Some(ShipId(128)));
        assert!(!screen.closed(), "the spaceport closes it");
    }

    #[test]
    fn the_selected_ship_shows_its_pilots_description_and_its_picture() {
        let art = art();
        let screen = screen_of(&art, list());
        assert_eq!(*art.asked.borrow(), ["dësc 14000"]);
        let picture = item(&screen, PICTURE_ITEM).min;
        assert!(
            drawn(&screen).iter().any(|command| matches!(
                command,
                DrawCommand::StretchedPicture { image, top_left, .. }
                    if image.id == 5000 && *top_left == picture
            )),
            "the picture item, not just the grid cell, shows PICT 5000"
        );
        let mut screen = screen;
        screen.input(&key(Key::Right, false));
        assert_eq!(*art.asked.borrow(), ["dësc 14000", "dësc 14012"]);
    }

    #[test]
    fn the_pilots_description_is_asked_to_load_and_shows_loaded() {
        let mut screen = screen();
        assert_eq!(screen.take_description_request(), Some(14000));
        assert_eq!(screen.take_description_request(), None, "once");
        screen.show_description(14000, "Loaded pilot.");
        let words: Vec<String> = texts(&screen).into_iter().map(|(t, _, _)| t).collect();
        assert!(words.contains(&"Loaded pilot.".to_owned()), "{words:?}");
        screen.show_description(14012, "Another.");
        let words: Vec<String> = texts(&screen).into_iter().map(|(t, _, _)| t).collect();
        assert!(
            !words.contains(&"Another.".to_owned()),
            "not the ship shown"
        );
    }

    #[test]
    fn the_grid_shows_each_ships_short_name_with_no_current_mark() {
        let words: Vec<String> = texts(&screen()).into_iter().map(|(t, _, _)| t).collect();
        for line in ["Ship", "128", "140"] {
            assert!(words.contains(&line.to_owned()), "{line}: {words:?}");
        }
        assert!(!words.contains(&"(current)".to_owned()));
    }

    /// `_DrawShipyardButtons` @0x2a3bf-0x2a3dc labels item 1 Hire Escort in
    /// hire mode and item 7 Done; `_DoShipyardDialog` hires on 1 (@0x5e9b0)
    /// and exits on 7 (@0x5f6bd).
    #[test]
    fn hire_escort_is_item_1_and_done_is_item_7() {
        let mut screen = screen();
        let labels = texts(&screen);
        for (label, number) in [(HIRE_ESCORT, 1), ("Done", 7)] {
            let bounds = item(&screen, number);
            assert!(
                labels
                    .iter()
                    .any(|(t, at, _)| t == label && bounds.contains(*at)),
                "{label} in item {number}: {labels:?}"
            );
        }
        let hire = item(&screen, 1).center();
        click(&mut screen, hire);
        assert_eq!(screen.take_order(), Some(ShipId(128)));
        assert!(!screen.closed());
        let done = item(&screen, 7).center();
        click(&mut screen, done);
        assert!(screen.closed());
    }

    #[test]
    fn done_return_and_escape_close_it() {
        for close in [Key::Enter, Key::Escape] {
            let mut screen = screen();
            screen.input(&key(close, false));
            assert!(screen.closed(), "{close:?}");
        }
        let mut screen = screen();
        let done = item(&screen, DONE_ITEM).center();
        click(&mut screen, done);
        assert!(screen.closed());
    }

    #[test]
    fn info_opens_the_panel_on_the_selected_ship_and_its_done_closes_it_alone() {
        let mut screen = screen();
        screen.input(&key(INFO_KEY, false));
        assert!(screen.info_panel().is_some());
        let words: Vec<String> = texts(&screen).into_iter().map(|(t, _, _)| t).collect();
        assert!(words.contains(&"Ship 128".to_owned()), "{words:?}");
        assert!(words.contains(&"Speed:".to_owned()), "{words:?}");
        assert!(words.contains(&"300".to_owned()), "{words:?}");
        screen.input(&key(HIRE_KEY, false));
        assert_eq!(screen.take_order(), None, "the panel takes the keys");
        screen.input(&key(Key::Escape, false));
        assert!(screen.info_panel().is_none());
        assert!(!screen.closed());
        // Ship 128's pilot `dësc` 14000 has a graphic whose picture
        // dialog is there: greyed Info still opens nothing for it.
        for art in [art(), art_with(vec![(14_000, 5000)])] {
            let mut bare = HireScreen::new(
                Ok((template(), Rc::new(MonoMetrics))),
                InfoTemplates {
                    plain: Err("no DLOG 1005".to_owned()),
                    picture: Ok(picture_info_template()),
                },
                list(),
                art,
                ButtonStyle::STOCK,
            );
            bare.input(&key(Key::Right, false));
            assert!(!enabled(&bare, INFO_LABEL));
            assert_eq!(bare.selected(), Some(0));
            bare.input(&key(INFO_KEY, false));
            let info = item(&bare, INFO_ITEM).center();
            click(&mut bare, info);
            assert!(bare.info_panel().is_none());
        }
    }

    #[test]
    fn the_hire_panel_draws_the_two_columns_and_the_stock_weapons() {
        let mut armed = row(128, 970, 100, Ok(()));
        armed.specs.weapons = vec![StockLine {
            count: 2,
            name: "Light Blaster".to_owned(),
            ammo: None,
        }];
        let panel_texts = |row: HireRow| {
            let mut screen = screen_of(
                &art(),
                HireList {
                    rules: nova_sim::Rulebook::default(),
                    rows: vec![row],
                    ..list()
                },
            );
            screen.input(&key(INFO_KEY, false));
            let area = screen
                .info_panel()
                .expect("open")
                .item_bounds(5)
                .expect("item 5");
            (texts(&screen), area.min)
        };
        let (drawn, at) = panel_texts(armed);
        let white = Color::WHITE;
        for expected in [
            (
                "Standard Weapons:",
                Point::new(at.x, at.y + 108.0 - 10.0),
                white,
            ),
            (
                "2 Light Blasters",
                Point::new(at.x + 8.0, at.y + 120.0 - 10.0),
                white,
            ),
            (
                "Speed:",
                Point::new(at.x, at.y + 2.0),
                Color::rgba(128, 128, 128, 255),
            ),
            ("300", Point::new(at.x + 45.0, at.y + 2.0), white),
        ] {
            let expected = (expected.0.to_owned(), expected.1, expected.2);
            assert!(drawn.contains(&expected), "{expected:?}: {drawn:?}");
        }
        let (bare, _) = panel_texts(row(128, 970, 100, Ok(())));
        assert!(
            bare.iter()
                .any(|(text, _, _)| text == "No standard weapons"),
            "{bare:?}"
        );
    }

    #[test]
    fn without_its_dialog_it_says_why_and_return_closes_it() {
        let mut screen = HireScreen::new(
            Err("no DLOG 1004".to_owned()),
            info_templates(),
            list(),
            art(),
            ButtonStyle::STOCK,
        );
        assert_eq!(screen.problem(), Some("no DLOG 1004"));
        assert!(
            texts(&screen)
                .iter()
                .any(|(text, _, _)| text.contains("no DLOG 1004"))
        );
        screen.input(&key(HIRE_KEY, false));
        assert!(!screen.closed());
        screen.input(&key(Key::Enter, false));
        assert!(screen.closed());
    }

    #[test]
    fn its_list_is_the_one_given() {
        let screen = screen();
        assert_eq!(screen.list(), &list());
        assert_eq!(screen.selected(), Some(0));
        let empty = screen_of(
            &art(),
            HireList {
                rules: nova_sim::Rulebook::default(),
                rows: Vec::new(),
                cash: 0,
                room: true,
            },
        );
        assert_eq!(empty.selected(), None);
        assert!(!enabled(&empty, HIRE_ESCORT));
        assert_eq!(info_box(&empty), Vec::<String>::new());
    }

    /// `count` ships, numbered from 200, two lines each.
    fn long(count: usize) -> HireList {
        HireList {
            rules: nova_sim::Rulebook::default(),
            rows: (0..count)
                .map(|n| row(200 + i16::try_from(n).expect("few"), 10, 1, Ok(())))
                .collect(),
            ..list()
        }
    }

    fn click_item(screen: &mut HireScreen, number: usize) {
        let at = item(screen, number).center();
        click(screen, at);
    }

    #[test]
    fn the_grid_draws_each_cell_as_the_shipyard_does_and_highlights_the_selected() {
        let art = art();
        let screen = screen_of(&art, list());
        let commands = drawn(&screen);
        for index in [0, 1] {
            let row = &screen.list.rows[index];
            let cell = screen.cell_bounds(index).expect("a cell");
            let mut expected = DrawList::new();
            let picture =
                ship_picture(row.id, &art.ship_base_images(), |id| art.picture_exists(id));
            grid::draw_cell_picture(&mut expected, cell, picture);
            grid::draw_cell_name(&mut expected, &MonoMetrics, cell, &row.short_name);
            let expected: Vec<DrawCommand> = expected.iter().cloned().collect();
            assert!(contains(&commands, &expected), "cell {index}: {commands:?}");
            let top_left = Point::new(cell.min.x + grid::INSET, cell.min.y + grid::INSET);
            assert!(
                !texts(&screen)
                    .iter()
                    .any(|(_, origin, _)| *origin == top_left),
                "cell {index} names at its top-left"
            );
        }
        assert!(contains(&commands, &bright_frame(&screen, 0)));
        assert!(!contains(&commands, &bright_frame(&screen, 1)), "one");
        assert!(contains(&commands, &frame(&screen, 1)));
    }

    #[test]
    fn a_ship_without_its_own_picture_shows_its_bases_in_the_grid_and_the_picture_item() {
        let art = art();
        let mut hires = list();
        hires.rows.push(row(141, 900, 1, Ok(())));
        let mut screen = screen_of(&art, hires);
        let bases = art.ship_base_images();
        assert!(!art.picture_exists(5013), "141's own picture is missing");
        assert_eq!(
            ship_picture(ShipId(141), &bases, |id| art.picture_exists(id)),
            Some(5000)
        );
        let cell = screen.cell_bounds(2).expect("a cell");
        let mut expected = DrawList::new();
        grid::draw_cell_picture(&mut expected, cell, Some(5000));
        let expected: Vec<DrawCommand> = expected.iter().cloned().collect();
        assert!(
            contains(&drawn(&screen), &expected),
            "the grid cell shows PICT 5000"
        );
        screen.input(&key(Key::Right, false));
        screen.input(&key(Key::Right, false));
        assert_eq!(screen.selected(), Some(2));
        let picture = item(&screen, PICTURE_ITEM).min;
        assert!(
            drawn(&screen).iter().any(|command| matches!(
                command,
                DrawCommand::StretchedPicture { image, top_left, .. }
                    if image.id == 5000 && *top_left == picture
            )),
            "the picture item shows the base's PICT 5000"
        );
    }

    /// Shown cell `index`'s frame in `color`.
    fn framed(screen: &HireScreen, index: usize, color: Color) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        frame_rect(&mut list, screen.cell_bounds(index).expect("a cell"), color);
        list.iter().cloned().collect()
    }

    /// Shown cell `index`'s frame in the fake's `GridDim`.
    fn frame(screen: &HireScreen, index: usize) -> Vec<DrawCommand> {
        framed(screen, index, grid::TEST_GRID_COLORS.dim)
    }

    /// Shown cell `index`'s selection: framed in the fake's `GridBright`.
    fn bright_frame(screen: &HireScreen, index: usize) -> Vec<DrawCommand> {
        framed(screen, index, grid::TEST_GRID_COLORS.bright)
    }

    fn contains(commands: &[DrawCommand], part: &[DrawCommand]) -> bool {
        commands.windows(part.len()).any(|window| window == part)
    }

    /// The frame rule itself is `grid::CellGrid::draw_cells`'s, tested
    /// there; this checks the hire screen hands it its grid item, rows,
    /// selection and `cölr` colours.
    #[test]
    fn the_grid_is_drawn_by_the_shared_cell_rule_on_its_item_in_the_catalogs_colours() {
        let mut screen = screen_of(&art(), long(22));
        let grid_item = item(&screen, GRID_ITEM);
        let commands = drawn(&screen);
        for index in 0..20 {
            assert_eq!(
                screen.cell_bounds(index),
                Some(grid::cell_bounds(grid_item, index))
            );
            assert!(contains(&commands, &frame(&screen, index)), "{index}");
        }
        assert!(contains(&commands, &bright_frame(&screen, 0)), "selected");
        screen.input(&key(Key::Right, false));
        let commands = drawn(&screen);
        assert!(!contains(&commands, &bright_frame(&screen, 0)));
        assert!(contains(&commands, &bright_frame(&screen, 1)));
    }

    #[test]
    fn a_press_is_hit_tested_on_the_84_by_55_division() {
        let mut screen = screen_of(&art(), long(22));
        let grid = item(&screen, GRID_ITEM);
        click(
            &mut screen,
            Point::new(grid.min.x + 84.0 + 1.0, grid.min.y + 1.0),
        );
        assert_eq!(screen.selected(), Some(1));
        click(
            &mut screen,
            Point::new(grid.min.x + 83.0, grid.min.y + 54.0),
        );
        assert_eq!(screen.selected(), Some(0), "83 right, 54 down is cell 0");
        click(
            &mut screen,
            Point::new(grid.min.x + 84.0, grid.min.y + 55.0),
        );
        assert_eq!(screen.selected(), Some(5));
        click(&mut screen, Point::new(grid.max.x - 0.5, grid.max.y - 0.5));
        assert_eq!(
            screen.selected(),
            Some(19),
            "the last cell reaches the corner"
        );
        click(&mut screen, Point::new(grid.max.x, grid.min.y + 1.0));
        assert_eq!(screen.selected(), Some(19), "the right edge is out");
        click(&mut screen, Point::new(grid.min.x + 1.0, grid.max.y));
        assert_eq!(screen.selected(), Some(19), "the bottom edge is out");
    }

    #[test]
    fn the_arrows_scroll_the_grid_a_row_at_a_time() {
        let mut screen = screen_of(&art(), long(22));
        assert_eq!(screen.top_row(), 0);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 1);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 1, "never past the end");
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!(screen.top_row(), 0);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 1);
        screen.input(&key(Key::Down, false));
        assert_eq!(screen.selected(), Some(4));
    }

    /// Ship 128's row, its `Long Name` "The Long 128", alone in the list.
    fn long_named() -> HireList {
        let mut row = row(128, 970, 100, Ok(()));
        row.specs.long_name = "The Long 128".to_owned();
        HireList {
            rules: nova_sim::Rulebook::default(),
            rows: vec![row],
            ..list()
        }
    }

    #[test]
    fn info_on_a_pilot_description_with_a_graphic_opens_the_picture_panel() {
        let art = art_with(vec![(13_000, 6000), (14_000, 5000)]);
        let mut screen = screen_of(&art, long_named());
        art.asked.borrow_mut().clear();
        screen.input(&key(INFO_KEY, false));
        let panel = screen.info_panel().expect("open");
        assert_eq!(panel.bounds(), rect(205.0, 115.0, 614.0, 537.0));
        assert_eq!(*art.asked.borrow(), ["graphic 14000"]);
        let graphic = panel.item_bounds(PANEL_GRAPHIC_ITEM).expect("item 7");
        let commands = drawn(&screen);
        assert!(commands.contains(&DrawCommand::Picture {
            image: PICTURE_INFO_BACKGROUND,
            top_left: Point::new(205.0, 115.0),
        }));
        assert!(commands.contains(&DrawCommand::StretchedPicture {
            image: ImageKey::picture(5000),
            top_left: graphic.min,
            width: 600.0,
            height: 400.0,
        }));
        let words: Vec<String> = texts(&screen).into_iter().map(|(t, _, _)| t).collect();
        assert!(words.contains(&"The Long 128".to_owned()), "{words:?}");
        assert!(!words.contains(&"Ship 128".to_owned()), "{words:?}");
    }

    #[test]
    fn info_on_a_pilot_description_without_a_graphic_opens_1005() {
        let art = art_with(vec![(13_000, 5000)]);
        let mut screen = screen_of(&art, long_named());
        screen.input(&key(INFO_KEY, false));
        let panel = screen.info_panel().expect("open");
        assert_eq!(panel.bounds(), rect(387.0, 241.0, 250.0, 285.0));
        assert!(drawn(&screen).contains(&DrawCommand::Picture {
            image: INFO_BACKGROUND,
            top_left: Point::new(387.0, 241.0),
        }));
        let words: Vec<String> = texts(&screen).into_iter().map(|(t, _, _)| t).collect();
        assert!(words.contains(&"Ship 128".to_owned()), "{words:?}");
        assert!(!words.contains(&"The Long 128".to_owned()), "{words:?}");
    }

    #[test]
    fn the_hire_picture_panel_draws_the_weapons_sentence() {
        let art = art_with(vec![(14_000, 5000)]);
        let mut list = long_named();
        list.rows[0].specs.weapons = vec![StockLine {
            count: 2,
            name: "Light Blaster".to_owned(),
            ammo: None,
        }];
        let mut screen = screen_of(&art, list);
        screen.input(&key(INFO_KEY, false));
        let panel = screen.info_panel().expect("open");
        let weapons = panel.item_bounds(8).expect("item 8");
        let text_area = panel.item_bounds(5).expect("item 5");
        let drawn = texts(&screen);
        let white = Color::WHITE;
        for expected in [
            (
                "Standard Weapons:",
                Point::new(weapons.min.x, weapons.min.y + 2.0),
            ),
            (
                "2 Light Blasters",
                Point::new(weapons.min.x, weapons.min.y + 16.0),
            ),
        ] {
            let expected = (expected.0.to_owned(), expected.1, white);
            assert!(drawn.contains(&expected), "{expected:?}: {drawn:?}");
        }
        let item_5s_heading = Point::new(text_area.min.x, text_area.min.y + 98.0);
        assert!(
            !drawn.iter().any(|(_, at, _)| *at == item_5s_heading),
            "{drawn:?}"
        );
    }

    #[test]
    fn the_hire_panel_title_is_times_18() {
        let art = art_with(vec![(14_000, 5000)]);
        let mut screen = screen_of(&art, long_named());
        screen.input(&key(INFO_KEY, false));
        let area = screen
            .info_panel()
            .expect("open")
            .item_bounds(3)
            .expect("item 3");
        let middle = f32::midpoint(area.min.x, area.max.x).floor();
        let width = MonoMetrics.width(Font::Times, 18.0, "The Long 128");
        let title = drawn(&screen)
            .into_iter()
            .find_map(|c| match c {
                DrawCommand::Text {
                    text,
                    font,
                    size,
                    origin,
                    color,
                    ..
                } if text == "The Long 128" => Some((font, size, origin, color)),
                _ => None,
            })
            .expect("the title");
        assert_eq!(
            title,
            (
                Font::Times,
                18.0,
                Point::new(middle - width / 2.0, area.min.y),
                Color::WHITE
            )
        );
    }

    #[test]
    fn info_clicked_opens_the_panel() {
        let mut screen = screen();
        click_item(&mut screen, INFO_ITEM);
        assert!(screen.info_panel().is_some());
        assert!(!screen.closed());
    }

    #[test]
    fn its_debug_names_what_it_holds() {
        let debug = format!("{:?}", screen());
        for part in ["HireScreen", "ShipyardCatalog", "TextMetrics", "Ship 128"] {
            assert!(debug.contains(part), "{part}: {debug}");
        }
    }

    /// A pointer click plays nothing: `_TrackEscortButtons` @0x27987 calls
    /// no sound routine.
    #[test]
    fn pointer_click_is_silent_as_the_original() {
        let mut screen = screen();
        let hire = item(&screen, HIRE_ITEM).center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: hire,
            });
            assert_eq!(screen.take_sounds(), [], "pressed: {pressed}");
        }
        assert_eq!(screen.take_order(), Some(ShipId(128)), "it still hires");
    }

    /// The original's hire mode of `_DoShipyardDialog` maps keys in `_ShipyardFilter` @0x60449 to items with no click sound.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        for k in [
            Key::Right,
            Key::Down,
            Key::Left,
            Key::Up,
            HIRE_KEY,
            INFO_KEY,
            Key::Enter,
            Key::Escape,
        ] {
            let mut screen = screen();
            for pressed in [true, false] {
                screen.input(&Input::Key {
                    key: k,
                    pressed,
                    repeat: false,
                });
            }
            assert_eq!(screen.take_sounds(), [], "{k:?} makes no click");
        }
    }

    // The selection as `_ShipyardFilter` keeps it in hire mode
    // (`RuleKey::GridSelection`).

    /// The Bible's reading of the grid's selection: the port's, where the
    /// selection is the highlighted cell.
    fn bible() -> nova_sim::Rulebook {
        nova_sim::Rulebook::default().with_override(
            nova_sim::RuleKey::GridSelection,
            nova_sim::RuleSource::Bible,
        )
    }

    fn click_cell(screen: &mut HireScreen, index: usize) {
        let at = screen.cell_bounds(index).expect("a cell").center();
        click(screen, at);
    }

    /// Whether the screen shows no ship: Hire Escort and Info greyed, and
    /// no cell bright, no picture, no info box and no description drawn.
    fn shows_no_ship(screen: &HireScreen) -> bool {
        let commands = drawn(screen);
        let bright = (0..20).any(|index| contains(&commands, &bright_frame(screen, index)));
        let picture = item(screen, PICTURE_ITEM);
        let pictured = commands.iter().any(|c| {
            matches!(c, DrawCommand::StretchedPicture { top_left, .. } if *top_left == picture.min)
        }) || texts(screen).iter().any(|(_, at, _)| picture.contains(*at));
        !enabled(screen, HIRE_ESCORT)
            && !enabled(screen, INFO_LABEL)
            && !bright
            && !pictured
            && info_box(screen).is_empty()
            && !texts(screen).iter().any(|(w, _, _)| w.starts_with("dësc"))
    }

    #[test]
    fn by_the_engine_it_opens_with_nothing_selected() {
        let art = art();
        let mut screen = opened_of(&art, list());
        assert_eq!(screen.selected(), None);
        assert!(shows_no_ship(&screen));
        assert_eq!(screen.take_description_request(), None, "none to load");
        assert!(art.asked.borrow().is_empty(), "no description read");
    }

    #[test]
    fn by_the_engine_an_empty_cell_pressed_selects_nothing() {
        let mut screen = opened_of(&art(), list());
        click_cell(&mut screen, 0);
        assert_eq!(screen.selected(), Some(0));
        assert!(!shows_no_ship(&screen));
        click_cell(&mut screen, 2);
        assert_eq!(screen.selected(), None, "the empty position past the last");
        assert!(shows_no_ship(&screen));
        assert_eq!(screen.take_order(), None, "it reaches no button");
        assert!(!screen.closed());
        assert_eq!(screen.take_sounds(), [], "silent");
    }

    #[test]
    fn by_the_engine_a_scroll_that_moves_selects_nothing() {
        let mut screen = opened_of(&art(), long(30));
        click_cell(&mut screen, 5);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!((screen.top_row(), screen.selected()), (1, None));
        assert!(shows_no_ship(&screen));
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        click_cell(&mut screen, 2);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(
            (screen.top_row(), screen.selected()),
            (3, Some(14)),
            "at the bottom it neither scrolls nor clears"
        );
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!((screen.top_row(), screen.selected()), (2, None));
    }

    #[test]
    fn by_the_other_reading_it_opens_on_the_first_cell_and_keeps_it() {
        let mut screen = opened_of(
            &art(),
            HireList {
                rules: bible(),
                ..long(30)
            },
        );
        assert_eq!(screen.selected(), Some(0), "the first cell");
        click_cell(&mut screen, 5);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(
            (screen.top_row(), screen.selected()),
            (1, Some(5)),
            "a scroll keeps it"
        );
        let mut short = opened_of(
            &art(),
            HireList {
                rules: bible(),
                ..list()
            },
        );
        click_cell(&mut short, 0);
        click_cell(&mut short, 2);
        assert_eq!(short.selected(), Some(0), "an empty cell is ignored");
        assert!(enabled(&short, HIRE_ESCORT));
    }

    #[test]
    fn a_second_press_on_a_cell_within_15_mac_ticks_opens_info_by_either_reading() {
        use nova_sim::mission::screens::mac_ticks;
        for rules in [nova_sim::Rulebook::default(), bible()] {
            let opened = || opened_of(&art(), HireList { rules, ..list() });
            let mut screen = opened();
            click_cell(&mut screen, 1);
            assert!(screen.info_panel().is_none(), "one press");
            screen.tick(mac_ticks(15));
            click_cell(&mut screen, 1);
            assert!(screen.info_panel().is_some(), "{rules:?}");
            assert!(texts(&screen).iter().any(|(w, _, _)| w == "Ship 140"));

            let mut slow = opened();
            click_cell(&mut slow, 1);
            slow.tick(mac_ticks(16));
            click_cell(&mut slow, 1);
            assert!(slow.info_panel().is_none(), "16 ticks apart");

            let mut broken = opened();
            click_cell(&mut broken, 1);
            click_cell(&mut broken, 2);
            click_cell(&mut broken, 1);
            assert!(broken.info_panel().is_none(), "an empty press between");
        }
    }

    #[test]
    fn a_double_click_is_by_position_so_one_across_a_scroll_opens_the_ship_now_shown() {
        let mut screen = opened_of(&art(), long(30));
        click_cell(&mut screen, 5);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert!(screen.info_panel().is_none());
        click_cell(&mut screen, 5);
        assert_eq!(screen.selected(), Some(9), "the ship now at position 5");
        assert!(screen.info_panel().is_some());
        assert!(texts(&screen).iter().any(|(w, _, _)| w == "Ship 209"));
    }

    #[test]
    fn h_refused_beeps_and_space_with_nothing_selected_is_silent() {
        let mut screen = opened_of(&art(), list());
        press_key(&mut screen, HIRE_KEY);
        assert_eq!(screen.take_order(), None, "nothing selected");
        assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::BuyRefused)]);
        press_key(&mut screen, INFO_KEY);
        assert!(screen.info_panel().is_none());
        assert_eq!(screen.take_sounds(), [], "Info with no ship is silent");
        for rules in [nova_sim::Rulebook::default(), bible()] {
            let mut screen = opened_of(&art(), HireList { rules, ..list() });
            click_cell(&mut screen, 1);
            press_key(&mut screen, HIRE_KEY);
            assert_eq!(screen.take_order(), None, "too dear");
            assert_eq!(
                screen.take_sounds(),
                [Sound::Ui(UiSound::BuyRefused)],
                "{rules:?}"
            );
        }
    }

    #[test]
    fn by_the_engine_an_arrow_from_nothing_selected_picks_the_first_or_last_shown() {
        let mut screen = opened_of(&art(), long(30));
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        press_key(&mut screen, Key::Left);
        assert_eq!(screen.selected(), Some(23), "the last shown");
        let mut short = opened_of(&art(), list());
        press_key(&mut short, Key::Down);
        assert_eq!(short.selected(), Some(0), "the first shown");
        assert!(enabled(&short, HIRE_ESCORT));
    }

    fn press_key(screen: &mut HireScreen, k: Key) {
        screen.input(&key(k, false));
    }

    #[test]
    fn by_the_engine_down_onto_an_empty_cell_and_up_on_the_first_row_stay() {
        // Six escorts, the second too dear: Down from 3 finds index 7
        // empty, and Up from 5 reaches 1, on the first row.
        let six = |rules: nova_sim::Rulebook| {
            let mut list = long(6);
            list.rows[1].hire = Err(HireRefusal::CannotAfford);
            HireList { rules, ..list }
        };
        let arrowed = |rules: nova_sim::Rulebook, from: usize, keys: &[Key]| {
            let mut screen = opened_of(&art(), six(rules));
            click_cell(&mut screen, from);
            for &k in keys {
                press_key(&mut screen, k);
            }
            (screen.selected(), enabled(&screen, HIRE_ESCORT))
        };
        let engine = nova_sim::Rulebook::default;
        assert_eq!(arrowed(engine(), 3, &[Key::Down]), (Some(3), true));
        assert_eq!(arrowed(engine(), 3, &[Key::PageDown]), (Some(3), true));
        assert_eq!(arrowed(engine(), 5, &[Key::Up]), (Some(1), false));
        assert_eq!(arrowed(engine(), 5, &[Key::Up, Key::Up]), (Some(1), false));
        assert_eq!(
            arrowed(engine(), 5, &[Key::PageUp, Key::PageUp]),
            (Some(1), false)
        );
        assert_eq!(arrowed(engine(), 1, &[Key::PageDown]), (Some(5), true));
        let clamped = || {
            nova_sim::Rulebook::default()
                .with_override(nova_sim::RuleKey::GridArrows, nova_sim::RuleSource::Bible)
        };
        assert_eq!(arrowed(clamped(), 3, &[Key::Down]), (Some(5), true));
        assert_eq!(arrowed(clamped(), 5, &[Key::Up, Key::Up]), (Some(0), true));
    }
}
