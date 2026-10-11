//! Changing the player's ship outside the shipyard, as the `C`, `E` and
//! `H` set operators do: which outfits the change carries over.
//!
//! [`carried_outfits`] is the outfit half of the original's shared
//! ship-change block in `_EvalSetExp` (@0x15493-0x156da), apart from the
//! session: what is kept ([`OutfitCarry`]), the new class's default items
//! added on top (`+=`, repeated slots summed, @0x15609), with [`Fold`]
//! the new class's stock weapons folded in, and, with [`Limits`], every
//! outfit owned held to its limit afterwards. The session applies it and
//! changes the class (see the session's `ship_change` module).
//!
//! **The arrays and the fold.** The original keeps the ship's weapons
//! and rounds in two arrays beside the outfit counts
//! ([`Arrays`]): `_SystemInfoToShipStats` (@0xca33, called @0x154ef after
//! the carry) builds them from each owned outfit's **first** mod alone,
//! `ModType` 1 counting for the weapon it names and 3 for the rounds
//! (@0xca8d-0xcac9). `E` and `H` then add the new class's `WCount`s and
//! `AmmoLoad`s to them signed (@0x1551c-0x155de, here
//! [`Arsenal::stock_added`](crate::combat::armament::Arsenal::stock_added)),
//! add the default items to the outfits only (@0x15609), and call
//! `_ShipStatsToSystemInfo` (@0xcaee-0xcce7, called @0x15622), which
//! [`fold`] is:
//!
//! 1. For each outfit by ascending ID that is owned and whose first mod
//!    is `ModType` 1 (or 3), the array's count a for its weapon: an
//!    outfit owning a or more becomes a and the array none; one owning
//!    fewer keeps its count, taken from the array (@0xcb55-0xcbf0).
//! 2. Every negative count becomes none (@0xcbf6-0xcc0c).
//! 3. Whatever is left in the arrays goes to the first outfit, by
//!    ascending ID, owned or not, whose first mod names the weapon; with
//!    none it is lost (@0xcc28-0xccd7).
//!
//! So the stock weapons and loads land on the lowest-ID outfit whose
//! first mod names them, a negative `WCount` or `AmmoLoad` takes that
//! many from the outfits holding them, floored at none
//! ([`RuleKey::ShipChangeNegativeStock`](crate::RuleKey::ShipChangeNegativeStock)),
//! and a default item that is a weapon or ammunition outfit, being in no
//! array, is pulled back to the array's count and lost, unless the array
//! has a surplus left after the holders before it
//! ([`RuleKey::ShipChangeDefaultWeapons`](crate::RuleKey::ShipChangeDefaultWeapons);
//! by its other reading the defaults' first mods are added to the arrays
//! too). An outfit naming a weapon only in a later mod is no holder and
//! is left alone. The original's fold skips a first mod naming a weapon
//! index past 255 (`wëap` 384 or above); that bound is not modelled.
//!
//! **The clamp** (@0x15627-0x15668). For each outfit owned, by ascending
//! ID, the original calls `_HasMaxOfItem(i, &lim, 0)` (@0x15648), reading
//! its second argument, and sets `if 0 < lim < count { count = lim }`.
//! `_HasMaxOfItem` (@0x4512-0x4894) gives, by the first that holds:
//!
//! 1. **Ammunition** (@0x457e-0x45e4): an outfit whose first `ModType` is
//!    3 naming a weapon of `MaxAmmo` above 0, owned at least `MaxAmmo`
//!    times the weapon's count in the weapon array, is limited to that
//!    cap, and `Max` and `ModType` 27 are never read. So with no launcher
//!    the cap is 0 and nothing is held, and ammunition at a cap above its
//!    `Max` is held to the cap; below the cap it goes on to its `Max`.
//! 2. **`Max`** times the `ModType` 27 multiplier
//!    ([`RuleKey::RaisedMax`](crate::RuleKey::RaisedMax)), when the count
//!    owned is at it or above (@0x45e9-0x46c3). A `Max` of 0 or less is a
//!    limit of 0 or less: nothing is held.
//! 3. **Guns** (flag 0x0001, @0x46c8-0x47b7): the new class's `MaxGun`
//!    (the class is set @0x154fc, before the clamp) plus the first
//!    `ModType` 45 of each outfit type owned. A limit of 0 or less holds
//!    nothing; when the guns owned, every outfit flagged 0x0001, are at
//!    the limit or above, the limit is the **whole** gun limit, so each
//!    gun outfit is held to the slot total on its own: three of one gun on
//!    two slots become two, but two each of two guns stay four. The
//!    alternative to this quirk is `ship_change_max`'s other reading.
//! 4. **Turrets** (flag 0x0002, @0x47bd-0x4866): the same with `MaxTur`
//!    and `ModType` 46, reached only when the gun check did not return.
//! 5. Otherwise the count owned: nothing changes.
//!
//! The loop reads the outfit counts as it leaves them, so an outfit held
//! earlier in it changes the gun totals and the `ModType` 27 multipliers
//! of those after it. The weapon array it reads for the ammunition is
//! another matter: the fold reads the arrays but never writes them, and
//! only `_SystemInfoToShipStats` (@0x1566a, after the clamp) builds them
//! again from the outfits. So the launchers counted are the weapon
//! array as the adds left it, fixed through the loop: a launcher held by
//! its gun slots still counts as it was, a stock weapon no outfit holds
//! still counts, and a default launcher the fold took away does not.
//! Without a fold the launchers are the outfits carried, default items
//! included, whose first mod is `ModType` 1. A negative count is no
//! launcher. Every sum and product is wide and saturates, where the
//! original's are 16-bit and wrap.

use std::collections::BTreeMap;

use crate::catalog::{OutfitId, OutfitRecord, WeaponId};
use crate::combat::armament::{MOD_AMMO, MOD_WEAPON, StockAdded};
use crate::outfitter::{Hardpoints, ItemRoom, OutfitFlags, item_limit};
use crate::rulebook::RuleSource;

/// The outfit flags `H` keeps an outfit by, by the engine: persistent
/// (0x0004) or persistent through a mission's change of ship (0x0020),
/// the original's `testb $0x24` (@0x154d5).
pub const SET_OP_PERSISTENT: u16 = OutfitFlags::PERSISTENT | OutfitFlags::MISSION_PERSISTENT;

/// How a change of ship treats the outfits owned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutfitCarry {
    /// `C`: every outfit kept; the new class's default items not added.
    Keep,
    /// `E`: every outfit kept, the new class's default items added.
    KeepWithDefaults,
    /// `H`: only the outfits whose `Flags` share a bit with `mask` kept
    /// (an outfit with no record has none), then the new class's default
    /// items added.
    Persistent {
        /// The flags that keep an outfit.
        mask: u16,
    },
}

/// What `E` and `H` hold the outfits to, besides the outfits themselves
/// (see the module docs).
#[derive(Clone, Copy)]
pub struct Limits<'a> {
    /// The new class's `MaxGun` and `MaxTur`.
    pub hardpoints: Hardpoints,
    /// Each weapon's `MaxAmmo`; none for no limit of its own.
    pub max_ammo: &'a dyn Fn(WeaponId) -> u32,
    /// How the raised maximums are read
    /// ([`RuleKey::RaisedMax`](crate::RuleKey::RaisedMax)).
    pub raised_max: RuleSource,
}

/// What `E` and `H` fold into the outfits carried (see the module docs).
#[derive(Clone, Copy)]
pub struct Fold<'a> {
    /// What the new class's stock weapons add to the arrays.
    pub stock: &'a StockAdded,
    /// Whether the default items' first mods are added to the arrays
    /// ([`RuleKey::ShipChangeDefaultWeapons`](crate::RuleKey::ShipChangeDefaultWeapons)):
    /// by the engine they are not, and the fold takes them away.
    pub default_weapons: RuleSource,
}

/// The original's weapon and rounds arrays: how many of each weapon, and
/// of each weapon's rounds. Wide and signed, where the original's are
/// 16-bit and wrap.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arrays {
    /// Each weapon's count.
    pub weapons: BTreeMap<WeaponId, i64>,
    /// Each weapon's rounds.
    pub rounds: BTreeMap<WeaponId, i64>,
}

impl Arrays {
    /// The arrays `owned` of `records` make, each outfit counting for
    /// the weapon (or the rounds) its first mod names, as
    /// `_SystemInfoToShipStats` builds them (@0xca8d-0xcac9).
    #[must_use]
    pub fn of(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> Self {
        let mut arrays = Self::default();
        for record in records {
            if let Some(&count) = owned.get(&record.id)
                && let Some(array) = arrays.array(record.mods[0].0)
            {
                add(array, WeaponId(record.mods[0].1), i64::from(count));
            }
        }
        arrays
    }

    /// Adds `other`'s counts to these.
    fn add(&mut self, other: &Self) {
        for (&weapon, &count) in &other.weapons {
            add(&mut self.weapons, weapon, count);
        }
        for (&weapon, &count) in &other.rounds {
            add(&mut self.rounds, weapon, count);
        }
    }

    /// The array a first mod of `mod_type` counts in: none for any other
    /// mod.
    fn array(&mut self, mod_type: i16) -> Option<&mut BTreeMap<WeaponId, i64>> {
        match mod_type {
            MOD_WEAPON => Some(&mut self.weapons),
            MOD_AMMO => Some(&mut self.rounds),
            _ => None,
        }
    }
}

impl From<&StockAdded> for Arrays {
    fn from(stock: &StockAdded) -> Self {
        Self {
            weapons: stock.weapons.clone(),
            rounds: stock.rounds.clone(),
        }
    }
}

/// Adds `count` to `weapon`'s entry of `array`, saturating.
fn add(array: &mut BTreeMap<WeaponId, i64>, weapon: WeaponId, count: i64) {
    let entry = array.entry(weapon).or_default();
    *entry = entry.saturating_add(count);
}

/// The outfits `owned` become on a change of ship as `carry` says, the
/// new class's default items being `defaults`; with `fold`, the new
/// class's stock is folded in after the defaults; with `limits`, each
/// outfit with a record is then held to its limit, by ascending ID (see
/// the module docs). `records` are every outfit's.
#[must_use]
pub fn carried_outfits(
    owned: &BTreeMap<OutfitId, u16>,
    carry: OutfitCarry,
    defaults: &BTreeMap<OutfitId, u16>,
    fold: Option<&Fold>,
    limits: Option<&Limits>,
    records: &[OutfitRecord],
) -> BTreeMap<OutfitId, u16> {
    let record = |id: &OutfitId| records.iter().find(|record| record.id == *id);
    let mut outfits = owned.clone();
    if let OutfitCarry::Persistent { mask } = carry {
        outfits.retain(|id, _| record(id).is_some_and(|record| record.flags & mask != 0));
    }
    let mut arrays = Arrays::of(&outfits, records);
    if let Some(fold) = fold {
        arrays.add(&Arrays::from(fold.stock));
    }
    if carry != OutfitCarry::Keep {
        if fold.is_none_or(|fold| fold.default_weapons == RuleSource::Bible) {
            arrays.add(&Arrays::of(defaults, records));
        }
        for (&id, &count) in defaults {
            let owned = outfits.entry(id).or_default();
            *owned = owned.saturating_add(count);
        }
    }
    if fold.is_some() {
        self::fold(&mut outfits, &arrays, records);
    }
    if let Some(limits) = limits {
        hold(&mut outfits, &arrays.weapons, limits, records);
    }
    outfits
}

/// Folds `arrays` into `outfits` of `records`, as `_ShipStatsToSystemInfo`
/// does (@0xcaee-0xcce7; see the module docs): each owned outfit whose
/// first mod names a weapon (or its rounds) held to what the array has
/// left, the rest of the array put on the lowest-ID outfit whose first
/// mod names it. An outfit left with none is no longer listed.
pub fn fold(outfits: &mut BTreeMap<OutfitId, u16>, arrays: &Arrays, records: &[OutfitRecord]) {
    let mut left = arrays.clone();
    let mut sorted: Vec<&OutfitRecord> = records.iter().collect();
    sorted.sort_by_key(|record| record.id);
    for record in &sorted {
        let (mod_type, weapon) = record.mods[0];
        let (Some(array), Some(count)) = (left.array(mod_type), outfits.get_mut(&record.id)) else {
            continue;
        };
        let have = array.entry(WeaponId(weapon)).or_default();
        let owned = i64::from(*count);
        if owned >= *have {
            // A negative array zeroes the holder (@0xcbf6-0xcc0c).
            *count = u16::try_from((*have).max(0)).unwrap_or(u16::MAX);
            *have = 0;
        } else {
            *have -= owned;
        }
    }
    for record in &sorted {
        let (mod_type, weapon) = record.mods[0];
        let Some(array) = left.array(mod_type) else {
            continue;
        };
        if let Some(have) = array.remove(&WeaponId(weapon))
            && have.is_positive()
        {
            let count = outfits.entry(record.id).or_default();
            *count = count.saturating_add(u16::try_from(have).unwrap_or(u16::MAX));
        }
    }
    outfits.retain(|_, count| *count > 0);
}

/// Holds each of `outfits` that has a record among `records` to its
/// limit, by ascending ID, each reading the counts as the ones before it
/// left them: `_HasMaxOfItem` (@0x4512, called @0x15648), then
/// `if 0 < lim < count { count = lim }`. The launchers it reads for the
/// ammunition are `weapons`, the weapon array before the clamp, which the
/// clamp leaves.
fn hold(
    outfits: &mut BTreeMap<OutfitId, u16>,
    weapons: &BTreeMap<WeaponId, i64>,
    limits: &Limits,
    records: &[OutfitRecord],
) {
    // A negative count is no launcher.
    let launchers: BTreeMap<WeaponId, u32> = weapons
        .iter()
        .map(|(&weapon, &count)| (weapon, u32::try_from(count.max(0)).unwrap_or(u32::MAX)))
        .collect();
    let room = ItemRoom {
        hardpoints: limits.hardpoints,
        launchers: &launchers,
        max_ammo: limits.max_ammo,
        raised_max: limits.raised_max,
    };
    let ids: Vec<OutfitId> = outfits.keys().copied().collect();
    for id in ids {
        let Some(record) = records.iter().find(|record| record.id == id) else {
            continue;
        };
        let limit = item_limit(record, outfits, records, &room);
        if let Some(count) = outfits.get_mut(&id)
            && limit > 0
            && limit < i64::from(*count)
        {
            *count = u16::try_from(limit).unwrap_or(u16::MAX);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::armament::{MOD_AMMO, MOD_WEAPON};
    use crate::outfitter::{MORE_GUNS, MORE_TURRETS, RAISES_MAX};
    use crate::stats::MORE_SHIELD;
    use crate::testkit::outfit;

    /// 400: a licence (0x0004 and can't be sold, 0x000c); 401: an ability
    /// (0x0020 only); 402: both (0x0024); 403: plain; 404: up to 2; 405:
    /// `Max` 0; 406: `Max` -1.
    fn records() -> Vec<OutfitRecord> {
        let flagged = |id, flags| OutfitRecord {
            flags,
            ..outfit(id, &[])
        };
        let most = |id, max| OutfitRecord {
            max,
            ..outfit(id, &[])
        };
        vec![
            flagged(400, 0x000c),
            flagged(401, 0x0020),
            flagged(402, 0x0024),
            outfit(403, &[]),
            most(404, 2),
            most(405, 0),
            most(406, -1),
        ]
    }

    fn map(pairs: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        pairs.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    fn carried(
        owned: &[(i16, u16)],
        carry: OutfitCarry,
        defaults: &[(i16, u16)],
    ) -> Vec<(i16, u16)> {
        carried_outfits(&map(owned), carry, &map(defaults), None, None, &records())
            .into_iter()
            .map(|(id, n)| (id.0, n))
            .collect()
    }

    #[test]
    fn set_op_persistence_is_either_flag() {
        assert_eq!(SET_OP_PERSISTENT, 0x0024);
        assert_eq!(OutfitFlags::MISSION_PERSISTENT, 0x0020);
    }

    #[test]
    fn keep_keeps_every_outfit_and_adds_no_default() {
        assert_eq!(
            carried(
                &[(400, 1), (403, 3), (999, 2)],
                OutfitCarry::Keep,
                &[(403, 1), (404, 1)]
            ),
            [(400, 1), (403, 3), (999, 2)]
        );
    }

    #[test]
    fn keep_with_defaults_adds_each_default_to_what_is_owned() {
        assert_eq!(
            carried(
                &[(400, 1), (403, 3), (999, 2)],
                OutfitCarry::KeepWithDefaults,
                &[(403, 2), (404, 1)]
            ),
            [(400, 1), (403, 5), (404, 1), (999, 2)]
        );
        assert_eq!(
            carried(
                &[(403, u16::MAX)],
                OutfitCarry::KeepWithDefaults,
                &[(403, 2)]
            ),
            [(403, u16::MAX)],
            "saturating"
        );
    }

    #[test]
    fn persistent_keeps_the_outfits_its_mask_names_then_adds_the_defaults() {
        let owned = [(400, 1), (401, 2), (402, 3), (403, 4), (999, 5)];
        assert_eq!(
            carried(
                &owned,
                OutfitCarry::Persistent {
                    mask: SET_OP_PERSISTENT
                },
                &[(403, 1), (401, 1)]
            ),
            [(400, 1), (401, 3), (402, 3), (403, 1)],
            "0x0004 or 0x0020 kept; plain and recordless dropped"
        );
        assert_eq!(
            carried(
                &owned,
                OutfitCarry::Persistent {
                    mask: OutfitFlags::MISSION_PERSISTENT
                },
                &[]
            ),
            [(401, 2), (402, 3)],
            "the licence (0x0004 only) goes"
        );
    }

    /// No weapon has a `MaxAmmo`.
    fn no_max_ammo(_: WeaponId) -> u32 {
        0
    }

    /// The limits of a class of `guns` and `turrets`, no weapon with a
    /// `MaxAmmo`, raised maximums read by the engine.
    fn limits(guns: i16, turrets: i16) -> Limits<'static> {
        Limits {
            hardpoints: Hardpoints { guns, turrets },
            max_ammo: &no_max_ammo,
            raised_max: RuleSource::Engine,
        }
    }

    /// `owned` of `records` kept by `E`, nothing added, held to `limits`.
    fn held(
        owned: &[(i16, u16)],
        limits: &Limits,
        records: &[OutfitRecord],
    ) -> BTreeMap<OutfitId, u16> {
        carried_outfits(
            &map(owned),
            OutfitCarry::KeepWithDefaults,
            &map(&[]),
            None,
            Some(limits),
            records,
        )
    }

    #[test]
    fn a_clamp_holds_each_outfit_with_a_max_to_it_old_and_default_alike() {
        let clamped = |owned: &[(i16, u16)], defaults: &[(i16, u16)]| {
            carried_outfits(
                &map(owned),
                OutfitCarry::KeepWithDefaults,
                &map(defaults),
                None,
                Some(&limits(9, 9)),
                &records(),
            )
        };
        assert_eq!(clamped(&[(404, 5)], &[]), map(&[(404, 2)]), "an old one");
        assert_eq!(
            clamped(&[(404, 1)], &[(404, 2)]),
            map(&[(404, 2)]),
            "a default"
        );
        assert_eq!(clamped(&[(404, 2)], &[]), map(&[(404, 2)]), "at its Max");
        assert_eq!(clamped(&[(404, 1)], &[]), map(&[(404, 1)]), "below it");
        assert_eq!(
            clamped(&[(405, 7), (406, 7), (999, 70)], &[]),
            map(&[(405, 7), (406, 7), (999, 70)]),
            "no Max above none, or no record: never clamped"
        );
        assert_eq!(
            carried_outfits(
                &map(&[(404, 5)]),
                OutfitCarry::Keep,
                &map(&[]),
                None,
                None,
                &records()
            ),
            map(&[(404, 5)]),
            "no clamp asked"
        );
        assert_eq!(
            carried_outfits(
                &map(&[(404, 5), (403, 1)]),
                OutfitCarry::Persistent {
                    mask: SET_OP_PERSISTENT
                },
                &map(&[(404, 4)]),
                None,
                Some(&limits(9, 9)),
                &records()
            ),
            map(&[(404, 2)]),
            "after the defaults"
        );
    }

    /// A gun (`GUN`) or turret (`TURRET`) outfit `id`, `Max` 10.
    fn mounted(id: i16, flags: u16) -> OutfitRecord {
        OutfitRecord {
            flags,
            ..outfit(id, &[])
        }
    }

    #[test]
    fn a_clamp_holds_each_gun_to_the_whole_gun_limit_once_the_guns_reach_it() {
        let records = [
            mounted(410, OutfitFlags::GUN),
            mounted(411, OutfitFlags::GUN),
            outfit(412, &[(MORE_GUNS, 1), (MORE_GUNS, 4)]),
            mounted(413, OutfitFlags::TURRET),
        ];
        let roomy = limits(2, 0);
        assert_eq!(held(&[(410, 3)], &roomy, &records), map(&[(410, 2)]));
        assert_eq!(
            held(&[(410, 2), (411, 2)], &roomy, &records),
            map(&[(410, 2), (411, 2)]),
            "each held to the slot total on its own, not the guns together"
        );
        assert_eq!(
            held(&[(410, 1)], &limits(0, 0), &records),
            map(&[(410, 1)]),
            "no gun slot: nothing held"
        );
        assert_eq!(
            held(&[(410, 3)], &limits(-1, 0), &records),
            map(&[(410, 3)])
        );
        assert_eq!(
            held(&[(410, 5), (412, 3)], &roomy, &records),
            map(&[(410, 3), (412, 3)]),
            "2 + 1, the first mod of 45, once whatever the count"
        );
        assert_eq!(
            held(&[(410, 2), (411, 1)], &limits(3, 0), &records),
            map(&[(410, 2), (411, 1)]),
            "below the limit"
        );
        assert_eq!(
            held(&[(413, 5)], &limits(1, 9), &records),
            map(&[(413, 5)]),
            "a turret is no gun"
        );
    }

    #[test]
    fn a_clamp_holds_each_turret_to_the_whole_turret_limit_once_the_turrets_reach_it() {
        let records = [
            mounted(410, OutfitFlags::TURRET),
            mounted(411, OutfitFlags::TURRET),
            outfit(412, &[(MORE_TURRETS, 1), (MORE_TURRETS, 4)]),
            mounted(413, OutfitFlags::GUN),
        ];
        let roomy = limits(0, 2);
        assert_eq!(held(&[(410, 3)], &roomy, &records), map(&[(410, 2)]));
        assert_eq!(
            held(&[(410, 2), (411, 2)], &roomy, &records),
            map(&[(410, 2), (411, 2)])
        );
        assert_eq!(held(&[(410, 1)], &limits(0, 0), &records), map(&[(410, 1)]));
        assert_eq!(
            held(&[(410, 3)], &limits(0, -1), &records),
            map(&[(410, 3)])
        );
        assert_eq!(
            held(&[(410, 5), (412, 3)], &roomy, &records),
            map(&[(410, 3), (412, 3)])
        );
        assert_eq!(
            held(&[(410, 2), (411, 1)], &limits(0, 3), &records),
            map(&[(410, 2), (411, 1)])
        );
        assert_eq!(
            held(&[(413, 5)], &limits(9, 1), &records),
            map(&[(413, 5)]),
            "a gun is no turret"
        );
    }

    #[test]
    fn an_outfit_both_gun_and_turret_is_held_by_the_turrets_once_the_guns_have_room() {
        let records = [mounted(410, OutfitFlags::GUN | OutfitFlags::TURRET)];
        assert_eq!(held(&[(410, 5)], &limits(9, 2), &records), map(&[(410, 2)]));
        assert_eq!(held(&[(410, 5)], &limits(3, 9), &records), map(&[(410, 3)]));
        assert_eq!(
            held(&[(410, 3)], &limits(3, 2), &records),
            map(&[(410, 3)]),
            "the guns exactly at the gun limit: it returns there, the turrets never read"
        );
    }

    #[test]
    fn a_gun_at_its_max_is_held_by_the_max_before_the_gun_limit() {
        let records = [OutfitRecord {
            max: 3,
            ..mounted(417, OutfitFlags::GUN)
        }];
        assert_eq!(
            held(&[(417, 3)], &limits(2, 0), &records),
            map(&[(417, 3)]),
            "exactly at its Max: it returns there, the gun limit of 2 never read"
        );
    }

    #[test]
    fn a_clamp_holds_an_outfit_to_its_max_as_mod_type_27_raises_it() {
        let records = [
            OutfitRecord {
                max: 4,
                ..outfit(414, &[])
            },
            outfit(415, &[(RAISES_MAX, 414)]),
            outfit(416, &[(RAISES_MAX, 414), (RAISES_MAX, 414)]),
        ];
        assert_eq!(
            held(&[(414, 10), (415, 2)], &limits(0, 0), &records),
            map(&[(414, 8), (415, 2)]),
            "two raisers: 4 x 2"
        );
        assert_eq!(
            held(&[(414, 10), (415, 1)], &limits(0, 0), &records),
            map(&[(414, 4), (415, 1)]),
            "one raiser leaves it"
        );
        assert_eq!(
            held(&[(414, 10), (416, 1)], &limits(0, 0), &records),
            map(&[(414, 8), (416, 1)]),
            "by the engine each mod counts"
        );
        let by_items = Limits {
            raised_max: RuleSource::Bible,
            ..limits(0, 0)
        };
        assert_eq!(
            held(&[(414, 10), (416, 1)], &by_items, &records),
            map(&[(414, 4), (416, 1)]),
            "by the other reading each item counts once"
        );
    }

    /// Weapon 300 has a `MaxAmmo` of 10.
    fn max_ammo_10(weapon: WeaponId) -> u32 {
        if weapon == WeaponId(300) { 10 } else { 0 }
    }

    /// 420 launches weapon 300; 421 is its ammunition, `Max` 50; 422 its
    /// ammunition, `Max` 5; 423 names it in a later mod; 424 is weapon
    /// 301's ammunition, `Max` 5.
    fn magazine() -> Vec<OutfitRecord> {
        let most = |id, mods: &[(i16, i16)], max| OutfitRecord {
            max,
            ..outfit(id, mods)
        };
        vec![
            outfit(420, &[(MOD_WEAPON, 300)]),
            most(421, &[(MOD_AMMO, 300)], 50),
            most(422, &[(MOD_AMMO, 300)], 5),
            most(423, &[(MORE_SHIELD, 1), (MOD_AMMO, 300)], 50),
            most(424, &[(MOD_AMMO, 301)], 5),
        ]
    }

    #[test]
    fn a_clamp_holds_ammunition_to_max_ammo_times_its_launchers_before_its_max() {
        let capped = Limits {
            max_ammo: &max_ammo_10,
            ..limits(0, 0)
        };
        let records = magazine();
        let at = |owned: &[(i16, u16)]| held(owned, &capped, &records);
        assert_eq!(at(&[(420, 1), (421, 12)]), map(&[(420, 1), (421, 10)]));
        assert_eq!(
            at(&[(420, 1), (422, 12)]),
            map(&[(420, 1), (422, 10)]),
            "the cap returns before the Max of 5"
        );
        assert_eq!(
            at(&[(420, 1), (422, 10)]),
            map(&[(420, 1), (422, 10)]),
            "exactly at the cap: it returns before the Max of 5"
        );
        assert_eq!(
            at(&[(420, 2), (422, 12)]),
            map(&[(420, 2), (422, 5)]),
            "under a cap of 20 the Max holds"
        );
        assert_eq!(at(&[(420, 2), (421, 12)]), map(&[(420, 2), (421, 12)]));
        assert_eq!(
            at(&[(421, 12), (422, 12)]),
            map(&[(421, 12), (422, 12)]),
            "no launcher: a cap of none, nothing held"
        );
        assert_eq!(
            at(&[(420, 1), (423, 12)]),
            map(&[(420, 1), (423, 12)]),
            "a later mod of 3 is not capped"
        );
        assert_eq!(
            at(&[(420, 1), (424, 12)]),
            map(&[(420, 1), (424, 5)]),
            "no MaxAmmo: its Max"
        );
    }

    #[test]
    fn the_clamp_reads_the_arrays_not_the_folded_outfits() {
        let capped = Limits {
            max_ammo: &max_ammo_10,
            ..limits(1, 0)
        };
        let stock = StockAdded {
            weapons: BTreeMap::from([(WeaponId(300), 2)]),
            rounds: BTreeMap::new(),
        };
        let clamped = |owned: &[(i16, u16)],
                       defaults: &[(i16, u16)],
                       default_weapons: RuleSource,
                       records: &[OutfitRecord]| {
            let fold = Fold {
                stock: &stock,
                default_weapons,
            };
            carried_outfits(
                &map(owned),
                OutfitCarry::KeepWithDefaults,
                &map(defaults),
                Some(&fold),
                Some(&capped),
                records,
            )
        };
        let unheld = &magazine()[1..];
        assert_eq!(
            clamped(&[(421, 25)], &[], RuleSource::Engine, unheld),
            map(&[(421, 20)]),
            "two stock launchers no outfit holds"
        );
        let mut gun = magazine();
        gun[0].flags = OutfitFlags::GUN;
        assert_eq!(
            clamped(&[(421, 25)], &[], RuleSource::Engine, &gun),
            map(&[(420, 1), (421, 20)]),
            "the launcher folded to 2, held to its one slot, still counts twice for its rounds"
        );
        let none = StockAdded::default();
        let by = |default_weapons| {
            let fold = Fold {
                stock: &none,
                default_weapons,
            };
            carried_outfits(
                &map(&[(421, 12)]),
                OutfitCarry::KeepWithDefaults,
                &map(&[(420, 1)]),
                Some(&fold),
                Some(&capped),
                &magazine(),
            )
        };
        assert_eq!(
            by(RuleSource::Engine),
            map(&[(421, 12)]),
            "the default launcher folded away counts for nothing: no cap"
        );
        assert_eq!(
            by(RuleSource::Bible),
            map(&[(420, 1), (421, 10)]),
            "by the other reading it is kept and counts"
        );
        assert_eq!(
            carried_outfits(
                &map(&[(421, 12)]),
                OutfitCarry::KeepWithDefaults,
                &map(&[(420, 1)]),
                None,
                Some(&capped),
                &magazine(),
            ),
            map(&[(420, 1), (421, 10)]),
            "with no fold the clamp counts the default launcher"
        );
    }

    /// Weapons 300 and 301 have a `MaxAmmo` of 10.
    fn max_ammo_10_each(weapon: WeaponId) -> u32 {
        if [WeaponId(300), WeaponId(301)].contains(&weapon) {
            10
        } else {
            0
        }
    }

    #[test]
    fn a_negative_launcher_count_in_the_array_is_no_launcher_for_the_clamp() {
        let capped = Limits {
            max_ammo: &max_ammo_10_each,
            ..limits(0, 0)
        };
        let stock = StockAdded {
            weapons: BTreeMap::from([(WeaponId(300), -1), (WeaponId(301), 2)]),
            rounds: BTreeMap::new(),
        };
        let fold = Fold {
            stock: &stock,
            default_weapons: RuleSource::Engine,
        };
        let unheld = &magazine()[1..];
        assert_eq!(
            carried_outfits(
                &map(&[(422, 12), (424, 25)]),
                OutfitCarry::KeepWithDefaults,
                &map(&[]),
                Some(&fold),
                Some(&capped),
                unheld,
            ),
            map(&[(422, 12), (424, 20)]),
            "300 at -1: a cap of none, its Max of 5 never read; 301 at 2: capped at 20"
        );
    }

    #[test]
    fn a_clamp_reads_the_counts_held_before_it() {
        let records = [
            OutfitRecord {
                max: 1,
                ..outfit(430, &[(RAISES_MAX, 431)])
            },
            OutfitRecord {
                max: 2,
                ..outfit(431, &[])
            },
        ];
        assert_eq!(
            held(&[(430, 3), (431, 5)], &limits(0, 0), &records),
            map(&[(430, 1), (431, 2)]),
            "the raiser held to 1 first: 431 is held to 2, not 6"
        );
    }

    // The fold.

    /// 440 and 441 launch weapon 300; 442 names it in a later mod; 443
    /// and 444 are its ammunition, 445 names that in a later mod; 447
    /// names weapon 302 in a later mod alone.
    fn holders() -> Vec<OutfitRecord> {
        vec![
            outfit(441, &[(MOD_WEAPON, 300)]),
            outfit(440, &[(MOD_WEAPON, 300)]),
            outfit(442, &[(MORE_SHIELD, 1), (MOD_WEAPON, 300)]),
            outfit(443, &[(MOD_AMMO, 300)]),
            outfit(444, &[(MOD_AMMO, 300)]),
            outfit(445, &[(MORE_SHIELD, 1), (MOD_AMMO, 300)]),
            outfit(447, &[(MORE_SHIELD, 1), (MOD_WEAPON, 302)]),
        ]
    }

    /// `pairs` as a map of weapons to signed counts.
    fn array(pairs: &[(i16, i64)]) -> BTreeMap<WeaponId, i64> {
        pairs.iter().map(|&(id, n)| (WeaponId(id), n)).collect()
    }

    /// `owned` of [`holders`] folded with `weapons` and `rounds`.
    fn folded(
        owned: &[(i16, u16)],
        weapons: &[(i16, i64)],
        rounds: &[(i16, i64)],
    ) -> BTreeMap<OutfitId, u16> {
        let mut outfits = map(owned);
        let arrays = Arrays {
            weapons: array(weapons),
            rounds: array(rounds),
        };
        fold(&mut outfits, &arrays, &holders());
        outfits
    }

    #[test]
    fn the_fold_leaves_each_holder_its_count_and_puts_the_surplus_on_the_lowest_id_holder() {
        assert_eq!(
            folded(&[(441, 2)], &[(300, 5)], &[]),
            map(&[(440, 3), (441, 2)]),
            "441 keeps its 2; the 3 left go to 440, owned or not"
        );
        assert_eq!(
            folded(&[(440, 1), (441, 2)], &[(300, 3)], &[]),
            map(&[(440, 1), (441, 2)]),
            "exactly the holders' counts: nothing left"
        );
        assert_eq!(
            folded(&[(440, 1)], &[(300, 4)], &[]),
            map(&[(440, 4)]),
            "the surplus onto the holder itself"
        );
        assert_eq!(
            folded(&[(440, u16::MAX)], &[(300, i64::MAX)], &[]),
            map(&[(440, u16::MAX)]),
            "saturating"
        );
    }

    #[test]
    fn the_fold_loses_a_surplus_no_first_mod_holds() {
        assert_eq!(
            folded(&[(447, 1)], &[(302, 4)], &[]),
            map(&[(447, 1)]),
            "only a later mod names 302"
        );
    }

    #[test]
    fn the_fold_pulls_a_holder_back_to_the_array_and_zeroes_those_after_it() {
        assert_eq!(
            folded(&[(440, 3), (441, 2)], &[(300, 2)], &[]),
            map(&[(440, 2)]),
            "440 to the 2 there are, 441 to none"
        );
        assert_eq!(
            folded(&[(440, 3), (441, 2)], &[(300, 4)], &[]),
            map(&[(440, 3), (441, 1)]),
            "441 gets what 440 leaves"
        );
        assert_eq!(folded(&[(441, 2)], &[], &[]), map(&[]), "no entry is none");
    }

    #[test]
    fn the_fold_zeroes_every_holder_of_a_negative_array() {
        assert_eq!(
            folded(&[(440, 3), (441, 2), (443, 4)], &[(300, -1)], &[(300, -2)]),
            map(&[]),
        );
    }

    #[test]
    fn the_fold_leaves_an_outfit_naming_the_weapon_in_a_later_mod() {
        assert_eq!(
            folded(&[(442, 3), (445, 4)], &[(300, 1)], &[(300, 1)]),
            map(&[(440, 1), (442, 3), (443, 1), (445, 4)]),
        );
        assert_eq!(
            folded(&[(442, 3), (445, 4), (999, 2)], &[], &[]),
            map(&[(442, 3), (445, 4), (999, 2)]),
            "and an outfit with no record"
        );
    }

    #[test]
    fn the_fold_holds_ammunition_by_the_rounds_array_alike() {
        assert_eq!(
            folded(&[(444, 2)], &[], &[(300, 5)]),
            map(&[(443, 3), (444, 2)])
        );
        assert_eq!(
            folded(&[(443, 3), (444, 2)], &[], &[(300, 4)]),
            map(&[(443, 3), (444, 1)])
        );
        assert_eq!(
            folded(&[(443, 3), (444, 2)], &[(300, 9)], &[]),
            map(&[(440, 9)]),
            "the weapon array holds no ammunition"
        );
    }

    /// No stock added.
    static NO_STOCK: StockAdded = StockAdded {
        weapons: BTreeMap::new(),
        rounds: BTreeMap::new(),
    };

    /// `owned` of [`holders`] changed by `E` to a class adding `stock`
    /// and `defaults`, default weapons read by `default_weapons`, folded
    /// and not clamped.
    fn changed(
        owned: &[(i16, u16)],
        stock: &StockAdded,
        defaults: &[(i16, u16)],
        default_weapons: RuleSource,
    ) -> BTreeMap<OutfitId, u16> {
        let fold = Fold {
            stock,
            default_weapons,
        };
        carried_outfits(
            &map(owned),
            OutfitCarry::KeepWithDefaults,
            &map(defaults),
            Some(&fold),
            None,
            &holders(),
        )
    }

    #[test]
    fn e_and_h_fold_the_stock_into_the_carried_outfits() {
        let stock = StockAdded {
            weapons: array(&[(300, 2)]),
            rounds: array(&[(300, -1)]),
        };
        assert_eq!(
            changed(&[(441, 1), (443, 5)], &stock, &[], RuleSource::Engine),
            map(&[(440, 2), (441, 1), (443, 4)]),
            "the 2 stock launchers on 440; one round taken"
        );
        assert_eq!(
            changed(&[(443, 5)], &NO_STOCK, &[], RuleSource::Engine),
            map(&[(443, 5)]),
            "the carried outfits are their own arrays"
        );
    }

    #[test]
    fn by_the_engine_a_default_weapon_or_ammunition_outfit_is_folded_away() {
        for defaults in [[(441, 1)], [(443, 2)]] {
            assert_eq!(
                changed(&[(440, 1)], &NO_STOCK, &defaults, RuleSource::Engine),
                map(&[(440, 1)]),
                "{defaults:?}"
            );
        }
        assert_eq!(
            changed(&[(441, 1)], &NO_STOCK, &[(440, 1)], RuleSource::Engine),
            map(&[(440, 1)]),
            "a default of lower ID takes the one there is from the holder owned"
        );
        assert_eq!(
            changed(
                &[(440, 1)],
                &NO_STOCK,
                &[(441, 1), (443, 2)],
                RuleSource::Bible
            ),
            map(&[(440, 1), (441, 1), (443, 2)]),
            "by the other reading the defaults are added to the arrays too"
        );
        assert_eq!(
            changed(&[], &NO_STOCK, &[(442, 1)], RuleSource::Engine),
            map(&[(442, 1)]),
            "a later mod is no holder: kept"
        );
        let stock = StockAdded {
            weapons: array(&[(300, 2)]),
            rounds: BTreeMap::new(),
        };
        assert_eq!(
            changed(&[], &stock, &[(440, 1)], RuleSource::Engine),
            map(&[(440, 2)]),
            "two stock and one default: two"
        );
    }

    #[test]
    fn c_folds_nothing_and_adds_no_default_to_the_arrays() {
        let fold = Fold {
            stock: &NO_STOCK,
            default_weapons: RuleSource::Bible,
        };
        assert_eq!(
            carried_outfits(
                &map(&[(441, 1)]),
                OutfitCarry::Keep,
                &map(&[(440, 3)]),
                Some(&fold),
                None,
                &holders(),
            ),
            map(&[(441, 1)])
        );
    }
}
