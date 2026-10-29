//! The escort menu's, the escort keys' and the escorts' brackets, labels
//! and target keys' component tests on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-10-comm-escorts-boarding-on-port): the
//! menu opened, timed out and drawn over the frame's rows
//! ([`FrameHud::escort_rows`](nova_sim::flight_frame::FrameHud::escort_rows)),
//! the commands the keys send and what the view says of the flight's
//! answers. What a command does to the escorts is nova-sim's
//! (`session::escorts`, `session::fighters`, `session::voices`). Kept
//! apart from `tests/mod.rs`, which is large.

use super::faked_travel::{faked_lasts, sent_but_lines};
use super::faked_world::{world, world_list};
use super::*;
use crate::flight::escorts::{ABSENT_ROW, MENU_AT, MENU_HEIGHT, MENU_TIMEOUT, MENU_WIDTH};
use nova_sim::flight_frame::{FlightEvent, FrameHud};
use nova_sim::{Commanded, EscortCommand, EscortGroup, EscortOrder, SimSound, TargetPick};

/// A frame in Sol, the player alone, the escort rows of `present` shown
/// and an escort flying when there are any.
pub(super) fn fleet_of(present: &[EscortClass]) -> FlightFrame {
    let mut frame = in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]);
    for row in &mut frame.hud.escort_rows {
        row.present = present.contains(&row.class);
    }
    frame.hud.escorts_flying = !present.is_empty();
    frame
}

/// A view over a flight with a warship escort.
fn escorted() -> Faked {
    faked(fleet_of(&[EscortClass::Warship]))
}

/// Presses `k` as an escort key, saying whether it was one.
fn press(view: &mut Faked, k: Key) -> bool {
    view.escort_key(k)
}

/// Presses `k` with Option held.
fn option_press(view: &mut Faked, k: Key) {
    view.held.insert(Key::Alt);
    press(view, k);
    view.held.remove(&Key::Alt);
}

/// The interface's sounds `view` raised since they were last taken.
fn ui_heard(view: &mut Faked) -> Vec<UiSound> {
    view.drained_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Ui(sound) => Some(sound),
            _ => None,
        })
        .collect()
}

/// `dt` of flight time, then the escort menu's frame of one flight step,
/// as a frame's tick runs it.
fn menu_tick(view: &mut Faked, dt: Duration) {
    view.step_stellars(dt);
    view.update_escort_menu(1);
}

/// Runs the escort menu's frames, one flight step each, until it shuts:
/// how many it took, at most 40.
fn steps_to_shut(view: &mut Faked) -> u32 {
    for steps in 1..=40 {
        view.update_escort_menu(1);
        if !view.escort_menu().is_open() {
            return steps;
        }
    }
    panic!("still open: {:?}", view.escort_menu().level());
}

/// The answer to a command that went to `group` as `command`.
pub(super) fn commanded(group: EscortGroup, command: EscortCommand, targeted: bool) -> Answer {
    Answer::Commanded(Commanded {
        group,
        command,
        targeted,
        underway: false,
    })
}

/// The command an escort key sends `group`.
pub(super) fn command(group: EscortGroup, command: EscortCommand) -> FlightCommand {
    FlightCommand::CommandEscorts { group, command }
}

// The menu.

#[test]
fn e_opens_the_menu_over_the_frames_rows_and_shutting_it_closes_it() {
    let mut view = escorted();
    assert!(press(&mut view, MENU_KEY));
    assert!(view.escort_menu().is_open());
    view.shut_escort_menu();
    assert!(!view.escort_menu().is_open());
    view.shut_escort_menu();
    assert!(!view.escort_menu().is_open(), "shut stays shut");
    assert_eq!(sent_but_lines(&view), [], "the menu sends nothing");
}

#[test]
fn e_opens_the_menu_and_its_keys_pick_the_group_commanded() {
    let mut view = escorted();
    assert!(
        !press(&mut view, GROUP_KEYS[3]),
        "1-5 leave a shut menu alone"
    );
    assert!(!view.escort_menu().is_open());
    assert_eq!(view.escort_menu().group(), EscortGroup::All);
    press(&mut view, MENU_KEY);
    assert!(press(&mut view, GROUP_KEYS[1]), "taken while open");
    assert_eq!(view.escort_menu().group(), EscortGroup::All, "no fighters");
    press(&mut view, GROUP_KEYS[3]);
    let warships = EscortGroup::Class(EscortClass::Warship);
    assert_eq!(view.escort_menu().group(), warships);
    view.sim
        .queue_answer(commanded(warships, EscortCommand::Hold, false));
    press(&mut view, ESCORT_HOLD_KEY);
    assert_eq!(
        sent_but_lines(&view),
        [command(warships, EscortCommand::Hold)]
    );
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  Warships holding position.")
    );
    press(&mut view, MENU_CLOSE_KEY);
    assert!(!view.escort_menu().is_open(), "Return closes it");
    press(&mut view, MENU_KEY);
    press(&mut view, MENU_KEY);
    assert!(!view.escort_menu().is_open(), "E again closes it");
    assert_eq!(MENU_CLOSE_KEY, Key::Enter);
}

#[test]
fn the_escort_menus_keys_beep_as_the_original_does() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    assert_eq!(ui_heard(&mut view), [UiSound::EscortMenuToggled], "opened");
    press(&mut view, GROUP_KEYS[1]);
    assert_eq!(ui_heard(&mut view), [], "an absent class is not selected");
    press(&mut view, GROUP_KEYS[3]);
    assert_eq!(ui_heard(&mut view), [UiSound::EscortGroupSelected]);
    view.sim.queue_answer(commanded(
        EscortGroup::Class(EscortClass::Warship),
        EscortCommand::Hold,
        false,
    ));
    view.sim
        .queue_event(FlightEvent::Sound(SimSound::EscortsCommanded));
    press(&mut view, ESCORT_HOLD_KEY);
    assert_eq!(
        view.drained_sounds(),
        [Sound::Sim(SimSound::EscortsCommanded)],
        "the flight's _beepSnd[1], as it raised it"
    );
    press(&mut view, MENU_KEY);
    assert_eq!(ui_heard(&mut view), [UiSound::EscortMenuToggled], "closed");
    press(&mut view, MENU_KEY);
    ui_heard(&mut view);
    menu_tick(&mut view, MENU_TIMEOUT + TICK);
    assert_eq!(view.escort_menu().level(), Some(31), "fading");
    assert_eq!(steps_to_shut(&mut view), 31);
    assert_eq!(ui_heard(&mut view), [], "it fades out silently");
}

#[test]
fn the_menu_fades_over_32_steps_once_timed_out_then_shuts_silently() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    ui_heard(&mut view);
    menu_tick(&mut view, MENU_TIMEOUT);
    assert_eq!(view.escort_menu().level(), Some(32), "480 ticks: not yet");
    view.step_stellars(TICK);
    view.update_escort_menu(0);
    assert_eq!(view.escort_menu().level(), Some(32), "no step, no drop");
    view.update_escort_menu(2);
    assert_eq!(view.escort_menu().level(), Some(30), "a level a step");
    assert_eq!(steps_to_shut(&mut view), 30);
    assert_eq!(ui_heard(&mut view), []);
}

#[test]
fn with_e_held_through_the_timeout_the_fades_end_beeps_once() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    view.held.insert(MENU_KEY);
    ui_heard(&mut view);
    menu_tick(&mut view, MENU_TIMEOUT + TICK);
    assert_eq!(ui_heard(&mut view), [], "fading: no beep yet");
    steps_to_shut(&mut view);
    assert_eq!(
        ui_heard(&mut view),
        [UiSound::EscortMenuToggled],
        "the close beeps while E is down (@0x6e01e-0x6e04d)"
    );
    view.update_escort_menu(1);
    assert_eq!(ui_heard(&mut view), [], "once");
}

#[test]
fn return_closes_it_at_once_silent_and_beeps_with_e_held() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    menu_tick(&mut view, MENU_TIMEOUT + TICK);
    assert_eq!(view.escort_menu().level(), Some(31), "fading");
    ui_heard(&mut view);
    press(&mut view, MENU_CLOSE_KEY);
    assert!(!view.escort_menu().is_open(), "at once");
    assert_eq!(ui_heard(&mut view), [], "silent");
    press(&mut view, MENU_KEY);
    view.held.insert(MENU_KEY);
    ui_heard(&mut view);
    press(&mut view, MENU_CLOSE_KEY);
    assert!(!view.escort_menu().is_open());
    assert_eq!(ui_heard(&mut view), [UiSound::EscortMenuToggled], "E held");
    press(&mut view, MENU_CLOSE_KEY);
    assert_eq!(ui_heard(&mut view), [], "shut already: nothing to close");
}

#[test]
fn e_during_the_fade_closes_it_with_its_beep() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    menu_tick(&mut view, MENU_TIMEOUT + TICK);
    ui_heard(&mut view);
    press(&mut view, MENU_KEY);
    assert!(!view.escort_menu().is_open());
    assert_eq!(ui_heard(&mut view), [UiSound::EscortMenuToggled]);
}

#[test]
fn a_press_of_an_absent_class_keeps_the_menu_open_another_8_seconds_in_silence() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    menu_tick(&mut view, Duration::from_secs(5));
    ui_heard(&mut view);
    press(&mut view, GROUP_KEYS[1]);
    assert_eq!(ui_heard(&mut view), [], "silent");
    assert_eq!(view.escort_menu().group(), EscortGroup::All, "kept");
    menu_tick(&mut view, Duration::from_secs(7));
    assert_eq!(view.escort_menu().level(), Some(32), "12 s after it opened");
    menu_tick(&mut view, Duration::from_secs(1) + TICK);
    assert_eq!(
        view.escort_menu().level(),
        Some(31),
        "fading 8 s after the press"
    );
}

#[test]
fn the_menu_closes_8_seconds_after_its_last_command() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    menu_tick(&mut view, Duration::from_secs(6));
    view.sim
        .queue_answer(commanded(EscortGroup::All, EscortCommand::Hold, false));
    press(&mut view, ESCORT_HOLD_KEY);
    menu_tick(&mut view, Duration::from_secs(6));
    assert_eq!(
        view.escort_menu().level(),
        Some(32),
        "the command kept it in full"
    );
    menu_tick(&mut view, Duration::from_secs(3));
    assert_eq!(view.escort_menu().level(), Some(31), "fading");
}

#[test]
fn a_command_during_the_fade_shows_the_menu_in_full_again() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    menu_tick(&mut view, MENU_TIMEOUT + TICK);
    view.update_escort_menu(9);
    assert_eq!(view.escort_menu().level(), Some(22));
    view.sim
        .queue_answer(commanded(EscortGroup::All, EscortCommand::Hold, false));
    press(&mut view, ESCORT_HOLD_KEY);
    assert_eq!(view.escort_menu().level(), Some(32));
}

#[test]
fn a_command_the_flight_answers_without_a_change_keeps_the_menus_time() {
    for answer in [Answer::Refused, Answer::Done] {
        let mut view = escorted();
        press(&mut view, MENU_KEY);
        menu_tick(&mut view, Duration::from_secs(6));
        view.sim.queue_answer(answer.clone());
        press(&mut view, ESCORT_HOLD_KEY);
        menu_tick(&mut view, Duration::from_secs(2) + TICK);
        assert_eq!(
            view.escort_menu().level(),
            Some(31),
            "{answer:?}: not touched, so fading"
        );
    }
}

#[test]
fn with_no_escort_flying_the_menu_fades_at_once_on_all_ships_though_its_row_stays() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    press(&mut view, GROUP_KEYS[3]);
    menu_tick(&mut view, TICK);
    assert_eq!(view.escort_menu().level(), Some(32), "an escort flying");
    view.sim.frame_mut().hud.escorts_flying = false;
    view.refresh_frame();
    menu_tick(&mut view, TICK);
    assert!(
        view.frame.hud.escort_rows.iter().any(|row| row.present),
        "its row stays"
    );
    assert_eq!(view.escort_menu().level(), Some(31), "fading at once");
    assert_eq!(view.escort_menu().group(), EscortGroup::All);
    assert_eq!(steps_to_shut(&mut view), 31);
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    view.sim.frame_mut().hud = FrameHud::default();
    view.refresh_frame();
    menu_tick(&mut view, TICK);
    assert_eq!(
        view.escort_menu().level(),
        Some(31),
        "the fleet empty, on its first frame"
    );
}

#[test]
fn pressing_1_while_none_is_flying_shows_the_menu_in_full_again() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    view.sim.frame_mut().hud.escorts_flying = false;
    view.refresh_frame();
    menu_tick(&mut view, TICK);
    menu_tick(&mut view, TICK);
    assert_eq!(view.escort_menu().level(), Some(30));
    assert!(press(&mut view, GROUP_KEYS[0]));
    assert_eq!(view.escort_menu().level(), Some(32), "with no escorts");
    menu_tick(&mut view, TICK);
    assert_eq!(
        view.escort_menu().level(),
        Some(32),
        "the fade not restarted"
    );
}

#[test]
fn e_with_no_escorts_beeps_and_says_so_and_a_command_says_nothing() {
    let mut view = faked(fleet_of(&[]));
    press(&mut view, MENU_KEY);
    assert!(!view.escort_menu().is_open());
    assert_eq!(view.message(), Some("You don't have any escorts."));
    assert_eq!(ui_heard(&mut view), [UiSound::NoEscorts]);
    press(&mut view, ESCORT_ATTACK_KEY);
    assert_eq!(
        sent_but_lines(&view),
        [command(EscortGroup::All, EscortCommand::Attack)],
        "the flight decides"
    );
    assert_eq!(
        view.message(),
        Some(NO_ESCORTS),
        "refused: nothing said over it"
    );
    faked_lasts(&mut view, NO_ESCORTS, 250);
}

#[test]
fn opening_the_escort_menu_clears_no_escorts_only() {
    let mut view = escorted();
    view.say_for(NO_ESCORTS, ESCORT_TICKS);
    press(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    assert_eq!(view.message(), None, "#51 cleared");
    view.say_for(NO_ESCORTS, ESCORT_TICKS);
    press(&mut view, MENU_KEY);
    assert!(!view.escort_menu().is_open());
    assert_eq!(view.message(), Some(NO_ESCORTS), "closing does not clear");
    view.say_for(MISSION_FAILED, 240);
    press(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    assert_eq!(view.message(), Some(MISSION_FAILED), "kept");
}

#[test]
fn return_clears_the_message_closes_the_menu_and_hides_the_floating_map() {
    let mut view = escorted();
    view.say_for(MISSION_FAILED, 240);
    press(&mut view, MENU_KEY);
    assert_eq!(view.message(), Some(MISSION_FAILED), "kept by the menu");
    let now = crate::time::sixtieths(view.elapsed);
    view.floating.show(now);
    assert!(press(&mut view, MENU_CLOSE_KEY));
    assert!(!view.escort_menu().is_open());
    assert_eq!(view.message(), None, "acknowledged");
    assert!(!view.floating.is_up(), "hidden");
    assert_eq!(sent_but_lines(&view), [FlightCommand::CommAcknowledged]);
    let mut view = faked(fleet_of(&[]));
    view.say_for(MISSION_FAILED, 240);
    press(&mut view, MENU_CLOSE_KEY);
    assert_eq!(view.message(), None, "with the menu shut too");
}

/// A left press at `at`, through the screen's input.
pub(super) fn click(view: &mut Faked, at: Point) {
    view.input(&Input::PointerButton {
        button: crate::MouseButton::Left,
        pressed: true,
        at,
    });
}

/// Whether `view` sent a click in space.
fn clicked_in_space(view: &Faked) -> bool {
    sent(view)
        .iter()
        .any(|command| matches!(command, FlightCommand::ClickInSpace { .. }))
}

/// The top-left corner of the menu's row `n`, from 0.
pub(super) fn row_at(n: usize) -> Point {
    Point::new(MENU_AT.x + 4.0, MENU_AT.y + 30.0 + 18.0 * n as f32)
}

#[test]
fn a_click_in_the_open_escort_menu_never_reaches_space_and_shut_it_does() {
    let mut view = escorted();
    let menu = Point::new(MENU_AT.x + MENU_WIDTH / 2.0, MENU_AT.y + MENU_HEIGHT / 2.0);
    press(&mut view, MENU_KEY);
    click(&mut view, menu);
    assert!(!clicked_in_space(&view), "the menu takes it");
    press(&mut view, MENU_KEY);
    click(&mut view, menu);
    assert!(clicked_in_space(&view), "shut, it goes to space");
}

#[test]
fn a_click_on_the_menus_right_edge_reaches_space() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    click(
        &mut view,
        Point::new(MENU_AT.x + MENU_WIDTH, MENU_AT.y + 40.0),
    );
    assert!(clicked_in_space(&view), "PtInRect leaves it out");
}

#[test]
fn a_row_click_selects_its_group_silently_and_the_next_command_goes_to_it() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    ui_heard(&mut view);
    click(&mut view, row_at(3));
    let warships = EscortGroup::Class(EscortClass::Warship);
    assert_eq!(view.escort_menu().group(), warships);
    assert_eq!(ui_heard(&mut view), [], "no sound (@0x6a0fc-0x6a119)");
    view.sim
        .queue_answer(commanded(warships, EscortCommand::Hold, false));
    press(&mut view, ESCORT_HOLD_KEY);
    assert!(sent_but_lines(&view).contains(&command(warships, EscortCommand::Hold)));
    click(&mut view, row_at(1));
    assert_eq!(
        view.escort_menu().group(),
        EscortGroup::Class(EscortClass::Fighter),
        "no fighters, selected all the same"
    );
}

#[test]
fn the_menu_is_asked_for_a_click_before_the_floating_map() {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    let now = crate::time::sixtieths(view.elapsed);
    view.floating.show(now);
    let before = sent(&view).len();
    click(&mut view, row_at(3));
    assert_eq!(
        view.escort_menu().group(),
        EscortGroup::Class(EscortClass::Warship)
    );
    assert_eq!(sent(&view)[before..], [], "nothing to the map or space");
}

// The closes with no fade (`_HandlePlayer` @0x6c50e,
// `_HandlePlayerDockRequest` @0x6705a).

/// [`escorted`] with the menu open, its sounds taken.
fn menu_up() -> Faked {
    let mut view = escorted();
    press(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    ui_heard(&mut view);
    view
}

#[test]
fn a_jumps_arrival_shuts_the_menu_open_through_the_warp_silently() {
    use super::faked_travel::{at_the_warps_end, jump_now};
    let mut view = menu_up();
    jump_now(&mut view);
    assert!(view.escort_menu().is_open(), "open through the warp");
    at_the_warps_end(&mut view, Vec::new());
    view.arrival_frame();
    assert!(
        sent(&view).contains(&FlightCommand::Arrive),
        "it arrived: {:?}",
        sent(&view)
    );
    assert!(!view.escort_menu().is_open(), "shut on the arrival's frame");
    assert_eq!(ui_heard(&mut view), [], "silent");
}

#[test]
fn a_gates_exit_shuts_the_menu_silently() {
    use super::faked_landing::{arrived, at_gate, came_out_in_alpha};
    let mut view = menu_up();
    view.sim.queue_answer(at_gate(nova_sim::GateKind::Wormhole));
    came_out_in_alpha(&mut view, SimMessage::PassedWormhole(SystemId(131)));
    view.sim.queue_answer(arrived());
    view.land();
    assert!(sent(&view).contains(&FlightCommand::EnterWormhole));
    assert!(!view.escort_menu().is_open());
    assert_eq!(ui_heard(&mut view), []);
}

#[test]
fn a_take_off_shuts_the_menu_silently() {
    let mut view = menu_up();
    view.sim.queue_answer(Answer::Departed(nova_sim::Departure {
        stellar: StellarId(128),
        settled: nova_sim::Settled::default(),
    }));
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert!(!view.escort_menu().is_open());
    assert_eq!(ui_heard(&mut view), []);
}

// The escort keys.

#[test]
fn f_d_v_and_c_send_their_commands_and_say_what_each_changed() {
    let mut view = escorted();
    let all = EscortGroup::All;
    for (k, sent_as, answer, said) in [
        (
            ESCORT_ATTACK_KEY,
            EscortCommand::Attack,
            commanded(all, EscortCommand::Attack, true),
            "New escort orders assigned:  All ships attacking target.",
        ),
        (
            ESCORT_ATTACK_KEY,
            EscortCommand::Attack,
            commanded(all, EscortCommand::Attack, false),
            "New escort orders assigned:  All ships will attack.",
        ),
        (
            ESCORT_DEFEND_KEY,
            EscortCommand::Defend,
            commanded(all, EscortCommand::Defend, false),
            "New escort orders assigned:  All ships defending.",
        ),
        (
            ESCORT_HOLD_KEY,
            EscortCommand::Hold,
            commanded(all, EscortCommand::Hold, false),
            "New escort orders assigned:  All ships holding position.",
        ),
        (
            ESCORT_RECALL_KEY,
            EscortCommand::Recall,
            commanded(all, EscortCommand::Recall, false),
            "New escort orders assigned:  All ships returning to formation.",
        ),
    ] {
        view.sim.queue_answer(answer);
        let before = sent_but_lines(&view).len();
        press(&mut view, k);
        assert_eq!(
            sent_but_lines(&view)[before..],
            [command(all, sent_as)],
            "{k:?}"
        );
        assert_eq!(view.message(), Some(said), "{k:?}");
    }
    assert_eq!(
        [
            ESCORT_ATTACK_KEY,
            ESCORT_DEFEND_KEY,
            ESCORT_HOLD_KEY,
            ESCORT_RECALL_KEY
        ],
        ['f', 'd', 'v', 'c'].map(Key::Char)
    );
}

#[test]
fn option_c_sends_return_to_hangar_and_c_alone_recall_to_the_menus_group() {
    let mut view = faked(fleet_of(&[EscortClass::Fighter, EscortClass::Warship]));
    view.sim
        .queue_answer(commanded(EscortGroup::All, EscortCommand::Dock, false));
    option_press(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships returning to hangar.")
    );
    press(&mut view, ESCORT_RECALL_KEY);
    press(&mut view, MENU_KEY);
    press(&mut view, GROUP_KEYS[1]);
    let fighters = EscortGroup::Class(EscortClass::Fighter);
    view.sim
        .queue_answer(commanded(fighters, EscortCommand::Dock, false));
    option_press(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  Fighters returning to hangar.")
    );
    view.sim
        .queue_answer(commanded(fighters, EscortCommand::Recall, false));
    option_press(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  Fighters returning to formation."),
        "an escort of its own went back to formation: the answer's words"
    );
    assert_eq!(
        sent_but_lines(&view),
        [
            command(EscortGroup::All, EscortCommand::Dock),
            command(EscortGroup::All, EscortCommand::Recall),
            command(fighters, EscortCommand::Dock),
            command(fighters, EscortCommand::Dock),
        ]
    );
}

#[test]
fn a_command_the_flight_refuses_or_that_changes_no_escort_says_nothing() {
    let mut view = escorted();
    for answer in [Answer::Refused, Answer::Done] {
        view.sim.queue_answer(answer.clone());
        press(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(view.message(), None, "{answer:?}");
    }
    view.sim
        .queue_answer(commanded(EscortGroup::All, EscortCommand::Hold, false));
    press(&mut view, ESCORT_HOLD_KEY);
    faked_lasts(
        &mut view,
        "New escort orders assigned:  All ships holding position.",
        ESCORT_TICKS,
    );
}

#[test]
fn an_escort_key_the_flight_does_not_take_sends_nothing() {
    let mut view = escorted();
    view.change_sim(|sim| sim.refuse(PlayerCommand::Escort));
    view.sim
        .queue_answer(commanded(EscortGroup::All, EscortCommand::Hold, false));
    press(&mut view, ESCORT_HOLD_KEY);
    assert_eq!(sent(&view), [], "the gate refused it");
    assert_eq!(view.message(), None);
}

// The escorts' brackets, labels and target keys.

#[test]
fn the_players_escort_or_fighter_is_bracketed_green() {
    let mut frame = among(vec![super::faked_hud::npc_of_140(0)]);
    frame.target = Some(record::target(0));
    frame.hud.target_brackets = Some(nova_sim::brackets::TargetBrackets {
        npc: NpcId(0),
        expand: 0.0,
        standing: nova_sim::brackets::Standing::Escort,
    });
    let view = super::faked_hud::barred(armed(1, &[]), frame);
    assert_eq!(
        bracket_icons(&super::faked_hud::flown(&view)),
        [10016, 10017, 10018, 10019]
    );
}

/// Where the target panel's label `label` is drawn in `list`, if it is:
/// its top-left.
fn label_drawn(list: &DrawList, label: &str) -> Option<Point> {
    list.iter().find_map(|command| match command {
        DrawCommand::Text { text, origin, .. } if text == label => Some(*origin),
        _ => None,
    })
}

/// Where `view` sets a label `label` against the target panel's bottom
/// right, measured by `MonoMetrics`: 7 in from the right, on the
/// baseline 6 above the bottom.
fn label_place(view: &Faked, label: &str) -> Point {
    let bar = view.status_bar.as_ref().expect("a status bar");
    let layout = &bar.layout;
    let area = layout.targ.offset(hud::bar_origin(bar, VIEW_SIZE.0));
    let width = crate::text::TextMetrics::width(
        &crate::text::fixture::MonoMetrics,
        layout.font,
        layout.font_size,
        label,
    );
    at(
        area.max.x - 7.0 - width,
        area.max.y - 6.0 - layout.font_size,
    )
}

/// A view over a flight targeting NPC 0 of government 140, labelled as
/// `tag` says, its text measured by `MonoMetrics`.
fn labelled(tag: Option<nova_sim::TargetTag>) -> Faked {
    let target = nova_sim::flight_frame::FrameTarget {
        tag,
        ..record::target(0)
    };
    let frame = targeted(vec![super::faked_hud::npc_of_140(0)], target);
    let mut view = super::faked_hud::barred(armed(1, &[]), frame);
    view.metrics = Some(super::super::Metrics(Rc::new(
        crate::text::fixture::MonoMetrics,
    )));
    view
}

#[test]
fn the_target_panel_labels_the_frames_escort_fighter_or_code_and_draws_none_without() {
    for (tag, label) in [
        (nova_sim::TargetTag::Escort, target::ESCORT),
        (nova_sim::TargetTag::Fighter, target::FIGHTER),
        (nova_sim::TargetTag::Code(GovtId(140)), "Fed."),
    ] {
        let view = labelled(Some(tag));
        let list = super::faked_hud::flown(&view);
        assert_eq!(
            label_drawn(&list, label),
            Some(label_place(&view, label)),
            "{label}"
        );
    }
    let view = labelled(None);
    let list = super::faked_hud::flown(&view);
    let bottom = label_place(&view, "").y;
    let at_bottom: Vec<String> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, origin, .. } if origin.y == bottom => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        at_bottom
            .iter()
            .all(|text| ![target::ESCORT, target::FIGHTER, "Fed."].contains(&text.as_str())),
        "no label drawn: {at_bottom:?}"
    );
}

#[test]
fn the_target_key_walks_back_with_shift_and_through_the_escorts_with_option() {
    let mut view = escorted();
    for (shift, option, pick) in [
        (false, false, TargetPick::Next),
        (false, true, TargetPick::NextEscort),
        (true, false, TargetPick::Previous),
        (true, true, TargetPick::PreviousEscort),
    ] {
        view.held.clear();
        if shift {
            view.held.insert(Key::Shift);
        }
        if option {
            view.held.insert(Key::Alt);
        }
        let before = sent(&view).len();
        view.press_target_key();
        assert_eq!(
            sent(&view)[before..],
            [FlightCommand::SelectTarget(pick)],
            "shift {shift}, option {option}"
        );
    }
}

// The menu drawn.

/// The texts drawn in `list`, those clipped to a frame among them, as the
/// menu's are.
fn drawn_texts(list: &DrawList) -> Vec<(String, Color)> {
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Clipped { command, .. } => match &**command {
                DrawCommand::Text { text, color, .. } => Some((text.clone(), *color)),
                _ => None,
            },
            DrawCommand::Text { text, color, .. } => Some((text.clone(), *color)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_menu_shows_return_to_hangar_for_a_class_ordered_home() {
    let mut frame = fleet_of(&[EscortClass::Fighter]);
    for row in &mut frame.hud.escort_rows {
        if row.class == EscortClass::Fighter {
            row.order = Some(EscortOrder::Dock);
        }
    }
    let mut view = world(catalog(), frame);
    press(&mut view, MENU_KEY);
    let drawn = drawn_texts(&world_list(&view));
    assert!(
        drawn.iter().any(|(text, _)| text == "Return to Hangar"),
        "{drawn:?}"
    );
}

#[test]
fn the_open_menu_is_drawn_over_the_flight_under_the_hud() {
    let mut view = world(catalog(), fleet_of(&[EscortClass::Warship])).with_overlays(TITLED);
    let shut = super::faked_hud::flown(&view);
    press(&mut view, MENU_KEY);
    let open = super::faked_hud::flown(&view);
    let commands: Vec<_> = open.iter().collect();
    let title = commands
        .iter()
        .position(|command| match command {
            DrawCommand::Clipped { command, .. } => {
                matches!(&**command, DrawCommand::Text { text, .. } if text == "Escort Commands")
            }
            _ => false,
        })
        .expect("the menu");
    let frame = commands
        .iter()
        .position(|command| {
            matches!(command, DrawCommand::Line { color, .. } if *color == MENU_COLORS.border)
        })
        .expect("its border");
    let help = commands
        .iter()
        .position(|command| **command == overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM))
        .expect("the help line");
    let player = commands
        .iter()
        .position(|command| matches!(command, DrawCommand::Sprite { image, .. } if *image == ImageKey::sprite(2000, 0)))
        .expect("the player");
    assert!(
        player < frame && frame < title && title < help,
        "{player} {frame} {title} {help}"
    );
    assert_eq!(
        open.len() - shut.len(),
        4 + 1 + 5 + 1 + 1,
        "border, title, rows, bar, order: no black fill"
    );
    assert!(
        drawn_texts(&open)
            .iter()
            .any(|(text, color)| text == "2) Fighters" && *color == ABSENT_ROW)
    );
    assert!(
        !commands.iter().any(|command| matches!(
            command,
            DrawCommand::Line { color, .. } if *color == Color::BLACK
        )),
        "its black ORs in nothing"
    );
}

/// The alpha of the escort menu's title as `view` draws it.
fn title_alpha(view: &Faked) -> Option<u8> {
    let list = super::faked_hud::flown(view);
    list.iter().find_map(|command| match command {
        DrawCommand::Clipped { command, .. } => match &**command {
            DrawCommand::Text { text, color, .. } if text == "Escort Commands" => Some(color.a),
            _ => None,
        },
        _ => None,
    })
}

#[test]
fn the_menu_is_drawn_at_its_fades_level() {
    let mut view = world(catalog(), fleet_of(&[EscortClass::Warship])).with_overlays(TITLED);
    press(&mut view, MENU_KEY);
    assert_eq!(title_alpha(&view), Some(255), "in full");
    view.step_stellars(MENU_TIMEOUT + TICK);
    view.update_escort_menu(16);
    assert_eq!(view.escort_menu().level(), Some(16));
    assert_eq!(title_alpha(&view), Some(128), "at 16/32");
    view.update_escort_menu(16);
    assert_eq!(title_alpha(&view), None, "shut");
}
