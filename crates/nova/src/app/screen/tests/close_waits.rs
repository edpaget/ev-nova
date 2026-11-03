//! The closes that wait for their key's release, through the real router,
//! flight and spaceport: the player info screen's for P and a mission
//! offer's for I (the router's `close_waits` module). The waits' own
//! rules are nova-view's (`ui::key_wait`, `ui::player_info`,
//! `spaceport::offer::close`).

use super::map_wait_keys::{offered_docked, offered_over_flight};
use super::*;
use nova_view::ui::mission_info::MISSION_INFO_KEY;
use nova_view::ui::player_info::PLAYER_INFO_KEY;

/// Presses and lets go of `k`.
fn tap(screen: &mut AppScreen, k: Key) {
    screen.input(&key(k, true));
    screen.input(&key(k, false));
}

/// The flight's frame gain as it stands.
fn gain(screen: &AppScreen) -> nova_sim::SpeedMult {
    flight(screen).session().expect("flying").speed_mult()
}

/// Flying, with the player info opened by P, still down, and closed with
/// Return.
fn closed_with_p_down() -> AppScreen {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(PLAYER_INFO_KEY, true));
    assert_eq!(screen.showing(), Showing::PlayerInfo);
    screen.focus_lost();
    screen.focus_gained();
    screen.input(&key(PLAYER_INFO_KEY, true));
    tap(&mut screen, Key::Enter);
    screen
}

#[test]
fn the_player_info_over_flight_closed_with_p_down_waits_for_ps_release() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(PLAYER_INFO_KEY, true));
    assert_eq!(screen.showing(), Showing::PlayerInfo);
    tap(&mut screen, Key::Enter);
    assert_eq!(screen.showing(), Showing::PlayerInfo, "waiting");
    let alpha = flight(&screen).alpha();
    screen.tick(TICK);
    assert_eq!(
        flight(&screen).alpha().to_bits(),
        alpha.to_bits(),
        "flight is paused"
    );
    screen.input(&key(PLAYER_INFO_KEY, false));
    assert_eq!(screen.showing(), Showing::Flight, "closed on the release");
}

#[test]
fn the_closes_refresh_and_zero_come_on_the_release() {
    // The focus switch sets the gain full, so the close's zero shows.
    let mut screen = closed_with_p_down();
    assert_eq!(gain(&screen), nova_sim::SpeedMult::FULL, "not on the press");
    screen.input(&key(PLAYER_INFO_KEY, false));
    assert_eq!(gain(&screen), nova_sim::SpeedMult::ZERO, "the close's");
}

#[test]
fn a_tap_of_p_closes_on_its_release() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    tap(&mut screen, PLAYER_INFO_KEY);
    assert_eq!(screen.showing(), Showing::PlayerInfo);
    screen.input(&key(PLAYER_INFO_KEY, true));
    assert_eq!(screen.showing(), Showing::PlayerInfo, "the press waits");
    screen.input(&key(PLAYER_INFO_KEY, false));
    assert_eq!(screen.showing(), Showing::Flight);
}

#[test]
fn over_flight_alone_nothing_typed_during_the_wait_is_replayed() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(PLAYER_INFO_KEY, true));
    tap(&mut screen, Key::Enter);
    tap(&mut screen, Key::Escape);
    screen.input(&key(PLAYER_INFO_KEY, false));
    assert_eq!(
        screen.showing(),
        Showing::Flight,
        "no Escape reached flight"
    );
}

#[test]
fn the_player_info_in_port_closed_with_p_down_waits_for_ps_release() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    screen.input(&key(PLAYER_INFO_KEY, true));
    assert!(spaceport(&screen).player_info().is_some(), "opened");
    tap(&mut screen, Key::Enter);
    assert!(spaceport(&screen).player_info().is_some(), "waiting");
    screen.input(&key(PLAYER_INFO_KEY, false));
    assert!(spaceport(&screen).player_info().is_none(), "closed");
    assert_eq!(screen.showing(), Showing::Spaceport);
}

#[test]
fn an_exchange_key_typed_during_the_ports_wait_opens_it_after_the_release() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    screen.input(&key(PLAYER_INFO_KEY, true));
    tap(&mut screen, Key::Enter);
    tap(&mut screen, Key::Char('t'));
    assert!(spaceport(&screen).open_trade().is_none(), "not yet");
    screen.input(&key(PLAYER_INFO_KEY, false));
    assert!(spaceport(&screen).open_trade().is_some(), "opened");
}

#[test]
fn an_offer_over_flight_answered_with_i_down_stays_until_is_release() {
    let mut screen = offered_over_flight();
    screen.input(&key(MISSION_INFO_KEY, true));
    assert_eq!(screen.showing(), Showing::Offer, "no mission listed");
    tap(&mut screen, Key::Enter);
    assert!(
        screen.offer().is_some_and(OfferDialog::answered),
        "answered"
    );
    assert_eq!(screen.showing(), Showing::Offer, "waiting");
    let alpha = flight(&screen).alpha();
    screen.tick(TICK);
    assert_eq!(
        flight(&screen).alpha().to_bits(),
        alpha.to_bits(),
        "flight is paused"
    );
    screen.input(&key(MISSION_INFO_KEY, false));
    assert_eq!(screen.showing(), Showing::Flight, "resumed on the release");
}

#[test]
fn an_offer_over_flight_answered_with_i_up_resumes_flight_at_once() {
    let mut screen = offered_over_flight();
    tap(&mut screen, Key::Enter);
    assert_eq!(screen.showing(), Showing::Flight);
}

#[test]
fn the_spaceports_offer_answered_with_i_down_holds_the_port_until_is_release() {
    let mut screen = offered_docked();
    screen.input(&key(MISSION_INFO_KEY, true));
    tap(&mut screen, Key::Enter);
    assert!(spaceport(&screen).offer_dialog().is_none(), "answered");
    tap(&mut screen, Key::Char('t'));
    screen.tick(TICK);
    assert!(
        spaceport(&screen).open_trade().is_none(),
        "the port took nothing"
    );
    screen.input(&key(MISSION_INFO_KEY, false));
    screen.tick(TICK);
    assert!(
        spaceport(&screen).open_trade().is_some(),
        "the queued T opened it"
    );
}

#[test]
fn a_key_typed_during_a_player_info_wait_over_an_offer_reaches_the_offer() {
    let mut screen = offered_over_flight();
    screen.input(&key(PLAYER_INFO_KEY, true));
    assert_eq!(screen.showing(), Showing::PlayerInfo, "over the offer");
    tap(&mut screen, Key::Escape);
    tap(&mut screen, Key::Enter);
    assert_eq!(screen.showing(), Showing::PlayerInfo, "waiting");
    screen.input(&key(PLAYER_INFO_KEY, false));
    assert_ne!(screen.showing(), Showing::Offer, "the Return answered it");
}
