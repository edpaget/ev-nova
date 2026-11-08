//! The router over the info dialogs' close and the frame gain: a
//! switch-in under the player info or the mission info over flight runs
//! `_SwitchedIn` there (`_MyModalDialog` @0x599be, @0x59a95), and
//! `_HandlePlayer` zeroes the gain once the dialog returns (@0x70357,
//! @0x704e8), so the zero wins. Wiring through the real router and
//! flight: which closes owe the zero is nova-sim's
//! (`session::time_adjustment`), and that the view tells the flight of
//! the close is nova-view's (`flight::view::tests::faked_player`).

use super::jump_white::flying_on_a_mission;
use super::*;

/// Presses and lets go of `k`.
fn tap(screen: &mut AppScreen, k: Key) {
    screen.input(&key(k, true));
    screen.input(&key(k, false));
}

/// The flight's frame gain as it stands.
fn gain(screen: &AppScreen) -> nova_sim::SpeedMult {
    flight(screen).session().expect("flying").speed_mult()
}

#[test]
fn a_switch_in_under_the_player_info_over_flight_gives_way_to_its_closes_zero() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    tap(&mut screen, PLAYER_INFO_KEY);
    assert_eq!(screen.showing(), Showing::PlayerInfo);
    screen.focus_lost();
    screen.focus_gained();
    assert_eq!(gain(&screen), nova_sim::SpeedMult::FULL, "the switch-in's");
    tap(&mut screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(gain(&screen), nova_sim::SpeedMult::ZERO, "the close's");
}

#[test]
fn a_switch_in_under_the_mission_info_over_flight_gives_way_to_its_closes_zero() {
    let store = MemoryPilots::new();
    let mut screen = flying_on_a_mission(&store);
    tap(&mut screen, MISSION_INFO_KEY);
    assert_eq!(screen.showing(), Showing::MissionInfo);
    screen.focus_lost();
    screen.focus_gained();
    assert_eq!(gain(&screen), nova_sim::SpeedMult::FULL, "the switch-in's");
    tap(&mut screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(gain(&screen), nova_sim::SpeedMult::ZERO, "the close's");
}
