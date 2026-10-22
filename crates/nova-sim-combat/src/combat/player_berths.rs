//! The player's failed launch (`_FirePlayerWeapon` @0x62c61-0x62c72,
//! @0x62d82-0x62d87, @0x62f6d-0x62f96 in the `EV Nova` executable): a
//! copy of a fighter bay is attempted only once `_LaunchFighter` finds a
//! ship slot (@0x3d657-0x3d662).
//!
//! The session tells the fight how many ship berths are free below the
//! spawn reserve before the player's trigger
//! ([`Combat::set_player_berths`]), as the traffic's room ledger does for
//! the NPCs. Each copy of a bay the trigger fires takes one
//! ([`PlayerBerths::take`]); a copy finding none is not attempted: it
//! spends no round, lights no glow and isn't counted. With no copy
//! counted the call is heard nowhere, reloads nothing and steps no burst;
//! a volley some of whose copies launched reloads by those
//! ([`Armament::fire_player`](super::armament::Armament::fire_player)). A
//! fight the session hasn't told launches every copy.

use super::Combat;
use super::weapon::WeaponSpec;

/// The ship berths free for the player's fighters this tick: any number
/// when none was set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerBerths(Option<u32>);

impl PlayerBerths {
    /// Whether a copy of `spec` may be attempted: any weapon but a bay
    /// may, and a bay's copy takes a berth while one is free.
    pub fn take(&mut self, spec: &WeaponSpec) -> bool {
        if spec.carried.is_none() {
            return true;
        }
        match &mut self.0 {
            None => true,
            Some(0) => false,
            Some(free) => {
                *free -= 1;
                true
            }
        }
    }
}

impl Combat {
    /// The player's next trigger launches at most `free` fighters, the
    /// ship berths free below the spawn reserve (see the module docs).
    pub fn set_player_berths(&mut self, free: u32) {
        self.player_berths = PlayerBerths(Some(free));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use nova_sim_catalog::fixture::weapon;

    fn bay() -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance: 99,
            ammo_type: 144,
            ..weapon(149)
        })
    }

    #[test]
    fn each_bay_copy_takes_a_berth_until_none_is_left() {
        let mut berths = PlayerBerths(Some(2));
        assert_eq!(
            [
                berths.take(&bay()),
                berths.take(&bay()),
                berths.take(&bay())
            ],
            [true, true, false]
        );
        assert_eq!(berths, PlayerBerths(Some(0)));
    }

    #[test]
    fn any_other_weapon_takes_none_and_an_untold_fight_any_number() {
        let mut berths = PlayerBerths(Some(0));
        assert!(berths.take(&WeaponSpec::new(&weapon(128))));
        let mut untold = PlayerBerths::default();
        assert!((0..100).all(|_| untold.take(&bay())));
    }

    #[test]
    fn the_session_sets_the_berths() {
        let mut combat = Combat::default();
        combat.set_player_berths(3);
        assert_eq!(combat.player_berths, PlayerBerths(Some(3)));
    }
}
