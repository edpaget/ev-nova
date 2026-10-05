//! The player's legal record: what the player's crimes against a ship do
//! to it, government by government.
//!
//! The [`LegalCode`] port says how a crime changes the records; Nova's is
//! [`NovaLaw`], and [`convict`] applies what it says to a pilot.
//!
//! The original (`_SlapWithPenalty` @0x9bbc and `_RecursivePenaltySlap`
//! @0x980a in the `EV Nova` executable) keeps the record per system, and
//! a crime against a ship of government V changes the record of every
//! system reached from the crime's, along the hyperlinks, by a factor of
//! 0.65 a jump. Here the record is per government, as the save keeps it:
//! the record with government G stands for the original's in G's
//! systems. So a crime against V, with P the crime's penalty field
//! (`DisabPenalty`, `BoardPenalty`, `KillPenalty`), changes the record
//! with each government S in the catalog by:
//!
//! - **-V.P** when S is V, or allied with V and not its enemy (@0x998d,
//!   @0x9a23);
//! - **+0.5 S.P**, S's own penalty halved, when S is not allied with V,
//!   whether its enemy or neutral (@0x9a51; 0.5 @0xdd128). That is the
//!   engine's rule, [`CrimeGains::Engine`], and the default. The Bible's,
//!   [`CrimeGains::Bible`] ("evil deeds to one government will improve
//!   your rating with its enemies... allied governments also communicate
//!   your actions"), gives +0.5 S.P only to an enemy of V, and leaves a
//!   neutral alone;
//! - for an independent victim, **+0.5 S.P** when S is xenophobic, else
//!   **-0.5 S.P** when S is nosy (`Flags` 0x0002);
//! - and nothing otherwise.
//!
//! Each change is truncated and skipped when below 1 either way, and the
//! record is held within [`RECORD_LIMIT`] either way (@0x9ad1-0x9b1e).
//! A derelict victim's government (`Flags` 0x0800) changes nothing, and
//! neither does [`Crime::Shoot`]: the original never passes
//! `ShootPenalty`, as the Bible says ("currently ignored").
//!
//! Stated divergence: there is no 0.65 decay along the hyperlinks (there
//! is no per-system record). So by the engine's rule a crime improves the
//! record with every government in the catalog not allied with the
//! victim's, where the original improves it only in the nearby systems
//! those governments hold.

use std::fmt::Debug;

use crate::catalog::GovtId;
use crate::govt::{Governments, NOSY};
use crate::pilot::Pilot;

/// How far either way of none a legal record goes.
pub const RECORD_LIMIT: i32 = 32_000;
/// What a pleased government's (or a nosy one's) own penalty counts for.
pub const ENEMY_SHARE: f32 = 0.5;

/// A crime against a ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crime {
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

/// Which governments a crime against a ship of government V improves the
/// record with (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrimeGains {
    /// The original engine's: every government not allied with V, by
    /// half its own penalty.
    #[default]
    Engine,
    /// The Bible's: only V's enemies, by half their own penalty.
    Bible,
}

/// Nova's law (see the module docs): the engine's by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NovaLaw {
    /// Which governments a crime improves the record with.
    pub gains: CrimeGains,
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
                    Some(_) if self.gains == CrimeGains::Engine => ENEMY_SHARE * theirs(id),
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

/// Applies `changes` to `pilot`'s records, each held within
/// [`RECORD_LIMIT`].
pub fn convict(pilot: &mut Pilot, changes: &[(GovtId, i32)]) {
    for &(govt, change) in changes {
        let record =
            (i32::from(pilot.legal_record(govt)) + change).clamp(-RECORD_LIMIT, RECORD_LIMIT);
        pilot.set_legal_record(govt, i16::try_from(record).unwrap_or_default());
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
        gains: CrimeGains::Engine,
    };
    const BIBLE: NovaLaw = NovaLaw {
        gains: CrimeGains::Bible,
    };

    #[test]
    fn nova_law_follows_the_engine_by_default() {
        assert_eq!(CrimeGains::default(), CrimeGains::Engine);
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
        for crime in [Crime::Disable, Crime::Board, Crime::Kill] {
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
    fn convicting_changes_each_record_held_within_32000() {
        let catalog = crate::testkit::catalog();
        let mut pilot = Pilot::new(&catalog, "").expect("a pilot");
        pilot.set_legal_record(VICTIM, -31_990);
        pilot.set_legal_record(ENEMY, 31_995);
        pilot.set_legal_record(ALLY, 10);
        convict(
            &mut pilot,
            &[(VICTIM, -20), (ENEMY, 20), (ALLY, -4), (NEUTRAL, 7)],
        );
        assert_eq!(pilot.legal_record(VICTIM), -32_000);
        assert_eq!(pilot.legal_record(ENEMY), 32_000);
        assert_eq!(pilot.legal_record(ALLY), 6);
        assert_eq!(pilot.legal_record(NEUTRAL), 7, "from none");
        convict(&mut pilot, &[(VICTIM, -70_000), (ENEMY, 70_000)]);
        assert_eq!(pilot.legal_record(VICTIM), -32_000);
        assert_eq!(pilot.legal_record(ENEMY), 32_000);
        assert_eq!((RECORD_LIMIT, ENEMY_SHARE), (32_000, 0.5));
    }
}
