//! Wiring: the app doesn't tell the spaceport's side the device used last
//! yet (rdm:phase/key-bindings/phase-9-gamepad-spaceport), so the pad's
//! focus never shows there, and its moves and South must act on nothing
//! (`ui::pad_focus`'s "Nothing hidden acts", pinned in that module and in
//! the dialog's).

use super::*;
use crate::input::PadButton;

fn tap(view: &mut SpaceportView, button: PadButton) {
    for pressed in [true, false] {
        view.input(&Input::PadButton { button, pressed });
    }
}

/// Every focus move, then South.
fn dpad_then_south(view: &mut SpaceportView) {
    for button in [
        PadButton::DPadUp,
        PadButton::DPadLeft,
        PadButton::DPadDown,
        PadButton::DPadRight,
        PadButton::RightShoulder,
        PadButton::LeftShoulder,
        PadButton::South,
    ] {
        tap(view, button);
    }
}

#[test]
fn untold_the_d_pad_and_south_neither_take_off_nor_open_anything() {
    let mut view = earth();
    dpad_then_south(&mut view);
    assert!(!view.left(), "no take-off");
    assert!(view.open_service().is_none());
    assert!(!view.take_recharge());
    assert_eq!(view.dialog().expect("laid out").pressed_item(), None);
}

#[test]
fn untold_east_still_leaves_as_escape_does() {
    let mut view = earth();
    tap(&mut view, PadButton::East);
    assert!(view.left());
}

#[test]
fn untold_the_d_pad_and_south_leave_no_bar() {
    let mut view = venues();
    view.take_offer_request();
    click_item(&mut view, 11);
    serve_openings(&mut view);
    assert!(view.open_bar().is_some(), "the bar is open");
    dpad_then_south(&mut view);
    assert!(view.open_bar().is_some_and(|bar| !bar.closed()));
    assert!(!view.left());
}
