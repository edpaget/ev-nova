//! Touches through the app (rdm:phase/key-bindings/phase-14-touch-input):
//! the app maps a finger's physical pixels to logical points, gives the
//! letterbox bars and the dev overlay their mouse rules, delivers what the
//! touch router routes, tells the device used last, and drops the mouse
//! while a finger is down. Where each finger goes is the router's
//! (`nova_view::touch`), pinned there; one or two fingers suffice here.

use nova_view::{Device, FingerId, Pinch, Surroundings, Touch, TouchPhase};

use super::*;

/// A pillarboxed retina window: content x 256..2304 at scale 2.
fn pillarboxed() -> FakeWindow {
    FakeWindow::new((2560, 1536), 2.0)
}

/// Where the pillarboxed window's pixel `px` lands in the logical space,
/// past the content's edges as the router maps it.
fn logical(window: &FakeWindow, px: (f64, f64)) -> Point {
    Viewport::for_shape(ViewShape::Classic, window.size_px(), window.scale_factor())
        .window_to_logical_unbounded(px)
        .expect("a window with area")
}

fn finger(f: usize) -> FingerId {
    FingerId::new(f)
}

fn down(f: usize, px: (f64, f64)) -> WindowEvent {
    WindowEvent::TouchStarted {
        finger: finger(f),
        px,
    }
}

fn to(f: usize, px: (f64, f64)) -> WindowEvent {
    WindowEvent::TouchMoved {
        finger: finger(f),
        px,
    }
}

fn up(f: usize, px: (f64, f64)) -> WindowEvent {
    WindowEvent::TouchEnded {
        finger: finger(f),
        px,
    }
}

fn cancelled(f: usize) -> WindowEvent {
    WindowEvent::TouchCancelled { finger: finger(f) }
}

fn button(pressed: bool) -> WindowEvent {
    WindowEvent::PointerButton {
        button: MouseButton::Left,
        pressed,
    }
}

fn touch(f: usize, phase: TouchPhase) -> Input {
    Input::Touch(Touch {
        finger: finger(f),
        phase,
    })
}

/// The app over `window`, its screen claiming touches left of logical
/// `x`.
fn claiming(window: &FakeWindow, x: f32) -> TestApp {
    let mut app = app(window);
    app.screen.claims_left_of = Some(x);
    app
}

/// The inputs `events` give `app`'s screen.
fn inputs(app: &mut TestApp, window: &mut FakeWindow, events: &[WindowEvent]) -> Vec<Input> {
    let from = app.screen().inputs.len();
    for &event in events {
        handle(app, window, event);
    }
    app.screen().inputs[from..].to_vec()
}

const IN_GAME: (f64, f64) = (456.0, 100.0);

#[test]
fn a_claimed_touch_reaches_the_screen_as_touch_inputs_at_logical_points() {
    let mut window = pillarboxed();
    let mut app = claiming(&window, f32::INFINITY);
    let got = inputs(
        &mut app,
        &mut window,
        &[
            down(1, IN_GAME),
            to(1, (10.0, 20.0)),
            up(1, (12.0, 22.0)),
            cancelled(1),
        ],
    );
    assert_eq!(
        got,
        [
            touch(1, TouchPhase::Started(logical(&window, IN_GAME))),
            touch(1, TouchPhase::Moved(logical(&window, (10.0, 20.0)))),
            touch(1, TouchPhase::Ended(logical(&window, (12.0, 22.0)))),
        ],
        "no pointer input, and nothing for winit's leave after the lift"
    );
    assert_eq!(app.screen().cancels, 0);
}

#[test]
fn a_lone_finger_is_the_screens_pointer_as_the_mouse_is() {
    let mut window = pillarboxed();
    let mut by_finger = app(&window);
    let touched = inputs(
        &mut by_finger,
        &mut window,
        &[
            down(1, IN_GAME),
            to(1, (500.0, 120.0)),
            up(1, (520.0, 130.0)),
            cancelled(1),
        ],
    );
    let mut by_mouse = app(&window);
    let clicked = inputs(
        &mut by_mouse,
        &mut window,
        &[
            moved(IN_GAME.0, IN_GAME.1),
            button(true),
            moved(500.0, 120.0),
            moved(520.0, 130.0),
            button(false),
        ],
    );
    assert_eq!(touched, clicked);
    assert_eq!(by_finger.screen().cancels, 0);
}

#[test]
fn a_touch_starting_in_a_letterbox_bar_reaches_no_screen_as_a_mouse_press_there_does() {
    let mut window = pillarboxed();
    let mut by_finger = app(&window);
    let touched = inputs(
        &mut by_finger,
        &mut window,
        &[
            down(1, (10.0, 10.0)),
            to(1, IN_GAME),
            up(1, IN_GAME),
            cancelled(1),
        ],
    );
    let mut by_mouse = app(&window);
    let pressed = inputs(
        &mut by_mouse,
        &mut window,
        &[moved(10.0, 10.0), button(true)],
    );
    assert_eq!(touched, pressed);
    assert_eq!(touched, [], "nothing, even once it moves into the game");
    assert_eq!(by_finger.screen().cancels, 0);
}

#[test]
fn a_finger_moving_into_a_bar_leaves_then_moves_outside_as_a_held_mouse_does() {
    let mut window = pillarboxed();
    let mut by_finger = app(&window);
    let touched = inputs(
        &mut by_finger,
        &mut window,
        &[down(1, IN_GAME), to(1, (10.0, 10.0)), to(1, (2500.0, 20.0))],
    );
    let mut by_mouse = app(&window);
    let held = inputs(
        &mut by_mouse,
        &mut window,
        &[
            moved(IN_GAME.0, IN_GAME.1),
            button(true),
            moved(10.0, 10.0),
            moved(2500.0, 20.0),
        ],
    );
    assert_eq!(touched, held);
    assert_eq!(
        touched[2..],
        [
            Input::PointerLeft,
            Input::PointerMovedOutside(logical(&window, (10.0, 10.0))),
            Input::PointerMovedOutside(logical(&window, (2500.0, 20.0))),
        ]
    );
}

#[test]
fn a_finger_lifting_in_a_bar_abandons_the_press_as_the_mouse_does() {
    let mut window = pillarboxed();
    let mut app = app(&window);
    handle(&mut app, &mut window, down(1, IN_GAME));
    handle(&mut app, &mut window, to(1, (10.0, 10.0)));
    let got = inputs(&mut app, &mut window, &[up(1, (12.0, 10.0))]);
    assert_eq!(app.screen().cancels, 1, "the press abandoned once");
    assert!(
        !got.iter()
            .any(|input| matches!(input, Input::PointerButton { pressed: false, .. })),
        "no release: {got:?}"
    );
}

#[test]
fn while_the_overlay_shows_a_touch_goes_to_it_alone() {
    let mut window = pillarboxed();
    let mut app = overlay_app(&window);
    app.screen.claims_left_of = Some(200.0);
    routed(&mut app, &mut window, key(BACKQUOTE, true));
    for event in [
        down(1, IN_GAME),
        to(1, (10.0, 10.0)),
        up(1, (10.0, 10.0)),
        cancelled(1),
        down(2, (1500.0, 100.0)),
        cancelled(2),
    ] {
        let handled_as = routed(&mut app, &mut window, event);
        assert_eq!(
            handled_as,
            handled(Control::Continue, Routing::Overlay),
            "{event:?}"
        );
        assert!(app.overlay_wants(Some(&handled_as)));
    }
    assert_eq!(app.screen().inputs, []);
    routed(&mut app, &mut window, key(BACKQUOTE, true));
    assert_eq!(
        inputs(&mut app, &mut window, &[to(2, IN_GAME), up(2, IN_GAME)]),
        [],
        "the router never saw those fingers"
    );
}

#[test]
fn opening_the_overlay_mid_touch_cancels_the_screens_press_and_its_claimed_touches() {
    let mut window = pillarboxed();
    let mut app = overlay_app(&window);
    // Logical (100, 50) is claimed; (622, 50) is the pointer.
    app.screen.claims_left_of = Some(200.0);
    handle(&mut app, &mut window, down(1, IN_GAME));
    handle(&mut app, &mut window, down(2, (1500.0, 100.0)));
    let from = app.screen().inputs.len();
    routed(&mut app, &mut window, key(BACKQUOTE, true));
    assert_eq!(
        app.screen().inputs[from..],
        [touch(1, TouchPhase::Cancelled)]
    );
    assert_eq!(app.screen().cancels, 1, "the pointer's press, once");
    routed(&mut app, &mut window, key(BACKQUOTE, true));
    assert_eq!(
        inputs(
            &mut app,
            &mut window,
            &[to(1, IN_GAME), up(1, IN_GAME), up(2, (1500.0, 100.0))]
        ),
        [],
        "both forgotten"
    );
}

#[test]
fn losing_focus_mid_touch_cancels_claimed_touches_and_ends_a_pinch() {
    let mut window = pillarboxed();
    let mut app = claiming(&window, 200.0);
    handle(&mut app, &mut window, down(1, IN_GAME));
    handle(&mut app, &mut window, down(2, (1500.0, 100.0)));
    handle(&mut app, &mut window, down(3, (1700.0, 100.0)));
    let from = app.screen().inputs.len();
    handle(&mut app, &mut window, WindowEvent::FocusLost);
    assert_eq!(
        app.screen().inputs[from..],
        [touch(1, TouchPhase::Cancelled), Input::Pinch(Pinch::Ended)]
    );
    assert_eq!(app.screen().cancels, 1, "the pointer's press, once");
    handle(&mut app, &mut window, WindowEvent::FocusGained);
    assert_eq!(
        inputs(
            &mut app,
            &mut window,
            &[
                up(1, IN_GAME),
                up(2, (1500.0, 100.0)),
                up(3, (1700.0, 100.0))
            ]
        ),
        []
    );
}

#[test]
fn a_cancelled_touch_abandons_the_screens_press() {
    let mut window = pillarboxed();
    let mut app = app(&window);
    let got = inputs(&mut app, &mut window, &[down(1, IN_GAME), cancelled(1)]);
    let at = logical(&window, IN_GAME);
    assert_eq!(
        got,
        [
            Input::PointerMoved(at),
            Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at
            }
        ],
        "no release"
    );
    assert_eq!(app.screen().cancels, 1);
    assert_eq!(
        inputs(&mut app, &mut window, &[to(1, (10.0, 10.0))]),
        [],
        "not held: no move outside"
    );
}

#[test]
fn a_pinch_reaches_the_screen_as_pinch_inputs_at_logical_points() {
    let mut window = pillarboxed();
    let mut app = app(&window);
    handle(&mut app, &mut window, down(1, IN_GAME));
    let got = inputs(
        &mut app,
        &mut window,
        &[
            down(2, (636.0, 100.0)),
            to(2, (456.0 + 2.0 * 121.0, 100.0)),
            up(2, (456.0 + 2.0 * 121.0, 100.0)),
        ],
    );
    assert_eq!(
        got,
        [
            Input::Pinch(Pinch::Began {
                primary: logical(&window, IN_GAME),
                second: logical(&window, (636.0, 100.0)),
            }),
            Input::Pinch(Pinch::ZoomIn),
            Input::Pinch(Pinch::Ended),
        ],
        "90 logical units apart, then 121"
    );
}

#[test]
fn mouse_events_while_a_finger_is_down_are_dropped_and_switch_no_device() {
    let mut window = pillarboxed();
    let mut app = app(&window);
    handle(&mut app, &mut window, down(1, IN_GAME));
    let told = app.screen().surroundings.len();
    let from = app.screen().inputs.len();
    for event in [
        moved(900.0, 300.0),
        button(true),
        button(false),
        WindowEvent::PointerLeft,
    ] {
        assert_eq!(
            routed(&mut app, &mut window, event),
            handled(Control::Continue, Routing::Game),
            "{event:?}"
        );
    }
    assert_eq!(app.screen().inputs[from..], [], "the mouse reached nothing");
    assert_eq!(app.screen().surroundings.len(), told, "no device switched");
    let lift = inputs(&mut app, &mut window, &[up(1, IN_GAME)]);
    let at = logical(&window, IN_GAME);
    assert_eq!(
        lift,
        [Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at
        }],
        "the finger's pointer stayed where the finger is"
    );
    assert_eq!(
        inputs(&mut app, &mut window, &[moved(900.0, 300.0)]),
        [Input::PointerMoved(logical(&window, (900.0, 300.0)))],
        "after the lift, the mouse works again"
    );
    assert_eq!(
        app.screen().surroundings.last(),
        Some(&Surroundings {
            device: Device::Mouse
        })
    );
}

#[test]
fn a_touch_tells_the_screen_touch_was_used_last() {
    let mut window = pillarboxed();
    let mut app = app(&window);
    handle(&mut app, &mut window, cancelled(1));
    assert_eq!(
        app.screen().surroundings,
        [],
        "a cancel alone switches nothing"
    );
    handle(&mut app, &mut window, down(1, (10.0, 10.0)));
    assert_eq!(
        app.screen().surroundings,
        [Surroundings {
            device: Device::Touch
        }],
        "even in a bar"
    );
    handle(&mut app, &mut window, key(Key::KeyA, true));
    assert_eq!(
        app.screen().surroundings.last(),
        Some(&Surroundings {
            device: Device::Keyboard
        })
    );
    for event in [to(2, IN_GAME), up(2, IN_GAME)] {
        let mut window = pillarboxed();
        let mut app = self::app(&window);
        handle(&mut app, &mut window, event);
        assert_eq!(
            app.screen().surroundings,
            [Surroundings {
                device: Device::Touch
            }],
            "{event:?}"
        );
    }
}
