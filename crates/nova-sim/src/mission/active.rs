//! The missions accepted: the 16 slots they are held in
//! ([`ActiveMissions`], [`ActiveMission`]), the checks an accept and an
//! abort make ([`accept_checks`], [`abort_checks`]), and the pure rules a
//! mission's end reads: its pay ([`pay_effect`]) and the legal records it
//! changes ([`success_records`], [`failure_records`], [`abort_penalty`]).
//! See the [module docs](super)' "Active missions" for the original's
//! order, and the [`story`](crate::story) module's "Mission lifecycle" for
//! the events that move a mission.

use crate::catalog::{GovtId, MissionId, MissionRecord};
use crate::date::GameDate;
use crate::govt::Governments;
use crate::legal::RECORD_LIMIT;
use crate::rulebook::Rulebook;
use crate::rules::{Check, Checks, Context};

use super::aux::AuxShips;
use super::cargo::Room;
use super::offer::{Offer, Stop, after};
use super::ships::SpecialShips;

/// How many missions can be active at once: the original's slots
/// (`_currentMissions`, 0x184688).
pub const SLOTS: usize = 16;

/// `Flags`: ends at once, aborted, when nothing is left to do
/// (`_DoMissionAccept` @0xa215a, `_MissionObjectivesCheck` @0x9ed7a).
pub const AUTO_ABORT: u16 = 0x0001;
/// `Flags`: an auto-abort takes a jump's fuel (`_AutoAbortMission`
/// @0x99cc4).
pub const AUTO_ABORT_FUEL: u16 = 0x0008;
/// `Flags`: the player's abort costs 5 × `CompReward` with `CompGovt`
/// (`_DoMissionInfoDialog` @0x9e58d).
pub const ABORT_PENALTY: u16 = 0x0040;
/// `Flags2`: an auto-abort pays `PayVal` (`_AutoAbortMission` @0x99cef).
pub const AUTO_ABORT_PAYS: u16 = 0x0002;
/// The fuel an auto-abort takes with [`AUTO_ABORT_FUEL`] (100.0
/// @0xdd060).
pub const AUTO_ABORT_FUEL_TAKEN: f32 = 100.0;

/// `PayVal` below which the accept takes cash (`_DoMissionAccept`
/// @0xa1da7-0xa1dee).
const TAKES_CASH: i64 = -50_000;

/// How an active mission failed: what its return landing still runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailedBy {
    /// It failed at once, by its deadline, its cargo jettisoned or a scan
    /// finding its cargo: `_QuickMissionFailure` (@0x99d1b) ran its
    /// `OnFailure` already, and applied no legal record.
    Quick,
    /// It was marked failed, by an `F` operator (@0x1539f) or a ship goal's
    /// check (a disable goal with a ship destroyed, an escort goal with one
    /// destroyed or disabled, @0x9e8a6, @0x9e922): nothing ran yet.
    Marked,
}

/// A mission accepted into a slot, with its offer's values
/// (`_LoadCurrentMissionData` @0xa0a38). The mission's own fields (its
/// scripts, `CanAbort`, `CompGovt`, `CompReward`, `DatePostInc`, `Flags`)
/// are read from its record when used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveMission {
    /// The mission.
    pub mission: MissionId,
    /// Where it goes: none for `TravelStel` -1.
    pub destination: Option<Stop>,
    /// Where it is completed: none when it has no return stellar.
    pub return_to: Option<Stop>,
    /// The commodity or `jünk` it carries, raw.
    pub cargo_type: Option<i16>,
    /// The tons it carries.
    pub cargo_tons: i64,
    /// Its pay, `PayVal` raw.
    pub pay: i64,
    /// The days left before its deadline: none without a `TimeLimit`. The
    /// original's -32000 for "none" is never stored.
    pub days_left: Option<u32>,
    /// The last day it may be completed: the accept date plus its
    /// `TimeLimit` (@0xa1cb4-0xa1d27).
    pub deadline: Option<GameDate>,
    /// Whether its destination has been reached (+1).
    pub reached: bool,
    /// Whether its ship goals are met (+2).
    pub goals_met: bool,
    /// Whether its cargo is aboard (+0x35): loaded and not yet unloaded,
    /// jettisoned or lost with its slot. A mission with no cargo type can
    /// have it aboard too, for its `DumpCargoText`.
    pub aboard: bool,
    /// Whether it failed, and how (+3).
    pub failed: Option<FailedBy>,
    /// Its special ships: where they are, how many are left, their
    /// timer and their name (see [`ships`](super::ships)).
    pub ships: SpecialShips,
    /// Its auxiliary ships: their countdown, how many have come and how
    /// many are left to come (see [`aux`](super::aux)).
    pub aux: AuxShips,
}

impl ActiveMission {
    /// `offer` of `record` accepted on `today`: its deadline counted from
    /// today, not from the roll, its destination not yet reached, and its
    /// goals not yet met, as `_DoMissionAccept` zeroes them
    /// (@0xa1c0e-0xa1c1c) for the first check to meet them (see the
    /// session's `mission_ships` module).
    #[must_use]
    pub fn accepted(record: &MissionRecord, offer: &Offer, today: GameDate) -> Self {
        let limit = (record.time_limit > 0).then_some(record.time_limit);
        Self {
            mission: offer.mission,
            destination: offer.destination,
            return_to: offer.return_to,
            cargo_type: offer.cargo_type,
            cargo_tons: offer.cargo_tons,
            pay: offer.pay,
            days_left: limit.and_then(|days| u32::try_from(days).ok()),
            deadline: limit.map(|days| after(today, days)),
            reached: false,
            goals_met: false,
            aboard: false,
            failed: None,
            ships: SpecialShips::default(),
            aux: AuxShips::default(),
        }
    }
}

/// The 16 slots, each free or holding an active mission. A mission goes
/// into the lowest free slot, and the slot names it: it is the record in
/// its subscriptions' ids.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActiveMissions {
    slots: [Option<ActiveMission>; SLOTS],
}

impl ActiveMissions {
    /// The lowest free slot, if any (`_DoMissionAccept`
    /// @0xa1b32-0xa1b40).
    #[must_use]
    pub fn lowest_free(&self) -> Option<u8> {
        self.slots
            .iter()
            .position(Option::is_none)
            .and_then(|slot| u8::try_from(slot).ok())
    }

    /// Puts `mission` into the lowest free slot: which, or `None` when
    /// every slot is taken, `mission` dropped.
    pub fn fill(&mut self, mission: ActiveMission) -> Option<u8> {
        let slot = self.lowest_free()?;
        self.slots[usize::from(slot)] = Some(mission);
        Some(slot)
    }

    /// Puts `mission` into `slot`, whatever it held, as a save restores
    /// it: `false`, changing nothing, for a slot past the last.
    pub fn put(&mut self, slot: u8, mission: ActiveMission) -> bool {
        let Some(held) = self.slots.get_mut(usize::from(slot)) else {
            return false;
        };
        *held = Some(mission);
        true
    }

    /// The mission in `slot`, if any.
    #[must_use]
    pub fn get(&self, slot: u8) -> Option<&ActiveMission> {
        self.slots.get(usize::from(slot))?.as_ref()
    }

    /// The mission in `slot`, if any, to change.
    pub fn get_mut(&mut self, slot: u8) -> Option<&mut ActiveMission> {
        self.slots.get_mut(usize::from(slot))?.as_mut()
    }

    /// Frees `slot`: the mission it held, if any.
    pub fn free(&mut self, slot: u8) -> Option<ActiveMission> {
        self.slots.get_mut(usize::from(slot))?.take()
    }

    /// Every mission held, in slot order, with its slot.
    pub fn iter(&self) -> impl Iterator<Item = (u8, &ActiveMission)> {
        (0..=u8::MAX)
            .zip(&self.slots)
            .filter_map(|(slot, held)| Some((slot, held.as_ref()?)))
    }

    /// The slots holding `mission`, in order.
    #[must_use]
    pub fn holding(&self, mission: MissionId) -> Vec<u8> {
        self.iter()
            .filter(|(_, active)| active.mission == mission)
            .map(|(slot, _)| slot)
            .collect()
    }

    /// Whether `slot` holds `mission`.
    #[must_use]
    pub fn holds(&self, slot: u8, mission: MissionId) -> bool {
        self.get(slot)
            .is_some_and(|active| active.mission == mission)
    }

    /// Counts each mission's days left down a day, as the daily step does
    /// (@0xb590-0xb5b6): only while above none, and never one without a
    /// limit.
    pub fn count_down(&mut self) {
        for active in self.slots.iter_mut().flatten() {
            if let Some(days) = &mut active.days_left {
                *days = days.saturating_sub(1);
            }
        }
    }

    /// The tons of mission cargo aboard, as `_TotalCargo` (@0x4a32-0x4a68)
    /// counts them for the player: every slot's with its cargo aboard and
    /// its tons not below none, whatever its cargo type.
    #[must_use]
    pub fn cargo_aboard(&self) -> i64 {
        self.iter()
            .filter(|(_, active)| active.aboard && active.cargo_tons >= 0)
            .map(|(_, active)| active.cargo_tons)
            .sum()
    }

    /// [`cargo_aboard`](Self::cargo_aboard) as the tons a hold counts,
    /// held at [`u32::MAX`] rather than wrapped.
    #[must_use]
    pub fn tons_aboard(&self) -> u32 {
        u32::try_from(self.cargo_aboard()).unwrap_or(u32::MAX)
    }

    /// The missions held, in slot order, a mission held twice listed
    /// twice.
    #[must_use]
    pub fn missions(&self) -> Vec<MissionId> {
        self.iter().map(|(_, active)| active.mission).collect()
    }
}

/// What the accept checks read: the slots, the room for the offer's
/// cargo, and the rule set.
#[derive(Clone, Copy, Debug)]
pub struct AcceptFacts<'a> {
    /// The missions active.
    pub active: &'a ActiveMissions,
    /// The offer's tons, and the room the ship has for them.
    pub cargo: Room,
    /// The rule set.
    pub rules: &'a Rulebook,
}

impl Context for AcceptFacts<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.rules
    }
}

/// Why a mission is not accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AcceptRefusal {
    /// All 16 slots are taken.
    #[error("every mission slot is taken")]
    SlotsFull,
    /// The ship's holds are fewer than the cargo's tons.
    #[error("the ship's cargo holds are too few for the mission's cargo")]
    NoHolds,
    /// The ship's free cargo space is less than the cargo's tons.
    #[error("the ship's free cargo space is too little for the mission's cargo")]
    NoSpace,
}

/// Why [`Session::accept_mission`](crate::Session::accept_mission) accepted nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AcceptError {
    /// It is not offered here.
    #[error("the mission is not offered here")]
    NotOffered,
    /// An accept check refused it.
    #[error("the mission cannot be accepted: {0}")]
    Refused(AcceptRefusal),
}

/// The checks an accept makes, in order (`_DoMissionAccept` @0xa1b11): a
/// free slot; then, for an offer of tons above none, whatever its
/// `PickupMode`, the ship's holds and then the free space for them
/// (@0xa1b61-0xa1be5). Passengers are cargo like any other, and nothing
/// counts berths, so the berths are the cargo space.
#[must_use]
pub fn accept_checks<'a>() -> Checks<AcceptFacts<'a>, AcceptRefusal> {
    [
        Check::plain("slot free", "_DoMissionAccept @0xa1b32-0xa1b40", slot_free),
        Check::plain(
            "cargo holds",
            "_DoMissionAccept @0xa1b61-0xa1bbc",
            cargo_holds,
        ),
        Check::plain(
            "cargo space",
            "_DoMissionAccept @0xa1bc3-0xa1be5",
            cargo_space,
        ),
    ]
    .into()
}

fn cargo_holds(facts: &AcceptFacts) -> Result<(), AcceptRefusal> {
    if facts.cargo.holds_take() {
        Ok(())
    } else {
        Err(AcceptRefusal::NoHolds)
    }
}

fn cargo_space(facts: &AcceptFacts) -> Result<(), AcceptRefusal> {
    if facts.cargo.space_takes() {
        Ok(())
    } else {
        Err(AcceptRefusal::NoSpace)
    }
}

fn slot_free(facts: &AcceptFacts) -> Result<(), AcceptRefusal> {
    match facts.active.lowest_free() {
        Some(_) => Ok(()),
        None => Err(AcceptRefusal::SlotsFull),
    }
}

/// What the abort checks read: the mission aborted, and the rule set.
#[derive(Clone, Copy, Debug)]
pub struct AbortFacts<'a> {
    /// The mission.
    pub mission: &'a MissionRecord,
    /// The rule set.
    pub rules: &'a Rulebook,
}

impl Context for AbortFacts<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.rules
    }
}

/// Why the player cannot abort a mission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbortRefusal {
    /// Its `CanAbort` is 0.
    CannotAbort,
}

/// The checks the player's abort makes, in order
/// (`_DoMissionInfoDialog` @0x9e56d). `A` makes none.
#[must_use]
pub fn abort_checks<'a>() -> Checks<AbortFacts<'a>, AbortRefusal> {
    [Check::plain(
        "CanAbort",
        "_DoMissionInfoDialog @0x9e583",
        can_abort,
    )]
    .into()
}

fn can_abort(facts: &AbortFacts) -> Result<(), AbortRefusal> {
    if facts.mission.can_abort == 0 {
        Err(AbortRefusal::CannotAbort)
    } else {
        Ok(())
    }
}

/// Whether accepting `record` into `active` aborts it at once
/// (`_DoMissionAccept` @0xa2139-0xa217b): docked, with `Flags` 0x0001,
/// its destination reached, and no return stellar nor ship.
#[must_use]
pub fn auto_aborts_at_accept(record: &MissionRecord, active: &ActiveMission, docked: bool) -> bool {
    docked
        && record.flags & AUTO_ABORT != 0
        && active.reached
        && active.return_to.is_none()
        && record.ships.count == 0
}

/// The cash left after accepting a mission of `pay` with `cash`: a pay
/// below -50000 takes `-50000 - pay`, leaving no less than none
/// (`_DoMissionAccept` @0xa1da7-0xa1dee).
#[must_use]
pub fn accept_charge(pay: i64, cash: i64) -> i64 {
    if pay < TAKES_CASH {
        (cash + pay - TAKES_CASH).max(0)
    } else {
        cash
    }
}

/// Whose negative records a cleaning pay sets to none, beside the
/// government it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Among {
    /// That government's.
    Govt,
    /// Every government allied with it (`_GovtAllies`), itself included.
    Allies,
    /// Every government sharing a class with it (`_GovtSharedClass`),
    /// itself included.
    SharedClass,
}

impl Among {
    /// Whether `govt` is among those of `of`, the governments as `govts`
    /// say.
    #[must_use]
    pub fn takes(self, govt: GovtId, of: Option<GovtId>, govts: &Governments) -> bool {
        match self {
            Self::Govt => Some(govt) == of,
            Self::Allies => govts.allies(Some(govt), of),
            Self::SharedClass => govts.shared_class(Some(govt), of),
        }
    }
}

/// What a mission's pay does (`_ApplyMissionPay` @0x98326).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayEffect {
    /// Nothing.
    Nothing,
    /// The cash goes up by this much.
    Cash(i64),
    /// Every negative record of the governments `among` those of `of`
    /// (none for an independent one) goes to none ([`cleaned`]): the
    /// records in the systems those governments hold, hidden ones
    /// included, as the original cleans them (@0x9835e-0x9844d), and the
    /// records with those governments (see [`legal`](crate::legal)).
    Clean {
        /// The government named: `gövt` index + 128.
        of: Option<GovtId>,
        /// Whose records, beside it.
        among: Among,
    },
    /// The cash loses this percent of itself ([`fined`]).
    Fine(u8),
}

/// The government a cleaning pay's index names: index + 128, none below
/// 0.
fn named(index: i64) -> Option<GovtId> {
    let id = i16::try_from(index.checked_add(128)?).ok();
    id.filter(|_| index >= 0).map(GovtId)
}

/// What a pay of `pay` does (`_ApplyMissionPay` @0x98326): above 0, cash;
/// -19999 to -10000, the government of index `-pay - 10128`'s negative
/// records cleaned; -29999 to -20000, its allies'; -39999 to -30000,
/// those sharing a class with it; -40099 to -40001, a fine of
/// `-pay - 40000` percent; anything else nothing (a pay below -50000 is
/// taken at accept, [`accept_charge`]).
#[must_use]
pub fn pay_effect(pay: i64) -> PayEffect {
    let clean = |base: i64, among| PayEffect::Clean {
        of: named(-pay - base),
        among,
    };
    match pay {
        1.. => PayEffect::Cash(pay),
        -19_999..=-10_000 => clean(10_128, Among::Govt),
        -29_999..=-20_000 => clean(20_128, Among::Allies),
        -39_999..=-30_000 => clean(30_128, Among::SharedClass),
        -40_099..=-40_001 => {
            u8::try_from(-pay - 40_000).map_or(PayEffect::Nothing, PayEffect::Fine)
        }
        _ => PayEffect::Nothing,
    }
}

/// `cash` less `percent` percent of itself, worked in doubles and
/// truncated as the original does: trunc(cash + cash × percent × -0.01)
/// (@0x98471-0x98491; -0.01 @0xdd580).
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn fined(cash: i64, percent: u8) -> i64 {
    let cash = cash as f64;
    (cash + f64::from(percent) * -0.01 * cash).trunc() as i64
}

/// `record`, truncated and held within [`RECORD_LIMIT`].
#[allow(clippy::cast_possible_truncation)]
fn held(record: f64) -> i16 {
    let limit = f64::from(RECORD_LIMIT);
    record.trunc().clamp(-limit, limit) as i16
}

/// The records a mission's success sets among `targets`, each a record's
/// key (a system, or a government) with the government it is changed by
/// (none for an independent system), `legal` giving each one now
/// (`_DoMissionSuccess` @0xa058e-0xa0672): with a `CompGovt`, a target of
/// it gets its record plus `CompReward`; one of an enemy of it, minus
/// half, and one of an ally plus half, the record and the half summed
/// and truncated. Each is held within [`RECORD_LIMIT`] (the original
/// stores it with a bare 16-bit `movw`); one unchanged is left out, and
/// one of no government is never changed. The original walks every
/// system and changes each one's record by its government; the session
/// does so for the records in each system, and with each government for
/// the per-government records (see [`legal`](crate::legal)).
#[must_use]
pub fn success_records<K: Copy>(
    record: &MissionRecord,
    govts: &Governments,
    targets: impl IntoIterator<Item = (K, Option<GovtId>)>,
    legal: impl Fn(K) -> i16,
) -> Vec<(K, i16)> {
    let Some(comp) = record.comp_govt else {
        return Vec::new();
    };
    let reward = f64::from(record.comp_reward);
    let changes = targets.into_iter().filter_map(|(key, govt)| {
        let govt = govt?;
        if govt == comp {
            Some((key, reward))
        } else if govts.enemies(Some(govt), Some(comp)) {
            Some((key, -0.5 * reward))
        } else if govts.allies(Some(govt), Some(comp)) {
            Some((key, 0.5 * reward))
        } else {
            None
        }
    });
    changed(changes, legal)
}

/// The records a mission's failure sets among `targets` (see
/// [`success_records`]), `legal` giving each one now
/// (`_DoMissionFailure` @0xa02f9-0xa033a): with a `CompGovt`, each
/// target of it less half the `CompReward`, toward none, held within
/// [`RECORD_LIMIT`]. The original lowers the record of each system the
/// `CompGovt` holds.
#[must_use]
pub fn failure_records<K: Copy>(
    record: &MissionRecord,
    targets: impl IntoIterator<Item = (K, Option<GovtId>)>,
    legal: impl Fn(K) -> i16,
) -> Vec<(K, i16)> {
    let change = -f64::from(record.comp_reward / 2);
    changed(of_comp(record.comp_govt, targets, change), legal)
}

/// The records the player's abort sets among `targets` (see
/// [`success_records`]), `legal` giving each one now
/// (`_DoMissionInfoDialog` @0x9e58d-0x9e5cf): with `Flags` 0x0040 and a
/// `CompGovt`, each target of it less 5 × `CompReward`, held within
/// [`RECORD_LIMIT`]. The original lowers the record of each system the
/// `CompGovt` holds (@0x9e5a3-0x9e5cf).
#[must_use]
pub fn abort_penalty<K: Copy>(
    record: &MissionRecord,
    targets: impl IntoIterator<Item = (K, Option<GovtId>)>,
    legal: impl Fn(K) -> i16,
) -> Vec<(K, i16)> {
    let comp = record
        .comp_govt
        .filter(|_| record.flags & ABORT_PENALTY != 0);
    let change = -5.0 * f64::from(record.comp_reward);
    changed(of_comp(comp, targets, change), legal)
}

/// Each of `targets` of `comp`'s government with `change`: none without
/// a `comp`.
fn of_comp<K>(
    comp: Option<GovtId>,
    targets: impl IntoIterator<Item = (K, Option<GovtId>)>,
    change: f64,
) -> impl Iterator<Item = (K, f64)> {
    targets
        .into_iter()
        .filter(move |&(_, govt)| comp.is_some() && govt == comp)
        .map(move |(key, _)| (key, change))
}

/// The records among `targets` (see [`success_records`]) that a cleaning
/// pay ([`PayEffect::Clean`]) of `of`, `among` its governments, sets to
/// none, `legal` giving each one now: each negative one of a government
/// [`among`](Among::takes) those of `of` (@0x9835e-0x9844d).
#[must_use]
pub fn cleaned<K: Copy>(
    targets: impl IntoIterator<Item = (K, Option<GovtId>)>,
    legal: impl Fn(K) -> i16,
    of: Option<GovtId>,
    among: Among,
    govts: &Governments,
) -> Vec<K> {
    targets
        .into_iter()
        .filter(|&(key, govt)| {
            govt.is_some_and(|govt| among.takes(govt, of, govts)) && legal(key).is_negative()
        })
        .map(|(key, _)| key)
        .collect()
}

/// Each record, `legal` giving it now, with its change added and held:
/// those it changes.
fn changed<K: Copy>(
    changes: impl IntoIterator<Item = (K, f64)>,
    legal: impl Fn(K) -> i16,
) -> Vec<(K, i16)> {
    changes
        .into_iter()
        .filter_map(|(key, change)| {
            let now = legal(key);
            let then = held(f64::from(now) + change);
            (then != now).then_some((key, then))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtRecord, MissionShip, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::rules::Verdict;
    use crate::testkit::{govt, mission};

    fn today() -> GameDate {
        GameDate::new(1177, 6, 24).expect("a date")
    }

    fn stop(stellar: i16) -> Stop {
        Stop {
            stellar: StellarId(stellar),
            system: Some(SystemId(130)),
        }
    }

    /// An offer of mission `id` to 150 and back to 128, 7 tons of
    /// commodity 2, paying 900.
    fn offer(id: i16) -> Offer {
        Offer {
            mission: MissionId(id),
            venue: super::super::Venue::Computer,
            destination: Some(stop(150)),
            return_to: Some(stop(128)),
            cargo_type: Some(2),
            cargo_tons: 7,
            pay: 900,
            deadline: None,
        }
    }

    fn accepted(id: i16) -> ActiveMission {
        ActiveMission::accepted(&mission(id), &offer(id), today())
    }

    #[test]
    fn an_accepted_mission_holds_its_offers_values_and_a_deadline_from_today() {
        let record = MissionRecord {
            time_limit: 3,
            ..mission(140)
        };
        let active = ActiveMission::accepted(&record, &offer(140), today());
        assert_eq!(
            active,
            ActiveMission {
                mission: MissionId(140),
                destination: Some(stop(150)),
                return_to: Some(stop(128)),
                cargo_type: Some(2),
                cargo_tons: 7,
                pay: 900,
                days_left: Some(3),
                deadline: GameDate::new(1177, 6, 27),
                reached: false,
                goals_met: false,
                aboard: false,
                failed: None,
                ships: crate::mission::SpecialShips::default(),
                aux: crate::mission::aux::AuxShips::default(),
            }
        );
        for limit in [0, -1, i16::MIN] {
            let record = MissionRecord {
                time_limit: limit,
                ..mission(140)
            };
            let active = ActiveMission::accepted(&record, &offer(140), today());
            assert_eq!((active.days_left, active.deadline), (None, None), "{limit}");
        }
    }

    #[test]
    fn the_accept_and_abort_facts_hand_over_their_rule_set() {
        let rules = Rulebook::default().with_override(
            crate::rulebook::RuleKey::StartActiveMission,
            crate::rulebook::RuleSource::Bible,
        );
        let active = ActiveMissions::default();
        let record = mission(140);
        let accept = AcceptFacts {
            active: &active,
            cargo: Room::default(),
            rules: &rules,
        };
        let abort = AbortFacts {
            mission: &record,
            rules: &rules,
        };
        assert_eq!(accept.rulebook(), &rules);
        assert_eq!(abort.rulebook(), &rules);
    }

    #[test]
    fn a_mission_goes_into_the_lowest_free_slot() {
        let mut active = ActiveMissions::default();
        assert_eq!(active.lowest_free(), Some(0));
        for slot in 0..16 {
            assert_eq!(active.fill(accepted(140 + slot)), Some(slot as u8));
        }
        assert_eq!(active.lowest_free(), None);
        assert_eq!(active.fill(accepted(200)), None, "full");
        assert_eq!(active.free(3).map(|m| m.mission), Some(MissionId(143)));
        assert_eq!(active.free(3), None, "freed already");
        assert_eq!(active.free(16), None, "no such slot");
        assert_eq!(active.lowest_free(), Some(3));
        assert_eq!(active.fill(accepted(200)), Some(3));
        assert_eq!(active.get(3).map(|m| m.mission), Some(MissionId(200)));
        assert_eq!(active.get(16), None);
    }

    #[test]
    fn the_slots_list_in_order_and_find_a_mission_held() {
        let mut active = ActiveMissions::default();
        active.fill(accepted(140));
        active.fill(accepted(141));
        active.fill(accepted(140));
        active.free(1);
        assert!(active.put(15, accepted(141)));
        assert!(!active.put(16, accepted(142)), "past the last");
        assert_eq!(
            active
                .iter()
                .map(|(slot, m)| (slot, m.mission.0))
                .collect::<Vec<_>>(),
            [(0, 140), (2, 140), (15, 141)]
        );
        assert_eq!(active.holding(MissionId(140)), [0, 2]);
        assert_eq!(active.holding(MissionId(142)), Vec::<u8>::new());
        assert_eq!(
            active.missions(),
            [MissionId(140), MissionId(140), MissionId(141)]
        );
        assert!(active.holds(15, MissionId(141)));
        assert!(!active.holds(15, MissionId(140)));
        assert!(!active.holds(1, MissionId(141)));
        active.get_mut(2).expect("held").reached = true;
        assert!(active.get(2).expect("held").reached);
        assert!(active.get_mut(1).is_none());
    }

    #[test]
    fn the_cargo_aboard_is_every_slots_tons_aboard_whatever_its_type() {
        let mut active = ActiveMissions::default();
        let with = |cargo_type, cargo_tons, aboard| ActiveMission {
            cargo_type,
            cargo_tons,
            aboard,
            ..accepted(140)
        };
        assert_eq!(active.cargo_aboard(), 0, "none held");
        active.put(0, with(Some(2), 7, true));
        active.put(3, with(Some(6), 5, false));
        active.put(9, with(None, 4, true));
        active.put(12, with(Some(1), -2, true));
        active.put(15, with(Some(0), 3_000_000_000, true));
        assert_eq!(
            active.cargo_aboard(),
            3_000_000_011,
            "7 typed, 4 untyped, none for the one not aboard nor below none; \
             wider than 16 bits"
        );
    }

    #[test]
    fn the_tons_aboard_are_the_cargo_aboard_held_at_the_widest_hold() {
        let mut active = ActiveMissions::default();
        let with = |cargo_tons| ActiveMission {
            cargo_tons,
            aboard: true,
            ..accepted(140)
        };
        assert_eq!(active.tons_aboard(), 0, "none held");
        active.put(0, with(7));
        assert_eq!(active.tons_aboard(), 7);
        active.put(1, with(i64::from(u32::MAX) - 7));
        assert_eq!(active.tons_aboard(), u32::MAX, "exactly the widest");
        active.put(2, with(1));
        assert_eq!(active.tons_aboard(), u32::MAX, "held there, not wrapped");
    }

    #[test]
    fn a_day_counts_each_limit_down_to_none_and_no_further() {
        let mut active = ActiveMissions::default();
        let with = |days_left| ActiveMission {
            days_left,
            ..accepted(140)
        };
        active.put(0, with(Some(2)));
        active.put(5, with(Some(0)));
        active.put(15, with(None));
        let left =
            |active: &ActiveMissions| active.iter().map(|(_, m)| m.days_left).collect::<Vec<_>>();
        active.count_down();
        assert_eq!(left(&active), [Some(1), Some(0), None]);
        active.count_down();
        active.count_down();
        assert_eq!(left(&active), [Some(0), Some(0), None]);
    }

    #[test]
    fn sixteen_missions_refuse_an_accept_as_slots_full() {
        let rules = Rulebook::default();
        let mut active = ActiveMissions::default();
        for id in 0..16 {
            let facts = AcceptFacts {
                active: &active,
                cargo: Room::default(),
                rules: &rules,
            };
            assert_eq!(
                accept_checks().first_refusal(&facts, &mut NeverFires),
                Ok(()),
                "{id}"
            );
            active.fill(accepted(140 + id));
        }
        let facts = AcceptFacts {
            active: &active,
            cargo: Room::default(),
            rules: &rules,
        };
        let refusals: Vec<_> = accept_checks()
            .all_refusals(&facts)
            .into_iter()
            .map(|(about, verdict)| (about.name(), verdict))
            .collect();
        assert_eq!(
            refusals,
            [("slot free", Verdict::Refuses(AcceptRefusal::SlotsFull))]
        );
    }

    #[test]
    fn a_cargo_offer_is_refused_without_the_holds_then_without_the_space_for_its_tons() {
        let rules = Rulebook::default();
        let active = ActiveMissions::default();
        let refusals = |tons, holds, space| {
            let facts = AcceptFacts {
                active: &active,
                cargo: Room { tons, holds, space },
                rules: &rules,
            };
            accept_checks()
                .all_refusals(&facts)
                .into_iter()
                .map(|(about, verdict)| (about.name(), verdict))
                .collect::<Vec<_>>()
        };
        let holds = ("cargo holds", Verdict::Refuses(AcceptRefusal::NoHolds));
        let space = ("cargo space", Verdict::Refuses(AcceptRefusal::NoSpace));
        assert_eq!(refusals(10, 10, 10), [], "room for exactly its tons");
        assert_eq!(refusals(10, 9, 10), [holds]);
        assert_eq!(refusals(10, 10, 9), [space]);
        assert_eq!(refusals(10, 5, -3), [holds, space], "holds first");
        assert_eq!(refusals(0, 0, -5), [], "no tons: no check");
        assert_eq!(refusals(-1, -2, -5), [], "below none: no check");
    }

    #[test]
    fn a_full_table_refuses_before_the_cargo() {
        let rules = Rulebook::default();
        let mut active = ActiveMissions::default();
        for id in 0..16 {
            active.fill(accepted(140 + id));
        }
        let facts = AcceptFacts {
            active: &active,
            cargo: Room {
                tons: 10,
                holds: 0,
                space: 0,
            },
            rules: &rules,
        };
        assert_eq!(
            accept_checks().first_refusal(&facts, &mut NeverFires),
            Err(AcceptRefusal::SlotsFull)
        );
        let names: Vec<_> = accept_checks()
            .descriptors()
            .map(crate::rules::Descriptor::name)
            .collect();
        assert_eq!(names, ["slot free", "cargo holds", "cargo space"]);
    }

    #[test]
    fn can_abort_0_refuses_the_players_abort() {
        let rules = Rulebook::default();
        let refusals = |can_abort| {
            let record = MissionRecord {
                can_abort,
                ..mission(140)
            };
            abort_checks()
                .all_refusals(&AbortFacts {
                    mission: &record,
                    rules: &rules,
                })
                .into_iter()
                .map(|(about, verdict)| (about.name(), verdict))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            refusals(0),
            [("CanAbort", Verdict::Refuses(AbortRefusal::CannotAbort))]
        );
        assert_eq!(refusals(1), []);
        assert_eq!(refusals(-1), [], "any but 0");
    }

    #[test]
    fn an_accept_auto_aborts_only_when_every_condition_holds() {
        let record = MissionRecord {
            flags: AUTO_ABORT,
            ..mission(140)
        };
        let done = ActiveMission {
            reached: true,
            return_to: None,
            ..accepted(140)
        };
        assert!(auto_aborts_at_accept(&record, &done, true));
        assert!(!auto_aborts_at_accept(&record, &done, false), "in flight");
        let plain = MissionRecord {
            flags: 0x0002,
            ..record.clone()
        };
        assert!(!auto_aborts_at_accept(&plain, &done, true), "no 0x0001");
        let unreached = ActiveMission {
            reached: false,
            ..done.clone()
        };
        assert!(
            !auto_aborts_at_accept(&record, &unreached, true),
            "not reached"
        );
        let returning = ActiveMission {
            return_to: Some(stop(128)),
            ..done.clone()
        };
        assert!(
            !auto_aborts_at_accept(&record, &returning, true),
            "a return"
        );
        for count in [1, -1] {
            let shipped = MissionRecord {
                ships: MissionShip {
                    count,
                    goal: -1,
                    ..MissionShip::NONE
                },
                ..record.clone()
            };
            assert!(
                !auto_aborts_at_accept(&shipped, &done, true),
                "{count} ships"
            );
        }
    }

    #[test]
    fn a_pay_below_minus_50000_takes_cash_at_accept_down_to_none() {
        assert_eq!(accept_charge(-50_100, 1000), 900);
        assert_eq!(accept_charge(-50_100, 100), 0);
        assert_eq!(accept_charge(-50_100, 50), 0, "floored at none");
        assert_eq!(accept_charge(-50_100, -20), 0, "floored at none");
        assert_eq!(accept_charge(-50_000, 1000), 1000, "not below");
        assert_eq!(accept_charge(-50_000, -20), -20, "not below: no floor");
        assert_eq!(accept_charge(500, 1000), 1000);
    }

    #[test]
    fn each_pay_range_has_its_effect_at_each_boundary() {
        use Among::{Allies, Govt, SharedClass};
        let clean = |of: Option<i16>, among| PayEffect::Clean {
            of: of.map(GovtId),
            among,
        };
        for (pay, effect) in [
            (1, PayEffect::Cash(1)),
            (0, PayEffect::Nothing),
            (-9_999, PayEffect::Nothing),
            (-10_000, clean(None, Govt)),
            (-10_127, clean(None, Govt)),
            (-10_128, clean(Some(128), Govt)),
            (-19_999, clean(Some(9_999), Govt)),
            (-20_000, clean(None, Allies)),
            (-20_130, clean(Some(130), Allies)),
            (-29_999, clean(Some(9_999), Allies)),
            (-30_000, clean(None, SharedClass)),
            (-30_129, clean(Some(129), SharedClass)),
            (-39_999, clean(Some(9_999), SharedClass)),
            (-40_000, PayEffect::Nothing),
            (-40_001, PayEffect::Fine(1)),
            (-40_099, PayEffect::Fine(99)),
            (-40_100, PayEffect::Nothing),
            (-50_100, PayEffect::Nothing),
            (i64::MIN, PayEffect::Nothing),
        ] {
            assert_eq!(pay_effect(pay), effect, "{pay}");
        }
    }

    #[test]
    fn a_fine_takes_its_percent_truncating_the_sum() {
        assert_eq!(fined(1000, 10), 900);
        assert_eq!(fined(150, 1), 148, "150 - 1.5 = 148.5");
        assert_eq!(fined(-150, 1), -148, "toward none");
        assert_eq!(fined(0, 99), 0);
        assert_eq!(fined(100, 99), 1);
    }

    /// The Federation (class 1, allied with 13, at war with 9); the
    /// Civvies (class 13); the pirates (class 9); a neutral (class 4).
    fn govts() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [13, -1, -1, -1],
                enemies: [9, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                classes: [13, -1, -1, -1],
                ..govt(129)
            },
            GovtRecord {
                classes: [9, -1, -1, -1],
                ..govt(130)
            },
            GovtRecord {
                classes: [4, -1, -1, -1],
                ..govt(131)
            },
        ])
    }

    fn rewarding(reward: i16, flags: u16) -> MissionRecord {
        MissionRecord {
            comp_govt: Some(GovtId(128)),
            comp_reward: reward,
            flags,
            ..mission(140)
        }
    }

    #[test]
    fn among_takes_the_government_its_allies_or_its_class() {
        let govts = govts();
        let fed = Some(GovtId(128));
        let takes = |among: Among, of, id| among.takes(GovtId(id), of, &govts);
        assert!(takes(Among::Govt, fed, 128));
        assert!(!takes(Among::Govt, fed, 129));
        assert!(!takes(Among::Govt, None, 129), "independent keeps none");
        assert!(takes(Among::Allies, fed, 128), "itself");
        assert!(takes(Among::Allies, fed, 129));
        assert!(!takes(Among::Allies, fed, 130));
        assert!(!takes(Among::Allies, None, 129));
        assert!(takes(Among::SharedClass, fed, 128));
        assert!(!takes(Among::SharedClass, fed, 129));
        assert!(!takes(Among::SharedClass, None, 129));
    }

    /// Each of `govts` as its own target: the records with each
    /// government.
    fn each_govt(govts: &Governments) -> Vec<(GovtId, Option<GovtId>)> {
        govts.ids().map(|id| (id, Some(id))).collect()
    }

    /// The Federation alone as a target, the record with it.
    const FED_ONLY: [(GovtId, Option<GovtId>); 1] = [(GovtId(128), Some(GovtId(128)))];

    /// Systems 1 and 2 of the Federation, 3 of the pirates (its enemy), 4
    /// independent, 5 of the neutral and 6 of the Civvies (its ally).
    const SYSTEMS: [(SystemId, Option<GovtId>); 6] = [
        (SystemId(1), Some(GovtId(128))),
        (SystemId(2), Some(GovtId(128))),
        (SystemId(3), Some(GovtId(130))),
        (SystemId(4), None),
        (SystemId(5), Some(GovtId(131))),
        (SystemId(6), Some(GovtId(129))),
    ];

    /// The record in each of [`SYSTEMS`]: 0, 100, 10, -5, 0 and -30.
    fn system_legal(system: SystemId) -> i16 {
        [0, 100, 10, -5, 0, -30][usize::try_from(system.0 - 1).expect("a system")]
    }

    #[test]
    fn success_changes_each_systems_own_record_by_its_government() {
        assert_eq!(
            success_records(&rewarding(20, 0), &govts(), SYSTEMS, system_legal),
            [
                (SystemId(1), 20),
                (SystemId(2), 120),
                (SystemId(3), 0),
                (SystemId(6), -20)
            ],
            "each by its own record; the independent and the neutral left"
        );
    }

    #[test]
    fn failure_and_abort_change_every_system_of_the_comp_govt() {
        assert_eq!(
            failure_records(&rewarding(20, 0), SYSTEMS, system_legal),
            [(SystemId(1), -10), (SystemId(2), 90)]
        );
        assert_eq!(
            abort_penalty(&rewarding(20, ABORT_PENALTY), SYSTEMS, system_legal),
            [(SystemId(1), -100), (SystemId(2), 0)]
        );
    }

    #[test]
    fn cleaning_zeroes_each_negative_record_of_a_qualifying_government() {
        let govts = govts();
        let fed = Some(GovtId(128));
        let legal = |system: SystemId| match system.0 {
            1 | 3 | 4 | 6 => -7,
            _ => 4,
        };
        assert_eq!(
            cleaned(SYSTEMS, legal, fed, Among::Allies, &govts),
            [SystemId(1), SystemId(6)],
            "the Federation's and its ally's negative ones; 2 positive kept, \
             the enemy's 3 and the independent 4 untouched"
        );
        assert_eq!(
            cleaned(SYSTEMS, legal, fed, Among::Govt, &govts),
            [SystemId(1)]
        );
        assert_eq!(
            cleaned(SYSTEMS, legal, None, Among::Allies, &govts),
            [] as [SystemId; 0],
            "an independent's pay cleans nothing"
        );
        assert_eq!(
            cleaned(each_govt(&govts), |_| -1, fed, Among::SharedClass, &govts),
            [GovtId(128)],
            "by government"
        );
    }

    #[test]
    fn a_success_rewards_the_government_and_half_to_its_allies_and_enemies() {
        let govts = govts();
        let records = |legal: [i16; 4]| {
            success_records(&rewarding(5, 0), &govts, each_govt(&govts), |govt| {
                legal[usize::try_from(govt.0 - 128).expect("a govt")]
            })
        };
        assert_eq!(
            records([10, -3, 3, 7]),
            [(GovtId(128), 15), (GovtId(129), 0), (GovtId(130), 0)],
            "-3 + 2.5 truncates to 0, 3 - 2.5 to 0, and the neutral is left"
        );
        assert_eq!(
            success_records(&rewarding(1, 0), &govts, each_govt(&govts), |govt| {
                if govt.0 == 130 { 3 } else { 0 }
            }),
            [(GovtId(128), 1), (GovtId(130), 2)],
            "3 - 0.5 truncates to 2; 0 + 0.5 to 0, unchanged"
        );
        assert_eq!(
            success_records(&rewarding(1, 0), &govts, each_govt(&govts), |_| 32_000),
            [(GovtId(130), 31_999)],
            "held at the limit"
        );
        assert_eq!(
            success_records(&rewarding(i16::MAX, 0), &govts, each_govt(&govts), |_| {
                -32_000
            }),
            [(GovtId(128), 767), (GovtId(129), -15_616)],
            "no 16-bit wrap; the enemy held at the limit, unchanged"
        );
        let unrewarded = MissionRecord {
            comp_govt: None,
            ..rewarding(5, 0)
        };
        assert_eq!(
            success_records(&unrewarded, &govts, each_govt(&govts), |_| 0),
            []
        );
    }

    #[test]
    fn a_failure_takes_half_the_reward_toward_none() {
        assert_eq!(
            failure_records(&rewarding(5, 0), FED_ONLY, |_| 10),
            [(GovtId(128), 8)]
        );
        assert_eq!(
            failure_records(&rewarding(-5, 0), FED_ONLY, |_| 10),
            [(GovtId(128), 12)]
        );
        assert_eq!(
            failure_records(&rewarding(1, 0), FED_ONLY, |_| 10),
            [],
            "nothing taken"
        );
        assert_eq!(
            failure_records(&rewarding(i16::MAX, 0), FED_ONLY, |_| -20_000),
            [(GovtId(128), -32_000)],
            "held at the limit"
        );
        let unrewarded = MissionRecord {
            comp_govt: None,
            ..rewarding(5, 0)
        };
        assert_eq!(failure_records(&unrewarded, FED_ONLY, |_| 10), []);
    }

    #[test]
    fn an_abort_with_flag_0x0040_takes_five_times_the_reward() {
        assert_eq!(
            abort_penalty(&rewarding(5, ABORT_PENALTY), FED_ONLY, |_| 10),
            [(GovtId(128), -15)]
        );
        assert_eq!(
            abort_penalty(&rewarding(5, 0x0001), FED_ONLY, |_| 10),
            [],
            "no 0x0040"
        );
        assert_eq!(
            abort_penalty(&rewarding(i16::MAX, ABORT_PENALTY), FED_ONLY, |_| 0),
            [(GovtId(128), -32_000)],
            "held at the limit"
        );
        let unrewarded = MissionRecord {
            comp_govt: None,
            ..rewarding(5, ABORT_PENALTY)
        };
        assert_eq!(abort_penalty(&unrewarded, FED_ONLY, |_| 10), []);
    }
}
