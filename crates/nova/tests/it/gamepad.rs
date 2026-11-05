//! The gamepads wired through the app: the runner's port, polled once a
//! frame by `App::with_gamepads`, reaches flight's step. A pad connected
//! mid-flight turns the ship with its right stick at the stick's fraction
//! of its turn rate, and a pad lost mid-flight lets go of its thrust and
//! turn.
//!
//! Over the recharge module's synthetic data, driven by window events and
//! the fake port. What each amount does to the step is nova-sim-base's and
//! the view's, pinned there; this pins that the pieces are joined.

use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, start_screen};
use nova::gamepad::fixture::FakeGamepads;
use nova::gamepad::{PadEvent, PadId};
use nova_data::GameData;
use nova_render::FontFaces;
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_view::keymap::FLIGHT_DEAD_ZONE;
use nova_view::{Key, PadAxis, PadButton};

use crate::recharge::{FakeWindow, game_data, interface};

const PAD: PadId = PadId(0);

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    pads: FakeGamepads,
    frames: u64,
}

impl Harness {
    /// The app reading the fake pads, flying from the ship browser with F.
    fn flying() -> Self {
        let data = game_data(10_000);
        let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
        let screen = start_screen(Rc::clone(&data)).with_dialogs(Rc::new(interface()), metrics);
        let pads = FakeGamepads::new();
        let mut harness = Self {
            app: App::new(&FakeWindow, data, screen).with_gamepads(Box::new(pads.clone())),
            gpu: RecordingGpu::new(),
            pads,
            frames: 0,
        };
        harness.key(Key::KeyF, true);
        harness.key(Key::KeyF, false);
        assert_eq!(harness.app.screen().showing(), Showing::Flight);
        harness.frames(5);
        harness
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue,
            "{event:?}"
        );
    }

    fn key(&mut self, key: Key, pressed: bool) {
        self.send(WindowEvent::Key {
            key,
            pressed,
            repeat: false,
        });
    }

    fn frames(&mut self, n: u64) {
        for _ in 0..n {
            self.frames += 1;
            self.send(WindowEvent::Redraw {
                elapsed: Duration::from_millis(16 * self.frames),
            });
        }
    }

    fn player(&self) -> nova_sim::ShipState {
        *self
            .app
            .screen()
            .flight_view()
            .and_then(|flight| flight.session().ok())
            .expect("flying")
            .player()
    }

    /// How far the ship turns right over `n` frames.
    fn turned_over(&mut self, n: u64) -> f32 {
        let start = self.player().heading;
        self.frames(n);
        (self.player().heading - start).rem_euclid(360.0)
    }
}

/// The raw stick value the flight dead zone reads as `amount`.
fn raw(amount: f32) -> f32 {
    FLIGHT_DEAD_ZONE + amount * (1.0 - FLIGHT_DEAD_ZONE)
}

const FRAMES: u64 = 60;

#[test]
fn a_pad_connected_mid_flight_turns_the_ship_at_about_half_its_turn_rate_with_the_right_stick_halfway()
 {
    let mut keyed = Harness::flying();
    keyed.key(Key::ArrowRight, true);
    let full = keyed.turned_over(FRAMES);
    assert!(full > 10.0, "the arrow turns it: {full}");

    let mut padded = Harness::flying();
    padded.pads.push(vec![
        PadEvent::Connected(PAD),
        PadEvent::Axis {
            pad: PAD,
            axis: PadAxis::RightX,
            value: raw(0.5),
        },
    ]);
    let half = padded.turned_over(FRAMES);
    let ratio = half / full;
    assert!((ratio - 0.5).abs() < 0.1, "{half} of {full}: {ratio}");
    assert!(
        padded.pads.polls() >= 5 + FRAMES as usize,
        "polled each frame"
    );
}

#[test]
fn a_pad_lost_mid_flight_stops_its_thrust_and_turn() {
    let mut harness = Harness::flying();
    harness.pads.push(vec![
        PadEvent::Connected(PAD),
        PadEvent::Axis {
            pad: PAD,
            axis: PadAxis::LeftY,
            value: 1.0,
        },
        PadEvent::Button {
            pad: PAD,
            button: PadButton::DPadRight,
            pressed: true,
        },
    ]);
    let turned = harness.turned_over(20);
    assert!(turned > 0.0, "the pad flies it");
    assert!(harness.player().velocity.length() > 0.0, "and thrusts");
    harness.pads.push(vec![PadEvent::Disconnected(PAD)]);
    harness.frames(1);
    let speed = harness.player().velocity.length();
    assert!(harness.turned_over(20).abs() < 1e-6, "no turn once lost");
    assert!(
        harness.player().velocity.length() <= speed + 1e-4,
        "no more thrust: {speed} then {}",
        harness.player().velocity.length()
    );
}

#[test]
fn select_and_start_leave_flight() {
    let mut harness = Harness::flying();
    let button = |button, pressed| PadEvent::Button {
        pad: PAD,
        button,
        pressed,
    };
    harness.pads.push(vec![
        PadEvent::Connected(PAD),
        button(PadButton::Select, true),
    ]);
    harness.frames(1);
    assert_eq!(
        harness.app.screen().showing(),
        Showing::Flight,
        "Select alone"
    );
    harness.pads.push(vec![button(PadButton::Start, true)]);
    harness.frames(1);
    assert_ne!(
        harness.app.screen().showing(),
        Showing::Flight,
        "the router's Pause took the pad's chord"
    );
}
