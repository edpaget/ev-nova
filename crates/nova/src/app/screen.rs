//! The screen router: every screen the app can show, as one [`Screen`].
//!
//! The router keeps every screen alive and shows one at a time; Tab
//! switches between them, so each keeps its state (the selected ship, the
//! map's view and selection) while hidden.

use std::rc::Rc;
use std::time::Duration;

use nova_data::GameData;
use nova_view::galaxy::GalaxyMap;
use nova_view::ships::ShipBrowser;
use nova_view::{Color, DrawList, Input, Key, Point, Screen, ScreenAction};

/// The hint the router draws over every screen, where it goes, its size and
/// its colour. Every screen leaves that corner free.
pub const HINT: &str = "Tab: ships / galaxy map";
pub const HINT_AT: Point = Point::new(16.0, 8.0);
pub const HINT_SIZE: f32 = 14.0;
pub const HINT_COLOR: Color = Color::DIM;

/// Which screen the app is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Showing {
    /// The ship browser.
    ShipBrowser,
    /// The galaxy map.
    GalaxyMap,
}

/// Every screen the app can show, and which one it is showing. The router
/// forwards input, ticks and drawing to the screen shown, and switches
/// screens on a Tab press (not on its key repeats).
#[derive(Clone, Debug)]
pub struct AppScreen {
    showing: Showing,
    /// The ship browser, reading the game data the renderer draws from.
    ships: ShipBrowser<Rc<GameData>>,
    /// The galaxy map, read from the same game data when it was built.
    map: GalaxyMap,
}

impl AppScreen {
    /// Every screen over `data`, showing the ship browser.
    #[must_use]
    pub fn new(data: Rc<GameData>) -> Self {
        Self {
            showing: Showing::ShipBrowser,
            map: GalaxyMap::new(&data),
            ships: ShipBrowser::new(data),
        }
    }

    /// Which screen is showing.
    #[must_use]
    pub fn showing(&self) -> Showing {
        self.showing
    }

    /// The ship browser, shown or not.
    #[must_use]
    pub fn ship_browser(&self) -> &ShipBrowser<Rc<GameData>> {
        &self.ships
    }

    /// The galaxy map, shown or not.
    #[must_use]
    pub fn galaxy_map(&self) -> &GalaxyMap {
        &self.map
    }

    fn shown(&self) -> &dyn Screen {
        match self.showing {
            Showing::ShipBrowser => &self.ships,
            Showing::GalaxyMap => &self.map,
        }
    }

    fn shown_mut(&mut self) -> &mut dyn Screen {
        match self.showing {
            Showing::ShipBrowser => &mut self.ships,
            Showing::GalaxyMap => &mut self.map,
        }
    }
}

/// The screen the app opens on: every screen over `data`, showing the ship
/// browser.
#[must_use]
pub fn start_screen(data: Rc<GameData>) -> AppScreen {
    AppScreen::new(data)
}

impl Screen for AppScreen {
    /// A Tab press switches screens, first cancelling any pointer gesture on
    /// the screen it hides (whose button release will now go to the other
    /// screen). Holding Tab switches once: its key repeats are consumed
    /// without switching, as is its release. Everything else, repeats
    /// included, goes to the screen shown.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key: Key::Tab,
            pressed,
            repeat,
        } = *input
        {
            if pressed && !repeat {
                self.shown_mut().cancel_pointer();
                self.showing = match self.showing {
                    Showing::ShipBrowser => Showing::GalaxyMap,
                    Showing::GalaxyMap => Showing::ShipBrowser,
                };
            }
            return ScreenAction::None;
        }
        self.shown_mut().input(input)
    }

    /// Only the screen shown ticks; a hidden one is paused.
    fn tick(&mut self, dt: Duration) {
        self.shown_mut().tick(dt);
    }

    fn draw(&self, list: &mut DrawList) {
        self.shown().draw(list);
        list.text(HINT, HINT_AT, HINT_SIZE, None, HINT_COLOR);
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;
    use std::rc::Rc;

    use nova_data::graphics::RLED;
    use nova_data::graphics::fixture::RledBuilder;
    use nova_data::records::ship::Ship;
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::records::system::System;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_data::{GameData, Record, SystemId};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader};
    use nova_view::galaxy::GalaxyMap;
    use nova_view::ships::{ShipBrowser, ShipId};
    use nova_view::{DrawCommand, Key, MouseButton, Point};

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

    /// An independent `sÿst` at (`x`, 0) with no hyperlinks or stellars.
    fn system(x: i16) -> Vec<u8> {
        let mut bytes = vec![0; System::SIZE.expect("fixed")];
        bytes[0..2].copy_from_slice(&x.to_be_bytes());
        for slot in 0..32 {
            bytes[0x04 + 2 * slot..0x06 + 2 * slot].copy_from_slice(&(-1_i16).to_be_bytes());
        }
        bytes[0x66..0x68].copy_from_slice(&(-1_i16).to_be_bytes());
        bytes
    }

    /// Ships 129 and 128, each with a `shän` naming a 4-frame `rlëD` (one
    /// set of 4 rotations), and systems 128 and 129, 300 apart.
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
        let fork = ForkBuilder::new()
            .resource(Ship::TYPE, 129, Some(b"Second"), &ship)
            .resource(Ship::TYPE, 128, Some(b"First"), &ship)
            .resource(ShipAnim::TYPE, 128, None, &anim)
            .resource(ShipAnim::TYPE, 129, None, &anim)
            .resource(RLED, 1000, None, &sheet)
            .resource(System::TYPE, 128, Some(b"Alpha"), &system(0))
            .resource(System::TYPE, 129, Some(b"Beta"), &system(300))
            .build()
            .bytes;
        let file = OneFile(fork);
        let data = GameData::load(&file, &file, Path::new("/data"), None).expect("opens");
        Rc::new(data)
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
    }

    /// The router's hint line.
    fn hint() -> DrawCommand {
        DrawCommand::Text {
            text: HINT.to_owned(),
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
        assert_eq!(HINT, "Tab: ships / galaxy map");
        assert_eq!((HINT_AT, HINT_SIZE), (Point::new(16.0, 8.0), 14.0));
    }
}
