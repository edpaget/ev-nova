//! `Screen::input`'s routing of the target, escort, board, mission, player and
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
use crate::flight::floating;
use crate::time::sixtieths;
use nova_sim::flight_frame::FlightEvent;
use nova_sim::navigation::NavMode;
use nova_sim::{EscortCommand, EscortGroup, StellarPick, TargetPick};

/// A view of a flight whose fleet holds escorts of `present`'s classes,
/// the course map's opening answered and its first redraw drawing Sol
/// ([`queue_map`]).
pub(super) fn fleet(present: &[EscortClass]) -> Faked {
    let mut view = world(catalog(), fleet_of(present));
    queue_map(&mut view, None, sol_drawing());
    view
}

/// [`fleet`] with its flight braking for a jump, or warping 300 ms into
/// it as `warping` says.
pub(super) fn fleet_jumping(present: &[EscortClass], warping: bool) -> Faked {
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

// Target Select.

#[test]
fn shift_tab_in_flight_selects_the_previous_ship() {
    // `_HandlePlayer` @0x69f74 reads Shift at the Tab press and walks
    // back (`_FindPrevShipInSystem`, which reads Option itself).
    let mut view = fleet(&[EscortClass::Fighter]);
    assert!(!view.jump_under_way(), "ordinary flight");
    let from = sent_count(&view);
    view.input(&key(Key::Shift, true));
    tap(&mut view, TARGET_KEY);
    view.input(&key(Key::Alt, true));
    tap(&mut view, TARGET_KEY);
    view.input(&key(Key::Alt, false));
    view.input(&key(Key::Shift, false));
    tap(&mut view, TARGET_KEY);
    assert_eq!(
        sent_since(&view, from),
        [
            FlightCommand::SelectTarget(TargetPick::Previous),
            FlightCommand::SelectTarget(TargetPick::PreviousEscort),
            FlightCommand::SelectTarget(TargetPick::Next),
        ]
    );
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
fn every_key_that_is_a_command_is_published() {
    let mut view = fleet(&[EscortClass::Fighter]);
    tap(&mut view, MENU_KEY);
    view.input(&key(Key::Command, true));
    let named = [
        Key::Left,
        Key::Right,
        Key::Up,
        Key::Down,
        Key::PageUp,
        Key::PageDown,
        Key::Home,
        Key::End,
        Key::Enter,
        Key::Escape,
        Key::Space,
        Key::Tab,
        Key::Backspace,
        Key::Alt,
        Key::Control,
        Key::Shift,
        Key::Command,
        Key::CapsLock,
    ];
    let keys = (' '..='~')
        .map(Key::Char)
        .chain((1..=24).map(Key::Function))
        .chain(named);
    for k in keys {
        if view.command_of(k).is_some() {
            assert!(every_key().any(|published| published == k), "{k:?}");
        }
    }
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
fn a_slot_keys_press_is_queued_input_until_the_frames_after_its_pick() {
    let mut view = fleet(&[EscortClass::Warship]);
    assert!(!view.input_queued(), "nothing pressed");
    tap(&mut view, Key::Char('1'));
    assert!(view.input_queued(), "pressed");
    view.tick(TICK);
    assert_eq!(
        picks_since(&view, 0),
        [FlightCommand::SelectStellar(StellarPick::Slot(0))]
    );
    assert!(view.input_queued(), "the latch to clear");
    view.tick(TICK);
    assert!(!view.input_queued(), "settled");
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

/// `view`'s escort menu past its timeout and 10 steps into its fade.
fn fading(view: &mut Faked) {
    view.step_stellars(crate::flight::escorts::MENU_TIMEOUT + TICK);
    view.update_escort_menu(10);
    assert_eq!(view.escort_menu().level(), Some(22), "fading");
}

#[test]
fn while_the_escort_menu_fades_1_to_4_pick_groups_not_stellars() {
    // `_QuickKeyCheck` hides 1-5 while the counter is above 0
    // (@0xa106-0xa124), the fade included.
    let mut view = fleet(&[EscortClass::Warship]);
    tap(&mut view, MENU_KEY);
    fading(&mut view);
    assert_eq!(
        view.slot_key_polled(Key::Char('1'), false),
        Some(false),
        "hidden from the slot block while it fades"
    );
    assert_eq!(
        view.slot_key_polled(Key::Char('1'), true),
        Some(true),
        "the menu's own loop polls it"
    );
    tap(&mut view, Key::Char('2'));
    view.tick(TICK);
    assert_eq!(picks_since(&view, 0), [], "the menu took it");
    assert_eq!(view.escort_menu().level(), Some(22), "no fighters: held");
    tap(&mut view, Key::Char('4'));
    assert_eq!(
        view.escort_menu().group(),
        EscortGroup::Class(EscortClass::Warship)
    );
    assert_eq!(view.escort_menu().level(), Some(32), "shown in full");
    view.tick(TICK);
    assert_eq!(picks_since(&view, 0), [], "no stellar");
}

#[test]
fn while_the_escort_menu_fades_command_5_picks_freighters_not_a_stellar() {
    let mut view = fleet(&[EscortClass::Freighter]);
    tap(&mut view, MENU_KEY);
    fading(&mut view);
    view.take_sounds();
    command_tap_faked(&mut view, NEAREST_STELLAR_COMMAND_KEY);
    view.tick(TICK);
    assert_eq!(
        view.escort_menu().group(),
        EscortGroup::Class(EscortClass::Freighter)
    );
    assert_eq!(picks_since(&view, 0), [], "no stellar");
    assert_eq!(ui_heard(&mut view), [UiSound::EscortGroupSelected]);
}

#[test]
fn a_row_click_is_taken_while_the_ship_brakes_and_while_the_jump_plays() {
    // The row click reads no +0x50 (@0x6a000-0x6a12b).
    use super::faked_escorts::{click, row_at};
    for warping in [false, true] {
        let mut view = fleet_jumping(&[EscortClass::Warship], warping);
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open(), "warping {warping}");
        let from = sent_count(&view);
        click(&mut view, row_at(3));
        assert_eq!(
            view.escort_menu().group(),
            EscortGroup::Class(EscortClass::Warship),
            "warping {warping}"
        );
        assert_eq!(sent_since(&view, from), [], "warping {warping}");
    }
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
    let mut faded = false;
    for _ in 0..60 {
        if !view.escort_menu().is_open() {
            break;
        }
        faded |= view.escort_menu().level() < Some(32);
        assert!(view.jump_under_way(), "shut before the ship arrives");
        view.tick(TICK);
    }
    assert!(faded, "it fades through the streak");
    assert!(!view.escort_menu().is_open(), "timed out");
    assert!(view.jump_under_way(), "shut within the streak");
}

// The Board key.

#[test]
fn a_tap_of_b_still_boards() {
    // Pressed and released before the next frame, through the press. No
    // map is opened, so none is queued before the boarding's answer.
    let mut view = world(catalog(), fleet_of(&[]));
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
    view.change_sim(|sim| sim.refuse(PlayerCommand::PlayerInfo));
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
    renamed(&mut view, |names| {
        names.ships.insert(
            ShipId(129),
            nova_sim::flight_frame::ShipNames {
                name: "Ship 129".to_owned(),
                long_name: String::new(),
            },
        );
    });
    faked_hud::changes(&mut view, |frame| {
        frame.player_ship_name = Some("Kestrel".to_owned());
    });
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

// The course map and the floating map.

#[test]
fn letting_go_of_the_keys_lets_go_of_the_map_key() {
    // The window loses focus with M still down, so its release never
    // arrives: the next key the map doesn't take leaves it open.
    let mut view = fleet(&[]);
    view.input(&key(MAP, true));
    assert!(view.map_open());
    view.release_keys();
    tap(&mut view, LAND);
    assert!(view.map_open(), "in flight");
    let mut docked = faked(FlightFrame {
        landed: Some(StellarId(128)),
        ..in_sol()
    });
    queue_map(&mut docked, None, sol_drawing());
    docked.open_bar_map();
    docked.port_map_input(&held(MAP));
    docked.release_keys();
    docked.port_map_input(&key(Key::Tab, true));
    assert!(docked.map_open(), "in port");
}

/// [`maps::floating_in`]'s view in mode 3, braking for a jump or 300 ms
/// into its warp as `warping` says, `\` held a tick so the floating map
/// shows.
fn floating_through_a_jump(warping: bool) -> Faked {
    let frame = FlightFrame {
        nav_mode: NavMode::Hyperspace(Some(SystemId(131))),
        jump_under_way: true,
        warping,
        ..in_sol()
    };
    let mut view = super::maps::floating_in(frame);
    if warping {
        view.start_jump_effect();
    }
    view.input(&key(HYPER_SELECT_KEY, true));
    view.tick(TICK);
    view.input(&key(HYPER_SELECT_KEY, false));
    assert!(view.jump_under_way());
    view
}

#[test]
fn zooms_are_taken_through_a_jump_and_refused_once_exploded() {
    for warping in [false, true] {
        let mut view = floating_through_a_jump(warping);
        view.take_sounds();
        tap(&mut view, ZOOM_OUT_KEY);
        assert!(
            view.take_sounds()
                .contains(&Sound::Ui(UiSound::FloatingMapZoomed)),
            "warping {warping}"
        );
        assert_eq!(view.floating.zoom(), 4, "warping {warping}");
    }
    let mut view = super::maps::floating_in(in_sol());
    view.change_sim(|sim| sim.refuse(PlayerCommand::Nav));
    view.floating.show(sixtieths(view.elapsed));
    view.take_sounds();
    tap(&mut view, ZOOM_OUT_KEY);
    assert_eq!(view.floating.zoom(), floating::START_ZOOM);
    assert!(
        !view
            .take_sounds()
            .contains(&Sound::Ui(UiSound::FloatingMapZoomed))
    );
}

#[test]
fn return_hides_it_through_a_jump_too() {
    let mut view = super::maps::floating_in(in_sol());
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    view.tick(TICK);
    view.input(&key(HYPERSPACE_MODE_KEY, false));
    assert_eq!(super::maps::alpha_of(&view), Some(255), "H shows it");
    tap(&mut view, MENU_CLOSE_KEY);
    assert_eq!(super::maps::alpha_of(&view), None);
    for warping in [false, true] {
        let mut view = floating_through_a_jump(warping);
        assert_eq!(super::maps::alpha_of(&view), Some(255), "warping {warping}");
        tap(&mut view, MENU_CLOSE_KEY);
        assert_eq!(super::maps::alpha_of(&view), None, "warping {warping}");
    }
}

#[test]
fn a_fresh_flights_map_opens_on_the_current_system_at_0_5625() {
    let mut view = faked(FlightFrame {
        system: SystemId(131),
        ..in_sol()
    });
    queue_map(&mut view, None, sol_drawing());
    tap(&mut view, MAP);
    assert!(view.map_open());
    assert_eq!(super::maps::place_of(view.course_map()), (0.5625, (600, 0)));
    assert_eq!(view.course_map_zoom(), Some(0.5625));
}

#[test]
fn a_zoom_given_to_the_flight_is_the_maps() {
    let view = faked(in_sol()).with_course_map_zoom(1.777_776);
    assert_eq!(view.course_map_zoom(), Some(1.777_776));
    assert_eq!(super::maps::place_of(view.course_map()).0, 1.777_776);
}

#[test]
fn the_flights_metrics_measure_the_course_maps_panel() {
    use nova_sim::map_panel::Strip;
    // The date set against the strip's right edge: 1016 − width − 5.
    let mut view = faked(in_sol()).with_metrics(Rc::new(crate::text::fixture::MonoMetrics));
    let date = "6/23/1177".to_owned();
    let mut panel = super::map_find::panel_titled("Sol");
    panel.strip = Strip::Info {
        ports: nova_sim::map_panel::Ports::Unknown,
        hazards: String::new(),
        date: date.clone(),
    };
    let opening = map_opening(&view.frame, Some(130));
    view.sim.queue_screen(opening);
    view.sim.queue_screen(redrawn_with(sol_drawing(), panel));
    tap(&mut view, MAP);
    let width =
        crate::text::fixture::MonoMetrics::advance(Font::Geneva, 9.0) * date.chars().count() as f32;
    let drawn_date = drawn(&view).iter().find_map(|command| match command {
        DrawCommand::Text {
            text, origin, size, ..
        } if *size == 9.0 && *origin == Point::new(801.0 - width, 600.0 - 9.0) => {
            Some(text.clone())
        }
        _ => None,
    });
    assert_eq!(drawn_date, Some(date));
}

#[test]
fn the_course_map_runs_its_pass_on_flights_ticks() {
    use super::map_borders::{faked_bordered, govts_on, sources};
    let mut view = faked_bordered(sources(true));
    tap(&mut view, MAP);
    view.tick(TICK);
    assert!(view.map_open());
    assert_eq!(
        view.course_map().border_passes().next(),
        nova_sim::map_borders::Next::Idle,
        "the map ticked"
    );
    assert_eq!(view.course_map().border_passes().step(), 1);
    assert_eq!(govts_on(view.course_map()), [128, 129], "the fine cells");
}

#[test]
fn the_map_opens_with_the_borders_setting_it_is_given() {
    use super::map_borders::{faked_bordered, govts_on, sources};
    use crate::galaxy::buttons::{HIDE_BORDERS_LABEL, SHOW_BORDERS_LABEL};
    let labelled = |view: &Faked, label: &str| {
        let mut list = DrawList::new();
        view.course_map().draw(&mut list);
        list.iter()
            .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == label))
    };
    let mut view = faked_bordered(sources(true)).with_map_borders(false);
    assert!(!view.map_borders_shown());
    tap(&mut view, MAP);
    assert!(!view.course_map().borders_shown());
    assert!(labelled(&view, SHOW_BORDERS_LABEL));
    view.tick(TICK);
    assert_eq!(view.course_map().border_cells(), [], "none drawn");
    tap(&mut view, Key::Char('b'));
    assert!(labelled(&view, HIDE_BORDERS_LABEL));
    assert_eq!(govts_on(view.course_map()), [128, 129]);
    tap(&mut view, MAP);
    assert_eq!(view.take_map_borders_save(), Some(true));
}

/// Types `text` into `view` as the platform sends it.
fn faked_typing(view: &mut Faked, text: &str) {
    for c in text.chars() {
        let k = if c == ' ' {
            Key::Space
        } else {
            Key::Char(c.to_ascii_lowercase())
        };
        view.input(&key(k, true));
        view.input(&Input::Text(c));
        view.input(&key(k, false));
    }
}

/// A measured view over a flight in Sol over [`map_find::named`]'s
/// catalog, its map drawing Sol, with the course map open by M.
fn faked_measured_on_the_map() -> Faked {
    let mut view = faked_in(super::map_find::named(catalog()), in_sol())
        .with_metrics(Rc::new(crate::text::fixture::MonoMetrics));
    queue_map(&mut view, Some(130), sol_drawing());
    tap(&mut view, MAP);
    view.take_sounds();
    view
}

#[test]
fn with_find_open_m_types_into_it_and_leaves_the_map_open() {
    let mut view = faked_measured_on_the_map();
    faked_typing(&mut view, "f");
    faked_typing(&mut view, "m");
    assert!(view.map_open(), "M typed");
    let typed = view
        .course_map()
        .find_dialog()
        .expect("open")
        .field()
        .text();
    assert_eq!(typed, "m");
}

#[test]
fn find_is_laid_out_from_the_template_and_style_the_flight_gives() {
    use crate::galaxy::find::CANCEL_ITEM;
    use crate::ui::dialog::ItemSpec;
    let mut template = crate::galaxy::find::fallback();
    template.items[CANCEL_ITEM - 1].kind = ItemSpec::Button("Never mind".to_owned());
    let mut view = faked_in(super::map_find::named(catalog()), in_sol())
        .with_metrics(Rc::new(crate::text::fixture::MonoMetrics))
        .with_find(Some(template), crate::ui::ButtonStyle::STOCK);
    queue_map(&mut view, None, sol_drawing());
    tap(&mut view, MAP);
    faked_typing(&mut view, "f");
    assert!(texts(&drawn(&view)).contains(&"Never mind".to_owned()));
}

// The gates come with the frame
// (rdm:phase/view-sim-seam/phase-20-view-frame-fills-per-tick).

#[test]
fn a_key_the_frames_gates_refuse_sends_nothing() {
    let refusing = FlightFrame {
        gates: nova_sim::flight_frame::Gates::EVERY.refusing(PlayerCommand::Land),
        ..in_sol()
    };
    let mut view = faked(refusing);
    tap(&mut view, LAND);
    assert_eq!(sent(&view), [], "the frame refuses the land key");
    assert_eq!(view.take_sounds(), [], "and says nothing");
    let mut view = faked(in_sol());
    tap(&mut view, LAND);
    assert_eq!(
        sent(&view),
        [FlightCommand::Land],
        "control: the gate taken, L sends the landing"
    );
}
