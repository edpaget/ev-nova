//! M, L, P, Y and I across a jump's end, on [`FakeFlight`]: what they do
//! under the white, on the pass a jump collapses lit or unlit, and held
//! past an arrival (the `faded_keys` module).

use super::faked_screen::{sent_count, sent_since};
use super::faked_travel::{lit, now_shows, warping};
use super::*;
use nova_sim::flight_frame::{FadedPress, FlightEvent, PassReport, StepRun};
use nova_sim::{HailRefusal, Hailed, MissionInfoPress, VenueOffer};

/// A report of one step, carrying a lit collapse as `lit_collapse` says.
fn one_step(lit_collapse: bool) -> StepReport {
    StepReport {
        steps: vec![StepRun {
            lit_collapse,
            ..StepRun::default()
        }],
        stopped: false,
    }
}

/// A view whose flight warps under the white, the map's opening queued,
/// its sounds taken.
fn under_the_white() -> Faked {
    let mut view = warping();
    lit(&mut view, 0.5);
    queue_map(&mut view, None, sol_drawing());
    view.take_sounds();
    view
}

/// The frame under the white.
fn white() -> FlightFrame {
    FlightFrame {
        hyper_gamma: 0.5,
        ..super::frames::under_way(true)
    }
}

/// A tick whose step runs with the warp still under way.
fn warps_on(view: &mut Faked) {
    view.sim.queue_report_showing(one_step(false), white());
    view.tick(TICK);
}

/// A tick whose step collapses the jump under the white.
fn collapses_lit(view: &mut Faked) {
    view.sim
        .queue_report_showing(one_step(true), super::frames::sol());
    view.tick(TICK);
}

/// A tick whose step ends the jump with no white up.
fn collapses_unlit(view: &mut Faked) {
    view.sim
        .queue_report_showing(one_step(false), super::frames::sol());
    view.tick(TICK);
}

/// A tick whose step ends the warp, and the arrival it runs.
fn arrives(view: &mut Faked) {
    view.sim.queue_report_showing(
        one_step(false),
        FlightFrame {
            warp_end_due: true,
            ..white()
        },
    );
    queue_arrival(view);
    view.tick(TICK);
    assert!(!view.jump_under_way(), "arrived");
}

/// The flight answers the arrival, over the frame of no jump.
fn queue_arrival(view: &mut Faked) {
    view.sim.queue_answer_showing(
        Answer::Arrived {
            system: SystemId(131),
            combat: Vec::new(),
        },
        super::frames::sol(),
    );
}

/// The commands sent since `from`, the arrival left out.
fn sent(view: &Faked, from: usize) -> Vec<FlightCommand> {
    let mut sent = sent_since(view, from);
    sent.retain(|command| *command != FlightCommand::Arrive);
    sent
}

/// A prompt for a person's mission.
fn offer() -> VenueOffer {
    VenueOffer::Prompt(super::faked_hail::prompt("Help?"))
}

const FADED_P: FlightCommand = FlightCommand::PressFaded(FadedPress::PlayerInfo);
const FADED_I: FlightCommand = FlightCommand::PressFaded(FadedPress::MissionInfo);
const FADED_Y: FlightCommand = FlightCommand::PressFaded(FadedPress::Hail { option: false });
const HAIL: FlightCommand = FlightCommand::Hail { option: false };

// Under the white.

#[test]
fn under_the_warp_nothing_opens_and_p_and_y_refuse_after_the_step() {
    let mut view = under_the_white();
    let from = sent_count(&view);
    for k in [
        MAP_KEY,
        PLAYER_INFO_KEY,
        HAIL_KEY,
        MISSION_INFO_KEY,
        LAND_KEY,
    ] {
        tap(&mut view, k);
    }
    assert_eq!(sent(&view, from), [], "kept for the pass");
    view.sim.queue_answer(Answer::PlayerInfoPressed(false));
    view.sim
        .queue_answer(Answer::HailRefused(HailRefusal::NoTarget));
    warps_on(&mut view);
    assert_eq!(
        sent(&view, from),
        [FlightCommand::PressPlayerInfo, HAIL],
        "P's and Y's refusals land on the pass"
    );
    assert!(!view.map_open());
    assert!(!view.take_player_info_request());
    assert_eq!(view.take_mission_info_request(), None);
    assert_eq!(map_sounds(&mut view), [], "M and I say nothing");
    warps_on(&mut view);
    assert_eq!(
        sent(&view, from),
        [FlightCommand::PressPlayerInfo, HAIL],
        "each press once"
    );
}

#[test]
fn a_press_waits_for_a_tick_that_runs_a_step() {
    let mut view = under_the_white();
    let from = sent_count(&view);
    tap(&mut view, PLAYER_INFO_KEY);
    view.tick(TICK);
    assert_eq!(sent(&view, from), [], "a tick with no step");
    warps_on(&mut view);
    assert_eq!(sent(&view, from), [FlightCommand::PressPlayerInfo]);
}

#[test]
fn a_shut_gate_sends_no_refusal() {
    let mut view = under_the_white();
    view.change_sim(|sim| {
        sim.refuse(PlayerCommand::PlayerInfo);
        sim.refuse(PlayerCommand::Hail);
    });
    let from = sent_count(&view);
    tap(&mut view, PLAYER_INFO_KEY);
    tap(&mut view, HAIL_KEY);
    warps_on(&mut view);
    assert_eq!(sent(&view, from), []);
}

// The lit collapse.

#[test]
fn m_on_a_lit_collapse_beeps_twice_and_opens_nothing_and_loses_its_keys() {
    let mut view = under_the_white();
    view.input(&key(Key::Up, true));
    view.input(&key(JUMP_KEY, true));
    view.input(&key(MAP_KEY, true));
    let asked = view.sim.screens_asked().len();
    collapses_lit(&mut view);
    assert!(!view.map_open());
    assert!(!view.map_key_held, "no map to wait on");
    assert_eq!(
        ui_sounds(&mut view),
        [UiSound::MapOpened, UiSound::MapClosed],
        "@0x6a809, @0x6a85a"
    );
    assert_eq!(
        view.sim.screens_asked().len(),
        asked,
        "no opening asked of the flight"
    );
    assert!(view.held.contains(&Key::Up), "the unlatched key stays");
    assert!(!view.held.contains(&JUMP_KEY), "the latched goes");
    assert!(!view.held.contains(&MAP_KEY));
    assert_eq!(view.take_map_replay(), [], "nothing queued");
}

#[test]
fn m_on_a_lit_collapse_with_its_gate_shut_does_nothing() {
    let mut view = under_the_white();
    view.input(&key(Key::Up, true));
    view.input(&key(JUMP_KEY, true));
    view.input(&key(MAP_KEY, true));
    view.change_sim(|sim| sim.refuse(PlayerCommand::Map));
    view.take_sounds();
    collapses_lit(&mut view);
    assert_eq!(ui_sounds(&mut view), []);
    assert!(view.held.contains(&JUMP_KEY), "nothing let go");
}

#[test]
fn p_on_a_lit_collapse_is_refused_in_silence_and_loses_its_keys() {
    let mut view = under_the_white();
    view.input(&key(JUMP_KEY, true));
    view.input(&key(Key::Up, true));
    view.input(&key(PLAYER_INFO_KEY, true));
    let from = sent_count(&view);
    view.sim.queue_answer(Answer::PlayerInfoFaded);
    collapses_lit(&mut view);
    assert_eq!(sent(&view, from), [FADED_P]);
    assert!(!view.take_player_info_request());
    assert!(view.held.contains(&Key::Up));
    assert!(!view.held.contains(&JUMP_KEY));
    assert!(!view.held.contains(&PLAYER_INFO_KEY));
}

#[test]
fn p_refused_by_its_own_beep_on_a_lit_collapse_keeps_the_keys_and_lets_y_read() {
    let mut view = under_the_white();
    view.input(&key(JUMP_KEY, true));
    tap(&mut view, PLAYER_INFO_KEY);
    tap(&mut view, HAIL_KEY);
    let from = sent_count(&view);
    view.sim.queue_answer(Answer::PlayerInfoPressed(false));
    view.sim
        .queue_answer(Answer::HailRefused(HailRefusal::NoResponse));
    collapses_lit(&mut view);
    assert_eq!(sent(&view, from), [FADED_P, FADED_Y]);
    assert!(view.held.contains(&JUMP_KEY));
    assert_eq!(view.message(), Some(NO_RESPONSE), "the hail's refusal");
}

#[test]
fn y_on_a_lit_collapse_carries_option_and_is_refused_in_silence() {
    let mut view = under_the_white();
    view.input(&key(Key::Alt, true));
    view.input(&key(JUMP_KEY, true));
    view.input(&key(HAIL_KEY, true));
    let from = sent_count(&view);
    view.sim.queue_answer(Answer::Hailed(Hailed::Faded));
    collapses_lit(&mut view);
    assert_eq!(
        sent(&view, from),
        [FlightCommand::PressFaded(FadedPress::Hail { option: true })]
    );
    assert!(!view.hail_opened());
    assert!(view.held.contains(&Key::Alt));
    assert!(!view.held.contains(&JUMP_KEY));
}

#[test]
fn y_on_a_lit_collapse_opens_a_persons_offer_and_ends_the_pass() {
    let mut view = under_the_white();
    tap(&mut view, HAIL_KEY);
    tap(&mut view, MISSION_INFO_KEY);
    let from = sent_count(&view);
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Offer(offer())));
    collapses_lit(&mut view);
    assert_eq!(sent(&view, from), [FADED_Y]);
    assert!(view.take_offer().is_some());
}

#[test]
fn i_on_a_lit_collapse_is_refused_in_silence_and_loses_its_keys() {
    let mut view = under_the_white();
    view.input(&key(JUMP_KEY, true));
    view.input(&key(Key::Up, true));
    view.input(&key(MISSION_INFO_KEY, true));
    let from = sent_count(&view);
    view.sim
        .queue_answer(Answer::MissionInfo(MissionInfoPress::Faded));
    collapses_lit(&mut view);
    assert_eq!(sent(&view, from), [FADED_I]);
    assert_eq!(view.take_mission_info_request(), None);
    assert!(view.held.contains(&Key::Up));
    assert!(!view.held.contains(&JUMP_KEY));
}

#[test]
fn i_with_no_mission_on_a_lit_collapse_says_so_and_held_does_nothing_after() {
    let mut view = under_the_white();
    view.input(&key(MISSION_INFO_KEY, true));
    let from = sent_count(&view);
    view.sim.queue_answer_telling(
        Answer::MissionInfo(MissionInfoPress::NoActiveMissions),
        super::frames::sol(),
        vec![FlightEvent::Message(SimMessage::NoActiveMissions)],
    );
    collapses_lit(&mut view);
    assert_eq!(sent(&view, from), [FADED_I]);
    assert_eq!(view.message(), Some(NO_ACTIVE_MISSIONS));
    view.tick(TICK);
    view.tick(TICK);
    assert_eq!(sent(&view, from), [FADED_I], "its latch set (@0x7047a)");
}

#[test]
fn l_on_a_lit_collapse_presses_the_land_key_once() {
    let mut view = under_the_white();
    view.input(&key(LAND_KEY, true));
    let from = sent_count(&view);
    collapses_lit(&mut view);
    assert_eq!(sent(&view, from), [FlightCommand::Land]);
    view.tick(TICK);
    assert_eq!(sent(&view, from), [FlightCommand::Land], "latched");
}

#[test]
fn a_lit_collapse_counts_on_any_step_of_the_tick() {
    let mut view = under_the_white();
    tap(&mut view, PLAYER_INFO_KEY);
    let from = sent_count(&view);
    view.sim.queue_answer(Answer::PlayerInfoFaded);
    let report = StepReport {
        steps: vec![
            one_step(true).steps.remove(0),
            one_step(false).steps.remove(0),
        ],
        stopped: false,
    };
    view.sim.queue_report_showing(report, super::frames::sol());
    view.tick(TICK);
    assert_eq!(sent(&view, from), [FADED_P]);
}

#[test]
fn m_on_a_lit_collapse_latches_p_y_and_i_but_not_l() {
    let mut view = under_the_white();
    for k in [
        PLAYER_INFO_KEY,
        HAIL_KEY,
        MISSION_INFO_KEY,
        LAND_KEY,
        MAP_KEY,
    ] {
        tap(&mut view, k);
    }
    let from = sent_count(&view);
    collapses_lit(&mut view);
    assert_eq!(sent(&view, from), [FlightCommand::Land]);
}

#[test]
fn p_faded_latches_y_and_i_and_y_faded_latches_i() {
    let mut view = under_the_white();
    for k in [PLAYER_INFO_KEY, HAIL_KEY, MISSION_INFO_KEY] {
        tap(&mut view, k);
    }
    let from = sent_count(&view);
    view.sim.queue_answer(Answer::PlayerInfoFaded);
    collapses_lit(&mut view);
    assert_eq!(sent(&view, from), [FADED_P]);

    let mut view = under_the_white();
    for k in [HAIL_KEY, MISSION_INFO_KEY] {
        tap(&mut view, k);
    }
    let from = sent_count(&view);
    view.sim.queue_answer(Answer::Hailed(Hailed::Faded));
    collapses_lit(&mut view);
    assert_eq!(sent(&view, from), [FADED_Y]);
}

#[test]
fn a_press_on_the_frame_after_a_lit_collapse_acts_as_a_plain_press() {
    let mut view = under_the_white();
    collapses_lit(&mut view);
    let from = sent_count(&view);
    view.sim.queue_answer(Answer::PlayerInfoPressed(true));
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(view.take_player_info_request());
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Comm(faked_hail_view())));
    tap(&mut view, HAIL_KEY);
    assert!(view.take_hail().is_some());
    view.sim
        .queue_answer(Answer::MissionInfo(MissionInfoPress::Opens(
            MissionInfo::default(),
        )));
    tap(&mut view, MISSION_INFO_KEY);
    assert!(view.take_mission_info_request().is_some());
    tap(&mut view, MAP_KEY);
    assert!(view.map_open());
    assert_eq!(
        sent(&view, from),
        [
            FlightCommand::PressPlayerInfo,
            HAIL,
            FlightCommand::PressMissionInfo,
        ]
    );
}

/// A comm dialog's contents.
fn faked_hail_view() -> nova_sim::HailView {
    super::faked_hail::hail_view("Channel open.")
}

#[test]
fn l_and_i_held_from_mid_jump_read_on_a_lit_collapse_and_nothing_after() {
    let mut view = under_the_white();
    view.input(&key(LAND_KEY, true));
    view.input(&key(MISSION_INFO_KEY, true));
    warps_on(&mut view);
    let from = sent_count(&view);
    warps_on(&mut view);
    assert_eq!(sent(&view, from), [], "they wait, latches clear");
    view.sim
        .queue_answer(Answer::MissionInfo(MissionInfoPress::Faded));
    collapses_lit(&mut view);
    assert_eq!(sent(&view, from), [FlightCommand::Land, FADED_I]);
    view.tick(TICK);
    assert_eq!(sent(&view, from), [FlightCommand::Land, FADED_I], "once");
}

#[test]
fn l_and_i_latched_by_a_press_do_not_read_on_a_jump_given_up_in_its_first_tick() {
    let mut view = faked(super::frames::sol());
    view.input(&key(LAND_KEY, true));
    view.sim
        .queue_answer(Answer::MissionInfo(MissionInfoPress::Refused));
    view.input(&key(MISSION_INFO_KEY, true));
    let from = sent_count(&view);
    view.held.insert(JUMP_KEY);
    view.sim.queue_answer_showing(
        Answer::Jumped(SystemId(131)),
        super::frames::under_way(true),
    );
    collapses_unlit(&mut view);
    assert_eq!(
        sent(&view, from),
        [FlightCommand::Jump],
        "their latches set"
    );
}

#[test]
fn a_let_go_of_input_forgets_the_kept_presses() {
    let mut view = under_the_white();
    tap(&mut view, PLAYER_INFO_KEY);
    tap(&mut view, HAIL_KEY);
    view.release_keys();
    let from = sent_count(&view);
    collapses_lit(&mut view);
    assert_eq!(sent(&view, from), []);
}

// The unlit collapse.

#[test]
fn m_on_an_unlit_collapse_opens_the_map_with_its_beep_and_stops_p() {
    let mut view = under_the_white();
    tap(&mut view, PLAYER_INFO_KEY);
    tap(&mut view, MAP_KEY);
    tap(&mut view, LAND_KEY);
    let from = sent_count(&view);
    collapses_unlit(&mut view);
    assert!(view.map_open());
    assert_eq!(ui_sounds(&mut view), [UiSound::MapOpened]);
    assert_eq!(sent(&view, from), [FlightCommand::Land], "L still reads");
}

#[test]
fn p_on_an_unlit_collapse_opens_its_dialog_and_ends_the_pass() {
    let mut view = under_the_white();
    tap(&mut view, PLAYER_INFO_KEY);
    tap(&mut view, HAIL_KEY);
    let from = sent_count(&view);
    view.sim.queue_answer(Answer::PlayerInfoPressed(true));
    collapses_unlit(&mut view);
    assert_eq!(sent(&view, from), [FlightCommand::PressPlayerInfo]);
    assert!(view.take_player_info_request());
}

#[test]
fn y_on_an_unlit_collapse_opens_its_dialog_and_ends_the_pass() {
    let mut view = under_the_white();
    tap(&mut view, HAIL_KEY);
    tap(&mut view, MISSION_INFO_KEY);
    let from = sent_count(&view);
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Comm(faked_hail_view())));
    collapses_unlit(&mut view);
    assert_eq!(sent(&view, from), [HAIL]);
    assert!(view.take_hail().is_some());
}

#[test]
fn i_on_an_unlit_collapse_opens_its_screen() {
    let mut view = under_the_white();
    tap(&mut view, MISSION_INFO_KEY);
    view.sim
        .queue_answer(Answer::MissionInfo(MissionInfoPress::Opens(
            MissionInfo::default(),
        )));
    collapses_unlit(&mut view);
    assert!(view.take_mission_info_request().is_some());
}

#[test]
fn a_refused_p_on_an_unlit_collapse_lets_y_read() {
    let mut view = under_the_white();
    tap(&mut view, PLAYER_INFO_KEY);
    tap(&mut view, HAIL_KEY);
    let from = sent_count(&view);
    view.sim.queue_answer(Answer::PlayerInfoPressed(false));
    view.sim
        .queue_answer(Answer::HailRefused(HailRefusal::NoTarget));
    collapses_unlit(&mut view);
    assert_eq!(sent(&view, from), [FlightCommand::PressPlayerInfo, HAIL]);
}

#[test]
fn a_shut_gate_stops_each_key_on_a_collapse() {
    for (k, gate) in [
        (MAP_KEY, PlayerCommand::Map),
        (PLAYER_INFO_KEY, PlayerCommand::PlayerInfo),
        (HAIL_KEY, PlayerCommand::Hail),
        (MISSION_INFO_KEY, PlayerCommand::MissionInfo),
        (LAND_KEY, PlayerCommand::Land),
    ] {
        for lit in [false, true] {
            let mut view = under_the_white();
            tap(&mut view, k);
            view.change_sim(|sim| sim.refuse(gate));
            view.take_sounds();
            let from = sent_count(&view);
            if lit {
                collapses_lit(&mut view);
            } else {
                collapses_unlit(&mut view);
            }
            assert_eq!(sent(&view, from), [], "{k:?} lit {lit}");
            assert!(!view.map_open(), "{k:?} lit {lit}");
            assert_eq!(ui_sounds(&mut view), [], "{k:?} lit {lit}");
        }
    }
}

// Over the arrival.

#[test]
fn m_kept_and_held_over_the_arrival_does_not_open_the_map() {
    let mut view = under_the_white();
    view.input(&key(MAP_KEY, true));
    arrives(&mut view);
    view.tick(TICK);
    assert!(!view.map_open(), "@0x6a89f latched it");
}

#[test]
fn p_kept_and_held_over_the_arrival_acts_on_the_next_tick() {
    let mut view = under_the_white();
    view.input(&key(PLAYER_INFO_KEY, true));
    let from = sent_count(&view);
    arrives(&mut view);
    assert_eq!(sent(&view, from), [], "the pass ended @0x6c625");
    view.sim.queue_answer(Answer::PlayerInfoPressed(true));
    view.tick(TICK);
    assert_eq!(sent(&view, from), [FlightCommand::PressPlayerInfo]);
    assert!(view.take_player_info_request());
    view.tick(TICK);
    assert_eq!(sent(&view, from), [FlightCommand::PressPlayerInfo], "once");
}

#[test]
fn y_kept_and_held_over_the_arrival_acts_on_the_next_tick() {
    let mut view = under_the_white();
    view.input(&key(HAIL_KEY, true));
    let from = sent_count(&view);
    arrives(&mut view);
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Comm(faked_hail_view())));
    view.tick(TICK);
    assert_eq!(sent(&view, from), [HAIL]);
    assert!(view.take_hail().is_some());
}

#[test]
fn p_and_y_let_go_before_the_arrivals_next_tick_do_nothing() {
    let mut view = under_the_white();
    view.input(&key(PLAYER_INFO_KEY, true));
    view.input(&key(HAIL_KEY, true));
    let from = sent_count(&view);
    arrives(&mut view);
    view.input(&key(PLAYER_INFO_KEY, false));
    view.input(&key(HAIL_KEY, false));
    view.tick(TICK);
    assert_eq!(sent(&view, from), []);
}

/// The arrival held for double time's frame: a step whose second pass
/// ends the warp, then the tick that stops before its step and arrives.
fn arrives_held(view: &mut Faked, between: impl Fn(&mut Faked)) {
    let pass = PassReport {
        double_time: true,
        ..PassReport::default()
    };
    let report = super::stepping::doubled(pass.clone(), pass, white());
    view.sim.queue_report_showing(
        report,
        FlightFrame {
            warp_end_due: true,
            ..white()
        },
    );
    view.tick(TICK);
    assert!(view.jump_under_way(), "the arrival waits for its frame");
    between(view);
    view.sim.queue_report(StepReport {
        steps: Vec::new(),
        stopped: true,
    });
    queue_arrival(view);
    view.tick(TICK);
    assert!(!view.jump_under_way(), "arrived");
}

#[test]
fn p_kept_waits_through_double_times_held_arrival() {
    let mut view = under_the_white();
    view.input(&key(PLAYER_INFO_KEY, true));
    let from = sent_count(&view);
    arrives_held(&mut view, |_| {});
    assert_eq!(sent(&view, from), [], "the wait outlasts the held tick");
    view.sim.queue_answer(Answer::PlayerInfoPressed(true));
    view.tick(TICK);
    assert_eq!(sent(&view, from), [FlightCommand::PressPlayerInfo]);
}

#[test]
fn y_let_go_during_double_times_held_arrival_does_nothing() {
    let mut view = under_the_white();
    view.input(&key(HAIL_KEY, true));
    let from = sent_count(&view);
    arrives_held(&mut view, |view| {
        view.input(&key(HAIL_KEY, false));
    });
    view.tick(TICK);
    assert_eq!(sent(&view, from), []);
}

#[test]
fn a_press_on_the_arrivals_faded_frame_opens_the_map() {
    let mut view = under_the_white();
    arrives(&mut view);
    assert!(view.screen_faded(), "the arrival's frame");
    tap(&mut view, MAP_KEY);
    assert!(view.map_open(), "the flag falls before that key is read");
}

#[test]
fn a_kept_key_that_opens_something_runs_no_step_that_tick() {
    let mut view = under_the_white();
    view.input(&key(PLAYER_INFO_KEY, true));
    arrives(&mut view);
    let stepped = view.sim.steps();
    view.sim.queue_answer(Answer::PlayerInfoPressed(true));
    view.tick(TICK);
    assert_eq!(view.sim.steps(), stepped, "the dialog comes first");
    assert!(view.take_player_info_request());
    view.tick(TICK);
    assert_eq!(view.sim.steps(), stepped + 1, "and then on");
}

#[test]
fn a_kept_y_that_opens_its_dialog_runs_no_step_that_tick() {
    let mut view = under_the_white();
    view.input(&key(HAIL_KEY, true));
    arrives(&mut view);
    let stepped = view.sim.steps();
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Comm(faked_hail_view())));
    view.tick(TICK);
    assert_eq!(view.sim.steps(), stepped);
}

#[test]
fn a_waiting_l_that_lands_runs_no_step_that_tick() {
    let mut view = faked(super::frames::sol());
    view.change_sim(|sim| sim.refuse(PlayerCommand::Land));
    view.input(&key(LAND_KEY, true));
    view.change_sim(|sim| sim.take(PlayerCommand::Land));
    view.sim
        .queue_answer(Answer::LandPress(nova_sim::LandPress::Outcome(
            nova_sim::landing::LandOutcome::Landed(StellarId(128)),
        )));
    let stepped = view.sim.steps();
    view.tick(TICK);
    assert_eq!(view.sim.steps(), stepped);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
}

#[test]
fn m_on_the_map_during_a_jump_goes_to_the_map() {
    let mut view = under_the_white();
    view.open_map();
    view.input(&key(MAP_KEY, true));
    view.input(&key(MAP_KEY, false));
    assert!(!view.map_open(), "only P stops short of the map");
}

#[test]
fn a_tick_a_kept_key_opens_nothing_on_still_steps() {
    let mut view = under_the_white();
    view.input(&key(PLAYER_INFO_KEY, true));
    arrives(&mut view);
    let stepped = view.sim.steps();
    view.sim.queue_answer(Answer::PlayerInfoPressed(false));
    view.tick(TICK);
    assert_eq!(view.sim.steps(), stepped + 1);
}

#[test]
fn a_kept_m_acts_after_a_jump_given_up_between_steps() {
    let mut view = under_the_white();
    view.input(&key(MAP_KEY, true));
    now_shows(&mut view, super::frames::sol());
    let stepped = view.sim.steps();
    view.tick(TICK);
    assert!(view.map_open(), "a plain press, the flag down");
    assert_eq!(view.sim.steps(), stepped, "the map shuts the tick");
}

// L and I held past a jump.

#[test]
fn l_and_i_held_from_mid_jump_act_on_the_first_tick_after_the_arrival_once() {
    let mut view = under_the_white();
    view.input(&key(LAND_KEY, true));
    view.input(&key(MISSION_INFO_KEY, true));
    warps_on(&mut view);
    let from = sent_count(&view);
    arrives(&mut view);
    assert_eq!(sent(&view, from), [], "@0x6a95d, and @0x6c625 for I");
    view.sim
        .queue_answer(Answer::MissionInfo(MissionInfoPress::NoActiveMissions));
    view.tick(TICK);
    let once = [FlightCommand::Land, FlightCommand::PressMissionInfo];
    assert_eq!(sent(&view, from), once);
    view.tick(TICK);
    view.tick(TICK);
    assert_eq!(sent(&view, from), once, "their latches set");
}

#[test]
fn l_and_i_let_go_before_the_arrival_do_nothing_after_it() {
    let mut view = under_the_white();
    tap(&mut view, LAND_KEY);
    tap(&mut view, MISSION_INFO_KEY);
    let from = sent_count(&view);
    arrives(&mut view);
    view.tick(TICK);
    assert_eq!(sent(&view, from), []);
}

#[test]
fn l_and_i_kept_past_the_warps_end_and_let_go_after_it_do_nothing() {
    let mut view = under_the_white();
    view.input(&key(LAND_KEY, true));
    view.input(&key(MISSION_INFO_KEY, true));
    let from = sent_count(&view);
    arrives(&mut view);
    view.input(&key(LAND_KEY, false));
    view.input(&key(MISSION_INFO_KEY, false));
    view.tick(TICK);
    assert_eq!(sent(&view, from), []);
}

#[test]
fn a_held_l_or_i_refused_by_its_gate_acts_once_the_gate_opens() {
    for (k, gate, command) in [
        (LAND_KEY, PlayerCommand::Land, FlightCommand::Land),
        (
            MISSION_INFO_KEY,
            PlayerCommand::MissionInfo,
            FlightCommand::PressMissionInfo,
        ),
    ] {
        let mut view = faked(super::frames::sol());
        view.change_sim(|sim| sim.refuse(gate));
        view.input(&key(k, true));
        let from = sent_count(&view);
        view.tick(TICK);
        assert_eq!(sent(&view, from), [], "{k:?}: the gate shut");
        view.change_sim(|sim| sim.take(gate));
        view.tick(TICK);
        assert_eq!(sent(&view, from), std::slice::from_ref(&command), "{k:?}");
        view.tick(TICK);
        assert_eq!(sent(&view, from), [command], "{k:?}: once");
    }
}

#[test]
fn a_press_of_l_or_i_outside_a_jump_acts_once_while_held() {
    for (k, command) in [
        (LAND_KEY, FlightCommand::Land),
        (MISSION_INFO_KEY, FlightCommand::PressMissionInfo),
    ] {
        let mut view = faked(super::frames::sol());
        let from = sent_count(&view);
        view.input(&key(k, true));
        view.tick(TICK);
        view.tick(TICK);
        assert_eq!(sent(&view, from), [command], "{k:?}");
    }
}

#[test]
fn a_held_l_or_i_shut_out_while_latched_acts_when_the_gate_opens() {
    // The failed gate clears the latch a press set (@0x6a95d, @0x705af).
    let mut view = faked(super::frames::sol());
    view.input(&key(LAND_KEY, true));
    view.input(&key(MISSION_INFO_KEY, true));
    let from = sent_count(&view);
    view.change_sim(|sim| {
        sim.refuse(PlayerCommand::Land);
        sim.refuse(PlayerCommand::MissionInfo);
    });
    view.tick(TICK);
    assert_eq!(sent(&view, from), [], "shut out");
    view.change_sim(|sim| {
        sim.take(PlayerCommand::Land);
        sim.take(PlayerCommand::MissionInfo);
    });
    view.tick(TICK);
    assert_eq!(
        sent(&view, from),
        [FlightCommand::Land, FlightCommand::PressMissionInfo]
    );
}
