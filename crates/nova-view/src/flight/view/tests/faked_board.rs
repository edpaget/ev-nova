//! The Board key's, the boarding's and the plunder dialog's component
//! tests on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-10-comm-escorts-boarding-on-port): the
//! commands B and the dialog's presses send, the Board key's latch over
//! the flight's answers, what the view holds for the router and what it
//! says, worded by [`faked_words`]. What boarding does and what a press
//! takes are nova-sim's (`session::board_attempt`, `session::plunder_loop`
//! and the session's boarding tests). Kept apart from `tests/mod.rs`,
//! which is large.

use super::faked_travel::{faked_lasts, faked_words, sent_but_lines};
use super::faked_world::{in_sol_with, the_player_at};
use super::*;
use nova_sim::board_repair::Recovery;
use nova_sim::flight_frame::FlightEvent;
use nova_sim::{Boarding, MissionBoardLine, PlunderView, VenueOffer};

/// A view over a flight in Sol, the player alone.
fn sol() -> Faked {
    faked(in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]))
}

/// What is on board NPC 0, a ship 129: 5,750 credits, 13 tons of food and
/// 2 of outfit 310's rounds.
const ON_BOARD: PlunderView = PlunderView {
    npc: NpcId(0),
    ship: ShipId(129),
    credits: 5750,
    cargo: Some((Good::Commodity(0), 13)),
    ammo: Some((OutfitId(310), 2)),
    fuel: 170,
    odds: 43,
};

/// The answer to a board that opened the plunder dialog on [`ON_BOARD`],
/// the boarding then under way.
fn opened(view: &mut Faked) {
    view.sim
        .queue_answer(Answer::Boarded(Boarding::Opened(ON_BOARD)));
    view.sim.comm_mut().boarding = Some(ON_BOARD);
}

/// The answer to a take that did `taken`, the dialog open after or not.
fn plundered(taken: Taken, open: bool) -> Answer {
    Answer::Plundered { taken, open }
}

/// The Board commands `view` sent.
fn boards(view: &Faked) -> usize {
    sent(view)
        .iter()
        .filter(|command| **command == FlightCommand::Board)
        .count()
}

// The board.

#[test]
fn b_sends_the_board_and_holds_the_plunder_dialog_once_letting_go_of_the_keys() {
    let mut view = sol();
    assert_eq!(view.take_boarding_with(faked_words()), None);
    opened(&mut view);
    view.held.insert(Key::Up);
    assert!(view.board(faked_words()), "a board that opens latches");
    assert_eq!(sent_but_lines(&view), [FlightCommand::Board]);
    let shown = view
        .take_boarding_with(faked_words())
        .expect("held for the router");
    assert_eq!(shown.view, ON_BOARD);
    assert_eq!(view.take_boarding_with(faked_words()), None, "once");
    assert_eq!(view.boarding(), Some(ON_BOARD), "still under way");
    assert_eq!(view.message(), None);
    assert!(view.held.is_empty(), "the flight keys let go");
    assert_eq!(BOARD_KEY, Key::Char('b'));
}

#[test]
fn the_plunder_dialog_names_its_good_and_outfit_in_the_words_given() {
    let mut view = sol();
    opened(&mut view);
    view.board(faked_words());
    let shown = view.take_boarding_with(faked_words()).expect("boarded");
    assert_eq!(
        (shown.good.as_deref(), shown.outfit.as_deref()),
        (Some("Commodity(0)"), Some("Rockets 310"))
    );
    assert_eq!(view.plunder_shown_with(faked_words()), Some(shown));
    view.sim.comm_mut().boarding = Some(PlunderView {
        cargo: None,
        ammo: None,
        ..ON_BOARD
    });
    let shown = view.plunder_shown_with(faked_words()).expect("under way");
    assert_eq!((shown.good, shown.outfit), (None, None), "nothing to name");
    view.sim.comm_mut().boarding = None;
    assert_eq!(view.plunder_shown_with(faked_words()), None, "over");
}

#[test]
fn each_refused_board_says_why_in_the_originals_words_for_its_ticks() {
    for (refusal, said) in [
        (BoardRefusal::CantBoard, CANT_BOARD),
        (BoardRefusal::TooFar, NOT_CLOSE_ENOUGH),
        (BoardRefusal::TooFast, TOO_FAST_TO_BOARD),
    ] {
        let mut view = sol();
        view.sim.queue_answer(Answer::BoardRefused(refusal));
        assert!(view.board(faked_words()), "{refusal:?} latches");
        assert_eq!(view.take_boarding_with(faked_words()), None);
        faked_lasts(&mut view, said, BOARD_REFUSAL_TICKS);
    }
    assert_eq!(BOARD_REFUSAL_TICKS, 360);
    let mut repelled = sol();
    repelled
        .sim
        .queue_answer(Answer::Boarded(Boarding::Repelled));
    assert!(repelled.board(faked_words()));
    assert_eq!(repelled.take_boarding_with(faked_words()), None);
    faked_lasts(&mut repelled, REPELLED, MESSAGE_TICKS);
    assert_eq!(MESSAGE_TICKS, 240);
    assert_eq!(
        [CANT_BOARD, NOT_CLOSE_ENOUGH, TOO_FAST_TO_BOARD, REPELLED],
        [
            "You can't board this ship.",
            "You're not close enough to board this ship.",
            "You're moving too fast to board this ship.",
            "You were repelled while attempting to board this ship.",
        ]
    );
}

#[test]
fn a_misaligned_missing_or_cloaked_board_says_nothing_and_does_not_latch_but_misaligned() {
    for (refusal, latches) in [
        (BoardRefusal::Misaligned, true),
        (BoardRefusal::NoTarget, false),
        (BoardRefusal::Cloaked, false),
    ] {
        let mut view = sol();
        view.sim.queue_answer(Answer::BoardRefused(refusal));
        assert_eq!(view.board(faked_words()), latches, "{refusal:?}");
        assert_eq!(view.message(), None, "{refusal:?}");
        assert_eq!(view.take_boarding_with(faked_words()), None);
    }
    assert_eq!(board_refusal_message(BoardRefusal::NoTarget), None);
    assert_eq!(board_refusal_message(BoardRefusal::Misaligned), None);
}

#[test]
fn a_board_the_flight_refused_says_nothing_and_does_not_latch() {
    let mut view = sol();
    assert!(!view.board(faked_words()));
    assert_eq!(view.message(), None);
    assert_eq!(view.take_boarding_with(faked_words()), None);
}

#[test]
fn a_recovery_latches_with_no_plunder_and_leaves_its_line_to_the_flights_messages() {
    let mut view = sol();
    view.sim.queue_answer(Answer::Boarded(Boarding::Recovered(
        Recovery::EscortRepaired,
    )));
    assert!(view.board(faked_words()));
    assert_eq!(view.take_boarding_with(faked_words()), None, "no plunder");
    assert_eq!(view.message(), None, "the flight's message says it");
}

#[test]
fn boarding_keeps_the_message() {
    let mut view = sol();
    view.say_for(MISSION_FAILED, 240);
    opened(&mut view);
    view.board(faked_words());
    view.take_boarding_with(faked_words()).expect("boarded");
    assert_eq!(view.message(), Some(MISSION_FAILED), "under the dialog");
    view.sim.queue_answer(plundered(Taken::Aborted, false));
    assert_eq!(
        view.plunder_held_with(Take::Abort, Duration::ZERO, faked_words()),
        Taken::Aborted
    );
    faked_lasts(&mut view, MISSION_FAILED, 240);
}

// The grants and the mission ships' lines.

#[test]
fn boarding_a_granting_person_says_what_it_retrieved_for_240_ticks() {
    let mut view = sol();
    opened(&mut view);
    view.sim.queue_event(FlightEvent::Grant(nova_sim::Granted {
        outfit: OutfitId(200),
        count: 2,
    }));
    view.board(faked_words());
    let said = "You retrieved two rockets 200 from this ship.";
    assert_eq!(view.message(), Some(said));
    faked_lasts(&mut view, said, GRANT_TICKS);
    let mut one = sol();
    opened(&mut one);
    one.sim.queue_event(FlightEvent::Grant(nova_sim::Granted {
        outfit: OutfitId(200),
        count: 1,
    }));
    one.board(faked_words());
    assert_eq!(
        one.message(),
        Some("You retrieved a rocket from this ship.")
    );
}

#[test]
fn boarding_a_person_that_grants_nothing_says_nothing() {
    let mut view = sol();
    opened(&mut view);
    view.board(faked_words());
    assert_eq!(view.message(), None);
}

#[test]
fn b_on_a_pickup_ship_says_its_line_for_250_frames_with_no_plunder() {
    let mut view = sol();
    let line = MissionBoardLine::Retrieved { cargo: Some(0) };
    view.sim
        .queue_answer(Answer::Boarded(Boarding::MissionShip(Some(line))));
    assert!(view.board(faked_words()));
    let said = format!("{line:?}");
    assert_eq!(view.message(), Some(said.as_str()));
    assert_eq!(view.take_boarding_with(faked_words()), None, "no plunder");
    faked_lasts(&mut view, &said, BOARD_LINE_TICKS);
    let mut silent = sol();
    silent
        .sim
        .queue_answer(Answer::Boarded(Boarding::MissionShip(None)));
    silent.board(faked_words());
    assert_eq!(silent.message(), None, "a ship boarded with no line");
}

#[test]
fn b_on_a_failed_missions_pickup_ship_says_its_line_first_and_opens_the_plunder() {
    let mut view = sol();
    let line = MissionBoardLine::Retrieved { cargo: Some(0) };
    view.sim.queue_event(FlightEvent::BoardLine(line));
    opened(&mut view);
    view.board(faked_words());
    let said = format!("{line:?}");
    assert_eq!(view.message(), Some(said.as_str()));
    assert!(
        view.take_boarding_with(faked_words()).is_some(),
        "plundered"
    );
    faked_lasts(&mut view, &said, BOARD_LINE_TICKS);
}

#[test]
fn b_on_a_pickup_ship_without_room_says_cant_board_and_its_dialog_is_taken_at_once() {
    let mut view = sol();
    view.sim
        .queue_answer(Answer::BoardRefused(BoardRefusal::CantBoard));
    let no_room = nova_sim::MissionText {
        kind: MissionTextKind::NoRoom,
        mission: MissionId(140),
        text: nova_sim::mission::cargo::NO_SPACE_TO_LOAD.to_owned(),
        graphic: None,
        movie: None,
        map: None,
    };
    view.sim.queue_event(FlightEvent::MissionShown {
        shown: MissionShown::Text(no_room.clone()),
        bar: None,
    });
    view.board(faked_words());
    assert_eq!(view.message(), Some(CANT_BOARD));
    assert_eq!(view.take_mission_texts(), [MissionShown::Text(no_room)]);
}

#[test]
fn b_on_a_person_offering_on_boarding_holds_the_offer_and_no_plunder() {
    let mut view = sol();
    let prompt = nova_sim::OfferPrompt {
        mission: MissionId(130),
        text: "Aboard.".to_owned(),
        labels: nova_sim::mission::screens::offer_labels("", "", true),
        graphic: None,
        movie: None,
    };
    view.sim
        .queue_answer(Answer::Boarded(Boarding::Offered(VenueOffer::Prompt(
            prompt.clone(),
        ))));
    view.held.insert(Key::Up);
    view.board(faked_words());
    assert_eq!(view.take_boarding_with(faked_words()), None, "no plunder");
    assert_eq!(view.pending_offer.take(), Some(prompt));
    assert!(view.held.is_empty(), "the flight keys let go");
    assert_eq!(view.message(), None, "nothing granted");
}

// The Board key's latch (`*0xdee18`).

#[test]
fn holding_b_with_no_target_tries_each_frame_and_boards_once_one_is_taken() {
    // No target returns before the latch is set (@0x65031 → @0x6504c),
    // and `_HandlePlayer` tries again every frame B is down
    // (@0x6a98b-0x6a99c).
    let mut view = sol();
    for _ in 0..3 {
        view.sim
            .queue_answer(Answer::BoardRefused(BoardRefusal::NoTarget));
        view.board_frame(true, faked_words());
    }
    assert_eq!(boards(&view), 3, "each frame");
    assert_eq!(view.take_boarding_with(faked_words()), None, "no target");
    opened(&mut view);
    view.board_frame(true, faked_words());
    assert!(view.take_boarding_with(faked_words()).is_some(), "boarded");
    view.board_frame(true, faked_words());
    assert_eq!(boards(&view), 4, "latched");
}

#[test]
fn holding_b_after_a_refusal_says_it_once_until_released() {
    let mut view = sol();
    view.sim
        .queue_answer(Answer::BoardRefused(BoardRefusal::TooFar));
    for _ in 0..5 {
        view.board_frame(true, faked_words());
    }
    assert_eq!(view.message(), Some(NOT_CLOSE_ENOUGH));
    assert_eq!(boards(&view), 1, "latched (@0x6504c)");
    view.board_frame(false, faked_words());
    view.board_frame(true, faked_words());
    assert_eq!(boards(&view), 2, "a fresh press");
    view.board_frame(false, faked_words());
    view.board_frame(false, faked_words());
    assert_eq!(boards(&view), 2, "nothing while up");
}

#[test]
fn holding_b_over_a_recovered_escort_boards_once_and_latches_until_released() {
    let mut view = sol();
    view.sim.queue_answer(Answer::Boarded(Boarding::Recovered(
        Recovery::EscortRepaired,
    )));
    for _ in 0..5 {
        view.board_frame(true, faked_words());
    }
    assert_eq!(boards(&view), 1, "no second attempt");
    view.board_frame(false, faked_words());
    view.board_frame(true, faked_words());
    assert_eq!(boards(&view), 2, "released and pressed again");
}

#[test]
fn a_held_b_while_cloaked_boards_once_the_cloak_lets_it() {
    // The cloak returns before the latch is set (@0x6503f → @0x6504c).
    let mut view = sol();
    for _ in 0..2 {
        view.sim
            .queue_answer(Answer::BoardRefused(BoardRefusal::Cloaked));
        view.board_frame(true, faked_words());
    }
    assert_eq!(view.take_boarding_with(faked_words()), None, "cloaked");
    opened(&mut view);
    view.board_frame(true, faked_words());
    assert!(
        view.take_boarding_with(faked_words()).is_some(),
        "the cloak dropped"
    );
}

#[test]
fn the_board_keys_frame_while_its_gate_is_refused_sends_nothing_and_leaves_the_latch() {
    let mut view = sol();
    view.sim.refuse(PlayerCommand::Board);
    view.board_frame(true, faked_words());
    assert_eq!(sent(&view), [], "the gate refused it");
    assert!(!view.board_latch, "B down: left clear");
    view.board_latch = true;
    view.board_frame(false, faked_words());
    assert!(
        view.board_latch,
        "B up: left set, the original's frame skipping the key"
    );
    view.sim.take(PlayerCommand::Board);
    view.board_frame(false, faked_words());
    assert!(!view.board_latch, "taken: cleared");
}

#[test]
fn the_held_keys_frame_runs_the_board_keys_with_b_held() {
    let mut view = sol();
    view.held.insert(BOARD_KEY);
    view.sim
        .queue_answer(Answer::BoardRefused(BoardRefusal::TooFar));
    view.held_keys_frame(faked_words());
    assert_eq!(boards(&view), 1);
    assert_eq!(view.message(), Some(NOT_CLOSE_ENOUGH), "in the words given");
    view.held.remove(&BOARD_KEY);
    view.held_keys_frame(faked_words());
    assert!(!view.board_latch, "let go");
}

// The plunder dialog.

#[test]
fn a_take_sends_its_press_and_says_what_it_took_counting_the_flights_line_after() {
    let mut view = sol();
    opened(&mut view);
    view.board(faked_words());
    view.sim
        .queue_answer(plundered(Taken::Credits(ON_BOARD.credits), true));
    assert_eq!(
        view.plunder_held_with(Take::Credits, Duration::ZERO, faked_words()),
        Taken::Credits(ON_BOARD.credits)
    );
    assert_eq!(
        sent_but_lines(&view)[1..],
        [
            FlightCommand::Plunder(Take::Credits),
            FlightCommand::CommTicked
        ]
    );
    let line = sent(&view)
        .iter()
        .position(|command| *command == FlightCommand::CommShown(MESSAGE_TICKS));
    let ticked = sent(&view)
        .iter()
        .position(|command| *command == FlightCommand::CommTicked);
    assert!(line < ticked, "the take's line set, then counted");
    faked_lasts(&mut view, STOLE_CREDITS, 239);
    view.sim.queue_answer(plundered(Taken::Aborted, false));
    assert_eq!(
        view.plunder_held_with(Take::Abort, Duration::ZERO, faked_words()),
        Taken::Aborted
    );
    assert_eq!(
        sent_but_lines(&view).last(),
        Some(&FlightCommand::Plunder(Take::Abort)),
        "closed: nothing counted"
    );
}

#[test]
fn a_take_names_its_cargo_and_its_rounds_in_the_words_given() {
    let mut view = sol();
    view.sim.queue_answer(plundered(
        Taken::Cargo {
            good: Good::Commodity(0),
            stored: 13,
        },
        true,
    ));
    view.plunder_held_with(Take::Cargo, Duration::ZERO, faked_words());
    assert_eq!(
        view.message(),
        Some("You salvaged 13 tons of Commodity(0) from this ship.")
    );
    view.sim.queue_answer(plundered(
        Taken::Ammo {
            outfit: OutfitId(310),
            count: 2,
        },
        true,
    ));
    view.plunder_held_with(Take::Ammo, Duration::ZERO, faked_words());
    assert_eq!(
        view.message(),
        Some("You salvaged 2 Rockets 310 from this ship.")
    );
}

#[test]
fn each_press_that_keeps_the_dialog_open_counts_the_message_a_tick() {
    let mut view = sol();
    view.say_for(MISSION_FAILED, 240);
    for _ in 0..2 {
        view.sim.queue_answer(plundered(Taken::Missed, true));
        assert_eq!(
            view.plunder_held_with(Take::Miss, Duration::ZERO, faked_words()),
            Taken::Missed
        );
    }
    assert!(
        !sent(&view).contains(&FlightCommand::CommTicked),
        "a miss says no line: the flight's pass counted its own"
    );
    faked_lasts(&mut view, MISSION_FAILED, 238);
}

#[test]
fn a_press_hides_the_floating_map_once_500_ticks_have_passed() {
    for (held, up) in [
        (Duration::from_secs(8), true),
        (Duration::from_secs(9), false),
    ] {
        let mut view = sol();
        let now = crate::time::sixtieths(view.elapsed);
        view.floating.show(now);
        view.sim.queue_answer(plundered(Taken::Missed, true));
        assert_eq!(
            view.plunder_held_with(Take::Miss, held, faked_words()),
            Taken::Missed
        );
        assert_eq!(view.floating.is_up(), up, "{held:?}");
    }
    let mut view = sol();
    let now = crate::time::sixtieths(view.elapsed);
    view.floating.show(now);
    view.sim.queue_answer(plundered(Taken::Aborted, false));
    assert_eq!(
        view.plunder_held_with(Take::Abort, Duration::from_secs(60), faked_words()),
        Taken::Aborted
    );
    assert!(
        view.floating.is_up(),
        "a press that closes the dialog counts nothing"
    );
}

#[test]
fn a_take_the_flight_refused_takes_nothing_and_says_nothing() {
    let mut view = sol();
    view.say_for(MISSION_FAILED, 240);
    assert_eq!(
        view.plunder_held_with(Take::Cargo, Duration::ZERO, faked_words()),
        Taken::Nothing
    );
    assert_eq!(view.message(), Some(MISSION_FAILED));
    assert!(!sent(&view).contains(&FlightCommand::CommTicked));
    faked_lasts(&mut view, MISSION_FAILED, 240);
}

#[test]
fn the_target_panel_is_taken_afresh_once_a_press_is_made() {
    let target = nova_sim::flight_frame::FrameTarget {
        status: nova_sim::TargetStatus::Disabled,
        ..record::target(0)
    };
    let frame = super::faked_hud::targeted(vec![super::faked_hud::npc_of_140(0)], target);
    let mut view = super::faked_hud::barred(armed(1, &[]), frame);
    view.retake_target();
    if let Some((_, shown)) = &mut view.target_scene {
        *shown = None;
    }
    assert_ne!(
        *view.target_shown(&view.frame),
        view.target_now(&view.frame),
        "held"
    );
    view.sim
        .queue_answer(plundered(Taken::Credits(ON_BOARD.credits), false));
    view.plunder_held_with(Take::Credits, Duration::ZERO, faked_words());
    assert_eq!(
        *view.target_shown(&view.frame),
        view.target_now(&view.frame),
        "redrawn at once"
    );
}

#[test]
fn letting_go_of_the_boarding_sends_it_and_makes_no_sound() {
    let mut view = sol();
    view.let_go_of_boarding();
    assert_eq!(sent(&view), [FlightCommand::LetGoOfBoarding]);
    assert_eq!(view.drained_sounds(), []);
}
