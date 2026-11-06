//! Wiring tests of where a frame rolls `_HandleShip`'s hail quotes
//! (@0x33f0f-0x33fba): in the pass, at the ship loop's end (`_HandleShip`
//! @0x44483), before the reinforcement clock's tick
//! (`_HandleReinforcements` @0x44504), on both passes. Each line goes
//! through `_DisplayComm` (@0x90201), which replaces the slot and the
//! count (@0x9024b, @0x90251), so a frame with a quote and a fleet's line
//! ends on the fleet's.

use crate::SimMessage;
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::chance::{Chance, NeverFires};
use crate::combat::{Rules, ShipRef};
use crate::comm_line::CommLine;
use crate::flight::Controls;
use crate::flight_frame::{RaisedLine, StepInput, StepReport};
use crate::person::QUOTE_ON_ATTACK;
use crate::session::reinforcements::tests::{FED, TIME, fight, flying, reinforced, timer};
use crate::session::{Session, StepCollaborators};
use crate::testkit::{Draws, FakePilotCatalog, npc, person, work_with};
use crate::traffic::npc::{Npc, NpcPerson};

/// The hail quote every person says, `STR#` 7101 #1.
const QUOTE: &str = "Die!";

/// The fleet's arrival quote, as [`reinforced`]'s lines have it.
fn fleet_quote() -> SimMessage {
    SimMessage::FleetArrivalQuote("Here we come.".to_owned())
}

/// Every NPC keeps its goal and calls nothing.
#[derive(Debug)]
struct Keeping;

impl Behaviour for Keeping {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }
}

/// [`reinforced`], with every person's hail quote [`QUOTE`].
fn quoting() -> FakePilotCatalog {
    let mut catalog = reinforced();
    catalog.strings.push((7101, vec![QUOTE.to_owned()]));
    catalog
}

/// A flight of `catalog` whose Federation fleet was called, and whose
/// clock has `left` ticks to run before the fleet arrives, with a person
/// of `flags` at the player's side, attacking it when `attacks`.
#[allow(clippy::float_cmp)] // the clock runs in whole ticks here
fn coming(catalog: &FakePilotCatalog, left: i16, flags: u16, attacks: bool) -> Session {
    let mut session = flying(catalog);
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    for _ in 0..TIME - left {
        work_with(
            &mut session,
            catalog,
            Controls::default(),
            &Keeping,
            Rules::default(),
            &mut NeverFires,
        );
    }
    assert_eq!(timer(&session), f32::from(left), "the clock to run");
    let _ = (session.take_messages(), session.take_sounds());
    // The announcement's line run out: the passes above count no line down
    // (the countdown runs with the quote roll, in a frame's steps).
    session.space.comm_line = CommLine::default();
    let mut ship = npc(0, session.stats());
    ship.govt = Some(FED);
    ship.state.position = session.player().position;
    ship.person = Some(NpcPerson {
        flags,
        hail_quote: 1,
        ..NpcPerson::of(&person(600, 128), false)
    });
    if attacks {
        ship.goal = Goal::Attack(ShipRef::Player);
    }
    session.space.traffic.add_npc(ship);
    assert!(session.comm_line().free_for_calls(), "the line free");
    session
}

/// One frame of one step of `catalog`, the draws below 140 (the quote
/// roll's) giving `roll`.
fn frame(session: &mut Session, catalog: &FakePilotCatalog, roll: u32) -> StepReport {
    let mut chance = Draws::of(&[]).keyed(140, roll);
    let mut collab = StepCollaborators {
        catalog,
        behaviour: &Keeping,
        rules: Rules::default(),
        chance: &mut chance,
    };
    let input = StepInput {
        window_width: 1024,
        ..StepInput::default()
    };
    let mut report = StepReport::default();
    session.step(&input, 1, &mut collab, &mut report);
    report
}

/// The quotes `report`'s passes said, each pass's in turn.
fn said(report: &StepReport) -> Vec<Vec<String>> {
    report.steps[0]
        .passes
        .iter()
        .map(|pass| pass.quotes.iter().map(|quote| quote.text.clone()).collect())
        .collect()
}

/// The line as a line of `ticks` just shown leaves it.
fn shown(ticks: u16) -> CommLine {
    let mut line = CommLine::default();
    line.show(ticks);
    line
}

#[test]
fn a_due_quote_and_a_fleet_arriving_on_one_pass_end_the_frame_on_the_fleets_line_and_count() {
    // The person's quote, due on attack, is said at `_HandleShip`
    // (@0x44483), then the fleet arrives at `_HandleReinforcements`
    // (@0x44504) and its quote replaces the line and its count (@0x9024b,
    // @0x90251): 360, not the quote's 420.
    let catalog = quoting();
    let mut session = coming(&catalog, 1, QUOTE_ON_ATTACK, true);
    let report = frame(&mut session, &catalog, 139);
    assert_eq!(said(&report), [vec![QUOTE.to_owned()]], "the quote said");
    assert_eq!(
        report.steps[0].last_line,
        Some(RaisedLine::Message(fleet_quote())),
        "the fleet's line raised last"
    );
    assert_eq!(session.comm_line(), shown(fleet_quote().ticks()));
}

#[test]
fn on_the_rest_of_the_frame_a_dialog_opened_on_the_quote_comes_before_the_fleets_line() {
    // The rest of the opening frame runs the pass from the key block on,
    // its ship loop's end (`_HandleShip` @0x44483) and the reinforcement
    // clock's tick (`_HandleReinforcements` @0x44504) among it, so the
    // due quote is said there and the fleet's line replaces it (@0x9024b,
    // @0x90251), as on a whole pass. The rest runs at the gain as it
    // stands; a close at the key block would have zeroed it, and the
    // fleet's clock runs at the gain, so the rest is owed here at full
    // gain for the fleet to arrive on it.
    let catalog = quoting();
    let mut session = coming(&catalog, 1, QUOTE_ON_ATTACK, true);
    session.opening_rest_owed = true;
    let report = frame(&mut session, &catalog, 139);
    assert!(!session.opening_rest_owed(), "the rest run");
    assert_eq!(said(&report), [vec![QUOTE.to_owned()]], "the quote said");
    assert_eq!(
        report.steps[0].last_line,
        Some(RaisedLine::Message(fleet_quote())),
        "the fleet's line raised last"
    );
    assert_eq!(session.comm_line(), shown(fleet_quote().ticks()));
}

#[test]
fn a_random_quote_reads_the_line_free_before_the_fleets_line_is_raised() {
    // The roll's gate reads `_commCount` (@0x33f42-0x33f4b) before
    // `_HandleReinforcements` (@0x44504) raises the fleet's line, so a
    // draw of 0 says the quote, and the frame still ends on the fleet's.
    let catalog = quoting();
    let mut session = coming(&catalog, 1, 0, false);
    let report = frame(&mut session, &catalog, 0);
    assert_eq!(said(&report), [vec![QUOTE.to_owned()]], "the line was free");
    assert_eq!(
        report.steps[0].last_line,
        Some(RaisedLine::Message(fleet_quote()))
    );
    assert_eq!(session.comm_line(), shown(fleet_quote().ticks()));
}

#[test]
fn on_the_second_pass_the_quote_comes_before_the_fleets_line_and_the_count_is_the_fleets() {
    // `_HandleShip` (@0x44483) and `_HandleReinforcements` (@0x44504) run
    // on both passes, the roll first; `_DisplayComm` replaces the line and
    // its count (@0x9024b, @0x90251). A person due on attack would be said
    // on the advance pass already, so this one is only eligible, and the
    // fleet comes on the second pass, where its roll draws 0 on a free
    // line (@0x33f42-0x33f4b).
    let catalog = quoting();
    let mut session = coming(&catalog, 2, 0, false);
    // The advance pass's roll draws 139, the second pass's 0.
    let report = frame_drawing(&mut session, &catalog, &[139, 0]);
    let held = &report.steps[0].second_pass_lines;
    let kinds: Vec<_> = held
        .iter()
        .map(|line| match line {
            RaisedLine::Quote(quote) => quote.text.clone(),
            RaisedLine::Message(message) => format!("{message:?}"),
            RaisedLine::Comm(note) => format!("{note:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        [QUOTE.to_owned(), format!("{:?}", fleet_quote())],
        "the quote, then the fleet's line"
    );
    assert_eq!(session.comm_line(), shown(fleet_quote().ticks()));
}

/// One double-time frame of `catalog`, the quote rolls (draws below
/// 140) giving `rolls` in turn and every other draw its last outcome.
fn frame_drawing(session: &mut Session, catalog: &FakePilotCatalog, rolls: &[u32]) -> StepReport {
    let mut chance = Rolls(rolls.iter().copied().collect());
    let mut collab = StepCollaborators {
        catalog,
        behaviour: &Keeping,
        rules: Rules::default(),
        chance: &mut chance,
    };
    let input = StepInput {
        double_time_key: true,
        window_width: 1024,
        ..StepInput::default()
    };
    let mut report = StepReport::default();
    session.step(&input, 1, &mut collab, &mut report);
    assert_eq!(report.steps[0].passes.len(), 2, "double time");
    report
}

/// The quote rolls' draws (below 140) from a queue, every other draw
/// the last outcome, as [`NeverFires`] draws.
struct Rolls(std::collections::VecDeque<u32>);

impl Chance for Rolls {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        let queued = if n == 140 { self.0.pop_front() } else { None };
        queued.unwrap_or_else(|| NeverFires.below(n))
    }

    fn roll(&mut self, _sides: u16) -> u16 {
        0
    }
}
