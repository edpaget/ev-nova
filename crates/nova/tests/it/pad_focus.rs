//! The pad alone through the app: Preferences, opened from the menu bar,
//! reads the fake pad's d-pad, South and East through the router, the
//! app's pad poll and the device switch, and its toggle's change reaches
//! the settings store.
//!
//! Over `preferences`' synthetic "new prefs dialog". Where the focus
//! lands, and what each button does, is nova-view's, pinned there
//! (`ui::pad_focus::every_screen`); this pins that the pieces are joined.

use std::time::Duration;

use nova::app::{Showing, WindowEvent};
use nova::gamepad::fixture::FakeGamepads;
use nova::gamepad::{PadEvent, PadId};
use nova::settings::SettingsKeeper;
use nova_view::ui::prefs::Focus;
use nova_view::{ControllerFamily, Device, MenuCommand, PadButton};

use crate::preferences::{Harness, seeded};

const PAD: PadId = PadId(0);

/// The preferences harness reading the fake pads, and every window event
/// it was sent.
struct Padded {
    harness: Harness,
    pads: FakeGamepads,
    sent: Vec<WindowEvent>,
}

impl Padded {
    fn new(store: &nova_audio::recording::MemorySettings) -> Self {
        let pads = FakeGamepads::new();
        let Harness {
            app,
            window,
            gpu,
            log,
            frames,
        } = Harness::new(store);
        let harness = Harness {
            app: app.with_gamepads(Box::new(pads.clone())),
            window,
            gpu,
            log,
            frames,
        };
        Self {
            harness,
            pads,
            sent: Vec::new(),
        }
    }

    fn send(&mut self, event: WindowEvent) {
        self.sent.push(event);
        self.harness.send(event);
    }

    /// One frame, which polls the pads.
    fn frame(&mut self) {
        self.harness.frames += 1;
        let elapsed = Duration::from_millis(16 * self.harness.frames);
        self.send(WindowEvent::Redraw { elapsed });
    }

    /// `button` pressed, a frame, released, a frame.
    fn tap(&mut self, button: PadButton) {
        for pressed in [true, false] {
            self.pads.push(vec![PadEvent::Button {
                pad: PAD,
                button,
                pressed,
            }]);
            self.frame();
        }
    }
}

/// Whether `part` is one of the ten toggles.
fn toggle(part: Focus) -> bool {
    !matches!(
        part,
        Focus::Display
            | Focus::Scaling
            | Focus::EffectsVolume
            | Focus::MusicVolume
            | Focus::KeySettings
            | Focus::Ok
    )
}

#[test]
fn the_pad_alone_moves_the_preferences_focus_flips_a_toggle_and_closes_it() {
    let store = seeded();
    let mut app = Padded::new(&store);
    app.send(WindowEvent::Menu(MenuCommand::Preferences));
    assert_eq!(app.harness.showing(), Showing::Preferences);
    let after_menu = app.sent.len();

    app.pads.push(vec![PadEvent::Connected(PAD)]);
    app.frame();
    let mut moves = 0;
    while !app.harness.dialog().focus().is_some_and(toggle) {
        assert!(moves < 20, "the d-pad reaches a toggle");
        app.tap(PadButton::DPadUp);
        moves += 1;
    }
    assert_eq!(
        app.harness.dialog().device(),
        Device::Gamepad(ControllerFamily::Generic),
        "the pad was used last"
    );
    let part = app.harness.dialog().focus().expect("a toggle");
    let before = SettingsKeeper::open(store.clone()).0.settings();

    app.tap(PadButton::South);
    let after = SettingsKeeper::open(store.clone()).0.settings();
    assert_ne!(after, before, "{part:?} flipped in the store");

    app.tap(PadButton::East);
    assert_ne!(app.harness.showing(), Showing::Preferences, "closed");

    assert!(
        app.sent[after_menu..]
            .iter()
            .all(|event| matches!(event, WindowEvent::Redraw { .. })),
        "no key or pointer event after the menu command: {:?}",
        app.sent
    );
}
