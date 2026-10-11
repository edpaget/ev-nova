//! Every stock mission with special ships and a ship goal, walked to the
//! end the original gives it: each pilot set up directly (its bits,
//! rating, record and ship), the mission offered where its `AvailLoc`
//! says (at a stellar, from a hailed person, or started by a script's
//! `S`), accepted, its goal met by the storyline walker's `meet goal` or
//! its ships flown in, and its end checked.
//!
//! [`OUTCOMES`] lists the missions that do not complete at their return
//! and how each ends instead; every other one completes.
//! [`EXPECTED_FAILURES`] lists the missions the engine cannot yet walk to
//! that end, each with why and the rdm item that fixes it. A listed
//! mission that now walks fails the test, so the list only shrinks.
//!
//! Whole storylines (crons, ranks, ratings earned in play) are walked
//! elsewhere; this test sets each pilot up for one mission. Skips,
//! passing, when `NOVA_DATA` is unset.

mod common;

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use nova_data::GameData;
use nova_sim::catalog::{CombatCatalog, MissionRecord, PilotCatalog, TrafficCatalog};
use nova_sim::fixture::{Step, StoryWalker, bits_allowing, candidates, expected_bits};
use nova_sim::mission::ShipSystem;
use nova_sim::rules::Verdict;
use nova_sim::{
    Chance, GovtId, NeverFires, NovaAi, NovaPersons, Pilot, Session, SetExpr, StellarId, SystemId,
    Test, is_landable,
};

/// How a mission the walk covers ends, the way the original plays it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    /// Completed at its return, once its goal is met.
    Completed,
    /// Ends as its goal is met: a `Flags` 0x0001 auto-abort, or its own
    /// `OnShipDone` aborting it.
    EndsAtGoal,
    /// Auto-aborts as its ships come in; its goal is never met.
    AbortsAtSpawn,
    /// Its goal met, still held: it has no return, so no landing
    /// completes it, and nothing in it ends it.
    HeldAfterGoal,
}

/// The missions that do not complete at their return, how each ends, and
/// why. Every other mission walked completes.
const OUTCOMES: &[(i16, Outcome, &str)] = &[
    (
        133,
        Outcome::AbortsAtSpawn,
        "Flags 0x0001: auto-aborts once its ships jump in",
    ),
    (
        141,
        Outcome::EndsAtGoal,
        "Flags 0x0001: auto-aborts at the board (@0x9ed87)",
    ),
    (
        596,
        Outcome::HeldAfterGoal,
        "no return; unabortable and hidden",
    ),
    (
        614,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts on the timer",
    ),
    (
        615,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts on the timer",
    ),
    (
        616,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts on the timer",
    ),
    (
        617,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts on the timer",
    ),
    (
        618,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts at the setup",
    ),
    (
        619,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts at the setup",
    ),
    (
        620,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts at the setup",
    ),
    (
        621,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts at the setup",
    ),
    (
        622,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts on the timer",
    ),
    (
        623,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts on the timer",
    ),
    (
        624,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts on the timer",
    ),
    (
        625,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts on the timer",
    ),
    (
        626,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts at the setup",
    ),
    (
        627,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts at the setup",
    ),
    (
        628,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts on the timer",
    ),
    (
        629,
        Outcome::AbortsAtSpawn,
        "an Avoid mission, Flags 0x0001: auto-aborts on the timer",
    ),
    (
        650,
        Outcome::EndsAtGoal,
        "Flags 0x0001: auto-aborts at the board (@0x9ed87)",
    ),
    (
        651,
        Outcome::EndsAtGoal,
        "Flags 0x0001: auto-aborts at the board (@0x9ed87)",
    ),
    (
        652,
        Outcome::EndsAtGoal,
        "Flags 0x0001: auto-aborts at the board (@0x9ed87)",
    ),
    (
        685,
        Outcome::HeldAfterGoal,
        "no return; another mission's A ends it",
    ),
    (
        724,
        Outcome::HeldAfterGoal,
        "no return; nothing in it ends it",
    ),
    (727, Outcome::EndsAtGoal, "its OnShipDone aborts it"),
    (781, Outcome::EndsAtGoal, "its OnShipDone aborts it"),
    (
        793,
        Outcome::HeldAfterGoal,
        "no return; another mission's A ends it",
    ),
    (867, Outcome::EndsAtGoal, "its OnShipDone aborts it"),
    (868, Outcome::EndsAtGoal, "its OnShipDone aborts it"),
    (869, Outcome::EndsAtGoal, "its OnShipDone aborts it"),
    (870, Outcome::EndsAtGoal, "its OnShipDone aborts it"),
    (871, Outcome::EndsAtGoal, "its OnShipDone aborts it"),
    (877, Outcome::EndsAtGoal, "its OnShipDone aborts it"),
    (
        891,
        Outcome::HeldAfterGoal,
        "no return; another mission's A ends it",
    ),
    (
        892,
        Outcome::HeldAfterGoal,
        "no return; another mission's A ends it",
    ),
    (910, Outcome::EndsAtGoal, "its OnShipDone aborts it"),
    (911, Outcome::EndsAtGoal, "its OnShipDone aborts it"),
];

/// A mission the engine cannot yet walk to its outcome.
struct ExpectedFailure {
    /// The `mïsn`.
    mission: i16,
    /// What goes wrong.
    reason: &'static str,
    /// The rdm item that fixes it: `rdm:phase/<roadmap>/<stem>`,
    /// `rdm:task/<slug>` or `rdm:roadmap/<slug>`.
    item: &'static str,
}

/// The missions the engine cannot yet walk to their outcome.
const EXPECTED_FAILURES: &[ExpectedFailure] = &[];

/// Every stock mission with special ships (`ShipCount` above 0) and a
/// ship goal (`ShipGoal` not -1).
const TARGETS: [i16; 135] = [
    132, 140, 141, 155, 156, 157, 158, 159, 171, 173, 174, 183, 186, 196, 257, 258, 259, 260, 261,
    264, 265, 270, 271, 272, 273, 274, 275, 278, 341, 344, 350, 384, 390, 391, 394, 400, 417, 469,
    473, 551, 552, 554, 596, 597, 601, 603, 604, 613, 614, 615, 616, 617, 618, 619, 620, 621, 622,
    623, 624, 625, 626, 627, 628, 629, 636, 639, 650, 651, 652, 657, 664, 665, 668, 669, 670, 671,
    674, 676, 678, 685, 688, 709, 710, 712, 714, 724, 727, 729, 737, 739, 741, 759, 760, 761, 762,
    763, 764, 781, 793, 831, 832, 833, 843, 844, 845, 849, 850, 851, 852, 853, 855, 856, 857, 862,
    863, 864, 865, 866, 867, 868, 869, 870, 871, 874, 876, 877, 878, 882, 889, 891, 892, 910, 911,
    914, 915,
];

/// The one mission outside [`TARGETS`] whose end turns on its special
/// ships: 133 (Derelict Decoy), `ShipGoal` -1, which auto-aborts once
/// they jump in.
const DERELICT_DECOY: i16 = 133;

/// How a mission comes to be held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Start {
    /// Offered at a stellar (`AvailRandom` above 0).
    Stellar,
    /// Started only by a script's `S` (`AvailRandom` 0).
    Script,
    /// Offered by a hailed person (`AvailLoc` 2).
    Ship,
}

/// How `record` comes to be held; 133 is started by `S`, its offer being
/// a derelict's, boarded.
fn start_of(record: &MissionRecord) -> Start {
    if record.id.0 == DERELICT_DECOY {
        Start::Script
    } else if record.avail_loc == 2 {
        Start::Ship
    } else if record.avail_random <= 0 {
        Start::Script
    } else {
        Start::Stellar
    }
}

/// A test function's share of the walk.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Shard {
    /// The stellar-offered missions in an ID range.
    Stellar(RangeInclusive<i16>),
    /// The script-started missions, 133 among them.
    Script,
    /// The person-offered missions.
    Ship,
}

/// The shards, one test function each.
fn shards() -> [Shard; 6] {
    [
        Shard::Stellar(128..=499),
        Shard::Stellar(500..=655),
        Shard::Stellar(656..=799),
        Shard::Stellar(800..=999),
        Shard::Script,
        Shard::Ship,
    ]
}

/// The stock missions the walk covers: [`TARGETS`] and 133.
fn walked(data: &GameData) -> Vec<MissionRecord> {
    data.missions()
        .into_iter()
        .filter(|record| {
            (record.ships.count > 0 && record.ships.goal != -1) || record.id.0 == DERELICT_DECOY
        })
        .collect()
}

/// The missions of `shard`.
fn members(data: &GameData, shard: &Shard) -> Vec<MissionRecord> {
    walked(data)
        .into_iter()
        .filter(|record| match (shard, start_of(record)) {
            (Shard::Stellar(ids), Start::Stellar) => ids.contains(&record.id.0),
            (Shard::Script, Start::Script) | (Shard::Ship, Start::Ship) => true,
            _ => false,
        })
        .collect()
}

/// The outcome `mission` is expected to reach.
fn outcome(mission: i16) -> Outcome {
    OUTCOMES
        .iter()
        .find(|&&(id, ..)| id == mission)
        .map_or(Outcome::Completed, |&(_, outcome, _)| outcome)
}

/// Whether `item` is an rdm ref.
fn is_rdm_ref(item: &str) -> bool {
    let Some(rest) = item.strip_prefix("rdm:") else {
        return false;
    };
    let parts: Vec<&str> = rest.split('/').collect();
    let named = |part: &&str| !part.is_empty() && !part.contains(' ');
    match parts.as_slice() {
        ["phase", roadmap, stem] => [roadmap, stem].iter().all(|part| named(part)),
        ["task" | "roadmap", slug] => named(slug),
        _ => false,
    }
}

#[test]
fn the_expected_lists_are_well_formed() {
    for failure in EXPECTED_FAILURES {
        assert!(!failure.reason.is_empty(), "{}: a reason", failure.mission);
        assert!(
            is_rdm_ref(failure.item),
            "{}: {:?} is an rdm ref",
            failure.mission,
            failure.item
        );
    }
    let failing: Vec<i16> = EXPECTED_FAILURES.iter().map(|f| f.mission).collect();
    let unique: BTreeSet<i16> = failing.iter().copied().collect();
    assert_eq!(unique.len(), failing.len(), "listed once each");
    let outcomes: Vec<i16> = OUTCOMES.iter().map(|&(id, ..)| id).collect();
    let unique: BTreeSet<i16> = outcomes.iter().copied().collect();
    assert_eq!(unique.len(), outcomes.len(), "listed once each");
    assert_eq!(outcomes.len(), 37);
    for &(id, _, reason) in OUTCOMES {
        assert!(!reason.is_empty(), "{id}: a reason");
    }
}

#[test]
fn rdm_refs_name_a_phase_a_task_or_a_roadmap() {
    for good in [
        "rdm:phase/mission-ships/phase-16-cloaked-mission-ships",
        "rdm:task/fix-it",
        "rdm:roadmap/missions",
    ] {
        assert!(is_rdm_ref(good), "{good}");
    }
    for bad in [
        "",
        "TBD",
        "phase/a/b",
        "rdm:phase/a",
        "rdm:phase//b",
        "rdm:task/",
        "rdm:task/a b",
        "rdm:roadmap/a/b",
        "rdm:review/a",
    ] {
        assert!(!is_rdm_ref(bad), "{bad}");
    }
}

#[test]
fn the_walk_covers_every_stock_mission_with_ship_goals() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let targets: Vec<i16> = data
        .missions()
        .iter()
        .filter(|record| record.ships.count > 0 && record.ships.goal != -1)
        .map(|record| record.id.0)
        .collect();
    assert_eq!(targets, TARGETS);
    let mut covered = Vec::new();
    for shard in shards() {
        covered.extend(members(&data, &shard).iter().map(|record| record.id.0));
    }
    covered.sort_unstable();
    let mut expected: Vec<i16> = TARGETS.to_vec();
    expected.push(DERELICT_DECOY);
    expected.sort_unstable();
    assert_eq!(covered, expected, "every one in exactly one shard");
    for id in OUTCOMES
        .iter()
        .map(|&(id, ..)| id)
        .chain(EXPECTED_FAILURES.iter().map(|failure| failure.mission))
    {
        assert!(expected.contains(&id), "{id} is walked");
    }
}

/// A new stock pilot docked at the first landable stellar of its system,
/// set up for `record`: the combat rating it asks, a record every
/// government passes when it asks one, the bits its `AvailBits` and
/// `extra` ask, and cash to spare.
fn set_up(data: &GameData, record: &MissionRecord, extra: &[&Test]) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let site = data
        .landing_sites(pilot.system())
        .into_iter()
        .find(is_landable)
        .expect("a landable stellar where the pilot starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(site.id.0);
    save["combat_rating"] = serde_json::json!(i64::from(record.avail_rating.max(0)));
    let mut docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    if record.avail_record > 0 {
        for govt in data.governments() {
            docked.set_legal_record(govt.id, 1000);
        }
    }
    let mut session = Session::fly(data, docked).expect("flies");
    let _ = session.settle_script(data, &mut NeverFires);
    session.set_credits(100_000_000);
    for test in std::iter::once(&record.avail_bits).chain(extra.iter().copied()) {
        let bits = bits_allowing(test).expect("bits alone open it");
        for (bit, on) in bits {
            session.set_control_bit(bit, on);
        }
    }
    session
}

/// The first line of `text`.
fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or_default()
}

/// Where `record` is offered: its `AvailStel` when that names a stellar,
/// else the first listed stellar whose refusals have neither `AvailStel`
/// nor `not going here`. `session` is left docked there.
fn offering_stellar(
    data: &GameData,
    session: &mut Session,
    record: &MissionRecord,
) -> Option<(StellarId, SystemId)> {
    let listed = candidates(&data.listed_stellars(), None);
    let named = (128..=2175).contains(&record.avail_stel);
    for (stellar, system) in listed {
        if named && stellar.0 != record.avail_stel {
            continue;
        }
        if session.relocate(data, system, stellar).is_err() {
            continue;
        }
        let refusals = session.offer_refusals(record.id).unwrap_or_default();
        let elsewhere = refusals.iter().any(|(name, verdict)| {
            matches!(verdict, Verdict::Refuses(_))
                && matches!(*name, "AvailStel" | "not going here")
        });
        if named || !elsewhere {
            return Some((stellar, system));
        }
    }
    None
}

/// The availability checks the player's ship decides.
const SHIP_CHECKS: [&str; 3] = ["AvailShipType", "InherentAI", "cargo space"];

/// Whether the ship docked in `session` is refused `record` for itself.
fn ship_refused(session: &Session, record: &MissionRecord) -> bool {
    session
        .offer_refusals(record.id)
        .unwrap_or_default()
        .iter()
        .any(|(name, verdict)| matches!(verdict, Verdict::Refuses(_)) && SHIP_CHECKS.contains(name))
}

/// When the docked ship is refused `record` for itself, changes it by `C`
/// for the ship its `AvailShipType` names, or else for the first stock
/// ship, by ID, that the checks take.
fn fit_the_ship(data: &GameData, session: &mut Session, record: &MissionRecord) {
    if !ship_refused(session, record) {
        return;
    }
    let ships: Vec<i16> = if (128..=896).contains(&record.avail_ship_type) {
        vec![record.avail_ship_type]
    } else {
        data.ships().iter().map(|ship| ship.id.0).collect()
    };
    for ship in ships {
        session.run_set(
            &SetExpr::parse(&format!("C{ship}")).expect("parses"),
            &mut NeverFires,
        );
        let _ = session.settle_script(data, &mut NeverFires);
        if !ship_refused(session, record) {
            return;
        }
    }
}

/// Takes `record`, offered at a stellar, into a slot.
fn take_at_a_stellar(
    data: &GameData,
    session: &mut Session,
    record: &MissionRecord,
) -> Result<(), String> {
    let (stellar, system) =
        offering_stellar(data, session, record).ok_or("no stellar takes its AvailStel")?;
    fit_the_ship(data, session, record);
    session
        .land_at(data, system, stellar)
        .map_err(|refusal| format!("lands at {stellar:?}: {refusal:?}"))?;
    let _ = session.settle_script(data, &mut NeverFires);
    StoryWalker::new(data)
        .walk(session, &[Step::Take(record.id)])
        .map(|_| ())
        .map_err(|failure| {
            format!(
                "take at {stellar:?}: {} {:?}",
                first_line(&failure.to_string()),
                session.offer_refusals(record.id)
            )
        })
}

/// Starts `record` by `S`, docked.
fn start_by_script(
    data: &GameData,
    session: &mut Session,
    record: &MissionRecord,
) -> Result<(), String> {
    session.run_set(
        &SetExpr::parse(&format!("S{}", record.id.0)).expect("parses"),
        &mut NeverFires,
    );
    let _ = session.settle_script(data, &mut NeverFires);
    if session.pilot().missions().holding(record.id).is_empty() {
        return Err("S did not start it".into());
    }
    Ok(())
}

/// Draws by the bound asked, as listed, and 0 for any other bound.
struct ByBound(Vec<(u32, u32)>);

impl Chance for ByBound {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0
            .iter()
            .find(|&&(bound, _)| bound == n)
            .map_or(0, |&(_, draw)| draw)
    }
}

/// Where a person who offers a mission appears.
#[derive(Clone, Copy, Debug)]
enum Haunt {
    /// This system.
    System(i16),
    /// Any system this government governs.
    Govt(i16),
}

/// The person-offered missions: the `përs` hailed and where it appears.
const SHIP_OFFERS: [(i16, i16, Haunt); 6] = [
    (132, 128, Haunt::System(128)),
    (140, 131, Haunt::System(132)),
    (141, 225, Haunt::Govt(134)),
    (650, 222, Haunt::Govt(134)),
    (651, 384, Haunt::Govt(131)),
    (652, 224, Haunt::Govt(134)),
];

/// `record`'s person, hailed in flight, offers it and it is accepted. The
/// pilot is set up for the mission and the person's `ActiveOn`, then
/// flown in the system the person appears in, every setup's person roll
/// landing on it.
fn take_from_a_ship(data: &GameData, record: &MissionRecord) -> Result<Session, String> {
    let &(_, person, haunt) = SHIP_OFFERS
        .iter()
        .find(|&&(mission, ..)| mission == record.id.0)
        .ok_or("no person listed")?;
    let persons = TrafficCatalog::persons(data);
    let index = persons
        .iter()
        .position(|p| p.id.0 == person)
        .ok_or("no such përs")?;
    let who = &persons[index];
    if who.link_mission != Some(record.id.0) {
        return Err(format!("përs {person} links {:?}", who.link_mission));
    }
    let system = match haunt {
        Haunt::System(system) => system,
        Haunt::Govt(govt) => {
            data.star_map()
                .into_iter()
                .find(|system| system.govt == Some(GovtId(govt)))
                .ok_or("no system of its government")?
                .id
                .0
        }
    };
    let docked = set_up(data, record, &[&who.active_on]);
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(docked.pilot())).expect("JSON");
    save["system"] = serde_json::json!(system);
    save["stellar"] = serde_json::Value::Null;
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(data, pilot)
        .expect("flies")
        .with_person_rules(std::rc::Rc::new(NovaPersons::default()));
    let index = u32::try_from(index).expect("an index");
    session.populate(data, &mut ByBound(vec![(7, 0), (1022, index), (100, 99)]));
    let npc = session
        .npcs()
        .iter()
        .find(|npc| npc.person.is_some_and(|p| p.id.0 == person))
        .map(|npc| npc.id)
        .ok_or_else(|| format!("përs {person} not placed in {system}"))?;
    let mut tries = 0;
    while session.select_target(nova_sim::TargetPick::Next) != Some(npc) {
        tries += 1;
        if tries > 500 {
            return Err("never targeted".into());
        }
    }
    let options = nova_sim::HailOptions::default();
    match session.open_hail(data, &options, &mut NeverFires) {
        Ok(nova_sim::Hailed::Offer(_)) => {}
        other => return Err(format!("the hail: {other:?}")),
    }
    session
        .accept_mission(record.id, &mut NeverFires)
        .map_err(|refused| format!("the accept: {refused:?}"))?;
    let _ = session.settle_script(data, &mut NeverFires);
    Ok(session)
}

/// The bits `record`'s `OnShipDone` sets that its `OnAbort` does not.
fn ship_done_only(record: &MissionRecord) -> Vec<nova_sim::Bit> {
    let aborted = expected_bits(&record.on_abort);
    expected_bits(&record.on_ship_done)
        .into_iter()
        .filter(|bit| !aborted.contains(bit))
        .collect()
}

/// The `OnShipDone` bits of `record` not set in `session`.
fn unset_ship_done_bits(session: &Session, record: &MissionRecord) -> Vec<nova_sim::Bit> {
    expected_bits(&record.on_ship_done)
        .into_iter()
        .filter(|&bit| !session.control_bit(bit))
        .collect()
}

/// Meets `record`'s goal with the walker and checks it ends as `outcome`
/// says.
fn meet_goal(
    data: &GameData,
    session: &mut Session,
    record: &MissionRecord,
    outcome: Outcome,
) -> Result<(), String> {
    let mut walker = StoryWalker::new(data);
    walker
        .walk(session, &[Step::MeetGoal(record.id)])
        .map_err(|failure| format!("meet goal: {}", first_line(&failure.to_string())))?;
    let held = session
        .pilot()
        .missions()
        .iter()
        .find(|(_, active)| active.mission == record.id)
        .map(|(_, active)| active.clone());
    match (outcome, held) {
        (Outcome::Completed, Some(_)) => walker
            .walk(session, &[Step::Deliver(record.id)])
            .map(|_| ())
            .map_err(|failure| format!("deliver: {}", first_line(&failure.to_string()))),
        (Outcome::EndsAtGoal, None) => {
            let unset = unset_ship_done_bits(session, record);
            if unset.is_empty() {
                Ok(())
            } else {
                Err(format!("ended, its OnShipDone bits {unset:?} clear"))
            }
        }
        (Outcome::HeldAfterGoal, Some(active)) => {
            let unset = unset_ship_done_bits(session, record);
            if active.goals_met && active.failed.is_none() && unset.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "held: goals met {}, failed {:?}, OnShipDone bits {unset:?} clear",
                    active.goals_met, active.failed
                ))
            }
        }
        (_, Some(active)) => Err(format!(
            "still held after meet goal: goals met {}, failed {:?}",
            active.goals_met, active.failed
        )),
        (_, None) => Err("ended at meet goal".into()),
    }
}

/// The traffic ticks a flown mission is given for its ships to come in.
const TICKS: u32 = 2_000;

/// Flies `record`, held, until its ships come in and it auto-aborts:
/// takes off (in its ships' system, when they have one), then ticks the
/// traffic. Its `OnShipDone` must not have run.
fn fly_until_aborted(
    data: &GameData,
    session: &mut Session,
    record: &MissionRecord,
) -> Result<(), String> {
    let slot = *session
        .pilot()
        .missions()
        .holding(record.id)
        .first()
        .ok_or("not held")?;
    let ships = session
        .pilot()
        .missions()
        .get(slot)
        .ok_or("not held")?
        .ships
        .system;
    if let ShipSystem::At(system) = ships
        && system != session.pilot().system()
    {
        let stellar = data
            .landing_sites(system)
            .into_iter()
            .find(is_landable)
            .ok_or_else(|| format!("nowhere to land in {system:?}"))?
            .id;
        session
            .relocate(data, system, stellar)
            .map_err(|refusal| format!("to {system:?}: {refusal:?}"))?;
    }
    session.take_off(&mut NeverFires).ok_or("takes off")?;
    let _ = session.settle_script(data, &mut NeverFires);
    let ai = NovaAi::default();
    let mut ticks = 0;
    while session.pilot().missions().get(slot).is_some() {
        if ticks == TICKS {
            return Err(format!("still held after {TICKS} ticks"));
        }
        session.tick_traffic(data, &ai, &mut NeverFires);
        let _ = session.settle_script(data, &mut NeverFires);
        ticks += 1;
    }
    let ran: Vec<_> = ship_done_only(record)
        .into_iter()
        .filter(|&bit| session.control_bit(bit))
        .collect();
    if ran.is_empty() {
        Ok(())
    } else {
        Err(format!("its OnShipDone ran: {ran:?} set"))
    }
}

/// Walks `record` to its outcome: `Ok` when it gets there, else what went
/// wrong.
fn walk(data: &GameData, record: &MissionRecord) -> Result<(), String> {
    let mut session = match start_of(record) {
        Start::Stellar => {
            let mut session = set_up(data, record, &[]);
            take_at_a_stellar(data, &mut session, record)?;
            session
        }
        Start::Script => {
            let mut session = set_up(data, record, &[]);
            start_by_script(data, &mut session, record)?;
            session
        }
        Start::Ship => take_from_a_ship(data, record)?,
    };
    match outcome(record.id.0) {
        Outcome::AbortsAtSpawn => fly_until_aborted(data, &mut session, record),
        expected => meet_goal(data, &mut session, record, expected),
    }
}

/// Walks every mission of `shard` and fails, once, with every mission
/// that missed its outcome unlisted or reached it listed.
fn walk_shard(shard: &Shard) {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut mismatches = Vec::new();
    for record in members(&data, shard) {
        let id = record.id.0;
        let listed = EXPECTED_FAILURES
            .iter()
            .find(|failure| failure.mission == id);
        match (walk(&data, &record), listed) {
            (Ok(()), Some(failure)) => mismatches.push(format!(
                "{id} now walks; remove it from EXPECTED_FAILURES ({})",
                failure.item
            )),
            (Err(why), None) => mismatches.push(format!(
                "{id} ({:?}) missed its outcome: {why}",
                outcome(id)
            )),
            _ => {}
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn the_stellar_offered_missions_128_to_499_walk() {
    walk_shard(&shards()[0]);
}

#[test]
fn the_stellar_offered_missions_500_to_655_walk() {
    walk_shard(&shards()[1]);
}

#[test]
fn the_stellar_offered_missions_656_to_799_walk() {
    walk_shard(&shards()[2]);
}

#[test]
fn the_stellar_offered_missions_800_to_999_walk() {
    walk_shard(&shards()[3]);
}

#[test]
fn the_script_started_missions_walk() {
    walk_shard(&Shard::Script);
}

#[test]
fn the_person_offered_missions_walk() {
    walk_shard(&Shard::Ship);
}
