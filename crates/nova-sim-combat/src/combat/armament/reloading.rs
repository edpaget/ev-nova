//! `_AIHandleCloaking`'s `Flags2` 0x0100 term (@0x8345d-0x834b3 in the
//! `EV Nova` executable): whether a weapon the ship carries reloads past
//! its `Reload` ([`Armament::reloading`]), which
//! [`cloak::think`](crate::cloak::think) reads as
//! [`Situation::reloading`](crate::cloak::Situation::reloading).

use super::Armament;
use crate::rulebook::RuleSource;

impl Armament {
    /// Whether one of the weapons the ship carries is reloading, as the
    /// `cloak_on_reload` rule
    /// ([`RuleKey::CloakOnReload`](crate::rulebook::RuleKey::CloakOnReload))
    /// reads it by `source`.
    ///
    /// By the engine, as the original loops over every weapon index
    /// (stride 0xc8 from +0xc8): a weapon carried (count above 0) whose
    /// reload timer (+0xf8, a float) is strictly above its `Reload` (the
    /// weapon table's float at +0x30, as `_FireAIShipWeapon` reads it
    /// @0x88f8b/@0x88f96). Only a burst reload longer than `Reload` (or
    /// a rating's stretch of the timer) gets there.
    ///
    /// By the Bible's "cloak when their weapon goes into burst reload": a
    /// weapon carried that bursts (`BurstCount` and `BurstReload` above 0)
    /// with its burst spent and its timer still running.
    #[must_use]
    pub fn reloading(&self, source: RuleSource) -> bool {
        self.mounts
            .iter()
            .filter(|mount| mount.count > 0)
            .any(|mount| {
                let spec = mount.spec;
                match source {
                    RuleSource::Engine => mount.reload > spec.reload,
                    RuleSource::Bible => {
                        spec.burst_count > 0
                            && spec.burst_reload > 0.0
                            && mount.burst == 0
                            && mount.reload > 0.0
                    }
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::combat::weapon::WeaponSpec;
    use nova_sim_catalog::fixture::weapon;

    /// Weapon `id` of `Reload` `reload`, `BurstCount` `count` and
    /// `BurstReload` `burst_reload`.
    fn spec(id: i16, reload: i16, count: i16, burst_reload: i16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            reload,
            burst_count: count,
            burst_reload,
            ..weapon(id)
        })
    }

    /// An armament of one `spec`, put on its burst reload.
    fn on_burst_reload(spec: WeaponSpec) -> Armament {
        let mut armament = Armament::new([(spec, 1)]);
        let class = armament.clone();
        armament.force_burst_reload(&class);
        armament
    }

    #[test]
    fn a_burst_reload_no_longer_than_reload_cloaks_only_by_the_bible() {
        // The pirates' Pulse Laser, 146: `Reload`, `BurstCount` and
        // `BurstReload` all 15.
        let pulse = on_burst_reload(spec(146, 15, 15, 15));
        assert!(!pulse.reloading(RuleSource::Engine));
        assert!(pulse.reloading(RuleSource::Bible));
    }

    #[test]
    fn a_burst_reload_longer_than_reload_cloaks_by_both() {
        // A Wraith's 165: `BurstCount` 1, `BurstReload` 45.
        let wraith = on_burst_reload(spec(165, 15, 1, 45));
        assert!(wraith.reloading(RuleSource::Engine));
        assert!(wraith.reloading(RuleSource::Bible));
    }

    #[test]
    fn a_ready_armament_is_not_reloading_by_either() {
        let ready = Armament::new([(spec(165, 15, 1, 45), 1)]);
        for source in RuleSource::ALL {
            assert!(!ready.reloading(source), "{source:?}");
        }
        assert!(!Armament::new([]).reloading(RuleSource::Engine));
    }

    #[test]
    fn a_mount_carried_no_more_is_ignored() {
        for source in RuleSource::ALL {
            let mut gone = on_burst_reload(spec(165, 15, 1, 45));
            gone.mounts[0].count = 0;
            assert!(!gone.reloading(source), "{source:?}");
        }
    }

    #[test]
    fn the_engine_tests_the_timer_strictly_above_reload() {
        let mut armament = Armament::new([(spec(146, 15, 0, 0), 1)]);
        let reload = armament.mounts[0].spec.reload;
        armament.mounts[0].reload = reload;
        assert!(!armament.reloading(RuleSource::Engine), "equal");
        armament.mounts[0].reload = reload + 0.01;
        assert!(armament.reloading(RuleSource::Engine), "above");
    }

    #[test]
    fn the_bible_reads_only_a_spent_burst_of_a_bursting_weapon() {
        let bursting = spec(165, 15, 3, 45);
        let mut mid_burst = Armament::new([(bursting, 1)]);
        mid_burst.mounts[0].burst = 1;
        mid_burst.mounts[0].reload = 10.0;
        assert!(!mid_burst.reloading(RuleSource::Bible), "burst under way");
        let mut done = on_burst_reload(bursting);
        done.mounts[0].reload = 0.0;
        assert!(!done.reloading(RuleSource::Bible), "reload run out");
        for plain in [spec(146, 15, 0, 45), spec(146, 15, 3, 0)] {
            let mut armament = Armament::new([(plain, 1)]);
            armament.mounts[0].reload = 60.0;
            assert!(!armament.reloading(RuleSource::Bible), "{plain:?}");
        }
    }
}
