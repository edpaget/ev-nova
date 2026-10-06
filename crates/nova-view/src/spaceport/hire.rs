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
//! ("Shipyard Info", `DLOG` 1005, [`spec_lines`]). Its info box shows
//! "Hiring Price:" and the fee (`STR#` 2002 #228), the daily pay ("Pay:
//! 100 credits per day", which the original's dialog does not show) and
//! "You Have:" and the cash (#217).
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

use nova_sim::hire::{HIRE_ESCORT, HIRING_PRICE, YOU_HAVE};
use nova_sim::{HireList, HireRow, ShipId};

use super::grid::{
    self, CellGrid, GREY, INSET, SELECTED_COLOR, Shown, TEXT_COLOR, TEXT_FONT, TEXT_SIZE,
    name_lines, text,
};
use super::layout::DONE_LABEL;
use super::shipyard::{
    BACKGROUND, BUY_ITEM, DESCRIPTION_ITEM, DONE_ITEM, GRID_ITEM, INFO_BOX_ITEM, INFO_ITEM,
    INFO_KEY, INFO_LABEL, PICTURE_ITEM, Panel, SCROLL_DOWN_ITEM, SCROLL_UP_ITEM, ShipyardCatalog,
    ship_picture, spec_lines,
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
use crate::ui::comm::{grouped, pay_text};
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

/// The info box's lines for `row`, with `cash` in hand: the fee, the
/// daily pay and the cash.
#[must_use]
pub fn hire_lines(row: &HireRow, cash: i64) -> [String; 3] {
    [
        format!("{HIRING_PRICE} {}", grouped(row.fee)),
        pay_text(row.wage),
        format!("{YOU_HAVE} {}", grouped(cash)),
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
    /// The info panel's template, if there is one.
    info: Option<DialogTemplate>,
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
        info: Result<DialogTemplate, String>,
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
            let info = info.ok();
            dialog.set_greyed(INFO_ITEM, info.is_none());
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
        let (title, lines) = (row.name.clone(), spec_lines(row.specs));
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
            shown.draw(list, area);
        }
        let Some(info) = laid.dialog.item_bounds(INFO_BOX_ITEM) else {
            return;
        };
        let line_height = laid.metrics.0.line_height(TEXT_FONT, TEXT_SIZE);
        for (n, line) in hire_lines(row, self.list.cash).iter().enumerate() {
            let origin = Point::new(info.min.x + INSET, info.min.y + line_height * n as f32);
            text(list, line, origin, TEXT_COLOR);
        }
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

    use nova_sim::{HireRefusal, ShipFields, ShipSpecs};

    use super::*;
    use crate::draw::{DrawCommand, fill_rect};
    use crate::input::MouseButton;
    use crate::spaceport::catalog::{PortRecord, SpaceportCatalog, StellarId};
    use crate::spaceport::shipyard::ShipBaseImages;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::catalog::DescriptionSource;
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

    fn art() -> Rc<FakeArt> {
        Rc::new(FakeArt {
            asked: RefCell::default(),
        })
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
            Ok(info_template()),
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

    #[test]
    fn the_named_values() {
        assert_eq!((HIRE_ITEM, HIRE_KEY), (7, Key::Char('h')));
        assert_eq!(pilot_desc_id(ShipId(128)), 14_000);
        assert_eq!(pilot_desc_id(ShipId(129)), 14_001);
        assert_eq!(pilot_desc_id(ShipId(895)), 14_767);
    }

    #[test]
    fn the_info_box_shows_the_fee_the_daily_pay_and_the_cash() {
        let screen = screen();
        assert_eq!(
            info_box(&screen),
            [
                "Hiring Price: 970",
                "Pay: 100 credits per day",
                "You Have: 25,000"
            ]
        );
        let info = item(&screen, INFO_BOX_ITEM);
        let line = MonoMetrics.line_height(TEXT_FONT, TEXT_SIZE);
        let origins: Vec<Point> = texts(&screen)
            .into_iter()
            .filter(|(_, at, _)| info.contains(*at))
            .map(|(_, at, _)| at)
            .collect();
        assert_eq!(
            origins,
            [0.0, 1.0, 2.0].map(|n| Point::new(info.min.x + INSET, info.min.y + line * n))
        );
        assert_eq!(
            hire_lines(&row(128, 12_000, 1, Ok(())), 3),
            [
                "Hiring Price: 12,000".to_owned(),
                "Pay: 1 credit per day".to_owned(),
                "You Have: 3".to_owned()
            ]
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
        assert!(words.contains(&"Speed: 300".to_owned()), "{words:?}");
        screen.input(&key(HIRE_KEY, false));
        assert_eq!(screen.take_order(), None, "the panel takes the keys");
        screen.input(&key(Key::Escape, false));
        assert!(screen.info_panel().is_none());
        assert!(!screen.closed());
        let mut bare = HireScreen::new(
            Ok((template(), Rc::new(MonoMetrics))),
            Err("no DLOG 1005".to_owned()),
            list(),
            art(),
            ButtonStyle::STOCK,
        );
        assert!(!enabled(&bare, INFO_LABEL));
        bare.input(&key(INFO_KEY, false));
        assert!(bare.info_panel().is_none());
    }

    #[test]
    fn without_its_dialog_it_says_why_and_return_closes_it() {
        let mut screen = HireScreen::new(
            Err("no DLOG 1004".to_owned()),
            Ok(info_template()),
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
}
