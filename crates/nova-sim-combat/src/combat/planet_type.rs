//! The planet-type rules: who a planet-type weapon (`wëap` `Flags2`
//! 0x0400, [`PLANET_TYPE`](super::weapon::PLANET_TYPE)) may hit, and who
//! may hit a planet-type ship (`shïp` `Flags` 0x0400,
//! [`HullSpec::planet_type`](super::hull::HullSpec::planet_type)).
//!
//! - **Shots** (`_ShotCanHitShip` rule 8, @0x44a8e-0x44acc): a shot hits
//!   a ship only when the weapon's planet-type bit matches the ship's
//!   ([`shot_may_hit`]). A planet-type shot hits only planet-type ships,
//!   and an ordinary shot never hits a planet-type ship, the player's
//!   included. A shot no ship fired obeys it too (@0x44a71-0x44a88 jumps
//!   to it), and every caller of the rules reads it: the contact passes,
//!   the proximity fuse, a seeking sub-munition's pick and point defence's
//!   shots at ships.
//!
//! None of these rules plays a sound.

use super::weapon::WeaponSpec;

/// Whether a weapon's planet-type bit, `weapon`, matches a ship's,
/// `ship`: the original's `(class Flags & 0x0400) != (wëap bit & 0x0400)`
/// test, false on a mismatch.
#[must_use]
pub const fn matches(weapon: bool, ship: bool) -> bool {
    weapon == ship
}

/// Whether a shot of `weapon` may hit a ship whose class is planet-type
/// when `ship` holds, by `_ShotCanHitShip`'s rule 8 (@0x44a8e-0x44acc):
/// its `Flags2` 0x0400 matches the ship's `Flags` 0x0400.
#[must_use]
pub fn shot_may_hit(weapon: &WeaponSpec, ship: bool) -> bool {
    matches(weapon.planet_type(), ship)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::combat::weapon::PLANET_TYPE;
    use nova_sim_catalog::fixture::weapon;

    fn spec(flags2: u16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            flags2,
            ..weapon(128)
        })
    }

    #[test]
    fn the_bits_match_when_both_are_set_or_both_clear() {
        assert!(matches(true, true));
        assert!(matches(false, false));
        assert!(!matches(true, false));
        assert!(!matches(false, true));
    }

    #[test]
    fn a_planet_type_shot_may_hit_only_a_planet_type_ship_and_an_ordinary_one_only_an_ordinary_ship()
     {
        assert!(shot_may_hit(&spec(PLANET_TYPE), true));
        assert!(!shot_may_hit(&spec(PLANET_TYPE), false));
        assert!(shot_may_hit(&spec(0), false));
        assert!(!shot_may_hit(&spec(0), true));
        assert!(!shot_may_hit(&spec(!PLANET_TYPE), true), "the bit alone");
    }
}
