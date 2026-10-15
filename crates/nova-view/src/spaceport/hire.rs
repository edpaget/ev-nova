//! The hire screen: the ships for hire in the bar, laid out by the
//! interface file's "Shipyard" dialog (`DLOG`/`DITL` 1004) over its
//! background picture, as the original opens its shipyard dialog in hire
//! mode (`_DoShipyardDialog` with `_shipBuyMode` 1).
//!
//! It is the shipyard screen's twin, built from the same pieces: the grid
//! of ships ([`grid`](super::grid)), each cell its `ShortName`'s lines; the
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
//! Hire Escort (item 7, `STR#` 150 #13), clicked or with H, asks for the
//! selected ship, and is greyed when it cannot be hired. H acts on a
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
use super::grid::{
    self, CellGrid, GREY, INSET, SELECTED_COLOR, Shown, TEXT_FONT, TEXT_SIZE, name_lines, text,
};
use super::layout::DONE_LABEL;
use super::shipyard::{
    BACKGROUND, BUY_ITEM, DESCRIPTION_ITEM, DONE_ITEM, GRID_ITEM, INFO_BOX_ITEM, INFO_ITEM,
    INFO_KEY, INFO_LABEL, InfoTemplates, PICTURE_ITEM, Panel, SCROLL_DOWN_ITEM, SCROLL_UP_ITEM,
    ShipyardCatalog, open_panel, ship_picture,
};
use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, Role, outline};

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
    /// Each ship's base image, read once.
    bases: Vec<(ShipId, i16)>,
    style: ButtonStyle,
    grid: CellGrid,
    /// The info panel, while it is open.
    panel: Option<Panel>,
    /// The ship asked for, until it is taken.
    order: Option<ShipId>,
    closed: bool,
    sounds: Vec<Sound>,
}

impl HireScreen {
    /// The ships `list` for hire, laid out by `layout`: the "Shipyard"
    /// dialog's template and the metrics its text is measured by, or why
    /// there are none; the info panel by `info`, "Shipyard Info" (Info is
    /// greyed without it). Its buttons are labelled in `style`, and each
    /// ship's picture and description read from `catalog`, which gives
    /// the ships' base images once, now. The first cell is selected.
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
        let bases = if laid.is_ok() {
            catalog.ship_base_images()
        } else {
            Vec::new()
        };
        let mut screen = Self {
            laid,
            list,
            catalog: CatalogHandle(catalog),
            bases,
            style,
            grid: CellGrid::default(),
            panel: None,
            order: None,
            closed: false,
            sounds: Vec::new(),
        };
        screen.select(0);
        screen
    }

    /// The ships shown.
    #[must_use]
    pub fn list(&self) -> &HireList {
        &self.list
    }

    /// The selected cell's index, if anything is listed.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.grid.selected(self.list.rows.len())
    }

    /// The selected ship's row, if anything is listed.
    fn selected_row(&self) -> Option<&HireRow> {
        self.selected().map(|index| &self.list.rows[index])
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

    /// Selects cell `index`, or the last when there is no such cell, and
    /// scrolls the grid to show it.
    fn select(&mut self, index: usize) {
        self.grid.select(index, self.list.rows.len());
        self.selected_changed();
    }

    /// Greys Hire Escort when the selected ship cannot be hired, and
    /// reads its picture and description if they are not shown already.
    fn selected_changed(&mut self) {
        let hires = self.selected_row().is_some_and(|row| row.hire.is_ok());
        let ship = self.selected_row().map(|row| row.id);
        let catalog = Rc::clone(&self.catalog.0);
        let Ok(laid) = &mut self.laid else {
            return;
        };
        laid.dialog.set_greyed(HIRE_ITEM, !hires);
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
        let description = Some(pilot_desc_id(ship));
        let shown = Shown::read(&*catalog, picture, description, area, &laid.metrics.0);
        laid.shown = Some((ship, shown));
    }

    /// Asks for the selected ship, if it can be hired.
    fn ask(&mut self) {
        if let Some(row) = self.selected_row().filter(|row| row.hire.is_ok()) {
            self.order = Some(row.id);
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
        let count = self.list.rows.len();
        match item {
            DONE_ITEM => self.closed = true,
            HIRE_ITEM => self.ask(),
            INFO_ITEM => self.open_info(),
            SCROLL_UP_ITEM => self.grid.scroll(false, count),
            SCROLL_DOWN_ITEM => self.grid.scroll(true, count),
            _ => {}
        }
    }

    /// The cell index under `at` on the grid, if a listed ship is there.
    fn cell_at(&self, at: Point) -> Option<usize> {
        let grid = self.dialog()?.item_bounds(GRID_ITEM)?;
        self.grid.cell_at(grid, at, self.list.rows.len())
    }

    fn draw_grid(&self, laid: &Laid, list: &mut DrawList) {
        let line_height = laid.metrics.0.line_height(TEXT_FONT, TEXT_SIZE);
        let Some(area) = laid.dialog.item_bounds(GRID_ITEM) else {
            return;
        };
        for (index, cell) in self.grid.shown(area, self.list.rows.len()) {
            let row = &self.list.rows[index];
            if Some(index) == self.selected() {
                fill_rect(list, cell, SELECTED_COLOR);
            }
            outline(list, cell, GREY);
            let mut y = cell.min.y + INSET;
            for (line, color) in name_lines(&row.short_name) {
                text(list, line, Point::new(cell.min.x + INSET, y), color);
                y += line_height;
            }
        }
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
            if panel.input(input, &mut self.sounds) {
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
        if arrow {
            self.selected_changed();
        } else if pressed_once(HIRE_KEY) {
            self.ask();
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
            self.grid.can_scroll(count),
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

    /// The buttons' sounds, in order.
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
    use crate::draw::{DrawCommand, fill_rect};
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
    /// "dësc n.". Records what it is asked.
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

    impl ShipBaseImages for FakeArt {
        fn ship_base_images(&self) -> Vec<(ShipId, i16)> {
            Vec::new()
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
            rows: vec![
                row(128, 970, 100, Ok(())),
                row(140, 50_000, 1, Err(HireRefusal::CannotAfford)),
            ],
            cash: 25_000,
            room: true,
        }
    }

    fn screen_of(art: &Rc<FakeArt>, list: HireList) -> HireScreen {
        let catalog: Rc<dyn ShipyardCatalog> = Rc::clone(art) as Rc<dyn ShipyardCatalog>;
        HireScreen::new(
            Ok((template(), Rc::new(MonoMetrics))),
            info_templates(),
            list,
            catalog,
            ButtonStyle::STOCK,
        )
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
        assert_eq!((HIRE_ITEM, HIRE_KEY), (7, Key::Char('h')));
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
        assert!(drawn(&screen).iter().any(|command| matches!(
            command,
            DrawCommand::StretchedPicture { image, .. } if image.id == 5000
        )));
        let mut screen = screen;
        screen.input(&key(Key::Right, false));
        assert_eq!(*art.asked.borrow(), ["dësc 14000", "dësc 14012"]);
    }

    #[test]
    fn the_grid_shows_each_ships_short_name_with_no_current_mark() {
        let words: Vec<String> = texts(&screen()).into_iter().map(|(t, _, _)| t).collect();
        for line in ["Ship", "128", "140"] {
            assert!(words.contains(&line.to_owned()), "{line}: {words:?}");
        }
        assert!(!words.contains(&"(current)".to_owned()));
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
    fn the_grid_shows_each_ships_lines_in_its_cell_and_highlights_the_selected() {
        let screen = screen();
        let commands = drawn(&screen);
        let cell = |index: usize, dy: f32| {
            let bounds = screen.cell_bounds(index).expect("a cell");
            Point::new(bounds.min.x + INSET, bounds.min.y + dy)
        };
        let line = MonoMetrics.line_height(TEXT_FONT, TEXT_SIZE);
        let shown = texts(&screen);
        for expected in [
            ("Ship".to_owned(), cell(0, INSET), TEXT_COLOR),
            ("128".to_owned(), cell(0, INSET + line), TEXT_COLOR),
            ("Ship".to_owned(), cell(1, INSET), TEXT_COLOR),
            ("140".to_owned(), cell(1, INSET + line), TEXT_COLOR),
        ] {
            assert!(shown.contains(&expected), "{expected:?}: {shown:?}");
        }
        let highlight = |index: usize| {
            let mut list = DrawList::new();
            fill_rect(
                &mut list,
                screen.cell_bounds(index).expect("a cell"),
                SELECTED_COLOR,
            );
            list.iter().cloned().collect::<Vec<_>>()
        };
        assert!(commands.windows(1).any(|w| w == highlight(0)));
        assert!(!commands.windows(1).any(|w| w == highlight(1)), "one");
        let mut frame = DrawList::new();
        outline(&mut frame, screen.cell_bounds(1).expect("a cell"), GREY);
        let frame: Vec<_> = frame.iter().cloned().collect();
        assert!(commands.windows(frame.len()).any(|w| w == frame));
    }

    #[test]
    fn the_arrows_scroll_the_grid_a_row_at_a_time() {
        let mut screen = screen_of(&art(), long(22));
        assert_eq!(screen.top_row(), 0);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 1);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        click_item(&mut screen, SCROLL_DOWN_ITEM);
        assert_eq!(screen.top_row(), 2, "never past the end");
        click_item(&mut screen, SCROLL_UP_ITEM);
        assert_eq!(screen.top_row(), 1);
        screen.input(&key(Key::Down, false));
        assert_eq!(screen.selected(), Some(4));
    }

    /// Ship 128's row, its `Long Name` "The Long 128", alone in the list.
    fn long_named() -> HireList {
        let mut row = row(128, 970, 100, Ok(()));
        row.specs.long_name = "The Long 128".to_owned();
        HireList {
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

    /// The original's hire mode of `_DoShipyardDialog` maps keys in `_ShipyardFilter` @0x60449 to items with no click sound.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        let mut screen = screen();
        for k in [
            Key::Right,
            Key::Down,
            Key::Left,
            Key::Up,
            Key::Enter,
            Key::Escape,
        ] {
            for pressed in [true, false] {
                screen.input(&Input::Key {
                    key: k,
                    pressed,
                    repeat: false,
                });
            }
        }
        assert_eq!(screen.take_sounds(), [], "keys make no click");
    }
}
