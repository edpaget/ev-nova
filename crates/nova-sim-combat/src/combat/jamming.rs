//! Jamming: how strongly a ship jams each of the four jamming types, and
//! whether a homing shot is jammed by it.
//!
//! **A ship's jamming** is `_ShipECM(ship, type)` (@0x3aff-0x3d48), for
//! each type 0-3 (Jamming 1-4):
//!
//! - The base is the `InhJam` of the type of the government whose
//!   attributes its class takes (`InherentGovt`, shïp runtime +0x9f6,
//!   [`ShipRecord::inherent_govt`](crate::catalog::ShipRecord::inherent_govt)),
//!   or none without one (@0x3b51-0x3ba9).
//! - The player adds the `ModVal` of every mod of `ModType` 33 + type of
//!   each outfit it owns, an outfit counted **once whatever its count**
//!   (@0x3bb6-0x3c1c), and is never halved ([`Jamming::of_player`]).
//! - An NPC adds those of each of its class's `DefaultItems` slots with a
//!   count above none, an outfit in two slots twice, its count never
//!   multiplied; outfits it gained any other way are never read
//!   (@0x3c21-0x3cae). Its current government's `Flags` 0x0080
//!   ([`HALF_JAMMING`](crate::govt::HALF_JAMMING)) then halves the sum
//!   toward zero, whatever its AI type (@0x3ccf-0x3cef), as [`Halving`]
//!   reads [`RuleKey::JamHalving`](crate::RuleKey::JamHalving)
//!   ([`Jamming::of_npc`]).
//! - Either is clamped to 0-100 after any halving (@0x3cf7-0x3d33).
//! - A disabled ship (`_IsDisabled`: disabled, breaking up or held
//!   disabled) jams nothing (@0x3b27), whatever it carries
//!   ([`Jamming::against`]).
//!
//! **The cache.** The original caches each value (+0xc926) until a spawn
//! or `_ResetPlayerPrecalcedValues` clears it (@0x3c72e, @0x3cc14,
//! @0x3da57, @0x3ffd0, @0x947c5, @0xc7be-0xc7d6). Here a ship's jamming is
//! worked out from its current inputs each fight tick, which is the same
//! value wherever the engine goes: an NPC's class and government never
//! change in flight, and the player's outfits change in flight only where
//! the reset runs too, but for its ammunition. Two stale cases are not
//! modelled: a ship taken over by its boarder (`_AIPlunderShipContents`
//! @0x84995 sets the government without a reset, so the halving reads the
//! old one), as the engine has no NPC takeover; and a jammer that is also
//! ammunition, whose last round fired keeps jamming until the next reset,
//! as no stock jammer is ammunition.
//!
//! **The jam test** ([`jams`], `_HandleShotGuidance` @0x321b7-0x32275): a
//! homing shot rolls each type at launch (`_SpawnShot` @0x3f051-0x3f0a7,
//! [`roll`]), and is jammed this tick when, for the first type whose roll
//! is above none, its target's jamming of the type is above 100 less the
//! roll. What being jammed does to it is
//! [`Shot`](super::projectile::Shot)'s: it plays no sound.

use std::collections::BTreeMap;

use crate::catalog::{OutfitId, OutfitRecord};
use crate::chance::Chance;
use crate::rulebook::RuleSource;

/// How many jamming types there are.
pub const JAMMING_TYPES: usize = 4;

/// The `ModType` of Jamming 1; Jamming 2-4 follow it (33 + type,
/// @0x3bc1).
pub const JAMMING_MOD: i16 = 33;

/// The most a ship jams of a type, and the roll a type's jamming is taken
/// from.
pub const FULL_JAMMING: i32 = 100;

/// How strongly a ship jams each jamming type, 0 to 100 (`_ShipECM`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Jamming(pub [i16; JAMMING_TYPES]);

/// How an NPC's government halves its jamming.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Halving {
    /// Not at all.
    #[default]
    None,
    /// Its whole sum, the original's for every ship of a `Flags` 0x0080
    /// government.
    Whole,
    /// Only its `InhJam` share, the Bible's for a freighter (AI type 1 or
    /// 2) of such a government, at "50% of the standard `InherentJam`
    /// value".
    InherentShare,
}

impl Halving {
    /// How an NPC is halved: its government `flagged` with `Flags` 0x0080
    /// or not, a `freighter` (AI type 1 or 2) or not, as `source` reads
    /// [`RuleKey::JamHalving`](crate::RuleKey::JamHalving).
    #[must_use]
    pub fn of(flagged: bool, freighter: bool, source: RuleSource) -> Self {
        match (flagged, source) {
            (true, RuleSource::Engine) => Self::Whole,
            (true, RuleSource::Bible) if freighter => Self::InherentShare,
            _ => Self::None,
        }
    }
}

impl Jamming {
    /// No jamming at all.
    pub const NONE: Self = Self([0; JAMMING_TYPES]);

    /// The player's jamming: `base`, its class government's `InhJam`, and
    /// once each outfit of `outfits` it has `owned` (see the module docs).
    #[must_use]
    pub fn of_player(
        base: [i16; JAMMING_TYPES],
        owned: &BTreeMap<OutfitId, u16>,
        outfits: &[OutfitRecord],
    ) -> Self {
        let carried = outfits
            .iter()
            .filter(|record| owned.get(&record.id).is_some_and(|&count| count > 0));
        Self::clamped(base.map(i32::from), sums(carried))
    }

    /// An NPC's jamming: `base`, its class government's `InhJam`, and each
    /// of its class's `defaults` slots of `outfits` with a count, as its
    /// government's `halving` says (see the module docs).
    #[must_use]
    pub fn of_npc(
        base: [i16; JAMMING_TYPES],
        defaults: &[(OutfitId, u16)],
        outfits: &[OutfitRecord],
        halving: Halving,
    ) -> Self {
        let carried = defaults
            .iter()
            .filter(|&&(_, count)| count > 0)
            .filter_map(|&(id, _)| outfits.iter().find(|record| record.id == id));
        let base = base.map(i32::from);
        let items = sums(carried);
        match halving {
            Halving::None => Self::clamped(base, items),
            // Signed division, toward zero (@0x3ce6-0x3cec).
            Halving::Whole => {
                Self::clamped([0; JAMMING_TYPES], add(base, items).map(|sum| sum / 2))
            }
            Halving::InherentShare => Self::clamped(base.map(|share| share / 2), items),
        }
    }

    /// `base` and `items`, each type clamped to 0-100.
    fn clamped(base: [i32; JAMMING_TYPES], items: [i32; JAMMING_TYPES]) -> Self {
        let sum = add(base, items);
        // Within 0..=100, so within an `i16`.
        #[allow(clippy::cast_possible_truncation)]
        Self(sum.map(|sum| sum.clamp(0, FULL_JAMMING) as i16))
    }

    /// How strongly it jams type `kind` (0-3) on a ship `disabled` or not:
    /// `_ShipECM`'s answer, none for a disabled ship (@0x3b27).
    #[must_use]
    pub fn against(self, kind: usize, disabled: bool) -> i16 {
        if disabled { 0 } else { self.0[kind] }
    }
}

/// The jamming `ModVal`s of each type over `carried`, every mod slot of
/// each outfit read.
fn sums<'a>(carried: impl Iterator<Item = &'a OutfitRecord>) -> [i32; JAMMING_TYPES] {
    let mut sums = [0; JAMMING_TYPES];
    for (kind, mod_val) in carried.flat_map(|record| record.mods) {
        let Some(index) = kind
            .checked_sub(JAMMING_MOD)
            .and_then(|index| usize::try_from(index).ok())
            .filter(|&index| index < JAMMING_TYPES)
        else {
            continue;
        };
        sums[index] += i32::from(mod_val);
    }
    sums
}

/// `a` and `b`, type by type.
fn add(a: [i32; JAMMING_TYPES], b: [i32; JAMMING_TYPES]) -> [i32; JAMMING_TYPES] {
    std::array::from_fn(|kind| a[kind] + b[kind])
}

/// A shot's rolls for each type, drawn on `chance` as it is launched
/// (`_SpawnShot` @0x3f051-0x3f0a7): `below(JamVuln + 1)`, 0 to its
/// weapon's `jam_vuln`, for a `JamVuln` above none, and none otherwise.
#[must_use]
pub fn roll(jam_vuln: [i16; JAMMING_TYPES], chance: &mut dyn Chance) -> [u16; JAMMING_TYPES] {
    jam_vuln.map(|vuln| match u16::try_from(vuln) {
        Ok(vuln) if vuln > 0 => {
            // Within 0..=vuln, so within a `u16`.
            #[allow(clippy::cast_possible_truncation)]
            {
                chance.below(u32::from(vuln) + 1) as u16
            }
        }
        _ => 0,
    })
}

/// Whether a shot that took `rolls` at launch is jammed this tick by a
/// target of `jamming`, `disabled` or not (see the module docs): for the
/// first type whose roll is above none and whose jamming is above 100
/// less it, which ends the test (@0x32275).
#[must_use]
pub fn jams(rolls: [u16; JAMMING_TYPES], jamming: Jamming, disabled: bool) -> bool {
    (0..JAMMING_TYPES).any(|kind| {
        let roll = i32::from(rolls[kind]);
        roll > 0 && i32::from(jamming.against(kind, disabled)) > FULL_JAMMING - roll
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_base::fixture::Ranged;
    use nova_sim_catalog::fixture::outfit;

    /// Outfits 240 (+30 to Jamming 1), 238 (+20 to Jamming 1 and +5 to
    /// Jamming 3), 241 (-10 to Jamming 2), 242 (a cloak and +50 to
    /// Jamming 4 in its second slot) and 300, a cloak alone.
    fn jammers() -> Vec<OutfitRecord> {
        vec![
            outfit(238, &[(JAMMING_MOD, 20), (JAMMING_MOD + 2, 5)]),
            outfit(240, &[(JAMMING_MOD, 30)]),
            outfit(241, &[(JAMMING_MOD + 1, -10)]),
            outfit(242, &[(17, 1), (JAMMING_MOD + 3, 50)]),
            outfit(300, &[(17, 1)]),
        ]
    }

    fn owned(items: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        items
            .iter()
            .map(|&(id, count)| (OutfitId(id), count))
            .collect()
    }

    fn slots(items: &[(i16, u16)]) -> Vec<(OutfitId, u16)> {
        items
            .iter()
            .map(|&(id, count)| (OutfitId(id), count))
            .collect()
    }

    #[test]
    fn the_player_jams_its_class_governments_inh_jam_and_each_jammer_it_owns_once() {
        let outfits = jammers();
        let player = |base, items: &[(i16, u16)]| Jamming::of_player(base, &owned(items), &outfits);
        assert_eq!(player([7, 0, 0, 0], &[]), Jamming([7, 0, 0, 0]));
        assert_eq!(player([7, 0, 0, 0], &[(240, 1)]), Jamming([37, 0, 0, 0]));
        assert_eq!(
            player([7, 0, 0, 0], &[(240, 3)]),
            Jamming([37, 0, 0, 0]),
            "once whatever its count"
        );
        assert_eq!(
            player([7, 0, 0, 0], &[(240, 0)]),
            Jamming([7, 0, 0, 0]),
            "a count of none adds nothing"
        );
        assert_eq!(
            player(
                [0, 15, 0, 0],
                &[(238, 1), (240, 1), (241, 1), (242, 1), (300, 2)]
            ),
            Jamming([50, 5, 5, 50]),
            "each type its own; a negative ModVal takes away; other mods add nothing"
        );
        let past = vec![outfit(250, &[(JAMMING_MOD + 4, 40), (JAMMING_MOD - 1, 40)])];
        assert_eq!(
            Jamming::of_player([0, 0, 0, 0], &owned(&[(250, 1)]), &past),
            Jamming::NONE,
            "the mods either side of the four jamming types add nothing"
        );
        assert_eq!(
            player([0, 0, 0, 0], &[(999, 1)]),
            Jamming::NONE,
            "an outfit with no record adds nothing"
        );
    }

    #[test]
    fn the_players_jamming_is_clamped_to_0_100_and_never_halved() {
        let outfits = vec![
            outfit(240, &[(JAMMING_MOD, 90)]),
            outfit(241, &[(JAMMING_MOD + 1, -50)]),
        ];
        let jamming = Jamming::of_player([30, 10, 0, 0], &owned(&[(240, 1), (241, 1)]), &outfits);
        assert_eq!(jamming, Jamming([100, 0, 0, 0]));
    }

    #[test]
    fn an_npc_jams_each_default_slot_with_a_count_the_same_outfit_twice_in_two_slots() {
        let outfits = jammers();
        let npc = |base, items: &[(i16, u16)]| {
            Jamming::of_npc(base, &slots(items), &outfits, Halving::None)
        };
        assert_eq!(npc([7, 0, 0, 0], &[(240, 1)]), Jamming([37, 0, 0, 0]));
        assert_eq!(
            npc([7, 0, 0, 0], &[(240, 4)]),
            Jamming([37, 0, 0, 0]),
            "a slot counts once whatever its count"
        );
        assert_eq!(
            npc([7, 0, 0, 0], &[(240, 1), (240, 2)]),
            Jamming([67, 0, 0, 0]),
            "two slots, twice"
        );
        assert_eq!(
            npc([7, 0, 0, 0], &[(240, 0), (238, 1)]),
            Jamming([27, 0, 5, 0]),
            "a slot of no count adds nothing"
        );
        assert_eq!(
            npc([0, 0, 0, 0], &[(241, 1)]),
            Jamming::NONE,
            "clamped at none"
        );
    }

    #[test]
    fn a_halving_government_halves_an_npcs_whole_sum_toward_zero_before_the_clamp() {
        let outfits = vec![
            outfit(240, &[(JAMMING_MOD, 50)]),
            outfit(241, &[(JAMMING_MOD + 1, -50)]),
            outfit(250, &[(JAMMING_MOD + 2, 243)]),
        ];
        let halved = Jamming::of_npc(
            [7, 7, 7, 7],
            &slots(&[(240, 1), (241, 1), (250, 1)]),
            &outfits,
            Halving::Whole,
        );
        assert_eq!(
            halved,
            Jamming([28, 0, 100, 3]),
            "57 to 28, -43 to -21 then 0, 250 to 125 then 100, 7 to 3"
        );
        let whole = Jamming::of_npc(
            [7, 7, 7, 7],
            &slots(&[(240, 1), (241, 1), (250, 1)]),
            &outfits,
            Halving::None,
        );
        assert_eq!(whole, Jamming([57, 0, 100, 7]));
    }

    #[test]
    fn by_the_bible_a_freighter_halves_only_its_inh_jam_share() {
        let outfits = vec![outfit(240, &[(JAMMING_MOD, 50)])];
        let freighter = Jamming::of_npc(
            [7, 9, 0, 0],
            &slots(&[(240, 1)]),
            &outfits,
            Halving::InherentShare,
        );
        assert_eq!(freighter, Jamming([53, 4, 0, 0]), "7 to 3 plus 50; 9 to 4");
    }

    #[test]
    fn the_halving_follows_the_government_flag_and_by_the_bible_the_freighters_alone() {
        for freighter in [false, true] {
            for source in [RuleSource::Engine, RuleSource::Bible] {
                assert_eq!(
                    Halving::of(false, freighter, source),
                    Halving::None,
                    "{freighter} {source:?}: no flag"
                );
            }
            assert_eq!(
                Halving::of(true, freighter, RuleSource::Engine),
                Halving::Whole,
                "{freighter}: the engine halves every ship of the government"
            );
        }
        assert_eq!(
            Halving::of(true, true, RuleSource::Bible),
            Halving::InherentShare
        );
        assert_eq!(Halving::of(true, false, RuleSource::Bible), Halving::None);
    }

    #[test]
    fn a_disabled_ship_jams_nothing() {
        let jamming = Jamming([30, 0, 12, 100]);
        assert_eq!(jamming.against(0, false), 30);
        assert_eq!(jamming.against(2, false), 12);
        assert_eq!(jamming.against(3, false), 100);
        for kind in 0..JAMMING_TYPES {
            assert_eq!(jamming.against(kind, true), 0, "{kind}");
        }
    }

    #[test]
    fn a_shot_rolls_each_type_from_none_to_its_jam_vuln_and_none_without_one() {
        let mut chance = Ranged::of(&[(41, 17), (101, 100)]);
        assert_eq!(roll([40, 0, -5, 100], &mut chance), [17, 0, 0, 100]);
        assert!(chance.was_asked(41) && chance.was_asked(101));
        assert_eq!(
            chance.asked.len(),
            2,
            "no roll for a JamVuln of none or less"
        );
        let mut last = Ranged::of(&[]);
        assert_eq!(
            roll([1, 2, 3, 4], &mut last),
            [1, 2, 3, 4],
            "each at most its JamVuln"
        );
    }

    #[test]
    fn a_shot_is_jammed_when_a_types_jamming_is_above_100_less_its_roll() {
        let thirty = Jamming([30, 0, 0, 0]);
        assert!(jams([71, 0, 0, 0], thirty, false));
        assert!(!jams([70, 0, 0, 0], thirty, false), "exactly 100 less 30");
        assert!(!jams([71, 0, 0, 0], thirty, true), "disabled");
        assert!(
            jams([101, 0, 0, 0], Jamming::NONE, false),
            "a roll above 100 is jammed by none"
        );
        assert!(
            !jams([0, 0, 0, 0], Jamming([100, 100, 100, 100]), false),
            "a roll of none is skipped"
        );
        assert!(
            !jams([0, 0, 0, 0], Jamming([i16::MAX; JAMMING_TYPES]), false),
            "a roll of none is skipped whatever the jamming"
        );
        let second = Jamming([100, 40, 0, 0]);
        assert!(
            jams([0, 61, 0, 0], second, false),
            "type 2's roll against type 2's"
        );
        assert!(
            !jams([0, 60, 0, 0], second, false),
            "type 1's jamming ignored"
        );
        assert!(jams([0, 0, 0, 1], Jamming([0, 0, 0, 100]), false));
    }
}
