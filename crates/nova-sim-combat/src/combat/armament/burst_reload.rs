//! `_AIForceBurstReload` (@0x84c37-0x84cb7 in the `EV Nova` executable):
//! a ship that cloaks when hit starts every burst weapon of its class on
//! its burst reload ([`Armament::force_burst_reload`]).

use super::Armament;

impl Armament {
    /// Starts each mount whose weapon `class`, the armament of the ship's
    /// class ([`Arsenal::of_class`](super::Arsenal::of_class)), carries,
    /// and that bursts (`BurstCount` and `BurstReload` both above 0), on
    /// its burst reload: its burst counted back to none and its timer at
    /// its `BurstReload`.
    ///
    /// The original loops over every weapon index and reads the count in
    /// the ship's class record (`shïp` +0x5e8[i] > 0, @0x84c5f), not the
    /// ship's own count, so a weapon a person adds beyond its class is
    /// left alone. It sets the ship's burst counter (+0x17c + i·0xc8) to 0
    /// and its reload timer (+0xf8 + i·0xc8) to `BurstReload` as a float
    /// (@0x84c85-0x84c9c). It plays no sound.
    pub fn force_burst_reload(&mut self, class: &Armament) {
        for mount in &mut self.mounts {
            let spec = mount.spec;
            let carried = class.mounts.iter().any(|own| own.spec.id == spec.id);
            if carried && spec.burst_count > 0 && spec.burst_reload > 0.0 {
                mount.burst = 0;
                mount.reload = spec.burst_reload;
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

    /// Weapon `id` of `BurstCount` `count` and `BurstReload` `reload`.
    fn bursting(id: i16, count: i16, reload: i16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            reload: 3,
            burst_count: count,
            burst_reload: reload,
            ..weapon(id)
        })
    }

    /// An armament of `weapons`, one of each, each mid-burst: 2 shots in,
    /// 1.5 ticks of its reload left.
    fn mid_burst(weapons: &[WeaponSpec]) -> Armament {
        let mut armament = Armament::new(weapons.iter().map(|&spec| (spec, 1)));
        for mount in &mut armament.mounts {
            mount.burst = 2;
            mount.reload = 1.5;
        }
        armament
    }

    /// Each mount's burst and reload.
    fn bursts(armament: &Armament) -> Vec<(u32, f32)> {
        armament
            .mounts()
            .iter()
            .map(|mount| (mount.burst, mount.reload))
            .collect()
    }

    #[test]
    fn a_burst_weapon_of_its_class_starts_on_its_burst_reload() {
        let weapons = [bursting(130, 15, 40)];
        let mut armament = mid_burst(&weapons);
        armament.force_burst_reload(&Armament::new([(weapons[0], 2)]));
        assert_eq!(bursts(&armament), [(0, 40.0)]);
    }

    #[test]
    fn a_weapon_with_no_burst_or_no_burst_reload_is_left_alone() {
        let weapons = [bursting(130, 0, 40), bursting(131, 15, 0)];
        let mut armament = mid_burst(&weapons);
        armament.force_burst_reload(&Armament::new(weapons.map(|spec| (spec, 1))));
        assert_eq!(bursts(&armament), [(2, 1.5), (2, 1.5)]);
    }

    #[test]
    fn a_weapon_its_class_does_not_carry_is_left_alone() {
        // `shïp` +0x5e8[i] > 0 (@0x84c5f): the class's count, not the
        // ship's own.
        let weapons = [bursting(130, 15, 40), bursting(131, 4, 20)];
        let mut armament = mid_burst(&weapons);
        armament.force_burst_reload(&Armament::new([(weapons[1], 1)]));
        assert_eq!(bursts(&armament), [(2, 1.5), (0, 20.0)]);
    }
}
