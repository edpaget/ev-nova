//! The screen router: every screen the app can show, as one [`Screen`].
//!
//! The router keeps two sides alive, the ship browser and the
//! [`Navigator`] (the galaxy map and the system opened from it), and shows
//! one at a time. Tab switches sides, so each keeps its state (the
//! selected ship; the map's view and selection, and the open system with
//! its camera) while hidden. The router also decides what Escape does:
//! back one level, from a system to the map, and quit at the top.
//!
//! F, from either side, enters flight: the [`FlightView`], built the first
//! time and kept, so flight resumes where it left off. In flight the
//! arrow keys fly the ship, Tab does nothing, and Escape goes back to the
//! screen flight was entered from; it never quits.
//!
//! I, outside flight, opens the About text in the game's "Desc Dialog"
//! over the screen shown, when the router was given the interface file's
//! dialogs ([`AppScreen::with_dialogs`]). The dialog is modal: it takes
//! every input until Done (Return, Escape or a click) closes it.

use std::rc::Rc;
use std::time::Duration;

use nova_data::GameData;
use nova_view::flight::FlightView;
use nova_view::galaxy::GalaxyMap;
use nova_view::ships::ShipBrowser;
use nova_view::system::SystemView;
use nova_view::text::TextMetrics;
use nova_view::ui::desc::DESC_DIALOG;
use nova_view::ui::{DescDialog, DescriptionSource, DialogResources};
use nova_view::{Color, DrawList, Input, Key, Navigator, Point, Screen, ScreenAction};

/// The hint the router draws over every screen, where it goes, its size and
/// its colour. Every screen leaves that corner free. The hint offers I
/// only when the router has dialogs, and so I does something; without
/// them it draws [`HINT_WITHOUT_DIALOGS`].
pub const HINT: &str = "Tab: ships / galaxy map   F: fly   I: about";
pub const HINT_WITHOUT_DIALOGS: &str = "Tab: ships / galaxy map   F: fly";
pub const HINT_AT: Point = Point::new(16.0, 8.0);
pub const HINT_SIZE: f32 = 14.0;
pub const HINT_COLOR: Color = Color::DIM;

/// The About text's `dësc`.
pub const ABOUT_TEXT: i16 = 32767;

/// Which screen the app is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Showing {
    /// The ship browser.
    ShipBrowser,
    /// The galaxy map.
    GalaxyMap,
    /// A system opened from the galaxy map.
    System,
    /// The player's ship in flight.
    Flight,
    /// The About text, over another screen.
    About,
}

/// The two sides Tab switches between, and flight, entered from either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Ships,
    Galaxy,
    Flight,
}

/// Every screen the app can show, and which side it is showing. The router
/// forwards input, ticks and drawing to the side shown, switches sides on
/// a Tab press (not on its key repeats) and decides what an Escape press
/// does.
#[derive(Clone, Debug)]
pub struct AppScreen {
    side: Side,
    /// The side flight was last entered from, which Escape goes back to.
    return_to: Side,
    /// The game data, which flight reads when it is first entered.
    data: Rc<GameData>,
    /// The ship browser, reading the game data the renderer draws from.
    ships: ShipBrowser<Rc<GameData>>,
    /// The galaxy map and any system opened from it, reading the same game
    /// data.
    galaxy: Navigator<Rc<GameData>>,
    /// Flight, once entered.
    flight: Option<FlightView>,
    /// What dialogs are built from, once given.
    dialogs: Option<Dialogs>,
    /// The About dialog, while it is open.
    about: Option<DescDialog>,
}

/// The interface file's dialogs and the metrics their text is laid out by.
#[derive(Clone)]
struct Dialogs {
    resources: Rc<dyn DialogResources>,
    metrics: Rc<dyn TextMetrics>,
}

impl std::fmt::Debug for Dialogs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialogs").finish_non_exhaustive()
    }
}

impl AppScreen {
    /// Every screen over `data`, showing the ship browser.
    #[must_use]
    pub fn new(data: Rc<GameData>) -> Self {
        Self {
            side: Side::Ships,
            return_to: Side::Ships,
            galaxy: Navigator::new(Rc::clone(&data)),
            ships: ShipBrowser::new(Rc::clone(&data)),
            data,
            flight: None,
            dialogs: None,
            about: None,
        }
    }

    /// The router with dialogs: I opens the About text in a dialog built
    /// from `dialogs`, its text laid out by `metrics`. Without dialogs, I
    /// does nothing.
    #[must_use]
    pub fn with_dialogs(
        self,
        dialogs: Rc<dyn DialogResources>,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        Self {
            dialogs: Some(Dialogs {
                resources: dialogs,
                metrics,
            }),
            ..self
        }
    }

    /// The About dialog, while it is open.
    #[must_use]
    pub fn about(&self) -> Option<&DescDialog> {
        self.about.as_ref()
    }

    /// Which screen is showing.
    #[must_use]
    pub fn showing(&self) -> Showing {
        if self.about.is_some() {
            return Showing::About;
        }
        match self.side {
            Side::Ships => Showing::ShipBrowser,
            Side::Galaxy if self.galaxy.system().is_some() => Showing::System,
            Side::Galaxy => Showing::GalaxyMap,
            Side::Flight => Showing::Flight,
        }
    }

    /// The ship browser, shown or not.
    #[must_use]
    pub fn ship_browser(&self) -> &ShipBrowser<Rc<GameData>> {
        &self.ships
    }

    /// The galaxy map, shown or not.
    #[must_use]
    pub fn galaxy_map(&self) -> &GalaxyMap {
        self.galaxy.map()
    }

    /// The open system's view, shown or not, if a system is open.
    #[must_use]
    pub fn system_view(&self) -> Option<&SystemView> {
        self.galaxy.system()
    }

    /// The flight screen, shown or not, once flight has been entered.
    #[must_use]
    pub fn flight_view(&self) -> Option<&FlightView> {
        self.flight.as_ref()
    }

    /// The galaxy side: the map and any open system.
    #[must_use]
    pub fn navigator(&self) -> &Navigator<Rc<GameData>> {
        &self.galaxy
    }

    fn shown(&self) -> &dyn Screen {
        match self.side {
            Side::Ships => &self.ships,
            Side::Galaxy => &self.galaxy,
            Side::Flight => self.flight.as_ref().expect(ENTERED),
        }
    }

    fn shown_mut(&mut self) -> &mut dyn Screen {
        match self.side {
            Side::Ships => &mut self.ships,
            Side::Galaxy => &mut self.galaxy,
            Side::Flight => self.flight.as_mut().expect(ENTERED),
        }
    }

    /// Hides the side shown, first cancelling its pointer gesture and
    /// letting go of its keys (their releases will go elsewhere), and shows
    /// `side`.
    fn switch_to(&mut self, side: Side) {
        let hidden = self.shown_mut();
        hidden.cancel_pointer();
        hidden.release_keys();
        self.side = side;
    }

    /// Shows flight, building it the first time, and remembers the side to
    /// go back to.
    fn enter_flight(&mut self) {
        let data = &self.data;
        self.flight
            .get_or_insert_with(|| FlightView::new(data.as_ref()));
        self.return_to = self.side;
        self.switch_to(Side::Flight);
    }

    /// Opens the About dialog over the side shown, first cancelling its
    /// pointer gesture and letting go of its keys (their releases will go
    /// to the dialog). With no dialogs, nothing opens; when the dialog
    /// cannot be built, nothing opens and the reason goes to stderr.
    fn open_about(&mut self) {
        let Some(dialogs) = &self.dialogs else {
            return;
        };
        match about_dialog(dialogs, &self.data) {
            Ok(dialog) => {
                let below = self.shown_mut();
                below.cancel_pointer();
                below.release_keys();
                self.about = Some(dialog);
            }
            Err(reason) => eprintln!("nova: cannot show the About text: {reason}"),
        }
    }

    /// The About dialog's input; it closes once Done is activated.
    fn about_input(&mut self, input: &Input) -> ScreenAction {
        if let Some(about) = &mut self.about {
            about.input(input);
            if about.closed() {
                self.about = None;
            }
        }
        ScreenAction::None
    }

    /// Flight's input: an Escape press goes back, and everything else goes
    /// to flight (which ignores Tab).
    fn flight_input(&mut self, input: &Input) -> ScreenAction {
        match *input {
            Input::Key {
                key: Key::Escape,
                pressed,
                repeat,
            } => {
                if pressed && !repeat {
                    self.switch_to(self.return_to);
                }
                ScreenAction::None
            }
            _ => self.shown_mut().input(input),
        }
    }
}

/// "Desc Dialog" showing the About text, from the dialogs and the game
/// data.
fn about_dialog(dialogs: &Dialogs, data: &GameData) -> Result<DescDialog, String> {
    let template = dialogs.resources.dialog_template(DESC_DIALOG)?;
    let text = data.description(ABOUT_TEXT)?;
    Ok(DescDialog::new(
        &template,
        &text,
        data.button_style(),
        Rc::clone(&dialogs.metrics),
    ))
}

/// Flight shows only once it has been built.
const ENTERED: &str = "flight is built when it is entered";

/// The screen the app opens on: every screen over `data`, showing the ship
/// browser.
#[must_use]
pub fn start_screen(data: Rc<GameData>) -> AppScreen {
    AppScreen::new(data)
}

impl Screen for AppScreen {
    /// A Tab press switches sides, first cancelling any pointer gesture on
    /// the side it hides and letting go of the keys it holds (their
    /// releases will now go to the other side). Holding Tab switches once:
    /// its key repeats are consumed without switching, as is its release.
    ///
    /// An Escape press goes back from an open system to the map, and
    /// otherwise quits; its repeats and release are consumed, so holding
    /// Escape goes back once and never quits. An F press enters flight,
    /// cancelling and letting go on the side it hides as Tab does; holding
    /// F enters once. Everything else, repeats included, goes to the side
    /// shown.
    ///
    /// In flight, an Escape press goes back to the side flight was entered
    /// from, letting go of the keys held in flight; it never quits, and its
    /// repeats and release are consumed. Everything else goes to flight,
    /// where Tab does nothing.
    ///
    /// With dialogs, an I press outside flight opens the About dialog
    /// over the side shown, cancelling and letting go on it as Tab does.
    /// While the dialog is open, every event goes to it alone (Escape
    /// closes it and never quits).
    fn input(&mut self, input: &Input) -> ScreenAction {
        if self.about.is_some() {
            return self.about_input(input);
        }
        if self.side == Side::Flight {
            return self.flight_input(input);
        }
        if let Input::Key {
            key: Key::Char('i'),
            pressed,
            repeat,
        } = *input
            && self.dialogs.is_some()
        {
            if pressed && !repeat {
                self.open_about();
            }
            return ScreenAction::None;
        }
        if let Input::Key {
            key: key @ (Key::Tab | Key::Escape | Key::Char('f')),
            pressed,
            repeat,
        } = *input
        {
            if !pressed || repeat {
                return ScreenAction::None;
            }
            match key {
                Key::Escape if self.showing() == Showing::System => {
                    return self.galaxy.input(input);
                }
                Key::Escape => return ScreenAction::Quit,
                Key::Tab if self.side == Side::Ships => self.switch_to(Side::Galaxy),
                Key::Tab => self.switch_to(Side::Ships),
                _ => self.enter_flight(),
            }
            return ScreenAction::None;
        }
        self.shown_mut().input(input)
    }

    /// Only the side shown ticks; a hidden one is paused. While the About
    /// dialog is open, only it ticks.
    fn tick(&mut self, dt: Duration) {
        match &mut self.about {
            Some(about) => about.tick(dt),
            None => self.shown_mut().tick(dt),
        }
    }

    /// The side shown, then the hint (flight has its own help line
    /// instead), then the About dialog when it is open.
    fn draw(&self, list: &mut DrawList) {
        self.shown().draw(list);
        if self.side != Side::Flight {
            let hint = if self.dialogs.is_some() {
                HINT
            } else {
                HINT_WITHOUT_DIALOGS
            };
            list.text(hint, HINT_AT, HINT_SIZE, None, HINT_COLOR);
        }
        if let Some(about) = &self.about {
            about.draw(list);
        }
    }

    fn cancel_pointer(&mut self) {
        match &mut self.about {
            Some(about) => about.cancel_pointer(),
            None => self.shown_mut().cancel_pointer(),
        }
    }

    fn release_keys(&mut self) {
        match &mut self.about {
            Some(about) => about.release_keys(),
            None => self.shown_mut().release_keys(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;
    use std::rc::Rc;

    use nova_data::graphics::RLED;
    use nova_data::graphics::fixture::RledBuilder;
    use nova_data::records::character::Character;
    use nova_data::records::desc::Desc;
    use nova_data::records::ship::Ship;
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::records::spin::Spin;
    use nova_data::records::stellar::Stellar;
    use nova_data::records::system::System;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_data::{GameData, Record, SystemId};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader};
    use nova_view::galaxy::GalaxyMap;
    use nova_view::galaxy::map::ENTER_BUTTON;
    use nova_view::geometry::Bounds;
    use nova_view::ships::{ShipBrowser, ShipId};
    use nova_view::text::fixture::MonoMetrics;
    use nova_view::ui::{DialogTemplate, ItemSpec, ItemTemplate, Placement};
    use nova_view::{DrawCommand, Font, Key, MouseButton, Point};

    use super::*;

    /// One data file, `/data/Nova Data`, holding a fork.
    struct OneFile(Vec<u8>);

    impl DirLister for OneFile {
        fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
            Ok(vec![Listing {
                name: "Nova Data".into(),
                kind: EntryKind::File,
            }])
        }
    }

    impl ForkReader for OneFile {
        fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            Ok((fork == Fork::Data).then(|| self.0.clone()))
        }
    }

    /// An independent `sÿst` at (`x`, 0) with no hyperlinks and these
    /// stellars.
    fn system(x: i16, stellars: &[i16]) -> Vec<u8> {
        let mut bytes = vec![0; System::SIZE.expect("fixed")];
        bytes[0..2].copy_from_slice(&x.to_be_bytes());
        for slot in 0..32 {
            bytes[0x04 + 2 * slot..0x06 + 2 * slot].copy_from_slice(&(-1_i16).to_be_bytes());
        }
        for (slot, id) in stellars.iter().enumerate() {
            bytes[0x24 + 2 * slot..0x26 + 2 * slot].copy_from_slice(&id.to_be_bytes());
        }
        bytes[0x66..0x68].copy_from_slice(&(-1_i16).to_be_bytes());
        bytes
    }

    /// Ships 129 and 128, each with a `shän` naming a 4-frame `rlëD` (one
    /// set of 4 rotations), and systems 128 and 129, 300 apart. System 128
    /// holds stellar 128 at (0, 0), whose `spïn` 1000 names the same
    /// `rlëD`. The only `chär` starts in ship 128, an average ship, in
    /// system 128.
    fn data() -> Rc<GameData> {
        let mut anim = vec![0; ShipAnim::SIZE.expect("fixed")];
        anim[0x00..0x02].copy_from_slice(&1000_i16.to_be_bytes());
        anim[0x04..0x06].copy_from_slice(&1_i16.to_be_bytes());
        anim[0x34..0x36].copy_from_slice(&4_i16.to_be_bytes());
        let sheet = (0..4)
            .fold(RledBuilder::new(1, 1), |sheet, _| {
                sheet.frame(|f| f.line().pixels(&[0x7C00]))
            })
            .build();
        let ship = vec![0; Ship::SIZE.expect("fixed")];
        let mut average = ship.clone();
        for (at, value) in [(0x04, 300_i16), (0x06, 300), (0x08, 10)] {
            average[at..at + 2].copy_from_slice(&value.to_be_bytes());
        }
        let mut character = vec![0; Character::SIZE.expect("fixed")];
        character[0x04..0x06].copy_from_slice(&128_i16.to_be_bytes());
        character[0x06..0x08].copy_from_slice(&128_i16.to_be_bytes());
        for slot in 1..4 {
            let at = 0x06 + 2 * slot;
            character[at..at + 2].copy_from_slice(&(-1_i16).to_be_bytes());
        }
        let mut spin = vec![0; Spin::SIZE.expect("fixed")];
        spin[0..2].copy_from_slice(&1000_i16.to_be_bytes());
        let fork = ForkBuilder::new()
            .resource(Ship::TYPE, 129, Some(b"Second"), &ship)
            .resource(Ship::TYPE, 128, Some(b"First"), &average)
            .resource(Character::TYPE, 128, Some(b"Pilot"), &character)
            .resource(ShipAnim::TYPE, 128, None, &anim)
            .resource(ShipAnim::TYPE, 129, None, &anim)
            .resource(RLED, 1000, None, &sheet)
            .resource(System::TYPE, 128, Some(b"Alpha"), &system(0, &[128]))
            .resource(System::TYPE, 129, Some(b"Beta"), &system(300, &[]))
            .resource(
                Stellar::TYPE,
                128,
                Some(b"Alpha Prime"),
                &vec![0; Stellar::SIZE.expect("fixed")],
            )
            .resource(Spin::TYPE, 1000, None, &spin)
            .resource(Desc::TYPE, ABOUT_TEXT, None, &about_text())
            .build()
            .bytes;
        let file = OneFile(fork);
        let data = GameData::load(&file, &file, Path::new("/data"), None).expect("opens");
        Rc::new(data)
    }

    /// The About text's `dësc`: ten short lines.
    fn about_text() -> Vec<u8> {
        let mut bytes: Vec<u8> = (0..10)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\r")
            .into_bytes();
        bytes.push(0);
        bytes.extend([0xFF; 2]);
        bytes.extend([0; 34]);
        bytes
    }

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    /// A key held down past the OS key-repeat delay: another press, marked
    /// as a repeat.
    fn held(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    fn drawn(screen: &impl Screen) -> DrawList {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list
    }

    /// Clicks the left button on system `id`'s dot.
    fn click_system(screen: &mut AppScreen, id: i16) {
        let map = screen.galaxy_map();
        let position = map
            .model()
            .system(SystemId(id))
            .expect("a system")
            .position();
        let at = map.view().world_to_screen(position);
        for pressed in [true, false] {
            let input = Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            };
            assert_eq!(screen.input(&input), ScreenAction::None);
        }
    }

    #[test]
    fn the_app_starts_on_the_ship_browser_at_the_first_ship() {
        let screen = start_screen(data());
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        assert_eq!(screen.ship_browser().selected(), Some(ShipId(128)));
        assert_eq!(screen.galaxy_map().selected(), None);
        assert_eq!(screen.galaxy_map().model().systems().len(), 2);
    }

    #[test]
    fn a_tab_press_switches_screens_and_its_release_does_nothing() {
        use Showing::{GalaxyMap as Map, ShipBrowser as Ships};
        let mut screen = AppScreen::new(data());
        let mut showing = Vec::new();
        for input in [
            key(Key::Tab, true),
            key(Key::Tab, false),
            key(Key::Tab, true),
            key(Key::Tab, false),
            key(Key::Tab, true),
        ] {
            assert_eq!(screen.input(&input), ScreenAction::None);
            showing.push(screen.showing());
        }
        assert_eq!(showing, [Map, Map, Ships, Ships, Map]);
    }

    #[test]
    fn holding_tab_switches_once() {
        use Showing::{GalaxyMap as Map, ShipBrowser as Ships};
        let mut screen = AppScreen::new(data());
        let mut showing = Vec::new();
        for input in [
            key(Key::Tab, true),
            held(Key::Tab),
            held(Key::Tab),
            held(Key::Tab),
            key(Key::Tab, false),
            key(Key::Tab, true),
        ] {
            assert_eq!(screen.input(&input), ScreenAction::None);
            showing.push(screen.showing());
        }
        assert_eq!(showing, [Map, Map, Map, Map, Map, Ships]);
    }

    #[test]
    fn a_held_tab_reaches_neither_screen() {
        let mut screen = AppScreen::new(data());
        screen.input(&key(Key::Tab, true));
        let map = screen.galaxy_map();
        let view = *map.view();
        let at = map.view().world_to_screen(
            map.model()
                .system(SystemId(129))
                .expect("a system")
                .position(),
        );
        let left = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };

        // A drag on the map survives a held Tab: no switch, so no cancel.
        screen.input(&left(true));
        screen.input(&held(Key::Tab));
        screen.input(&Input::PointerMoved(Point::new(at.x + 50.0, at.y)));
        assert_eq!(screen.showing(), Showing::GalaxyMap);
        assert_ne!(*screen.galaxy_map().view(), view, "the drag goes on");
    }

    #[test]
    fn held_keys_reach_the_screen_shown() {
        let mut screen = AppScreen::new(data());
        screen.input(&held(Key::Right));
        assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)));
        screen.input(&key(Key::Tab, true));
        let fitted = *screen.galaxy_map().view();
        screen.input(&held(Key::Right));
        assert_ne!(*screen.galaxy_map().view(), fitted, "the map pans");
    }

    #[test]
    fn keys_reach_only_the_screen_shown() {
        let mut screen = AppScreen::new(data());
        let fitted = *screen.galaxy_map().view();
        assert_eq!(screen.input(&key(Key::Right, true)), ScreenAction::None);
        assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)));
        assert_eq!(*screen.galaxy_map().view(), fitted, "the map stays put");

        screen.input(&key(Key::Tab, true));
        screen.input(&key(Key::Right, true));
        assert_ne!(*screen.galaxy_map().view(), fitted, "the map pans");
        assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)));
    }

    #[test]
    fn each_screen_keeps_its_state_across_a_round_trip() {
        let mut screen = AppScreen::new(data());
        screen.input(&key(Key::Right, true));
        screen.input(&key(Key::Tab, true));
        click_system(&mut screen, 129);
        screen.input(&key(Key::Char('='), true));
        let (view, selected) = (*screen.galaxy_map().view(), screen.galaxy_map().selected());
        assert_eq!(selected, Some(SystemId(129)));

        screen.input(&key(Key::Tab, true));
        assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)));
        screen.input(&key(Key::Tab, true));
        assert_eq!(*screen.galaxy_map().view(), view);
        assert_eq!(screen.galaxy_map().selected(), selected);
    }

    #[test]
    fn switching_away_cancels_a_drag_whose_release_the_other_screen_gets() {
        let mut screen = AppScreen::new(data());
        screen.input(&key(Key::Tab, true));
        let map = screen.galaxy_map();
        let view = *map.view();
        let at = map.view().world_to_screen(
            map.model()
                .system(SystemId(129))
                .expect("a system")
                .position(),
        );
        let left = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };

        screen.input(&left(true));
        screen.input(&key(Key::Tab, true));
        screen.input(&left(false));
        screen.input(&key(Key::Tab, true));
        assert_eq!(screen.showing(), Showing::GalaxyMap);
        screen.input(&Input::PointerMoved(Point::new(at.x + 50.0, at.y)));
        assert_eq!(*screen.galaxy_map().view(), view, "no drag");
        assert_eq!(screen.galaxy_map().selected(), None);
    }

    #[test]
    fn an_escape_press_quits_from_either_screen() {
        let mut screen = AppScreen::new(data());
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
        screen.input(&key(Key::Tab, true));
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
    }

    #[test]
    fn an_escape_repeat_or_release_does_nothing() {
        let mut screen = AppScreen::new(data());
        assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
        assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::ShipBrowser);
    }

    /// Opens the map, selects system `id` and enters it with Return.
    fn enter(screen: &mut AppScreen, id: i16) {
        screen.input(&key(Key::Tab, true));
        click_system(screen, id);
        assert_eq!(screen.input(&key(Key::Enter, true)), ScreenAction::None);
    }

    fn camera(screen: &AppScreen) -> Point {
        screen
            .system_view()
            .expect("a system open")
            .camera()
            .center()
    }

    #[test]
    fn return_on_a_selected_system_shows_it() {
        let mut screen = AppScreen::new(data());
        assert_eq!(screen.system_view().map(|v| v.scene().id()), None);
        enter(&mut screen, 128);
        assert_eq!(screen.showing(), Showing::System);
        let view = screen.system_view().expect("a system open");
        assert_eq!(view.scene().id(), SystemId(128));
        assert_eq!(view.scene().stellars().len(), 1);
        assert_eq!(
            screen.navigator().system().map(|v| v.scene().id()),
            Some(SystemId(128))
        );
    }

    #[test]
    fn the_enter_button_shows_the_selected_system() {
        let mut screen = AppScreen::new(data());
        screen.input(&key(Key::Tab, true));
        click_system(&mut screen, 129);
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: ENTER_BUTTON.center(),
            });
        }
        assert_eq!(screen.showing(), Showing::System);
    }

    #[test]
    fn escape_goes_back_to_the_map_then_quits() {
        let mut screen = AppScreen::new(data());
        enter(&mut screen, 128);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::GalaxyMap);
        assert_eq!(screen.galaxy_map().selected(), Some(SystemId(128)));
        assert!(screen.system_view().is_none());
        // Holding Escape after going back does not quit.
        assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
        assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::GalaxyMap);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
    }

    #[test]
    fn tab_from_a_system_goes_to_the_ships_and_back_to_it_as_it_was() {
        let mut screen = AppScreen::new(data());
        enter(&mut screen, 128);
        screen.input(&key(Key::Right, true));
        screen.tick(Duration::from_millis(250));
        screen.input(&key(Key::Right, false));
        let moved = camera(&screen);
        assert_eq!(moved, Point::new(240.0, 0.0));

        screen.input(&key(Key::Tab, true));
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
        screen.input(&key(Key::Tab, true));
        assert_eq!(screen.showing(), Showing::System);
        assert_eq!(camera(&screen), moved);
    }

    #[test]
    fn a_tab_switch_lets_go_of_the_keys_held_in_a_system() {
        let mut screen = AppScreen::new(data());
        enter(&mut screen, 128);
        screen.input(&key(Key::Down, true));
        screen.input(&key(Key::Tab, true));
        // Down's release goes to the ship browser.
        screen.input(&key(Key::Down, false));
        screen.input(&key(Key::Tab, true));
        screen.tick(Duration::from_millis(250));
        assert_eq!(camera(&screen), Point::new(0.0, 0.0), "no drift");
    }

    #[test]
    fn releasing_the_keys_reaches_the_screen_shown() {
        let mut screen = AppScreen::new(data());
        enter(&mut screen, 128);
        screen.input(&key(Key::Up, true));
        screen.release_keys();
        screen.tick(Duration::from_millis(250));
        assert_eq!(camera(&screen), Point::new(0.0, 0.0));
    }

    #[test]
    fn cancelling_the_pointer_reaches_the_screen_shown() {
        let mut screen = AppScreen::new(data());
        screen.input(&key(Key::Tab, true));
        let view = *screen.galaxy_map().view();
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: Point::new(100.0, 100.0),
        });
        screen.cancel_pointer();
        screen.input(&Input::PointerMoved(Point::new(150.0, 100.0)));
        assert_eq!(*screen.galaxy_map().view(), view, "no drag");
    }

    #[test]
    fn the_hint_is_drawn_over_the_system() {
        let data = data();
        let mut screen = AppScreen::new(Rc::clone(&data));
        enter(&mut screen, 128);
        let mut expected = drawn(screen.system_view().expect("open"));
        expected.push(hint());
        assert_eq!(drawn(&screen), expected);
    }

    #[test]
    fn only_the_screen_shown_ticks() {
        let mut screen = AppScreen::new(data());
        let tick = Duration::from_secs(1) / 30;
        screen.tick(tick);
        assert_eq!(screen.ship_browser().frame(), Some(1));
        screen.input(&key(Key::Tab, true));
        screen.tick(tick);
        assert_eq!(
            screen.ship_browser().frame(),
            Some(1),
            "paused while hidden"
        );
        screen.input(&key(Key::Tab, true));
        screen.tick(tick);
        assert_eq!(screen.ship_browser().frame(), Some(2));

        // Nor while a system is shown.
        enter(&mut screen, 128);
        screen.tick(tick);
        assert_eq!(screen.ship_browser().frame(), Some(2));
        screen.input(&key(Key::Tab, true));
        screen.tick(tick);
        assert_eq!(screen.ship_browser().frame(), Some(3));
    }

    /// The hint line of a router with no dialogs.
    fn hint() -> DrawCommand {
        DrawCommand::Text {
            text: HINT_WITHOUT_DIALOGS.to_owned(),
            font: Font::Geneva,
            origin: HINT_AT,
            size: HINT_SIZE,
            wrap_width: None,
            color: HINT_COLOR,
        }
    }

    #[test]
    fn it_draws_the_screen_shown_then_the_hint() {
        let data = data();
        let mut screen = AppScreen::new(Rc::clone(&data));
        let mut ships = ShipBrowser::new(Rc::clone(&data));
        let mut expected = drawn(&ships);
        expected.push(hint());
        assert_eq!(drawn(&screen), expected);

        screen.input(&key(Key::Tab, true));
        let mut expected = drawn(&GalaxyMap::new(&data));
        expected.push(hint());
        assert_eq!(drawn(&screen), expected);

        screen.input(&key(Key::Tab, true));
        ships.tick(Duration::from_millis(100));
        screen.tick(Duration::from_millis(100));
        let mut expected = drawn(&ships);
        expected.push(hint());
        assert_eq!(drawn(&screen), expected);
        assert_eq!(HINT, "Tab: ships / galaxy map   F: fly   I: about");
        assert_eq!((HINT_AT, HINT_SIZE), (Point::new(16.0, 8.0), 14.0));
    }

    // Flight.

    /// Enters flight with an F press and checks it shows.
    fn fly(screen: &mut AppScreen) {
        assert_eq!(screen.input(&key(Key::Char('f'), true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::Flight);
    }

    fn flight(screen: &AppScreen) -> &FlightView {
        screen.flight_view().expect("flight entered")
    }

    fn ship(screen: &AppScreen) -> nova_sim::ShipState {
        *flight(screen).session().expect("flying").player()
    }

    const TICK: Duration = nova_sim::TICK;

    #[test]
    fn f_enters_flight_from_each_screen_and_escape_goes_back_to_it() {
        let mut screen = AppScreen::new(data());
        assert!(screen.flight_view().is_none(), "not until entered");
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.system(), SystemId(128));
        assert_eq!(session.ship(), nova_sim::ShipId(128));
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::ShipBrowser);

        screen.input(&key(Key::Tab, true));
        fly(&mut screen);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::GalaxyMap);

        click_system(&mut screen, 128);
        screen.input(&key(Key::Enter, true));
        fly(&mut screen);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::System);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::GalaxyMap);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
    }

    #[test]
    fn an_f_repeat_or_release_does_not_enter_flight() {
        let mut screen = AppScreen::new(data());
        assert_eq!(screen.input(&held(Key::Char('f'))), ScreenAction::None);
        assert_eq!(
            screen.input(&key(Key::Char('f'), false)),
            ScreenAction::None
        );
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        assert!(screen.flight_view().is_none());
    }

    #[test]
    fn escape_in_flight_never_quits_and_its_repeats_and_release_do_nothing() {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
        assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        // Holding Escape after leaving does not quit either.
        assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
        assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::ShipBrowser);
    }

    #[test]
    fn tab_does_nothing_in_flight() {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        for input in [key(Key::Tab, true), held(Key::Tab), key(Key::Tab, false)] {
            assert_eq!(screen.input(&input), ScreenAction::None);
            assert_eq!(screen.showing(), Showing::Flight);
        }
    }

    #[test]
    fn the_flight_keys_fly_the_ship_and_flight_resumes_where_it_left_off() {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        screen.tick(TICK * 10);
        let flown = ship(&screen);
        assert!(flown.position.y < 0.0, "{flown:?}");
        assert_eq!(
            screen.ship_browser().selected(),
            Some(ShipId(128)),
            "the keys went to flight"
        );
        screen.input(&key(Key::Escape, true));
        screen.input(&key(Key::Char('f'), true));
        assert_eq!(ship(&screen), flown, "the same flight");
    }

    #[test]
    fn entering_and_leaving_flight_let_go_of_the_keys_held() {
        let mut screen = AppScreen::new(data());
        enter(&mut screen, 128);
        screen.input(&key(Key::Right, true));
        fly(&mut screen);
        // Right's release goes to flight.
        screen.input(&key(Key::Right, false));
        screen.input(&key(Key::Escape, true));
        screen.tick(Duration::from_millis(250));
        assert_eq!(camera(&screen), Point::new(0.0, 0.0), "no drift");

        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        screen.tick(TICK * 5);
        screen.input(&key(Key::Escape, true));
        // Up's release goes to the system view.
        screen.input(&key(Key::Up, false));
        fly(&mut screen);
        let coasting = ship(&screen);
        screen.tick(TICK * 5);
        assert_eq!(ship(&screen).velocity, coasting.velocity, "no thrust");
    }

    #[test]
    fn entering_flight_cancels_a_drag_on_the_map() {
        let mut screen = AppScreen::new(data());
        screen.input(&key(Key::Tab, true));
        let view = *screen.galaxy_map().view();
        let left = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: Point::new(100.0, 100.0),
        };
        screen.input(&left(true));
        fly(&mut screen);
        screen.input(&left(false));
        screen.input(&key(Key::Escape, true));
        screen.input(&Input::PointerMoved(Point::new(150.0, 100.0)));
        assert_eq!(*screen.galaxy_map().view(), view, "no drag");
    }

    #[test]
    fn only_flight_ticks_while_it_is_shown() {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        screen.tick(TICK);
        assert_eq!(screen.ship_browser().frame(), Some(0), "paused");
        let moving = ship(&screen);
        assert_ne!(moving, nova_sim::ShipState::default());
        screen.input(&key(Key::Escape, true));
        screen.tick(TICK * 3);
        assert_eq!(ship(&screen), moving, "paused while hidden");
        assert_eq!(screen.ship_browser().frame(), Some(3));
    }

    #[test]
    fn releasing_the_keys_reaches_flight() {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        screen.release_keys();
        screen.tick(TICK * 3);
        assert_eq!(ship(&screen), nova_sim::ShipState::default());
    }

    #[test]
    fn flight_is_drawn_without_the_hint() {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        screen.tick(TICK);
        assert_eq!(drawn(&screen), drawn(flight(&screen)));
        screen.input(&key(Key::Escape, true));
        let list = drawn(&screen);
        assert_eq!(list.iter().last(), Some(&hint()));
    }

    // The About dialog.

    /// Dialog templates by ID: "Desc Dialog" (3003) alone, 200 x 100 at
    /// (0, 0), with Done (1) and a 100 x 36 text box (3): three lines of
    /// the About text show.
    struct Dialogs;

    impl DialogResources for Dialogs {
        fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
            if id != DESC_DIALOG {
                return Err(format!("no DLOG {id}"));
            }
            let item = |x, y, w, h, kind| ItemTemplate {
                bounds: Bounds::at(Point::new(x, y), w, h),
                enabled: true,
                kind,
            };
            Ok(DialogTemplate {
                bounds: Bounds::at(Point::new(0.0, 0.0), 200.0, 100.0),
                placement: Placement::Fixed,
                items: vec![
                    item(100.0, 70.0, 99.0, 25.0, ItemSpec::User),
                    item(10.0, 150.0, 20.0, 20.0, ItemSpec::Picture(1431)),
                    item(10.0, 10.0, 100.0, 36.0, ItemSpec::User),
                ],
            })
        }
    }

    /// No dialogs at all.
    struct NoDialogs;

    impl DialogResources for NoDialogs {
        fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
            Err(format!("no DLOG {id}"))
        }
    }

    fn with_dialogs(data: Rc<GameData>) -> AppScreen {
        AppScreen::new(data).with_dialogs(Rc::new(Dialogs), Rc::new(MonoMetrics))
    }

    fn open_about(screen: &mut AppScreen) {
        assert_eq!(screen.input(&key(Key::Char('i'), true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::About);
    }

    fn about(screen: &AppScreen) -> &DescDialog {
        screen.about().expect("the About dialog is open")
    }

    #[test]
    fn i_opens_the_about_text_over_the_side_shown() {
        let data = data();
        let mut screen = with_dialogs(Rc::clone(&data));
        assert!(screen.about().is_none());
        let below = drawn(&screen);
        open_about(&mut screen);
        assert_eq!(about(&screen).lines()[..2], ["line 0", "line 1"]);
        assert_eq!(about(&screen).lines().len(), 10);
        // The side, the hint, then the dialog.
        let list = drawn(&screen);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        let below: Vec<&DrawCommand> = below.iter().collect();
        assert_eq!(commands[..below.len()], below[..]);
        assert_eq!(
            drawn(about(&screen)).iter().collect::<Vec<_>>(),
            commands[below.len()..]
        );

        // From the galaxy map too.
        let mut screen = with_dialogs(data);
        screen.input(&key(Key::Tab, true));
        open_about(&mut screen);
    }

    #[test]
    fn the_about_dialog_uses_the_game_s_button_style() {
        let mut screen = with_dialogs(data());
        open_about(&mut screen);
        let list = drawn(about(&screen));
        let done = list
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text {
                    text, font, size, ..
                } if text == "Done" => Some((*font, *size)),
                _ => None,
            })
            .expect("the Done label");
        assert_eq!(done, (Font::Charcoal, 12.0), "stock, as there is no cölr");
    }

    #[test]
    fn without_dialogs_i_does_nothing() {
        let mut screen = AppScreen::new(data());
        let before = drawn(&screen);
        assert_eq!(screen.input(&key(Key::Char('i'), true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        assert!(screen.about().is_none());
        assert_eq!(drawn(&screen), before);
    }

    /// The text of the last thing `screen` draws: the hint.
    fn hint_text(screen: &AppScreen) -> String {
        match drawn(screen).iter().last() {
            Some(DrawCommand::Text { text, .. }) => text.clone(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_hint_offers_i_only_with_dialogs() {
        let data = data();
        let without = hint_text(&AppScreen::new(Rc::clone(&data)));
        assert_eq!(without, "Tab: ships / galaxy map   F: fly");
        assert!(!without.contains("I:"), "{without}");
        assert_eq!(
            hint_text(&with_dialogs(data)),
            "Tab: ships / galaxy map   F: fly   I: about"
        );
    }

    #[test]
    fn a_missing_dialog_or_text_opens_nothing() {
        let mut screen =
            AppScreen::new(data()).with_dialogs(Rc::new(NoDialogs), Rc::new(MonoMetrics));
        screen.input(&key(Key::Char('i'), true));
        assert_eq!(screen.showing(), Showing::ShipBrowser);

        let file = OneFile(ForkBuilder::new().build().bytes);
        let empty = Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"));
        let mut screen = with_dialogs(empty);
        screen.input(&key(Key::Char('i'), true));
        assert!(screen.about().is_none());
    }

    #[test]
    fn an_i_repeat_or_release_opens_nothing() {
        let mut screen = with_dialogs(data());
        screen.input(&key(Key::Char('i'), false));
        screen.input(&held(Key::Char('i')));
        assert_eq!(screen.showing(), Showing::ShipBrowser);
    }

    #[test]
    fn i_does_nothing_in_flight() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        screen.input(&key(Key::Char('i'), true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(screen.about().is_none());
    }

    #[test]
    fn while_open_input_reaches_only_the_dialog() {
        let mut screen = with_dialogs(data());
        open_about(&mut screen);
        for input in [
            key(Key::Right, true),
            key(Key::Tab, true),
            key(Key::Char('f'), true),
            key(Key::Char('i'), true),
        ] {
            assert_eq!(screen.input(&input), ScreenAction::None);
        }
        assert_eq!(screen.showing(), Showing::About);
        assert_eq!(screen.ship_browser().selected(), Some(ShipId(128)));
        assert!(screen.flight_view().is_none());
        screen.input(&key(Key::Down, true));
        screen.input(&held(Key::Down));
        assert_eq!(about(&screen).first_line(), 2);
    }

    #[test]
    fn escape_closes_the_dialog_and_never_quits() {
        let mut screen = with_dialogs(data());
        screen.input(&key(Key::Tab, true));
        open_about(&mut screen);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::GalaxyMap);
        assert!(screen.about().is_none());
        assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
    }

    #[test]
    fn return_or_a_click_on_done_closes_the_dialog() {
        let mut screen = with_dialogs(data());
        open_about(&mut screen);
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::ShipBrowser);

        open_about(&mut screen);
        let done = about(&screen)
            .dialog()
            .item_bounds(1)
            .expect("Done")
            .center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: done,
            });
        }
        assert_eq!(screen.showing(), Showing::ShipBrowser);
    }

    #[test]
    fn only_the_dialog_ticks_while_it_is_open() {
        let mut screen = with_dialogs(data());
        let tick = Duration::from_millis(100);
        screen.tick(tick);
        let frame = screen.ship_browser().frame();
        open_about(&mut screen);
        screen.tick(tick);
        assert_eq!(screen.ship_browser().frame(), frame, "paused below");
        screen.input(&key(Key::Escape, true));
        screen.tick(tick);
        assert_ne!(screen.ship_browser().frame(), frame);
    }

    #[test]
    fn opening_cancels_a_drag_and_lets_go_of_the_keys_below() {
        let mut screen = with_dialogs(data());
        enter(&mut screen, 128);
        let camera_at = camera(&screen);
        screen.input(&key(Key::Char('d'), true));
        open_about(&mut screen);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::System);
        screen.tick(Duration::from_secs(1));
        assert_eq!(camera(&screen), camera_at, "D was let go");

        let mut screen = with_dialogs(data());
        screen.input(&key(Key::Tab, true));
        let view = *screen.galaxy_map().view();
        let at = Point::new(500.0, 400.0);
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        });
        open_about(&mut screen);
        screen.input(&key(Key::Escape, true));
        screen.input(&Input::PointerMoved(Point::new(600.0, 400.0)));
        assert_eq!(*screen.galaxy_map().view(), view, "the drag was cancelled");
    }

    #[test]
    fn debug_shows_whether_there_are_dialogs() {
        let debug = format!("{:?}", with_dialogs(data()));
        assert!(debug.contains("dialogs: Some(Dialogs { .. })"), "{debug}");
        let debug = format!("{:?}", AppScreen::new(data()));
        assert!(debug.contains("dialogs: None"), "{debug}");
    }

    #[test]
    fn cancelling_and_releasing_reach_the_dialog_while_open() {
        let mut screen = with_dialogs(data());
        open_about(&mut screen);
        let done = about(&screen)
            .dialog()
            .item_bounds(1)
            .expect("Done")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        };
        screen.input(&button(true));
        screen.release_keys();
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.showing(), Showing::About, "the click was abandoned");
    }
}
