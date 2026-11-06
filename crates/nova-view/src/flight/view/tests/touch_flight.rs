//! Flying from the touch controls
//! (rdm:phase/key-bindings/phase-15-touch-flight-controls): the stick's
//! analog turn and throttle in the step input, each control doing what
//! its key does through flight's own key path, fire held by a finger
//! until it lifts or is cancelled, the overlay shown only while touch was
//! used last and flight flies live, and a control whose press opens a
//! dialog released as it opens. The overlay's own roles are
//! `touch_controls`' tests; the router's drain is `nova`'s.
//!
//! [`touch`] does what the router does with what the overlay raises:
//! each input it drains goes back through flight's input.

use super::controls::{clicked, cursor_sprites, faked_with_npc, left_press, left_release};
use super::faked_flight::{at_rest, last_step_input, ticked};
use super::faked_hail::hail_view;
use super::*;
use crate::flight::touch_controls::layout::{ControlId, EDGE, LARGE, MENU};
use crate::input::{ControllerFamily, Device, FingerId, PadAxis, Surroundings, Touch, TouchPhase};
use crate::keymap::command::Kind;
use crate::keymap::{FlightCommand as F, KeyMap, Modifiers, dead_zoned};
use nova_sim::{Hailed, Turn};

/// Finger `n` does `phase` on `view`'s touch controls; what the overlay
/// raises is fed back through flight's input, as the router does. The
/// actions every input answered with.
fn touch(view: &mut Faked, n: usize, phase: TouchPhase) -> Vec<ScreenAction> {
    let mut actions = vec![view.input(&Input::Touch(Touch {
        finger: FingerId::new(n),
        phase,
    }))];
    drain(view, &mut actions);
    actions
}

/// Feeds what the overlay raised back through flight's input until it
/// raises nothing more.
fn drain(view: &mut Faked, actions: &mut Vec<ScreenAction>) {
    loop {
        let queued = view.take_touch_controls();
        if queued.is_empty() {
            return;
        }
        for input in queued {
            actions.push(view.input(&input));
        }
    }
}

fn told(view: &mut Faked, device: Device) {
    view.input(&Input::Surroundings(Surroundings::of(device)));
    let mut actions = Vec::new();
    drain(view, &mut actions);
}

/// A view at rest in Sol, told touch was used last.
fn touched() -> Faked {
    let mut view = at_rest(catalog());
    told(&mut view, Device::Touch);
    view
}

fn centre(view: &Faked, id: ControlId) -> Point {
    view.touch_controls
        .layout()
        .get(id)
        .unwrap_or_else(|| panic!("{id:?} placed"))
        .rect
        .center()
}

fn fire(view: &Faked) -> Point {
    centre(view, ControlId::Command(Command::Flight(F::FirePrimary)))
}

/// The step input after a tick.
fn stepped(view: &mut Faked) -> nova_sim::flight_frame::StepInput {
    ticked(view, 1);
    last_step_input(view)
}

fn radius(view: &Faked) -> f32 {
    view.touch_controls.stick_radius()
}

/// The stick held, with the thumb at (`x`, `y`) radii from where it
/// landed, forward up.
fn stick_at(view: &mut Faked, x: f32, y: f32) -> Point {
    let base = centre(view, ControlId::Stick);
    touch(view, 1, TouchPhase::Started(base));
    let r = radius(view);
    let at = Point::new(base.x + x * r, base.y - y * r);
    touch(view, 1, TouchPhase::Moved(at));
    at
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn the_stick_carries_analog_turn_and_throttle_into_the_step_input() {
    let mut view = touched();
    stick_at(&mut view, 0.45, 0.55);
    let input = stepped(&mut view);
    let (turn, thrust) = dead_zoned(0.45, 0.55);
    assert_eq!(input.controls.turn, Turn::Right);
    assert!(input.controls.thrust);
    assert!(near(input.amounts.turn, turn), "{input:?}");
    assert!(near(input.amounts.thrust, thrust), "{input:?}");
    assert!(!input.controls.reverse);
    let mut left = touched();
    stick_at(&mut left, -0.7, 0.0);
    let input = stepped(&mut left);
    assert_eq!(input.controls.turn, Turn::Left);
    assert!(
        near(input.amounts.turn, dead_zoned(0.7, 0.0).0),
        "{input:?}"
    );
    assert!(!input.controls.thrust);
}

#[test]
fn a_half_radius_push_sideways_gives_about_half_the_turn() {
    let mut view = touched();
    stick_at(&mut view, 0.5, 0.0);
    let input = stepped(&mut view);
    assert_eq!(input.controls.turn, Turn::Right);
    assert!(near(input.amounts.turn, dead_zoned(0.5, 0.0).0));
    assert!((0.35..0.6).contains(&input.amounts.turn), "{input:?}");
}

#[test]
fn pulling_the_stick_back_past_the_threshold_holds_reverse() {
    let mut view = touched();
    stick_at(&mut view, 0.0, -0.6);
    let input = stepped(&mut view);
    assert!(input.controls.reverse, "{input:?}");
    assert!(!input.controls.thrust);
    let mut short = touched();
    stick_at(&mut short, 0.0, -0.3);
    assert!(
        !stepped(&mut short).controls.reverse,
        "short of the threshold"
    );
}

#[test]
fn lifting_the_finger_sets_both_amounts_to_zero() {
    let mut view = touched();
    let at = stick_at(&mut view, 0.6, -0.8);
    assert!(stepped(&mut view).controls.reverse);
    touch(&mut view, 1, TouchPhase::Ended(at));
    let input = stepped(&mut view);
    assert_eq!(input.controls, Controls::default(), "{input:?}");
    assert_eq!(input.amounts.turn, 0.0);
    assert_eq!(input.amounts.thrust, 0.0);
}

#[test]
fn a_key_still_wins_over_a_smaller_stick_amount() {
    let mut view = touched();
    stick_at(&mut view, 0.4, 0.0);
    super::faked_flight::hold(&mut view, Key::ArrowLeft, true);
    let input = stepped(&mut view);
    assert_eq!(input.controls.turn, Turn::Left);
    assert_eq!(input.amounts.turn, 1.0);
}

#[test]
fn the_larger_of_the_pads_and_the_sticks_turns() {
    let mut view = touched();
    stick_at(&mut view, 0.9, 0.0);
    view.input(&Input::PadAxis {
        axis: PadAxis::RightX,
        value: -0.4,
    });
    let input = stepped(&mut view);
    assert_eq!(input.controls.turn, Turn::Right, "the stick's, larger");
    assert!(near(input.amounts.turn, dead_zoned(0.9, 0.0).0));
}

#[test]
fn touch_held_fire_fires_until_the_finger_lifts() {
    let mut view = touched();
    let at = fire(&view);
    touch(&mut view, 1, TouchPhase::Started(at));
    assert!(view.keys.held(F::FirePrimary));
    ticked(&mut view, 1);
    assert!(view.keys.held(F::FirePrimary), "held across a tick");
    touch(&mut view, 1, TouchPhase::Ended(at));
    assert!(!view.keys.held(F::FirePrimary));
}

#[test]
fn a_touch_sliding_off_fire_keeps_firing_until_it_ends() {
    let mut view = touched();
    let at = fire(&view);
    touch(&mut view, 1, TouchPhase::Started(at));
    let off = Point::new(300.0, 300.0);
    touch(&mut view, 1, TouchPhase::Moved(off));
    ticked(&mut view, 1);
    assert!(view.keys.held(F::FirePrimary));
    touch(&mut view, 1, TouchPhase::Ended(off));
    assert!(!view.keys.held(F::FirePrimary));
}

#[test]
fn a_cancelled_touch_on_fire_stops_firing() {
    let mut view = touched();
    let at = fire(&view);
    touch(&mut view, 1, TouchPhase::Started(at));
    touch(&mut view, 1, TouchPhase::Cancelled);
    assert!(!view.keys.held(F::FirePrimary));
    assert!(view.holds_no_key());
}

/// Whether `view` shows its overlay: it claims a touch on fire, and draws.
fn shows(view: &Faked) -> bool {
    let claims = view.claims_touch(fire(view));
    let mut list = DrawList::new();
    view.draw_touch_controls(&mut list);
    assert_eq!(claims, list.iter().next().is_some(), "claims as it draws");
    claims
}

#[test]
fn the_overlay_shows_while_touch_was_used_last_and_hides_after_a_key_a_pad_or_the_mouse() {
    let mut view = at_rest(catalog());
    assert!(!shows(&view), "the keyboard to start with");
    for device in [
        Device::Keyboard,
        Device::Gamepad(ControllerFamily::Xbox),
        Device::Mouse,
    ] {
        told(&mut view, Device::Touch);
        assert!(shows(&view), "{device:?}: touch");
        told(&mut view, device);
        assert!(!shows(&view), "{device:?}: hidden");
    }
    told(&mut view, Device::Touch);
    assert!(shows(&view), "a touch shows it again");
}

#[test]
fn hiding_mid_hold_stops_firing() {
    let mut view = touched();
    let at = fire(&view);
    touch(&mut view, 1, TouchPhase::Started(at));
    assert!(view.keys.held(F::FirePrimary));
    told(&mut view, Device::Keyboard);
    assert!(!view.keys.held(F::FirePrimary));
    told(&mut view, Device::Touch);
    touch(&mut view, 1, TouchPhase::Ended(at));
    assert!(!view.keys.held(F::FirePrimary), "the finger was let go");
}

#[test]
fn the_overlay_claims_nothing_with_the_map_open_a_movie_playing_or_flight_failed() {
    let mut mapped = touched();
    let at = fire(&mapped);
    touch(&mut mapped, 1, TouchPhase::Started(at));
    mapped.open_map_by_key();
    mapped.input(&Input::PointerLeft);
    let mut actions = Vec::new();
    drain(&mut mapped, &mut actions);
    assert!(mapped.map_open());
    assert!(!shows(&mapped), "the map open");
    assert!(
        !mapped.keys.held(F::FirePrimary),
        "let go as the map opened"
    );
    let mut failed = touched();
    failed.failure = Some("no flight".to_owned());
    failed.input(&Input::PointerLeft);
    assert!(!shows(&failed), "flight failed");
    let mut dead = touched();
    dead.frame.hold_expired = true;
    dead.input(&Input::PointerLeft);
    assert!(!shows(&dead), "dead");
    let (mut movie, _) =
        super::faked_movies::filming(super::faked_movies::filmed(&[129], &[]), true);
    told(&mut movie, Device::Touch);
    assert!(shows(&movie), "live before the movie");
    let at = fire(&movie);
    touch(&mut movie, 1, TouchPhase::Started(at));
    super::faked_movies::land_on_mars(&mut movie);
    movie.input(&Input::PointerLeft);
    let mut actions = Vec::new();
    drain(&mut movie, &mut actions);
    assert!(movie.playing());
    assert!(!shows(&movie), "a movie playing");
    assert!(
        !movie.keys.held(F::FirePrimary),
        "let go as the movie began"
    );
}

#[test]
fn a_tap_off_the_controls_is_still_a_click_in_space() {
    let (mut by_mouse, id) = faked_with_npc();
    let over = npc_on_screen(&by_mouse, id);
    by_mouse.input(&Input::PointerMoved(over));
    left_press(&mut by_mouse, over);
    left_release(&mut by_mouse, over);
    let (mut by_touch, _) = faked_with_npc();
    told(&mut by_touch, Device::Touch);
    assert!(!by_touch.claims_touch(over), "{over:?} is off the controls");
    {
        let mut fingers = crate::touch::testkit::Fingers::claimed_by_screen(&mut by_touch);
        fingers.down(1, over);
        fingers.up(1, over);
    }
    assert!(clicked(&by_touch), "{:?}", sent(&by_touch));
    let cursor = |view: &Faked| cursor_sprites(&super::faked_world::world_list(view));
    assert_eq!(cursor(&by_touch), cursor(&by_mouse));
    let mut fired = faked_with_npc().0;
    told(&mut fired, Device::Touch);
    let at = fire(&fired);
    {
        let mut fingers = crate::touch::testkit::Fingers::claimed_by_screen(&mut fired);
        fingers.down(1, at);
    }
    assert!(!clicked(&fired), "a claimed finger is no click");
    assert!(
        fired.take_touch_controls().contains(&Input::TouchControl(
            crate::input::TouchControl::Button {
                command: F::FirePrimary.into(),
                pressed: true,
            }
        )),
        "it went to the overlay"
    );
}

#[test]
fn a_control_whose_press_opens_a_dialog_is_released_as_it_opens_and_its_lift_does_nothing() {
    let mut view = touched();
    view.sim
        .queue_answer(nova_sim::flight_frame::Answer::Hailed(Hailed::Comm(
            hail_view("Channel open."),
        )));
    let hail = centre(&view, ControlId::Command(F::Communicate.into()));
    let fire_at = fire(&view);
    touch(&mut view, 2, TouchPhase::Started(fire_at));
    view.input(&Input::Touch(Touch {
        finger: FingerId::new(1),
        phase: TouchPhase::Started(hail),
    }));
    let pressed = view.take_touch_controls();
    assert_eq!(
        pressed,
        [Input::TouchControl(crate::input::TouchControl::Button {
            command: F::Communicate.into(),
            pressed: true,
        })]
    );
    let mut opened = view.input(&pressed[0]);
    assert_eq!(opened, ScreenAction::None);
    let queued = view.take_touch_controls();
    let release = |command: F| {
        Input::TouchControl(crate::input::TouchControl::Button {
            command: command.into(),
            pressed: false,
        })
    };
    assert!(view.take_hail().is_some(), "the comm dialog opened");
    assert!(queued.contains(&release(F::Communicate)), "{queued:?}");
    assert!(queued.contains(&release(F::FirePrimary)), "{queued:?}");
    assert!(view.holds_no_key(), "nothing held under the dialog");
    for input in queued {
        opened = view.input(&input);
        assert_eq!(opened, ScreenAction::None);
    }
    assert_eq!(
        touch(&mut view, 1, TouchPhase::Ended(hail)),
        [ScreenAction::None]
    );
    assert_eq!(view.take_touch_controls(), [], "its lift routes nothing");
}

/// The keys that press `command`'s default combo, modifiers first.
fn combo_keys(command: Command) -> Option<Vec<Key>> {
    let combo = KeyMap::default().combo(command)?;
    let mut keys: Vec<Key> = [
        (Modifiers::COMMAND, Key::MetaLeft),
        (Modifiers::CONTROL, Key::ControlLeft),
        (Modifiers::OPTION, Key::AltLeft),
        (Modifiers::SHIFT, Key::ShiftLeft),
    ]
    .into_iter()
    .filter(|&(modifier, _)| combo.modifiers.contains(modifier))
    .map(|(_, key)| key)
    .collect();
    keys.push(combo.key);
    Some(keys)
}

/// What a run did, to compare a key's with a control's.
#[derive(Debug, PartialEq)]
struct Outcome {
    applied: Vec<FlightCommand>,
    asks: Vec<FlightCommand>,
    inputs: Vec<nova_sim::flight_frame::StepInput>,
    sounds: Vec<Sound>,
    message: Option<String>,
    map_open: bool,
    actions: Vec<ScreenAction>,
}

fn outcome(view: &mut Faked, actions: Vec<ScreenAction>) -> Outcome {
    Outcome {
        applied: view.sim.applied().to_vec(),
        asks: view.sim.asks().to_vec(),
        inputs: view.sim.inputs(),
        sounds: view.take_sounds(),
        message: view.message().map(str::to_owned),
        map_open: view.map_open(),
        actions: actions
            .into_iter()
            .filter(|&action| action != ScreenAction::None)
            .collect(),
    }
}

/// `command` by its key: its combo pressed, a tick, then let go.
fn by_key(keys: &[Key]) -> Outcome {
    let mut view = touched();
    let mut actions = Vec::new();
    for &k in keys {
        actions.push(view.input(&key(k, true)));
    }
    view.tick(TICK);
    for &k in keys.iter().rev() {
        actions.push(view.input(&key(k, false)));
    }
    outcome(&mut view, actions)
}

/// `command` by its control on the overlay: touched, a tick, then lifted.
fn by_touch(command: Command) -> Outcome {
    let mut view = touched();
    let mut actions = Vec::new();
    let id = ControlId::Command(command);
    if view.touch_controls.layout().get(id).is_none() {
        let more = centre(&view, ControlId::More);
        actions.extend(touch(&mut view, 9, TouchPhase::Started(more)));
        actions.extend(touch(&mut view, 9, TouchPhase::Ended(more)));
    }
    let at = centre(&view, id);
    actions.extend(touch(&mut view, 1, TouchPhase::Started(at)));
    view.tick(TICK);
    actions.extend(touch(&mut view, 1, TouchPhase::Ended(at)));
    outcome(&mut view, actions)
}

#[test]
fn every_overlay_control_does_what_its_key_does() {
    // Pause and Double Time are the router's (`nova`'s
    // `app/screen/tests/touch_controls.rs` pins them).
    let routers = [
        Command::Pause(crate::keymap::PauseCommand::Pause),
        Command::App(crate::keymap::AppCommand::DoubleTime),
    ];
    let idle = by_key(&[]);
    let (mut checked, mut acted) = (0, 0);
    for command in LARGE.into_iter().chain(EDGE).chain(MENU) {
        if routers.contains(&command) || command.kind() == Kind::Axis {
            continue;
        }
        let Some(keys) = combo_keys(command) else {
            continue;
        };
        let touched = by_touch(command);
        if touched != idle {
            acted += 1;
        }
        assert_eq!(touched, by_key(&keys), "{command:?} by {keys:?}");
        checked += 1;
    }
    assert!(checked > 40, "{checked}");
    // Most commands do something visible to the fake at rest in Sol, so
    // the comparison is no comparison of two idle runs.
    assert!(acted > 25, "{acted} of {checked} acted");
}
