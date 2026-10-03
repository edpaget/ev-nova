//! The galaxy map screen: every system, hyperlink and nebula, which pans
//! and zooms, and an info panel listing the selected system's stellars.
//!
//! The screen reads the galaxy once, through the [`GalaxyCatalog`] port,
//! when it is built; drawing and input never read anything.
//!
//! Input (presses only, except the left button's release):
//!
//! - The arrow keys pan the view by [`PAN_STEP`]; `=` or `+` zooms in and
//!   `-` zooms out.
//! - Pressing the left button in the map area and moving the pointer drags
//!   the map. Releasing it after moving no more than [`CLICK_SLOP`] is a
//!   click instead: it selects the system under the pointer (clicking again
//!   cycles through systems that share a position) or, in empty space,
//!   clears the selection.
//! - Return (a press, not its repeats), or a click on the "Enter system"
//!   button the panel shows while a system is selected, asks to enter the
//!   selected system. The map only records the request; its owner takes
//!   it with [`GalaxyMap::take_entry`]. A click on the button is a press
//!   and a release both on it.
//! - Tab is the app's router's and Escape the navigator's.

use std::time::Duration;

use super::catalog::{GalaxyCatalog, SystemEntry, SystemId};
use super::model::{GalaxyModel, placement};
use super::view::{Bounds, MAP_HEIGHT, MAP_WIDTH, MapView, PAN_STEP};
use crate::{Color, DrawList, ImageKey, Input, Key, MouseButton, Point, Screen, ScreenAction};

/// How far the pointer may travel between pressing and releasing the left
/// button for the press to count as a click rather than a drag.
pub const CLICK_SLOP: f32 = 4.0;

/// Hyperlinks' colour.
pub const LINK: Color = Color::rgba(90, 90, 110, 255);
/// Hyperlinks' width.
const LINK_WIDTH: f32 = 1.0;
/// The outline drawn under every dot, so black and dark governments'
/// systems show on the black map.
pub const OUTLINE: Color = Color::rgba(128, 128, 128, 255);
/// The outline's size.
pub const OUTLINE_SIZE: f32 = 8.0;
/// A system dot's size.
pub const DOT_SIZE: f32 = 6.0;
/// The side of the square drawn round the selected system, and its width.
pub const HIGHLIGHT_SIZE: f32 = 16.0;
const HIGHLIGHT_WIDTH: f32 = 1.0;

/// The bottom of the screen, where the info panel ends.
const SCREEN_HEIGHT: f32 = 768.0;
/// The info panel's background.
pub const PANEL: Color = Color::rgba(16, 16, 28, 255);
/// The line between the map and the panel: its colour and width.
const SEPARATOR: Color = OUTLINE;
const SEPARATOR_WIDTH: f32 = 1.0;

/// The panel's left column: the selected system's name, then its stellars.
const LEFT: f32 = 16.0;
const LEFT_WRAP: f32 = 496.0;
const TITLE_TOP: f32 = 616.0;
const TITLE_SIZE: f32 = 24.0;
const STELLARS_TOP: f32 = 652.0;
const STELLAR_SPACING: f32 = 20.0;
const STELLAR_SIZE: f32 = 16.0;
/// The panel's right column: help, zoom, the stack line and problems.
const RIGHT: f32 = 528.0;
const RIGHT_WRAP: f32 = 480.0;
const RIGHT_SIZE: f32 = 14.0;
const HELP_TOP: f32 = 616.0;
const ZOOM_TOP: f32 = 636.0;
const STACK_TOP: f32 = 656.0;
const PROBLEMS_TOP: f32 = 676.0;
/// The help line.
pub const HELP: &str = "Arrows or drag: pan   +/-: zoom   Click: select   Return: enter";
/// The "Enter system" button, in the panel's right column under the
/// problems line; shown only while a system is selected.
pub const ENTER_BUTTON: Bounds = Bounds {
    min: Point::new(528.0, 712.0),
    max: Point::new(708.0, 744.0),
};
/// The button's colour.
pub const BUTTON: Color = Color::rgba(48, 48, 80, 255);
/// The button's label, where it goes and its size.
pub const ENTER_LABEL: &str = "Enter system (Return)";
const ENTER_LABEL_AT: Point = Point::new(540.0, 720.0);
const ENTER_LABEL_SIZE: f32 = 14.0;

// The two columns stay on screen and apart.
const _: () = assert!(LEFT + LEFT_WRAP <= RIGHT);
const _: () = assert!(RIGHT + RIGHT_WRAP <= MAP_WIDTH);
const _: () = assert!(TITLE_TOP > MAP_HEIGHT && HELP_TOP > MAP_HEIGHT);

/// A left-button press that has not been released yet.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Press {
    /// Where the pointer was last seen.
    last: Point,
    /// How far the pointer has travelled since the press.
    travelled: f32,
}

/// The galaxy map.
#[derive(Clone, Debug)]
pub struct GalaxyMap {
    model: GalaxyModel,
    view: MapView,
    selected: Option<SystemId>,
    press: Option<Press>,
    /// Whether the left button went down on the "Enter system" button and
    /// has not been released yet.
    armed: bool,
    /// A request to enter a system, not yet taken.
    entry: Option<SystemId>,
}

impl GalaxyMap {
    /// The map of `catalog`'s galaxy, fitted to the map area, with nothing
    /// selected.
    pub fn new(catalog: &impl GalaxyCatalog) -> Self {
        let model = GalaxyModel::new(catalog.galaxy());
        let view = MapView::fit(model.bounds());
        Self {
            model,
            view,
            selected: None,
            press: None,
            armed: false,
            entry: None,
        }
    }

    /// The selected system, if any.
    #[must_use]
    pub fn selected(&self) -> Option<SystemId> {
        self.selected
    }

    /// What the map area shows.
    #[must_use]
    pub fn view(&self) -> &MapView {
        &self.view
    }

    /// The galaxy as the map lays it out.
    #[must_use]
    pub fn model(&self) -> &GalaxyModel {
        &self.model
    }

    /// The system the map has been asked to enter, by Return or the
    /// "Enter system" button, if any; taking it clears the request.
    pub fn take_entry(&mut self) -> Option<SystemId> {
        self.entry.take()
    }

    /// Everything that could not be read or laid out.
    #[must_use]
    pub fn problems(&self) -> &[String] {
        self.model.problems()
    }
}

impl GalaxyMap {
    /// Handles a key press.
    fn key(&mut self, key: Key) {
        match key {
            Key::Left => self.view.pan(-PAN_STEP, 0.0),
            Key::Right => self.view.pan(PAN_STEP, 0.0),
            Key::Up => self.view.pan(0.0, -PAN_STEP),
            Key::Down => self.view.pan(0.0, PAN_STEP),
            Key::Char('=' | '+') => self.view.zoom_in(),
            Key::Char('-') => self.view.zoom_out(),
            _ => {}
        }
    }

    /// Asks to enter the selected system, if there is one.
    fn enter(&mut self) {
        if let Some(id) = self.selected {
            self.entry = Some(id);
        }
    }

    /// Ends the press, if there is one, with the left button released at
    /// `at`: a click if the pointer stayed within [`CLICK_SLOP`] and is
    /// still over the map, otherwise the end of a drag.
    fn release(&mut self, at: Point) {
        let Some(press) = self.press.take() else {
            return;
        };
        if press.travelled <= CLICK_SLOP && in_map(at) {
            self.selected = self.model.click(at, &self.view, self.selected);
        }
    }

    /// Drags the map with the pointer, if the left button is down.
    fn pointer_moved(&mut self, to: Point) {
        let Some(press) = &mut self.press else {
            return;
        };
        let (dx, dy) = (to.x - press.last.x, to.y - press.last.y);
        press.travelled += dx.hypot(dy);
        press.last = to;
        self.view.pan(-dx, -dy);
    }

    fn draw_map(&self, list: &mut DrawList) {
        // Every nebula before any line or dot, so behind the systems.
        for nebula in self.model.nebulae() {
            if let Some(placed) = placement(nebula, &self.view) {
                list.stretched_picture(
                    ImageKey::picture(placed.picture.id),
                    placed.top_left,
                    placed.width,
                    placed.height,
                );
            }
        }
        let screen = |id| {
            self.model
                .system(id)
                .map(|system| self.view.world_to_screen(system.position()))
        };
        for &(from, to) in self.model.links() {
            if let (Some(from), Some(to)) = (screen(from), screen(to)) {
                list.line(from, to, LINK_WIDTH, LINK);
            }
        }
        // Highest ID first, so where systems share a position the lowest
        // ID, the one a first click selects, is on top.
        let systems = self.model.systems().iter().rev();
        for system in systems.clone() {
            let at = self.view.world_to_screen(system.position());
            list.dot(at, OUTLINE_SIZE, OUTLINE);
        }
        for system in systems {
            let at = self.view.world_to_screen(system.position());
            list.dot(at, DOT_SIZE, system.color);
        }
        if let Some(at) = self.selected.and_then(screen) {
            let half = HIGHLIGHT_SIZE / 2.0;
            let corners = [
                Point::new(at.x - half, at.y - half),
                Point::new(at.x + half, at.y - half),
                Point::new(at.x + half, at.y + half),
                Point::new(at.x - half, at.y + half),
            ];
            for (index, &from) in corners.iter().enumerate() {
                list.line(
                    from,
                    corners[(index + 1) % 4],
                    HIGHLIGHT_WIDTH,
                    Color::WHITE,
                );
            }
        }
    }

    fn draw_panel(&self, list: &mut DrawList) {
        let panel = Bounds {
            min: Point::new(0.0, MAP_HEIGHT),
            max: Point::new(MAP_WIDTH, SCREEN_HEIGHT),
        };
        fill_rect(list, panel, PANEL);
        list.line(
            Point::new(0.0, MAP_HEIGHT),
            Point::new(MAP_WIDTH, MAP_HEIGHT),
            SEPARATOR_WIDTH,
            SEPARATOR,
        );
        let selected = self.selected.and_then(|id| self.model.system(id));
        if selected.is_some() {
            fill_rect(list, ENTER_BUTTON, BUTTON);
        }
        match selected {
            Some(system) => draw_system(&system.entry, list),
            None => {
                list.text(
                    "Click a system to see its stellars",
                    Point::new(LEFT, TITLE_TOP),
                    STELLAR_SIZE,
                    Some(LEFT_WRAP),
                    Color::DIM,
                );
            }
        }
        let right = |list: &mut DrawList, text: String, top: f32, color: Color| {
            list.text(
                text,
                Point::new(RIGHT, top),
                RIGHT_SIZE,
                Some(RIGHT_WRAP),
                color,
            );
        };
        right(list, HELP.to_owned(), HELP_TOP, Color::DIM);
        let percent = (self.view.scale() * 100.0).round();
        right(list, format!("Zoom {percent}%"), ZOOM_TOP, Color::DIM);
        if let Some(system) = selected {
            let stack = self.model.stack(system.entry.id);
            if stack.len() > 1 {
                let index = stack.iter().position(|&id| id == system.entry.id);
                let ids: Vec<String> = stack.iter().map(|id| id.0.to_string()).collect();
                let line = format!(
                    "{} of {} systems here (sÿst {}): click again for the next",
                    index.map_or(0, |index| index + 1),
                    stack.len(),
                    ids.join(", ")
                );
                right(list, line, STACK_TOP, Color::WHITE);
            }
        }
        let problems = self.model.problems();
        if let Some(first) = problems.first() {
            let line = format!(
                "{} problem(s) reading the map data: {first}",
                problems.len()
            );
            right(list, line, PROBLEMS_TOP, Color::ERROR);
        }
        if selected.is_some() {
            list.text(
                ENTER_LABEL,
                ENTER_LABEL_AT,
                ENTER_LABEL_SIZE,
                None,
                Color::WHITE,
            );
        }
    }
}

/// The system's name and ID, then its stellars.
fn draw_system(system: &SystemEntry, list: &mut DrawList) {
    list.text(
        format!("{} (sÿst {})", system.name, system.id.0),
        Point::new(LEFT, TITLE_TOP),
        TITLE_SIZE,
        Some(LEFT_WRAP),
        Color::WHITE,
    );
    let names: Vec<&str> = if system.stellars.is_empty() {
        vec!["No stellars"]
    } else {
        system.stellars.iter().map(|s| s.name.as_str()).collect()
    };
    for (index, name) in names.into_iter().enumerate() {
        let top = STELLARS_TOP + index as f32 * STELLAR_SPACING;
        list.text(
            name,
            Point::new(LEFT, top),
            STELLAR_SIZE,
            Some(LEFT_WRAP),
            Color::WHITE,
        );
    }
}

/// Whether `at` is in the map area rather than the panel.
fn in_map(at: Point) -> bool {
    at.y < MAP_HEIGHT
}

/// Fills `area` with `color`. There is no rectangle command: a horizontal
/// line as thick as the area, along its middle, is one.
fn fill_rect(list: &mut DrawList, area: Bounds, color: Color) {
    let middle = area.center().y;
    list.line(
        Point::new(area.min.x, middle),
        Point::new(area.max.x, middle),
        area.height(),
        color,
    );
}

impl Screen for GalaxyMap {
    /// Never quits: Escape is the navigator's.
    fn input(&mut self, input: &Input) -> ScreenAction {
        match *input {
            // Return enters once, however long it is held.
            Input::Key {
                key: Key::Enter,
                pressed: true,
                repeat,
            } => {
                if !repeat {
                    self.enter();
                }
            }
            // Repeats too: holding a key keeps panning or zooming.
            Input::Key {
                key, pressed: true, ..
            } => self.key(key),
            Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            } => {
                if !pressed {
                    // A click on the button: pressed and released on it.
                    if std::mem::take(&mut self.armed) && ENTER_BUTTON.contains(at) {
                        self.enter();
                    }
                    self.release(at);
                } else if self.selected.is_some() && ENTER_BUTTON.contains(at) {
                    self.armed = true;
                } else {
                    // Replaces any button press whose release was lost.
                    self.armed = false;
                    if in_map(at) {
                        // Replaces any press whose release was lost.
                        self.press = Some(Press {
                            last: at,
                            travelled: 0.0,
                        });
                    }
                }
            }
            Input::PointerMoved(to) => self.pointer_moved(to),
            _ => {}
        }
        ScreenAction::None
    }

    /// The map does not animate.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        self.draw_map(list);
        self.draw_panel(list);
    }

    /// Forgets the left-button press, if any: the map stops dragging and the
    /// release, wherever it ends up, is not a click, on the map or on the
    /// "Enter system" button.
    fn cancel_pointer(&mut self) {
        self.press = None;
        self.armed = false;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::Cell;
    use std::collections::BTreeMap;

    use super::*;
    use crate::galaxy::catalog::{
        Galaxy, GovtId, NebulaEntry, NebulaId, NebulaPicture, StellarEntry, StellarId, SystemEntry,
    };
    use crate::galaxy::model::{NEUTRAL, placement};
    use crate::galaxy::view::{MAP_CENTER, SCALES};
    use crate::{DrawCommand, Font, ImageKey, Key, MouseButton};

    /// A canned galaxy; counts how often it is read.
    struct FakeCatalog {
        galaxy: Galaxy,
        reads: Cell<usize>,
    }

    impl FakeCatalog {
        fn new(galaxy: Galaxy) -> Self {
            Self {
                galaxy,
                reads: Cell::new(0),
            }
        }
    }

    impl GalaxyCatalog for FakeCatalog {
        fn galaxy(&self) -> Galaxy {
            self.reads.set(self.reads.get() + 1);
            self.galaxy.clone()
        }
    }

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn system(id: i16, x: i16, y: i16) -> SystemEntry {
        SystemEntry {
            id: SystemId(id),
            name: format!("System {id}"),
            x,
            y,
            links: Vec::new(),
            govt: None,
            stellars: Vec::new(),
        }
    }

    const BLUE: u32 = 0x002C_2CAF;

    /// Alpha (128) at (0, 0), Federation, with two stellars; Beta (129) at
    /// (600, 0), independent; Gamma (130) at (0, 300), Federation; and the
    /// alternate Alpha (131) at Alpha's position. Alpha and Beta link both
    /// ways and Gamma links one way to Alpha. A nebula covers Alpha.
    fn galaxy() -> Galaxy {
        let alpha = SystemEntry {
            name: "Alpha".to_owned(),
            links: vec![SystemId(129)],
            govt: Some(GovtId(128)),
            stellars: vec![
                StellarEntry {
                    id: StellarId(128),
                    name: "Alpha Prime".to_owned(),
                },
                StellarEntry {
                    id: StellarId(129),
                    name: "Alpha Station".to_owned(),
                },
            ],
            ..system(128, 0, 0)
        };
        let beta = SystemEntry {
            name: "Beta".to_owned(),
            links: vec![SystemId(128)],
            ..system(129, 600, 0)
        };
        let gamma = SystemEntry {
            name: "Gamma".to_owned(),
            links: vec![SystemId(128)],
            govt: Some(GovtId(128)),
            ..system(130, 0, 300)
        };
        let alternate = SystemEntry {
            name: "Alpha Reborn".to_owned(),
            ..system(131, 0, 0)
        };
        Galaxy {
            systems: vec![alpha, beta, gamma, alternate],
            govt_colors: BTreeMap::from([(GovtId(128), BLUE)]),
            nebulae: vec![NebulaEntry {
                id: NebulaId(128),
                name: "Holpa Nebula".to_owned(),
                left: -20,
                top: -20,
                width: 60,
                height: 60,
                pictures: vec![
                    NebulaPicture {
                        id: 9500,
                        width: 30,
                        height: 30,
                    },
                    NebulaPicture {
                        id: 9502,
                        width: 60,
                        height: 60,
                    },
                ],
            }],
            problems: Vec::new(),
        }
    }

    fn map() -> GalaxyMap {
        GalaxyMap::new(&FakeCatalog::new(galaxy()))
    }

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    /// A key held down past the OS key-repeat delay.
    fn held(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    fn button(button: MouseButton, pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button,
            pressed,
            at,
        }
    }

    /// Presses and releases the left button at `at`.
    fn click(map: &mut GalaxyMap, at: Point) {
        assert_eq!(
            map.input(&button(MouseButton::Left, true, at)),
            ScreenAction::None
        );
        assert_eq!(
            map.input(&button(MouseButton::Left, false, at)),
            ScreenAction::None
        );
    }

    /// Clicks on system `id`'s dot.
    fn click_on(map: &mut GalaxyMap, id: i16) {
        let at = dot(map, id);
        click(map, at);
    }

    /// Where system `id` is drawn.
    fn dot(map: &GalaxyMap, id: i16) -> Point {
        let system = map.model().system(SystemId(id)).expect("a system");
        map.view().world_to_screen(system.position())
    }

    fn drawn(map: &GalaxyMap) -> DrawList {
        let mut list = DrawList::new();
        map.draw(&mut list);
        list
    }

    fn texts(list: &DrawList) -> Vec<String> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    fn text(list: &DrawList, starting: &str) -> DrawCommand {
        list.iter()
            .find(|command| matches!(command, DrawCommand::Text { text, .. } if text.starts_with(starting)))
            .cloned()
            .unwrap_or_else(|| panic!("no text starting {starting:?} in {:?}", texts(list)))
    }

    fn origin(list: &DrawList, starting: &str) -> Point {
        match text(list, starting) {
            DrawCommand::Text { origin, .. } => origin,
            _ => unreachable!(),
        }
    }

    /// The dots of `size`, as (centre, colour), in draw order.
    fn dots(list: &DrawList, of_size: f32) -> Vec<(Point, Color)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Dot {
                    center,
                    size,
                    color,
                } if *size == of_size => Some((*center, *color)),
                _ => None,
            })
            .collect()
    }

    /// The lines of `color`, as (from, to, width), in draw order.
    fn lines(list: &DrawList, of_color: Color) -> Vec<(Point, Point, f32)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Line {
                    from,
                    to,
                    width,
                    color,
                } if *color == of_color => Some((*from, *to, *width)),
                _ => None,
            })
            .collect()
    }

    // Building.

    #[test]
    fn it_reads_the_galaxy_once_and_starts_fitted_with_nothing_selected() {
        let catalog = FakeCatalog::new(galaxy());
        let mut map = GalaxyMap::new(&catalog);
        assert_eq!(catalog.reads.get(), 1);
        assert_eq!(map.selected(), None);
        // 600 x 300: 133% fits, 178% does not.
        assert_eq!(map.view().scale(), SCALES[4]);
        assert_eq!(map.view().center(), at(300.0, 150.0));
        assert_eq!(map.model().systems().len(), 4);
        drawn(&map);
        click(&mut map, MAP_CENTER);
        map.tick(Duration::from_secs(1));
        assert_eq!(catalog.reads.get(), 1, "only building reads the galaxy");
    }

    #[test]
    fn problems_are_the_catalogs_and_the_models() {
        let mut galaxy = galaxy();
        galaxy.problems.push("sÿst 140: bad".to_owned());
        galaxy.systems[1].govt = Some(GovtId(200));
        let map = GalaxyMap::new(&FakeCatalog::new(galaxy));
        assert_eq!(map.problems(), ["sÿst 140: bad", "sÿst 129: no gövt 200"]);
    }

    // Keys.

    #[test]
    fn the_arrow_keys_pan_one_step() {
        let mut map = map();
        let scale = map.view().scale();
        let start = map.view().center();
        let steps = [
            (Key::Right, (1.0, 0.0)),
            (Key::Left, (0.0, 0.0)),
            (Key::Left, (-1.0, 0.0)),
            (Key::Down, (-1.0, 1.0)),
            (Key::Up, (-1.0, 0.0)),
        ];
        for (key, (x, y)) in steps {
            assert_eq!(map.input(&press(key)), ScreenAction::None);
            let expected = at(
                start.x + x * PAN_STEP / scale,
                start.y + y * PAN_STEP / scale,
            );
            let center = map.view().center();
            assert!(
                (center.x - expected.x).abs() < 1e-3 && (center.y - expected.y).abs() < 1e-3,
                "{key:?}: {center:?} is not {expected:?}"
            );
        }
    }

    #[test]
    fn held_keys_go_on_panning_and_zooming() {
        let mut map = map();
        let (scale, start, fitted) = (map.view().scale(), map.view().center(), map.view().zoom());
        map.input(&press(Key::Right));
        assert_eq!(map.input(&held(Key::Right)), ScreenAction::None);
        map.input(&held(Key::Right));
        let center = map.view().center();
        let expected = start.x + 3.0 * PAN_STEP / scale;
        assert!((center.x - expected).abs() < 1e-3, "{center:?}");
        map.input(&held(Key::Char('=')));
        assert_eq!(map.view().zoom(), fitted + 1);
    }

    #[test]
    fn equals_and_plus_zoom_in_and_minus_zooms_out() {
        let mut map = map();
        let fitted = map.view().zoom();
        map.input(&press(Key::Char('=')));
        assert_eq!(map.view().zoom(), fitted + 1);
        map.input(&press(Key::Char('+')));
        assert_eq!(map.view().zoom(), fitted + 2);
        for _ in 0..3 {
            assert_eq!(map.input(&press(Key::Char('-'))), ScreenAction::None);
        }
        assert_eq!(map.view().zoom(), fitted - 1);
    }

    #[test]
    fn releases_and_other_keys_change_nothing() {
        let mut map = map();
        click_on(&mut map, 129);
        let (view, selected) = (*map.view(), map.selected());
        let others = [
            Input::Key {
                key: Key::Right,
                pressed: false,
                repeat: false,
            },
            Input::Key {
                key: Key::Char('='),
                pressed: false,
                repeat: false,
            },
            press(Key::Escape),
            press(Key::Tab),
            press(Key::Space),
            press(Key::Char('x')),
            press(Key::Other),
        ];
        for input in others {
            assert_eq!(map.input(&input), ScreenAction::None, "{input:?}");
        }
        assert_eq!((*map.view(), map.selected()), (view, selected));
    }

    // The pointer.

    #[test]
    fn a_click_on_a_system_selects_it_and_on_empty_space_clears_it() {
        let mut map = map();
        click_on(&mut map, 129);
        assert_eq!(map.selected(), Some(SystemId(129)));
        let beta = dot(&map, 129);
        click(&mut map, at(beta.x - 40.0, beta.y));
        assert_eq!(map.selected(), None);
    }

    #[test]
    fn clicking_a_stacked_position_again_cycles_through_its_systems() {
        let mut map = map();
        let alpha = dot(&map, 128);
        let mut visited = Vec::new();
        for _ in 0..3 {
            click(&mut map, alpha);
            visited.push(map.selected().expect("a system").0);
        }
        assert_eq!(visited, [128, 131, 128]);
    }

    #[test]
    fn a_press_moved_no_more_than_the_slop_is_still_a_click() {
        let mut map = map();
        let beta = dot(&map, 129);
        map.input(&button(MouseButton::Left, true, beta));
        map.input(&Input::PointerMoved(at(beta.x + 2.0, beta.y)));
        map.input(&Input::PointerMoved(at(beta.x + 2.0, beta.y + 2.0)));
        map.input(&button(
            MouseButton::Left,
            false,
            at(beta.x + 2.0, beta.y + 2.0),
        ));
        assert_eq!(map.selected(), Some(SystemId(129)));
    }

    #[test]
    fn a_press_moved_past_the_slop_drags_the_map_and_keeps_the_selection() {
        let mut map = map();
        click_on(&mut map, 129);
        let start = map.view().center();
        let scale = map.view().scale();
        let gamma = dot(&map, 130);
        map.input(&button(MouseButton::Left, true, gamma));
        map.input(&Input::PointerMoved(at(gamma.x + 3.0, gamma.y - 4.0)));
        // Travelled 5 in all, back where it started.
        map.input(&Input::PointerMoved(gamma));
        map.input(&button(MouseButton::Left, false, gamma));
        assert_eq!(
            map.selected(),
            Some(SystemId(129)),
            "a drag selects nothing"
        );
        assert_eq!(map.view().center(), start, "dragged there and back");

        let empty = at(gamma.x + 100.0, gamma.y);
        map.input(&button(MouseButton::Left, true, empty));
        map.input(&Input::PointerMoved(at(empty.x - 20.0, empty.y + 10.0)));
        map.input(&button(
            MouseButton::Left,
            false,
            at(empty.x - 20.0, empty.y + 10.0),
        ));
        let center = map.view().center();
        assert_eq!(center, at(start.x + 20.0 / scale, start.y - 10.0 / scale));
        assert_eq!(map.selected(), Some(SystemId(129)), "nor clears it");
    }

    #[test]
    fn moving_the_pointer_without_a_press_pans_nothing() {
        let mut map = map();
        let view = *map.view();
        map.input(&Input::PointerMoved(at(10.0, 10.0)));
        map.input(&Input::PointerMoved(at(400.0, 300.0)));
        assert_eq!(*map.view(), view);
    }

    #[test]
    fn a_release_without_a_press_does_nothing() {
        let mut map = map();
        click_on(&mut map, 129);
        let alpha = dot(&map, 128);
        map.input(&button(MouseButton::Left, false, alpha));
        assert_eq!(map.selected(), Some(SystemId(129)));
    }

    /// Systems 128 at (0, -250) and 129 at (0, 250), fitted at 100%
    /// round (0, 0), then dragged 52 down so 129's dot is 2 above the panel.
    fn near_the_panel() -> GalaxyMap {
        let galaxy = Galaxy {
            systems: vec![system(128, 0, -250), system(129, 0, 250)],
            ..Galaxy::default()
        };
        let mut map = GalaxyMap::new(&FakeCatalog::new(galaxy));
        assert_eq!(map.view().scale(), 1.0);
        map.input(&button(MouseButton::Left, true, at(300.0, 300.0)));
        map.input(&Input::PointerMoved(at(300.0, 352.0)));
        map.input(&button(MouseButton::Left, false, at(300.0, 352.0)));
        assert_eq!(dot(&map, 129), at(512.0, 606.0));
        map
    }

    #[test]
    fn a_press_in_the_panel_starts_nothing() {
        let mut map = near_the_panel();
        let low = dot(&map, 129);
        let view = *map.view();
        // From the panel's top edge, 2 below the dot: no click.
        map.input(&button(MouseButton::Left, true, at(low.x, MAP_HEIGHT)));
        map.input(&Input::PointerMoved(low));
        map.input(&button(MouseButton::Left, false, low));
        assert_eq!(map.selected(), None);
        // Nor a drag.
        map.input(&button(MouseButton::Left, true, at(100.0, 700.0)));
        map.input(&Input::PointerMoved(at(200.0, 300.0)));
        map.input(&button(MouseButton::Left, false, at(200.0, 300.0)));
        assert_eq!(*map.view(), view);
        // Just above the edge is the map.
        map.input(&button(
            MouseButton::Left,
            true,
            at(low.x, MAP_HEIGHT - 0.5),
        ));
        map.input(&Input::PointerMoved(low));
        map.input(&button(MouseButton::Left, false, low));
        assert_eq!(map.selected(), Some(SystemId(129)));
    }

    #[test]
    fn a_click_released_in_the_panel_keeps_the_selection() {
        let mut map = near_the_panel();
        click_on(&mut map, 128);
        let low = dot(&map, 129);
        // Released on the panel's top edge, 2 below the dot.
        map.input(&button(MouseButton::Left, true, low));
        map.input(&button(MouseButton::Left, false, at(low.x, MAP_HEIGHT)));
        assert_eq!(map.selected(), Some(SystemId(128)));
        // Released just above it.
        map.input(&button(MouseButton::Left, true, low));
        map.input(&button(
            MouseButton::Left,
            false,
            at(low.x, MAP_HEIGHT - 0.5),
        ));
        assert_eq!(map.selected(), Some(SystemId(129)));
    }

    #[test]
    fn other_buttons_do_nothing() {
        let mut map = map();
        let beta = dot(&map, 129);
        for other in [MouseButton::Right, MouseButton::Middle, MouseButton::Other] {
            map.input(&button(other, true, beta));
            map.input(&Input::PointerMoved(at(beta.x + 50.0, beta.y)));
            map.input(&button(other, false, at(beta.x + 50.0, beta.y)));
        }
        assert_eq!(map.selected(), None);
        assert_eq!(*map.view(), *self::map().view());
    }

    #[test]
    fn a_new_press_replaces_one_whose_release_was_lost() {
        let mut map = map();
        let beta = dot(&map, 129);
        map.input(&button(MouseButton::Left, true, at(beta.x - 100.0, beta.y)));
        map.input(&Input::PointerMoved(at(beta.x - 50.0, beta.y)));
        // The release happened outside the window.
        let beta = dot(&map, 129);
        click(&mut map, beta);
        assert_eq!(map.selected(), Some(SystemId(129)));
    }

    #[test]
    fn cancelling_the_pointer_forgets_the_press_without_a_click() {
        let mut map = map();
        let view = *map.view();
        let beta = dot(&map, 129);
        map.input(&button(MouseButton::Left, true, beta));
        map.cancel_pointer();
        map.input(&Input::PointerMoved(at(beta.x + 50.0, beta.y)));
        assert_eq!(*map.view(), view, "no drag");
        map.input(&button(MouseButton::Left, false, beta));
        assert_eq!(map.selected(), None, "no click");

        // The next press starts afresh.
        click(&mut map, beta);
        assert_eq!(map.selected(), Some(SystemId(129)));
    }

    // Drawing.

    fn kind(command: &DrawCommand) -> &'static str {
        match command {
            DrawCommand::Sprite { .. } => "sprite",
            DrawCommand::Picture { .. } => "picture",
            DrawCommand::StretchedPicture { .. } => "stretched",
            DrawCommand::Text { .. } => "text",
            DrawCommand::Line { .. } => "line",
            DrawCommand::Dot { .. } => "dot",
        }
    }

    #[test]
    fn nebulae_are_drawn_before_every_line_and_dot() {
        let list = drawn(&map());
        let kinds: Vec<&str> = list.iter().map(kind).collect();
        let last_picture = kinds.iter().rposition(|&k| k == "stretched");
        let first_shape = kinds.iter().position(|&k| k == "line" || k == "dot");
        assert_eq!(last_picture, Some(0));
        assert_eq!(first_shape, Some(1));
        assert!(!kinds.contains(&"sprite") && !kinds.contains(&"picture"));
    }

    #[test]
    fn a_nebula_is_its_placement_stretched() {
        let mut map = map();
        for _ in 0..SCALES.len() {
            let list = drawn(&map);
            let nebula = &map.model().nebulae()[0];
            let placed = placement(nebula, map.view()).expect("placed");
            assert_eq!(
                list.iter().next(),
                Some(&DrawCommand::StretchedPicture {
                    image: ImageKey::picture(placed.picture.id),
                    top_left: placed.top_left,
                    width: placed.width,
                    height: placed.height,
                })
            );
            map.input(&press(Key::Char('-')));
        }
    }

    #[test]
    fn each_hyperlink_is_one_line_between_its_systems() {
        let map = map();
        let list = drawn(&map);
        assert_eq!(
            lines(&list, LINK),
            [
                (dot(&map, 128), dot(&map, 129), 1.0),
                (dot(&map, 128), dot(&map, 130), 1.0),
            ]
        );
    }

    #[test]
    fn every_system_has_an_outline_then_a_dot_in_its_colour_highest_id_first() {
        let map = map();
        let list = drawn(&map);
        let blue = Color::from_rgb24(BLUE);
        let outlines: Vec<(Point, Color)> = [131, 130, 129, 128]
            .into_iter()
            .map(|id| (dot(&map, id), OUTLINE))
            .collect();
        assert_eq!(dots(&list, OUTLINE_SIZE), outlines);
        assert_eq!(
            dots(&list, DOT_SIZE),
            [
                (dot(&map, 131), NEUTRAL),
                (dot(&map, 130), blue),
                (dot(&map, 129), NEUTRAL),
                (dot(&map, 128), blue),
            ]
        );
        let kinds: Vec<&str> = list.iter().map(kind).collect();
        let last_outline = list
            .iter()
            .rposition(|c| matches!(c, DrawCommand::Dot { size, .. } if *size == OUTLINE_SIZE));
        let first_fill = list
            .iter()
            .position(|c| matches!(c, DrawCommand::Dot { size, .. } if *size == DOT_SIZE));
        assert!(last_outline < first_fill, "{kinds:?}");
        let last_link = list
            .iter()
            .rposition(|c| matches!(c, DrawCommand::Line { color, .. } if *color == LINK));
        assert!(
            last_link
                < list
                    .iter()
                    .position(|c| matches!(c, DrawCommand::Dot { .. }))
        );
    }

    #[test]
    fn dots_follow_the_view() {
        let mut map = map();
        let before = dot(&map, 129);
        map.input(&press(Key::Right));
        let list = drawn(&map);
        let beta = dots(&list, DOT_SIZE)[2].0;
        assert!(
            (beta.x - (before.x - PAN_STEP)).abs() < 1e-3,
            "{beta:?} from {before:?}"
        );
        assert_eq!(beta.y, before.y);
    }

    /// The white lines: the highlight round the selected system.
    fn highlight(list: &DrawList) -> Vec<(Point, Point, f32)> {
        lines(list, Color::WHITE)
    }

    #[test]
    fn only_the_selected_system_is_highlighted() {
        let mut map = map();
        assert_eq!(highlight(&drawn(&map)), []);
        click_on(&mut map, 129);
        let c = dot(&map, 129);
        let h = HIGHLIGHT_SIZE / 2.0;
        let corners = [
            at(c.x - h, c.y - h),
            at(c.x + h, c.y - h),
            at(c.x + h, c.y + h),
            at(c.x - h, c.y + h),
        ];
        assert_eq!(
            highlight(&drawn(&map)),
            [
                (corners[0], corners[1], 1.0),
                (corners[1], corners[2], 1.0),
                (corners[2], corners[3], 1.0),
                (corners[3], corners[0], 1.0),
            ]
        );
        let list = drawn(&map);
        let last_dot = list
            .iter()
            .rposition(|c| matches!(c, DrawCommand::Dot { .. }));
        let first_white = list
            .iter()
            .position(|c| matches!(c, DrawCommand::Line { color, .. } if *color == Color::WHITE));
        assert!(
            last_dot < first_white,
            "the highlight is on top of the dots"
        );
    }

    #[test]
    fn the_panel_covers_the_bottom_of_the_screen_under_a_separator_then_its_text() {
        let list = drawn(&map());
        assert_eq!(
            lines(&list, PANEL),
            [(at(0.0, 688.0), at(MAP_WIDTH, 688.0), 160.0)]
        );
        assert_eq!(
            lines(&list, SEPARATOR)
                .into_iter()
                .filter(|(from, _, _)| from.y == MAP_HEIGHT)
                .collect::<Vec<_>>(),
            [(at(0.0, MAP_HEIGHT), at(MAP_WIDTH, MAP_HEIGHT), 1.0)]
        );
        let kinds: Vec<&str> = list.iter().map(kind).collect();
        let panel = list
            .iter()
            .position(|c| matches!(c, DrawCommand::Line { color, .. } if *color == PANEL))
            .expect("the panel");
        assert_eq!(kinds[panel + 1], "line", "the separator is next");
        assert!(kinds[..panel].iter().all(|&k| k != "text"));
        assert!(kinds[panel + 2..].iter().all(|&k| k == "text"), "{kinds:?}");
        assert!(list.iter().last().is_some_and(|c| kind(c) == "text"));
    }

    // The panel's text.

    #[test]
    fn with_nothing_selected_the_panel_says_how_to_select() {
        let list = drawn(&map());
        assert_eq!(
            text(&list, "Click a system"),
            DrawCommand::Text {
                text: "Click a system to see its stellars".to_owned(),
                font: Font::Geneva,
                origin: at(LEFT, TITLE_TOP),
                size: STELLAR_SIZE,
                wrap_width: Some(LEFT_WRAP),
                color: Color::DIM,
            }
        );
        assert_eq!(
            text(&list, "Arrows"),
            DrawCommand::Text {
                text: HELP.to_owned(),
                font: Font::Geneva,
                origin: at(RIGHT, HELP_TOP),
                size: RIGHT_SIZE,
                wrap_width: Some(RIGHT_WRAP),
                color: Color::DIM,
            }
        );
        assert_eq!(texts(&list).len(), 3, "{:?}", texts(&list));
    }

    #[test]
    fn a_selected_system_shows_its_name_and_id_then_its_stellars() {
        let mut map = map();
        click_on(&mut map, 128);
        let list = drawn(&map);
        assert_eq!(
            text(&list, "Alpha"),
            DrawCommand::Text {
                text: "Alpha (sÿst 128)".to_owned(),
                font: Font::Geneva,
                origin: at(LEFT, TITLE_TOP),
                size: TITLE_SIZE,
                wrap_width: Some(LEFT_WRAP),
                color: Color::WHITE,
            }
        );
        assert_eq!(
            text(&list, "Alpha Station"),
            DrawCommand::Text {
                text: "Alpha Station".to_owned(),
                font: Font::Geneva,
                origin: at(LEFT, STELLARS_TOP + STELLAR_SPACING),
                size: STELLAR_SIZE,
                wrap_width: Some(LEFT_WRAP),
                color: Color::WHITE,
            }
        );
        assert_eq!(origin(&list, "Alpha Prime"), at(LEFT, STELLARS_TOP));
        assert!(!texts(&list).iter().any(|t| t.starts_with("Click a system")));
    }

    #[test]
    fn a_system_without_stellars_says_so() {
        let mut map = map();
        click_on(&mut map, 129);
        let list = drawn(&map);
        let texts = texts(&list);
        assert!(texts.contains(&"Beta (sÿst 129)".to_owned()), "{texts:?}");
        assert_eq!(origin(&list, "No stellars"), at(LEFT, STELLARS_TOP));
        // The title, "No stellars", help, zoom and the button's label: no
        // stack line.
        assert_eq!(texts.len(), 5, "{texts:?}");
    }

    #[test]
    fn a_system_sharing_its_position_says_which_of_the_stack_it_is() {
        let mut map = map();
        click_on(&mut map, 128);
        let list = drawn(&map);
        let line = "1 of 2 systems here (sÿst 128, 131): click again for the next";
        assert_eq!(
            text(&list, "1 of 2"),
            DrawCommand::Text {
                text: line.to_owned(),
                font: Font::Geneva,
                origin: at(RIGHT, STACK_TOP),
                size: RIGHT_SIZE,
                wrap_width: Some(RIGHT_WRAP),
                color: Color::WHITE,
            }
        );
        click_on(&mut map, 128);
        let list = drawn(&map);
        assert!(texts(&list).contains(&"Alpha Reborn (sÿst 131)".to_owned()));
        text(&list, "2 of 2 systems here (sÿst 128, 131)");
    }

    #[test]
    fn the_zoom_is_shown_as_a_rounded_percentage() {
        let mut map = map();
        for _ in 0..SCALES.len() {
            map.input(&press(Key::Char('-')));
        }
        let mut shown = Vec::new();
        for _ in 0..SCALES.len() {
            let list = drawn(&map);
            assert_eq!(origin(&list, "Zoom"), at(RIGHT, ZOOM_TOP));
            shown.push(
                texts(&list)
                    .into_iter()
                    .find(|t| t.starts_with("Zoom"))
                    .expect("zoom"),
            );
            map.input(&press(Key::Char('=')));
        }
        assert_eq!(
            shown,
            [
                "Zoom 42%",
                "Zoom 56%",
                "Zoom 75%",
                "Zoom 100%",
                "Zoom 133%",
                "Zoom 178%",
                "Zoom 237%"
            ]
        );
    }

    #[test]
    fn problems_are_counted_with_the_first_one_in_red() {
        let mut galaxy = galaxy();
        galaxy.problems = vec!["sÿst 140: bad".to_owned(), "gövt 129: bad".to_owned()];
        let list = drawn(&GalaxyMap::new(&FakeCatalog::new(galaxy)));
        assert_eq!(
            text(&list, "2 problem"),
            DrawCommand::Text {
                text: "2 problem(s) reading the map data: sÿst 140: bad".to_owned(),
                font: Font::Geneva,
                origin: at(RIGHT, PROBLEMS_TOP),
                size: RIGHT_SIZE,
                wrap_width: Some(RIGHT_WRAP),
                color: Color::ERROR,
            }
        );
        assert!(!texts(&drawn(&map())).iter().any(|t| t.contains("problem")));
    }

    #[test]
    fn an_empty_galaxy_draws_only_the_panel() {
        let map = GalaxyMap::new(&FakeCatalog::new(Galaxy::default()));
        let list = drawn(&map);
        let kinds: Vec<&str> = list.iter().map(kind).collect();
        assert_eq!(kinds, ["line", "line", "text", "text", "text"]);
        assert_eq!(map.view().zoom(), 3);
    }

    // Entering a system.

    fn release(key: Key) -> Input {
        Input::Key {
            key,
            pressed: false,
            repeat: false,
        }
    }

    #[test]
    fn return_on_a_selected_system_asks_to_enter_it_once() {
        let mut map = map();
        click_on(&mut map, 129);
        let (view, selected) = (*map.view(), map.selected());
        assert_eq!(map.input(&press(Key::Enter)), ScreenAction::None);
        assert_eq!(map.take_entry(), Some(SystemId(129)));
        assert_eq!(map.take_entry(), None, "taken");
        assert_eq!((*map.view(), map.selected()), (view, selected));
    }

    #[test]
    fn return_with_nothing_selected_asks_nothing() {
        let mut map = map();
        map.input(&press(Key::Enter));
        assert_eq!(map.take_entry(), None);
        click_on(&mut map, 130);
        map.input(&press(Key::Enter));
        // The last request is the one taken.
        click_on(&mut map, 129);
        map.input(&press(Key::Enter));
        assert_eq!(map.take_entry(), Some(SystemId(129)));
    }

    #[test]
    fn a_return_repeat_or_release_asks_nothing() {
        let mut map = map();
        click_on(&mut map, 129);
        map.input(&held(Key::Enter));
        map.input(&release(Key::Enter));
        assert_eq!(map.take_entry(), None);
    }

    /// The button's centre.
    fn button_centre() -> Point {
        ENTER_BUTTON.center()
    }

    /// The button's quad: a line through its middle as thick as it is.
    fn button_quad() -> (Point, Point, f32) {
        (at(528.0, 728.0), at(708.0, 728.0), 32.0)
    }

    #[test]
    fn the_enter_button_is_drawn_only_while_a_system_is_selected() {
        let mut map = map();
        let list = drawn(&map);
        assert_eq!(lines(&list, BUTTON), []);
        assert!(!texts(&list).contains(&ENTER_LABEL.to_owned()));

        click_on(&mut map, 129);
        let list = drawn(&map);
        assert_eq!(lines(&list, BUTTON), [button_quad()]);
        assert_eq!(
            text(&list, "Enter system"),
            DrawCommand::Text {
                text: "Enter system (Return)".to_owned(),
                font: Font::Geneva,
                origin: at(540.0, 720.0),
                size: 14.0,
                wrap_width: None,
                color: Color::WHITE,
            }
        );
        assert_eq!(
            ENTER_BUTTON,
            Bounds {
                min: at(528.0, 712.0),
                max: at(708.0, 744.0)
            }
        );
        assert_eq!(BUTTON, Color::rgba(48, 48, 80, 255));
        // Over the panel, under its text.
        let quad = list
            .iter()
            .position(|c| matches!(c, DrawCommand::Line { color, .. } if *color == BUTTON));
        let panel = list
            .iter()
            .position(|c| matches!(c, DrawCommand::Line { color, .. } if *color == PANEL));
        let first_text = list
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { .. }));
        assert!(panel < quad && quad < first_text, "{quad:?}");
    }

    #[test]
    fn clicking_the_enter_button_asks_to_enter_the_selected_system() {
        let mut map = map();
        click_on(&mut map, 130);
        let (view, selected) = (*map.view(), map.selected());
        click(&mut map, button_centre());
        assert_eq!(map.take_entry(), Some(SystemId(130)));
        assert_eq!((*map.view(), map.selected()), (view, selected));
        // At its corners too.
        for corner in [ENTER_BUTTON.min, ENTER_BUTTON.max] {
            click(&mut map, corner);
            assert_eq!(map.take_entry(), Some(SystemId(130)), "{corner:?}");
        }
    }

    #[test]
    fn the_enter_button_does_nothing_with_nothing_selected() {
        let mut map = map();
        click(&mut map, button_centre());
        assert_eq!(map.take_entry(), None);
        assert_eq!(map.selected(), None);
    }

    #[test]
    fn a_press_on_the_button_released_off_it_does_not_enter() {
        let mut map = map();
        click_on(&mut map, 129);
        map.input(&button(MouseButton::Left, true, button_centre()));
        map.input(&button(MouseButton::Left, false, at(720.0, 728.0)));
        assert_eq!(map.take_entry(), None);
        // That release disarmed it: a release on the button now is not a
        // click.
        map.input(&button(MouseButton::Left, false, button_centre()));
        assert_eq!(map.take_entry(), None);
    }

    #[test]
    fn a_press_elsewhere_released_on_the_button_does_not_enter() {
        let mut map = map();
        click_on(&mut map, 129);
        let view = *map.view();
        for start in [at(300.0, 728.0), at(512.0, 300.0)] {
            map.input(&button(MouseButton::Left, true, start));
            map.input(&button(MouseButton::Left, false, button_centre()));
            assert_eq!(map.take_entry(), None, "{start:?}");
        }
        assert_eq!(map.selected(), Some(SystemId(129)), "nor clears it");
        assert_eq!(*map.view(), view);
    }

    #[test]
    fn a_new_press_off_the_button_disarms_it() {
        let mut map = map();
        click_on(&mut map, 129);
        map.input(&button(MouseButton::Left, true, button_centre()));
        // Its release was lost; the next press is elsewhere in the panel.
        map.input(&button(MouseButton::Left, true, at(300.0, 728.0)));
        map.input(&button(MouseButton::Left, false, button_centre()));
        assert_eq!(map.take_entry(), None);
    }

    #[test]
    fn a_press_on_the_button_starts_no_drag() {
        let mut map = map();
        click_on(&mut map, 129);
        let view = *map.view();
        map.input(&button(MouseButton::Left, true, button_centre()));
        map.input(&Input::PointerMoved(at(100.0, 100.0)));
        assert_eq!(*map.view(), view);
    }

    #[test]
    fn other_buttons_on_the_enter_button_do_nothing() {
        let mut map = map();
        click_on(&mut map, 129);
        for other in [MouseButton::Right, MouseButton::Middle, MouseButton::Other] {
            map.input(&button(other, true, button_centre()));
            map.input(&button(other, false, button_centre()));
        }
        assert_eq!(map.take_entry(), None);
    }

    #[test]
    fn cancelling_the_pointer_disarms_the_button() {
        let mut map = map();
        click_on(&mut map, 129);
        map.input(&button(MouseButton::Left, true, button_centre()));
        map.cancel_pointer();
        map.input(&button(MouseButton::Left, false, button_centre()));
        assert_eq!(map.take_entry(), None);
    }

    #[test]
    fn the_help_line_mentions_return() {
        assert_eq!(
            HELP,
            "Arrows or drag: pan   +/-: zoom   Click: select   Return: enter"
        );
    }
}
