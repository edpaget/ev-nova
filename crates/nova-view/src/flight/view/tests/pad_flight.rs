//! Flying from a gamepad: the stick axes' analog turn and throttle in the
//! step input, the d-pad's full-rate turn and thrust, the throttle's back
//! direction as reverse, the layers, and a lost pad. The table's rules
//! are the keymap's (`keymap::gamepad`'s tests); these pin that flight
//! reads them into its step.

use super::faked_flight::{at_rest, last_step_input, ticked};
use super::*;
use crate::input::{PadAxis, PadButton};
use crate::keymap::FLIGHT_DEAD_ZONE;
use nova_sim::{Amounts, Turn};

fn axis(view: &mut Faked, axis: PadAxis, value: f32) {
    view.input(&Input::PadAxis { axis, value });
}

fn pad(view: &mut Faked, button: PadButton, pressed: bool) {
    view.input(&Input::PadButton { button, pressed });
}

/// The raw stick value whose dead-zoned amount is `amount`, on one axis.
fn raw(amount: f32) -> f32 {
    amount.signum() * (FLIGHT_DEAD_ZONE + amount.abs() * (1.0 - FLIGHT_DEAD_ZONE))
}

/// The step input after a tick.
fn stepped(view: &mut Faked) -> nova_sim::flight_frame::StepInput {
    ticked(view, 1);
    last_step_input(view)
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn the_right_stick_x_carries_a_signed_turn_amount_into_the_step() {
    for (value, way) in [(0.4, Turn::Right), (-0.7, Turn::Left), (1.0, Turn::Right)] {
        let mut view = at_rest(catalog());
        axis(&mut view, PadAxis::RightX, raw(value));
        let input = stepped(&mut view);
        assert_eq!(input.controls.turn, way, "{value}");
        assert!(near(input.amounts.turn, value.abs()), "{value}: {input:?}");
        assert!(!input.controls.thrust, "{value}");
    }
}

#[test]
fn the_left_stick_y_forward_carries_a_throttle_amount() {
    let mut view = at_rest(catalog());
    axis(&mut view, PadAxis::LeftY, raw(0.6));
    let input = stepped(&mut view);
    assert!(input.controls.thrust);
    assert!(near(input.amounts.thrust, 0.6), "{input:?}");
    assert_eq!(input.controls.turn, Turn::None);
    assert!(!input.controls.reverse, "below the press threshold");
    axis(&mut view, PadAxis::LeftY, raw(-0.3));
    let input = stepped(&mut view);
    assert!(!input.controls.thrust, "pulled back: no thrust");
    assert_eq!(input.amounts.thrust, 0.0);
}

#[test]
fn a_stick_inside_the_dead_zone_carries_zero() {
    let mut view = at_rest(catalog());
    axis(&mut view, PadAxis::RightX, 0.1);
    axis(&mut view, PadAxis::LeftY, -0.1);
    let input = stepped(&mut view);
    assert_eq!(input.controls, Controls::default());
    assert_eq!(
        input.amounts,
        Amounts {
            turn: 0.0,
            thrust: 0.0
        }
    );
}

#[test]
fn with_keys_only_each_amount_is_none_or_full() {
    let mut view = at_rest(catalog());
    let input = stepped(&mut view);
    assert_eq!(
        input.amounts,
        Amounts {
            turn: 0.0,
            thrust: 0.0
        }
    );
    super::faked_flight::hold(&mut view, Key::ArrowUp, true);
    super::faked_flight::hold(&mut view, Key::ArrowLeft, true);
    let input = stepped(&mut view);
    assert_eq!(input.amounts, Amounts::FULL);
    assert!(input.controls.thrust);
    assert_eq!(input.controls.turn, Turn::Left);
}

#[test]
fn the_dpad_turns_and_thrusts_at_full_rate() {
    let mut view = at_rest(catalog());
    pad(&mut view, PadButton::DPadUp, true);
    pad(&mut view, PadButton::DPadRight, true);
    let input = stepped(&mut view);
    assert!(input.controls.thrust);
    assert_eq!(input.controls.turn, Turn::Right);
    assert_eq!(input.amounts, Amounts::FULL);
    pad(&mut view, PadButton::DPadRight, false);
    pad(&mut view, PadButton::DPadLeft, true);
    assert_eq!(stepped(&mut view).controls.turn, Turn::Left);
    pad(&mut view, PadButton::DPadUp, false);
    pad(&mut view, PadButton::DPadLeft, false);
    assert_eq!(stepped(&mut view).controls, Controls::default());
}

#[test]
fn a_key_wins_over_a_smaller_stick_amount() {
    let mut view = at_rest(catalog());
    axis(&mut view, PadAxis::RightX, raw(0.4));
    axis(&mut view, PadAxis::LeftY, raw(0.3));
    super::faked_flight::hold(&mut view, Key::ArrowLeft, true);
    super::faked_flight::hold(&mut view, Key::ArrowUp, true);
    let input = stepped(&mut view);
    assert_eq!(input.controls.turn, Turn::Left, "the key's way");
    assert_eq!(input.amounts, Amounts::FULL);
    assert!(input.controls.thrust);
}

#[test]
fn pulling_the_throttle_back_past_the_threshold_holds_reverse_and_letting_it_back_releases_it() {
    let mut view = at_rest(catalog());
    // `nova`'s pad module presses the direction past the threshold, and
    // sends the axis's value beside it.
    axis(&mut view, PadAxis::LeftY, -0.8);
    pad(&mut view, PadButton::LeftStickDown, true);
    let input = stepped(&mut view);
    assert!(input.controls.reverse);
    assert!(!input.controls.thrust);
    pad(&mut view, PadButton::LeftStickDown, false);
    axis(&mut view, PadAxis::LeftY, -0.2);
    assert!(!stepped(&mut view).controls.reverse);
}

#[test]
fn south_fires_the_primary_and_select_south_jumps_not() {
    let mut view = at_rest(catalog());
    pad(&mut view, PadButton::South, true);
    assert_eq!(stepped(&mut view).fire, (true, false));
    pad(&mut view, PadButton::South, false);
    assert_eq!(stepped(&mut view).fire, (false, false));
    pad(&mut view, PadButton::Select, true);
    assert_eq!(stepped(&mut view).fire, (false, false), "Select alone");
    pad(&mut view, PadButton::South, true);
    assert_eq!(
        stepped(&mut view).fire,
        (false, false),
        "Select and South: the jump, not the fire"
    );
    assert!(view.keys.held(crate::keymap::FlightCommand::HyperJump));
}

#[test]
fn a_lost_pad_leaves_nothing_held_and_both_axes_zero() {
    let mut view = at_rest(catalog());
    axis(&mut view, PadAxis::RightX, raw(0.5));
    axis(&mut view, PadAxis::LeftY, raw(0.5));
    pad(&mut view, PadButton::South, true);
    pad(&mut view, PadButton::DPadLeft, true);
    let held = stepped(&mut view);
    assert!(held.controls.thrust && held.fire.0);
    // What `nova` sends as a pad is lost: each button's release, and each
    // axis at none.
    pad(&mut view, PadButton::South, false);
    pad(&mut view, PadButton::DPadLeft, false);
    for each in PadAxis::ALL {
        axis(&mut view, each, 0.0);
    }
    let input = stepped(&mut view);
    assert_eq!(input.controls, Controls::default());
    assert_eq!(input.fire, (false, false));
    assert_eq!(
        input.amounts,
        Amounts {
            turn: 0.0,
            thrust: 0.0
        }
    );
}

#[test]
fn release_keys_zeroes_the_flight_axes() {
    let mut view = at_rest(catalog());
    axis(&mut view, PadAxis::RightX, raw(0.5));
    axis(&mut view, PadAxis::LeftY, raw(0.5));
    pad(&mut view, PadButton::DPadUp, true);
    view.release_keys();
    let input = stepped(&mut view);
    assert_eq!(input.controls, Controls::default());
    assert_eq!(input.amounts.turn, 0.0);
    assert_eq!(input.amounts.thrust, 0.0);
}

#[test]
fn a_stick_outside_the_dead_zone_fades_the_cursor_as_a_helm_key() {
    let mut view = at_rest(catalog());
    view.frame.key_fades = nova_sim::flight_frame::KeyFades::EVERY;
    assert!(!view.cursor_keyed());
    axis(&mut view, PadAxis::RightX, 0.1);
    assert!(!view.cursor_keyed(), "in the dead zone");
    axis(&mut view, PadAxis::RightX, raw(0.2));
    assert!(view.cursor_keyed());
}

#[test]
fn the_surroundings_reach_flight_s_reader() {
    let mut view = at_rest(catalog());
    let used = crate::input::Device::Gamepad(crate::input::ControllerFamily::Generic);
    view.input(&Input::Surroundings(crate::input::Surroundings {
        device: used,
    }));
    assert_eq!(view.keys.device(), used);
}
