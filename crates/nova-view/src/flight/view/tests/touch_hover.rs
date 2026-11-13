//! The hover cursor under a finger
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps): a finger down is
//! the pointer, so the cursor shows under it, follows it and animates
//! over a ship as hover does; its lift (or cancel) is the pointer leaving,
//! so the cursor hides at once. A mouse release, whatever device was told
//! last, still leaves the cursor where it was.
//!
//! The router's leave after a lift is `touch::router`'s; these drive the
//! real router through [`Fingers`], with flight's own claim.

use super::controls::{cursor_sprites, faked_cursor_alpha, faked_with_npc};
use super::faked_flight::{at_rest, ticked};
use super::faked_world::world_list;
use super::*;
use crate::input::{ControllerFamily, Device, Surroundings};
use crate::touch::testkit::Fingers;

/// A point in open space, clear of the touch controls.
const SPACE: Point = Point::new(300.0, 200.0);

/// Another such point.
const ELSEWHERE: Point = Point::new(420.0, 260.0);

/// `view` told `device` was used last, with what its overlay raised fed
/// back through its input.
fn told(view: &mut Faked, device: Device) {
    view.input(&Input::Surroundings(Surroundings::of(device)));
    for input in view.take_touch_controls() {
        view.input(&input);
    }
}

/// A view at rest in Sol, told touch was used last.
fn touched() -> Faked {
    let mut view = at_rest(catalog());
    told(&mut view, Device::Touch);
    assert!(!view.claims_touch(SPACE) && !view.claims_touch(ELSEWHERE));
    view
}

fn cursor(view: &Faked) -> Vec<(u16, Point)> {
    cursor_sprites(&world_list(view))
}

#[test]
fn the_hover_cursor_shows_at_the_finger_while_it_is_down() {
    let mut view = touched();
    let mut fingers = Fingers::claimed_by_screen(&mut view);
    fingers.down(1, SPACE);
    assert_eq!(fingers.screen().cursor_at(), Some(SPACE));
    assert_eq!(cursor(fingers.screen()), [(0, SPACE)]);
    assert_eq!(faked_cursor_alpha(fingers.screen()), Some(255), "whole");
    fingers.move_to(1, ELSEWHERE);
    assert_eq!(fingers.screen().cursor_at(), Some(ELSEWHERE));
    assert_eq!(cursor(fingers.screen()), [(0, ELSEWHERE)]);
}

#[test]
fn the_hover_cursor_hides_once_the_finger_lifts() {
    let mut view = touched();
    let mut fingers = Fingers::claimed_by_screen(&mut view);
    fingers.down(1, SPACE);
    fingers.up(1, SPACE);
    assert_eq!(fingers.screen().cursor_at(), None);
    assert_eq!(cursor(fingers.screen()), []);
    ticked(fingers.screen(), 30);
    assert_eq!(cursor(fingers.screen()), [], "still none after ticks");
}

#[test]
fn a_cancelled_touch_hides_the_cursor() {
    let mut view = touched();
    let mut fingers = Fingers::claimed_by_screen(&mut view);
    fingers.down(1, SPACE);
    fingers.cancel(1);
    assert_eq!(fingers.screen().cursor_at(), None);
    assert_eq!(cursor(fingers.screen()), []);
}

#[test]
fn the_next_touch_shows_the_cursor_again_whole() {
    let mut view = touched();
    let mut fingers = Fingers::claimed_by_screen(&mut view);
    fingers.down(1, SPACE);
    fingers.up(1, SPACE);
    ticked(fingers.screen(), 90);
    fingers.down(2, ELSEWHERE);
    assert_eq!(cursor(fingers.screen()), [(0, ELSEWHERE)]);
    assert_eq!(faked_cursor_alpha(fingers.screen()), Some(255));
}

#[test]
fn a_finger_resting_on_a_ship_animates_the_cursor_as_hover_does() {
    // Passes when written: phase 14 made the finger the pointer.
    let (mut by_mouse, id) = faked_with_npc();
    let over = npc_on_screen(&by_mouse, id);
    by_mouse
        .sim
        .set_hover(Some(nova_sim_frame::Hover::Ship(id)));
    by_mouse.input(&Input::PointerMoved(over));
    ticked(&mut by_mouse, 2);
    let (mut by_touch, _) = faked_with_npc();
    by_touch
        .sim
        .set_hover(Some(nova_sim_frame::Hover::Ship(id)));
    let mut fingers = Fingers::claimed_by_screen(&mut by_touch);
    fingers.down(1, over);
    ticked(fingers.screen(), 2);
    assert_eq!(cursor(fingers.screen()), cursor(&by_mouse));
    assert_eq!(cursor(fingers.screen()), [(1, over)], "animated");
}

#[test]
fn a_finger_on_the_touch_controls_leaves_the_cursor_alone() {
    // Passes when written: a claimed finger is never the pointer.
    let mut view = touched();
    let fire = view
        .touch_controls
        .layout()
        .get(crate::flight::touch_controls::layout::ControlId::Command(
            crate::keymap::Command::Flight(crate::keymap::FlightCommand::FirePrimary),
        ))
        .expect("fire placed")
        .rect
        .center();
    view.input(&Input::PointerMoved(SPACE));
    let mut fingers = Fingers::claimed_by_screen(&mut view);
    fingers.down(1, fire);
    assert_eq!(fingers.screen().cursor_at(), Some(SPACE), "down");
    fingers.up(1, fire);
    assert_eq!(fingers.screen().cursor_at(), Some(SPACE), "lifted");
}

#[test]
fn with_the_keyboard_mouse_or_pad_last_used_a_mouse_release_leaves_the_cursor_where_it_was() {
    // Passes when written: a mouse release is no leave.
    for device in [
        Device::Keyboard,
        Device::Mouse,
        Device::Gamepad(ControllerFamily::Generic),
    ] {
        let mut view = at_rest(catalog());
        told(&mut view, device);
        view.input(&Input::PointerMoved(SPACE));
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: SPACE,
        };
        view.input(&button(true));
        view.input(&button(false));
        ticked(&mut view, 3);
        assert_eq!(view.cursor_at(), Some(SPACE), "{device:?}");
        assert_eq!(cursor(&view), [(0, SPACE)], "{device:?}");
    }
}
