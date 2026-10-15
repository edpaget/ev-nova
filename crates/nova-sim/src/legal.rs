//! The player's legal record: what the player's crimes against a ship do
//! to it, and the records it is kept in.
//!
//! The [`LegalCode`] port says how a crime changes the records; Nova's is
//! [`NovaLaw`], and [`convict`] applies what it says to a pilot.
//!
//! # Two stores, one reading
//!
//! The original keeps `_playerRecord`, one record a system (2048 `i16`
//! at 0x230760, the save keeping all of them, `_WritePilotData`
//! @0x72420, `_LoadPilotData` @0x75567). The pilot keeps it so too, the
//! record in each system, and beside it the record with each government,
//! the engine's earlier store. Every writer changes both, and the
//! [`Rulebook`]'s [`RuleKey::LegalRecords`] entry says which one the
//! readers read (the markup, the hail and the AI through the session's
//! system record, landing and a mission's `AvailRecord`): by the engine,
//! the record in the player's system, whatever the landed stellar's
//! government, as the original reads it (`_HandlePlayerDockRequest`
//! @0x67903, `_IsMissionAvailable` @0x9b59d, `_DoTradeDialog` @0x5dc17);
//! by the Bible, the record with the system's government (at a landing,
//! the stellar's, [`landing_record`](crate::landing::landing_record)).
//!
//! The records in each system are derived when the pilot first flies,
//! for a new pilot and for a save from before they were kept
//! ([`systems_from_govts`]): each system gets its government's record,
//! else that government's `InitialRec`, and none when independent, as
//! the original's `_ResetPlayer` seeds them (@0x1d9a4-0x1da0b). A
//! government override leaves each system's record as it is; only later
//! writes read the new government (the original never writes a system's
//! government in play, `_systemData` +0x8). The escape pod's rescue seeds
//! them again the same way, from each government's `InitialRec` alone,
//! hidden systems included, and sets the record with each government to
//! it too (`_HandlePlayer` @0x69257-0x69290; the session's `rescue`
//! module). The `chär`'s statuses spread to allied and enemy systems
//! (`_ActivateCharResource` @0x76dc0) and the readers the engine lacks
//! belong to
//! rdm:phase/missions-fidelity/phase-23-legal-record-other-readers-and-writers.
//!
//! The mission writers (success, failure, abort and the cleaning pay,
//! see [`mission`](crate::mission)) change the record in every system of
//! a qualifying government, hidden ones included, as the original's
//! loops over every slot do; the clean-record outfit cleans the visible
//! ones (see [`outfit_effects`](crate::outfit_effects)).
//!
//! # Crimes
//!
//! The original (`_SlapWithPenalty` @0x9bbc and `_RecursivePenaltySlap`
//! @0x980a in the `EV Nova` executable) changes, for a crime against a
//! ship of government V, the record in every system reached from the
//! crime's, along the hyperlinks, by a factor of 0.65 a jump: the record
//! in each system ([`LegalCode::system_penalties`], **The walk** below).
//! The record with each government ([`LegalCode::penalties`]) takes the
//! same rules, whole, undecayed and for every government in the
//! catalog: one function gives both stores the change for a government
//! S, and only the independent system's rules are the walk's own. With P the crime's penalty field (`SmugPenalty`,
//! `DisabPenalty`, `BoardPenalty`, `KillPenalty`: types 0 to 3,
//! `_RecursivePenaltySlap` reading gövt +0x48 + 2×type @0x9954), a
//! crime against V changes the record with government S, or in a system
//! of government S, by:
//!
//! - **-V.P** when S is V, or allied with V and not its enemy (@0x998d,
//!   @0x9a23);
//! - **+0.5 S.P**, S's own penalty halved, when S is not allied with V,
//!   whether its enemy or neutral (@0x9a51; 0.5 @0xdd128). That is the
//!   engine's rule, and the default. The Bible's ("evil deeds to one
//!   government will improve your rating with its enemies... allied
//!   governments also communicate your actions") gives +0.5 S.P only to
//!   an enemy of V, and leaves a neutral alone. Which applies is the
//!   [`Rulebook`]'s [`RuleKey::CrimeGains`] entry;
//! - for an independent victim, **+0.5 S.P** when S is xenophobic, else
//!   **-0.5 S.P** when S is nosy (`Flags` 0x0002) (@0x999a-0x99c8);
//! - and nothing otherwise.
//!
//! In an independent system (the walk alone), the change is **+0.5 V.P**
//! for a xenophobic V, **-0.25 V.P** for any other ([`INDEPENDENT_SHARE`],
//! 0.25 @0xdd130), and **-0.5 P** of the first government, ID 128, for an
//! independent victim (@0x9945-0x9988; Bible Appendix II, "if the system
//! is independent, it is based on the first government's"), none when
//! there is no 128.
//!
//! A change to the record with a government is truncated and skipped when
//! below 1 either way. A derelict victim's government (`Flags` 0x0800)
//! changes nothing (@0x9bf2), and neither does [`Crime::Shoot`]: the
//! original never passes `ShootPenalty`, as the Bible says ("currently
//! ignored"). Every record is held within [`RECORD_LIMIT`] either way
//! (@0x9ad1-0x9b1e).
//!
//! # The walk
//!
//! `_RecursivePenaltySlap` walks depth-first from the player's system,
//! where every caller of `_SlapWithPenalty` says the crime is (player
//! +0x74), keeping the systems walked (a 0x800-byte array cleared for
//! each crime, @0x9bfd-0x9c0e) and stopping at one walked already
//! (@0x9829-0x9857). At each system it changes its place's chain: every
//! catalog system at the same map position, hidden or not, by ascending
//! ID, the head first ([`CrimeMap::chain`]; `+0xba`/`+0xb8`, built by
//! `_LoadObjectData` before any system is hidden, @0x783d9-0x78517). Each
//! member counts as walked, but only a visible one takes a change
//! (@0x98f3).
//!
//! A member's change, the rules above times the factor (1 from the
//! crime's system), is taken in single precision (@0x9a86-0x9a93) and
//! skipped when strictly between -1 and 1, so a change of 1 either way
//! applies (@0x9a97-0x9aa9). The new record is the record plus the change,
//! truncated ([`convict`]). A member counts as changed whenever its
//! change applied, the limit holding the record or not (@0x9b18).
//!
//! Only when a member of the chain changed does the walk go on: along the
//! Con slots of the system it was passed (not the chain's other
//! members'), in order, each to the head of that link's place, at the
//! factor times [`DECAY`] (0.65 @0xdd138, @0x9b41-0x9bb2). A link to a
//! system not in the catalog changes nothing. So a place of hidden
//! systems, or one whose change is under 1, ends the walk there.
//!
//! The walk never reads the records, so the ten slaps of a stellar's kill
//! walk the same systems ten times, each conviction truncating in turn.
//! Neither function plays a sound.
//!
//! A crime against a mission's special ship changes no record. The
//! original's callers pass the slot of the ship's mission mark, and
//! `_SlapWithPenalty` returns before the walk unless it is -1 (@0x9c10),
//! reading the mark alone: a ship of a mission active, failed, or whose
//! slot is freed with the mark not yet cleared, escapes it. Disabling,
//! destroying and boarding pass the mark (`_DamageShip` @0x3adb3,
//! @0x3ae08; `_HandlePlayerBoardAttempt` @0x6582b); a stellar's kill,
//! the scans and the planet hails pass -1. The same return skips the
//! ranks the crime would take (`_DeactivateRank` @0x9c9e; see
//! [`rank`](crate::rank)'s **Crimes**).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

use crate::catalog::{GovtId, SystemId};
use crate::govt::{Governments, NOSY};
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// How far either way of none a legal record goes.
pub const RECORD_LIMIT: i32 = 32_000;
/// What a pleased government's (or a nosy one's) own penalty counts for.
pub const ENEMY_SHARE: f32 = 0.5;
/// What the victim's penalty counts for in an independent system, unless
/// the victim is xenophobic or independent (0xdd130).
pub const INDEPENDENT_SHARE: f32 = 0.25;
/// What a crime's change is multiplied by a jump along the walk (0xdd138).
pub const DECAY: f64 = 0.65;
/// The government whose penalty an independent victim's crime in an
/// independent system reads: the first, gövt slot 0 (@0x994c-0x9959).
const FIRST_GOVT: GovtId = GovtId(128);

/// A crime against a ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crime {
    /// Carrying contraband its scan finds (`SmugPenalty`): the original's
    /// type 0 (`_ScanPlayer` @0x7ebcc, @0x7f01f; see the
    /// [`scan`](crate::scan) module).
    Smuggle,
    /// Disabling it (`DisabPenalty`).
    Disable,
    /// Boarding it (`BoardPenalty`).
    Board,
    /// Destroying it (`KillPenalty`).
    Kill,
    /// Shooting it (`ShootPenalty`).
    Shoot,
}

/// What a crime's walk reads of the galaxy (see the module docs' **The
/// walk**).
pub trait CrimeMap {
    /// Every system at `system`'s place, `system` included, by ascending
    /// ID, hidden or not: the original's chain, its head first. Empty for
    /// a system not in the catalog.
    fn chain(&self, system: SystemId) -> Vec<SystemId>;
    /// `system`'s Con slots as the catalog reads them, in order,
    /// unresolved, hidden links and repeats kept. Empty for a system not
    /// in the catalog.
    fn links(&self, system: SystemId) -> &[SystemId];
    /// Whether `system`, one of a chain, is visible.
    fn visible(&self, system: SystemId) -> bool;
    /// `system`'s government, `None` when it is independent.
    fn govt(&self, system: SystemId) -> Option<GovtId>;
}

/// How the player's crimes change the legal records.
pub trait LegalCode: Debug {
    /// The change to the record with each government that `crime`
    /// against a ship of `victim` (or an independent one) makes, with the
    /// governments in `govts`: each change already truncated, none of
    /// them none.
    fn penalties(
        &self,
        crime: Crime,
        victim: Option<GovtId>,
        govts: &Governments,
    ) -> Vec<(GovtId, i32)>;

    /// The change to the record in each system that `crime` against a
    /// ship of `victim` (or an independent one), committed in `from`,
    /// makes along the walk through `map`, with the governments in
    /// `govts`: in the order the walk reaches them, each a record delta
    /// (below none is worse), already decayed and at least 1 either way.
    fn system_penalties(
        &self,
        crime: Crime,
        victim: Option<GovtId>,
        from: SystemId,
        map: &dyn CrimeMap,
        govts: &Governments,
    ) -> Vec<(SystemId, f32)>;
}

/// Nova's law (see the module docs): the engine's by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NovaLaw {
    /// Which governments a crime against a ship of government V improves
    /// the record with, by half their own penalty: by the engine, every
    /// government not allied with V; by the Bible, only V's enemies.
    pub crime_gains: RuleSource,
}

impl NovaLaw {
    /// The law `rulebook` chooses: its [`RuleKey::CrimeGains`] entry.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            crime_gains: rulebook.source_for(RuleKey::CrimeGains),
        }
    }

    /// The change, untruncated and undecayed, that `crime` against a ship
    /// of `victim` (or an independent one) makes to the record with
    /// government `s`, or in a system of `s` (see the module docs'
    /// **Crimes**). Both stores read it: [`LegalCode::penalties`]
    /// truncates it, and the walk decays it.
    fn govt_change(
        self,
        crime: Crime,
        victim: Option<GovtId>,
        s: GovtId,
        govts: &Governments,
    ) -> f32 {
        let s = Some(s);
        let penalty = |govt: Option<GovtId>| f32::from(govts.penalty(govt, crime));
        match victim {
            Some(_) if govts.enemies(victim, s) => ENEMY_SHARE * penalty(s),
            // V itself too (@0x992e): a government is its own ally, and
            // never its own enemy.
            Some(_) if govts.allies(victim, s) => -penalty(victim),
            Some(_) if self.crime_gains == RuleSource::Engine => ENEMY_SHARE * penalty(s),
            None if govts.xenophobic(s) => ENEMY_SHARE * penalty(s),
            None if govts.flag(s, NOSY) => -ENEMY_SHARE * penalty(s),
            // By the Bible, a neutral is left alone.
            Some(_) | None => 0.0,
        }
    }
}

impl LegalCode for NovaLaw {
    fn penalties(
        &self,
        crime: Crime,
        victim: Option<GovtId>,
        govts: &Governments,
    ) -> Vec<(GovtId, i32)> {
        if crime == Crime::Shoot || govts.derelict(victim) {
            return Vec::new();
        }
        govts
            .ids()
            .filter_map(|id| {
                // Truncated: a change of less than 1 either way is none.
                let change = self.govt_change(crime, victim, id, govts).trunc() as i32;
                (change != 0).then_some((id, change))
            })
            .collect()
    }

    fn system_penalties(
        &self,
        crime: Crime,
        victim: Option<GovtId>,
        from: SystemId,
        map: &dyn CrimeMap,
        govts: &Governments,
    ) -> Vec<(SystemId, f32)> {
        if crime == Crime::Shoot || govts.derelict(victim) {
            return Vec::new();
        }
        let mut walk = Walk {
            law: *self,
            crime,
            victim,
            map,
            govts,
            visited: BTreeSet::new(),
            changes: Vec::new(),
        };
        walk.visit(from, 1.0);
        walk.changes
    }
}

/// One crime's walk through the galaxy, `_RecursivePenaltySlap`
/// (@0x980a; see the module docs' **The walk**).
struct Walk<'a> {
    law: NovaLaw,
    crime: Crime,
    victim: Option<GovtId>,
    map: &'a dyn CrimeMap,
    govts: &'a Governments,
    /// The systems walked, the original's array at 0xde93c.
    visited: BTreeSet<SystemId>,
    /// Each change made, in order.
    changes: Vec<(SystemId, f32)>,
}

impl Walk<'_> {
    /// Walks `system`'s place at `factor`, and on from `system`'s own
    /// links when the place changed.
    fn visit(&mut self, system: SystemId, factor: f64) {
        if !self.visited.insert(system) {
            return;
        }
        let mut changed = false;
        for member in self.map.chain(system) {
            self.visited.insert(member);
            if !self.map.visible(member) {
                continue;
            }
            // The base (exact in single precision) widened, decayed and
            // narrowed again, as @0x9a8a-0x9a93 does.
            let change = (f64::from(self.base(member)) * factor) as f32;
            if change > -1.0 && change < 1.0 {
                continue;
            }
            self.changes.push((member, change));
            changed = true;
        }
        if !changed {
            return;
        }
        for &link in self.map.links(system) {
            let head = self.map.chain(link).first().copied().unwrap_or(link);
            self.visit(head, factor * DECAY);
        }
    }

    /// The undecayed change to the record in `system` (@0x9900-0x9a86):
    /// in a governed system, its government's change, as the record with
    /// it takes ([`NovaLaw::govt_change`]); the independent system's rules
    /// are the walk's alone.
    fn base(&self, system: SystemId) -> f32 {
        let govts = self.govts;
        let victim = self.victim;
        if let Some(owner) = self.map.govt(system) {
            return self.law.govt_change(self.crime, victim, owner, govts);
        }
        let penalty = |govt: Option<GovtId>| f32::from(govts.penalty(govt, self.crime));
        match victim {
            None => -ENEMY_SHARE * penalty(Some(FIRST_GOVT)),
            Some(_) if govts.xenophobic(victim) => ENEMY_SHARE * penalty(victim),
            Some(_) => -INDEPENDENT_SHARE * penalty(victim),
        }
    }
}

/// The record in each of `systems` (each with its government) derived
/// from the records with each government: the system's government's
/// record if it has one, else that government's `InitialRec`, and none in
/// an independent system, as the original's `_ResetPlayer` seeds a new
/// pilot's (@0x1d9a4-0x1da0b). A system whose record is none is left
/// out.
#[must_use]
pub fn systems_from_govts(
    systems: impl IntoIterator<Item = (SystemId, Option<GovtId>)>,
    legal: &BTreeMap<GovtId, i16>,
    govts: &Governments,
) -> BTreeMap<SystemId, i16> {
    systems
        .into_iter()
        .filter_map(|(system, govt)| {
            let govt = govt?;
            let record = legal
                .get(&govt)
                .copied()
                .unwrap_or_else(|| govts.initial_record(Some(govt)));
            (record != 0).then_some((system, record))
        })
        .collect()
}

/// Applies one conviction to `pilot`: `govt_changes` to its records with
/// each government, and `system_changes` (a [`LegalCode`]'s walk) to its
/// record in each system. A system's new record is its record plus the
/// change in single precision, truncated (`cvttss2si` of the float sum,
/// @0x9aab-0x9b1e), so a change's fraction counts. Every record is held
/// within [`RECORD_LIMIT`].
pub fn convict(
    pilot: &mut Pilot,
    govt_changes: &[(GovtId, i32)],
    system_changes: &[(SystemId, f32)],
) {
    let held = |record: i32| {
        let record = record.clamp(-RECORD_LIMIT, RECORD_LIMIT);
        i16::try_from(record).unwrap_or_default()
    };
    for &(govt, change) in govt_changes {
        let record = i32::from(pilot.legal_record(govt)) + change;
        pilot.set_legal_record(govt, held(record));
    }
    for &(system, change) in system_changes {
        let record = (f32::from(pilot.system_record(system)) + change) as i32;
        pilot.set_system_record(system, held(record));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtRecord, Penalties};
    use crate::govt::{DERELICT, XENOPHOBIC};
    use crate::testkit::govt;

    const VICTIM: GovtId = GovtId(130);
    const ALLY: GovtId = GovtId(131);
    const ENEMY: GovtId = GovtId(132);
    const NEUTRAL: GovtId = GovtId(133);
    const XENOPHOBE: GovtId = GovtId(134);
    const NOSY_ONE: GovtId = GovtId(135);
    const WRECKS: GovtId = GovtId(136);

    /// `govt` with these disable, board, kill and shoot penalties.
    fn penalised(id: i16, [disable, board, kill, shoot]: [i16; 4]) -> GovtRecord {
        GovtRecord {
            penalties: Penalties {
                smuggle: 99,
                disable,
                board,
                kill,
                shoot,
            },
            ..govt(id)
        }
    }

    /// The victim (class 1), its ally (class 2, allied with 1), its enemy
    /// (at war with class 1) and a neutral, each with its own penalties.
    fn four() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..penalised(130, [3, 5, 7, 2])
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                allies: [1, -1, -1, -1],
                ..penalised(131, [11, 13, 17, 19])
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..penalised(132, [8, 6, 10, 4])
            },
            penalised(133, [20, 20, 20, 20]),
        ])
    }

    const ENGINE: NovaLaw = NovaLaw {
        crime_gains: RuleSource::Engine,
    };
    const BIBLE: NovaLaw = NovaLaw {
        crime_gains: RuleSource::Bible,
    };

    /// A hand-built galaxy for the walk: each system's place, links,
    /// government and visibility.
    #[derive(Debug, Default)]
    struct FakeMap {
        systems: BTreeMap<SystemId, Row>,
    }

    #[derive(Debug)]
    struct Row {
        place: u8,
        links: Vec<SystemId>,
        govt: Option<GovtId>,
        visible: bool,
    }

    impl FakeMap {
        /// Adds visible system `id` at `place`, linked to `links`, of
        /// `govt`.
        fn with(mut self, id: i16, place: u8, links: &[i16], govt: Option<GovtId>) -> Self {
            self.systems.insert(
                SystemId(id),
                Row {
                    place,
                    links: links.iter().copied().map(SystemId).collect(),
                    govt,
                    visible: true,
                },
            );
            self
        }

        /// Hides system `id`.
        fn hiding(mut self, id: i16) -> Self {
            if let Some(row) = self.systems.get_mut(&SystemId(id)) {
                row.visible = false;
            }
            self
        }
    }

    impl CrimeMap for FakeMap {
        fn chain(&self, system: SystemId) -> Vec<SystemId> {
            let Some(place) = self.systems.get(&system).map(|row| row.place) else {
                return Vec::new();
            };
            self.systems
                .iter()
                .filter(|(_, row)| row.place == place)
                .map(|(&id, _)| id)
                .collect()
        }

        fn links(&self, system: SystemId) -> &[SystemId] {
            self.systems
                .get(&system)
                .map_or(&[], |row| row.links.as_slice())
        }

        fn visible(&self, system: SystemId) -> bool {
            self.systems.get(&system).is_some_and(|row| row.visible)
        }

        fn govt(&self, system: SystemId) -> Option<GovtId> {
            self.systems.get(&system).and_then(|row| row.govt)
        }
    }

    /// `law`'s walk of disabling a ship of `victim` in `from`.
    fn walk(
        law: NovaLaw,
        victim: Option<GovtId>,
        from: i16,
        map: &FakeMap,
        govts: &Governments,
    ) -> Vec<(i16, f32)> {
        law.system_penalties(Crime::Disable, victim, SystemId(from), map, govts)
            .into_iter()
            .map(|(system, change)| (system.0, change))
            .collect()
    }

    /// The victim's government alone, with a disabling penalty of
    /// `penalty`.
    fn alone(penalty: i16) -> Governments {
        Governments::new([penalised(130, [penalty, 0, 0, 0])])
    }

    /// Whether `walked` changes the same systems as `expected`, in the
    /// same order, each by the same delta within a float's rounding.
    fn assert_walk(walked: &[(i16, f32)], expected: &[(i16, f32)], why: &str) {
        let systems =
            |changes: &[(i16, f32)]| changes.iter().map(|&(id, _)| id).collect::<Vec<_>>();
        assert_eq!(systems(walked), systems(expected), "{why}: {walked:?}");
        for (&(id, got), &(_, want)) in walked.iter().zip(expected) {
            assert!(
                (got - want).abs() < 1e-4,
                "{why}: system {id} took {got}, not {want}"
            );
        }
    }

    #[test]
    fn a_crime_lowers_the_record_in_the_victims_systems_decaying_by_065_a_jump() {
        let map = (1..=8).fold(FakeMap::default(), |map, id| {
            map.with(id, id as u8, &[id - 1, id + 1], Some(VICTIM))
        });
        assert_walk(
            &walk(ENGINE, Some(VICTIM), 1, &map, &alone(10)),
            &[
                (1, -10.0),
                (2, -6.5),
                (3, -4.225),
                (4, -2.746_25),
                (5, -1.785_062_5),
                (6, -1.160_290_6),
            ],
            "0.754 in 7 skipped, and the walk ends there",
        );
    }

    #[test]
    fn the_walk_is_depth_first() {
        let map = FakeMap::default()
            .with(1, 1, &[2, 3], Some(VICTIM))
            .with(2, 2, &[3], Some(VICTIM))
            .with(3, 3, &[], Some(VICTIM));
        assert_walk(
            &walk(ENGINE, Some(VICTIM), 1, &map, &alone(10)),
            &[(1, -10.0), (2, -6.5), (3, -4.225)],
            "3 reached through 2 first, at 0.65 squared, and changed once",
        );
    }

    #[test]
    fn hidden_systems_take_no_change() {
        let map = FakeMap::default()
            .with(1, 1, &[3], Some(VICTIM))
            .with(2, 1, &[], Some(VICTIM))
            .with(3, 3, &[4], Some(VICTIM))
            .with(4, 4, &[], Some(VICTIM))
            .hiding(2);
        assert_walk(
            &walk(ENGINE, Some(VICTIM), 1, &map, &alone(10)),
            &[(1, -10.0), (3, -6.5), (4, -4.225)],
            "a hidden member of a changed place is passed over",
        );
        let map = map.hiding(3);
        assert_walk(
            &walk(ENGINE, Some(VICTIM), 1, &map, &alone(10)),
            &[(1, -10.0)],
            "a hidden system alone at its place stops the walk there",
        );
    }

    #[test]
    fn a_place_changes_as_one_chain_following_only_the_passed_systems_links() {
        let map = FakeMap::default()
            .with(1, 1, &[3], Some(VICTIM))
            .with(2, 1, &[4], Some(VICTIM))
            .with(3, 3, &[], Some(VICTIM))
            .with(4, 4, &[1], Some(VICTIM));
        assert_walk(
            &walk(ENGINE, Some(VICTIM), 2, &map, &alone(10)),
            &[(1, -10.0), (2, -10.0), (4, -6.5)],
            "a crime in 2 changes its place, head first, and follows 2's links, not 1's; \
             4's link back to the head finds the place walked",
        );
        let map = FakeMap::default()
            .with(1, 1, &[6], Some(VICTIM))
            .with(5, 5, &[7], Some(VICTIM))
            .with(6, 5, &[8], Some(VICTIM))
            .with(7, 7, &[], Some(VICTIM))
            .with(8, 8, &[], Some(VICTIM))
            .hiding(6);
        assert_walk(
            &walk(ENGINE, Some(VICTIM), 1, &map, &alone(10)),
            &[(1, -10.0), (5, -6.5), (7, -4.225)],
            "a link to hidden 6 walks its place from the head, 5, and follows 5's links",
        );
    }

    #[test]
    fn the_walk_stops_past_a_place_that_did_not_change() {
        let govts = four();
        let map = FakeMap::default()
            .with(1, 1, &[2], Some(VICTIM))
            .with(2, 2, &[3], Some(NEUTRAL))
            .with(3, 3, &[], Some(VICTIM));
        assert_walk(
            &walk(BIBLE, Some(VICTIM), 1, &map, &govts),
            &[(1, -3.0)],
            "by the Bible the neutral 2 is unchanged, so 3 is never reached",
        );
        assert_walk(
            &walk(ENGINE, Some(VICTIM), 1, &map, &govts),
            &[(1, -3.0), (2, 6.5), (3, -1.2675)],
            "by the engine 2 gains, so the walk goes on to 3",
        );
        let govts = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..penalised(130, [1, 0, 0, 0])
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..penalised(132, [100, 0, 0, 0])
            },
        ]);
        let map = FakeMap::default()
            .with(1, 1, &[2], Some(VICTIM))
            .with(2, 2, &[3], Some(VICTIM))
            .with(3, 3, &[], Some(ENEMY));
        assert_walk(
            &walk(ENGINE, Some(VICTIM), 1, &map, &govts),
            &[(1, -1.0)],
            "2's 0.65 is skipped, so the enemy's 21 in 3 is never reached",
        );
    }

    #[test]
    fn cycles_repeats_and_missing_links_end_the_walk() {
        let map = FakeMap::default()
            .with(1, 1, &[2, 2, 1, 99], Some(VICTIM))
            .with(2, 2, &[1, 2], Some(VICTIM));
        assert_walk(
            &walk(ENGINE, Some(VICTIM), 1, &map, &alone(10)),
            &[(1, -10.0), (2, -6.5)],
            "each changed once; the missing 99 nothing",
        );
        assert_eq!(
            walk(ENGINE, Some(VICTIM), 99, &map, &alone(10)),
            [],
            "a crime in a system not in the catalog walks nowhere"
        );
    }

    #[test]
    fn each_system_takes_its_governments_rule() {
        let map = FakeMap::default()
            .with(1, 1, &[], Some(VICTIM))
            .with(2, 1, &[], Some(ALLY))
            .with(3, 1, &[], Some(ENEMY))
            .with(4, 1, &[], Some(NEUTRAL));
        assert_walk(
            &walk(ENGINE, Some(VICTIM), 1, &map, &four()),
            &[(1, -3.0), (2, -3.0), (3, 4.0), (4, 10.0)],
            "the ally loses the victim's penalty; the enemy and the neutral gain half their own",
        );
        assert_walk(
            &walk(BIBLE, Some(VICTIM), 1, &map, &four()),
            &[(1, -3.0), (2, -3.0), (3, 4.0)],
            "by the Bible the neutral is left alone",
        );
        let govts = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..penalised(130, [3, 0, 0, 0])
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                enemies: [1, -1, -1, -1],
                ..penalised(131, [6, 0, 0, 0])
            },
        ]);
        for law in [ENGINE, BIBLE] {
            assert_walk(
                &walk(law, Some(VICTIM), 1, &map, &govts),
                &[(1, -3.0), (2, 3.0)],
                "an ally that is also an enemy gains as an enemy",
            );
        }
    }

    #[test]
    fn an_independent_victim_pleases_xenophobes_and_angers_the_nosy_in_their_systems() {
        let govts = Governments::new([
            GovtRecord {
                flags: XENOPHOBIC | NOSY,
                ..penalised(134, [8, 0, 0, 0])
            },
            GovtRecord {
                flags: NOSY,
                ..penalised(135, [6, 0, 0, 0])
            },
            penalised(133, [20, 0, 0, 0]),
        ]);
        let map = FakeMap::default()
            .with(1, 1, &[], Some(XENOPHOBE))
            .with(2, 1, &[], Some(NOSY_ONE))
            .with(3, 1, &[], Some(NEUTRAL));
        for law in [ENGINE, BIBLE] {
            assert_walk(
                &walk(law, None, 1, &map, &govts),
                &[(1, 4.0), (2, -3.0)],
                "the xenophobe first, though also nosy; the neutral unchanged",
            );
        }
    }

    #[test]
    fn a_crime_in_an_independent_system() {
        let map = FakeMap::default().with(1, 1, &[], None);
        let govts = Governments::new([
            penalised(128, [6, 0, 0, 0]),
            penalised(130, [8, 0, 0, 0]),
            GovtRecord {
                flags: XENOPHOBIC,
                ..penalised(134, [10, 0, 0, 0])
            },
        ]);
        let independent = |victim| walk(ENGINE, victim, 1, &map, &govts);
        assert_walk(
            &independent(Some(VICTIM)),
            &[(1, -2.0)],
            "a quarter of the victim's penalty",
        );
        assert_walk(
            &independent(Some(XENOPHOBE)),
            &[(1, 5.0)],
            "a xenophobic victim's half, gained",
        );
        assert_walk(
            &independent(None),
            &[(1, -3.0)],
            "an independent victim: half the penalty of government 128",
        );
        assert_eq!(
            walk(ENGINE, None, 1, &map, &alone(10)),
            [],
            "and none when there is no 128"
        );
    }

    #[test]
    fn each_crime_reads_its_own_penalty_in_each_system() {
        let map = FakeMap::default()
            .with(1, 1, &[], Some(VICTIM))
            .with(2, 1, &[], Some(ENEMY));
        let govts = four();
        let changes = |crime| {
            ENGINE
                .system_penalties(crime, Some(VICTIM), SystemId(1), &map, &govts)
                .into_iter()
                .map(|(_, change)| change)
                .collect::<Vec<_>>()
        };
        assert_eq!(changes(Crime::Smuggle), [-99.0, 49.5]);
        assert_eq!(changes(Crime::Board), [-5.0, 3.0]);
        assert_eq!(changes(Crime::Kill), [-7.0, 5.0]);
    }

    #[test]
    fn shooting_and_a_derelict_victim_walk_nowhere() {
        let map = FakeMap::default()
            .with(1, 1, &[], Some(VICTIM))
            .with(2, 1, &[], Some(WRECKS));
        let govts = Governments::new([
            GovtRecord {
                flags: DERELICT,
                ..penalised(136, [3, 3, 3, 3])
            },
            penalised(130, [3, 3, 3, 3]),
        ]);
        for law in [ENGINE, BIBLE] {
            assert_eq!(
                law.system_penalties(Crime::Shoot, Some(VICTIM), SystemId(1), &map, &govts),
                []
            );
            for crime in [Crime::Smuggle, Crime::Disable, Crime::Board, Crime::Kill] {
                assert_eq!(
                    law.system_penalties(crime, Some(WRECKS), SystemId(1), &map, &govts),
                    [],
                    "{crime:?} {law:?}"
                );
            }
        }
    }

    #[test]
    fn a_change_strictly_between_minus_1_and_1_is_skipped_and_1_applies() {
        let govts = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..penalised(130, [1, 0, 0, 0])
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..penalised(132, [2, 0, 0, 0])
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..penalised(133, [-2, 0, 0, 0])
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..penalised(134, [3, 0, 0, 0])
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..penalised(135, [-3, 0, 0, 0])
            },
        ]);
        let map = FakeMap::default()
            .with(1, 1, &[5], Some(VICTIM))
            .with(2, 1, &[], Some(ENEMY))
            .with(3, 1, &[], Some(NEUTRAL))
            .with(5, 5, &[], Some(XENOPHOBE))
            .with(6, 5, &[], Some(NOSY_ONE));
        assert_eq!(
            walk(ENGINE, Some(VICTIM), 1, &map, &govts),
            [(1, -1.0), (2, 1.0), (3, -1.0)],
            "-1 and +1 apply; one jump on, 0.975 and -0.975 are skipped"
        );
    }

    #[test]
    fn the_law_takes_its_crime_gains_from_the_rulebook() {
        use crate::rulebook::{RuleKey, Rulebook};
        assert_eq!(NovaLaw::from_rulebook(&Rulebook::default()), ENGINE);
        assert_eq!(
            NovaLaw::from_rulebook(&Rulebook::new(RuleSource::Bible)),
            BIBLE
        );
        let overridden =
            Rulebook::new(RuleSource::Bible).with_override(RuleKey::CrimeGains, RuleSource::Engine);
        assert_eq!(NovaLaw::from_rulebook(&overridden), ENGINE);
        let overridden =
            Rulebook::new(RuleSource::Engine).with_override(RuleKey::CrimeGains, RuleSource::Bible);
        assert_eq!(NovaLaw::from_rulebook(&overridden), BIBLE);
    }

    #[test]
    fn nova_law_follows_the_engine_by_default() {
        assert_eq!(NovaLaw::default(), ENGINE);
        assert_eq!(
            NovaLaw::default().penalties(Crime::Disable, Some(VICTIM), &four()),
            ENGINE.penalties(Crime::Disable, Some(VICTIM), &four())
        );
    }

    #[test]
    fn by_the_engine_disabling_costs_the_victims_penalty_and_pleases_all_but_its_allies() {
        assert_eq!(
            ENGINE.penalties(Crime::Disable, Some(VICTIM), &four()),
            [(VICTIM, -3), (ALLY, -3), (ENEMY, 4), (NEUTRAL, 10)],
            "the enemy and the neutral gain half their own penalty"
        );
    }

    /// `_RecursivePenaltySlap` reads `SmugPenalty` for type 0, gövt +0x48
    /// (@0x9954): the scan's contraband, every government's 99 here.
    #[test]
    fn smuggling_costs_the_smuggling_penalty_as_the_other_crimes_cost_theirs() {
        assert_eq!(
            ENGINE.penalties(Crime::Smuggle, Some(VICTIM), &four()),
            [(VICTIM, -99), (ALLY, -99), (ENEMY, 49), (NEUTRAL, 49)]
        );
        assert_eq!(
            BIBLE.penalties(Crime::Smuggle, Some(VICTIM), &four()),
            [(VICTIM, -99), (ALLY, -99), (ENEMY, 49)]
        );
    }

    #[test]
    fn by_the_bible_disabling_pleases_only_the_victims_enemies() {
        let changes = BIBLE.penalties(Crime::Disable, Some(VICTIM), &four());
        assert_eq!(changes, [(VICTIM, -3), (ALLY, -3), (ENEMY, 4)]);
        assert!(
            !changes.iter().any(|&(govt, _)| govt == NEUTRAL),
            "the neutral is left alone"
        );
    }

    #[test]
    fn each_crime_reads_its_own_penalty() {
        let govts = four();
        assert_eq!(
            ENGINE.penalties(Crime::Kill, Some(VICTIM), &govts),
            [(VICTIM, -7), (ALLY, -7), (ENEMY, 5), (NEUTRAL, 10)]
        );
        assert_eq!(
            ENGINE.penalties(Crime::Board, Some(VICTIM), &govts),
            [(VICTIM, -5), (ALLY, -5), (ENEMY, 3), (NEUTRAL, 10)]
        );
        assert_eq!(
            BIBLE.penalties(Crime::Kill, Some(VICTIM), &govts),
            [(VICTIM, -7), (ALLY, -7), (ENEMY, 5)]
        );
    }

    #[test]
    fn an_ally_that_is_also_an_enemy_counts_as_an_enemy() {
        let govts = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..penalised(130, [3, 0, 0, 0])
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                enemies: [1, -1, -1, -1],
                ..penalised(131, [6, 0, 0, 0])
            },
        ]);
        for law in [ENGINE, BIBLE] {
            assert_eq!(
                law.penalties(Crime::Disable, Some(VICTIM), &govts),
                [(VICTIM, -3), (ALLY, 3)],
                "{law:?}"
            );
        }
    }

    #[test]
    fn a_change_is_truncated_and_one_below_1_skipped() {
        let govts = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..penalised(130, [3, 0, 0, 0])
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..penalised(131, [1, 0, 0, 0])
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..penalised(132, [5, 0, 0, 0])
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..penalised(133, [-3, 0, 0, 0])
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..penalised(134, [-1, 0, 0, 0])
            },
            penalised(135, [1, 0, 0, 0]),
            penalised(136, [-3, 0, 0, 0]),
        ]);
        assert_eq!(
            ENGINE.penalties(Crime::Disable, Some(VICTIM), &govts),
            [(VICTIM, -3), (ENEMY, 2), (NEUTRAL, -1), (WRECKS, -1)],
            "0.5 skipped, 2.5 to 2, -1.5 to -1, -0.5 skipped; \
             the neutrals' 0.5 skipped and -1.5 to -1"
        );
        let lenient = Governments::new([penalised(130, [0, 0, 0, 0]), penalised(131, [0; 4])]);
        for law in [ENGINE, BIBLE] {
            assert_eq!(
                law.penalties(Crime::Disable, Some(VICTIM), &lenient),
                [],
                "none is no change"
            );
        }
    }

    #[test]
    fn a_crime_against_an_independent_pleases_xenophobes_and_angers_the_nosy() {
        let govts = Governments::new([
            GovtRecord {
                flags: XENOPHOBIC | NOSY,
                ..penalised(134, [8, 0, 0, 0])
            },
            GovtRecord {
                flags: NOSY,
                ..penalised(135, [6, 0, 0, 0])
            },
            penalised(133, [20, 0, 0, 0]),
        ]);
        for law in [ENGINE, BIBLE] {
            assert_eq!(
                law.penalties(Crime::Disable, None, &govts),
                [(XENOPHOBE, 4), (NOSY_ONE, -3)],
                "the xenophobe first, though also nosy; the neutral unchanged"
            );
        }
    }

    #[test]
    fn a_derelict_victim_and_shooting_change_nothing() {
        let mut govts = four();
        for law in [ENGINE, BIBLE] {
            assert_eq!(law.penalties(Crime::Shoot, Some(VICTIM), &govts), []);
        }
        govts = Governments::new([
            GovtRecord {
                flags: DERELICT,
                ..penalised(136, [3, 3, 3, 3])
            },
            GovtRecord {
                enemies: [-1; 4],
                ..penalised(130, [3, 3, 3, 3])
            },
        ]);
        for crime in [Crime::Smuggle, Crime::Disable, Crime::Board, Crime::Kill] {
            for law in [ENGINE, BIBLE] {
                assert_eq!(
                    law.penalties(crime, Some(WRECKS), &govts),
                    [],
                    "{crime:?} {law:?}"
                );
            }
        }
    }

    #[test]
    fn each_systems_record_is_its_governments_or_else_its_initial_record() {
        use std::collections::BTreeMap;

        use crate::catalog::SystemId;
        let govts = Governments::new([
            govt(130),
            GovtRecord {
                initial_rec: -5,
                ..govt(131)
            },
            govt(133),
            GovtRecord {
                initial_rec: 7,
                ..govt(134)
            },
        ]);
        let legal = BTreeMap::from([(VICTIM, 10), (XENOPHOBE, 0)]);
        let systems = [
            (SystemId(130), Some(VICTIM)),
            (SystemId(131), Some(ALLY)),
            (SystemId(132), None),
            (SystemId(133), Some(NEUTRAL)),
            (SystemId(134), Some(XENOPHOBE)),
        ];
        assert_eq!(
            systems_from_govts(systems, &legal, &govts),
            BTreeMap::from([(SystemId(130), 10), (SystemId(131), -5)]),
            "the government's record, else its InitialRec; the independent, \
             the zero InitialRec and the record of 0 over an InitialRec of 7 \
             left out as none"
        );
    }

    #[test]
    fn convicting_truncates_each_systems_new_record_not_its_change() {
        let catalog = crate::testkit::catalog();
        let mut pilot = Pilot::new(&catalog, "").expect("a pilot");
        for (system, record) in [(1, -31_990), (2, 5), (3, -5), (4, 0), (5, 31_999), (6, 8)] {
            pilot.set_system_record(SystemId(system), record);
        }
        convict(
            &mut pilot,
            &[(VICTIM, -20)],
            &[
                (SystemId(1), -20.5),
                (SystemId(2), -2.6),
                (SystemId(3), 2.6),
                (SystemId(4), -1.5),
                (SystemId(5), 1.5),
                (SystemId(7), 3.9),
            ],
        );
        let records: Vec<i16> = (1..=7)
            .map(|system| pilot.system_record(SystemId(system)))
            .collect();
        assert_eq!(
            records,
            [-32_000, 2, -2, -1, 32_000, 8, 3],
            "held; 2.4 to 2 and -2.4 to -2, not 5 - 2 and -5 + 2; -1.5 to -1; \
             held; untouched; from none"
        );
        assert_eq!(pilot.legal_record(VICTIM), -20, "and the government's");
        convict(
            &mut pilot,
            &[],
            &[(SystemId(1), -70_000.0), (SystemId(5), 70_000.0)],
        );
        assert_eq!(pilot.system_record(SystemId(1)), -32_000);
        assert_eq!(pilot.system_record(SystemId(5)), 32_000);
    }

    #[test]
    fn convicting_changes_each_record_held_within_32000() {
        let catalog = crate::testkit::catalog();
        let mut pilot = Pilot::new(&catalog, "").expect("a pilot");
        pilot.set_legal_record(VICTIM, -31_990);
        pilot.set_legal_record(ENEMY, 31_995);
        pilot.set_legal_record(ALLY, 10);
        convict(
            &mut pilot,
            &[(VICTIM, -20), (ENEMY, 20), (ALLY, -4), (NEUTRAL, 7)],
            &[],
        );
        assert_eq!(pilot.legal_record(VICTIM), -32_000);
        assert_eq!(pilot.legal_record(ENEMY), 32_000);
        assert_eq!(pilot.legal_record(ALLY), 6);
        assert_eq!(pilot.legal_record(NEUTRAL), 7, "from none");
        convict(&mut pilot, &[(VICTIM, -70_000), (ENEMY, 70_000)], &[]);
        assert_eq!(pilot.legal_record(VICTIM), -32_000);
        assert_eq!(pilot.legal_record(ENEMY), 32_000);
        assert_eq!((RECORD_LIMIT, ENEMY_SHARE), (32_000, 0.5));
    }
}
