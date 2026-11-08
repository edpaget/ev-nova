//! What flight takes back of the pad as the last dialog over it closes,
//! on [`FakeFlight`] (the `pad_hand_back` module): each button with the
//! layer it went down under, read by its exact layer only, the press the
//! closing dialog took let go of, and the stick axes as they stand. Which
//! dialogs hand back, and which press is taken, is the router's, pinned
//! in the `nova` crate's `app::screen::tests::pad_live_keys`. The layer
//! rules are the keymap's (`keymap::gamepad`'s tests).

use super::faked_flight::{at_rest, last_step_input, ticked};
use super::faked_screen::in_flight;
use super::live_keys::dialog_opens;
use super::*;
use crate::input::{PadAxis, PadButton as P};
use crate::keymap::{Command, FlightCommand as Fc, Gamepad, PadMap};

/// A dialog no site follows opening over `view`'s flight: the router's
/// letting go, which marks nothing.
fn unlatching_dialog_opens(view: &mut Faked) {
    view.let_go_of_input();
}

fn pad_press(button: P) -> Input {
    Input::PadButton {
        button,
        pressed: true,
        repeat: false,
    }
}

fn pad_release(button: P) -> Input {
    Input::PadButton {
        button,
        pressed: false,
        repeat: false,
    }
}

fn pad_repeat(button: P) -> Input {
    Input::PadButton {
        button,
        pressed: true,
        repeat: true,
    }
}

/// A pad that followed `inputs` on the default table, as the router's
/// does.
fn pad_of(inputs: &[Input]) -> Gamepad {
    let map = PadMap::default();
    let mut pad = Gamepad::default();
    for input in inputs {
        pad.follow(input, &map);
    }
    pad
}

/// A pad with `buttons` pressed, in order.
fn pressed(buttons: &[P]) -> Gamepad {
    let presses: Vec<Input> = buttons.iter().map(|&b| pad_press(b)).collect();
    pad_of(&presses)
}

/// The last dialog over `view`'s flight closing with `pad` down, `before`
/// down at flight's last poll, and `taken` the press the dialog took:
/// what comes back to replay.
fn back_pad(view: &mut Faked, pad: Gamepad, before: Gamepad, taken: Option<P>) -> Vec<Input> {
    view.take_back_live_input(LiveKeys {
        pad,
        pad_before: before,
        pad_taken: taken,
        ..LiveKeys::default()
    })
}

/// Whether `command` waits for its button's release.
fn latched(view: &Faked, command: Fc) -> bool {
    view.latched_down.contains(&Command::Flight(command))
}

/// The raw stick value whose dead-zoned amount is `amount`, on one axis.
fn raw(amount: f32) -> f32 {
    amount.signum()
        * (crate::keymap::FLIGHT_DEAD_ZONE + amount.abs() * (1.0 - crate::keymap::FLIGHT_DEAD_ZONE))
}

#[test]
fn a_dpad_direction_held_across_the_close_thrusts_on_the_next_step() {
    let mut view = at_rest(catalog());
    view.input(&pad_press(P::DPadUp));
    dialog_opens(&mut view);
    let replay = back_pad(&mut view, pressed(&[P::DPadUp]), Gamepad::default(), None);
    assert_eq!(replay, [], "read held");
    ticked(&mut view, 1);
    assert!(last_step_input(&view).controls.thrust);
}

#[test]
fn a_stick_held_still_across_the_close_turns_on_the_next_step() {
    let mut view = at_rest(catalog());
    let turn = Input::PadAxis {
        axis: PadAxis::RightX,
        value: raw(0.6),
    };
    view.input(&turn);
    dialog_opens(&mut view);
    let pad = pad_of(&[turn]);
    back_pad(&mut view, pad, Gamepad::default(), None);
    ticked(&mut view, 1);
    let input = last_step_input(&view);
    assert_eq!(input.controls.turn, Turn::Right);
    assert!((input.amounts.turn - 0.6).abs() < 1e-5, "{input:?}");
}

#[test]
fn an_unlatched_pad_button_let_go_under_the_dialog_isnt_held() {
    // Left down by a path that kept it: its release went elsewhere.
    let mut view = at_rest(catalog());
    view.input(&pad_press(P::DPadUp));
    view.input(&pad_press(P::Select));
    back_pad(&mut view, Gamepad::default(), Gamepad::default(), None);
    ticked(&mut view, 1);
    assert!(!last_step_input(&view).controls.thrust, "let go");
    assert!(!view.keys.is_down(&pad_press(P::Select)), "the layer too");
}

#[test]
fn the_taken_button_is_let_go_and_its_release_does_nothing() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    let replay = back_pad(
        &mut view,
        pressed(&[P::East]),
        Gamepad::default(),
        Some(P::East),
    );
    assert_eq!(replay, []);
    assert!(!view.keys.is_down(&pad_press(P::East)), "not taken back");
    assert!(!view.keys.held(Fc::LandDock));
    let from = sent(&view).len();
    view.input(&pad_release(P::East));
    view.held_keys_frame();
    assert_eq!(sent(&view)[from..], [], "nothing");
    // Not taken, East comes back as L does, held for its own latch.
    let mut view = in_flight();
    dialog_opens(&mut view);
    back_pad(&mut view, pressed(&[P::East]), Gamepad::default(), None);
    assert!(view.keys.held(Fc::LandDock));
}

#[test]
fn select_and_south_held_across_a_sites_close_wait_as_hyper_jump_and_never_fire() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    let replay = back_pad(
        &mut view,
        pressed(&[P::Select, P::South]),
        Gamepad::default(),
        None,
    );
    assert_eq!(replay, [], "nothing to replay");
    assert!(latched(&view, Fc::HyperJump), "waits for its release");
    assert!(view.jump_latch, "its latch set");
    let from = sent(&view).len();
    view.held_keys_frame();
    assert!(!sent(&view)[from..].contains(&FlightCommand::Jump));
    ticked(&mut view, 1);
    assert_eq!(last_step_input(&view).fire, (false, false), "no fire");
}

#[test]
fn select_and_south_pressed_under_an_unlatching_dialog_come_back_as_a_hyper_jump_press() {
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = back_pad(
        &mut view,
        pressed(&[P::Select, P::South]),
        Gamepad::default(),
        None,
    );
    assert_eq!(replay, [pad_press(P::South)]);
    let from = sent(&view).len();
    view.input(&replay[0]);
    assert!(
        sent(&view)[from..].contains(&FlightCommand::Jump),
        "{:?}",
        sent(&view)
    );
    ticked(&mut view, 1);
    assert_eq!(last_step_input(&view).fire, (false, false), "no fire");
}

#[test]
fn south_pressed_before_select_and_held_across_the_close_fires() {
    let mut view = at_rest(catalog());
    dialog_opens(&mut view);
    let replay = back_pad(
        &mut view,
        pressed(&[P::South, P::Select]),
        Gamepad::default(),
        None,
    );
    assert_eq!(replay, []);
    ticked(&mut view, 1);
    assert_eq!(last_step_input(&view).fire, (true, false));
}

#[test]
fn a_button_whose_layer_went_up_first_holds_nothing_after_the_close() {
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let pad = pad_of(&[
        pad_press(P::Select),
        pad_press(P::South),
        pad_release(P::Select),
    ]);
    let replay = back_pad(&mut view, pad, Gamepad::default(), None);
    assert_eq!(replay, [], "no Hyper Jump, no Fire Primary");
    assert!(
        view.keys.is_down(&pad_press(P::South)),
        "held, meaning nothing"
    );
    assert!(!latched(&view, Fc::HyperJump));
    ticked(&mut view, 1);
    assert_eq!(last_step_input(&view).fire, (false, false));
}

#[test]
fn start_and_south_held_across_the_close_are_read_by_attack_targets_own_latch() {
    for opens in [dialog_opens, unlatching_dialog_opens] {
        let mut view = in_flight();
        opens(&mut view);
        let replay = back_pad(
            &mut view,
            pressed(&[P::Start, P::South]),
            Gamepad::default(),
            None,
        );
        assert_eq!(replay, [], "never a press");
        assert!(view.keys.held(Fc::AttackTarget), "held");
        assert!(!latched(&view, Fc::AttackTarget));
        assert!(!view.keys.held(Fc::FirePrimary));
    }
    // Board's (Select and East) boards at the next poll, its latch clear.
    let mut view = in_flight();
    dialog_opens(&mut view);
    back_pad(
        &mut view,
        pressed(&[P::Select, P::East]),
        Gamepad::default(),
        None,
    );
    let from = sent(&view).len();
    view.held_keys_frame();
    assert!(
        sent(&view)[from..].contains(&FlightCommand::Board),
        "{:?}",
        sent(&view)
    );
}

#[test]
fn a_plain_west_pressed_before_select_comes_back_as_closest_target_not_communicate() {
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = back_pad(
        &mut view,
        pressed(&[P::West, P::Select]),
        Gamepad::default(),
        None,
    );
    assert_eq!(replay, [pad_press(P::West)]);
    assert_eq!(
        view.keys.peek(&replay[0]),
        Some(Command::Flight(Fc::ClosestTarget))
    );
    view.input(&replay[0]);
    assert!(!view.hail_opened(), "no hail");
}

#[test]
fn a_plain_button_pressed_under_an_unlatching_dialog_comes_back_as_a_press() {
    // The right shoulder binds latched commands on every layer (Target
    // Select, Select's Target Escort, Start's escort menu): adopted, it
    // is down in flight's reader, and still comes back.
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = back_pad(
        &mut view,
        pressed(&[P::RightShoulder]),
        Gamepad::default(),
        None,
    );
    assert_eq!(replay, [pad_press(P::RightShoulder)]);
    assert_eq!(
        view.keys.peek(&replay[0]),
        Some(Command::Flight(Fc::TargetSelect))
    );
}

#[test]
fn select_held_across_the_close_layers_a_press_after_it() {
    let mut view = at_rest(catalog());
    dialog_opens(&mut view);
    back_pad(&mut view, pressed(&[P::Select]), Gamepad::default(), None);
    view.input(&pad_press(P::South));
    assert!(view.keys.held(Fc::HyperJump));
    ticked(&mut view, 1);
    assert_eq!(last_step_input(&view).fire, (false, false));
}

#[test]
fn a_latched_layered_direction_held_across_a_sites_close_ignores_its_repeats_until_let_go() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    back_pad(
        &mut view,
        pressed(&[P::Select, P::DPadUp]),
        Gamepad::default(),
        None,
    );
    assert!(latched(&view, Fc::HyperspaceMode));
    for _ in 0..3 {
        view.input(&pad_repeat(P::DPadUp));
        view.tick(TICK);
    }
    assert!(latched(&view, Fc::HyperspaceMode), "still waiting");
    view.input(&pad_release(P::DPadUp));
    assert!(!latched(&view, Fc::HyperspaceMode), "let go");
}

#[test]
fn a_latched_pad_chord_down_at_the_poll_before_an_unlatching_dialog_waits() {
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let chord = [P::Select, P::South];
    let replay = back_pad(&mut view, pressed(&chord), pressed(&chord), None);
    assert_eq!(replay, []);
    assert!(latched(&view, Fc::HyperJump));
    assert!(view.jump_latch);
}

#[test]
fn a_latched_pad_press_read_since_the_poll_waits() {
    let mut view = in_flight();
    view.input(&pad_press(P::West));
    unlatching_dialog_opens(&mut view);
    let replay = back_pad(&mut view, pressed(&[P::West]), Gamepad::default(), None);
    assert_eq!(replay, [], "read on its press");
    assert!(latched(&view, Fc::ClosestTarget));
}

#[test]
fn a_latched_pad_button_flight_still_holds_is_no_fresh_press() {
    let mut view = in_flight();
    view.keys.follow(&pad_press(P::West));
    let replay = back_pad(&mut view, pressed(&[P::West]), Gamepad::default(), None);
    assert_eq!(replay, []);
    assert!(view.keys.is_down(&pad_press(P::West)), "kept");
}

#[test]
fn pad_and_key_presses_come_back_in_one_poll_order() {
    // Tab (Target Select) before West (Closest Target) before J.
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = view.take_back_live_input(LiveKeys {
        down: [JUMP_KEY, Key::Tab].into_iter().collect(),
        pad: pressed(&[P::West]),
        ..LiveKeys::default()
    });
    assert_eq!(
        replay,
        [key(Key::Tab, true), pad_press(P::West), key(JUMP_KEY, true)]
    );
    // A slot picked by Start and the d-pad acts at the poll: J after it
    // is kept for that poll.
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = view.take_back_live_input(LiveKeys {
        down: [JUMP_KEY].into_iter().collect(),
        pad: pressed(&[P::Start, P::DPadUp]),
        ..LiveKeys::default()
    });
    assert_eq!(replay, [pad_press(P::DPadUp)]);
    assert_eq!(view.after_poll, [key(JUMP_KEY, true)]);
}

#[test]
fn an_own_latch_pad_entry_clear_keeps_the_presses_after_it_for_the_poll() {
    // Start and South (Attack Target) acts at the poll, its latch clear
    // (@0x6e1b6, before P's read): P after it is kept for that poll.
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = view.take_back_live_input(LiveKeys {
        down: [PLAYER_INFO_KEY, Key::Tab].into_iter().collect(),
        pad: pressed(&[P::Start, P::South]),
        ..LiveKeys::default()
    });
    assert_eq!(replay, [key(Key::Tab, true)], "Tab polled before it");
    assert_eq!(view.after_poll, [key(PLAYER_INFO_KEY, true)]);
}

#[test]
fn a_dialog_opening_lets_go_of_the_adopted_pad_and_its_axes() {
    let mut view = at_rest(catalog());
    dialog_opens(&mut view);
    let mut pad = pressed(&[P::DPadUp]);
    pad.set_axis(PadAxis::RightX, raw(0.6));
    back_pad(&mut view, pad, Gamepad::default(), None);
    unlatching_dialog_opens(&mut view);
    ticked(&mut view, 1);
    let input = last_step_input(&view);
    assert!(!input.controls.thrust);
    assert_eq!(input.controls.turn, Turn::None);
    assert!(input.amounts.turn.abs() < 1e-6, "{input:?}");
}
