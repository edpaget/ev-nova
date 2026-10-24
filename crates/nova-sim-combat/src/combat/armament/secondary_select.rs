//! `_HandlePlayer`'s secondary-weapon block (@0x6abcc-0x6ae20 in the `EV
//! Nova` executable): which secondary the Weapon Select key picks
//! ([`select_secondary`]) and which selection running out of ammunition
//! lets go of ([`empty_lets_go`]). The session runs the block and its
//! sounds (`nova_sim::session`'s `secondary_select`).
//!
//! The selection is the word at the ship's +0x72, the weapon's slot
//! (`wëap` ID − 128), or -1 for none. The select key steps it by +1, or
//! -1 with Option down (`QuickKeyCheck(0x3a)` @0x6acf6), wrapping into
//! slots 0..255 by ±256 (@0x6ad11-0x6ad2f), until a slot is a candidate
//! (@0x6ad3e-0x6ad89). So the order is `wëap` ID order, whatever order the
//! ship mounts its weapons in: forward from none it starts at slot 0 (wëap
//! 128); backward from none at slot 254 (-1 − 1 = -2, wrapped), so wëap
//! 382 comes before 383, which comes last; a stale selection steps from
//! its own slot. The original counts the candidates first
//! (@0x6ac48-0x6acb0) and refuses with none, so the step always ends.
//!
//! A slot is a candidate when all four hold:
//! - the ship carries it (the count at +0xc8 + 0xc8 × slot above 0,
//!   @0x6ac51);
//! - it has `Flags` 0x0002, a secondary (+0x20 & 2, @0x6ac67);
//! - its raw `Guidance` (+0x6), signed, is 8 or less, or 99
//!   (@0x6ac6d-0x6ac81), so point defence (9, 10) and 11-98 are never
//!   picked ([`selectable_guidance`]);
//! - with `Flags2` 0x0800 ([`HIDE_WHEN_EMPTY`](super::super::weapon::HIDE_WHEN_EMPTY),
//!   +0x23 & 8, @0x6ac83), `_WeaponHasAmmo` holds (@0x6ac93).
//!
//! The let-go (@0x6abcc-0x6ac08) tests the selection's `Flags` 0x0800
//! ([`PERSISTENT_SMOKE`], +0x21 & 8, @0x6abe9), the Bible's "more
//! persistent smoke", before `_WeaponHasAmmo` (@0x6abfa): by the other
//! reading of [`RuleKey::EmptySecondary`], `Flags2` 0x0800, the bit the
//! select key tests.
//!
//! [`RuleKey::EmptySecondary`]: crate::RuleKey::EmptySecondary

use super::Mount;
use crate::bay::FIGHTER_BAY;
use crate::catalog::WeaponId;
use crate::combat::weapon::{FIRST_AMMO_WEAPON, Guidance, PERSISTENT_SMOKE, WeaponSpec};
use crate::rulebook::RuleSource;

/// The weapon slots the select key steps through, 0..255 (wëap 128-383).
const SLOTS: i32 = 256;

/// The secondary the Weapon Select key picks from `current`, `backwards`
/// (Option down) or not, among `mounts`, `has_ammo` saying whether a
/// mount has the ammunition for a shot (`_WeaponHasAmmo`): the next
/// candidate in `wëap` ID order, wrapping (see the module docs); `current`
/// itself when it is the only candidate; none when there is no candidate,
/// when the original keeps the selection and refuses.
pub fn select_secondary(
    mounts: &[Mount],
    current: Option<WeaponId>,
    backwards: bool,
    has_ammo: impl Fn(&Mount) -> bool,
) -> Option<WeaponId> {
    let slot = |id: WeaponId| i32::from(id.0) - i32::from(FIRST_AMMO_WEAPON);
    let mut candidates = [false; SLOTS as usize];
    for mount in mounts.iter().filter(|mount| candidate(mount, &has_ammo)) {
        if let Ok(at) = usize::try_from(slot(mount.spec.id))
            && let Some(taken) = candidates.get_mut(at)
        {
            *taken = true;
        }
    }
    if !candidates.contains(&true) {
        return None;
    }
    let step = if backwards { -1 } else { 1 };
    let mut at = current.map_or(-1, slot);
    loop {
        at = (at + step).rem_euclid(SLOTS);
        // `at` is in 0..256, so both conversions hold.
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        if candidates[at as usize] {
            return Some(WeaponId(at as i16 + FIRST_AMMO_WEAPON));
        }
    }
}

/// Whether the select key can pick `mount` (see the module docs).
fn candidate(mount: &Mount, has_ammo: &impl Fn(&Mount) -> bool) -> bool {
    let spec = &mount.spec;
    mount.count > 0
        && spec.secondary()
        && selectable_guidance(spec.guidance)
        && (!spec.hides_when_empty() || has_ammo(mount))
}

/// Whether the select key takes a weapon of `guidance`: a raw guidance of
/// 8 or less, or 99 (@0x6ac6d-0x6ac81). Point defence (9, 10) and every
/// other raw value from 11 to 98 are never picked.
#[must_use]
pub fn selectable_guidance(guidance: Guidance) -> bool {
    match guidance {
        Guidance::PointDefence | Guidance::PointDefenceBeam => false,
        Guidance::Other(raw) => raw <= 8 || raw == FIGHTER_BAY,
        _ => true,
    }
}

/// Whether running out of ammunition lets go of a selected `spec` by
/// `source`'s reading of [`RuleKey::EmptySecondary`]: by the engine's, a
/// weapon of `Flags` [`PERSISTENT_SMOKE`] (@0x6abe9); by the other, one
/// hidden when empty (`Flags2` 0x0800, [`WeaponSpec::hides_when_empty`]).
/// The caller then asks `_WeaponHasAmmo`.
///
/// [`RuleKey::EmptySecondary`]: crate::RuleKey::EmptySecondary
#[must_use]
pub fn empty_lets_go(spec: &WeaponSpec, source: RuleSource) -> bool {
    match source {
        RuleSource::Engine => spec.flags & PERSISTENT_SMOKE != 0,
        RuleSource::Bible => spec.hides_when_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::combat::weapon::{HIDE_WHEN_EMPTY, SECONDARY};
    use nova_sim_catalog::fixture::weapon;

    /// A weapon `id` of `flags`, `flags2` and raw `guidance`, carried once.
    fn mount(id: i16, flags: u16, flags2: u16, guidance: i16) -> Mount {
        Mount {
            spec: WeaponSpec::new(&WeaponRecord {
                flags,
                flags2,
                guidance,
                ..weapon(id)
            }),
            count: 1,
            reload: 0.0,
            burst: 0,
        }
    }

    /// An unguided secondary `id`.
    fn secondary(id: i16) -> Mount {
        mount(id, SECONDARY, 0, -1)
    }

    fn ids(ids: &[i16]) -> Vec<WeaponId> {
        ids.iter().copied().map(WeaponId).collect()
    }

    /// The secondaries picked in turn from `start` among `mounts`,
    /// `backwards` or not, each with its ammunition.
    fn picks(mounts: &[Mount], start: Option<i16>, backwards: bool, turns: usize) -> Vec<WeaponId> {
        let mut current = start.map(WeaponId);
        let mut picked = Vec::new();
        for _ in 0..turns {
            current = select_secondary(mounts, current, backwards, |_| true);
            picked.extend(current);
        }
        picked
    }

    #[test]
    fn the_select_key_goes_in_weap_id_order_whatever_the_mount_order() {
        let mounts = [secondary(142), secondary(140), secondary(141)];
        assert_eq!(picks(&mounts, None, false, 4), ids(&[140, 141, 142, 140]));
        assert_eq!(picks(&mounts, Some(141), false, 2), ids(&[142, 140]));
    }

    #[test]
    fn option_steps_backwards_and_wraps() {
        let mounts = [secondary(142), secondary(140), secondary(141)];
        assert_eq!(picks(&mounts, Some(141), true, 3), ids(&[140, 142, 141]));
    }

    #[test]
    fn backwards_from_none_starts_at_382_and_383_comes_last() {
        let mounts = [secondary(383), secondary(140), secondary(382)];
        assert_eq!(picks(&mounts, None, true, 4), ids(&[382, 140, 383, 382]));
    }

    #[test]
    fn forward_from_none_starts_at_the_lowest_id() {
        let mounts = [secondary(383), secondary(129), secondary(128)];
        assert_eq!(picks(&mounts, None, false, 3), ids(&[128, 129, 383]));
    }

    #[test]
    fn a_stale_selection_steps_from_its_own_slot() {
        let mounts = [secondary(140), secondary(200), secondary(141)];
        assert_eq!(picks(&mounts, Some(199), false, 1), ids(&[200]));
        assert_eq!(picks(&mounts, Some(199), true, 1), ids(&[141]));
        assert_eq!(picks(&mounts, Some(383), false, 1), ids(&[140]), "wraps");
        assert_eq!(picks(&mounts, Some(128), true, 1), ids(&[200]), "wraps");
    }

    #[test]
    fn a_selection_outside_the_slots_wraps_into_them() {
        // Slot 300 + 1 wraps to 45 (wëap 173); slot -129 - 1 to 126 (254).
        let mounts = [
            secondary(172),
            secondary(174),
            secondary(253),
            secondary(255),
        ];
        assert_eq!(picks(&mounts, Some(428), false, 1), ids(&[174]));
        assert_eq!(picks(&mounts, Some(-1), true, 1), ids(&[253]));
    }

    #[test]
    fn a_primary_is_never_selected() {
        let mounts = [secondary(140), mount(141, 0, 0, -1), secondary(142)];
        assert_eq!(picks(&mounts, Some(140), false, 1), ids(&[142]));
    }

    #[test]
    fn a_weapon_carried_none_of_is_never_selected() {
        let mut none = secondary(141);
        none.count = 0;
        let mounts = [secondary(140), none, secondary(142)];
        assert_eq!(picks(&mounts, Some(140), false, 1), ids(&[142]));
    }

    #[test]
    fn point_defence_and_guidance_past_8_but_99_are_never_selected() {
        let mounts = [
            mount(140, SECONDARY, 0, 9),
            mount(141, SECONDARY, 0, 10),
            mount(142, SECONDARY, 0, 11),
            mount(143, SECONDARY, 0, 98),
            secondary(150),
        ];
        assert_eq!(picks(&mounts, None, false, 2), ids(&[150, 150]));
    }

    #[test]
    fn every_guidance_of_8_or_less_and_99_is_selected() {
        let mounts = [
            mount(140, SECONDARY, 0, -1),
            mount(141, SECONDARY, 0, 0),
            mount(142, SECONDARY, 0, 2),
            mount(143, SECONDARY, 0, 8),
            mount(144, SECONDARY, 0, FIGHTER_BAY),
            mount(145, SECONDARY, 0, -5),
        ];
        assert_eq!(
            picks(&mounts, None, false, 6),
            ids(&[140, 141, 142, 143, 144, 145])
        );
    }

    #[test]
    fn a_bay_launching_a_ship_is_selected() {
        let mut bay = mount(140, SECONDARY, 0, FIGHTER_BAY);
        bay.spec = WeaponSpec::new(&WeaponRecord {
            flags: SECONDARY,
            guidance: FIGHTER_BAY,
            ammo_type: 144,
            ..weapon(140)
        });
        assert_eq!(bay.spec.guidance, Guidance::FighterBay);
        assert_eq!(picks(&[bay], None, false, 1), ids(&[140]));
    }

    #[test]
    fn the_selectable_guidances() {
        let taken = [
            Guidance::Unguided,
            Guidance::Beam,
            Guidance::Homing,
            Guidance::TurretBeam,
            Guidance::Turret,
            Guidance::FreefallBomb,
            Guidance::Rocket,
            Guidance::FrontTurret,
            Guidance::RearTurret,
            Guidance::FighterBay,
            Guidance::Other(99),
            Guidance::Other(8),
            Guidance::Other(-5),
        ];
        for guidance in taken {
            assert!(selectable_guidance(guidance), "{guidance:?}");
        }
        let refused = [
            Guidance::PointDefence,
            Guidance::PointDefenceBeam,
            Guidance::Other(9),
            Guidance::Other(11),
            Guidance::Other(98),
            Guidance::Other(100),
        ];
        for guidance in refused {
            assert!(!selectable_guidance(guidance), "{guidance:?}");
        }
    }

    #[test]
    fn a_secondary_hidden_when_empty_is_skipped_without_ammunition() {
        let mounts = [
            secondary(140),
            mount(141, SECONDARY, HIDE_WHEN_EMPTY, -1),
            secondary(142),
        ];
        let empty = |mount: &Mount| mount.spec.id != WeaponId(141);
        assert_eq!(
            select_secondary(&mounts, Some(WeaponId(140)), false, empty),
            Some(WeaponId(142))
        );
        assert_eq!(
            select_secondary(&mounts, Some(WeaponId(140)), false, |_| true),
            Some(WeaponId(141)),
            "with it"
        );
    }

    #[test]
    fn a_secondary_that_does_not_hide_is_selected_without_ammunition() {
        let mounts = [
            secondary(140),
            mount(141, SECONDARY | PERSISTENT_SMOKE, 0, -1),
        ];
        assert_eq!(
            select_secondary(&mounts, Some(WeaponId(140)), false, |_| false),
            Some(WeaponId(141))
        );
    }

    #[test]
    fn the_only_candidate_selected_picks_itself() {
        let mounts = [secondary(140), mount(141, SECONDARY, 0, 9)];
        assert_eq!(picks(&mounts, Some(140), false, 1), ids(&[140]));
        assert_eq!(picks(&mounts, Some(140), true, 1), ids(&[140]));
    }

    #[test]
    fn with_no_candidate_there_is_none() {
        assert_eq!(select_secondary(&[], None, false, |_| true), None);
        let mounts = [
            mount(140, 0, 0, -1),
            mount(141, SECONDARY, HIDE_WHEN_EMPTY, -1),
        ];
        assert_eq!(
            select_secondary(&mounts, Some(WeaponId(141)), true, |_| false),
            None
        );
    }

    #[test]
    fn a_weapon_past_383_is_never_selected() {
        assert_eq!(picks(&[secondary(384)], None, false, 1), []);
        assert_eq!(picks(&[secondary(127)], None, false, 1), []);
        let mounts = [secondary(384), secondary(140)];
        assert_eq!(picks(&mounts, Some(140), false, 1), ids(&[140]));
    }

    #[test]
    fn running_out_lets_go_of_persistent_smoke_by_the_engine() {
        let smoke = mount(140, SECONDARY | PERSISTENT_SMOKE, 0, -1).spec;
        let hides = mount(141, SECONDARY, HIDE_WHEN_EMPTY, -1).spec;
        let plain = secondary(142).spec;
        assert!(empty_lets_go(&smoke, RuleSource::Engine));
        assert!(!empty_lets_go(&hides, RuleSource::Engine));
        assert!(!empty_lets_go(&plain, RuleSource::Engine));
    }

    #[test]
    fn running_out_lets_go_of_one_hidden_when_empty_by_the_other_reading() {
        let smoke = mount(140, SECONDARY | PERSISTENT_SMOKE, 0, -1).spec;
        let hides = mount(141, SECONDARY, HIDE_WHEN_EMPTY, -1).spec;
        let plain = secondary(142).spec;
        assert!(!empty_lets_go(&smoke, RuleSource::Bible));
        assert!(empty_lets_go(&hides, RuleSource::Bible));
        assert!(!empty_lets_go(&plain, RuleSource::Bible));
    }
}
