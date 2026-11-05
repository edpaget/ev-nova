//! The Board key's, the boarding's and the plunder dialog's component
//! tests on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-10-comm-escorts-boarding-on-port): the
//! commands B and the dialog's presses send, the Board key's latch over
//! the flight's answers, what the view holds for the router and what it
//! says, named by the flight's names (its opening's,
//! [`FakeFlight::set_names`]). What
//! boarding does and what a press
//! takes are nova-sim's (`session::board_attempt`, `session::plunder_loop`
//! and the session's boarding tests). Kept apart from `tests/mod.rs`,
//! which is large.

use super::faked_travel::{faked_lasts, sent_but_lines};
use super::live_keys::{back, latches_its_close};
use super::*;
use nova_sim::board_repair::Recovery;
use nova_sim::flight_frame::FlightEvent;
use nova_sim::{Boarding, MissionBoardLine, PlunderView, VenueOffer};

/// A view over a flight in Sol, the player alone, the flight naming
/// commodity 0 Food ("food"), outfit 310 Rockets ("rocket pack", "rocket
/// packs"), and outfit 200 "rocket", "rockets".
fn sol() -> Faked {
    let mut names = Names::default();
    names.goods.insert(Good::Commodity(0), "Food".to_owned());
    names.lc_goods.insert(Good::Commodity(0), "food".to_owned());
    names.outfits.insert(
        OutfitId(310),
        nova_sim::flight_frame::OutfitNames {
            name: "Rockets".to_owned(),
            lc_name: "rocket pack".to_owned(),
            lc_plural: "rocket packs".to_owned(),
        },
    );
    names.outfits.insert(
        OutfitId(200),
        nova_sim::flight_frame::OutfitNames {
            name: "Rocket".to_owned(),
            lc_name: "rocket".to_owned(),
            lc_plural: "rockets".to_owned(),
        },
    );
    let mut fake = FakeFlight::new();
    fake.set_frame(in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]));
    fake.set_names(names);
    scripted_over(catalog(), fake, &Rulebook::default())
}

/// What is on board NPC 0, a ship 129: 5,750 credits, 13 tons of food and
/// 2 of outfit 310's rounds.
const ON_BOARD: PlunderView = PlunderView {
    npc: NpcId(0),
    ship: ShipId(129),
    credits: 5750,
    cargo: Some((Good::Commodity(0), 13)),
    ammo: Some(nova_sim::AmmoOffer {
        slot: nova_sim::WeaponId(140),
        rounds: 2,
        named: Some(OutfitId(310)),
    }),
    fuel: 170,
    odds: 43,
};

/// The answer to a board that opened the plunder dialog on [`ON_BOARD`],
/// the boarding then under way: the board's, and the boarding's as the
/// dialog opens ([`FlightView::take_boarding`]).
pub(super) fn opened(view: &mut Faked) {
    view.sim
        .queue_answer(Answer::Boarded(Boarding::Opened(ON_BOARD)));
    view.sim.queue_screen(ScreenAnswer::Boarding(ON_BOARD));
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
    assert_eq!(view.take_boarding(), None);
    opened(&mut view);
    view.hold_key(Key::ArrowUp);
    assert!(view.board(), "a board that opens latches");
    assert_eq!(sent_but_lines(&view), [FlightCommand::Board]);
    let shown = view.take_boarding().expect("held for the router");
    assert_eq!(shown.view, ON_BOARD);
    assert_eq!(view.take_boarding(), None, "once");
    assert_eq!(
        view.sim.screens_asked(),
        [ScreenAsk::Boarding],
        "asked as it opened"
    );
    view.sim.queue_screen(ScreenAnswer::Boarding(ON_BOARD));
    assert_eq!(view.boarding(), Some(ON_BOARD), "still under way");
    assert_eq!(view.message(), None);
    assert!(view.holds_no_key(), "the flight keys let go");
    assert_eq!(BOARD_KEY, Key::KeyB);
}

#[test]
fn b_opening_the_plunder_dialog_lets_go_of_the_mouse_button_too() {
    let mut view = super::faked_screen::in_flight();
    super::key_fades::button_down(&mut view);
    opened(&mut view);
    view.input(&key(BOARD_KEY, true));
    assert!(view.take_boarding().is_some(), "opened");
    view.let_go_of_boarding();
    super::key_fades::fades_with_no_release(&mut view);
}

#[test]
fn the_plunder_dialog_names_its_good_and_outfit_as_the_flight_names_them() {
    let mut view = sol();
    opened(&mut view);
    view.board();
    let shown = view.take_boarding().expect("boarded");
    assert_eq!(
        (shown.good.as_deref(), shown.outfit.clone()),
        (
            Some("food"),
            Some(("rocket pack".to_owned(), "rocket packs".to_owned()))
        )
    );
    view.sim.queue_screen(ScreenAnswer::Boarding(ON_BOARD));
    assert_eq!(view.plunder_shown(), Some(shown));
    view.sim.queue_screen(ScreenAnswer::Boarding(PlunderView {
        cargo: None,
        ammo: None,
        ..ON_BOARD
    }));
    let shown = view.plunder_shown().expect("under way");
    assert_eq!((shown.good, shown.outfit), (None, None), "nothing to name");
    view.sim.queue_screen(ScreenAnswer::Boarding(PlunderView {
        cargo: Some((Good::Commodity(3), 1)),
        ammo: Some(nova_sim::AmmoOffer {
            slot: nova_sim::WeaponId(140),
            rounds: 2,
            named: None,
        }),
        ..ON_BOARD
    }));
    let shown = view.plunder_shown().expect("under way");
    assert_eq!(
        (shown.good, shown.outfit),
        (None, None),
        "a good unnamed, and rounds no outfit names"
    );
    view.sim.queue_screen(ScreenAnswer::Boarding(PlunderView {
        ammo: Some(nova_sim::AmmoOffer {
            slot: nova_sim::WeaponId(140),
            rounds: 2,
            named: Some(OutfitId(999)),
        }),
        ..ON_BOARD
    }));
    let shown = view.plunder_shown().expect("under way");
    assert_eq!(shown.outfit, None, "an outfit the names don't have");
    assert_eq!(view.plunder_shown(), None, "over: nothing answered");
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
        assert!(view.board(), "{refusal:?} latches");
        assert_eq!(view.take_boarding(), None);
        faked_lasts(&mut view, said, BOARD_REFUSAL_TICKS);
    }
    assert_eq!(BOARD_REFUSAL_TICKS, 360);
    let mut repelled = sol();
    repelled
        .sim
        .queue_answer(Answer::Boarded(Boarding::Repelled));
    assert!(repelled.board());
    assert_eq!(repelled.take_boarding(), None);
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
        assert_eq!(view.board(), latches, "{refusal:?}");
        assert_eq!(view.message(), None, "{refusal:?}");
        assert_eq!(view.take_boarding(), None);
    }
    assert_eq!(board_refusal_message(BoardRefusal::NoTarget), None);
    assert_eq!(board_refusal_message(BoardRefusal::Misaligned), None);
}

#[test]
fn a_board_the_flight_refused_says_nothing_and_does_not_latch() {
    let mut view = sol();
    assert!(!view.board());
    assert_eq!(view.message(), None);
    assert_eq!(view.take_boarding(), None);
}

#[test]
fn a_recovery_latches_with_no_plunder_and_leaves_its_line_to_the_flights_messages() {
    let mut view = sol();
    view.sim.queue_answer(Answer::Boarded(Boarding::Recovered(
        Recovery::EscortRepaired,
    )));
    assert!(view.board());
    assert_eq!(view.take_boarding(), None, "no plunder");
    assert_eq!(view.message(), None, "the flight's message says it");
}

// `_HandlePlayerBoardAttempt`'s sites: the plunder (@0x659f1, @0x65f3e),
// the boarding offer (@0x65956) and the cargo pickup (@0x6535a), which
// `_SetAllKeyCmdFlags` follows whatever `_DoMissionCargoPickup` answered.

#[test]
fn the_plunder_dialogs_and_the_boarding_offers_close_latch_every_key() {
    let mut view = sol();
    opened(&mut view);
    assert!(view.board());
    latches_its_close(&mut view);

    let mut view = sol();
    view.sim
        .queue_answer(Answer::Boarded(Boarding::Offered(VenueOffer::Prompt(
            super::faked_hail::prompt("Help?"),
        ))));
    assert!(view.board());
    latches_its_close(&mut view);
}

#[test]
fn the_pickup_refused_for_room_latches_its_texts_close() {
    let mut view = sol();
    view.sim
        .queue_answer(Answer::BoardRefused(BoardRefusal::NoRoomForCargo));
    assert!(view.board(), "it latches the board");
    assert_eq!(view.message(), Some(CANT_BOARD), "#130");
    latches_its_close(&mut view);
}

#[test]
fn the_other_refusals_mark_no_site() {
    let mut view = sol();
    view.sim
        .queue_answer(Answer::BoardRefused(BoardRefusal::CantBoard));
    view.board();
    unlatched_close(&mut view);
}

#[test]
fn a_pickup_that_fits_latches_the_keys_held_at_the_next_pass() {
    let picked = MissionBoardLine::Retrieved { cargo: None };
    let mut view = sol();
    view.sim
        .queue_answer(Answer::Boarded(Boarding::MissionShip(Some(picked.clone()))));
    view.hold_key(JUMP_KEY);
    view.board();
    view.tick(TICK);
    assert!(view.latched_key(JUMP_KEY), "latched");
    // A failed mission's ship takes the same pickup, then the plunder.
    let mut view = sol();
    view.sim.queue_event(FlightEvent::BoardLine(picked));
    view.sim.queue_answer(Answer::Boarded(Boarding::Repelled));
    view.hold_key(JUMP_KEY);
    view.board();
    view.tick(TICK);
    assert!(view.latched_key(JUMP_KEY), "latched by the pickup");
}

#[test]
fn the_assist_paths_line_and_a_recovery_mark_no_site() {
    let answers = [
        Boarding::MissionShip(Some(MissionBoardLine::Boarded)),
        Boarding::MissionShip(None),
        Boarding::Recovered(Recovery::EscortRepaired),
        Boarding::Repelled,
    ];
    for answer in answers {
        let mut view = sol();
        view.sim.queue_answer(Answer::Boarded(answer));
        view.board();
        unlatched_close(&mut view);
    }
}

/// Asserts that `view`'s next close reads by the latches, no site marked:
/// J pressed under it comes back as a press.
fn unlatched_close(view: &mut Faked) {
    view.let_go_of_input();
    let replay = back(view, [JUMP_KEY], [], false);
    assert_eq!(replay, [key(JUMP_KEY, true)], "no site");
}

#[test]
fn boarding_keeps_the_message() {
    let mut view = sol();
    view.say_for(MISSION_FAILED, 240);
    opened(&mut view);
    view.board();
    view.take_boarding().expect("boarded");
    assert_eq!(view.message(), Some(MISSION_FAILED), "under the dialog");
    view.sim.queue_answer(plundered(Taken::Aborted, false));
    assert_eq!(
        view.plunder_held(Take::Abort, Duration::ZERO),
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
    view.board();
    let said = "You retrieved two rockets from this ship.";
    assert_eq!(view.message(), Some(said));
    faked_lasts(&mut view, said, GRANT_TICKS);
    let mut one = sol();
    opened(&mut one);
    one.sim.queue_event(FlightEvent::Grant(nova_sim::Granted {
        outfit: OutfitId(200),
        count: 1,
    }));
    one.board();
    assert_eq!(
        one.message(),
        Some("You retrieved a rocket from this ship.")
    );
}

#[test]
fn boarding_a_person_that_grants_nothing_says_nothing() {
    let mut view = sol();
    opened(&mut view);
    view.board();
    assert_eq!(view.message(), None);
}

#[test]
fn b_on_a_pickup_ship_says_its_line_for_250_frames_with_no_plunder() {
    let mut view = sol();
    let line = MissionBoardLine::Retrieved {
        cargo: Some("Gems".to_owned()),
    };
    view.sim
        .queue_answer(Answer::Boarded(Boarding::MissionShip(Some(line))));
    assert!(view.board());
    let said = retrieved_message("Gems");
    assert_eq!(view.message(), Some(said.as_str()));
    assert_eq!(view.take_boarding(), None, "no plunder");
    faked_lasts(&mut view, &said, BOARD_LINE_TICKS);
    let mut silent = sol();
    silent
        .sim
        .queue_answer(Answer::Boarded(Boarding::MissionShip(None)));
    silent.board();
    assert_eq!(silent.message(), None, "a ship boarded with no line");
}

#[test]
fn b_on_a_failed_missions_pickup_ship_says_its_line_first_and_opens_the_plunder() {
    let mut view = sol();
    let line = MissionBoardLine::Retrieved {
        cargo: Some("Gems".to_owned()),
    };
    view.sim.queue_event(FlightEvent::BoardLine(line));
    opened(&mut view);
    view.board();
    let said = retrieved_message("Gems");
    assert_eq!(view.message(), Some(said.as_str()));
    assert!(view.take_boarding().is_some(), "plundered");
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
    view.board();
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
    view.hold_key(Key::ArrowUp);
    view.board();
    assert_eq!(view.take_boarding(), None, "no plunder");
    assert_eq!(view.take_offer(), Some(prompt));
    assert!(view.holds_no_key(), "the flight keys let go");
    assert_eq!(view.message(), None, "nothing granted");
}

#[test]
fn b_answered_with_an_offer_leaves_no_key_held_as_the_offer_takes_its_release() {
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
    view.input(&key(BOARD_KEY, true));
    assert!(!view.holds_keys(), "the offer dialog takes B's release");
    assert_eq!(view.take_offer(), Some(prompt));
}

#[test]
fn b_refused_stays_held_until_its_release() {
    let mut view = sol();
    view.sim
        .queue_answer(Answer::BoardRefused(BoardRefusal::NoTarget));
    view.input(&key(BOARD_KEY, true));
    assert!(view.holds_keys(), "held");
    view.input(&key(BOARD_KEY, false));
    assert!(!view.holds_keys(), "released");
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
        view.board_frame(true);
    }
    assert_eq!(boards(&view), 3, "each frame");
    assert_eq!(view.take_boarding(), None, "no target");
    opened(&mut view);
    view.board_frame(true);
    assert!(view.take_boarding().is_some(), "boarded");
    view.board_frame(true);
    assert_eq!(boards(&view), 4, "latched");
}

#[test]
fn holding_b_after_a_refusal_says_it_once_until_released() {
    let mut view = sol();
    view.sim
        .queue_answer(Answer::BoardRefused(BoardRefusal::TooFar));
    for _ in 0..5 {
        view.board_frame(true);
    }
    assert_eq!(view.message(), Some(NOT_CLOSE_ENOUGH));
    assert_eq!(boards(&view), 1, "latched (@0x6504c)");
    view.board_frame(false);
    view.board_frame(true);
    assert_eq!(boards(&view), 2, "a fresh press");
    view.board_frame(false);
    view.board_frame(false);
    assert_eq!(boards(&view), 2, "nothing while up");
}

#[test]
fn holding_b_over_a_recovered_escort_boards_once_and_latches_until_released() {
    let mut view = sol();
    view.sim.queue_answer(Answer::Boarded(Boarding::Recovered(
        Recovery::EscortRepaired,
    )));
    for _ in 0..5 {
        view.board_frame(true);
    }
    assert_eq!(boards(&view), 1, "no second attempt");
    view.board_frame(false);
    view.board_frame(true);
    assert_eq!(boards(&view), 2, "released and pressed again");
}

#[test]
fn a_held_b_while_cloaked_boards_once_the_cloak_lets_it() {
    // The cloak returns before the latch is set (@0x6503f → @0x6504c).
    let mut view = sol();
    for _ in 0..2 {
        view.sim
            .queue_answer(Answer::BoardRefused(BoardRefusal::Cloaked));
        view.board_frame(true);
    }
    assert_eq!(view.take_boarding(), None, "cloaked");
    opened(&mut view);
    view.board_frame(true);
    assert!(view.take_boarding().is_some(), "the cloak dropped");
}

#[test]
fn the_board_keys_frame_while_its_gate_is_refused_sends_nothing_and_leaves_the_latch() {
    let mut view = sol();
    view.change_sim(|sim| sim.refuse(PlayerCommand::Board));
    view.board_frame(true);
    assert_eq!(sent(&view), [], "the gate refused it");
    assert!(!view.board_latch, "B down: left clear");
    view.board_latch = true;
    view.board_frame(false);
    assert!(
        view.board_latch,
        "B up: left set, the original's frame skipping the key"
    );
    view.change_sim(|sim| sim.take(PlayerCommand::Board));
    view.board_frame(false);
    assert!(!view.board_latch, "taken: cleared");
}

#[test]
fn the_held_keys_frame_runs_the_board_keys_with_b_held() {
    let mut view = sol();
    view.hold_key(BOARD_KEY);
    view.sim
        .queue_answer(Answer::BoardRefused(BoardRefusal::TooFar));
    view.held_keys_frame();
    assert_eq!(boards(&view), 1);
    assert_eq!(view.message(), Some(NOT_CLOSE_ENOUGH), "in the words given");
    view.let_go_key(BOARD_KEY);
    view.held_keys_frame();
    assert!(!view.board_latch, "let go");
}

// The plunder dialog.

#[test]
fn a_take_sends_its_press_and_says_what_it_took_counting_the_flights_line_after() {
    let mut view = sol();
    opened(&mut view);
    view.board();
    view.sim
        .queue_answer(plundered(Taken::Credits(ON_BOARD.credits), true));
    assert_eq!(
        view.plunder_held(Take::Credits, Duration::ZERO),
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
        view.plunder_held(Take::Abort, Duration::ZERO),
        Taken::Aborted
    );
    assert_eq!(
        sent_but_lines(&view).last(),
        Some(&FlightCommand::Plunder(Take::Abort)),
        "closed: nothing counted"
    );
}

#[test]
fn a_take_names_its_cargo_and_its_rounds_as_the_flight_names_them() {
    let mut view = sol();
    view.sim.queue_answer(plundered(
        Taken::Cargo {
            good: Good::Commodity(0),
            stored: 13,
        },
        true,
    ));
    view.plunder_held(Take::Cargo, Duration::ZERO);
    assert_eq!(
        view.message(),
        Some("You salvaged 13 tons of food from this ship."),
        "the good in lower case (`_DoPlunderDialog` @0x93922)"
    );
    // #115, `_StrcatOrdinalNumber`, `LCName` at 1 or less else `LCPlural`,
    // #108 (@0x93ded-0x93ec8).
    for (count, said) in [
        (1, "You salvaged one rocket pack from this ship."),
        (2, "You salvaged two rocket packs from this ship."),
        (10, "You salvaged ten rocket packs from this ship."),
        (11, "You salvaged 11 rocket packs from this ship."),
    ] {
        view.sim.queue_answer(plundered(
            Taken::Ammo {
                outfit: OutfitId(310),
                count,
            },
            true,
        ));
        view.plunder_held(Take::Ammo, Duration::ZERO);
        assert_eq!(view.message(), Some(said), "{count}");
    }
}

#[test]
fn rounds_or_a_good_the_flight_does_not_name_say_nothing() {
    // The original's Ammo press takes nothing when no outfit names the
    // rounds (@0x93d0a-0x93d2f), and every good has a name, so it has no
    // fallback words.
    for taken in [
        Taken::Ammo {
            outfit: OutfitId(999),
            count: 3,
        },
        Taken::Cargo {
            good: Good::Commodity(5),
            stored: 3,
        },
    ] {
        let mut view = sol();
        view.say_for(MISSION_FAILED, 240);
        view.sim.queue_answer(plundered(taken, true));
        view.plunder_held(Take::Ammo, Duration::ZERO);
        assert_eq!(view.message(), Some(MISSION_FAILED), "{taken:?}");
    }
}

#[test]
fn each_press_that_keeps_the_dialog_open_counts_the_message_a_tick() {
    let mut view = sol();
    view.say_for(MISSION_FAILED, 240);
    for _ in 0..2 {
        view.sim.queue_answer(plundered(Taken::Missed, true));
        assert_eq!(view.plunder_held(Take::Miss, Duration::ZERO), Taken::Missed);
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
        assert_eq!(view.plunder_held(Take::Miss, held), Taken::Missed);
        assert_eq!(view.floating.is_up(), up, "{held:?}");
    }
    let mut view = sol();
    let now = crate::time::sixtieths(view.elapsed);
    view.floating.show(now);
    view.sim.queue_answer(plundered(Taken::Aborted, false));
    assert_eq!(
        view.plunder_held(Take::Abort, Duration::from_secs(60)),
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
        view.plunder_held(Take::Cargo, Duration::ZERO),
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
    let frame = targeted(vec![super::faked_hud::npc_of_140(0)], target);
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
    view.plunder_held(Take::Credits, Duration::ZERO);
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
