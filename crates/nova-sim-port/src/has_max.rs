//! Whether the player holds the most of an outfit it may: `_HasMaxOfItem`
//! (@0x4512-0x4894 in the `EV Nova` executable), ported whole, once.
//!
//! The one check serves every caller the original has: the outfitter's
//! buy check and its info box's words (`_CanBuyOutfitItem` @0x4e87d,
//! `_OutfitDialogUpdate` @0x57dd6; see [`outfitter`](crate::outfitter)),
//! `H`'s hold (`_EvalSetExp` @0x15648, [`item_limit`]; see
//! [`ship_change`](crate::ship_change)), a boarding grant's pick
//! (`_DoPlunderDialog` @0x93119, @0x93192; see [`grant`](crate::grant))
//! and a boarding's Ammo take (@0x93d9f; `nova_sim::Session::plunder`).
//!
//! [`has_max_of_item`] asks, in the original's order, and stops at the
//! first that holds ([`AtMax`]):
//!
//! 1. **Range** (@0x451b-0x4547): an outfit ID outside 128-639, an index
//!    above 511 read unsigned, is at its most, with a limit of none.
//! 2. **Ammunition** (@0x457e-0x45e4): an outfit whose *first* mod is
//!    `ModType` 3 naming a weapon of `MaxAmmo` above none is capped at that
//!    `MaxAmmo` times the player's count of the weapon, its weapon array
//!    +0xc8, stock weapons included ([`ammo_cap`]). A `ModType` 3 in a
//!    later slot is never capped.
//! 3. **`Max`** (@0x45e9-0x46c3), as the `ModType` 27 outfits owned raise
//!    it ([`raised_max`]).
//! 4. **Guns** (@0x46c8-0x47b7), for an outfit flagged
//!    [`OutfitFlags::GUN`]: a limit (`MaxGun`, plus the first
//!    [`MORE_GUNS`](crate::outfitter::MORE_GUNS) `ModVal` of each outfit
//!    owned) of none or less, or the outfits owned flagged as guns, the
//!    outfit itself included, at the limit or above.
//! 5. **Turrets** (@0x47bd-0x4866), the same with `MaxTur`,
//!    [`MORE_TURRETS`](crate::outfitter::MORE_TURRETS) and
//!    [`OutfitFlags::TURRET`], once the gun limit has passed.
//!
//! Otherwise it is not at its most, and its limit is the count owned
//! (@0x4868-0x4886). The original's sums and products are 16-bit
//! (`imulw` @0x45aa, `addw`) and wrap; here they are wide and saturate.

use std::collections::BTreeMap;

use crate::catalog::{OutfitId, OutfitRecord, WeaponId};
use crate::combat::armament::MOD_AMMO;
use crate::outfitter::{Armed, Hardpoints, OutfitFlags, raised_max};
use crate::rulebook::RuleSource;

/// The first and last outfit IDs `_HasMaxOfItem` reads a record for: an
/// index of 0 to 511 (@0x451b-0x4547).
const IDS: core::ops::RangeInclusive<i16> = 128..=639;

/// What `_HasMaxOfItem` reads, beyond the outfits owned.
#[derive(Clone, Copy)]
pub struct ItemRoom<'a> {
    /// The ship class's `MaxGun` and `MaxTur`.
    pub hardpoints: Hardpoints,
    /// How many of each weapon the ship carries, as its weapon array
    /// holds them, stock weapons included.
    pub launchers: &'a BTreeMap<WeaponId, u32>,
    /// Each weapon's `MaxAmmo`; none for no limit of its own.
    pub max_ammo: &'a dyn Fn(WeaponId) -> u32,
    /// How the `ModType` 27 outfits raise a `Max`
    /// ([`RuleKey::RaisedMax`](crate::RuleKey::RaisedMax)).
    pub raised_max: RuleSource,
}

/// What [`has_max_of_item`] reads of one outfit on the player's ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemFacts {
    /// The outfit.
    pub id: OutfitId,
    /// How many the player owns.
    pub owned: i64,
    /// Its `Max`, as the `ModType` 27 outfits owned raise it.
    pub max: i64,
    /// Its ammunition cap, when it has one ([`ammo_cap`]).
    pub ammo_cap: Option<i64>,
    /// Its `Flags`: the check reads [`OutfitFlags::GUN`] and
    /// [`OutfitFlags::TURRET`].
    pub flags: u16,
    /// The guns and turrets the ship carries, and its limits.
    pub armed: Armed,
}

impl ItemFacts {
    /// What the check reads of `record`, `owned` of `records` being owned
    /// and `room` the rest.
    #[must_use]
    pub fn of(
        record: &OutfitRecord,
        owned: &BTreeMap<OutfitId, u16>,
        records: &[OutfitRecord],
        room: &ItemRoom,
    ) -> Self {
        Self {
            id: record.id,
            owned: i64::from(owned.get(&record.id).copied().unwrap_or_default()),
            max: raised_max(record, owned, records, room.raised_max),
            ammo_cap: ammo_cap(record, room),
            flags: record.flags,
            armed: Armed::of(room.hardpoints, owned, records),
        }
    }
}

/// Which of `_HasMaxOfItem`'s limits an outfit is at, each with what its
/// `*lim` out-parameter holds ([`AtMax::limit`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtMax {
    /// Its ID is outside 128-639 (@0x451b-0x4547).
    OutOfRange,
    /// It is at its ammunition cap, this one (@0x45cd-0x45e4).
    AmmoCap(i64),
    /// It is at its `Max` as raised, this one (@0x46a5-0x46c3).
    Max(i64),
    /// It is a gun and the guns are at this limit, or the limit is none or
    /// less (@0x47aa-0x47b7, @0x4842-0x4866).
    Guns(i64),
    /// It is a turret and the turrets are at this limit, or the limit is
    /// none or less (@0x4820-0x4866).
    Turrets(i64),
}

impl AtMax {
    /// What `_HasMaxOfItem` gives through `*lim`: none out of range, and
    /// none for a gun or turret limit of none or less; otherwise the limit
    /// reached.
    #[must_use]
    pub fn limit(self) -> i64 {
        match self {
            Self::OutOfRange => 0,
            Self::AmmoCap(limit) | Self::Max(limit) => limit,
            Self::Guns(limit) | Self::Turrets(limit) => limit.max(0),
        }
    }
}

/// Which limit of `_HasMaxOfItem` (@0x4512-0x4894) the outfit `facts`
/// describes is at, the first that holds in the original's order (see the
/// module docs); none when it may have more.
#[must_use]
pub fn has_max_of_item(facts: &ItemFacts) -> Option<AtMax> {
    if !IDS.contains(&facts.id.0) {
        return Some(AtMax::OutOfRange);
    }
    if let Some(cap) = facts.ammo_cap
        && facts.owned >= cap
    {
        return Some(AtMax::AmmoCap(cap));
    }
    if facts.owned >= facts.max {
        return Some(AtMax::Max(facts.max));
    }
    let armed = facts.armed;
    let full = |flag: u16, owned: i32, limit: i32| {
        facts.flags & flag != 0 && (limit <= 0 || owned >= limit)
    };
    if full(OutfitFlags::GUN, armed.guns, armed.gun_limit) {
        return Some(AtMax::Guns(i64::from(armed.gun_limit)));
    }
    if full(OutfitFlags::TURRET, armed.turrets, armed.turret_limit) {
        return Some(AtMax::Turrets(i64::from(armed.turret_limit)));
    }
    None
}

/// `record`'s ammunition cap, when it has one (@0x457e-0x45e4): its
/// **first** mod must be [`MOD_AMMO`] naming a weapon whose `MaxAmmo` is
/// above none, and the cap is that `MaxAmmo` times the ship's count of the
/// weapon (`room`'s launchers), a wide product where the original's
/// `imulw` (@0x45aa) is 16-bit.
#[must_use]
pub fn ammo_cap(record: &OutfitRecord, room: &ItemRoom) -> Option<i64> {
    let (MOD_AMMO, weapon) = record.mods[0] else {
        return None;
    };
    let weapon = WeaponId(weapon);
    let max_ammo = (room.max_ammo)(weapon);
    (max_ammo > 0).then(|| {
        let launchers = room.launchers.get(&weapon).copied().unwrap_or_default();
        i64::from(max_ammo).saturating_mul(i64::from(launchers))
    })
}

/// The limit `_HasMaxOfItem` gives `record` through `*lim`, `owned` of
/// `records` being owned and `room` the rest it reads: the limit it is at
/// ([`AtMax::limit`]), or the count owned.
#[must_use]
pub fn item_limit(
    record: &OutfitRecord,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
    room: &ItemRoom,
) -> i64 {
    let facts = ItemFacts::of(record, owned, records, room);
    has_max_of_item(&facts).map_or(facts.owned, AtMax::limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outfitter::{MORE_GUNS, MORE_TURRETS, RAISES_MAX};
    use nova_sim_catalog::fixture::outfit;

    /// Weapon 140's `MaxAmmo` is 3; 141's is none.
    fn max_ammo(weapon: WeaponId) -> u32 {
        if weapon == WeaponId(140) { 3 } else { 0 }
    }

    /// A ship of `MaxGun` 2 and `MaxTur` 1 carrying `launchers`.
    fn room(launchers: &BTreeMap<WeaponId, u32>) -> ItemRoom<'_> {
        ItemRoom {
            hardpoints: Hardpoints {
                guns: 2,
                turrets: 1,
            },
            launchers,
            max_ammo: &max_ammo,
            raised_max: RuleSource::Engine,
        }
    }

    /// Facts of outfit 200 with `owned` of a `Max` of 10, nothing else.
    fn facts(owned: i64) -> ItemFacts {
        ItemFacts {
            id: OutfitId(200),
            owned,
            max: 10,
            ammo_cap: None,
            flags: 0,
            armed: Armed {
                guns: 0,
                turrets: 0,
                gun_limit: 2,
                turret_limit: 1,
            },
        }
    }

    #[test]
    fn an_id_outside_128_to_639_is_at_its_most_with_a_limit_of_none() {
        for id in [127, 640, -1, 1000] {
            let facts = ItemFacts {
                id: OutfitId(id),
                ..facts(0)
            };
            assert_eq!(has_max_of_item(&facts), Some(AtMax::OutOfRange), "{id}");
        }
        for id in [128, 639] {
            let facts = ItemFacts {
                id: OutfitId(id),
                ..facts(0)
            };
            assert_eq!(has_max_of_item(&facts), None, "{id}");
        }
        assert_eq!(AtMax::OutOfRange.limit(), 0);
    }

    #[test]
    fn an_ammunition_cap_holds_at_the_cap_and_lets_one_fewer_through() {
        let capped = |owned| ItemFacts {
            ammo_cap: Some(6),
            ..facts(owned)
        };
        assert_eq!(has_max_of_item(&capped(6)), Some(AtMax::AmmoCap(6)));
        assert_eq!(has_max_of_item(&capped(7)), Some(AtMax::AmmoCap(6)));
        assert_eq!(has_max_of_item(&capped(5)), None);
        assert_eq!(
            has_max_of_item(&ItemFacts {
                max: 4,
                ..capped(6)
            }),
            Some(AtMax::AmmoCap(6)),
            "the cap before the Max"
        );
        assert_eq!(AtMax::AmmoCap(6).limit(), 6);
    }

    #[test]
    fn the_ammunition_cap_is_max_ammo_times_the_launchers_of_a_first_mod() {
        let launchers = BTreeMap::from([(WeaponId(140), 2), (WeaponId(141), 4)]);
        let room = room(&launchers);
        assert_eq!(ammo_cap(&outfit(200, &[(MOD_AMMO, 140)]), &room), Some(6));
        assert_eq!(
            ammo_cap(&outfit(200, &[(MOD_AMMO, 141)]), &room),
            None,
            "a weapon of MaxAmmo 0"
        );
        assert_eq!(
            ammo_cap(&outfit(200, &[(1, 140), (MOD_AMMO, 140)]), &room),
            None,
            "a later slot"
        );
        assert_eq!(ammo_cap(&outfit(200, &[(1, 140)]), &room), None);
        let none = BTreeMap::new();
        assert_eq!(
            ammo_cap(&outfit(200, &[(MOD_AMMO, 140)]), &self::room(&none)),
            Some(0),
            "no launcher caps it at none"
        );
    }

    #[test]
    fn stock_launchers_counted_in_the_weapon_array_raise_the_cap() {
        let one = BTreeMap::from([(WeaponId(140), 1)]);
        let three = BTreeMap::from([(WeaponId(140), 3)]);
        let ammo = outfit(200, &[(MOD_AMMO, 140)]);
        assert_eq!(ammo_cap(&ammo, &room(&one)), Some(3));
        assert_eq!(ammo_cap(&ammo, &room(&three)), Some(9));
    }

    #[test]
    fn the_ammunition_cap_is_a_wide_product() {
        let big = |_: WeaponId| 16_384;
        let launchers = BTreeMap::from([(WeaponId(140), 2)]);
        let room = ItemRoom {
            max_ammo: &big,
            ..room(&launchers)
        };
        assert_eq!(
            ammo_cap(&outfit(200, &[(MOD_AMMO, 140)]), &room),
            Some(32_768)
        );
    }

    #[test]
    fn the_max_as_raised_holds_and_a_max_of_none_is_reached_with_none_owned() {
        assert_eq!(has_max_of_item(&facts(10)), Some(AtMax::Max(10)));
        assert_eq!(has_max_of_item(&facts(9)), None);
        let none = ItemFacts { max: 0, ..facts(0) };
        assert_eq!(has_max_of_item(&none), Some(AtMax::Max(0)));
        let below = ItemFacts {
            max: -1,
            ..facts(0)
        };
        assert_eq!(has_max_of_item(&below), Some(AtMax::Max(-1)));
        assert_eq!(AtMax::Max(10).limit(), 10);
        let target = OutfitRecord {
            max: 3,
            ..outfit(200, &[])
        };
        let raiser = outfit(300, &[(RAISES_MAX, 200)]);
        let records = [target.clone(), raiser];
        let owned = BTreeMap::from([(OutfitId(300), 2)]);
        let launchers = BTreeMap::new();
        assert_eq!(
            ItemFacts::of(&target, &owned, &records, &room(&launchers)).max,
            6,
            "two raisers double it"
        );
    }

    /// The facts of a gun outfit 200 with `guns` owned against `limit`.
    fn gun(guns: i32, limit: i32) -> ItemFacts {
        ItemFacts {
            flags: OutfitFlags::GUN,
            armed: Armed {
                guns,
                gun_limit: limit,
                ..facts(0).armed
            },
            ..facts(0)
        }
    }

    #[test]
    fn a_gun_limit_of_none_or_less_is_at_its_most_with_a_limit_of_none() {
        assert_eq!(has_max_of_item(&gun(0, 0)), Some(AtMax::Guns(0)));
        assert_eq!(has_max_of_item(&gun(-3, -1)), Some(AtMax::Guns(-1)));
        assert_eq!(AtMax::Guns(-1).limit(), 0);
        assert_eq!(AtMax::Guns(0).limit(), 0);
    }

    #[test]
    fn guns_at_the_limit_are_at_its_most_and_one_fewer_is_not() {
        assert_eq!(has_max_of_item(&gun(2, 2)), Some(AtMax::Guns(2)));
        assert_eq!(has_max_of_item(&gun(3, 2)), Some(AtMax::Guns(2)));
        assert_eq!(has_max_of_item(&gun(1, 2)), None);
        assert_eq!(AtMax::Guns(2).limit(), 2);
        let unflagged = ItemFacts {
            flags: 0,
            ..gun(5, 2)
        };
        assert_eq!(has_max_of_item(&unflagged), None, "the flag decides");
    }

    #[test]
    fn the_gun_limit_is_max_gun_and_the_first_more_guns_mod_once_an_outfit() {
        let gun = OutfitRecord {
            flags: OutfitFlags::GUN,
            ..outfit(200, &[])
        };
        let mount = outfit(300, &[(MORE_GUNS, 1), (MORE_GUNS, 5)]);
        let records = [gun.clone(), mount];
        let launchers = BTreeMap::new();
        let room = room(&launchers);
        let owned = BTreeMap::from([(OutfitId(200), 2), (OutfitId(300), 4)]);
        let facts = ItemFacts::of(&gun, &owned, &records, &room);
        assert_eq!((facts.armed.guns, facts.armed.gun_limit), (2, 3));
        assert_eq!(has_max_of_item(&facts), None, "MaxGun 2 + 1, two owned");
        let owned = BTreeMap::from([(OutfitId(200), 3), (OutfitId(300), 4)]);
        assert_eq!(
            has_max_of_item(&ItemFacts::of(&gun, &owned, &records, &room)),
            Some(AtMax::Guns(3))
        );
    }

    /// The facts of a turret outfit 200 with `turrets` owned against
    /// `limit`.
    fn turret(turrets: i32, limit: i32) -> ItemFacts {
        ItemFacts {
            flags: OutfitFlags::TURRET,
            armed: Armed {
                turrets,
                turret_limit: limit,
                ..facts(0).armed
            },
            ..facts(0)
        }
    }

    #[test]
    fn a_turret_is_checked_as_a_gun_is_with_its_own_limit() {
        assert_eq!(has_max_of_item(&turret(1, 1)), Some(AtMax::Turrets(1)));
        assert_eq!(has_max_of_item(&turret(0, 1)), None);
        assert_eq!(has_max_of_item(&turret(0, 0)), Some(AtMax::Turrets(0)));
        assert_eq!(AtMax::Turrets(-2).limit(), 0);
        assert_eq!(AtMax::Turrets(1).limit(), 1);
        let turret_record = OutfitRecord {
            flags: OutfitFlags::TURRET,
            ..outfit(200, &[])
        };
        let mount = outfit(300, &[(MORE_TURRETS, 2)]);
        let records = [turret_record.clone(), mount];
        let launchers = BTreeMap::new();
        let owned = BTreeMap::from([(OutfitId(200), 2), (OutfitId(300), 1)]);
        let facts = ItemFacts::of(&turret_record, &owned, &records, &room(&launchers));
        assert_eq!((facts.armed.turrets, facts.armed.turret_limit), (2, 3));
    }

    #[test]
    fn an_outfit_flagged_both_stops_at_the_gun_limit_first() {
        let both = |guns, turrets| ItemFacts {
            flags: OutfitFlags::GUN | OutfitFlags::TURRET,
            armed: Armed {
                guns,
                turrets,
                gun_limit: 2,
                turret_limit: 1,
            },
            ..facts(0)
        };
        assert_eq!(has_max_of_item(&both(2, 1)), Some(AtMax::Guns(2)));
        assert_eq!(
            has_max_of_item(&both(1, 1)),
            Some(AtMax::Turrets(1)),
            "the turret limit once the gun limit passes"
        );
        assert_eq!(has_max_of_item(&both(1, 0)), None);
    }

    #[test]
    fn an_outfit_at_none_of_them_has_room_and_its_limit_is_the_count_owned() {
        let record = outfit(200, &[]);
        let records = [record.clone()];
        let launchers = BTreeMap::new();
        let room = room(&launchers);
        let owned = BTreeMap::from([(OutfitId(200), 4)]);
        assert_eq!(item_limit(&record, &owned, &records, &room), 4);
        let owned = BTreeMap::from([(OutfitId(200), 12)]);
        assert_eq!(item_limit(&record, &owned, &records, &room), 10, "Max");
    }
}
