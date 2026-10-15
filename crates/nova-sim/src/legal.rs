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
//! ship of government V, the record of every system reached from the
//! crime's, along the hyperlinks, by a factor of 0.65 a jump. Here a
//! crime against V, with P the crime's penalty field (`SmugPenalty`,
//! `DisabPenalty`, `BoardPenalty`, `KillPenalty`: types 0 to 3,
//! `_RecursivePenaltySlap` reading gövt +0x48 + 2×type @0x9954),
//! changes the record with each government S in the catalog by:
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
//!   **-0.5 S.P** when S is nosy (`Flags` 0x0002);
//! - and nothing otherwise.
//!
//! Each change is truncated and skipped when below 1 either way, and the
//! record is held within [`RECORD_LIMIT`] either way (@0x9ad1-0x9b1e).
//! A derelict victim's government (`Flags` 0x0800) changes nothing, and
//! neither does [`Crime::Shoot`]: the original never passes
//! `ShootPenalty`, as the Bible says ("currently ignored"). The record in
//! each system of S takes S's change whole.
//!
//! Stated divergence: there is no 0.65 decay along the hyperlinks. So by
//! the engine's rule a crime improves the record in every system of
//! every government not allied with the victim's, where the original
//! improves it only in the nearby systems those governments hold. The
//! walk is rdm:phase/missions-fidelity/phase-22-crime-walk-per-system.
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

use std::collections::BTreeMap;
use std::fmt::Debug;

use crate::catalog::{GovtId, SystemId};
use crate::govt::{Governments, NOSY};
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// How far either way of none a legal record goes.
pub const RECORD_LIMIT: i32 = 32_000;
/// What a pleased government's (or a nosy one's) own penalty counts for.
pub const ENEMY_SHARE: f32 = 0.5;

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
        let theirs = |govt: GovtId| f32::from(govts.penalty(Some(govt), crime));
        let own = f32::from(govts.penalty(victim, crime));
        govts
            .ids()
            .filter_map(|id| {
                let other = Some(id);
                let change = match victim {
                    Some(_) if govts.enemies(victim, other) => ENEMY_SHARE * theirs(id),
                    Some(_) if govts.allies(victim, other) => -own,
                    Some(_) if self.crime_gains == RuleSource::Engine => ENEMY_SHARE * theirs(id),
                    None if govts.xenophobic(other) => ENEMY_SHARE * theirs(id),
                    None if govts.flag(other, NOSY) => -ENEMY_SHARE * theirs(id),
                    // By the Bible, a neutral is left alone.
                    Some(_) | None => 0.0,
                };
                // Truncated: a change of less than 1 either way is none.
                let change = change.trunc() as i32;
                (change != 0).then_some((id, change))
            })
            .collect()
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

/// Applies `changes` to `pilot`'s records with each government, and to
/// its record in each of `systems` (each with its government) whose
/// government a change names, each held within [`RECORD_LIMIT`]. Until
/// rdm:phase/missions-fidelity/phase-22-crime-walk-per-system walks a
/// crime as `_RecursivePenaltySlap` does, each system of a government
/// takes that government's change whole (see the module docs).
pub fn convict(
    pilot: &mut Pilot,
    changes: &[(GovtId, i32)],
    systems: impl IntoIterator<Item = (SystemId, Option<GovtId>)>,
) {
    let held = |record: i16, change: i32| {
        let record = (i32::from(record) + change).clamp(-RECORD_LIMIT, RECORD_LIMIT);
        i16::try_from(record).unwrap_or_default()
    };
    for &(govt, change) in changes {
        pilot.set_legal_record(govt, held(pilot.legal_record(govt), change));
    }
    for (system, govt) in systems {
        let change = changes.iter().find(|&&(changed, _)| Some(changed) == govt);
        if let Some(&(_, change)) = change {
            pilot.set_system_record(system, held(pilot.system_record(system), change));
        }
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
    fn convicting_changes_each_system_of_a_changed_government() {
        use crate::catalog::SystemId;
        let catalog = crate::testkit::catalog();
        let mut pilot = Pilot::new(&catalog, "").expect("a pilot");
        pilot.set_system_record(SystemId(1), -31_990);
        pilot.set_system_record(SystemId(2), 5);
        pilot.set_system_record(SystemId(3), 40);
        pilot.set_system_record(SystemId(4), 8);
        convict(
            &mut pilot,
            &[(VICTIM, -20), (ALLY, -4)],
            [
                (SystemId(1), Some(VICTIM)),
                (SystemId(2), Some(VICTIM)),
                (SystemId(3), Some(ALLY)),
                (SystemId(4), None),
                (SystemId(5), Some(NEUTRAL)),
            ],
        );
        let records: Vec<i16> = (1..=5)
            .map(|system| pilot.system_record(SystemId(system)))
            .collect();
        assert_eq!(
            records,
            [-32_000, -15, 36, 8, 0],
            "the victim's two, one held; the ally's; the independent and the \
             unchanged neutral untouched"
        );
        assert_eq!(pilot.legal_record(VICTIM), -20, "and the government's");
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
            [],
        );
        assert_eq!(pilot.legal_record(VICTIM), -32_000);
        assert_eq!(pilot.legal_record(ENEMY), 32_000);
        assert_eq!(pilot.legal_record(ALLY), 6);
        assert_eq!(pilot.legal_record(NEUTRAL), 7, "from none");
        convict(&mut pilot, &[(VICTIM, -70_000), (ENEMY, 70_000)], []);
        assert_eq!(pilot.legal_record(VICTIM), -32_000);
        assert_eq!(pilot.legal_record(ENEMY), 32_000);
        assert_eq!((RECORD_LIMIT, ENEMY_SHARE), (32_000, 0.5));
    }
}
