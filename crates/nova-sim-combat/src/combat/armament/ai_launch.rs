//! `_AILaunchFighter` (@0x81372-0x81511 in the `EV Nova` executable): an
//! NPC launching a fighter from its bay ([`Armament::launch_bay`]).

use super::{Armament, Launch, Rounds, inaccuracy};
use crate::chance::Chance;
use crate::combat::aim::Aim;
use crate::combat::hull::Condition;
use crate::combat::report::Reports;
use crate::combat::weapon::{EXCLUSIVE, WeaponSpec};

impl Armament {
    /// Launches a fighter from the bay an NPC in `condition` launches
    /// from now, as `_AILaunchFighter` does once `_LaunchFighter` has
    /// found the fighter a slot: what it launched, aimed as `aim` says,
    /// its inaccuracy drawn on `chance`; none when nothing launched. The
    /// fight turns it into a sortie, sounding the bay
    /// ([`BAY_LAUNCH_PRIORITY`](crate::combat::BAY_LAUNCH_PRIORITY)).
    ///
    /// 1. Nothing launches unless the ship is [`Condition::Intact`] (the
    ///    high level's `_IsDisabled` gate, @0x8e6eb-0x8e6f2).
    /// 2. The bay is [`Armament::bay_to_launch`]'s: the one of lowest
    ///    weapon ID holding rounds of its own that AI ships may use
    ///    (@0x813c8-0x813f7), once it has reloaded; while it reloads no
    ///    later bay is tried (@0x81400-0x8140b). It reads its own rounds
    ///    alone, asking no `_WeaponHasAmmo`: no cloak, key ship or fuel
    ///    test.
    /// 3. It spends one round of the bay's own slot (@0x81481) and sets
    ///    the bay's reload to `Reload` over the bays carried
    ///    (@0x81473-0x81491), with no burst (no `BurstCount` is read).
    /// 4. An exclusive bay (`Flags3` [`EXCLUSIVE`], @0x8149f) holds every
    ///    other weapon's reload at its own plus
    ///    [`EXCLUSIVE_HOLD`](crate::combat::weapon::EXCLUSIVE_HOLD), where
    ///    that is higher (@0x814a6-0x814e1).
    ///
    /// Each flag it carries that the simulation ignores is reported to
    /// `reports`, as for any weapon fired.
    pub fn launch_bay(
        &mut self,
        condition: Condition,
        rounds: &mut dyn Rounds,
        chance: &mut dyn Chance,
        reports: &mut Reports,
        aim: &mut dyn FnMut(&WeaponSpec) -> Option<Aim>,
    ) -> Option<Launch> {
        if condition != Condition::Intact {
            return None;
        }
        let bay = self.bay_to_launch(rounds)?;
        let index = self.mounts.iter().position(|mount| mount.spec.id == bay)?;
        let mount = &mut self.mounts[index];
        let aimed = aim(&mount.spec)?;
        let spec = mount.spec;
        reports.fired(&spec);
        let inaccuracy = inaccuracy(spec.inaccuracy, chance);
        rounds.spend(bay);
        let held = spec.reload / mount.count as f32;
        mount.reload = held;
        if spec.flags3 & EXCLUSIVE != 0 {
            self.hold_others(index, held);
        }
        Some(Launch {
            weapon: spec,
            pointing: aimed.pointing,
            inaccuracy,
            target: aimed.target,
        })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{WeaponId, WeaponRecord};
    use crate::chance::NeverFires;
    use crate::combat::NpcId;
    use crate::combat::ShipRef;
    use crate::combat::armament::LauncherRounds;
    use crate::combat::weapon::{AI_WONT_USE, NEEDS_KEY_SHIP};
    use nova_sim_catalog::fixture::weapon;

    /// A fighter bay `id` of ship 144, reloading every `reload` ticks.
    fn bay(id: i16, reload: i16) -> WeaponRecord {
        WeaponRecord {
            guidance: 99,
            ammo_type: 144,
            flags: 0x0002,
            reload,
            ..weapon(id)
        }
    }

    /// An armament of `weapons`, each with its count.
    fn armed(weapons: &[(WeaponRecord, u32)]) -> Armament {
        Armament::new(
            weapons
                .iter()
                .map(|(record, count)| (WeaponSpec::new(record), *count)),
        )
    }

    const TARGET: ShipRef = ShipRef::Npc(NpcId(7));

    /// Launches from `armament` on an intact ship, aimed along 90 at
    /// [`TARGET`].
    fn launch(armament: &mut Armament, rounds: &mut LauncherRounds) -> Option<Launch> {
        launch_in(armament, rounds, Condition::Intact)
    }

    fn launch_in(
        armament: &mut Armament,
        rounds: &mut LauncherRounds,
        condition: Condition,
    ) -> Option<Launch> {
        armament.launch_bay(
            condition,
            rounds,
            &mut NeverFires,
            &mut Reports::default(),
            &mut |_| Some(Aim::along(90.0, Some(TARGET))),
        )
    }

    fn reloads(armament: &Armament) -> Vec<f32> {
        armament.mounts().iter().map(|mount| mount.reload).collect()
    }

    #[test]
    fn ai_launch_spends_a_round_and_reloads_reload_over_the_bays() {
        let inaccurate = WeaponRecord {
            inaccuracy: 10,
            ..bay(149, 60)
        };
        let mut armament = armed(&[(inaccurate, 2)]);
        let mut rounds = LauncherRounds::from([(WeaponId(149), 3)]);
        let launch = launch(&mut armament, &mut rounds).expect("launches");
        assert_eq!(launch.weapon.id, WeaponId(149));
        assert_eq!(launch.pointing, Aim::along(90.0, None).pointing);
        assert_eq!(launch.target, Some(TARGET));
        assert_eq!(launch.inaccuracy, 9.0, "19 of 20, less 10");
        assert_eq!(rounds.slot(WeaponId(149)), 2, "one round (@0x81481)");
        assert_eq!(reloads(&armament), [30.0], "60 over 2 (@0x81473-0x81491)");
    }

    #[test]
    fn ai_launch_skips_a_bay_ai_ships_wont_use() {
        // `Flags2` 0x0100 (@0x813f3-0x813f7) passes the bay over for the
        // next.
        let wont = WeaponRecord {
            flags2: AI_WONT_USE,
            ..bay(149, 60)
        };
        let mut armament = armed(&[(bay(150, 70), 1), (wont, 1)]);
        let mut rounds = LauncherRounds::from([(WeaponId(149), 2), (WeaponId(150), 2)]);
        assert_eq!(armament.bay_to_launch(&rounds), Some(WeaponId(150)));
        let launch = launch(&mut armament, &mut rounds).expect("launches");
        assert_eq!(launch.weapon.id, WeaponId(150));
        assert_eq!(rounds.slot(WeaponId(149)), 2);
        let mut alone = armed(&[(wont, 1)]);
        let mut rounds = LauncherRounds::from([(WeaponId(149), 2)]);
        assert_eq!(launch_in(&mut alone, &mut rounds, Condition::Intact), None);
        assert_eq!(rounds.slot(WeaponId(149)), 2, "nothing spent");
        assert_eq!(reloads(&alone), [0.0]);
    }

    #[test]
    fn ai_launch_waits_while_its_first_bay_reloads_and_tries_no_later_bay() {
        // @0x81400-0x8140b returns, where a later bay is ready.
        let mut armament = armed(&[(bay(150, 70), 1), (bay(149, 60), 1)]);
        let mut rounds = LauncherRounds::from([(WeaponId(149), 2), (WeaponId(150), 2)]);
        armament.hold(WeaponId(149));
        assert_eq!(launch(&mut armament, &mut rounds), None);
        assert_eq!(
            rounds,
            LauncherRounds::from([(WeaponId(149), 2), (WeaponId(150), 2)])
        );
        assert_eq!(reloads(&armament), [0.0, 60.0]);
        // Empty, it is passed over.
        let mut rounds = LauncherRounds::from([(WeaponId(150), 2)]);
        let launch = launch(&mut armament, &mut rounds).expect("launches");
        assert_eq!(launch.weapon.id, WeaponId(150));
    }

    #[test]
    fn ai_launch_of_an_exclusive_bay_holds_every_other_weapon_at_its_reload_plus_2() {
        // @0x8149f-0x814e1: where the bay's reload + 2.0 is higher.
        let exclusive = WeaponRecord {
            flags3: EXCLUSIVE,
            ..bay(149, 60)
        };
        let mut armament = armed(&[(weapon(128), 1), (exclusive, 2), (weapon(129), 1)]);
        armament.mounts[2].reload = 70.0;
        let mut rounds = LauncherRounds::from([(WeaponId(149), 2)]);
        assert!(launch(&mut armament, &mut rounds).is_some());
        assert_eq!(reloads(&armament), [32.0, 30.0, 70.0]);
        let mut plain = armed(&[(weapon(128), 1), (bay(149, 60), 1)]);
        assert!(launch(&mut plain, &mut rounds).is_some());
        assert_eq!(reloads(&plain), [0.0, 60.0], "not exclusive");
    }

    #[test]
    fn ai_launch_counts_no_burst() {
        let bursting = WeaponRecord {
            burst_count: 2,
            burst_reload: 100,
            ..bay(149, 60)
        };
        let mut armament = armed(&[(bursting, 1)]);
        let mut rounds = LauncherRounds::from([(WeaponId(149), 3)]);
        for _ in 0..2 {
            assert!(launch(&mut armament, &mut rounds).is_some());
            assert_eq!(reloads(&armament), [60.0], "Reload, never BurstReload");
            assert_eq!(armament.mounts()[0].burst, 0);
            armament.mounts[0].reload = 0.0;
        }
    }

    #[test]
    fn ai_launch_reads_its_rounds_alone() {
        // `_AILaunchFighter` never asks `_WeaponHasAmmo` (@0x813cc-0x813e8):
        // a bay needing a key ship launches with none aboard.
        let keyed = WeaponRecord {
            flags2: NEEDS_KEY_SHIP,
            ..bay(149, 60)
        };
        let mut armament = armed(&[(keyed, 1)]);
        let mut rounds = LauncherRounds::from([(WeaponId(149), 1)]);
        assert!(launch(&mut armament, &mut rounds).is_some());
        assert_eq!(rounds.slot(WeaponId(149)), 0);
        assert_eq!(launch(&mut armament, &mut rounds), None, "no rounds");
    }

    #[test]
    fn a_ship_not_intact_launches_nothing() {
        for condition in [Condition::Disabled, Condition::Destroyed] {
            let mut armament = armed(&[(bay(149, 60), 1)]);
            let mut rounds = LauncherRounds::from([(WeaponId(149), 2)]);
            assert_eq!(launch_in(&mut armament, &mut rounds, condition), None);
            assert_eq!(rounds.slot(WeaponId(149)), 2, "{condition:?}");
            assert_eq!(reloads(&armament), [0.0]);
        }
    }
}
