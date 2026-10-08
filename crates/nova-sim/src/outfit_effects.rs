//! What granting one outfit does: the original's `_GrantOutfitItem`
//! (@0x44d4f in the `EV Nova` executable), which the outfitter's
//! purchases, boarding grants and the `G` set operator all go through.
//!
//! Three `ModType`s act at once instead of being added to the ship's
//! outfits ([`GrantEffect::of`]). The steps run in this order:
//!
//! 1. **Map ([`MAP`], 16)**, @0x44d9b-0x44ea9. Only the first slot of
//!    `ModType` 16 is read ([`MapReach::of`]). Its `ModVal`:
//!    - above 0 explores the systems within that many jumps of the
//!      player's ([`MapReach::Jumps`]), by `_AutoSetExploration`;
//!    - -1 explores every existing independent system that is inhabited
//!      ([`MapReach::InhabitedIndependent`], `_SystemIsInhabited`);
//!    - -1000 and below explores every system whose government has the
//!      class -`ModVal` - 1000 in one of its four `Class` fields
//!      ([`MapReach::GovtClass`]; the government's +0x26..+0x2c). It is a
//!      government *class*, as the Bible says, not a government ID;
//!    - 0, and -2 to -999, explore nothing ([`MapReach::Nothing`]).
//!
//!    Any map slot, whatever its `ModVal`, keeps the outfit from being
//!    added, so by the engine a map that explores nothing is used up all
//!    the same. Whether such a map is instead left to be added as a plain
//!    item is the [`RuleKey::InvalidMap`](crate::RuleKey::InvalidMap)
//!    rule: the obvious fix, as the Bible says nothing of those values.
//! 2. **Paint ([`PAINT`], 43)**, @0x44ec1 and @0x44f9a-0x44fd5. Only the
//!    first slot is read: the ship is painted red (`ModVal` >> 10) & 31,
//!    green (`ModVal` >> 5) & 31 and blue `ModVal` & 31 ([`Rgb15::of`]),
//!    and the outfit is not added. (0, 0, 0) is unpainted, as
//!    `_GetShipPaintColor` (@0x7d0c) draws it as a new pilot's (32, 32,
//!    32). A map and a paint can both apply.
//! 3. **Clean legal record ([`CLEAN_RECORD`], 21)**, @0x44ee2-0x44f8e,
//!    only when neither a map nor a paint slot was found. *Every* slot of
//!    `ModType` 21 applies ([`CleanScope::of`]): each record below none
//!    with the government its `ModVal` names, or with every government
//!    for -1, is raised to none, and records above none stay
//!    ([`clean_records`]). The outfit is not added, even when its
//!    `ModVal` names no government. `_LoadObjectData` (@0x78de3-0x78df7)
//!    takes 128 off a `ModVal` above 127 for `ModType` 1, 3 and 21, so
//!    the value is matched as a government's index: 128 and up is that
//!    government's ID, and 0 to 127 the ID 128 above it.
//! 4. **Otherwise** (@0x44fdf) one more is owned. The grant itself checks
//!    neither the outfit's `Max`, nor the free mass, nor `Flags` 0x0010.
//!
//! **Stated divergence.** The original keeps the legal record per system
//! and cleans the records of the systems the government holds (or of
//! every system); here the record is per government (see
//! [`legal`](crate::legal)), so the record with the government named is
//! cleaned even when it holds no system, and -1 cleans every
//! government's record but leaves independent systems, which have no
//! record here.

use std::collections::BTreeMap;

use crate::catalog::{GovtId, OutfitRecord};
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// The disputed rules of granting and removing outfits that the session
/// follows (see [`Session::with_outfit_rules`](crate::Session::with_outfit_rules)):
/// the engine's by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutfitRules {
    /// Which systems a map explores, and which are inhabited
    /// ([`RuleKey::MapExplore`]; see [`exploration`](crate::exploration)).
    pub map_explore: RuleSource,
    /// Whether a map that explores nothing is used up
    /// ([`RuleKey::InvalidMap`]; see [`GrantEffect::of`]).
    pub invalid_map: RuleSource,
    /// Whether `G` may pass the outfit's `Max` and the free mass
    /// ([`RuleKey::GrantMax`], which the boarding grant reads too).
    pub grant_max: RuleSource,
    /// Whether `D` pays for the outfit it removes
    /// ([`RuleKey::RemoveRefund`]).
    pub remove_refund: RuleSource,
}

impl OutfitRules {
    /// The rules `rulebook` chooses: its [`RuleKey::MapExplore`],
    /// [`RuleKey::InvalidMap`], [`RuleKey::GrantMax`] and
    /// [`RuleKey::RemoveRefund`] entries.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            map_explore: rulebook.source_for(RuleKey::MapExplore),
            invalid_map: rulebook.source_for(RuleKey::InvalidMap),
            grant_max: rulebook.source_for(RuleKey::GrantMax),
            remove_refund: rulebook.source_for(RuleKey::RemoveRefund),
        }
    }
}

/// `ModType` 16: a map, which explores systems.
pub const MAP: i16 = 16;
/// `ModType` 21: cleans the legal record.
pub const CLEAN_RECORD: i16 = 21;
/// `ModType` 43: paints the ship.
pub const PAINT: i16 = 43;
/// A map's `ModVal` that explores every inhabited independent system.
pub const INHABITED_INDEPENDENT: i16 = -1;
/// A map's `ModVal` at and below which it explores a government class's
/// systems, the class being this less the `ModVal` (@0x44e28-0x44e35).
pub const GOVT_CLASS_BASE: i16 = -1000;
/// A clean-record `ModVal` that cleans every government's record.
pub const EVERY_GOVT: i16 = -1;

/// Which systems a map explores (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapReach {
    /// The systems within this many jumps of the player's.
    Jumps(u16),
    /// Every inhabited independent system.
    InhabitedIndependent,
    /// Every system whose government has this class.
    GovtClass(i16),
    /// None: the `ModVal` is 0, or -2 to -999.
    Nothing,
}

impl MapReach {
    /// What a map of `ModVal` `value` explores (@0x44da6-0x44e2c).
    #[must_use]
    pub fn of(value: i16) -> Self {
        if value > 0 {
            Self::Jumps(value as u16)
        } else if value == INHABITED_INDEPENDENT {
            Self::InhabitedIndependent
        } else if value <= GOVT_CLASS_BASE {
            Self::GovtClass((i32::from(GOVT_CLASS_BASE) - i32::from(value)) as i16)
        } else {
            Self::Nothing
        }
    }
}

/// A ship's paint: red, green and blue, each 0 to 31.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb15 {
    /// Red, 0 to 31.
    pub r: u8,
    /// Green, 0 to 31.
    pub g: u8,
    /// Blue, 0 to 31.
    pub b: u8,
}

impl Rgb15 {
    /// The paint a `ModVal` of `value` gives (@0x44f9a-0x44fd5): `None`,
    /// unpainted, for (0, 0, 0).
    #[must_use]
    pub fn of(value: i16) -> Option<Self> {
        let bits = value as u16;
        let paint = Self {
            r: ((bits >> 10) & 31) as u8,
            g: ((bits >> 5) & 31) as u8,
            b: (bits & 31) as u8,
        };
        (paint != Self { r: 0, g: 0, b: 0 }).then_some(paint)
    }
}

/// Whose legal record a clean-record outfit cleans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanScope {
    /// Every government's.
    All,
    /// This government's.
    Govt(GovtId),
}

impl CleanScope {
    /// Whose record a `ModVal` of `value` cleans (see the module docs):
    /// none for a value below 0 but -1.
    #[must_use]
    pub fn of(value: i16) -> Option<Self> {
        match value {
            EVERY_GOVT => Some(Self::All),
            128.. => Some(Self::Govt(GovtId(value))),
            0..128 => Some(Self::Govt(GovtId(value + 128))),
            _ => None,
        }
    }

    /// Whether it cleans the record with `govt`.
    fn covers(self, govt: GovtId) -> bool {
        match self {
            Self::All => true,
            Self::Govt(id) => id == govt,
        }
    }
}

/// What granting one of an outfit does (see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantEffect {
    /// The systems it explores, when it is a map.
    pub map: Option<MapReach>,
    /// The paint it gives the ship, when it paints: `Some(None)` unpaints.
    pub paint: Option<Option<Rgb15>>,
    /// Whose legal records it cleans: only with no map and no paint.
    pub clean: Vec<CleanScope>,
    /// Whether one more is owned: only when it is none of the three.
    pub added: bool,
}

impl GrantEffect {
    /// What granting one of `record` does, a map that explores nothing
    /// used up or not as `invalid_map` says (see the module docs).
    #[must_use]
    pub fn of(record: &OutfitRecord, invalid_map: RuleSource) -> Self {
        let first = |kind: i16| {
            record
                .mods
                .iter()
                .find(|&&(mod_type, _)| mod_type == kind)
                .map(|&(_, value)| value)
        };
        let map = first(MAP)
            .map(MapReach::of)
            .filter(|&reach| invalid_map == RuleSource::Engine || reach != MapReach::Nothing);
        let paint = first(PAINT).map(Rgb15::of);
        let acts_at_once = map.is_some() || paint.is_some();
        let cleaning = record
            .mods
            .iter()
            .filter(|&&(mod_type, _)| mod_type == CLEAN_RECORD);
        let clean = if acts_at_once {
            Vec::new()
        } else {
            cleaning
                .clone()
                .filter_map(|&(_, value)| CleanScope::of(value))
                .collect()
        };
        let added = !acts_at_once && cleaning.count() == 0;
        Self {
            map,
            paint,
            clean,
            added,
        }
    }
}

/// Raises each of `legal`'s records below none with a government one of
/// `scopes` covers to none (@0x44f3c-0x44f84).
pub fn clean_records(legal: &mut BTreeMap<GovtId, i16>, scopes: &[CleanScope]) {
    for (&govt, record) in legal.iter_mut() {
        if *record < 0 && scopes.iter().any(|scope| scope.covers(govt)) {
            *record = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::outfit;

    fn effect(mods: &[(i16, i16)]) -> GrantEffect {
        GrantEffect::of(&outfit(300, mods), RuleSource::Engine)
    }

    const PLAIN: GrantEffect = GrantEffect {
        map: None,
        paint: None,
        clean: Vec::new(),
        added: true,
    };

    #[test]
    fn a_plain_outfit_is_added() {
        assert_eq!(effect(&[]), PLAIN);
        assert_eq!(effect(&[(1, 128), (17, 5)]), PLAIN);
    }

    #[test]
    fn a_map_explores_as_its_mod_val_says_and_is_never_added() {
        for (value, reach) in [
            (3, MapReach::Jumps(3)),
            (1, MapReach::Jumps(1)),
            (i16::MAX, MapReach::Jumps(32_767)),
            (-1, MapReach::InhabitedIndependent),
            (-1000, MapReach::GovtClass(0)),
            (-1003, MapReach::GovtClass(3)),
            (i16::MIN, MapReach::GovtClass(31_768)),
            (0, MapReach::Nothing),
            (-2, MapReach::Nothing),
            (-5, MapReach::Nothing),
            (-999, MapReach::Nothing),
        ] {
            assert_eq!(
                effect(&[(MAP, value)]),
                GrantEffect {
                    map: Some(reach),
                    added: false,
                    ..PLAIN
                },
                "{value}"
            );
        }
    }

    #[test]
    fn only_the_first_map_slot_is_read() {
        assert_eq!(
            effect(&[(1, 5), (MAP, 2), (MAP, -1)]).map,
            Some(MapReach::Jumps(2))
        );
        assert_eq!(effect(&[(MAP, 0), (MAP, 4)]).map, Some(MapReach::Nothing));
    }

    #[test]
    fn a_map_that_explores_nothing_is_added_as_a_plain_item_by_the_other_reading() {
        let invalid = outfit(300, &[(MAP, -5), (MAP, 4)]);
        assert_eq!(GrantEffect::of(&invalid, RuleSource::Bible), PLAIN);
        let valid = outfit(300, &[(MAP, 4)]);
        assert_eq!(
            GrantEffect::of(&valid, RuleSource::Bible).map,
            Some(MapReach::Jumps(4)),
            "a map of any reach is still one"
        );
        let painting = outfit(300, &[(MAP, 0), (PAINT, 1)]);
        assert_eq!(
            GrantEffect::of(&painting, RuleSource::Bible),
            GrantEffect {
                paint: Some(Rgb15::of(1)),
                added: false,
                ..PLAIN
            },
            "the paint still applies"
        );
    }

    #[test]
    fn a_paint_paints_its_five_bit_colours_and_none_unpaints() {
        assert_eq!(
            effect(&[(PAINT, 0x7FFF)]),
            GrantEffect {
                paint: Some(Some(Rgb15 {
                    r: 31,
                    g: 31,
                    b: 31
                })),
                added: false,
                ..PLAIN
            }
        );
        assert_eq!(effect(&[(PAINT, 0)]).paint, Some(None), "unpainted");
        assert_eq!(
            Rgb15::of((3 << 10) | (5 << 5) | 7),
            Some(Rgb15 { r: 3, g: 5, b: 7 })
        );
        assert_eq!(
            Rgb15::of(-1),
            Some(Rgb15 {
                r: 31,
                g: 31,
                b: 31
            })
        );
        assert_eq!(Rgb15::of(1 << 15), None, "the top bit is no colour");
        assert_eq!(
            effect(&[(PAINT, 1), (PAINT, 2)]).paint,
            Some(Rgb15::of(1)),
            "only the first"
        );
    }

    #[test]
    fn a_map_and_a_paint_both_apply_and_the_legal_record_is_left() {
        assert_eq!(
            effect(&[(CLEAN_RECORD, -1), (PAINT, 1), (MAP, 2)]),
            GrantEffect {
                map: Some(MapReach::Jumps(2)),
                paint: Some(Rgb15::of(1)),
                clean: Vec::new(),
                added: false,
            }
        );
        assert_eq!(
            effect(&[(CLEAN_RECORD, -1), (MAP, 0)]).clean,
            [],
            "a map of no reach is a map"
        );
    }

    #[test]
    fn each_clean_record_slot_names_a_government_or_all() {
        assert_eq!(
            effect(&[(CLEAN_RECORD, 130)]),
            GrantEffect {
                clean: vec![CleanScope::Govt(GovtId(130))],
                added: false,
                ..PLAIN
            }
        );
        assert_eq!(
            effect(&[(CLEAN_RECORD, 2), (5, 1), (CLEAN_RECORD, -1)]).clean,
            [CleanScope::Govt(GovtId(130)), CleanScope::All]
        );
        for (value, scope) in [
            (128, Some(CleanScope::Govt(GovtId(128)))),
            (127, Some(CleanScope::Govt(GovtId(255)))),
            (0, Some(CleanScope::Govt(GovtId(128)))),
            (-1, Some(CleanScope::All)),
            (-2, None),
            (-7, None),
        ] {
            assert_eq!(CleanScope::of(value), scope, "{value}");
        }
        assert_eq!(
            effect(&[(CLEAN_RECORD, -7)]),
            GrantEffect {
                added: false,
                ..PLAIN
            },
            "naming no government, it is still not added"
        );
    }

    #[test]
    fn the_outfit_rules_follow_their_four_rulebook_keys() {
        assert_eq!(
            OutfitRules::from_rulebook(&Rulebook::default()),
            OutfitRules::default()
        );
        assert_eq!(
            OutfitRules::default(),
            OutfitRules {
                map_explore: RuleSource::Engine,
                invalid_map: RuleSource::Engine,
                grant_max: RuleSource::Engine,
                remove_refund: RuleSource::Engine,
            }
        );
        let bible = |key| {
            OutfitRules::from_rulebook(&Rulebook::default().with_override(key, RuleSource::Bible))
        };
        assert_eq!(
            bible(RuleKey::MapExplore),
            OutfitRules {
                map_explore: RuleSource::Bible,
                ..OutfitRules::default()
            }
        );
        assert_eq!(
            bible(RuleKey::InvalidMap),
            OutfitRules {
                invalid_map: RuleSource::Bible,
                ..OutfitRules::default()
            }
        );
        assert_eq!(
            bible(RuleKey::GrantMax),
            OutfitRules {
                grant_max: RuleSource::Bible,
                ..OutfitRules::default()
            }
        );
        assert_eq!(
            bible(RuleKey::RemoveRefund),
            OutfitRules {
                remove_refund: RuleSource::Bible,
                ..OutfitRules::default()
            }
        );
        assert_eq!(bible(RuleKey::CrimeGains), OutfitRules::default());
    }

    fn records(pairs: &[(i16, i16)]) -> BTreeMap<GovtId, i16> {
        pairs
            .iter()
            .map(|&(id, record)| (GovtId(id), record))
            .collect()
    }

    #[test]
    fn cleaning_raises_only_the_records_below_none() {
        let mut legal = records(&[(128, -50), (129, -1), (130, 20), (131, 0)]);
        clean_records(&mut legal, &[CleanScope::Govt(GovtId(128))]);
        assert_eq!(legal, records(&[(128, 0), (129, -1), (130, 20), (131, 0)]));
        clean_records(&mut legal, &[CleanScope::Govt(GovtId(130))]);
        assert_eq!(legal, records(&[(128, 0), (129, -1), (130, 20), (131, 0)]));
        let mut legal = records(&[(128, -50), (129, -1), (130, 20)]);
        clean_records(&mut legal, &[CleanScope::All]);
        assert_eq!(legal, records(&[(128, 0), (129, 0), (130, 20)]));
        let mut legal = records(&[(128, -50), (129, -1), (130, -3)]);
        clean_records(
            &mut legal,
            &[CleanScope::Govt(GovtId(129)), CleanScope::Govt(GovtId(130))],
        );
        assert_eq!(legal, records(&[(128, -50), (129, 0), (130, 0)]));
        clean_records(&mut legal, &[]);
        assert_eq!(legal, records(&[(128, -50), (129, 0), (130, 0)]));
    }
}
