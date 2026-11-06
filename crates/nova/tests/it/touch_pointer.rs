//! A finger through the whole app
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps): its lift is the
//! pointer leaving, so after a tap in space and a dialog over flight the
//! app's resync tells flight the pointer left, not the old finger point,
//! and the cursor stays hidden. The router's leave is `nova_view`'s and
//! the app's delivery is pinned in `app::tests::touch`; this is wiring.

use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, start_screen};
use nova_data::GameData;
use nova_render::FontFaces;
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_view::{FingerId, Key};

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
}

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
