//! Flight's cursor under the dialogs over it: each opener hides it with a
//! zero fade, as the original calls `_HideCursorSprite` before each
//! (see `AppScreen::hide_flight_cursor` for the addresses), so the pointer's
//! resync at its unchanged point after the close doesn't show it again.

use super::encounters::{board, boardable, boarding_data, hailable};
use super::jump_white::flying_on_a_mission;
use super::planet_comm::planet_data;
use super::*;
use nova_view::ui::mission_info::MISSION_INFO_KEY;
use nova_view::ui::player_info::PLAYER_INFO_KEY;

const AT: Point = Point::new(300.0, 300.0);
const OTHER: Point = Point::new(320.0, 310.0);

fn tap(screen: &mut AppScreen, k: Key) {
    screen.input(&key(k, true));
    screen.input(&key(k, false));
}

/// Flight, ticked, with the pointer over it at [`AT`].
fn pointed(screen: &mut AppScreen) {
    screen.tick(TICK);
    screen.input(&Input::PointerMoved(AT));
    assert!(flight(screen).cursor_at().is_some(), "over flight");
}

/// Under the dialog open, then after `close` closes it, the cursor stays
/// hidden through a move at the same point, and a move elsewhere shows it.
fn hidden_until_moved(screen: &mut AppScreen, open: Showing, close: impl FnOnce(&mut AppScreen)) {
    assert_eq!(screen.showing(), open);
    assert_eq!(flight(screen).cursor_at(), None, "under {open:?}");
    close(screen);
    assert_eq!(screen.showing(), Showing::Flight);
    screen.input(&Input::PointerMoved(AT));
    screen.tick(TICK);
    assert_eq!(flight(screen).cursor_at(), None, "after {open:?}");
    screen.input(&Input::PointerMoved(OTHER));
    assert_eq!(flight(screen).cursor_at(), Some(OTHER));
}

#[test]
fn the_comm_dialog_hides_flights_cursor_until_the_pointer_moves() {
    let mut screen = hailable(with_dialogs(boarding_data()));
    fly(&mut screen);
    pointed(&mut screen);
    hail(&mut screen);
    hidden_until_moved(&mut screen, Showing::Comm, |screen| {
        tap(screen, Key::Escape);
    });
}

#[test]
fn the_planet_comm_dialog_hides_flights_cursor_until_the_pointer_moves() {
    let mut screen = with_dialogs(planet_data(0));
    fly(&mut screen);
    pointed(&mut screen);
    tap(&mut screen, Key::Char('1'));
    screen.tick(TICK);
    tap(&mut screen, Key::Char('y'));
    hidden_until_moved(&mut screen, Showing::Comm, |screen| {
        tap(screen, Key::Escape);
    });
}

#[test]
fn the_plunder_dialog_hides_flights_cursor_until_the_pointer_moves() {
    let mut screen = boardable(with_dialogs(boarding_data()));
    fly(&mut screen);
    pointed(&mut screen);
    board(&mut screen);
    hidden_until_moved(&mut screen, Showing::Plunder, |screen| {
        tap(screen, Key::Escape);
    });
}

#[test]
fn the_escort_dialog_hides_flights_cursor() {
    // `_DoEscortDialog` runs inside `_DoCommDialog` (@0x95742), after the
    // comm's `_HideCursorSprite`. Opened directly: the fixtures have no
    // escort that answers a hail.
    use nova_sim::escort_dialog::{EscortDialogView, UpgradeLine};
    let mut screen = with_dialogs(boarding_data());
    fly(&mut screen);
    pointed(&mut screen);
    screen.open_escort_dialog(&EscortDialogView {
        ship: nova_sim::ShipId(128),
        hired: false,
        comm_name: "Shuttle".to_owned(),
        subtitle: None,
        upgrade: UpgradeLine::Cannot,
        sale: None,
        pay: None,
        can_upgrade: false,
        can_sell: false,
        mark: None,
    });
    assert_eq!(flight(&screen).cursor_at(), None);
    screen.input(&Input::PointerMoved(AT));
    screen.tick(TICK);
    assert_eq!(flight(&screen).cursor_at(), None, "a same-point move");
}

#[test]
fn the_player_info_hides_flights_cursor_until_the_pointer_moves() {
    // `_HandlePlayer` @0x70343, before `_DoPlayerInfoDialog`.
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    pointed(&mut screen);
    tap(&mut screen, PLAYER_INFO_KEY);
    hidden_until_moved(&mut screen, Showing::PlayerInfo, |screen| {
        tap(screen, Key::Escape);
    });
}

#[test]
fn the_mission_info_hides_flights_cursor_until_the_pointer_moves() {
    // `_HandlePlayer` @0x704d4, before `_DoMissionInfoDialog`.
    let store = MemoryPilots::new();
    let mut screen = flying_on_a_mission(&store);
    pointed(&mut screen);
    tap(&mut screen, MISSION_INFO_KEY);
    hidden_until_moved(&mut screen, Showing::MissionInfo, |screen| {
        tap(screen, Key::Escape);
    });
}

#[test]
fn a_text_over_flight_hides_its_cursor_until_the_pointer_moves() {
    // `_HandlePlayer` @0x687a0, before `_ShowTextDialog`.
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    pointed(&mut screen);
    screen.show_over_flight(vec![DescShown {
        text: "Done.".to_owned(),
        graphic: None,
        movie: None,
    }]);
    hidden_until_moved(&mut screen, Showing::MissionText, |screen| {
        tap(screen, Key::Enter);
    });
}

#[test]
fn a_focus_loss_keeps_the_fade_so_a_same_point_move_shows_the_cursor() {
    let mut screen = hailable(with_dialogs(boarding_data()));
    fly(&mut screen);
    pointed(&mut screen);
    screen.cancel_pointer();
    assert_eq!(flight(&screen).cursor_at(), None);
    screen.input(&Input::PointerMoved(AT));
    assert_eq!(flight(&screen).cursor_at(), Some(AT));
}
