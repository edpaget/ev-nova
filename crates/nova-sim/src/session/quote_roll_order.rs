//! Wiring tests of where a frame says its hail quotes: each hit's attack
//! quote as its hit settles (`_DamageShip`, `_AIMakeShipAttackPlayer`
//! @0x3be69), in the collision passes (@0x43e70-0x44010) or at
//! `_HandleShot`'s lock (@0x35855); and `_HandleShip`'s quote roll
//! (@0x33f0f-0x33fba) at each NPC's turn of the ship loop (@0x44483),
//! after its self-repair's line and among its display's lines, before the
//! reinforcement clock's tick (`_HandleReinforcements` @0x44504), on both
//! passes; against `_HandleComm`'s countdown (@0x4421e), after the
//! collision passes and before the arrivals. Each line goes through
//! `_DisplayComm` (@0x90201), which replaces the slot and the count
//! (@0x9024b, @0x90251), so the frame ends on the line raised last, with
//! that line's count.

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
    // (`_HandleComm`'s countdown, @0x4421e, runs only in a frame's steps,
    // which hand the pass the quotes' catalog).
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

// Where each of the frame's own hail quotes is said against the other
// lines (`_DisplayComm` @0x9024b, @0x90251): a hit's quote as its hit
// settles, and the roll slot by slot in the ship loop.

/// The person NPC 0 a hit turns, as `crate::session::tests::quoting_armed`
/// words it.
const DIE: &str = "Prepare to die!";

/// The bribed person's words, `STR#` 7101 #2.
const REGRET: &str = "You'll regret that.";

/// Every NPC keeps its goal and answers its hits as Nova's do.
#[derive(Debug, Default)]
struct Answering(crate::ai::NovaAi);

impl Behaviour for Answering {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn react(
        &self,
        npc: &Npc,
        strike: &crate::combat::Strike,
        around: &Surroundings,
    ) -> crate::ai::Reaction {
        self.0.react(npc, strike, around)
    }
}

/// The session's own string lists: `STR#` 7101 [`DIE`] and [`REGRET`],
/// and `STR#` 7600 "Hail #", a fleet's arrival quote.
#[derive(Debug)]
struct SessionLines;

impl crate::catalog::CommCatalog for SessionLines {
    fn string_list(&self, id: i16) -> Vec<String> {
        match id {
            crate::person::HAIL_QUOTES => vec![DIE.to_owned(), REGRET.to_owned()],
            ARRIVAL_QUOTES => vec!["Hail #".to_owned()],
            _ => Vec::new(),
        }
    }
}

/// The `STR#` of the arriving fleet's quote.
const ARRIVAL_QUOTES: i16 = 7600;

/// The arriving fleet's line, its `#` the last outcome of `Rand(9) + 1`.
fn arrival_quote() -> SimMessage {
    SimMessage::FleetArrivalQuote("Hail 9".to_owned())
}

/// One frame of one step of `catalog`, its NPCs answering as Nova's do,
/// the player firing when `fire`, drawing on `chance`.
fn fire_frame(
    session: &mut Session,
    catalog: &FakePilotCatalog,
    fire: bool,
    chance: &mut dyn Chance,
) -> StepReport {
    let behaviour = Answering::default();
    let mut collab = StepCollaborators {
        catalog,
        behaviour: &behaviour,
        rules: Rules::default(),
        chance,
    };
    let input = StepInput {
        window_width: 1024,
        fire: (fire, false),
        ..StepInput::default()
    };
    let mut report = StepReport::default();
    session.step(&input, 1, &mut collab, &mut report);
    report
}

/// `session` flown, the player firing one shot at NPC 0 on the first
/// frame, to the frame `landed` finds in its report: that frame is run on
/// a copy readied by `ready` and drawing on `chance`, the frames before
/// it as [`NeverFires`] draws. The session after that frame, and its
/// report.
fn the_frame_of_the_hit(
    mut session: Session,
    catalog: &FakePilotCatalog,
    ready: impl Fn(&mut Session),
    chance: impl Fn() -> Box<dyn Chance>,
    landed: impl Fn(&StepReport) -> bool,
) -> (Session, StepReport) {
    let _ = (
        session.take_messages(),
        session.take_sounds(),
        session.take_quotes(),
    );
    for frame in 0..60 {
        session.chart.target = Some(crate::traffic::npc::NpcId(0));
        let mut tried = session.clone();
        ready(&mut tried);
        let report = fire_frame(&mut tried, catalog, frame == 0, &mut *chance());
        if landed(&report) {
            return (tried, report);
        }
        fire_frame(&mut session, catalog, frame == 0, &mut NeverFires);
        let _ = (
            session.take_messages(),
            session.take_sounds(),
            session.take_quotes(),
        );
    }
    panic!("never landed");
}

/// Whether `report`'s frame hit NPC 0.
fn hits_npc_0(report: &StepReport) -> bool {
    report.steps[0].passes.iter().any(|pass| {
        pass.combat.iter().any(|event| {
            matches!(event, crate::combat::CombatEvent::Hit { ship, .. }
                if *ship == ShipRef::Npc(crate::traffic::npc::NpcId(0)))
        })
    })
}

/// Whether `report`'s frame said a quote.
fn quotes(report: &StepReport) -> bool {
    report.steps[0]
        .passes
        .iter()
        .any(|pass| !pass.quotes.is_empty())
}

/// `catalog`'s session facing NPC 0, a person of `Flags` 0x0010 waiting
/// out a coast, so the player's hit turns it with its quote at once
/// (`_DamageShip` @0x3be69); its words read from [`SessionLines`].
fn facing_a_coasting_person(catalog: &FakePilotCatalog) -> Session {
    let mut session = crate::session::tests::facing_an_npc(catalog, 180)
        .with_strings(std::rc::Rc::new(SessionLines));
    crate::session::tests::make_quoting_warship(&mut session);
    session.space.traffic.npcs_mut()[0].mode =
        crate::traffic::npc::Mode::Coasting { ticks_left: 1000.0 };
    session.space.comm_line = CommLine::default();
    session
}

/// `session`'s traffic table taking a fleet of ship 129 on a `Rand(500)`
/// of 1, under a cap of 3 ships, quoting `STR#` [`ARRIVAL_QUOTES`] as it
/// arrives by hyperspace (`_HyperSpawnFleet` @0x42607-0x426f4).
fn a_fleet_may_arrive(session: &mut Session) {
    use crate::catalog::{FleetId, FleetRecord, ShipId};
    let table = session.space.traffic.table_mut();
    table.avg_ships = 3;
    table.dude_fleets = vec![(FleetId(132), 100)];
    table.fleets.insert(
        FleetId(132),
        FleetRecord {
            id: FleetId(132),
            lead: Some(ShipId(129)),
            escorts: Vec::new(),
            govt: None,
            link_syst: 0,
            appear_on: crate::control::Test::default(),
            quote: Some(ARRIVAL_QUOTES),
            flags: 0,
        },
    );
}

/// A chance that brings the fleet: the first `Rand(500)` 1.
fn arriving() -> Box<dyn Chance> {
    Box::new(crate::session::tests::FleetArrival::default())
}

#[test]
fn a_collision_hits_quote_is_said_before_the_fleet_arriving_in_its_frame_whose_line_ends_it() {
    // `_DamageShip` from `_HandleShipHit` (@0x36c7a) says the quote in the
    // collision passes (@0x43e70-0x44010); `_HandleComm` counts the line
    // down (@0x4421e); then `_EnterMoreShips` (@0x44237) brings the fleet,
    // whose quote (`_DisplayComm` @0x426f4) replaces the line and its count
    // (@0x9024b, @0x90251).
    let catalog = crate::session::tests::quoting_armed();
    let mut session = facing_a_coasting_person(&catalog);
    a_fleet_may_arrive(&mut session);
    let (session, report) = the_frame_of_the_hit(session, &catalog, |_| {}, arriving, hits_npc_0);
    assert_eq!(
        said(&report),
        [vec![DIE.to_owned()]],
        "the hit's quote said"
    );
    assert_eq!(
        report.steps[0].last_line,
        Some(RaisedLine::Message(arrival_quote())),
        "the fleet's line raised last"
    );
    assert_eq!(session.comm_line(), shown(arrival_quote().ticks()));
}

#[test]
fn a_collision_hits_quote_ending_its_frame_is_counted_down_once_in_it() {
    // `_DamageShip` from `_HandleShipHit` (@0x36c7a) says the quote in the
    // collision passes (@0x43e70-0x44010), before `_HandleComm` counts the
    // line down (@0x4421e): with no later line, the frame ends on the quote
    // a tick short of its count.
    let catalog = crate::session::tests::quoting_armed();
    let session = facing_a_coasting_person(&catalog);
    let (session, report) = the_frame_of_the_hit(
        session,
        &catalog,
        |_| {},
        || Box::new(NeverFires),
        hits_npc_0,
    );
    assert_eq!(
        said(&report),
        [vec![DIE.to_owned()]],
        "the hit's quote said"
    );
    assert_eq!(last_quote(&report).as_deref(), Some(DIE), "the quote last");
    let mut line = shown(crate::person::QUOTE_SHOWN_TICKS);
    line.tick();
    assert_eq!(session.comm_line(), line, "counted down once");
}

#[test]
fn a_declined_bribes_quote_and_a_collision_hits_come_before_the_fleet_arriving_in_their_frame() {
    // A declined bribe (`_DoCommDialog` @0x9621f, `Session::act`'s attack,
    // which queues the quote as `turn_with_quote` does here) is said at the
    // next frame's first settle, the player's display (@0x43e29); the hit's
    // in the collision passes (@0x43e70-0x44010); and the fleet's line
    // (@0x44237, @0x426f4), after `_HandleComm` (@0x4421e), ends the frame.
    let catalog = crate::session::tests::quoting_armed();
    let mut session = facing_a_coasting_person(&catalog);
    a_fleet_may_arrive(&mut session);
    let mut bribed = crate::testkit::npc(0, session.stats());
    bribed.state.position = crate::geometry::Vec2::new(300.0, 300.0);
    bribed.person = Some(NpcPerson {
        flags: QUOTE_ON_ATTACK,
        hail_quote: 2,
        ..NpcPerson::of(&person(601, 128), false)
    });
    let bribed = session.space.traffic.add_npc(bribed);
    let declined = |session: &mut Session| {
        session
            .space
            .traffic
            .turn_with_quote(bribed, crate::person::GateView::default());
    };
    let (session, report) = the_frame_of_the_hit(session, &catalog, declined, arriving, hits_npc_0);
    assert_eq!(
        said(&report),
        [vec![REGRET.to_owned(), DIE.to_owned()]],
        "the bribe's, then the hit's"
    );
    assert_eq!(
        report.steps[0].last_line,
        Some(RaisedLine::Message(arrival_quote())),
        "the fleet's line raised last"
    );
    assert_eq!(session.comm_line(), shown(arrival_quote().ticks()));
}

/// [`crate::session::tests::quoting_armed`] with the player's gun a slow
/// homing missile, whose lock (`_HandleShot` @0x35855) comes some frames
/// after it is fired.
fn missile_armed() -> FakePilotCatalog {
    let missile = crate::catalog::WeaponRecord {
        guidance: 1,
        count: 40,
        speed: 100,
        ..crate::session::tests::blaster()
    };
    let mut catalog = crate::session::tests::arming(missile, crate::testkit::weapon(129), 0);
    catalog.strings = vec![(crate::person::HAIL_QUOTES, vec![DIE.to_owned()])];
    catalog
}

/// The words of `report`'s quote raised last, if its last line is one.
fn last_quote(report: &StepReport) -> Option<String> {
    match &report.steps[0].last_line {
        Some(RaisedLine::Quote(quote)) => Some(quote.text.clone()),
        _ => None,
    }
}

#[test]
fn a_repair_note_at_its_slot_of_the_ai_loop_comes_before_a_missile_locks_quote() {
    // The repairer's note (`_LowLevelAIHandler` @0x86ef4) is `_AIDispatch`'s
    // (@0x44270-0x442ae), before `_HandleShot` (@0x44385), whose lock
    // (@0x35855) says the quote: the frame ends on the quote, with its
    // count. The engine settled it so before this phase
    // (rdm:phase/combat-ai-fidelity-cross-roadmap/phase-12-left-out-of-phase-9),
    // so this pins it.
    use crate::traffic::plunder::PirateEvent;
    let catalog = missile_armed();
    let mut session = facing_a_coasting_person(&catalog);
    let mut helper = crate::testkit::npc(0, session.stats());
    helper.state.position = crate::geometry::Vec2::new(-300.0, 300.0);
    let helper = session.space.traffic.add_npc(helper);
    let repaired = |session: &mut Session| {
        let ship = session.space.npc(helper).expect("here").ship;
        session
            .space
            .queued_in_frame
            .push(PirateEvent::PlayerRepaired { helper, ship });
    };
    let (session, report) =
        the_frame_of_the_hit(session, &catalog, repaired, || Box::new(NeverFires), quotes);
    assert_eq!(report.steps[0].comm.len(), 1, "the repair note");
    assert_eq!(last_quote(&report).as_deref(), Some(DIE), "the quote last");
    assert_eq!(session.comm_line(), shown(crate::person::QUOTE_SHOWN_TICKS));
}

/// Adds to `session`, at berth `berth`, a disabled former escort carrying
/// the repair gear, far off, which repairs itself at its turn of the ship
/// loop on a draw of 0 (`_HandleShip` @0x33950-0x33bdb) and rejoins.
fn repairing_escort(session: &mut Session, berth: u32, at: crate::geometry::Vec2) -> NpcId {
    use crate::reserves::Reserves;
    let mut ship = crate::testkit::npc(0, session.stats());
    ship.state.position = at;
    ship.reserves = Reserves::full(30.0, 45.0, 300.0);
    ship.reserves.armor.now = 3.0;
    ship.condition = crate::combat::hull::Condition::Disabled;
    ship.self_repair = true;
    ship.former = Some(crate::self_repair::FormerEscort {
        record: crate::pilot::Escort {
            ship: ship.ship,
            reserves: ship.reserves,
            order: None,
            carried: false,
            wage: None,
            person: None,
            mark: None,
            voice: None,
            berth: None,
        },
    });
    session.space.traffic.add_npc_at(ship, berth)
}

/// A chance whose repair draws (`Rand(REPAIR_ODDS)`) give 0 and whose
/// quote rolls (`Rand(140)`) give `roll`.
fn repairing_rolling(roll: u32) -> Draws {
    Draws::of(&[])
        .keyed(crate::self_repair::REPAIR_ODDS, 0)
        .keyed(140, roll)
}

#[test]
fn a_missile_locks_quote_comes_before_a_self_repair_line_in_the_ship_loop() {
    // `_HandleShot` (@0x44385) says the lock's quote before the ship loop's
    // `_HandleShip` (@0x44483) raises the self-repair line (@0x33bac),
    // which replaces it and its count (@0x9024b, @0x90251).
    let catalog = missile_armed();
    let mut session = facing_a_coasting_person(&catalog);
    let escort = repairing_escort(&mut session, 9, crate::geometry::Vec2::new(-300.0, 300.0));
    let (session, report) = the_frame_of_the_hit(
        session,
        &catalog,
        |_| {},
        || Box::new(repairing_rolling(139)),
        quotes,
    );
    assert!(session.is_escort(escort), "repaired, it rejoined");
    assert_eq!(said(&report), [vec![DIE.to_owned()]], "the lock's quote");
    assert_eq!(
        report.steps[0].last_line,
        Some(RaisedLine::Message(SimMessage::EscortRepaired)),
        "the repair's line raised last"
    );
    assert_eq!(
        session.comm_line(),
        shown(SimMessage::EscortRepaired.ticks())
    );
}

// The ship loop's own lines and the quote roll, slot by slot.

/// The quote the second person says, `STR#` 7101 #2.
const HALT: &str = "Halt!";

/// [`crate::testkit::catalog`] with `STR#` 7101 [`QUOTE`] and [`HALT`].
fn rolling() -> FakePilotCatalog {
    let mut catalog = crate::testkit::catalog();
    catalog.strings = vec![(
        crate::person::HAIL_QUOTES,
        vec![QUOTE.to_owned(), HALT.to_owned()],
    )];
    catalog
}

/// A flight of `catalog` with no traffic, the line free and nothing
/// raised yet.
fn quiet_flight(catalog: &FakePilotCatalog) -> Session {
    let pilot = crate::pilot::Pilot::new(catalog, "Ada").expect("a pilot");
    let mut session = Session::fly(catalog, pilot).expect("flies");
    work_with(
        &mut session,
        catalog,
        Controls::default(),
        &Keeping,
        Rules::default(),
        &mut NeverFires,
    );
    let _ = (
        session.take_messages(),
        session.take_sounds(),
        session.take_quotes(),
    );
    session.space.comm_line = CommLine::default();
    session
}

/// Adds to `session`, at berth `berth` beside the player, a person
/// saying `STR#` 7101 #`hail_quote`, of `Flags` `flags`, attacking the
/// player when `attacks`.
fn person_at(
    session: &mut Session,
    berth: u32,
    hail_quote: i16,
    flags: u16,
    attacks: bool,
) -> NpcId {
    let mut ship = npc(0, session.stats());
    ship.state.position = session.player().position;
    ship.person = Some(NpcPerson {
        flags,
        hail_quote,
        ..NpcPerson::of(&person(600 + hail_quote, 128), false)
    });
    if attacks {
        ship.goal = Goal::Attack(ShipRef::Player);
        ship.target = Some(ShipRef::Player);
    }
    session.space.traffic.add_npc_at(ship, berth)
}

/// One frame of one step of `catalog`, its NPCs keeping their goals,
/// drawing on `chance`.
fn frame_on(
    session: &mut Session,
    catalog: &FakePilotCatalog,
    chance: &mut dyn Chance,
) -> StepReport {
    let mut collab = StepCollaborators {
        catalog,
        behaviour: &Keeping,
        rules: Rules::default(),
        chance,
    };
    let input = StepInput {
        window_width: 1024,
        ..StepInput::default()
    };
    let mut report = StepReport::default();
    session.step(&input, 1, &mut collab, &mut report);
    report
}

use crate::traffic::npc::NpcId;

#[test]
fn a_due_quote_in_a_lower_slot_than_a_self_repair_is_replaced_by_the_repairs_line() {
    // `_HandleShip` (@0x44483) runs slot by slot: the lower slot's quote
    // roll (@0x33f9e), then the higher slot's self-repair line (@0x33bac),
    // which replaces it and its count (@0x9024b, @0x90251).
    let catalog = rolling();
    let mut session = quiet_flight(&catalog);
    let far = crate::geometry::Vec2::new(-300.0, 300.0);
    // Listed first, in the higher berth: list order and berth order differ.
    let escort = repairing_escort(&mut session, 6, far);
    person_at(&mut session, 2, 1, QUOTE_ON_ATTACK, true);
    let report = frame_on(&mut session, &catalog, &mut repairing_rolling(139));
    assert!(session.is_escort(escort), "repaired, it rejoined");
    assert_eq!(said(&report), [vec![QUOTE.to_owned()]], "the quote said");
    assert_eq!(
        report.steps[0].last_line,
        Some(RaisedLine::Message(SimMessage::EscortRepaired))
    );
    assert_eq!(
        session.comm_line(),
        shown(SimMessage::EscortRepaired.ticks())
    );
}

#[test]
fn a_due_quote_in_a_higher_slot_than_a_self_repair_replaces_the_repairs_line() {
    // The lower slot's self-repair line (@0x33bac), then the higher slot's
    // quote (@0x33f9e), due now (@0x33f65-0x33f6c) though a line shows.
    let catalog = rolling();
    let mut session = quiet_flight(&catalog);
    let far = crate::geometry::Vec2::new(-300.0, 300.0);
    person_at(&mut session, 6, 1, QUOTE_ON_ATTACK, true);
    let escort = repairing_escort(&mut session, 2, far);
    let report = frame_on(&mut session, &catalog, &mut repairing_rolling(139));
    assert!(session.is_escort(escort), "repaired, it rejoined");
    assert_eq!(said(&report), [vec![QUOTE.to_owned()]], "the quote said");
    assert_eq!(last_quote(&report).as_deref(), Some(QUOTE));
    assert_eq!(session.comm_line(), shown(crate::person::QUOTE_SHOWN_TICKS));
}

#[test]
fn a_random_quote_reads_the_line_as_the_lower_slots_left_it() {
    // The roll's gate reads `_commCount` (@0x33f42-0x33f4b) at its own
    // slot: a lower slot's self-repair line (@0x33bac) keeps a draw of 0
    // off, and a higher slot's comes after the quote.
    let catalog = rolling();
    let far = crate::geometry::Vec2::new(-300.0, 300.0);
    for (person_berth, escort_berth, quoted) in [(2, 6, true), (6, 2, false)] {
        let mut session = quiet_flight(&catalog);
        person_at(&mut session, person_berth, 1, 0, false);
        repairing_escort(&mut session, escort_berth, far);
        let report = frame_on(&mut session, &catalog, &mut repairing_rolling(0));
        let expected = if quoted {
            vec![QUOTE.to_owned()]
        } else {
            Vec::new()
        };
        assert_eq!(said(&report), [expected], "person in berth {person_berth}");
        assert_eq!(
            report.steps[0].last_line,
            Some(RaisedLine::Message(SimMessage::EscortRepaired))
        );
    }
}

#[test]
fn two_due_quotes_are_said_in_slot_order_not_list_order() {
    // Each person's roll is `_HandleShip`'s at its own slot (@0x44483,
    // @0x33f9e): the lower slot's quote first, the higher's replacing it
    // (@0x9024b, @0x90251), whichever came into the system first.
    let catalog = rolling();
    for (first, second) in [(6, 2), (2, 6)] {
        let mut session = quiet_flight(&catalog);
        person_at(&mut session, first, 1, QUOTE_ON_ATTACK, true);
        person_at(&mut session, second, 2, QUOTE_ON_ATTACK, true);
        let report = frame_on(&mut session, &catalog, &mut Draws::of(&[]).keyed(140, 139));
        let (lower, higher) = if first < second {
            (QUOTE, HALT)
        } else {
            (HALT, QUOTE)
        };
        assert_eq!(
            said(&report),
            [vec![lower.to_owned(), higher.to_owned()]],
            "listed {first}, then {second}"
        );
        assert_eq!(last_quote(&report).as_deref(), Some(higher));
    }
}

/// Adds to `session`, at berth `berth`, 40 pixels above the player, a
/// ship of mass 200 breaking up with its timer at 2: its own step takes it
/// to 1, and its display (`_HandleShipDisplay` @0x444ad) makes its final
/// explosion, whose shockwave (`_DamageShip` @0x2da89) reaches the player.
fn wreck_above_the_player(session: &mut Session, berth: u32) -> NpcId {
    let mut ship = npc(0, session.stats());
    ship.hull.mass = 200.0;
    ship.hull.death_delay = 5;
    ship.reserves.armor.now = 0.0;
    ship.condition = crate::combat::hull::Condition::Dying {
        ticks_left: 2,
        starting: false,
    };
    ship.state.position = session.player().position + crate::geometry::Vec2::new(0.0, -40.0);
    session.space.traffic.add_npc_at(ship, berth)
}

/// [`quiet_flight`] with the player's shield down and its armour 13,340
/// of 40,000, a hair above a third: a shockwave disables it ("Ship
/// disabled.", `_DamageShip` @0x3b181), as its floor that keeps a ship
/// from being newly disabled clears the line only for a hull of 33,333
/// armour or less (@0x3acc2-0x3ad4a).
fn a_frail_player(catalog: &FakePilotCatalog) -> Session {
    let mut session = quiet_flight(catalog);
    session.pilot.reserves.shield = crate::reserves::Gauge { now: 0.0, max: 0.0 };
    session.pilot.reserves.armor = crate::reserves::Gauge {
        now: 13_340.0,
        max: 40_000.0,
    };
    session
}

#[test]
fn a_due_quote_and_a_wrecks_downing_line_end_the_frame_on_the_higher_slots() {
    // `_HandleShipDisplay` (@0x444ad) runs at the wreck's slot of the ship
    // loop, its shockwave's `_DamageShip` (@0x2da89) raising "Ship
    // disabled." (@0x3b181); `_HandleShip`'s roll (@0x33f9e) at the
    // person's: the higher slot's line replaces the lower's and its count
    // (@0x9024b, @0x90251). The shockwave passes `_DamageShip` no direct
    // hit (@0x2da52), so it skips the reaction block (@0x3b44d) and the
    // person it strikes says no quote of its own: the display's line, not
    // a quote, is what takes the wreck's slot.
    let catalog = rolling();
    for (wreck_berth, person_berth) in [(2, 6), (6, 2)] {
        let mut session = a_frail_player(&catalog);
        wreck_above_the_player(&mut session, wreck_berth);
        person_at(&mut session, person_berth, 1, QUOTE_ON_ATTACK, true);
        let report = frame_on(&mut session, &catalog, &mut Draws::of(&[]).keyed(140, 139));
        assert_eq!(
            session.player_condition(),
            crate::combat::hull::Condition::Disabled,
            "downed"
        );
        assert_eq!(
            said(&report),
            [vec![QUOTE.to_owned()]],
            "the roll's quote alone: the struck person says none"
        );
        if wreck_berth < person_berth {
            assert_eq!(
                last_quote(&report).as_deref(),
                Some(QUOTE),
                "the quote last"
            );
            assert_eq!(session.comm_line(), shown(crate::person::QUOTE_SHOWN_TICKS));
        } else {
            assert_eq!(
                report.steps[0].last_line,
                Some(RaisedLine::Message(SimMessage::ShipDisabled)),
                "the downing line last"
            );
            assert_eq!(session.comm_line(), shown(SimMessage::ShipDisabled.ticks()));
        }
    }
}

#[test]
fn a_self_repair_line_and_a_wrecks_downing_line_end_the_frame_on_the_higher_slots() {
    // Both are the ship loop's, at their own slots (`_HandleShip` @0x33bac,
    // `_HandleShipDisplay` @0x444ad → @0x3b181), so the higher slot's line
    // replaces the lower's (@0x9024b, @0x90251). The engine raised both at
    // their turns of the loop before this phase, so this pins their order;
    // the count is the line's full one, as `_HandleComm` (@0x4421e) counts
    // down before the loop.
    let catalog = rolling();
    let far = crate::geometry::Vec2::new(-300.0, 300.0);
    for (wreck_berth, escort_berth) in [(2, 6), (6, 2)] {
        let mut session = a_frail_player(&catalog);
        wreck_above_the_player(&mut session, wreck_berth);
        repairing_escort(&mut session, escort_berth, far);
        let report = frame_on(&mut session, &catalog, &mut repairing_rolling(139));
        let last = if wreck_berth < escort_berth {
            SimMessage::EscortRepaired
        } else {
            SimMessage::ShipDisabled
        };
        assert_eq!(
            report.steps[0].last_line,
            Some(RaisedLine::Message(last.clone())),
            "wreck in berth {wreck_berth}"
        );
        assert_eq!(session.comm_line(), shown(last.ticks()));
    }
}

// The countdown and the quote clock: once a frame, on the advance pass,
// and only in a frame's steps.

/// The quote clock's ticks.
fn clock(session: &Session) -> u64 {
    session.space.escorts.quote_clock.ticks
}

#[test]
fn a_frame_counts_the_line_down_and_moves_the_quote_clock_once_and_a_pass_alone_neither() {
    // `_HandleComm` (@0x4421e) runs on the advance pass, and the quote
    // clock stands for `TickCount` there; a pass run alone, with no quotes'
    // catalog, does neither.
    let catalog = rolling();
    let mut session = quiet_flight(&catalog);
    session.space.comm_line = shown(10);
    let before = clock(&session);
    work_with(
        &mut session,
        &catalog,
        Controls::default(),
        &Keeping,
        Rules::default(),
        &mut NeverFires,
    );
    assert_eq!(session.comm_line(), shown(10), "a pass alone");
    assert_eq!(clock(&session), before, "a pass alone");
    frame_on(&mut session, &catalog, &mut NeverFires);
    let mut line = shown(10);
    line.tick();
    assert_eq!(session.comm_line(), line, "a frame");
    assert_eq!(clock(&session), before + 1, "a frame");
}

#[test]
fn a_double_time_frame_counts_down_and_moves_the_clock_on_its_advance_pass_alone() {
    // `_HandleComm` (@0x4421e) runs on the advance pass only, and the
    // second pass reads the quote clock unmoved.
    let catalog = quoting();
    let mut session = coming(&catalog, 5, 0, false);
    session.space.comm_line = shown(10);
    let before = clock(&session);
    frame_drawing(&mut session, &catalog, &[139, 139]);
    let mut line = shown(10);
    line.tick();
    assert_eq!(session.comm_line(), line);
    assert_eq!(clock(&session), before + 1);
}
