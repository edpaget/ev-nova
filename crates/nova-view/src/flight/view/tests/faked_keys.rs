//! `Screen::input`'s routing of the escort, board, mission, player and
//! map keys, and `Screen::tick`'s menu and map work, on `FakeFlight`,
//! now that the `Screen` impl is generic
//! (rdm:phase/view-sim-seam/phase-13-lifecycle-at-the-edge): the tests
//! kept real for this phase, converted under their own names. What each
//! command does is the session's, named in the commits that moved them.

use super::faked_escorts::{command, commanded, fleet_of};
use super::faked_screen::{faked_stage, picks_since, sent_count, sent_since};
use super::faked_travel::{jump_now, lit};
use super::faked_world::world;
use super::*;
use nova_sim::flight_frame::FlightEvent;
use nova_sim::{EscortCommand, EscortGroup, StellarPick};

/// A view of a flight whose fleet holds escorts of `present`'s classes,
/// the course map drawing Sol.
fn fleet(present: &[EscortClass]) -> Faked {
    let mut view = world(catalog(), fleet_of(present));
    view.sim.map_mut().drawing = Some(sol_drawing());
    view
}

/// [`fleet`] with its flight braking for a jump, or warping 300 ms into
/// it as `warping` says.
fn fleet_jumping(present: &[EscortClass], warping: bool) -> Faked {
    let mut view = fleet(present);
    let jumping = FlightFrame {
        jump_under_way: true,
        warping: false,
        ..fleet_of(present)
    };
    view.sim
        .queue_answer_showing(Answer::Jumped(SystemId(131)), jumping.clone());
    tap(&mut view, JUMP);
    if warping {
        view.sim.set_frame(FlightFrame {
            warping: true,
            ..jumping
        });
        view.refresh_frame();
        view.start_jump_effect();
        view.tick(ms(300));
        assert!(view.jump_effect().is_some(), "warping");
    }
    assert!(view.jump_under_way());
    view
}

/// Taps `k` with Option held.
fn alt_tap_faked(view: &mut Faked, k: Key) {
    view.input(&key(Key::Alt, true));
    tap(view, k);
    view.input(&key(Key::Alt, false));
}

/// Taps `k` with Command held.
fn command_tap_faked(view: &mut Faked, k: Key) {
    view.input(&key(Key::Command, true));
    tap(view, k);
    view.input(&key(Key::Command, false));
}

/// The interface's sounds `view` raised since they were last taken.
fn ui_heard(view: &mut Faked) -> Vec<UiSound> {
    view.take_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Ui(sound) => Some(sound),
            _ => None,
        })
        .collect()
}

// The escort keys.

#[test]
fn each_key_asks_the_session_for_its_command() {
    let mut view = fleet(&[EscortClass::Fighter]);
    let commands = [
        (LAND_KEY, PlayerCommand::Land),
        (BOARD_KEY, PlayerCommand::Board),
        (HAIL_KEY, PlayerCommand::Hail),
        (MAP_KEY, PlayerCommand::Map),
        (JUMP_KEY, PlayerCommand::Jump),
        (JETTISON_KEY, PlayerCommand::Jettison),
        (EJECT_KEY, PlayerCommand::Eject),
        (MISSION_INFO_KEY, PlayerCommand::MissionInfo),
        (HYPER_SELECT_KEY, PlayerCommand::SelectSystem),
        (HYPERSPACE_MODE_KEY, PlayerCommand::Nav),
        (TARGET_KEY, PlayerCommand::Target),
        (NEAREST_KEY, PlayerCommand::Target),
        (MENU_KEY, PlayerCommand::Escort),
        (MENU_CLOSE_KEY, PlayerCommand::Escort),
        (ESCORT_ATTACK_KEY, PlayerCommand::Escort),
        (ESCORT_DEFEND_KEY, PlayerCommand::Escort),
        (ESCORT_HOLD_KEY, PlayerCommand::Escort),
        (ESCORT_RECALL_KEY, PlayerCommand::Escort),
        (SELECT_KEY, PlayerCommand::SelectWeapon),
        (DESELECT_KEY, PlayerCommand::DeselectWeapon),
        (NEAREST_STELLAR_KEY, PlayerCommand::Nav),
        (NAV_OFF_KEY, PlayerCommand::Nav),
        (Key::Function(1), PlayerCommand::Nav),
        (ZOOM_OUT_KEY, PlayerCommand::Nav),
        (ZOOM_IN_KEY, PlayerCommand::Nav),
        (ZOOM_IN_KEYPAD_KEY, PlayerCommand::Nav),
    ];
    for (k, command) in commands {
        assert_eq!(view.command_of(k), Some(command), "{k:?}");
    }
    for k in [
        FIRE_KEY,
        Key::Up,
        AFTERBURNER_KEY,
        Key::Char('q'),
        Key::Other,
    ] {
        assert_eq!(view.command_of(k), None, "{k:?}");
    }
    assert!(!view.escort_menu().is_open());
    assert_eq!(view.command_of(Key::Char('1')), Some(PlayerCommand::Nav));
    assert_eq!(view.command_of(GROUP_KEYS[4]), None, "5 is no stellar slot");
    view.input(&key(Key::Command, true));
    assert_eq!(
        view.command_of(NEAREST_STELLAR_COMMAND_KEY),
        Some(PlayerCommand::Nav),
        "Command-5"
    );
    view.input(&key(Key::Command, false));
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    assert_eq!(view.command_of(Key::Char('1')), Some(PlayerCommand::Escort));
    assert_eq!(view.command_of(GROUP_KEYS[4]), Some(PlayerCommand::Escort));
    assert_eq!(view.command_of(Key::Char('q')), None);
}

#[test]
fn with_the_escort_menu_open_1_to_4_pick_groups_not_stellars() {
    let mut view = fleet(&[EscortClass::Warship]);
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    tap(&mut view, Key::Char('2'));
    view.tick(TICK);
    assert_eq!(picks_since(&view, 0), [], "the menu took it");
    tap(&mut view, Key::Function(2));
    view.tick(TICK);
    assert_eq!(
        picks_since(&view, 0),
        [FlightCommand::SelectStellar(StellarPick::Slot(1))],
        "F2 never does"
    );
    tap(&mut view, MENU_CLOSE_KEY);
    assert!(!view.escort_menu().is_open());
    let from = sent_count(&view);
    tap(&mut view, Key::Char('1'));
    view.tick(TICK);
    assert_eq!(
        picks_since(&view, from),
        [FlightCommand::SelectStellar(StellarPick::Slot(0))],
        "shut: a stellar"
    );
    assert_eq!(view.escort_menu().group(), EscortGroup::All);
}

#[test]
fn with_the_escort_menu_open_command_5_picks_freighters_not_a_stellar() {
    let mut view = fleet(&[EscortClass::Freighter]);
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    view.take_sounds();
    command_tap_faked(&mut view, NEAREST_STELLAR_COMMAND_KEY);
    view.tick(TICK);
    assert_eq!(
        view.escort_menu().group(),
        EscortGroup::Class(EscortClass::Freighter)
    );
    assert_eq!(picks_since(&view, 0), [], "no stellar");
    assert_eq!(ui_heard(&mut view), [UiSound::EscortGroupSelected]);
    tap(&mut view, MENU_CLOSE_KEY);
    command_tap_faked(&mut view, NEAREST_STELLAR_COMMAND_KEY);
    view.tick(TICK);
    tap(&mut view, NEAREST_STELLAR_KEY);
    view.tick(TICK);
    assert_eq!(
        picks_since(&view, 0),
        [
            FlightCommand::SelectStellar(StellarPick::Nearest),
            FlightCommand::SelectStellar(StellarPick::Nearest),
        ],
        "shut: the nearest, as F5's"
    );
}

#[test]
fn escort_keys_are_taken_while_the_jump_plays() {
    let mut view = fleet_jumping(&[EscortClass::Warship], true);
    view.take_sounds();
    let all = EscortGroup::All;
    let warships = EscortGroup::Class(EscortClass::Warship);
    let from = sent_count(&view);
    view.sim
        .queue_answer(commanded(all, EscortCommand::Hold, false));
    view.sim
        .queue_event(FlightEvent::Sound(SimSound::EscortsCommanded));
    tap(&mut view, ESCORT_HOLD_KEY);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships holding position.")
    );
    assert!(
        view.take_sounds()
            .contains(&Sound::Sim(SimSound::EscortsCommanded)),
        "_beepSnd[1], the flight's"
    );
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    assert_eq!(ui_heard(&mut view), [UiSound::EscortMenuToggled]);
    tap(&mut view, GROUP_KEYS[3]);
    assert_eq!(view.escort_menu().group(), warships);
    assert_eq!(ui_heard(&mut view), [UiSound::EscortGroupSelected]);
    view.sim
        .queue_answer(commanded(warships, EscortCommand::Defend, false));
    tap(&mut view, ESCORT_DEFEND_KEY);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  Warships defending.")
    );
    tap(&mut view, MENU_CLOSE_KEY);
    assert!(!view.escort_menu().is_open(), "Return closes it");
    assert_eq!(
        sent_since(&view, from),
        [
            command(all, EscortCommand::Hold),
            command(warships, EscortCommand::Defend),
        ]
    );
    assert!(view.jump_effect().is_some(), "the streak plays on");
}

#[test]
fn escort_keys_are_taken_while_the_ship_brakes_to_jump() {
    let mut view = fleet_jumping(&[EscortClass::Warship], false);
    let from = sent_count(&view);
    view.sim
        .queue_answer(commanded(EscortGroup::All, EscortCommand::Defend, false));
    tap(&mut view, ESCORT_DEFEND_KEY);
    assert_eq!(
        sent_since(&view, from),
        [command(EscortGroup::All, EscortCommand::Defend)]
    );
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships defending.")
    );
}

#[test]
fn option_c_during_the_jump_is_return_to_hangar() {
    let mut view = fleet_jumping(&[EscortClass::Fighter], true);
    let from = sent_count(&view);
    alt_tap_faked(&mut view, ESCORT_RECALL_KEY);
    tap(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(
        sent_since(&view, from),
        [
            command(EscortGroup::All, EscortCommand::Dock),
            command(EscortGroup::All, EscortCommand::Recall),
        ],
        "Option-C: Return to Hangar; C: recall"
    );
}

#[test]
fn option_released_during_the_jump_lets_c_recall() {
    let mut view = fleet_jumping(&[EscortClass::Fighter], false);
    view.input(&key(Key::Alt, true));
    lit(&mut view, 0.0);
    view.tick(ms(300));
    view.input(&key(Key::Alt, false));
    let from = sent_count(&view);
    tap(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(
        sent_since(&view, from),
        [command(EscortGroup::All, EscortCommand::Recall)],
        "C alone: formation, not Dock"
    );
}

#[test]
fn the_escort_menu_times_out_during_the_streak() {
    let mut view = fleet(&[EscortClass::Warship]);
    tap(&mut view, MENU_KEY);
    let wait = crate::flight::escorts::MENU_TIMEOUT
        .checked_sub(ms(500))
        .expect("longer than half a second");
    view.tick(wait);
    jump_now(&mut view);
    assert!(view.escort_menu().is_open(), "open as the streak begins");
    for _ in 0..30 {
        if !view.escort_menu().is_open() {
            break;
        }
        assert!(view.jump_under_way(), "shut before the ship arrives");
        view.tick(TICK);
    }
    assert!(!view.escort_menu().is_open(), "timed out");
    assert!(view.jump_under_way(), "shut within the streak");
}

// The Board key.

#[test]
fn a_tap_of_b_still_boards() {
    // Pressed and released before the next frame, through the press.
    let mut view = fleet(&[]);
    super::faked_board::opened(&mut view);
    tap(&mut view, BOARD_KEY);
    assert_eq!(sent_since(&view, 0), [FlightCommand::Board]);
    assert!(view.take_boarding().is_some());
}

// The player info and mission info keys.

/// Queues the flight's refusal of a player info press, with its beep.
fn refuses_p(view: &mut Faked) {
    view.sim.queue_answer(Answer::PlayerInfoPressed(false));
    view.sim.queue_event(FlightEvent::Sound(SimSound::Refused));
}

#[test]
fn p_during_a_jump_beeps_and_asks_nothing() {
    for warping in [false, true] {
        let mut view = fleet_jumping(&[], warping);
        let stage = faked_stage(&view);
        view.take_sounds();
        let from = sent_count(&view);
        view.input(&Input::Key {
            key: PLAYER_INFO_KEY,
            pressed: true,
            repeat: true,
        });
        assert_eq!(sent_since(&view, from), [], "{stage}: not a repeat");
        assert!(view.take_sounds().is_empty(), "{stage}: not a repeat");
        refuses_p(&mut view);
        tap(&mut view, PLAYER_INFO_KEY);
        assert_eq!(
            sent_since(&view, from),
            [FlightCommand::PressPlayerInfo],
            "{stage}: the flight refuses it"
        );
        assert!(!view.take_player_info_request(), "{stage}");
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::Refused)],
            "{stage}"
        );
    }
}

#[test]
fn p_once_the_player_has_exploded_or_ejected_does_nothing_and_says_nothing() {
    let mut view = fleet(&[]);
    view.sim.refuse(PlayerCommand::PlayerInfo);
    view.take_sounds();
    tap(&mut view, PLAYER_INFO_KEY);
    assert_eq!(sent_since(&view, 0), [], "the gate shut");
    assert!(!view.take_player_info_request());
    assert_eq!(view.take_sounds(), []);
}

#[test]
fn p_with_the_map_open_does_nothing() {
    let mut view = fleet(&[]);
    tap(&mut view, MAP_KEY);
    let from = sent_count(&view);
    tap(&mut view, PLAYER_INFO_KEY);
    assert_eq!(sent_since(&view, from), []);
    assert!(!view.take_player_info_request());
    assert!(view.map_open());
    assert!(HELP.contains("P: player info"), "{HELP}");
}

#[test]
fn i_with_the_map_open_does_nothing() {
    let mut view = fleet(&[]);
    tap(&mut view, MAP_KEY);
    assert!(view.map_open());
    let from = sent_count(&view);
    tap(&mut view, MISSION_INFO_KEY);
    assert_eq!(sent_since(&view, from), []);
    assert_eq!(view.message(), None);
    assert_eq!(view.take_mission_info_request(), None);
}

#[test]
fn p_through_the_screen_asks_once_and_a_repeat_does_not() {
    let mut view = fleet(&[]);
    assert!(!view.take_player_info_request());
    view.sim.queue_answer(Answer::PlayerInfoPressed(true));
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(view.take_player_info_request());
    view.input(&Input::Key {
        key: PLAYER_INFO_KEY,
        pressed: true,
        repeat: true,
    });
    assert_eq!(sent_since(&view, 0), [FlightCommand::PressPlayerInfo]);
    assert!(!view.take_player_info_request(), "not a repeat");
}

// A scan's notice under the streak.

#[test]
fn a_scan_in_the_warps_lead_in_is_said_under_the_streak() {
    let mut view = fleet_jumping(&[], true);
    let names = view.sim.names_mut();
    names.ship_names.push((ShipId(129), "Ship 129".to_owned()));
    names.player_ship_name = Some("Kestrel".to_owned());
    view.sim
        .queue_event(FlightEvent::Message(SimMessage::Smuggling {
            scanner: ShipId(129),
            ship_name: None,
            fine: 500,
        }));
    view.tick(TICK);
    assert_eq!(
        view.message(),
        Some(super::super::messages::smuggling_message("Ship 129", "Kestrel", 500).as_str()),
        "said under the streak"
    );
    assert!(view.jump_effect().is_some());
}

// The land key and the message line.

#[test]
fn a_repeat_or_release_of_l_does_nothing() {
    let mut view = fleet(&[]);
    view.input(&held(LAND));
    view.input(&key(LAND, false));
    view.tick(TICK);
    assert_eq!(sent_since(&view, 0), [], "no L");
    assert_eq!(view.take_landing(), None);
    assert_eq!(view.message(), None, "no refusal either");
}

#[test]
fn a_message_waits_out_the_map() {
    let mut view = fleet(&[]);
    view.say_for("A", 240);
    for _ in 0..40 {
        view.tick(TICK);
    }
    tap(&mut view, MAP_KEY);
    assert!(view.map_open());
    for _ in 0..300 {
        view.tick(TICK);
    }
    tap(&mut view, MAP_KEY);
    assert!(!view.map_open());
    assert_eq!(view.message(), Some("A"));
    for _ in 0..199 {
        view.tick(TICK);
    }
    assert_eq!(view.message(), Some("A"), "still shown after 199 ticks");
    view.tick(TICK);
    assert_eq!(view.message(), None, "gone after 200 ticks");
}
