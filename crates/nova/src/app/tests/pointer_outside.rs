//! A move outside the game, in a letterbox bar or off the window, while the
//! primary button that went down over the game is still held, reaches the
//! screen as `Input::PointerMovedOutside` after the leave, so a drag can
//! follow it, as `_DoSystemMap`'s loop polls `_GetMouse` with no rect test
//! until `_StillDown` goes false (@0x137e9-0x138d8). With no such hold the
//! move routes as before: one leave, then nothing.

use super::*;

fn button(button: MouseButton, pressed: bool) -> WindowEvent {
    WindowEvent::PointerButton { button, pressed }
}

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

/// The inputs `app`'s screen took since `from`.
fn since(app: &TestApp, from: usize) -> Vec<Input> {
    app.screen().inputs[from..].to_vec()
}

/// `app` with the left button pressed over the game at (456, 100).
fn pressed(window: &mut FakeWindow) -> TestApp {
    let mut app = app(window);
    handle(&mut app, window, moved(456.0, 100.0));
    handle(&mut app, window, button(MouseButton::Left, true));
    app
}

#[test]
fn a_held_move_into_a_bar_routes_the_leave_then_the_move_outside() {
    let mut window = pillarboxed();
    let mut app = pressed(&mut window);
    let from = app.screen().inputs.len();
    handle(&mut app, &mut window, moved(10.0, 10.0));
    assert_eq!(
        since(&app, from),
        [
            Input::PointerLeft,
            Input::PointerMovedOutside(logical(&window, (10.0, 10.0))),
        ]
    );
    let from = app.screen().inputs.len();
    handle(&mut app, &mut window, moved(2500.0, 20.0));
    assert_eq!(
        since(&app, from),
        [Input::PointerMovedOutside(logical(&window, (2500.0, 20.0)))],
        "no second leave"
    );
}

#[test]
fn a_held_move_off_the_window_routes_after_its_leave() {
    let mut window = pillarboxed();
    let mut app = pressed(&mut window);
    let from = app.screen().inputs.len();
    handle(&mut app, &mut window, WindowEvent::PointerLeft);
    handle(&mut app, &mut window, moved(-40.0, -30.0));
    assert_eq!(
        since(&app, from),
        [
            Input::PointerLeft,
            Input::PointerMovedOutside(logical(&window, (-40.0, -30.0))),
        ]
    );
}

#[test]
fn a_move_into_a_bar_with_no_button_held_routes_only_the_leave() {
    let mut window = pillarboxed();
    let mut app = app(&window);
    handle(&mut app, &mut window, moved(456.0, 100.0));
    let from = app.screen().inputs.len();
    handle(&mut app, &mut window, moved(10.0, 10.0));
    handle(&mut app, &mut window, moved(20.0, 10.0));
    assert_eq!(since(&app, from), [Input::PointerLeft]);
}

#[test]
fn after_a_release_outside_further_moves_outside_route_nothing() {
    let mut window = pillarboxed();
    let mut app = pressed(&mut window);
    handle(&mut app, &mut window, moved(10.0, 10.0));
    let from = app.screen().inputs.len();
    handle(&mut app, &mut window, button(MouseButton::Left, false));
    assert_eq!(app.screen().cancels, 1, "the press abandoned once");
    handle(&mut app, &mut window, moved(20.0, 10.0));
    assert_eq!(since(&app, from), [], "no release, no move");
}

#[test]
fn after_a_release_inside_further_moves_outside_route_only_the_leave() {
    let mut window = pillarboxed();
    let mut app = pressed(&mut window);
    handle(&mut app, &mut window, button(MouseButton::Left, false));
    let from = app.screen().inputs.len();
    handle(&mut app, &mut window, moved(10.0, 10.0));
    handle(&mut app, &mut window, moved(20.0, 10.0));
    assert_eq!(since(&app, from), [Input::PointerLeft]);
}

#[test]
fn another_buttons_release_keeps_the_hold() {
    let mut window = pillarboxed();
    let mut app = pressed(&mut window);
    handle(&mut app, &mut window, button(MouseButton::Right, false));
    let from = app.screen().inputs.len();
    handle(&mut app, &mut window, moved(10.0, 10.0));
    assert_eq!(
        since(&app, from),
        [
            Input::PointerLeft,
            Input::PointerMovedOutside(logical(&window, (10.0, 10.0))),
        ]
    );
}

#[test]
fn after_the_focus_is_lost_moves_outside_route_only_the_leave() {
    let mut window = pillarboxed();
    let mut app = pressed(&mut window);
    handle(&mut app, &mut window, WindowEvent::FocusLost);
    handle(&mut app, &mut window, WindowEvent::FocusGained);
    let from = app.screen().inputs.len();
    handle(&mut app, &mut window, moved(10.0, 10.0));
    handle(&mut app, &mut window, moved(20.0, 10.0));
    assert!(
        !since(&app, from)
            .iter()
            .any(|input| matches!(input, Input::PointerMovedOutside(_))),
        "{:?}",
        since(&app, from)
    );
}

#[test]
fn a_press_outside_the_game_holds_nothing() {
    let mut window = pillarboxed();
    let mut app = app(&window);
    handle(&mut app, &mut window, moved(10.0, 10.0));
    handle(&mut app, &mut window, button(MouseButton::Left, true));
    let from = app.screen().inputs.len();
    handle(&mut app, &mut window, moved(20.0, 10.0));
    assert_eq!(since(&app, from), []);
}

#[test]
fn a_press_the_overlay_takes_holds_nothing() {
    let mut window = pillarboxed();
    let mut app = overlay_app(&window);
    routed(&mut app, &mut window, key(BACKQUOTE, true));
    routed(&mut app, &mut window, moved(456.0, 100.0));
    routed(&mut app, &mut window, button(MouseButton::Left, true));
    routed(&mut app, &mut window, key(BACKQUOTE, true));
    let from = app.screen().inputs.len();
    routed(&mut app, &mut window, moved(10.0, 10.0));
    routed(&mut app, &mut window, moved(20.0, 10.0));
    assert_eq!(since(&app, from), [Input::PointerLeft]);
}

#[test]
fn opening_the_overlay_ends_the_hold() {
    let mut window = pillarboxed();
    let mut app = overlay_app(&window);
    routed(&mut app, &mut window, moved(456.0, 100.0));
    routed(&mut app, &mut window, button(MouseButton::Left, true));
    routed(&mut app, &mut window, key(BACKQUOTE, true));
    routed(&mut app, &mut window, key(BACKQUOTE, true));
    let from = app.screen().inputs.len();
    routed(&mut app, &mut window, moved(10.0, 10.0));
    routed(&mut app, &mut window, moved(20.0, 10.0));
    assert_eq!(since(&app, from), [Input::PointerLeft]);
}
