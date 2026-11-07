//! The app's bindings handed through the whole app: the router given a
//! rebound handle ([`AppScreen::with_bindings`]) hands it to every screen
//! it builds, so flight, a dialog over flight, the course map and the
//! spaceport each read the rebound keys, and their old keys no longer.
//! And the bindings a settings file saves reach flight as `main` wires
//! them: loaded by [`game_settings`] and handed to the router.
//!
//! Over the recharge module's synthetic data and interface file (a ship
//! starting over the one planet it lands on, and the stock "Spaceport"
//! dialog), driven by window events. What each screen does with a key is
//! nova-view's, pinned there.

use std::rc::Rc;
use std::time::Duration;

use nova::app::screen::AppScreen;
use nova::app::{App, Control, Showing, WindowEvent, start_screen};
use nova::settings::game_settings;
use nova_audio::recording::MemorySettings;
use nova_data::GameData;
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_render::{FontFaces, Frame};
use nova_view::keymap::{PlayCommand, UiCommand};
use nova_view::{Bindings, Key, KeyCombo, SharedBindings};

use crate::common::Frames;
use crate::recharge::{FakeWindow, game_data, interface};

/// The defaults with the map key on G, the player info key on Q and the
/// shared Back on the backquote, none of them bound beside those.
fn rebound() -> SharedBindings {
    SharedBindings::new(
        Bindings::default()
            .rebound(PlayCommand::GalaxyMap, KeyCombo::key(Key::KeyG))
            .rebound(PlayCommand::PlayerInfo, KeyCombo::key(Key::KeyQ))
            .rebound(UiCommand::Back, KeyCombo::key(Key::Backquote)),
    )
}

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
}

impl Harness {
    /// The app given `bindings`, flying from the ship browser with F.
    fn flying(bindings: SharedBindings) -> Self {
        let data = game_data(10_000);
        let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
        let screen = start_screen(Rc::clone(&data))
            .with_dialogs(Rc::new(interface()), metrics)
            .with_bindings(bindings);
        let mut harness = Self {
            app: App::new(&FakeWindow, data, screen),
            gpu: RecordingGpu::new(),
            frames: 0,
        };
        harness.press(Key::KeyF);
        assert_eq!(harness.showing(), Showing::Flight);
        harness.frame();
        harness
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue,
            "{event:?}"
        );
    }

    /// Presses and lets go of `key`.
    fn press(&mut self, key: Key) {
        for pressed in [true, false] {
            self.send(WindowEvent::Key {
                key,
                pressed,
                repeat: false,
            });
        }
    }

    fn screen(&self) -> &AppScreen {
        self.app.screen()
    }

    fn showing(&self) -> Showing {
        self.screen().showing()
    }

    /// Whether the spaceport shows its player info screen.
    fn port_player_info(&self) -> bool {
        self.screen()
            .spaceport_view()
            .is_some_and(|port| port.player_info().is_some())
    }

    fn frame(&mut self) -> Frame {
        self.frames += 1;
        self.send(WindowEvent::Redraw {
            elapsed: Duration::from_millis(16 * self.frames),
        });
        assert_eq!(self.app.take_failures(), []);
        (*self.gpu.submits().last().expect("a frame")).clone()
    }

    /// Presses L twice, as the player lands: the first requests
    /// clearance, the dock approach clears it, and the second lands.
    fn land(&mut self) {
        self.land_with(Key::KeyL);
    }

    /// Presses `key` twice, as [`Harness::land`] presses L, and gives the
    /// frames the dock approach took.
    fn land_with(&mut self, key: Key) -> u64 {
        self.press(key);
        self.settle();
        let start = self.frames;
        self.settle_until("the dock approach's clearance", Self::dock_cleared);
        let approach = self.frames - start;
        self.press(key);
        assert_eq!(self.showing(), Showing::Spaceport);
        approach
    }

    fn dock_cleared(&mut self) -> bool {
        self.screen()
            .flight_view()
            .and_then(|flight| flight.session().ok())
            .is_some_and(nova_sim::Session::dock_cleared)
    }
}

/// The app flying with the bindings `text`, a settings file, gives, wired
/// as `main` wires them.
fn flying_with_settings(text: &str) -> Harness {
    let (_, settings, warning) = game_settings(Some(Box::new(MemorySettings::holding(text))));
    assert_eq!(warning, None);
    Harness::flying(SharedBindings::new(settings.bindings))
}

crate::common::frames!(Harness, frame);

#[test]
fn flight_its_dialogs_its_map_and_the_spaceport_read_the_bindings_the_app_is_given() {
    let mut game = Harness::flying(rebound());

    // The course map, opened by flight and closed by the map's Back.
    game.press(Key::KeyM);
    assert_eq!(game.showing(), Showing::Flight, "M opens no map");
    game.press(Key::KeyG);
    assert_eq!(game.showing(), Showing::FlightMap, "G opens it");
    game.press(Key::Escape);
    assert_eq!(game.showing(), Showing::FlightMap, "Escape closes nothing");
    game.press(Key::Backquote);
    assert_eq!(game.showing(), Showing::Flight, "the backquote closes it");

    // A dialog over flight.
    game.press(Key::KeyP);
    assert_eq!(game.showing(), Showing::Flight, "P opens nothing");
    game.press(Key::KeyQ);
    assert_eq!(game.showing(), Showing::PlayerInfo, "Q opens it");
    game.press(Key::Escape);
    assert_eq!(game.showing(), Showing::PlayerInfo, "Escape closes nothing");
    game.press(Key::Backquote);
    assert_eq!(game.showing(), Showing::Flight, "the backquote closes it");
    game.settle();

    // The spaceport.
    game.land();
    game.press(Key::KeyP);
    assert!(!game.port_player_info(), "P opens nothing");
    game.press(Key::KeyQ);
    assert!(game.port_player_info(), "Q opens it");
    game.press(Key::Backquote);
    assert!(!game.port_player_info(), "the backquote closes it");
    game.press(Key::Escape);
    assert_eq!(game.showing(), Showing::Spaceport, "Escape doesn't leave");
    game.press(Key::Backquote);
    assert_eq!(game.showing(), Showing::Flight, "the backquote leaves");
}

#[test]
fn a_flight_key_the_settings_file_rebinds_acts_on_its_new_key_and_not_its_old() {
    let text = r#"{"bindings": {"keyboard": {"Flight.LandDock": "KeyK"}}}"#;
    let approach = flying_with_settings(text).land_with(Key::KeyK);

    let mut game = flying_with_settings(text);
    game.press(Key::KeyL);
    game.settle();
    for _ in 0..approach {
        game.frame();
    }
    assert!(!game.dock_cleared(), "L requests no clearance");
    game.press(Key::KeyL);
    assert_eq!(game.showing(), Showing::Flight, "L doesn't land");
}
