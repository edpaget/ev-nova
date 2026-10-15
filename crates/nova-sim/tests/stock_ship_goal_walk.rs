//! Every stock mission with special ships and a ship goal, walked to the
//! end the original gives it: each pilot set up directly (its bits,
//! rating, record and ship), the mission offered where its `AvailLoc`
//! says (at a stellar, from a hailed person, or started by a script's
//! `S`), accepted, its goal met by the storyline walker's `meet goal` or
//! its ships flown in, and its end checked. Each is flown on a probed
//! catalog ([`Probed`]) whose `OnShipDone`, `OnSuccess`, `OnFailure` and
//! `OnAbort` for it each first set a bit of their own, so the walk tells
//! an abort apart from a completion or a failure by which of them ran.
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
use std::sync::Arc;

use nova_data::GameData;
use nova_sim::catalog::{
    self as cat, CombatCatalog, MissionId, MissionRecord, OutfitId, PilotCatalog, TrafficCatalog,
};
use nova_sim::fixture::{
    Step, StoryWalker, bits_allowing, candidates, expected_bits, expected_pay,
};
use nova_sim::mission::ShipSystem;
use nova_sim::mission::active::{
    AUTO_ABORT, AUTO_ABORT_FUEL, AUTO_ABORT_FUEL_TAKEN, AUTO_ABORT_PAYS,
};
use nova_sim::rules::Verdict;
use nova_sim::{
    Bit, Chance, GovtId, NeverFires, NovaAi, NovaPersons, Pilot, Script, Session, SetExpr, SetOp,
    StellarId, SystemId, Test, is_landable,
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

/// The missions of `failing` that `outcomes` lists too.
fn listed_in_both(failing: &[ExpectedFailure], outcomes: &[(i16, Outcome, &str)]) -> Vec<i16> {
    failing
        .iter()
        .map(|failure| failure.mission)
        .filter(|&id| outcomes.iter().any(|&(listed, ..)| listed == id))
        .collect()
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

/// What is wrong with mission `id` walking to `result`, `listed` or not:
/// an unlisted miss, or a listed mission that now walks.
fn mismatch(
    id: i16,
    result: Result<(), String>,
    listed: Option<&ExpectedFailure>,
) -> Option<String> {
    match (result, listed) {
        (Ok(()), Some(failure)) => Some(format!(
            "{id} now walks; remove it from EXPECTED_FAILURES ({})",
            failure.item
        )),
        (Err(why), None) => Some(format!(
            "{id} ({:?}) missed its outcome: {why}",
            outcome(id)
        )),
        (Ok(()), None) | (Err(_), Some(_)) => None,
    }
}

#[test]
fn a_mismatch_is_an_unlisted_miss_or_a_listed_walk() {
    let listed = ExpectedFailure {
        mission: 132,
        reason: "it cannot",
        item: "rdm:task/fix-it",
    };
    assert_eq!(mismatch(132, Ok(()), None), None);
    assert_eq!(
        mismatch(132, Ok(()), Some(&listed)),
        Some("132 now walks; remove it from EXPECTED_FAILURES (rdm:task/fix-it)".to_owned())
    );
    assert_eq!(
        mismatch(
            132,
            Err("meet goal: step 0\n      bits set: b1".to_owned()),
            None
        ),
        Some(
            "132 (Completed) missed its outcome: meet goal: step 0\n      bits set: b1".to_owned()
        )
    );
    assert_eq!(
        mismatch(132, Err("meet goal: step 0".to_owned()), Some(&listed)),
        None
    );
}

#[test]
fn a_report_keeps_every_line_nested_under_its_headline() {
    assert_eq!(
        report(&"step 0 (Take) failed\n  stellar 128 in system 128: AvailStel\n  bits set: b1"),
        "step 0 (Take) failed\n      stellar 128 in system 128: AvailStel\n      bits set: b1"
    );
    assert_eq!(report(&"one line"), "one line");
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
    assert_eq!(
        listed_in_both(EXPECTED_FAILURES, OUTCOMES),
        [0_i16; 0],
        "an expected failure is not also an outcome"
    );
}

#[test]
fn a_mission_listed_as_failing_and_as_an_outcome_is_caught() {
    let failing = [
        ExpectedFailure {
            mission: 132,
            reason: "it cannot",
            item: "rdm:task/fix-it",
        },
        ExpectedFailure {
            mission: 141,
            reason: "it cannot",
            item: "rdm:task/fix-it",
        },
    ];
    let outcomes = [
        (141, Outcome::EndsAtGoal, "ends at its goal"),
        (133, Outcome::AbortsAtSpawn, "aborts as its ships come in"),
    ];
    assert_eq!(listed_in_both(&failing, &outcomes), [141]);
    assert_eq!(listed_in_both(&failing[..1], &outcomes), [0_i16; 0]);
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
/// `extra` ask, and cash to spare. Its session reads `record` probed
/// ([`Probed`]).
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
    let probed = Probed {
        data,
        mission: record.id,
    };
    let mut session = Session::fly(&probed, docked).expect("flies");
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

/// The whole of the walker's `failure` report, each line after its
/// headline indented four more spaces so it stays nested under the
/// mismatch it ends up in.
fn report(failure: &impl std::fmt::Display) -> String {
    failure.to_string().replace('\n', "\n    ")
}

/// One of the scripts by which a mission ends, or meets its goals, that
/// the walk tells apart: the probe ([`Probed`]) has each set its own bit
/// as it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hook {
    /// `OnShipDone`, run as its goals are met.
    ShipDone,
    /// `OnSuccess`, run as it completes.
    Success,
    /// `OnFailure`, run as it fails.
    Failure,
    /// `OnAbort`, run as it is aborted.
    Abort,
}

impl Hook {
    /// Every hook.
    const ALL: [Self; 4] = [Self::ShipDone, Self::Success, Self::Failure, Self::Abort];

    /// The bit this hook's probe sets: 9990 to 9993, which no stock
    /// record uses.
    fn bit(self) -> Bit {
        let n = match self {
            Self::ShipDone => 9990,
            Self::Success => 9991,
            Self::Failure => 9992,
            Self::Abort => 9993,
        };
        Bit::new(n).expect("a bit")
    }

    /// This hook's script of `record`.
    fn script(self, record: &mut MissionRecord) -> &mut Script {
        match self {
            Self::ShipDone => &mut record.on_ship_done,
            Self::Success => &mut record.on_success,
            Self::Failure => &mut record.on_failure,
            Self::Abort => &mut record.on_abort,
        }
    }
}

/// The stock data, with `mission`'s hooks ([`Hook`]) each setting its own
/// bit before it does anything else, so the walk sees which of them ran:
/// how the mission ended, not just that its slot was freed.
struct Probed<'d> {
    /// The stock data.
    data: &'d GameData,
    /// The mission probed.
    mission: MissionId,
}

impl PilotCatalog for Probed<'_> {
    fn first_character(&self) -> Result<cat::CharacterStart, cat::StartError> {
        self.data.first_character()
    }

    fn ship_fields(&self, id: cat::ShipId) -> Result<nova_sim::ShipFields, String> {
        self.data.ship_fields(id)
    }

    fn default_outfits(&self, id: cat::ShipId) -> Vec<(OutfitId, u16)> {
        self.data.default_outfits(id)
    }

    fn outfits(&self) -> Vec<cat::OutfitRecord> {
        self.data.outfits()
    }

    fn ships(&self) -> Vec<cat::ShipRecord> {
        self.data.ships()
    }

    fn system_exists(&self, id: SystemId) -> bool {
        self.data.system_exists(id)
    }

    fn landing_sites(&self, system: SystemId) -> Vec<cat::LandingSite> {
        self.data.landing_sites(system)
    }

    fn star_map(&self) -> Vec<cat::StarSystem> {
        self.data.star_map()
    }

    fn commodity_strings(&self) -> cat::CommodityStrings {
        self.data.commodity_strings()
    }

    fn string_patch(&self, _id: i16) -> nova_sim::catalog::StringPatch {
        nova_sim::catalog::StringPatch::Absent
    }

    fn buoy_strings(&self) -> Vec<String> {
        Vec::new()
    }

    fn junk(&self) -> Vec<cat::JunkRecord> {
        self.data.junk()
    }

    fn disasters(&self) -> Vec<cat::DisasterRecord> {
        self.data.disasters()
    }

    fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
        self.data.stellar_flags()
    }

    fn date_affixes(&self) -> cat::DateAffixes {
        self.data.date_affixes()
    }

    fn gate_sites(&self) -> Vec<cat::GateSite> {
        self.data.gate_sites()
    }

    fn stellar_lives(&self) -> Vec<cat::StellarLife> {
        self.data.stellar_lives()
    }

    fn fallback_frame_size(&self) -> Option<(u32, u32)> {
        self.data.fallback_frame_size()
    }

    fn missions(&self) -> Vec<MissionRecord> {
        let mut missions = self.data.missions();
        for record in missions.iter_mut().filter(|r| r.id == self.mission) {
            for hook in Hook::ALL {
                let script = hook.script(record);
                *script = probed(script, hook.bit());
            }
        }
        missions
    }

    fn mission_texts(&self) -> Vec<cat::MissionTexts> {
        self.data.mission_texts()
    }

    fn listed_stellars(&self) -> Vec<cat::ListedStellar> {
        self.data.listed_stellars()
    }

    fn system_visibility(&self) -> Vec<cat::SystemVisibility> {
        self.data.system_visibility()
    }

    fn place_names(&self) -> cat::PlaceNames {
        self.data.place_names()
    }

    fn stellar_marks(&self) -> Vec<cat::StellarMark> {
        self.data.stellar_marks()
    }

    fn crons(&self) -> Vec<cat::CronRecord> {
        self.data.crons()
    }

    fn ranks(&self) -> Vec<cat::RankRecord> {
        self.data.ranks()
    }

    fn nebulae(&self) -> Vec<cat::NebulaRecord> {
        self.data.nebulae()
    }
}

impl CombatCatalog for Probed<'_> {
    fn weapons(&self) -> Vec<cat::WeaponRecord> {
        self.data.weapons()
    }

    fn hulls(&self) -> Vec<cat::HullRecord> {
        self.data.hulls()
    }

    fn governments(&self) -> Vec<cat::GovtRecord> {
        self.data.governments()
    }
}

/// `script` setting `bit` first, then doing what it did.
fn probed(script: &Script, bit: Bit) -> Script {
    let mut ops = vec![SetOp::Set(bit)];
    ops.extend(
        script
            .tree()
            .map(|tree| tree.ops.clone())
            .unwrap_or_default(),
    );
    Script::from(Arc::new(Ok(SetExpr { ops })))
}

/// `Ok` when, of the probed hooks, exactly `want` have run in `session`;
/// else which did.
fn ran_only(session: &Session, want: &[Hook]) -> Result<(), String> {
    let ran: Vec<Hook> = Hook::ALL
        .into_iter()
        .filter(|hook| session.control_bit(hook.bit()))
        .collect();
    let wanted: Vec<Hook> = Hook::ALL
        .into_iter()
        .filter(|hook| want.contains(hook))
        .collect();
    if ran == wanted {
        Ok(())
    } else {
        Err(format!("ran {ran:?}, not {wanted:?}"))
    }
}

#[test]
fn the_probe_sets_its_bit_then_runs_the_script() {
    let bit = Bit::new(9990).expect("a bit");
    let script = probed(&Script::parse("!b6100 G348"), bit);
    assert_eq!(
        script.tree().map(|tree| tree.ops.clone()),
        SetExpr::parse("b9990 !b6100 G348")
            .ok()
            .map(|tree| tree.ops)
    );
    let blank = probed(&Script::default(), bit);
    assert_eq!(
        blank.tree().map(|tree| tree.ops.clone()),
        Some(vec![SetOp::Set(bit)])
    );
}

#[test]
fn no_stock_record_uses_the_probe_bits() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let records: Vec<String> = data
        .missions()
        .iter()
        .map(|record| format!("{record:?}"))
        .chain(
            TrafficCatalog::persons(&data)
                .iter()
                .map(|person| format!("{person:?}")),
        )
        .chain(data.outfits().iter().map(|outfit| format!("{outfit:?}")))
        .chain(data.ships().iter().map(|ship| format!("{ship:?}")))
        .collect();
    for hook in Hook::ALL {
        let named = format!("{:?}", hook.bit());
        assert!(
            records.iter().all(|record| !record.contains(&named)),
            "{named} is used"
        );
    }
}

/// The outfits of `outfit` the ship carries.
fn outfit_count(session: &Session, outfit: OutfitId) -> u16 {
    session
        .pilot()
        .outfits()
        .find(|&(id, _)| id == outfit)
        .map_or(0, |(_, count)| count)
}

/// What a script must leave behind, the session primed so that each of
/// its effects shows: a bit it sets cleared first, one it clears set,
/// and an outfit it removes granted.
#[derive(Debug, Default)]
struct Effects {
    /// Each bit it writes, and how it leaves it.
    bits: Vec<(Bit, bool)>,
    /// Each outfit it grants or removes, and how many it leaves.
    outfits: Vec<(OutfitId, u16)>,
    /// Each mission it starts or aborts, and whether it leaves it held.
    held: Vec<(MissionId, bool)>,
}

/// Sets `key`'s value in `list` to `value`.
fn upsert<K: PartialEq, V>(list: &mut Vec<(K, V)>, key: K, value: V) {
    match list.iter_mut().find(|(held, _)| *held == key) {
        Some(entry) => entry.1 = value,
        None => list.push((key, value)),
    }
}

impl Effects {
    /// The effects of `script` as it would run in `session` now, the
    /// session primed for them; or why it cannot be.
    fn primed(session: &mut Session, script: &Script) -> Result<Self, String> {
        let mut effects = Self::default();
        let ops = script
            .tree()
            .map(|tree| tree.ops.clone())
            .unwrap_or_default();
        for op in ops {
            match op {
                SetOp::Set(bit) | SetOp::Clear(bit) => {
                    let on = matches!(op, SetOp::Set(_));
                    if effects.bits.iter().all(|&(held, _)| held != bit) {
                        session.set_control_bit(bit, !on);
                    }
                    upsert(&mut effects.bits, bit, on);
                }
                SetOp::GrantOutfit(outfit) => {
                    let count = effects.outfit(session, outfit);
                    upsert(&mut effects.outfits, outfit, count + 1);
                }
                SetOp::RemoveOutfit(outfit) => {
                    let mut count = effects.outfit(session, outfit);
                    if count == 0 {
                        session.run_set(
                            &SetExpr::parse(&format!("G{}", outfit.0)).expect("parses"),
                            &mut NeverFires,
                        );
                        count = outfit_count(session, outfit);
                        if count == 0 {
                            return Err(format!("outfit {} cannot be granted", outfit.0));
                        }
                    }
                    upsert(&mut effects.outfits, outfit, count - 1);
                }
                SetOp::StartMission(mission) => upsert(&mut effects.held, mission, true),
                SetOp::AbortMission(mission) => upsert(&mut effects.held, mission, false),
                _ => {}
            }
        }
        Ok(effects)
    }

    /// How many of `outfit` the script has left so far, else the ship's.
    fn outfit(&self, session: &Session, outfit: OutfitId) -> u16 {
        self.outfits
            .iter()
            .find(|&&(held, _)| held == outfit)
            .map_or_else(|| outfit_count(session, outfit), |&(_, count)| count)
    }

    /// `Ok` when `session` shows every effect; else those it does not.
    fn check(&self, session: &Session) -> Result<(), String> {
        let mut missing: Vec<String> = Vec::new();
        for &(bit, on) in &self.bits {
            if session.control_bit(bit) != on {
                missing.push(format!("{bit:?} {}", if on { "set" } else { "clear" }));
            }
        }
        for &(outfit, count) in &self.outfits {
            let now = outfit_count(session, outfit);
            if now != count {
                missing.push(format!("{count} of outfit {}, not {now}", outfit.0));
            }
        }
        for &(mission, held) in &self.held {
            if session.pilot().missions().holding(mission).is_empty() == held {
                missing.push(format!(
                    "{} {}",
                    mission.0,
                    if held { "held" } else { "ended" }
                ));
            }
        }
        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!("missing {}", missing.join(", ")))
        }
    }
}

/// The session as a mission's goals are about to be met: the cash and
/// fuel, the pay its slot holds, and what its `OnShipDone` must leave.
struct AtGoal {
    /// The cash before.
    cash: i64,
    /// The fuel before.
    fuel: f32,
    /// The pay the slot holds.
    pay: i64,
    /// What its `OnShipDone` must leave.
    ship_done: Effects,
}

impl AtGoal {
    /// The session as `record`'s goals are about to be met, primed for
    /// its `OnShipDone`.
    fn before(session: &mut Session, record: &MissionRecord) -> Result<Self, String> {
        let pay = session
            .pilot()
            .missions()
            .iter()
            .find(|(_, active)| active.mission == record.id)
            .map(|(_, active)| active.pay)
            .ok_or("not held")?;
        let ship_done = Effects::primed(session, &record.on_ship_done)
            .map_err(|why| format!("OnShipDone: {why}"))?;
        Ok(Self {
            cash: session.pilot().cash(),
            fuel: session.reserves().fuel.now,
            pay,
            ship_done,
        })
    }

    /// `Ok` when `record` ended by abort as its goals were met: its
    /// `OnShipDone` ran, leaving its effects, and its `OnAbort`, and
    /// neither its `OnSuccess` nor its `OnFailure`; with `Flags` 0x0001
    /// by auto-abort, which takes a jump's fuel with `Flags` 0x0008 and
    /// pays with `Flags2` 0x0002, and otherwise with neither.
    fn ended_by_abort(&self, session: &Session, record: &MissionRecord) -> Result<(), String> {
        ran_only(session, &[Hook::ShipDone, Hook::Abort])?;
        self.ship_done
            .check(session)
            .map_err(|why| format!("OnShipDone: {why}"))?;
        let auto = record.flags & AUTO_ABORT != 0;
        let paid = if auto && record.flags2 & AUTO_ABORT_PAYS != 0 {
            expected_pay(self.pay).ok_or("a fine")?
        } else {
            0
        };
        let cash = session.pilot().cash();
        if cash != self.cash + paid {
            return Err(format!("cash {} to {cash}, not paid {paid}", self.cash));
        }
        let taken = if auto && record.flags & AUTO_ABORT_FUEL != 0 {
            AUTO_ABORT_FUEL_TAKEN
        } else {
            0.0
        };
        let fuel = session.reserves().fuel.now;
        if (self.fuel - fuel - taken).abs() > 0.01 {
            return Err(format!("fuel {} to {fuel}, not less {taken}", self.fuel));
        }
        Ok(())
    }
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
                "take at {stellar:?}: {}\n      offer refusals here: {:?}",
                report(&failure),
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
    let probed = Probed {
        data,
        mission: record.id,
    };
    let mut session = Session::fly(&probed, pilot)
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

/// The `OnShipDone` bits of `record` not set in `session`.
fn unset_ship_done_bits(session: &Session, record: &MissionRecord) -> Vec<nova_sim::Bit> {
    expected_bits(&record.on_ship_done)
        .into_iter()
        .filter(|&bit| !session.control_bit(bit))
        .collect()
}

/// Meets `record`'s goal with the walker and checks it ends as `outcome`
/// says; one that ends at its goal, by abort ([`AtGoal::ended_by_abort`]).
fn meet_goal(
    data: &GameData,
    session: &mut Session,
    record: &MissionRecord,
    outcome: Outcome,
) -> Result<(), String> {
    let at_goal = match outcome {
        Outcome::EndsAtGoal => Some(AtGoal::before(session, record)?),
        _ => None,
    };
    let mut walker = StoryWalker::new(data);
    walker
        .walk(session, &[Step::MeetGoal(record.id)])
        .map_err(|failure| format!("meet goal: {}", report(&failure)))?;
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
            .map_err(|failure| format!("deliver: {}", report(&failure))),
        (Outcome::EndsAtGoal, None) => at_goal
            .ok_or("not primed")?
            .ended_by_abort(session, record)
            .map_err(|why| format!("ended at its goal: {why}")),
        (Outcome::HeldAfterGoal, Some(_)) => {
            let slot = held_after_goal(session, record).map_err(|held| format!("held: {held}"))?;
            land_held(data, session, record, slot)
        }
        (_, Some(active)) => Err(format!(
            "still held after meet goal: goals met {}, failed {:?}",
            active.goals_met, active.failed
        )),
        (_, None) => Err("ended at meet goal".into()),
    }
}

/// The slot holding `record`, its goal met, not failed and its
/// `OnShipDone` bits set; else what is off.
fn held_after_goal(session: &Session, record: &MissionRecord) -> Result<u8, String> {
    let (slot, active) = session
        .pilot()
        .missions()
        .iter()
        .find(|(_, active)| active.mission == record.id)
        .ok_or("not held")?;
    let unset = unset_ship_done_bits(session, record);
    if active.goals_met && active.failed.is_none() && unset.is_empty() {
        Ok(slot)
    } else {
        Err(format!(
            "goals met {}, failed {:?}, OnShipDone bits {unset:?} clear",
            active.goals_met, active.failed
        ))
    }
}

/// Lands `record`, held in `slot` with its goal met, at a landable
/// stellar of the system the ship is in, and checks no landing completes
/// it: still held in `slot` as [`held_after_goal`] asks, and its
/// `OnSuccess` has set none of its bits.
fn land_held(
    data: &GameData,
    session: &mut Session,
    record: &MissionRecord,
    slot: u8,
) -> Result<(), String> {
    let unset_before: Vec<_> = expected_bits(&record.on_success)
        .into_iter()
        .filter(|&bit| !session.control_bit(bit))
        .collect();
    let system = session.pilot().system();
    let stellar = data
        .landing_sites(system)
        .into_iter()
        .find(is_landable)
        .ok_or_else(|| format!("nowhere to land in {system:?}"))?
        .id;
    session
        .land_at(data, system, stellar)
        .map_err(|refusal| format!("lands at {stellar:?}: {refusal:?}"))?;
    let _ = session.settle_script(data, &mut NeverFires);
    let now = held_after_goal(session, record)
        .map_err(|held| format!("after landing at {stellar:?}: {held}"))?;
    if now != slot {
        return Err(format!(
            "after landing at {stellar:?}: held in slot {now}, not {slot}"
        ));
    }
    let ran: Vec<_> = unset_before
        .into_iter()
        .filter(|&bit| session.control_bit(bit))
        .collect();
    if ran.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "after landing at {stellar:?}: its OnSuccess ran: {ran:?} set"
        ))
    }
}

/// The traffic ticks a flown mission is given for its ships to come in.
const TICKS: u32 = 2_000;

/// The ships in `session`, by ID.
fn npc_ids(session: &Session) -> BTreeSet<nova_sim::NpcId> {
    session.npcs().iter().map(|npc| npc.id).collect()
}

/// How many ships of `record`'s special ships' `düde` are in `session`
/// and not among `before`: those that came in with the last step, its
/// own and any traffic of that `düde`.
fn came_in(
    data: &GameData,
    session: &Session,
    record: &MissionRecord,
    before: &BTreeSet<nova_sim::NpcId>,
) -> usize {
    let ships: Vec<cat::ShipId> = record
        .ships
        .dude
        .and_then(|dude| TrafficCatalog::dude(data, dude))
        .map(|dude| dude.ships.iter().map(|&(ship, _)| ship).collect())
        .unwrap_or_default();
    session
        .npcs()
        .iter()
        .filter(|npc| !before.contains(&npc.id) && ships.contains(&npc.ship))
        .count()
}

/// Flies `record`, held, until its ships come in and it auto-aborts:
/// takes off (in its ships' system, when they have one), then ticks the
/// traffic. Its ships must have come in with the step that freed its
/// slot, as they do before its auto-abort; it must have ended by its
/// `OnAbort`, leaving that script's effects (the session primed for them
/// before the take-off), and neither its `OnShipDone`, `OnSuccess` nor
/// `OnFailure` may have run.
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
    let on_abort =
        Effects::primed(session, &record.on_abort).map_err(|why| format!("OnAbort: {why}"))?;
    let mut before = npc_ids(session);
    session.take_off(data, &mut NeverFires).ok_or("takes off")?;
    let _ = session.settle_script(data, &mut NeverFires);
    let ai = NovaAi::default();
    let mut ticks = 0;
    while session.pilot().missions().get(slot).is_some() {
        if ticks == TICKS {
            return Err(format!("still held after {TICKS} ticks"));
        }
        before = npc_ids(session);
        session.tick_traffic(data, &ai, &mut NeverFires);
        let _ = session.settle_script(data, &mut NeverFires);
        ticks += 1;
    }
    let came = came_in(data, session, record, &before);
    let count = usize::try_from(record.ships.count).unwrap_or_default();
    if came < count {
        return Err(format!(
            "ended after {ticks} ticks with {came} of its {count} ships come in"
        ));
    }
    ran_only(session, &[Hook::Abort]).map_err(|why| format!("ended: {why}"))?;
    on_abort
        .check(session)
        .map_err(|why| format!("ended: OnAbort: {why}"))
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
    let mismatches: Vec<String> = members(&data, shard)
        .iter()
        .filter_map(|record| {
            let id = record.id.0;
            let listed = EXPECTED_FAILURES
                .iter()
                .find(|failure| failure.mission == id);
            mismatch(id, walk(&data, record), listed)
        })
        .collect();
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
