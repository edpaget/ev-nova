//! The gamepads: polled once a frame on a redraw, before the screen's
//! tick; what they did routed as screen input; the device used last told
//! as surroundings when it changes; what they did in the background
//! dropped as the focus comes back; and their input dropped while the
//! developer overlay shows.

use nova_view::{ControllerFamily, Device, PadAxis, PadButton, Surroundings};

use super::*;
use crate::gamepad::fixture::FakeGamepads;
use crate::gamepad::{PadEvent, PadId};

const ONE: PadId = PadId(0);

fn redraw_at(app: &mut TestApp, window: &mut FakeWindow, ms: u64) {
    let elapsed = Duration::from_millis(ms);
    handle(app, window, WindowEvent::Redraw { elapsed });
}

fn south(pressed: bool) -> PadEvent {
    PadEvent::Button {
        pad: ONE,
        button: PadButton::South,
        pressed,
    }
}

fn told(device: Device) -> Surroundings {
    Surroundings { device }
}

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

#[test]
fn each_redraw_polls_the_gamepads_once() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let pads = FakeGamepads::new();
    let mut app = app(&window).with_gamepads(Box::new(pads.clone()));
    for frame in 1..=3 {
        redraw_at(&mut app, &mut window, 16 * frame);
    }
    assert_eq!(pads.polls(), 3);
    handle(&mut app, &mut window, key(Key::KeyA, true));
    handle(
        &mut app,
        &mut window,
        WindowEvent::PointerMoved { px: (5.0, 5.0) },
    );
    assert_eq!(pads.polls(), 3, "other events poll nothing");
}

#[test]
fn a_pads_events_reach_the_screen_before_its_tick() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let pads = FakeGamepads::with([vec![
        PadEvent::Connected(ONE),
        south(true),
        PadEvent::Axis {
            pad: ONE,
            axis: PadAxis::LeftY,
            value: -0.9,
        },
    ]]);
    let mut app = app(&window).with_gamepads(Box::new(pads));
    redraw_at(&mut app, &mut window, 16);
    assert_eq!(
        app.screen().inputs,
        [
            Input::PadButton {
                button: PadButton::South,
                pressed: true
            },
            Input::PadAxis {
                axis: PadAxis::LeftY,
                value: -0.9
            },
            Input::PadButton {
                button: PadButton::LeftStickDown,
                pressed: true
            },
        ]
    );
    assert_eq!(app.screen().ticks.len(), 1);
    assert_eq!(app.screen().drawn_after.borrow()[0], 3, "before the draw");
}

#[test]
fn the_screen_hears_surroundings_when_the_device_changes() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let pads = FakeGamepads::new();
    let mut app = app(&window).with_gamepads(Box::new(pads.clone()));
    handle(&mut app, &mut window, key(Key::KeyA, true));
    assert_eq!(app.screen().surroundings, [], "the keyboard to start with");
    handle(
        &mut app,
        &mut window,
        WindowEvent::PointerMoved { px: (5.0, 5.0) },
    );
    handle(
        &mut app,
        &mut window,
        WindowEvent::PointerMoved { px: (6.0, 5.0) },
    );
    assert_eq!(app.screen().surroundings, [told(Device::Mouse)], "once");
    pads.push(vec![PadEvent::Connected(ONE), south(true)]);
    redraw_at(&mut app, &mut window, 16);
    assert_eq!(app.screen().surroundings, [told(Device::Mouse), told(PAD)]);
    handle(&mut app, &mut window, key(Key::KeyA, false));
    assert_eq!(app.screen().surroundings.len(), 2, "a release says nothing");
    handle(&mut app, &mut window, key(Key::KeyB, true));
    assert_eq!(
        app.screen().surroundings.last(),
        Some(&told(Device::Keyboard))
    );
    assert_eq!(
        app.screen().inputs.last(),
        Some(&Input::Key {
            key: Key::KeyB,
            pressed: true,
            repeat: false
        }),
        "the key itself reached the screen"
    );
}

#[test]
fn presses_queued_while_unfocused_are_dropped_on_focus_gained() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let pads = FakeGamepads::with([vec![PadEvent::Connected(ONE)]]);
    let mut app = app(&window).with_gamepads(Box::new(pads.clone()));
    redraw_at(&mut app, &mut window, 16);
    handle(&mut app, &mut window, WindowEvent::FocusLost);
    pads.push(vec![south(true)]);
    handle(&mut app, &mut window, WindowEvent::FocusGained);
    assert_eq!(pads.polls(), 2, "drained");
    redraw_at(&mut app, &mut window, 32);
    let pressed = |input: &Input| matches!(input, Input::PadButton { .. });
    assert!(
        !app.screen().inputs.iter().any(pressed),
        "{:?}",
        app.screen().inputs
    );
    pads.push(vec![south(false)]);
    redraw_at(&mut app, &mut window, 48);
    assert_eq!(
        app.screen().inputs.last(),
        Some(&Input::PadButton {
            button: PadButton::South,
            pressed: false
        }),
        "its release follows the press the pad state kept"
    );
}

#[test]
fn with_no_gamepads_a_redraw_routes_nothing() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = app(&window);
    redraw_at(&mut app, &mut window, 16);
    handle(&mut app, &mut window, WindowEvent::FocusGained);
    assert_eq!(app.screen().inputs, []);
    assert_eq!(app.screen().surroundings, []);
}

#[test]
fn a_screen_quitting_on_a_pad_press_exits() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let pads = FakeGamepads::with([vec![PadEvent::Connected(ONE), south(true)]]);
    let mut app = app(&window).with_gamepads(Box::new(pads));
    app.screen.asks_quit = true;
    assert_eq!(
        handle(
            &mut app,
            &mut window,
            WindowEvent::Redraw {
                elapsed: Duration::from_millis(16)
            }
        ),
        Control::Exit
    );
}

/// Shows or hides the developer overlay with a tap of its toggle key.
fn toggle_overlay(app: &mut TestApp, window: &mut FakeWindow) {
    for pressed in [true, false] {
        handle(app, window, key(nova_view::devtools::TOGGLE_KEY, pressed));
    }
}

#[test]
fn the_pads_input_goes_nowhere_while_the_overlay_shows_and_its_release_follows_once_it_hides() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let pads = FakeGamepads::with([vec![PadEvent::Connected(ONE)]]);
    let mut app = app(&window)
        .with_gamepads(Box::new(pads.clone()))
        .with_dev_overlay();
    redraw_at(&mut app, &mut window, 16);
    toggle_overlay(&mut app, &mut window);
    assert!(app.dev_overlay().is_some_and(DevOverlay::visible));
    pads.push(vec![south(true)]);
    redraw_at(&mut app, &mut window, 32);
    let pad_input = |input: &Input| matches!(input, Input::PadButton { .. });
    assert!(
        !app.screen().inputs.iter().any(pad_input),
        "{:?}",
        app.screen().inputs
    );
    assert_eq!(
        app.screen().surroundings,
        [told(PAD)],
        "the device used is still told, as a key's is under the overlay"
    );
    toggle_overlay(&mut app, &mut window);
    redraw_at(&mut app, &mut window, 48);
    assert!(
        !app.screen().inputs.iter().any(pad_input),
        "no press made up for the button still down"
    );
    pads.push(vec![south(false)]);
    redraw_at(&mut app, &mut window, 64);
    let pads_inputs: Vec<&Input> = app
        .screen()
        .inputs
        .iter()
        .filter(|input| pad_input(input))
        .collect();
    assert_eq!(
        pads_inputs,
        [&Input::PadButton {
            button: PadButton::South,
            pressed: false
        }],
        "its release follows the press the pad state kept, as a key's \
         release after the overlay reaches the screen"
    );
    assert_eq!(
        app.screen().surroundings,
        [told(PAD), told(Device::Keyboard)],
        "the toggle key's press told the keyboard, and a release tells nothing"
    );
}
