//! `_HandleShip`'s idle-bay hold (@0x3390a-0x3394e in the `EV Nova`
//! executable): an NPC with no target holds its bays at their full
//! `Reload` ([`Armament::hold_idle_bays`]).
//!
//! After the reload countdown (@0x33896-0x338ce) and the ion hold
//! (@0x338d6-0x338f0), an NPC whose target (+0x70) is -1 (none, or a
//! ship whose slot is gone, cleared just before at @0x33865-0x33888) sets
//! the reload of every weapon of guidance 99 (+0x6 = 0x63) it carries
//! (count +0xc8 above 0) to its `Reload` (+0x30), every frame: not divided
//! by the count, and whatever rounds it holds. So a bay counts down only
//! from the first frame its ship has a target, and `_AILaunchFighter`,
//! which runs before `_HandleShip` in a frame, first finds it ready
//! `Reload` frames later. The player's ship never takes it
//! (`_HandlePlayer`).

use super::Armament;

impl Armament {
    /// Holds every mount of raw guidance 99 at its full `Reload` (see the
    /// module docs); every other weapon is left as it is. The original's
    /// count test (+0xc8 above 0) always holds here, as an armament keeps
    /// no mount of none ([`Armament::new`]).
    pub fn hold_idle_bays(&mut self) {
        for mount in &mut self.mounts {
            if mount.spec.guidance_99() {
                mount.reload = mount.spec.reload;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::combat::weapon::WeaponSpec;
    use nova_sim_catalog::fixture::weapon;

    /// A bay of `Reload` 60 launching ship 144.
    fn bay(id: i16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance: 99,
            ammo_type: 144,
            reload: 60,
            ..weapon(id)
        })
    }

    /// Each mount's reload.
    fn reloads(armament: &Armament) -> Vec<f32> {
        armament.mounts().iter().map(|mount| mount.reload).collect()
    }

    /// `weapons` with each count, each mount's reload at 3.
    fn armed(weapons: &[(WeaponSpec, u32)]) -> Armament {
        let mut armament = Armament::new(weapons.iter().copied());
        for mount in &mut armament.mounts {
            mount.reload = 3.0;
        }
        armament
    }

    #[test]
    fn every_bay_is_held_at_its_full_reload_and_every_other_weapon_left() {
        let gun = WeaponSpec::new(&WeaponRecord {
            reload: 20,
            ..weapon(128)
        });
        // Two bays: the full `Reload`, not divided by the count.
        let mut armament = armed(&[(gun, 1), (bay(149), 2)]);
        armament.hold_idle_bays();
        assert_eq!(reloads(&armament), [3.0, 60.0]);
    }

    #[test]
    fn a_guidance_99_weapon_carrying_no_ship_is_held_too() {
        // The original tests the raw guidance (+0x6 = 0x63) alone.
        let odd = WeaponSpec::new(&WeaponRecord {
            guidance: 99,
            ammo_type: 5,
            reload: 40,
            ..weapon(150)
        });
        assert!(!odd.is_bay());
        let mut armament = armed(&[(odd, 1)]);
        armament.hold_idle_bays();
        assert_eq!(reloads(&armament), [40.0]);
    }

    #[test]
    fn an_emptied_bay_is_held_too() {
        // No rounds are tested: the armament holds none here.
        let mut armament = armed(&[(bay(149), 1)]);
        armament.hold_idle_bays();
        assert_eq!(reloads(&armament), [60.0]);
    }
}
