//! Mission cargo: the room an accept and a pickup need for it ([`Room`],
//! [`pickup_checks`]), when it is loaded and unloaded
//! ([`loads_at_accept`], [`picks_up_at_destination`], [`drops_off`]), and
//! what a jettison ([`jettison`]) and a scan ([`scan`]) do with it. See
//! the [module docs](super)' "Cargo" for the original's rules.

use crate::catalog::{GovtRecord, MissionId, MissionRecord};
use crate::chance::Chance;
use crate::rulebook::Rulebook;
use crate::rules::{Check, Checks, Context};

use super::active::{ActiveMission, ActiveMissions};
use super::screens::HIDDEN;

/// `ShipGoal`: none.
const NO_GOAL: i16 = -1;

/// `STR#` 2002 #357: a pickup refused for want of cargo holds.
pub const NO_HOLDS_TO_LOAD: &str = "Your ship doesn't have enough cargo space to load this cargo.";
/// `STR#` 2002 #358: a pickup refused for want of free cargo space.
pub const NO_SPACE_TO_LOAD: &str = "Your ship doesn't have enough free cargo space to load this cargo. Sell or jettison some and try again.";

/// `PickupMode`: the cargo is loaded as the mission is accepted.
pub const PICKUP_AT_ACCEPT: i16 = 0;
/// `PickupMode`: the cargo is loaded at the destination.
pub const PICKUP_AT_DESTINATION: i16 = 1;
/// `PickupMode`: the cargo is loaded on boarding the mission's special
/// ship.
pub const PICKUP_BOARDING: i16 = 2;
/// `DropOffMode`: the cargo is unloaded at the destination.
pub const DROP_OFF_AT_DESTINATION: i16 = 0;
/// `DropOffMode`: the cargo is unloaded at the return.
pub const DROP_OFF_AT_RETURN: i16 = 1;

/// Whether accepting `record` loads its cargo (`_DoMissionAccept`
/// @0xa1f28-0xa1f3f): with `PickupMode` 0, whatever its cargo type.
#[must_use]
pub fn loads_at_accept(record: &MissionRecord) -> bool {
    record.pickup_mode == PICKUP_AT_ACCEPT
}

/// Whether landing at its destination picks up the cargo of `active`, a
/// mission of `record` (`_MissionLandCargoCheck` @0x9ee0e-0x9ee1d):
/// with `PickupMode` 1 and nothing aboard, whatever its cargo type.
#[must_use]
pub fn picks_up_at_destination(record: &MissionRecord, active: &ActiveMission) -> bool {
    record.pickup_mode == PICKUP_AT_DESTINATION && !active.aboard
}

/// Whether a landing unloads the cargo of `active`, a mission of
/// `record`, the landing at its destination or its return as `at_destination`
/// and `at_return` say (`_MissionLandCargoCheck` @0x9ef25-0x9ef3e,
/// @0x9f00c-0x9f070): with its cargo aboard, `DropOffMode` 0 at the
/// destination, or `DropOffMode` 1 at the return once its ship goals are
/// met or it has no `ShipGoal`. (`ShipGoal` 3's escorted ships left,
/// @0x9f061-0x9f070, are rdm:phase/mission-ships/phase-2-ship-goals'.)
#[must_use]
pub fn drops_off(
    record: &MissionRecord,
    active: &ActiveMission,
    at_destination: bool,
    at_return: bool,
) -> bool {
    let at_destination = at_destination && record.drop_off_mode == DROP_OFF_AT_DESTINATION;
    let at_return = at_return
        && record.drop_off_mode == DROP_OFF_AT_RETURN
        && (active.goals_met || record.ships.goal == NO_GOAL);
    active.aboard && (at_destination || at_return)
}

/// What the player is told of a jettison, in flight (`STR#` 2002).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JettisonMessage {
    /// #284, "Mission failed.": a mission whose cargo went failed, and
    /// it lacks `Flags` 0x0400.
    MissionFailed,
    /// #289, "Cargo jettisoned.": everything went.
    CargoJettisoned,
    /// #290, "Non-mission cargo jettisoned.": the mission cargo stayed.
    NonMissionCargoJettisoned,
}

/// What a jettison does ([`jettison`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Jettisoned {
    /// The slots whose cargo goes, each mission failing at once, in slot
    /// order.
    pub slots: Vec<u8>,
    /// The tons jettisoned: the goods held and the missions' cargo, at
    /// least 1 once a mission's cargo goes.
    pub total: i64,
    /// What the player is told in flight, if anything.
    pub message: Option<JettisonMessage>,
}

/// What jettisoning does (`_JettisonCargo` @0x3dffd), with `held` tons of
/// goods held (commodities and `jünk`) and the missions in `missions`,
/// `record_of` giving each one's record; `all` also jettisons the mission
/// cargo, which Option keeps.
///
/// The total starts with the goods held (@0x3e01e, @0x3e034-0x3e055).
/// With `all` (@0x3e07a-0x3e1bc), each slot in order whose cargo is
/// aboard, typed, of tons not below none, and whose mission has a
/// `CanAbort` other than 0, loses its cargo, which adds to the total; a
/// mission that leaves the total at none makes it 1. An unabortable
/// mission's cargo stays aboard. The player is told "Mission failed." when
/// a mission that failed lacks `Flags` 0x0400 (@0x3e114-0x3e182);
/// otherwise, with a total above none, "Cargo jettisoned." or, without
/// `all`, "Non-mission cargo jettisoned." (@0x3e2fe-0x3e3b0).
#[must_use]
pub fn jettison(
    missions: &ActiveMissions,
    record_of: impl Fn(MissionId) -> MissionRecord,
    all: bool,
    held: i64,
) -> Jettisoned {
    let mut jettisoned = Jettisoned {
        total: held,
        ..Jettisoned::default()
    };
    let mut failed_shown = false;
    for (slot, active) in missions.iter().filter(|_| all) {
        let record = record_of(active.mission);
        let goes = active.aboard
            && active.cargo_type.is_some()
            && active.cargo_tons >= 0
            && record.can_abort != 0;
        if !goes {
            continue;
        }
        jettisoned.slots.push(slot);
        jettisoned.total += active.cargo_tons;
        failed_shown |= record.flags & HIDDEN == 0;
        if jettisoned.total == 0 {
            jettisoned.total = 1;
        }
    }
    jettisoned.message = if failed_shown {
        Some(JettisonMessage::MissionFailed)
    } else if jettisoned.total <= 0 {
        None
    } else if all {
        Some(JettisonMessage::CargoJettisoned)
    } else {
        Some(JettisonMessage::NonMissionCargoJettisoned)
    };
    jettisoned
}

/// `Flags`: the mission fails if its cargo is found by a scan.
pub const FAILS_IF_SCANNED: u16 = 0x0020;
/// The highest draw of `Rand(100)` with which a scan goes on
/// (`_ScanPlayer` @0x7e54c-0x7e55c).
pub const SCAN_ROLL: u32 = 75;
/// What a `ScanFine` below none takes of the cash, times its negation
/// (0.0001 @0xdda58), before it is counted in hundreds.
const SCAN_FINE_SHARE: f64 = 0.0001;

/// What a scan of the player's mission cargo comes to ([`scan`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanOutcome {
    /// Nothing is found, or nothing scanned.
    Nothing,
    /// The cargo of the mission in `slot` is found and it fails at once,
    /// the player told unless `announce` is false (its `Flags` 0x0400).
    Fail {
        /// The slot.
        slot: u8,
        /// Whether the player is told.
        announce: bool,
    },
    /// The cargo of the mission in `slot` is found: the player is told of
    /// the smuggling, and fined `credits`, none for no fine.
    Fined {
        /// The slot.
        slot: u8,
        /// The fine.
        credits: i64,
    },
}

/// What a ship of government `govt` finds scanning the player's mission
/// cargo (`_ScanPlayer` @0x7e46e), the missions in `missions`, `record_of`
/// giving each one's record, with `cash`, drawing on `chance`.
///
/// Nothing without a `SmugPenalty`, and then nothing is drawn
/// (@0x7e497-0x7e4bf); nothing on a draw of `Rand(100)` above 75
/// (@0x7e54c-0x7e55c). Otherwise the first slot in order whose cargo is
/// aboard and typed, and whose mission's `ScanMask` shares a bit with the
/// government's (@0x7e6a7-0x7e6c7), decides: a mission with `Flags`
/// 0x0020 not failed already fails (@0x7e6e2-0x7e7c9); any other is fined
/// (@0x7e7ce-0x7ea9b) its `ScanFine` above none, or below none
/// max(1, trunc(−`ScanFine` × 0.0001 × cash) × 100): that percent of the
/// cash in hundreds, at least 1. A `ScanFine` of none fines nothing.
pub fn scan(
    govt: &GovtRecord,
    missions: &ActiveMissions,
    record_of: impl Fn(MissionId) -> MissionRecord,
    cash: i64,
    chance: &mut dyn Chance,
) -> ScanOutcome {
    if govt.penalties.smuggle == 0 || chance.below(100) > SCAN_ROLL {
        return ScanOutcome::Nothing;
    }
    let found = missions.iter().find_map(|(slot, active)| {
        let record = record_of(active.mission);
        (active.aboard && active.cargo_type.is_some() && record.scan_mask & govt.scan_mask != 0)
            .then_some((slot, active, record))
    });
    let Some((slot, active, record)) = found else {
        return ScanOutcome::Nothing;
    };
    if record.flags & FAILS_IF_SCANNED != 0 && active.failed.is_none() {
        return ScanOutcome::Fail {
            slot,
            announce: record.flags & HIDDEN == 0,
        };
    }
    ScanOutcome::Fined {
        slot,
        credits: scan_fine(govt.scan_fine, cash),
    }
}

/// The credits a `ScanFine` of `fine` takes of `cash` (`_ScanPlayer`
/// @0x7e982-0x7e9c1): above none as it is; below none
/// max(1, trunc(−`fine` × 0.0001 × cash) × 100), worked in doubles.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn scan_fine(fine: i16, cash: i64) -> i64 {
    if fine >= 0 {
        return i64::from(fine);
    }
    let share = (-f64::from(fine) * SCAN_FINE_SHARE * cash as f64).trunc() as i64;
    (share * 100).max(1)
}

/// What the pickup checks read: the room for the cargo, and the rule
/// set.
#[derive(Clone, Copy, Debug)]
pub struct PickupFacts<'a> {
    /// The cargo's tons, and the room the ship has for them.
    pub cargo: Room,
    /// The rule set.
    pub rules: &'a Rulebook,
}

impl Context for PickupFacts<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.rules
    }
}

/// Why a mission's cargo is not picked up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickupRefusal {
    /// The ship's holds are fewer than the cargo's tons.
    NoHolds,
    /// The ship's free cargo space is less than the cargo's tons.
    NoSpace,
}

impl PickupRefusal {
    /// What the original says: `STR#` 2002 #357 without the holds, #358
    /// without the free space (`_DoMissionCargoPickup` @0x98292).
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Self::NoHolds => NO_HOLDS_TO_LOAD,
            Self::NoSpace => NO_SPACE_TO_LOAD,
        }
    }
}

/// The checks a pickup makes, in order (`_DoMissionCargoPickup`
/// @0x98292): for tons above none, the ship's holds, then the free space,
/// as the accept's ([`accept_checks`](super::accept_checks)).
#[must_use]
pub fn pickup_checks<'a>() -> Checks<PickupFacts<'a>, PickupRefusal> {
    [
        Check::plain(
            "cargo holds",
            "_DoMissionCargoPickup @0x98292",
            pickup_holds,
        ),
        Check::plain(
            "cargo space",
            "_DoMissionCargoPickup @0x98292",
            pickup_space,
        ),
    ]
    .into()
}

fn pickup_holds(facts: &PickupFacts) -> Result<(), PickupRefusal> {
    pass(facts.cargo.holds_take(), PickupRefusal::NoHolds)
}

fn pickup_space(facts: &PickupFacts) -> Result<(), PickupRefusal> {
    pass(facts.cargo.space_takes(), PickupRefusal::NoSpace)
}

/// `Err(refusal)` unless `ok`.
fn pass<R>(ok: bool, refusal: R) -> Result<(), R> {
    if ok { Ok(()) } else { Err(refusal) }
}

/// The room a mission's cargo needs, as an accept (`_DoMissionAccept`
/// @0xa1b61-0xa1be5) and a pickup (`_DoMissionCargoPickup` @0x98292)
/// reckon it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Room {
    /// The cargo's tons.
    pub tons: i64,
    /// The ship's own holds, with its cargo outfits (`_ShipTotalHolds`
    /// @0x5cb3).
    pub holds: i64,
    /// The room for a mission's cargo (`_TotalMissionCargoSpace`
    /// @0xcffb): the ship's own holds less what it holds that the trader
    /// escorts' holds do not take; below 0 when it holds more.
    pub space: i64,
}

impl Room {
    /// Whether the ship's holds take the tons: always for none or fewer.
    #[must_use]
    pub fn holds_take(self) -> bool {
        self.tons <= 0 || self.holds >= self.tons
    }

    /// Whether the free space takes the tons: always for none or fewer.
    #[must_use]
    pub fn space_takes(self) -> bool {
        self.tons <= 0 || self.space >= self.tons
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::mission;

    fn picking_up(pickup_mode: i16, cargo_type: i16) -> MissionRecord {
        MissionRecord {
            cargo_type,
            pickup_mode,
            ..mission(140)
        }
    }

    #[test]
    fn only_pickup_mode_0_loads_at_accept_whatever_the_cargo() {
        assert!(loads_at_accept(&picking_up(0, 0)));
        assert!(loads_at_accept(&picking_up(0, -1)), "no cargo type");
        for mode in [1, 2, -1, 3] {
            assert!(!loads_at_accept(&picking_up(mode, 0)), "{mode}");
        }
    }

    fn room(tons: i64, holds: i64, space: i64) -> Room {
        Room { tons, holds, space }
    }

    /// Mission 140 to 150 and back to 128, with no ships, picked up and
    /// dropped off as the modes say, its cargo aboard or not.
    fn carried(
        pickup_mode: i16,
        drop_off_mode: i16,
        aboard: bool,
    ) -> (MissionRecord, ActiveMission) {
        let record = MissionRecord {
            pickup_mode,
            drop_off_mode,
            ..mission(140)
        };
        let active = ActiveMission {
            aboard,
            ..crate::testkit::carrying(140, 5)
        };
        (record, active)
    }

    #[test]
    fn pickup_mode_1_picks_up_at_the_destination_with_nothing_aboard() {
        let (record, active) = carried(1, 0, false);
        assert!(picks_up_at_destination(&record, &active));
        let (record, active) = carried(1, 0, true);
        assert!(!picks_up_at_destination(&record, &active), "aboard");
        for mode in [0, 2, -1] {
            let (record, active) = carried(mode, 0, false);
            assert!(!picks_up_at_destination(&record, &active), "{mode}");
        }
    }

    #[test]
    fn drop_off_mode_0_unloads_at_the_destination_with_the_cargo_aboard() {
        let (record, active) = carried(0, 0, true);
        assert!(drops_off(&record, &active, true, false));
        assert!(drops_off(&record, &active, true, true));
        assert!(!drops_off(&record, &active, false, true), "the return");
        let (record, active) = carried(0, 0, false);
        assert!(!drops_off(&record, &active, true, false), "nothing aboard");
    }

    #[test]
    fn drop_off_mode_1_unloads_at_the_return_once_the_goals_are_met_or_there_is_none() {
        let (record, active) = carried(0, 1, true);
        assert!(drops_off(&record, &active, false, true));
        assert!(!drops_off(&record, &active, true, false), "the destination");
        let unmet = ActiveMission {
            goals_met: false,
            ..active.clone()
        };
        let goal = |goal| MissionRecord {
            ships: crate::catalog::MissionShip {
                count: 1,
                goal,
                ..crate::catalog::MissionShip::NONE
            },
            ..record.clone()
        };
        assert!(!drops_off(&goal(0), &unmet, false, true), "a goal not met");
        assert!(
            !drops_off(&goal(3), &unmet, false, true),
            "escorts: mission-ships'"
        );
        assert!(drops_off(&goal(-1), &unmet, false, true), "no goal");
        let empty = ActiveMission {
            aboard: false,
            ..active
        };
        assert!(!drops_off(&record, &empty, false, true), "nothing aboard");
    }

    #[test]
    fn drop_off_mode_minus_1_never_unloads() {
        let (record, active) = carried(0, -1, true);
        for (destination, at_return) in [(true, true), (true, false), (false, true)] {
            assert!(!drops_off(&record, &active, destination, at_return));
        }
    }

    #[test]
    fn a_pickup_is_refused_without_the_holds_then_without_the_space() {
        let rules = Rulebook::default().with_override(
            crate::rulebook::RuleKey::StartActiveMission,
            crate::rulebook::RuleSource::Bible,
        );
        let refusals = |tons, holds, space| {
            pickup_checks()
                .all_refusals(&PickupFacts {
                    cargo: room(tons, holds, space),
                    rules: &rules,
                })
                .into_iter()
                .map(|(about, verdict)| (about.name(), verdict))
                .collect::<Vec<_>>()
        };
        let holds = (
            "cargo holds",
            crate::rules::Verdict::Refuses(PickupRefusal::NoHolds),
        );
        let space = (
            "cargo space",
            crate::rules::Verdict::Refuses(PickupRefusal::NoSpace),
        );
        assert_eq!(refusals(5, 5, 5), [], "exactly");
        assert_eq!(refusals(5, 4, 5), [holds]);
        assert_eq!(refusals(5, 5, 4), [space]);
        assert_eq!(refusals(5, 4, 4), [holds, space]);
        assert_eq!(refusals(0, -1, -1), []);
        let facts = PickupFacts {
            cargo: room(5, 4, 4),
            rules: &rules,
        };
        assert_eq!(facts.rulebook(), &rules);
        assert_eq!(
            pickup_checks().first_refusal(&facts, &mut crate::chance::NeverFires),
            Err(PickupRefusal::NoHolds)
        );
    }

    /// Missions in slots 0, 2, 3 and 5: 140 with 4 tons aboard; 141
    /// unabortable with 6; 142 with 7 not aboard; 143 hidden (`Flags`
    /// 0x0400) with 3.
    fn four_missions() -> (ActiveMissions, impl Fn(MissionId) -> MissionRecord) {
        let mut missions = ActiveMissions::default();
        missions.put(0, crate::testkit::carrying(140, 4));
        missions.put(2, crate::testkit::carrying(141, 6));
        missions.put(
            3,
            ActiveMission {
                aboard: false,
                ..crate::testkit::carrying(142, 7)
            },
        );
        missions.put(5, crate::testkit::carrying(143, 3));
        let record_of = |id: MissionId| MissionRecord {
            can_abort: i16::from(id.0 != 141),
            flags: if id.0 == 143 { HIDDEN } else { 0 },
            ..mission(id.0)
        };
        (missions, record_of)
    }

    #[test]
    fn jettisoning_all_takes_every_abortable_missions_cargo_aboard_and_fails_it() {
        let (missions, record_of) = four_missions();
        assert_eq!(
            jettison(&missions, record_of, true, 10),
            Jettisoned {
                slots: vec![0, 5],
                total: 17,
                message: Some(JettisonMessage::MissionFailed),
            },
            "10 held, 4 and 3 aboard; the unabortable and the unloaded kept"
        );
    }

    #[test]
    fn with_option_only_the_goods_go() {
        let (missions, record_of) = four_missions();
        assert_eq!(
            jettison(&missions, &record_of, false, 10),
            Jettisoned {
                slots: vec![],
                total: 10,
                message: Some(JettisonMessage::NonMissionCargoJettisoned),
            }
        );
        assert_eq!(
            jettison(&missions, record_of, false, 0),
            Jettisoned::default(),
            "nothing held: nothing said"
        );
    }

    #[test]
    fn the_goods_alone_are_cargo_jettisoned() {
        let missions = ActiveMissions::default();
        assert_eq!(
            jettison(&missions, |id| mission(id.0), true, 8),
            Jettisoned {
                slots: vec![],
                total: 8,
                message: Some(JettisonMessage::CargoJettisoned),
            },
            "the commodities and jünk held"
        );
        assert_eq!(
            jettison(&missions, |id| mission(id.0), true, 0),
            Jettisoned::default()
        );
    }

    #[test]
    fn a_hidden_missions_cargo_fails_it_silently() {
        let mut missions = ActiveMissions::default();
        missions.put(1, crate::testkit::carrying(143, 3));
        let hidden = |id: MissionId| MissionRecord {
            flags: HIDDEN | 0x0001,
            ..mission(id.0)
        };
        assert_eq!(
            jettison(&missions, hidden, true, 0),
            Jettisoned {
                slots: vec![1],
                total: 3,
                message: Some(JettisonMessage::CargoJettisoned),
            }
        );
    }

    #[test]
    fn a_mission_of_no_tons_makes_a_total_of_none_one() {
        let mut missions = ActiveMissions::default();
        missions.put(0, crate::testkit::carrying(140, 0));
        missions.put(1, crate::testkit::carrying(141, 0));
        let hidden = |id: MissionId| MissionRecord {
            flags: HIDDEN,
            ..mission(id.0)
        };
        assert_eq!(
            jettison(&missions, hidden, true, 0),
            Jettisoned {
                slots: vec![0, 1],
                total: 1,
                message: Some(JettisonMessage::CargoJettisoned),
            },
            "each lifts none to 1, and 1 stays"
        );
        assert_eq!(jettison(&missions, hidden, true, 2).total, 2, "held");
    }

    #[test]
    fn untyped_or_negative_cargo_stays_aboard() {
        let mut missions = ActiveMissions::default();
        missions.put(
            0,
            ActiveMission {
                cargo_type: None,
                ..crate::testkit::carrying(140, 4)
            },
        );
        missions.put(1, crate::testkit::carrying(141, -1));
        assert_eq!(
            jettison(&missions, |id| mission(id.0), true, 0),
            Jettisoned::default()
        );
    }

    // The scan.

    use crate::catalog::{GovtRecord, Penalties};
    use crate::testkit::{Draws, govt};

    /// A government scanning for bit 0x0004, with a `SmugPenalty` of 5
    /// and a `ScanFine` of `fine`.
    fn scanning(fine: i16) -> GovtRecord {
        GovtRecord {
            penalties: Penalties {
                smuggle: 5,
                ..Penalties::default()
            },
            scan_mask: 0x0004,
            scan_fine: fine,
            ..govt(128)
        }
    }

    /// Mission 140, masked `mask`, with `Flags` `flags`, carrying 4 tons
    /// aboard in slot `slot`.
    fn smuggled(slot: u8, mask: u16, flags: u16) -> (ActiveMissions, MissionRecord) {
        let mut missions = ActiveMissions::default();
        missions.put(slot, crate::testkit::carrying(140, 4));
        let record = MissionRecord {
            scan_mask: mask,
            flags,
            ..mission(140)
        };
        (missions, record)
    }

    fn scanned(
        govt: &GovtRecord,
        missions: &ActiveMissions,
        record: &MissionRecord,
        cash: i64,
        draw: u32,
    ) -> (ScanOutcome, Vec<u32>) {
        let mut chance = Draws::of(&[draw]);
        let outcome = scan(govt, missions, |_| record.clone(), cash, &mut chance);
        (outcome, chance.asked)
    }

    #[test]
    fn a_government_with_no_smuggling_penalty_scans_nothing_and_draws_nothing() {
        let (missions, record) = smuggled(0, 0x0004, 0x0020);
        let lax = GovtRecord {
            penalties: Penalties::default(),
            ..scanning(500)
        };
        assert_eq!(
            scanned(&lax, &missions, &record, 1000, 0),
            (ScanOutcome::Nothing, vec![])
        );
    }

    #[test]
    fn a_draw_above_75_scans_nothing() {
        let (missions, record) = smuggled(0, 0x0004, 0x0020);
        assert_eq!(
            scanned(&scanning(500), &missions, &record, 1000, 76),
            (ScanOutcome::Nothing, vec![100])
        );
        assert_eq!(
            scanned(&scanning(500), &missions, &record, 1000, 75).0,
            ScanOutcome::Fail {
                slot: 0,
                announce: true
            },
            "75 scans"
        );
    }

    #[test]
    fn masked_cargo_of_a_mission_with_flag_0x0020_fails_it() {
        let (missions, record) = smuggled(3, 0x0006, 0x0020);
        assert_eq!(
            scanned(&scanning(500), &missions, &record, 1000, 0).0,
            ScanOutcome::Fail {
                slot: 3,
                announce: true
            }
        );
        let (missions, hidden) = smuggled(3, 0x0004, 0x0420);
        assert_eq!(
            scanned(&scanning(500), &missions, &hidden, 1000, 0).0,
            ScanOutcome::Fail {
                slot: 3,
                announce: false
            },
            "0x0400: silently"
        );
    }

    #[test]
    fn a_mask_sharing_no_bit_scans_nothing() {
        let (missions, record) = smuggled(0, 0x0003, 0x0020);
        assert_eq!(
            scanned(&scanning(500), &missions, &record, 1000, 0).0,
            ScanOutcome::Nothing
        );
    }

    #[test]
    fn cargo_not_aboard_or_untyped_is_not_found() {
        let (mut missions, record) = smuggled(0, 0x0004, 0x0020);
        missions.get_mut(0).expect("held").aboard = false;
        assert_eq!(
            scanned(&scanning(500), &missions, &record, 1000, 0).0,
            ScanOutcome::Nothing
        );
        let (mut missions, record) = smuggled(0, 0x0004, 0x0020);
        missions.get_mut(0).expect("held").cargo_type = None;
        assert_eq!(
            scanned(&scanning(500), &missions, &record, 1000, 0).0,
            ScanOutcome::Nothing
        );
    }

    #[test]
    fn masked_cargo_of_any_other_mission_is_fined() {
        let (missions, record) = smuggled(2, 0x0004, 0);
        assert_eq!(
            scanned(&scanning(500), &missions, &record, 1000, 0).0,
            ScanOutcome::Fined {
                slot: 2,
                credits: 500
            }
        );
        assert_eq!(
            scanned(&scanning(0), &missions, &record, 1000, 0).0,
            ScanOutcome::Fined {
                slot: 2,
                credits: 0
            },
            "no fine, still found"
        );
        let (mut failed, record) = smuggled(2, 0x0004, 0x0020);
        failed.get_mut(2).expect("held").failed = Some(super::super::FailedBy::Quick);
        assert_eq!(
            scanned(&scanning(500), &failed, &record, 1000, 0).0,
            ScanOutcome::Fined {
                slot: 2,
                credits: 500
            },
            "failed already: fined"
        );
    }

    #[test]
    fn a_fine_below_none_is_that_percent_of_the_cash_in_hundreds_at_least_1() {
        let (missions, record) = smuggled(0, 0x0004, 0);
        let fine =
            |scan_fine, cash| match scanned(&scanning(scan_fine), &missions, &record, cash, 0).0 {
                ScanOutcome::Fined { credits, .. } => credits,
                other => panic!("{other:?}"),
            };
        assert_eq!(fine(-5, 123_456), 6100, "5% of 123,456 is 6,172.8");
        assert_eq!(fine(-10, 1000), 100);
        assert_eq!(fine(-10, 999), 1, "9.99 in hundreds is none: 1");
        assert_eq!(fine(-1, 0), 1);
        assert_eq!(fine(-100, 5_000_000_000), 5_000_000_000, "no 32-bit wrap");
    }

    #[test]
    fn only_the_first_masked_slot_is_acted_on() {
        let mut missions = ActiveMissions::default();
        missions.put(1, crate::testkit::carrying(141, 2));
        missions.put(4, crate::testkit::carrying(140, 4));
        missions.put(6, crate::testkit::carrying(142, 4));
        let record_of = |id: MissionId| MissionRecord {
            scan_mask: if id.0 == 141 { 0x0001 } else { 0x0004 },
            flags: if id.0 == 140 { 0 } else { 0x0020 },
            ..mission(id.0)
        };
        let outcome = scan(
            &scanning(500),
            &missions,
            record_of,
            1000,
            &mut Draws::of(&[0]),
        );
        assert_eq!(
            outcome,
            ScanOutcome::Fined {
                slot: 4,
                credits: 500
            },
            "141 shares no bit; 140 is first, and 142 is left"
        );
    }

    #[test]
    fn a_refused_pickup_says_the_originals_words() {
        assert_eq!(
            PickupRefusal::NoHolds.message(),
            "Your ship doesn't have enough cargo space to load this cargo."
        );
        assert_eq!(
            PickupRefusal::NoSpace.message(),
            "Your ship doesn't have enough free cargo space to load this cargo. Sell or jettison some and try again."
        );
    }

    #[test]
    fn the_holds_and_the_space_each_take_up_to_their_tons() {
        assert!(room(10, 10, 10).holds_take(), "exactly");
        assert!(room(10, 10, 10).space_takes(), "exactly");
        assert!(!room(10, 9, 10).holds_take());
        assert!(room(10, 9, 10).space_takes());
        assert!(!room(10, 10, 9).space_takes());
        assert!(room(10, 10, 9).holds_take());
    }

    #[test]
    fn no_tons_need_no_room() {
        for tons in [0, -1, i64::MIN] {
            assert!(room(tons, -5, -5).holds_take(), "{tons}");
            assert!(room(tons, -5, -5).space_takes(), "{tons}");
        }
        assert!(!room(1, 0, 0).holds_take());
        assert!(!room(1, 0, 0).space_takes());
    }
}
