//! The gate autopilot's component tests on [`FakeFlight`]: the view
//! presses L for it when the frame asks
//! ([`FlightView::gate_pilot_frame`]), leaving L's latch alone, enters
//! its hypergate without the map, its movie first with movies on, words
//! a refusal as L does, and starts no jump for J's
//! [`Answer::GatePilot`]. The flight decides whose press it is; the view
//! never says.

use super::faked_landing::{arrived, at_gate, at_the_hypergate, came_out_in_alpha};
use super::faked_movies::{filmed, filming, run_out};
use super::faked_travel::{now_shows, sent_but_lines};
use super::frames::{sol, under_way};
use super::*;
use nova_sim::flight_frame::{GateEntry, GatePilotShown};
use nova_sim::landing::LandOutcome;
use nova_sim::{GateKind, LandPress, LandingRefusal, SimMessage};

/// Alpha Centauri, where the autopilot flies.
const ALPHA: SystemId = SystemId(131);

/// The entry of the hypergate for Alpha Centauri.
const ENTER_ALPHA: FlightCommand = FlightCommand::EnterHypergate(Some(ALPHA));

/// `frame` with the gate autopilot flying to Alpha Centauri, its last
/// step asking for L as `press_land`.
fn piloting(frame: FlightFrame, press_land: bool) -> FlightFrame {
    FlightFrame {
        gate_pilot: Some(GatePilotShown {
            to: ALPHA,
            press_land,
        }),
        ..frame
    }
}

/// A view in Sol with the autopilot asking for L as `press_land`.
fn piloted(press_land: bool) -> Faked {
    faked(piloting(sol(), press_land))
}

/// How many times `view` sent L.
fn lands_sent(view: &Faked) -> usize {
    sent(view)
        .iter()
        .filter(|command| **command == FlightCommand::Land)
        .count()
}

/// The answer to a press that requested clearance at gate 300.
fn requested() -> Answer {
    Answer::LandPress(LandPress::Outcome(LandOutcome::Requested {
        stellar: StellarId(300),
        station: false,
    }))
}

#[test]
fn a_frame_asking_for_l_sends_land_after_the_steps_and_leaves_ls_latch_clear() {
    let mut view = piloted(true);
    view.sim.queue_answer(requested());
    view.tick(TICK);
    assert_eq!(lands_sent(&view), 1);
    assert!(!view.faded_keys.land_latched(), "no dock try (@0x6a970)");
}

#[test]
fn the_players_own_l_sets_the_latch_where_the_autopilots_does_not() {
    let mut view = piloted(false);
    view.sim.queue_answer(requested());
    view.press_land();
    assert!(view.faded_keys.land_latched());
}

#[test]
fn a_frame_not_asking_or_with_a_jump_under_way_or_with_the_map_open_sends_nothing() {
    let mut quiet = piloted(false);
    quiet.tick(TICK);
    assert_eq!(lands_sent(&quiet), 0, "not asked");
    let mut jumping = faked(piloting(under_way(false), true));
    jumping.tick(TICK);
    assert_eq!(lands_sent(&jumping), 0, "a jump under way");
    let mut opened = at_the_hypergate();
    let before = lands_sent(&opened);
    now_shows(&mut opened, piloting(sol(), true));
    opened.gate_pilot_frame();
    assert_eq!(lands_sent(&opened), before, "the map open");
}

#[test]
fn a_frame_asking_while_a_landing_waits_sends_nothing() {
    let mut view = piloted(true);
    view.landed_on(StellarId(128));
    view.gate_pilot_frame();
    assert_eq!(lands_sent(&view), 0);
}

#[test]
fn at_a_cleared_hypergate_it_enters_for_its_system_without_the_map() {
    let mut view = piloted(true);
    view.sim.queue_answer(at_gate(GateKind::Hypergate));
    came_out_in_alpha(&mut view, SimMessage::ExitedHypergate(ALPHA));
    view.sim.queue_answer(arrived());
    view.gate_pilot_frame();
    let sent = sent_but_lines(&view);
    assert_eq!(sent, [FlightCommand::Land, ENTER_ALPHA]);
    assert!(!sent.contains(&FlightCommand::OpenHypergate));
    assert!(!view.map_open());
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeIn(0.0)),
        "the come-out's white fade"
    );
}

#[test]
fn with_movies_on_it_plays_the_gates_movie_before_entering() {
    let (mut view, player) = filming(filmed(&[131], &[]), true);
    now_shows(&mut view, piloting(sol(), true));
    view.sim.queue_answer(at_gate(GateKind::Hypergate));
    view.sim
        .queue_screen(ScreenAnswer::GateEntryDesc(Some(131)));
    view.gate_pilot_frame();
    assert_eq!(
        view.sim.screens_asked().last(),
        Some(&ScreenAsk::GateEntryDesc(GateEntry::Hypergate(Some(ALPHA))))
    );
    assert!(view.playing());
    assert!(!sent(&view).contains(&ENTER_ALPHA), "held behind the movie");
    assert!(!view.map_open());
    assert_eq!(player.calls().len(), 1, "started");
    came_out_in_alpha(&mut view, SimMessage::ExitedHypergate(ALPHA));
    view.sim.queue_answer(arrived());
    run_out(&mut view);
    assert_eq!(sent_but_lines(&view).last(), Some(&ENTER_ALPHA));
    assert!(!sent(&view).contains(&FlightCommand::OpenHypergate));
}

#[test]
fn a_refusal_is_worded_as_the_players_l_words_it() {
    let refusal = || Answer::LandRefused {
        refusal: LandingRefusal::TooFar {
            stellar: StellarId(128),
            station: false,
        },
        gate: None,
        requested: true,
    };
    let mut players = faked(sol());
    players.sim.queue_answer(refusal());
    players.land();
    let mut autopilots = piloted(true);
    autopilots.sim.queue_answer(refusal());
    autopilots.gate_pilot_frame();
    assert!(players.message().is_some());
    assert_eq!(autopilots.message(), players.message());
}

#[test]
fn j_engaging_the_autopilot_starts_no_jump_and_says_nothing() {
    let mut view = faked(sol());
    view.say_for(TOO_CLOSE, 240);
    view.sim
        .queue_answer_showing(Answer::GatePilot(ALPHA), piloting(sol(), false));
    view.jump_frame(true);
    view.start_jump_effect();
    assert_eq!(view.jump_effect(), None);
    assert_eq!(view.message(), Some(TOO_CLOSE), "the line is kept");
}

#[test]
fn the_players_own_l_over_a_hypergate_still_opens_the_map() {
    let view = at_the_hypergate();
    assert_eq!(
        sent_but_lines(&view),
        [FlightCommand::Land, FlightCommand::OpenHypergate]
    );
    assert!(view.map_open());
    let mut piloted = piloted(false);
    piloted
        .sim
        .queue_screen(map_opening(&piloted.frame, Some(130)));
    piloted.sim.queue_answer(at_gate(GateKind::Hypergate));
    piloted.sim.queue_answer(Answer::Offered(vec![ALPHA]));
    piloted.land();
    assert!(
        piloted.map_open(),
        "the player's press, the autopilot shown"
    );
}
