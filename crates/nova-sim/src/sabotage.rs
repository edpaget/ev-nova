//! Sabotage outfits: the flag a sabotaged loadout sets, and the `dësc`s a
//! sabotaged player sees as it dies.
//!
//! # The flag
//!
//! [`sabotage_flag`] is `_playerSabotaged` as `_ResetPlayerPrecalcedValues`
//! sets it whenever the loadout changes: cleared (@0xc3ff), then for each
//! owned outfit (count > 0, @0xc5b2) and each of its four mod slots, a
//! [`SABOTAGE_KILL`] slot sets it to 1 (@0xc5d4-0xc5e8), and a
//! [`SABOTAGE_BOMB`] slot sets it to 2 unless it is already set
//! (@0xc5f6-0xc60a). So a kill slot anywhere wins, and no slot's `ModVal`
//! is read. `_InitObjects` clears it too (@0x1d11f). The engine derives
//! it fresh from the loadout and never stores it. The in-flight sabotage
//! tick (`_HandlePlayer` @0x692e5-0x69a11, `phase-11-in-flight-sabotage-tick`)
//! is the reader that tells [`Sabotage::Kill`] from [`Sabotage::Bomb`].
//!
//! # The death-time dialogs
//!
//! [`death_descs`] lists the `dësc`s `_HandlePlayer` shows once the
//! player's death hold expires (@0x68739-0x68832), each in its own text
//! dialog over the stopped flight scene.

use std::collections::BTreeMap;

use crate::catalog::{OutfitId, OutfitRecord};

/// The `ModType` of a sabotage slot that kills the player
/// (`_ResetPlayerPrecalcedValues` @0xc5d4), and whose `ModVal` names the
/// `dësc` shown as the player dies (`_HandlePlayer` @0x68772).
pub const SABOTAGE_KILL: i16 = 47;

/// The `ModType` of a sabotage slot that sets off bombs
/// (`_ResetPlayerPrecalcedValues` @0xc5f6-0xc60a).
pub const SABOTAGE_BOMB: i16 = 50;

/// `_playerSabotaged`: what a loadout's sabotage outfits do (the
/// original's 0, 1 and 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sabotage {
    /// No owned outfit has a sabotage slot (0).
    None,
    /// An owned outfit has a [`SABOTAGE_KILL`] slot (1).
    Kill,
    /// Owned outfits have [`SABOTAGE_BOMB`] slots but none a
    /// [`SABOTAGE_KILL`] one (2).
    Bomb,
}

/// The sabotage flag of the outfits `owned`, read from `records` (see the
/// module docs).
#[must_use]
pub fn sabotage_flag(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> Sabotage {
    let slots: Vec<i16> = held(owned, records)
        .flat_map(|record| record.mods.map(|(mod_type, _)| mod_type))
        .collect();
    if slots.contains(&SABOTAGE_KILL) {
        Sabotage::Kill
    } else if slots.contains(&SABOTAGE_BOMB) {
        Sabotage::Bomb
    } else {
        Sabotage::None
    }
}

/// The `dësc` IDs a dying sabotaged player sees, in order
/// (`_HandlePlayer` @0x68739-0x68832).
///
/// Nothing unless [`sabotage_flag`] is set (the gate @0x6873e). The gate
/// can't change the list, since any outfit that adds to it sets the
/// flag, but it is kept as the original has it. Then, for each outfit in
/// index order (@0x68828, whatever `records`' order) that `owned` holds
/// at least once (@0x6875a), the `ModVal` of its first
/// [`SABOTAGE_KILL`] slot with a `ModVal` above 0 (@0x68772, @0x6877d):
/// that slot's `dësc`, not the outfit's own (@0x687b7). An outfit adds
/// one at most, whatever its count (@0x68807). The loop plays no sound.
#[must_use]
pub fn death_descs(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> Vec<i16> {
    if sabotage_flag(owned, records) == Sabotage::None {
        return Vec::new();
    }
    let mut held: Vec<&OutfitRecord> = held(owned, records).collect();
    held.sort_by_key(|record| record.id);
    held.into_iter()
        .filter_map(|record| {
            record
                .mods
                .iter()
                .find(|&&(mod_type, mod_val)| mod_type == SABOTAGE_KILL && mod_val > 0)
                .map(|&(_, mod_val)| mod_val)
        })
        .collect()
}

/// The records of the outfits `owned` holds at least once.
fn held<'a>(
    owned: &'a BTreeMap<OutfitId, u16>,
    records: &'a [OutfitRecord],
) -> impl Iterator<Item = &'a OutfitRecord> + 'a {
    records
        .iter()
        .filter(|record| owned.get(&record.id).is_some_and(|&count| count > 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::outfit;

    fn owning(counts: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        counts.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    #[test]
    fn the_flag_is_none_when_the_only_sabotage_outfits_are_owned_zero_times() {
        let records = [
            outfit(128, &[(SABOTAGE_KILL, 5000)]),
            outfit(129, &[(SABOTAGE_BOMB, 1)]),
        ];
        let owned = owning(&[(128, 0), (129, 0)]);
        assert_eq!(sabotage_flag(&owned, &records), Sabotage::None);
        assert_eq!(sabotage_flag(&owning(&[]), &records), Sabotage::None);
    }

    #[test]
    fn a_kill_outfit_wins_over_a_bomb_outfit_before_it() {
        let records = [
            outfit(128, &[(SABOTAGE_BOMB, 1)]),
            outfit(129, &[(1, 3), (SABOTAGE_KILL, 5000)]),
        ];
        let owned = owning(&[(128, 1), (129, 1)]);
        assert_eq!(sabotage_flag(&owned, &records), Sabotage::Kill);
    }

    #[test]
    fn bomb_outfits_alone_give_the_bomb_flag() {
        let records = [
            outfit(128, &[(SABOTAGE_BOMB, 1)]),
            outfit(129, &[(SABOTAGE_KILL, 5000)]),
        ];
        let owned = owning(&[(128, 2)]);
        assert_eq!(sabotage_flag(&owned, &records), Sabotage::Bomb);
    }

    #[test]
    fn the_flag_ignores_mod_val() {
        let records = [outfit(128, &[(SABOTAGE_KILL, 0)])];
        assert_eq!(
            sabotage_flag(&owning(&[(128, 1)]), &records),
            Sabotage::Kill
        );
    }

    #[test]
    fn death_descs_are_the_slots_mod_val_not_the_outfits_own_desc() {
        // An outfit's own `dësc` is 3000 + its index (128 here).
        let records = [outfit(128, &[(SABOTAGE_KILL, 5000)])];
        assert_eq!(death_descs(&owning(&[(128, 1)]), &records), [5000]);
    }

    #[test]
    fn death_descs_list_an_outfit_once_whatever_its_count() {
        let records = [outfit(128, &[(SABOTAGE_KILL, 5000)])];
        assert_eq!(death_descs(&owning(&[(128, 3)]), &records), [5000]);
    }

    #[test]
    fn death_descs_skip_a_non_positive_mod_val_for_a_later_slot_on_the_same_outfit() {
        let records = [outfit(
            128,
            &[
                (SABOTAGE_KILL, 0),
                (SABOTAGE_KILL, -5),
                (SABOTAGE_KILL, 700),
            ],
        )];
        assert_eq!(death_descs(&owning(&[(128, 1)]), &records), [700]);
    }

    #[test]
    fn death_descs_take_only_the_first_qualifying_slot() {
        let records = [outfit(
            128,
            &[
                (SABOTAGE_BOMB, 9),
                (SABOTAGE_KILL, 700),
                (SABOTAGE_KILL, 701),
            ],
        )];
        assert_eq!(death_descs(&owning(&[(128, 1)]), &records), [700]);
    }

    #[test]
    fn death_descs_list_outfits_in_index_order() {
        let records = [
            outfit(130, &[(SABOTAGE_KILL, 730)]),
            outfit(128, &[(SABOTAGE_KILL, 728)]),
            outfit(129, &[(SABOTAGE_KILL, 729)]),
        ];
        let owned = owning(&[(130, 1), (128, 1)]);
        assert_eq!(death_descs(&owned, &records), [728, 730]);
    }

    #[test]
    fn death_descs_are_empty_for_bombs_only_and_without_sabotage() {
        let records = [
            outfit(128, &[(SABOTAGE_BOMB, 728)]),
            outfit(129, &[(2, 729)]),
        ];
        assert_eq!(
            death_descs(&owning(&[(128, 1)]), &records),
            Vec::<i16>::new()
        );
        assert_eq!(
            death_descs(&owning(&[(129, 1)]), &records),
            Vec::<i16>::new()
        );
    }
}
