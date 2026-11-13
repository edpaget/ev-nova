//! A finger through the whole app
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps), as wiring:
//!
//! - its lift is the pointer leaving, so after a tap in space and a
//!   dialog over flight the app's resync tells flight the pointer left,
//!   not the old finger point, and the cursor stays hidden (the router's
//!   leave is `nova_view`'s, the app's delivery `app::tests::touch`'s);
//! - in the spaceport, the play keys' strip opens the player info screen
//!   and the galaxy map by touch alone (each opening on each port screen,
//!   mission info's among them, is `nova_view`'s
//!   `spaceport::view::tests::play_keys`).

use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, start_screen};
use nova_data::GameData;
use nova_flight::SessionViewExt as _;
use nova_render::FontFaces;
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_view::keymap::{Command, PlayCommand};
use nova_view::{FingerId, Key};

use crate::common::Frames;
use crate::recharge::{FakeWindow, game_data, interface};

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
}

impl Harness {
    /// The app flying from the ship browser with F, with its dialogs.
    fn flying() -> Self {
        let data = game_data(10_000);
        let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
        let screen = start_screen(Rc::clone(&data)).with_dialogs(Rc::new(interface()), metrics);
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

    fn press(&mut self, key: Key) {
        for pressed in [true, false] {
            self.send(WindowEvent::Key {
                key,
                pressed,
                repeat: false,
            });
        }
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn cursor_at(&self) -> Option<nova_view::Point> {
        self.app
            .screen()
            .flight_view()
            .expect("flight entered")
            .cursor_at()
    }

    fn frame(&mut self) {
        self.frames += 1;
        self.send(WindowEvent::Redraw {
            elapsed: Duration::from_millis(16 * self.frames),
        });
        assert_eq!(self.app.take_failures(), []);
    }

    /// One finger taps logical point `at` (window pixels here).
    fn tap(&mut self, at: nova_view::Point) {
        let finger = FingerId::new(7);
        let px = (f64::from(at.x), f64::from(at.y));
        self.send(WindowEvent::TouchStarted { finger, px });
        self.send(WindowEvent::TouchEnded { finger, px });
    }

    /// Presses L twice, as the player lands: the first requests
    /// clearance, the dock approach clears it, and the second lands.
    fn land(&mut self) {
        self.press(Key::KeyL);
        self.settle();
        self.settle_until("the dock approach's clearance", |game| {
            game.app
                .screen()
                .flight_view()
                .and_then(|flight| flight.session().ok())
                .is_some_and(nova_sim::Session::dock_cleared)
        });
        self.press(Key::KeyL);
        assert_eq!(self.showing(), Showing::Spaceport);
    }

    /// Where the spaceport's play key for `command` is, once a touch has
    /// told it touch was used last.
    fn play_key(&mut self, command: Command) -> nova_view::Point {
        self.tap(nova_view::Point::new(2.0, 2.0));
        let port = self.app.screen().spaceport_view().expect("landed");
        assert!(port.play_keys().shown(), "shown under touch");
        port.play_keys().rect(command).expect("a play key").center()
    }
}

crate::common::frames!(Harness, frame);

#[test]
fn a_dialog_closing_after_a_tap_leaves_the_cursor_hidden() {
    let mut game = Harness::flying();
    let finger = FingerId::new(1);
    let px = (300.0, 300.0);
    game.send(WindowEvent::TouchStarted { finger, px });
    assert!(game.cursor_at().is_some(), "under the finger");
    game.send(WindowEvent::TouchEnded { finger, px });
    assert_eq!(game.cursor_at(), None, "lifted");
    game.frame();
    game.press(Key::KeyP);
    assert_eq!(game.showing(), Showing::PlayerInfo);
    game.frame();
    game.press(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    game.frame();
    assert_eq!(game.cursor_at(), None, "after the close");
}

#[test]
fn in_the_spaceport_pilot_opens_the_player_info_screen_by_touch_alone() {
    let mut game = Harness::flying();
    game.land();
    let pilot = game.play_key(Command::Play(PlayCommand::PlayerInfo));
    game.tap(pilot);
    let port = game.app.screen().spaceport_view().expect("landed");
    assert!(port.player_info().is_some(), "player info open");
}

#[test]
fn in_the_spaceport_map_opens_the_galaxy_map_by_touch_alone() {
    let mut game = Harness::flying();
    game.land();
    let map = game.play_key(Command::Play(PlayCommand::GalaxyMap));
    game.tap(map);
    assert_eq!(game.showing(), Showing::FlightMap);
}
