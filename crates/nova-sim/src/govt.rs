//! Governments and their relations: who is allied with whom, who is at
//! war, and the flags that say how a government's ships behave.
//!
//! The rules are the original's (`_GovtAllies` @0x4e3d, `_GovtEnemies`
//! @0x4f22 and `_IsShipMyEnemy` @0x8196c in the `EV Nova` executable),
//! with `None` for an independent ship or system:
//!
//! - **Allies** ([`Governments::allies`]): the same government, an
//!   independent with an independent included. Otherwise both must be
//!   governments, neither [`DERELICT`]; then they are allies when a class
//!   of one is among the other's `Ally1-4`, either way round. So the
//!   relation is symmetric and not transitive. A -1 slot is unused.
//! - **Enemies** ([`Governments::enemies`]): never the same government.
//!   Two governments of which either is derelict are not enemies; two
//!   others are when a class of one is among the other's `Enemy1-4`,
//!   either way round, even when they are allies too. Otherwise (an
//!   independent among them, or no enemy class) allies are not enemies,
//!   and a [`XENOPHOBIC`] government is the enemy of everyone else. An
//!   enemy of an ally is not an enemy.
//! - **NPC enemies** ([`Governments::npc_enemies`]): two ships that both
//!   have a government, different ones, that are enemies. An independent
//!   ship is never an NPC's enemy.
//! - A government the table does not hold has no classes and no flags.
//! - `MaxOdds` ([`Governments::max_odds`]) is a fraction, `MaxOdds / 100`,
//!   no less than [`MIN_ODDS`] (`_LoadObjectData` @0x7bbb3); an
//!   independent ship's is even odds, a placeholder.

use std::collections::BTreeMap;

use crate::catalog::{CombatCatalog, GovtId, GovtRecord};
use crate::legal::Crime;

/// `Flags`: xenophobic, the enemy of every other government.
pub const XENOPHOBIC: u16 = 0x0001;
/// `Flags`: nosy: its warships answer attacks on ships of governments it
/// is not allied with, and hunt a player wanted elsewhere.
pub const NOSY: u16 = 0x0002;
/// `Flags`: its warships always attack the player.
pub const ALWAYS_ATTACKS_PLAYER: u16 = 0x0004;
/// `Flags`: its warships retreat when badly damaged or outnumbered.
pub const WARSHIPS_RETREAT: u16 = 0x0010;
/// `Flags`: nobody comes to the help of its ships under attack.
pub const IGNORED_WHEN_ATTACKED: u16 = 0x0020;
/// `Flags`: its warships never attack the player.
pub const NEVER_ATTACKS_PLAYER: u16 = 0x0040;
/// `Flags`: its interceptors retreat when outnumbered.
pub const INTERCEPTORS_RETREAT: u16 = 0x0100;
/// `Flags`: derelict: no relations at all, and no crime against it.
pub const DERELICT: u16 = 0x0800;
/// The least `MaxOdds` a government has, as a fraction.
pub const MIN_ODDS: f32 = 0.01;
/// What `MaxOdds` is out of.
pub const ODDS_PERCENT: f32 = 100.0;

/// Every government, by ID.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Governments {
    records: BTreeMap<GovtId, GovtRecord>,
}

impl Governments {
    /// The table of `records`.
    #[must_use]
    pub fn new(records: impl IntoIterator<Item = GovtRecord>) -> Self {
        Self {
            records: records
                .into_iter()
                .map(|record| (record.id, record))
                .collect(),
        }
    }

    /// The table `catalog` gives.
    #[must_use]
    pub fn read(catalog: &(impl CombatCatalog + ?Sized)) -> Self {
        Self::new(catalog.governments())
    }

    /// Government `id`'s record, if the table holds it.
    #[must_use]
    pub fn get(&self, id: GovtId) -> Option<&GovtRecord> {
        self.records.get(&id)
    }

    /// Every government the table holds, by ascending ID.
    pub fn ids(&self) -> impl Iterator<Item = GovtId> + '_ {
        self.records.keys().copied()
    }

    /// Whether `govt` has the `Flags` `bit`: never for an independent or
    /// a government the table does not hold.
    #[must_use]
    pub fn flag(&self, govt: Option<GovtId>, bit: u16) -> bool {
        self.flags(govt) & bit != 0
    }

    /// Whether `govt` is xenophobic.
    #[must_use]
    pub fn xenophobic(&self, govt: Option<GovtId>) -> bool {
        self.flag(govt, XENOPHOBIC)
    }

    /// Whether `govt` is derelict.
    #[must_use]
    pub fn derelict(&self, govt: Option<GovtId>) -> bool {
        self.flag(govt, DERELICT)
    }

    /// `govt`'s `CrimeTol`: none for an independent or a government the
    /// table does not hold.
    #[must_use]
    pub fn crime_tol(&self, govt: Option<GovtId>) -> i16 {
        self.record(govt).map_or(0, |record| record.crime_tol)
    }

    /// `govt`'s penalty for `crime` against its ships: none for an
    /// independent or a government the table does not hold.
    #[must_use]
    pub fn penalty(&self, govt: Option<GovtId>, crime: Crime) -> i16 {
        self.record(govt).map_or(0, |record| {
            let penalties = record.penalties;
            match crime {
                Crime::Disable => penalties.disable,
                Crime::Board => penalties.board,
                Crime::Kill => penalties.kill,
                Crime::Shoot => penalties.shoot,
            }
        })
    }

    /// The odds `govt`'s ships still take on, as a fraction (see the
    /// module docs).
    #[must_use]
    pub fn max_odds(&self, govt: Option<GovtId>) -> f32 {
        if govt.is_none() {
            return 1.0;
        }
        let raw = self.record(govt).map_or(0, |record| record.max_odds);
        (f32::from(raw) / ODDS_PERCENT).max(MIN_ODDS)
    }

    /// Whether `a` and `b` are allies (see the module docs).
    #[must_use]
    pub fn allies(&self, a: Option<GovtId>, b: Option<GovtId>) -> bool {
        if a == b {
            return true;
        }
        let (Some(first), Some(second)) = (self.related(a), self.related(b)) else {
            return false;
        };
        lists_a_class(second.allies, first.classes) || lists_a_class(first.allies, second.classes)
    }

    /// Whether `a` and `b` are enemies (see the module docs).
    #[must_use]
    pub fn enemies(&self, a: Option<GovtId>, b: Option<GovtId>) -> bool {
        if a == b {
            return false;
        }
        if let (Some(_), Some(_)) = (a, b) {
            let (Some(first), Some(second)) = (self.related(a), self.related(b)) else {
                return false;
            };
            if lists_a_class(second.enemies, first.classes)
                || lists_a_class(first.enemies, second.classes)
            {
                return true;
            }
        }
        if self.allies(a, b) {
            return false;
        }
        self.xenophobic(a) || self.xenophobic(b)
    }

    /// Whether ships of `a` and `b` are enemies to an NPC: both have a
    /// government, different ones, that are enemies.
    #[must_use]
    pub fn npc_enemies(&self, a: Option<GovtId>, b: Option<GovtId>) -> bool {
        a.is_some() && b.is_some() && self.enemies(a, b)
    }

    /// `govt`'s record, if it is a government the table holds.
    fn record(&self, govt: Option<GovtId>) -> Option<&GovtRecord> {
        self.records.get(&govt?)
    }

    /// `govt`'s record for relations: a government, not derelict, with
    /// the default (no classes) for one the table does not hold.
    fn related(&self, govt: Option<GovtId>) -> Option<GovtRecord> {
        let id = govt?;
        let record = self.records.get(&id).copied().unwrap_or(GovtRecord {
            id,
            flags: 0,
            flags2: 0,
            crime_tol: 0,
            penalties: crate::catalog::Penalties::default(),
            max_odds: 0,
            classes: [UNUSED; 4],
            allies: [UNUSED; 4],
            enemies: [UNUSED; 4],
        });
        (record.flags & DERELICT == 0).then_some(record)
    }

    /// `govt`'s `Flags`: none for an independent or a government the
    /// table does not hold.
    fn flags(&self, govt: Option<GovtId>) -> u16 {
        self.record(govt).map_or(0, |record| record.flags)
    }
}

/// An unused class slot.
const UNUSED: i16 = -1;

/// Whether `list` names any of `classes`, unused slots aside.
fn lists_a_class(list: [i16; 4], classes: [i16; 4]) -> bool {
    classes
        .iter()
        .filter(|&&class| class != UNUSED)
        .any(|class| list.contains(class))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::testkit::govt;

    const FED: Option<GovtId> = Some(GovtId(128));
    const CIVVIES: Option<GovtId> = Some(GovtId(157));
    const PIRATES: Option<GovtId> = Some(GovtId(137));
    const AURORA: Option<GovtId> = Some(GovtId(140));
    const WRECKS: Option<GovtId> = Some(GovtId(150));
    const TRADERS: Option<GovtId> = Some(GovtId(160));
    const UNKNOWN: Option<GovtId> = Some(GovtId(999));

    /// The Federation (class 1, allied with 12 and 13, at war with 9 and
    /// 2); the Civvies (class 13); xenophobic pirates (class 9, which
    /// lists the Federation's class 1 among its allies too); the Aurorans
    /// (class 2, allied with 9); derelicts (class 12, at war with 1);
    /// traders (class 20, allied with the Civvies' 13).
    fn table() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [12, 13, -1, -1],
                enemies: [9, 2, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                classes: [13, -1, -1, -1],
                ..govt(157)
            },
            GovtRecord {
                flags: XENOPHOBIC,
                classes: [9, -1, -1, -1],
                allies: [1, -1, -1, -1],
                ..govt(137)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                allies: [9, -1, -1, -1],
                ..govt(140)
            },
            GovtRecord {
                flags: DERELICT,
                classes: [12, -1, -1, -1],
                enemies: [1, -1, -1, -1],
                ..govt(150)
            },
            GovtRecord {
                classes: [-1, 20, -1, -1],
                allies: [-1, -1, 13, -1],
                ..govt(160)
            },
        ])
    }

    #[test]
    fn a_government_is_its_own_ally_and_so_is_an_independent() {
        let govts = table();
        for govt in [FED, PIRATES, WRECKS, UNKNOWN, None] {
            assert!(govts.allies(govt, govt), "{govt:?}");
        }
    }

    #[test]
    fn allies_share_a_class_either_way_round() {
        let govts = table();
        assert!(govts.allies(FED, CIVVIES), "the Federation lists 13");
        assert!(govts.allies(CIVVIES, FED), "symmetric");
        assert!(govts.allies(TRADERS, CIVVIES), "past unused slots");
        assert!(govts.allies(CIVVIES, TRADERS));
    }

    #[test]
    fn allies_are_not_transitive_and_need_two_governments() {
        let govts = table();
        assert!(!govts.allies(FED, TRADERS), "an ally's ally");
        assert!(!govts.allies(FED, None));
        assert!(!govts.allies(None, CIVVIES));
        assert!(!govts.allies(FED, UNKNOWN), "no classes");
        assert!(!govts.allies(AURORA, CIVVIES));
    }

    #[test]
    fn a_derelict_government_has_no_allies() {
        let govts = table();
        assert!(!govts.allies(FED, WRECKS), "though the Federation lists 12");
        assert!(!govts.allies(WRECKS, FED));
    }

    #[test]
    fn unused_slots_name_no_class() {
        let govts = Governments::new([govt(128), govt(129)]);
        assert!(!govts.allies(Some(GovtId(128)), Some(GovtId(129))));
        assert!(!govts.enemies(Some(GovtId(128)), Some(GovtId(129))));
    }

    #[test]
    fn enemies_list_a_class_either_way_round_even_when_also_allies() {
        let govts = table();
        assert!(govts.enemies(FED, AURORA));
        assert!(govts.enemies(AURORA, FED), "symmetric");
        assert!(govts.allies(FED, PIRATES), "the pirates list class 1");
        assert!(govts.enemies(FED, PIRATES), "the enemy class wins");
        assert!(govts.enemies(PIRATES, FED));
    }

    #[test]
    fn a_government_is_never_its_own_enemy() {
        let govts = table();
        for govt in [FED, PIRATES, None] {
            assert!(!govts.enemies(govt, govt), "{govt:?}");
        }
    }

    #[test]
    fn a_xenophobe_is_the_enemy_of_anyone_not_allied_independents_included() {
        let govts = table();
        assert!(govts.enemies(PIRATES, CIVVIES));
        assert!(govts.enemies(CIVVIES, PIRATES));
        assert!(govts.enemies(PIRATES, None));
        assert!(govts.enemies(None, PIRATES));
        assert!(!govts.enemies(PIRATES, AURORA), "the Aurorans are allies");
    }

    #[test]
    fn a_derelict_government_is_nobodys_enemy() {
        let govts = table();
        assert!(!govts.enemies(WRECKS, FED), "though it lists class 1");
        assert!(!govts.enemies(FED, WRECKS));
        assert!(!govts.enemies(WRECKS, PIRATES), "even a xenophobe's");
        assert!(!govts.enemies(PIRATES, WRECKS));
    }

    #[test]
    fn an_enemy_of_an_ally_is_not_an_enemy() {
        let govts = table();
        assert!(govts.enemies(FED, AURORA));
        assert!(govts.allies(FED, CIVVIES));
        assert!(!govts.enemies(CIVVIES, AURORA));
        assert!(!govts.enemies(FED, None), "no xenophobe");
        assert!(!govts.enemies(FED, UNKNOWN));
    }

    #[test]
    fn npc_enemies_need_two_governments() {
        let govts = table();
        assert!(govts.npc_enemies(FED, AURORA));
        assert!(govts.npc_enemies(PIRATES, CIVVIES));
        assert!(!govts.npc_enemies(PIRATES, None));
        assert!(!govts.npc_enemies(None, PIRATES));
        assert!(!govts.npc_enemies(FED, CIVVIES));
    }

    #[test]
    fn max_odds_are_a_fraction_no_less_than_a_hundredth() {
        let govts = Governments::new([
            GovtRecord {
                max_odds: 200,
                ..govt(128)
            },
            GovtRecord {
                max_odds: 0,
                ..govt(129)
            },
            GovtRecord {
                max_odds: 1,
                ..govt(130)
            },
        ]);
        assert_eq!(govts.max_odds(Some(GovtId(128))), 2.0);
        assert_eq!(govts.max_odds(Some(GovtId(129))), 0.01);
        assert_eq!(govts.max_odds(Some(GovtId(130))), 0.01);
        assert_eq!(govts.max_odds(Some(GovtId(131))), 0.01, "unknown");
        assert_eq!(govts.max_odds(None), 1.0, "independent");
        assert_eq!((MIN_ODDS, ODDS_PERCENT), (0.01, 100.0));
    }

    #[test]
    fn flags_and_tolerance_are_the_governments_and_none_for_others() {
        let govts = Governments::new([GovtRecord {
            flags: NOSY | WARSHIPS_RETREAT,
            crime_tol: 6,
            ..govt(128)
        }]);
        assert!(govts.flag(FED, NOSY));
        assert!(govts.flag(FED, WARSHIPS_RETREAT));
        assert!(!govts.flag(FED, XENOPHOBIC));
        assert!(!govts.xenophobic(FED));
        assert!(!govts.flag(None, NOSY));
        assert!(!govts.flag(UNKNOWN, NOSY));
        assert_eq!(govts.crime_tol(FED), 6);
        assert_eq!(govts.crime_tol(None), 0);
        assert_eq!(govts.crime_tol(UNKNOWN), 0);
        assert!(table().xenophobic(PIRATES));
        assert!(table().derelict(WRECKS));
        assert!(!table().derelict(FED));
    }

    #[test]
    fn the_table_holds_each_government_by_id() {
        let govts = table();
        assert_eq!(
            govts.get(GovtId(157)).map(|record| record.classes[0]),
            Some(13)
        );
        assert_eq!(govts.get(GovtId(999)), None);
        assert_eq!(
            govts.ids().map(|id| id.0).collect::<Vec<_>>(),
            [128, 137, 140, 150, 157, 160]
        );
        let catalog = crate::testkit::FakePilotCatalog {
            govts: vec![govt(130), govt(128)],
            ..crate::testkit::catalog()
        };
        assert_eq!(
            Governments::read(&catalog),
            Governments::new([govt(128), govt(130)])
        );
        assert_eq!(
            Governments::read(&crate::testkit::catalog()),
            Governments::default()
        );
    }

    #[test]
    fn each_crime_has_its_own_penalty_and_none_for_others() {
        let govts = Governments::new([GovtRecord {
            penalties: crate::catalog::Penalties {
                smuggle: 1,
                disable: 3,
                board: 5,
                kill: 7,
                shoot: 2,
            },
            ..govt(128)
        }]);
        assert_eq!(
            [Crime::Disable, Crime::Board, Crime::Kill, Crime::Shoot]
                .map(|crime| govts.penalty(FED, crime)),
            [3, 5, 7, 2]
        );
        assert_eq!(govts.penalty(None, Crime::Kill), 0);
        assert_eq!(govts.penalty(UNKNOWN, Crime::Kill), 0);
    }

    #[test]
    fn the_flags_are_the_bibles() {
        assert_eq!(
            [
                XENOPHOBIC,
                NOSY,
                ALWAYS_ATTACKS_PLAYER,
                WARSHIPS_RETREAT,
                IGNORED_WHEN_ATTACKED,
                NEVER_ATTACKS_PLAYER,
                INTERCEPTORS_RETREAT,
                DERELICT
            ],
            [
                0x0001, 0x0002, 0x0004, 0x0010, 0x0020, 0x0040, 0x0100, 0x0800
            ]
        );
    }
}
