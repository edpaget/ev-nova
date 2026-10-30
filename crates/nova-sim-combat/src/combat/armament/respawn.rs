//! An escort's weapons as `_RespawnEscort` (@0x3d4b8-0x3d62c in the
//! `EV Nova` executable) leaves its ship-table slot when the player
//! enters a system: [`Armament::respawned`] and
//! [`Armament::restocked_from`].
//!
//! Each slot holds one entry for each weapon type (stride 0xc8 from
//! +0xc8): its count (+0xc8), its rounds (+0xd0) and its reload (+0xf8).
//! With its restock argument 1 (a take-off, a pilot flown) the function
//! writes each entry's count from the class's +0x5e8 table and its rounds
//! from +0x7e8 (@0x3d5a7-0x3d600); with 0 (a jump's arrival, a gate's
//! exit) it writes neither. It never writes an entry's reload, nor the
//! exit cursors (+0xc8fe-+0xc905) or the fold (+0xc8fa, +0xc8fc), so those
//! are the slot's in either case. It always writes the last gun its pick
//! fired, +0xc8da, to -1 (@0x3d522).

use super::Armament;

impl Armament {
    /// This armament as a respawn with no restock leaves it: every
    /// weapon, its count, reload and burst kept, and the exit cursors and
    /// fold, with its last gun cleared (+0xc8da = -1, @0x3d522).
    #[must_use]
    pub fn respawned(mut self) -> Self {
        self.last_gun = None;
        self
    }

    /// `stock`, a class's stock armament, as a respawn with a restock
    /// leaves a slot that held this armament: `stock`'s weapons and
    /// counts, each with the reload and burst this one's mount of the same
    /// weapon held (none for a weapon it didn't carry), and this one's exit
    /// cursors and fold, with no last gun.
    #[must_use]
    pub fn restocked_from(&self, stock: Armament) -> Self {
        let mut restocked = stock;
        for mount in &mut restocked.mounts {
            if let Some(held) = self
                .mounts
                .iter()
                .find(|held| held.spec.id == mount.spec.id)
            {
                mount.reload = held.reload;
                mount.burst = held.burst;
            }
        }
        restocked.exits = self.exits;
        restocked.fold = self.fold;
        restocked.last_gun = None;
        restocked
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::{Armament, Fold};
    use crate::catalog::{WeaponId, WeaponRecord};
    use crate::combat::exits::ExitKind;
    use crate::combat::weapon::WeaponSpec;
    use nova_sim_catalog::fixture::weapon;

    /// Weapon `id` of `Reload` `reload`.
    fn spec(id: i16, reload: i16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            reload,
            ..weapon(id)
        })
    }

    /// Two guns, 128 of reload 10 and 129 of reload 20, the first reloading,
    /// a cursor moved, folded and with a last gun.
    fn worn() -> Armament {
        let mut armament = Armament::new([(spec(128, 10), 2), (spec(129, 20), 1)]);
        armament.hold(WeaponId(128));
        armament.mounts[0].burst = 3;
        armament.exits.step(ExitKind::Turret);
        armament.fold = Fold {
            set: 2,
            dir: 1,
            since_fire: 7,
        };
        armament.last_gun = Some(WeaponId(129));
        armament
    }

    #[test]
    fn respawned_keeps_every_weapon_and_clears_the_last_gun() {
        let worn = worn();
        let respawned = worn.clone().respawned();
        assert_eq!(respawned.mounts, worn.mounts);
        assert_eq!(respawned.exits, worn.exits);
        assert_eq!(respawned.fold, worn.fold);
        assert_eq!(respawned.last_gun(), None, "+0xc8da = -1");
    }

    #[test]
    fn restocked_takes_the_stocks_counts_and_keeps_each_weapons_reload() {
        let worn = worn();
        let stock = Armament::new([(spec(128, 10), 3), (spec(130, 5), 1)]);
        let restocked = worn.restocked_from(stock);
        let mounts: Vec<_> = restocked
            .mounts()
            .iter()
            .map(|mount| (mount.spec.id.0, mount.count, mount.reload, mount.burst))
            .collect();
        assert_eq!(
            mounts,
            [(128, 3, 10.0, 3), (130, 1, 0.0, 0)],
            "the stock's counts, 128's reload and burst kept, 130 ready"
        );
        assert_eq!(restocked.exits, worn.exits, "the slot's cursors");
        assert_eq!(restocked.fold, worn.fold, "the slot's fold");
        assert_eq!(restocked.last_gun(), None);
    }
}
