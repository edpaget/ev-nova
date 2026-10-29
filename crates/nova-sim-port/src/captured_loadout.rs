//! Taking over a captured ship: which outfits the player has after Use
//! As My Ship, as `_DoShipCapture`'s weapons merge makes them
//! (@0x41849-0x41a3e).
//!
//! The player keeps its persistent outfits and the weapon slots they
//! name, takes the captured ship's **live** weapons and rounds into the
//! slots left empty, and gets the captured class's default items on top
//! ([`captured_outfits`]). There is no stock top-up: the captured ship's
//! loadout, its spawn's stock less what it fired or a person's fitted
//! ship, is what the player takes. No sound plays anywhere in the merge.
//!
//! **Persistent** is `Flags` 0x0004 alone
//! ([`OutfitFlags::PERSISTENT`]): the outfit record's byte +0x378, which
//! `_LoadObjectData` alone writes, clearing it (@0x78c92) and setting it
//! when the `oütf`'s `Flags & 0x0004` (@0x78cad-0x78cb3).
//!
//! The merge works on the weapon and rounds arrays ([`Arrays`]) the
//! player's outfits make, for each weapon w of `wëap` 128 to 383
//! ([`FOLDED_WEAPONS`]):
//!
//! 1. **Which of the player's slots survive** (@0x41849-0x418d6). Slot w
//!    is kept when some outfit record flagged persistent, owned or not,
//!    has a mod of `ModType` 1 naming w in any of its four mods
//!    (@0x4185a-0x41892). Otherwise the player's count at w and the
//!    player's rounds at slot w (the rounds of ammunition-weapon w) are
//!    set to none (@0x418ab, @0x418bc). Only `ModType` 1 protects: a
//!    persistent ammunition outfit (`ModType` 3) keeps no rounds.
//! 2. **The captured ship's live loadout** (@0x418dc-0x419c8), w
//!    ascending. Its count c at w is set as the player's when `c > 0`
//!    and the player's count at w is none or below (@0x41907,
//!    @0x4191f). Its rounds a at its launcher w's own slot go over when
//!    `a > 0` and the player's rounds at slot w are none or below
//!    (@0x41931, @0x41950), by `wëap` w: a guidance of 99 sets the
//!    player's slot w (@0x41966-0x41973); otherwise an `AmmoType` of 0
//!    to 255 (read unsigned, @0x41983) sets the slot of `wëap`
//!    128 + `AmmoType` (@0x41990); otherwise they go nowhere. Each write
//!    sets, and the `AmmoType` slot's own value is never tested, so of
//!    two launchers sharing an `AmmoType` the higher ID's rounds are
//!    kept. A weapon the arsenal has no record of takes its rounds
//!    nowhere.
//! 3. **Strip, fold and defaults** (@0x419ce-0x41a3e). Every outfit not
//!    flagged persistent is set to none (@0x419dd-0x419fb), an outfit
//!    with no record among them. `_ShipStatsToSystemInfo` (@0x419fd)
//!    folds the arrays into the outfits, [`ship_change::fold`]: the
//!    persistent holders keep up to the array's count and the rest lands
//!    on the lowest-ID outfit whose first mod names the weapon. **Then**
//!    the captured class's default items are added, each only when its
//!    count is above none (@0x41a19-0x41a3c), so a default weapon or
//!    ammunition outfit is never pulled back, unlike `E` and `H`'s
//!    ([`ship_change::carried_outfits`]). No outfit is held to its
//!    `Max`. `_SystemInfoToShipStats` (@0x41a3e) rebuilds the arrays from
//!    the outfits, as the session's refit does.
//!
//! The original's counts and rounds are 16-bit; these are wide and
//! saturate. Its protection walk covers all 512 record slots, an empty
//! slot's byte cleared, so walking the loaded records is the same.

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{OutfitId, OutfitRecord, WeaponId};
use crate::combat::armament::{Armament, Arrays, Arsenal, LauncherRounds, MOD_WEAPON};
use crate::combat::weapon::{FIRST_AMMO_WEAPON, LAST_AMMO_TYPE};
use crate::outfitter::OutfitFlags;
use crate::ship_change::{self, FOLDED_WEAPONS};

/// The captured ship's live weapons: its mounts' counts and its
/// launchers' rounds, each on its launcher's own slot.
#[derive(Clone, Copy, Debug)]
pub struct Captured<'a> {
    /// Its mounts, each with how many it carries.
    pub armament: &'a Armament,
    /// Its launchers' rounds, as it has them left.
    pub rounds: &'a LauncherRounds,
}

/// The outfits the player has after taking over the `captured` ship:
/// its `owned` outfits of `records` merged with the captured ship's live
/// weapons and rounds, `wëap`s read from `arsenal`, then the captured
/// class's `defaults` added (see the module docs).
#[must_use]
pub fn captured_outfits(
    owned: &BTreeMap<OutfitId, u16>,
    captured: Captured<'_>,
    defaults: &BTreeMap<OutfitId, u16>,
    arsenal: &Arsenal,
    records: &[OutfitRecord],
) -> BTreeMap<OutfitId, u16> {
    let mut arrays = Arrays::of(owned, records);
    let kept = kept_slots(records);
    let unkept = |weapon: &WeaponId| FOLDED_WEAPONS.contains(&weapon.0) && !kept.contains(weapon);
    arrays.weapons.retain(|weapon, _| !unkept(weapon));
    arrays.rounds.retain(|weapon, _| !unkept(weapon));
    merge(&mut arrays, captured, arsenal);
    let mut outfits = owned.clone();
    outfits.retain(|id, _| {
        records
            .iter()
            .any(|record| record.id == *id && record.flags & OutfitFlags::PERSISTENT != 0)
    });
    ship_change::fold(&mut outfits, &arrays, records);
    for (&id, &count) in defaults {
        if count > 0 {
            let owned = outfits.entry(id).or_default();
            *owned = owned.saturating_add(count);
        }
    }
    outfits
}

/// The weapon slots some outfit of `records` flagged persistent names
/// with a `ModType` 1 in any of its four mods, owned or not
/// (@0x4185a-0x41892).
fn kept_slots(records: &[OutfitRecord]) -> BTreeSet<WeaponId> {
    records
        .iter()
        .filter(|record| record.flags & OutfitFlags::PERSISTENT != 0)
        .flat_map(|record| record.mods)
        .filter(|&(mod_type, _)| mod_type == MOD_WEAPON)
        .map(|(_, weapon)| WeaponId(weapon))
        .collect()
}

/// Copies the `captured` ship's live counts and rounds into the player's
/// `arrays` where the player has none, `wëap` by `wëap` ascending
/// (@0x418dc-0x419c8; see the module docs).
fn merge(arrays: &mut Arrays, captured: Captured<'_>, arsenal: &Arsenal) {
    for id in FOLDED_WEAPONS {
        let weapon = WeaponId(id);
        let count: u32 = captured
            .armament
            .mounts()
            .iter()
            .filter(|mount| mount.spec.id == weapon)
            .map(|mount| mount.count)
            .sum();
        // The original copies when `c > 0` and the player's count is none
        // or below (@0x41907, @0x4191f). The player's counts come from
        // `u16` outfit counts, never below none, so copying a count of
        // none too writes none over none.
        let have = arrays.weapons.entry(weapon).or_default();
        if *have <= 0 {
            *have = i64::from(count);
        }
        let rounds = captured.rounds.slot(weapon);
        if rounds <= 0 || arrays.rounds.get(&weapon).is_some_and(|&held| held > 0) {
            continue;
        }
        let Some(spec) = arsenal.weapon(weapon) else {
            continue;
        };
        let slot = if spec.guidance_99() {
            weapon
        } else if (0..=LAST_AMMO_TYPE).contains(&spec.ammo_type) {
            WeaponId(FIRST_AMMO_WEAPON + spec.ammo_type)
        } else {
            continue;
        };
        arrays.rounds.insert(slot, i64::from(rounds));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::combat::armament::MOD_AMMO;
    use nova_sim_catalog::fixture::{outfit, weapon};

    /// A plain outfit `id` of `mods`.
    fn plain(id: i16, mods: &[(i16, i16)]) -> OutfitRecord {
        outfit(id, mods)
    }

    /// A persistent (0x0004) outfit `id` of `mods`.
    fn persistent(id: i16, mods: &[(i16, i16)]) -> OutfitRecord {
        OutfitRecord {
            flags: OutfitFlags::PERSISTENT,
            ..outfit(id, mods)
        }
    }

    /// `wëap` `id` of raw `guidance` and `AmmoType`.
    fn spec(id: i16, guidance: i16, ammo_type: i16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            ammo_type,
            ..weapon(id)
        }
    }

    /// The arsenal: guns 128 and 130; launchers 134, 139 and 140 firing
    /// 134's rounds (`AmmoType` 6); 136, 137 and 138 of `AmmoType` -1,
    /// 256 and -1000; bay 150 of ship 144; 149, a guidance-99 weapon of
    /// `AmmoType` 5 and no ship; 127 and 400 of `AmmoType` 6, outside the
    /// arrays.
    fn arsenal() -> Arsenal {
        Arsenal::new(
            &[
                spec(128, -1, -1),
                spec(130, -1, -1),
                spec(134, 1, 6),
                spec(139, 1, 6),
                spec(140, 1, 6),
                spec(136, 1, -1),
                spec(137, 1, 256),
                spec(138, 1, -1000),
                spec(150, 99, 144),
                spec(149, 99, 5),
                spec(127, 1, 6),
                spec(400, 1, 6),
            ],
            Vec::new(),
        )
    }

    /// The outfit records: 250 and 256 guns of 128, 254 of 130, 252 of
    /// 134, 257 of 140, 258 of bay 150, 259 of 149; 403 the rounds of
    /// 134, 311 of 150, 312 of 149, 313 of 133, 314 of 136, 315 of 137,
    /// 316 of 138; 400 persistent and 401 plain, of no mod; 402 a
    /// default item.
    fn base() -> Vec<OutfitRecord> {
        vec![
            plain(250, &[(MOD_WEAPON, 128)]),
            plain(252, &[(MOD_WEAPON, 134)]),
            plain(254, &[(MOD_WEAPON, 130)]),
            plain(256, &[(MOD_WEAPON, 128)]),
            plain(257, &[(MOD_WEAPON, 140)]),
            plain(258, &[(MOD_WEAPON, 150)]),
            plain(259, &[(MOD_WEAPON, 149)]),
            plain(403, &[(MOD_AMMO, 134)]),
            plain(311, &[(MOD_AMMO, 150)]),
            plain(312, &[(MOD_AMMO, 149)]),
            plain(313, &[(MOD_AMMO, 133)]),
            plain(314, &[(MOD_AMMO, 136)]),
            plain(315, &[(MOD_AMMO, 137)]),
            plain(316, &[(MOD_AMMO, 138)]),
            persistent(400, &[]),
            plain(401, &[]),
            plain(402, &[]),
        ]
    }

    fn map(pairs: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        pairs.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    /// The merge of `owned` with a captured ship mounting `mounts`
    /// (weapon, count) and holding `rounds` (launcher, rounds), then
    /// `defaults`, over `records`, as raw pairs.
    fn merged(
        records: &[OutfitRecord],
        owned: &[(i16, u16)],
        mounts: &[(i16, u32)],
        rounds: &[(i16, i32)],
        defaults: &[(i16, u16)],
    ) -> Vec<(i16, u16)> {
        let arsenal = arsenal();
        let armament = Armament::new(mounts.iter().map(|&(id, count)| {
            let spec = *arsenal.weapon(WeaponId(id)).expect("in the arsenal");
            (spec, count)
        }));
        let rounds: LauncherRounds = rounds
            .iter()
            .map(|&(id, rounds)| (WeaponId(id), rounds))
            .collect();
        let captured = Captured {
            armament: &armament,
            rounds: &rounds,
        };
        captured_outfits(&map(owned), captured, &map(defaults), &arsenal, records)
            .into_iter()
            .map(|(id, count)| (id.0, count))
            .collect()
    }

    #[test]
    fn non_persistent_outfits_are_stripped_and_the_defaults_added_on_top() {
        assert_eq!(
            merged(
                &base(),
                &[(400, 1), (401, 2), (999, 1)],
                &[],
                &[],
                &[(400, 1), (402, 1)]
            ),
            [(400, 2), (402, 1)],
            "401 stripped, 999 (no record) too; 400 kept and the default added"
        );
    }

    #[test]
    fn a_default_weapon_or_ammunition_outfit_is_never_pulled_back() {
        assert_eq!(
            merged(&base(), &[], &[], &[], &[(250, 1), (403, 4)]),
            [(250, 1), (403, 4)],
            "added after the fold (@0x41a19-0x41a3c), unlike E and H"
        );
    }

    #[test]
    fn a_default_item_of_none_adds_no_entry() {
        assert_eq!(merged(&base(), &[], &[], &[], &[(402, 0)]), []);
    }

    #[test]
    fn an_unprotected_gun_gives_way_to_the_captured_ships_on_the_lowest_id_holder() {
        assert_eq!(
            merged(&base(), &[(256, 1)], &[(128, 2)], &[], &[]),
            [(250, 2)],
            "the player's 256 zeroed and stripped, the captured 2 on 250"
        );
    }

    #[test]
    fn a_persistent_outfit_not_owned_naming_a_weapon_in_a_later_mod_keeps_its_slot() {
        let mut records = base();
        records.push(persistent(260, &[(17, 5), (MOD_WEAPON, 130)]));
        assert_eq!(
            merged(&records, &[(254, 1)], &[(130, 3)], &[], &[]),
            [(254, 1)],
            "slot 130 kept at 1, the captured 3 not copied, 254 stripped and refolded"
        );
        assert_eq!(
            merged(&base(), &[(254, 1)], &[(130, 3)], &[], &[]),
            [(254, 3)],
            "unprotected, the captured 3"
        );
    }

    #[test]
    fn a_persistent_ammunition_outfit_protects_no_rounds() {
        let mut records = base();
        records.push(persistent(404, &[(MOD_AMMO, 134)]));
        assert_eq!(
            merged(&records, &[(404, 5)], &[(134, 1)], &[(134, 3)], &[]),
            [(252, 1), (404, 3)],
            "its 5 zeroed and the captured 3 landing on it"
        );
    }

    #[test]
    fn a_launchers_rounds_set_its_ammo_type_slot_kept_by_protection() {
        // 405 protects 134, so the player's 5 rounds of 134 (404) stay;
        // 140's slot is empty, so its 3 rounds replace them.
        let mut records = base();
        records.push(persistent(404, &[(MOD_AMMO, 134)]));
        records.push(persistent(405, &[(MOD_WEAPON, 134)]));
        assert_eq!(
            merged(&records, &[(404, 5)], &[(140, 1)], &[(140, 3)], &[]),
            [(257, 1), (404, 3)]
        );
    }

    #[test]
    fn the_players_rounds_at_the_launchers_own_slot_block_the_copy() {
        // 407 protects 140, so the player's 2 rounds of 140 (406) stay
        // and block 140's 3, though 134's slot is empty.
        let mut records = base();
        records.push(persistent(406, &[(MOD_AMMO, 140)]));
        records.push(persistent(407, &[(MOD_WEAPON, 140)]));
        assert_eq!(
            merged(&records, &[(406, 2)], &[(140, 1)], &[(140, 3)], &[]),
            [(257, 1), (406, 2)]
        );
    }

    #[test]
    fn rounds_owned_at_none_block_no_copy() {
        // 407 protects 140, and the player holds 140's rounds (406) at
        // none: 140's 3 still go to 134's slot.
        let mut records = base();
        records.push(persistent(406, &[(MOD_AMMO, 140)]));
        records.push(persistent(407, &[(MOD_WEAPON, 140)]));
        assert_eq!(
            merged(&records, &[(406, 0)], &[(140, 1)], &[(140, 3)], &[]),
            [(257, 1), (403, 3)]
        );
    }

    #[test]
    fn a_guidance_99_weapons_rounds_go_to_its_own_slot() {
        assert_eq!(
            merged(&base(), &[], &[(150, 1)], &[(150, 3)], &[]),
            [(258, 1), (311, 3)],
            "a bay's fighters aboard, not slot 128 + 144"
        );
        assert_eq!(
            merged(&base(), &[], &[(149, 1)], &[(149, 2)], &[]),
            [(259, 1), (312, 2)],
            "a guidance-99 weapon with no ship, not slot 133"
        );
    }

    #[test]
    fn rounds_of_an_ammo_type_outside_0_to_255_go_nowhere() {
        assert_eq!(
            merged(
                &base(),
                &[],
                &[(136, 1), (137, 1), (138, 1)],
                &[(136, 4), (137, 4), (138, 4)],
                &[]
            ),
            [],
            "-1, 256 and -1000: neither their own slots nor any other"
        );
    }

    #[test]
    fn rounds_of_none_or_below_copy_nothing() {
        // 139's 3 set 134's slot; 140's -1 would overwrite it.
        assert_eq!(
            merged(
                &base(),
                &[],
                &[(139, 1), (140, 1)],
                &[(139, 3), (140, -1)],
                &[]
            ),
            [(257, 1), (403, 3)]
        );
        assert_eq!(
            merged(
                &base(),
                &[],
                &[(139, 1), (140, 1)],
                &[(139, 3), (140, 0)],
                &[]
            ),
            [(257, 1), (403, 3)]
        );
    }

    #[test]
    fn of_two_launchers_sharing_an_ammo_type_the_higher_ids_rounds_win() {
        assert_eq!(
            merged(
                &base(),
                &[],
                &[(139, 1), (140, 1)],
                &[(139, 2), (140, 5)],
                &[]
            ),
            [(257, 1), (403, 5)],
            "set, not added"
        );
    }

    #[test]
    fn a_captured_weapon_outside_the_arrays_hands_over_nothing() {
        assert_eq!(
            merged(
                &base(),
                &[],
                &[(127, 1), (400, 1)],
                &[(127, 4), (400, 4)],
                &[]
            ),
            []
        );
    }
}
