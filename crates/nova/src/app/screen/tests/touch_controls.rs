//! Flight's touch controls through the router
//! (rdm:phase/key-bindings/phase-15-touch-flight-controls): the router
//! claims a touch for flight only while flight shows with nothing over
//! it, hands the claimed finger to flight's overlay, and runs what the
//! overlay raises back through its own input from the top, as it does a
//! key, so its own readers see Pause and Double Time. The overlay's rules
//! are `nova_view::flight::touch_controls`' and flight's path is
//! `nova_view`'s `flight/view/tests/touch_flight.rs`; this pins the
//! wiring over the real flight.

#![allow(clippy::float_cmp)]

use super::*;
use nova_view::flight::touch_controls::draw::{FILL, HIGHLIGHT};
use nova_view::flight::touch_controls::layout::ControlId;
use nova_view::keymap::{AppCommand, FlightCommand, PauseCommand, PlayCommand};
use nova_view::{Command, Device, FingerId, Touch, TouchPhase};

fn told(screen: &mut AppScreen, device: Device) {
    screen.input(&Input::Surroundings(nova_view::Surroundings::of(device)));
}

/// Flight, told touch was used last.
fn flying_by_touch() -> AppScreen {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    told(&mut screen, Device::Touch);
    screen
}

/// Where control `id` is placed now.
fn centre(screen: &AppScreen, id: ControlId) -> Point {
    flight(screen)
        .touch_controls()
        .layout()
        .get(id)
        .unwrap_or_else(|| panic!("{id:?} placed"))
        .rect
        .center()
}

fn command(command: impl Into<Command>) -> ControlId {
    ControlId::Command(command.into())
}

/// Finger `n` does `phase`, as the app routes a claimed finger.
fn finger(screen: &mut AppScreen, n: usize, phase: TouchPhase) -> ScreenAction {
    screen.input(&Input::Touch(Touch {
        finger: FingerId::new(n),
        phase,
    }))
}

/// A tap on control `id`, opening the menu first when it is a cell.
fn tap(screen: &mut AppScreen, id: ControlId) {
    if flight(screen).touch_controls().layout().get(id).is_none() {
        let more = centre(screen, ControlId::More);
        finger(screen, 9, TouchPhase::Started(more));
        finger(screen, 9, TouchPhase::Ended(more));
    }
    let at = centre(screen, id);
    assert!(screen.claims_touch(at), "{id:?} claimed");
    finger(screen, 1, TouchPhase::Started(at));
    finger(screen, 1, TouchPhase::Ended(at));
}

#[test]
fn the_router_claims_a_touch_for_flight_only_while_flight_shows_alone() {
    let mut screen = flying_by_touch();
    let fire = centre(&screen, command(FlightCommand::FirePrimary));
    assert!(screen.claims_touch(fire), "flight shows");
    assert!(
        !screen.claims_touch(Point::new(500.0, 300.0)),
        "off the controls"
    );
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert!(flight(&screen).map_open());
    assert!(!screen.claims_touch(fire), "the map open");
    let mut over = flying_by_touch();
    over.menu_command(MenuCommand::Preferences);
    assert_eq!(over.showing(), Showing::Preferences);
    assert!(!over.claims_touch(fire), "a dialog over flight");
    let mut left = flying_by_touch();
    left.input(&key(Key::Escape, true));
    assert_ne!(left.showing(), Showing::Flight);
    assert!(!left.claims_touch(fire), "flight left");
    let mut unflown = with_dialogs(data());
    assert!(!unflown.claims_touch(fire), "no flight");
    told(&mut unflown, Device::Touch);
    assert!(!unflown.claims_touch(fire), "no flight, by touch");
}

#[test]
fn a_stick_push_through_the_router_turns_the_ship_as_the_turn_key_does() {
    let mut pushed = flying_by_touch();
    let base = centre(&pushed, ControlId::Stick);
    let radius = flight(&pushed).touch_controls().stick_radius();
    finger(&mut pushed, 1, TouchPhase::Started(base));
    finger(
        &mut pushed,
        1,
        TouchPhase::Moved(Point::new(base.x + 2.0 * radius, base.y)),
    );
    let mut keyed = flying_by_touch();
    keyed.input(&key(Key::ArrowRight, true));
    let before = ship(&pushed).heading;
    for _ in 0..10 {
        pushed.tick(TICK);
        keyed.tick(TICK);
    }
    assert_ne!(ship(&pushed).heading, before, "turned");
    assert_eq!(ship(&pushed).heading, ship(&keyed).heading);
}

#[test]
fn the_overlays_pause_leaves_flight_as_escape_does() {
    let mut touched = flying_by_touch();
    tap(&mut touched, command(PauseCommand::Pause));
    let mut escaped = flying_by_touch();
    escaped.input(&key(Key::Escape, true));
    escaped.input(&key(Key::Escape, false));
    assert_ne!(escaped.showing(), Showing::Flight);
    assert_eq!(touched.showing(), escaped.showing());
}

#[test]
fn the_overlays_double_time_toggles_double_time_as_its_key_does() {
    let mut touched = flying_by_touch();
    tap(&mut touched, command(AppCommand::DoubleTime));
    assert!(touched.double_time, "on");
    assert!(flight(&touched).double_time());
    tap(&mut touched, command(AppCommand::DoubleTime));
    assert!(!touched.double_time, "off again");
    let mut keyed = flying_by_touch();
    keyed.input(&key(Key::CapsLock, true));
    keyed.input(&key(Key::CapsLock, false));
    assert!(keyed.double_time);
}

#[test]
fn a_control_that_opens_a_dialog_is_released_as_it_opens_and_its_lift_does_nothing() {
    let mut screen = flying_by_touch();
    let fire = centre(&screen, command(FlightCommand::FirePrimary));
    finger(&mut screen, 2, TouchPhase::Started(fire));
    assert!(flight(&screen).holds_keys(), "fire held");
    let more = centre(&screen, ControlId::More);
    finger(&mut screen, 9, TouchPhase::Started(more));
    finger(&mut screen, 9, TouchPhase::Ended(more));
    let info = centre(&screen, command(PlayCommand::PlayerInfo));
    finger(&mut screen, 1, TouchPhase::Started(info));
    assert_eq!(screen.showing(), Showing::PlayerInfo, "opened");
    assert!(!flight(&screen).holds_keys(), "nothing held in flight");
    assert!(
        !screen.flight_keys.held(FlightCommand::FirePrimary),
        "nothing held in the router"
    );
    assert!(!screen.flight_keys.held(FlightCommand::FirePrimary));
    finger(&mut screen, 1, TouchPhase::Ended(info));
    finger(&mut screen, 2, TouchPhase::Ended(fire));
    assert_eq!(
        screen.showing(),
        Showing::PlayerInfo,
        "the lifts route nothing"
    );
    assert!(!screen.keys.anything_down());
}

#[test]
fn opening_the_map_from_the_overlay_lets_go_of_what_it_held() {
    let mut screen = flying_by_touch();
    let fire = centre(&screen, command(FlightCommand::FirePrimary));
    finger(&mut screen, 2, TouchPhase::Started(fire));
    tap(&mut screen, command(PlayCommand::GalaxyMap));
    assert!(flight(&screen).map_open());
    assert!(!flight(&screen).holds_keys(), "fire let go under the map");
    assert!(!screen.flight_keys.held(FlightCommand::FirePrimary));
}

/// Whether `screen` draws the overlay: its fill.
fn overlay_drawn(screen: &AppScreen) -> bool {
    let mut list = DrawList::new();
    screen.draw(&mut list);
    list.iter().any(|command| {
        matches!(*command, DrawCommand::Line { color, .. } if color == FILL || color == HIGHLIGHT)
    })
}

#[test]
fn the_overlay_is_drawn_over_flight_and_not_under_a_dialog() {
    let mut screen = flying_by_touch();
    assert!(overlay_drawn(&screen));
    let mut list = DrawList::new();
    screen.draw(&mut list);
    let drawn: Vec<&DrawCommand> = list.iter().collect();
    let first_overlay = drawn
        .iter()
        .position(|command| matches!(**command, DrawCommand::Line { color, .. } if color == FILL))
        .expect("drawn");
    let mut flight_only = DrawList::new();
    flight(&screen).draw(&mut flight_only);
    assert!(first_overlay >= flight_only.iter().count(), "over flight");
    screen.menu_command(MenuCommand::Preferences);
    assert!(!overlay_drawn(&screen), "not under a dialog");
    let mut keyed = flying_by_touch();
    told(&mut keyed, Device::Keyboard);
    assert!(!overlay_drawn(&keyed), "hidden");
}

#[test]
fn a_key_press_hides_the_overlay_and_a_touch_shows_it() {
    let mut screen = flying_by_touch();
    let fire = centre(&screen, command(FlightCommand::FirePrimary));
    finger(&mut screen, 1, TouchPhase::Started(fire));
    assert!(flight(&screen).holds_keys());
    told(&mut screen, Device::Keyboard);
    assert!(!screen.claims_touch(fire));
    assert!(!flight(&screen).holds_keys(), "hiding let go of fire");
    assert!(!screen.flight_keys.held(FlightCommand::FirePrimary));
    told(&mut screen, Device::Touch);
    assert!(screen.claims_touch(fire));
}

#[test]
fn letting_go_of_the_keys_releases_the_overlay_through_the_router() {
    let mut screen = flying_by_touch();
    let fire = centre(&screen, command(FlightCommand::FirePrimary));
    assert_eq!(
        finger(&mut screen, 1, TouchPhase::Started(fire)),
        ScreenAction::None,
        "fed through, quitting nothing"
    );
    assert!(
        screen.flight_keys.held(FlightCommand::FirePrimary),
        "the router saw fire down"
    );
    screen.release_keys();
    assert!(!screen.flight_keys.held(FlightCommand::FirePrimary));
    assert!(!flight(&screen).holds_keys());
    assert_eq!(
        finger(&mut screen, 1, TouchPhase::Ended(fire)),
        ScreenAction::None
    );
}
