//! Flight finds its commands through the bindings it is given
//! ([`FlightView::with_bindings`]): a rebound command answers its new key
//! and no longer its old one, a variant follows its rebound base, a fixed
//! twin stays put, and the escort menu's groups shadow the stellar slots
//! while it shows (rdm:phase/key-bindings/phase-2-command-key-map's AC 6).

use super::faked_flight::{at_rest, last_controls, ticked};
use super::faked_keys::fleet;
use super::faked_screen::{sent_count, sent_since};
use super::*;
use crate::keymap::{Bindings, FlightCommand as Fc, KeyCombo, PlayCommand, SharedBindings};
use nova_sim::{StellarPick, TargetPick};

/// The default bindings with `command` bound to `k` alone.
fn rebound(command: impl Into<crate::keymap::Command>, k: Key) -> SharedBindings {
    SharedBindings::new(Bindings::default().rebound(command, KeyCombo::key(k)))
}

/// Holds `k` for one tick of `view`'s flight, then lets it go: whether
/// that tick's step thrusts.
fn thrusts_holding(view: &mut Faked, k: Key) -> bool {
    view.input(&key(k, true));
    ticked(view, 1);
    let thrust = last_controls(view).thrust;
    view.input(&key(k, false));
    thrust
}

/// The stellars `view` sent to select since `from` of its commands.
fn picks_since(view: &Faked, from: usize) -> Vec<StellarPick> {
    sent_since(view, from)
        .into_iter()
        .filter_map(|command| match command {
            FlightCommand::SelectStellar(pick) => Some(pick),
            _ => None,
        })
        .collect()
}

/// Taps `k` on `view`, then runs the slot block's next frame: the
/// stellars picked.
fn tap_slot(view: &mut Faked, k: Key) -> Vec<StellarPick> {
    let from = sent_count(view);
    tap(view, k);
    view.nav_held_frame();
    picks_since(view, from)
}

#[test]
fn a_rebound_held_command_holds_on_its_new_key_and_not_its_old() {
    let mut view = at_rest(catalog()).with_bindings(rebound(Fc::Accelerate, Key::KeyT));
    assert!(thrusts_holding(&mut view, Key::KeyT), "T thrusts");
    assert!(
        !thrusts_holding(&mut view, Key::ArrowUp),
        "Up no longer does"
    );
}

#[test]
fn a_rebound_pressed_command_acts_on_its_new_key_and_not_its_old() {
    let mut view = faked(in_sol());
    view.set_bindings(rebound(Fc::LandDock, Key::KeyT));
    let from = sent_count(&view);
    tap(&mut view, LAND_KEY);
    assert!(
        !sent_since(&view, from).contains(&FlightCommand::Land),
        "L no longer lands"
    );
    tap(&mut view, Key::KeyT);
    assert!(
        sent_since(&view, from).contains(&FlightCommand::Land),
        "T does"
    );
}

#[test]
fn a_variant_follows_its_rebound_base() {
    let mut view =
        fleet(&[EscortClass::Fighter]).with_bindings(rebound(Fc::TargetSelect, Key::KeyT));
    let from = sent_count(&view);
    view.input(&key(Key::ShiftLeft, true));
    tap(&mut view, TARGET_KEY);
    assert_eq!(sent_since(&view, from), [], "Shift-Tab no longer targets");
    tap(&mut view, Key::KeyT);
    view.input(&key(Key::ShiftLeft, false));
    assert_eq!(
        sent_since(&view, from),
        [FlightCommand::SelectTarget(TargetPick::Previous)],
        "Shift-T targets backwards"
    );
}

#[test]
fn a_fixed_twin_stays_on_its_key_when_its_command_is_rebound() {
    let mut view = faked(in_sol());
    view.set_bindings(rebound(Fc::StellarSlot1, Key::KeyQ));
    assert_eq!(tap_slot(&mut view, Key::Digit1), [], "1 no longer picks");
    assert_eq!(tap_slot(&mut view, Key::KeyQ), [StellarPick::Slot(0)], "Q");
    view.nav_held_frame();
    assert_eq!(tap_slot(&mut view, Key::F1), [StellarPick::Slot(0)], "F1");
}

#[test]
fn while_the_escort_menu_shows_1_picks_its_group_and_leaves_the_stellar() {
    let mut view = fleet(&[EscortClass::Warship]);
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    let _ = view.take_sounds();
    assert_eq!(tap_slot(&mut view, Key::Digit1), [], "the menu's");
    assert!(
        view.take_sounds()
            .contains(&Sound::Ui(UiSound::EscortGroupSelected)),
        "group 1 selected"
    );
    tap(&mut view, MENU_KEY);
    assert!(!view.escort_menu().is_open());
    view.nav_held_frame();
    assert_eq!(
        tap_slot(&mut view, Key::Digit1),
        [StellarPick::Slot(0)],
        "hidden, the stellar slot's"
    );
}

// What flight builds that reads keys reads flight's bindings.

#[test]
fn the_rebound_map_key_opens_the_course_map_and_its_release_ends_the_wait() {
    let mut view = faked_sol(in_sol());
    view.set_bindings(rebound(PlayCommand::GalaxyMap, Key::KeyT));
    tap(&mut view, MAP_KEY);
    assert!(!view.map_open(), "M no longer opens it");
    view.input(&key(Key::KeyT, true));
    assert!(view.map_open(), "T does");
    view.input(&key(Key::KeyT, false));
    view.input(&key(Key::KeyT, true));
    assert!(view.map_waiting(), "T's close waits for its release");
    view.input(&key(MAP_KEY, false));
    assert!(view.map_waiting(), "M's release is no T's");
    view.input(&key(Key::KeyT, false));
    assert!(!view.map_open(), "closed on T's release");
}

#[test]
fn the_course_map_and_a_port_maps_wait_read_flights_bindings() {
    let mut view = super::faked_shops::docked();
    let shared = rebound(PlayCommand::GalaxyMap, Key::KeyT);
    view.set_bindings(shared.clone());
    assert_eq!(view.course_map().bindings().get(), shared.get());
    queue_map(&mut view, Some(130), sol_drawing());
    view.open_bar_map();
    assert!(view.map_open());
    view.port_map_input(&key(Key::KeyT, true));
    assert!(view.map_waiting(), "T closes it, waiting for its release");
    view.port_map_input(&held(Key::KeyT));
    view.port_map_input(&key(Key::KeyX, true));
    view.port_map_input(&key(Key::KeyT, false));
    assert!(!view.map_open());
    assert_eq!(
        view.take_map_replay(),
        [key(Key::KeyX, true)],
        "the waited key, T, its repeat too, isn't queued for the screen"
    );
}
