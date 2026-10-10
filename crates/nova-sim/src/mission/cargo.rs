//! Mission cargo: the room an accept and a pickup need for it ([`Room`],
//! [`pickup_checks`]), and when it is loaded and unloaded
//! ([`loads_at_accept`], [`picks_up_at_destination`], [`drops_off`]). See
//! the [module docs](super)' "Cargo" for the original's rules.

use crate::catalog::MissionRecord;
use crate::rulebook::Rulebook;
use crate::rules::{Check, Checks, Context};

use super::active::ActiveMission;

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
            ships: crate::catalog::MissionShip { count: 1, goal },
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
        let rules = Rulebook::default();
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
