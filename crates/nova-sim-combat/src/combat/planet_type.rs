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
//! - **Blasts** ([`blast_reaches`]), as the rulebook's
//!   [`RuleKey::PlanetBlasts`](crate::RuleKey::PlanetBlasts) says
//!   ([`PlanetBlasts`]). By the original, an expiry's blast (`_HandleShot`)
//!   and an asteroid hit's (`_HandleAsteroidHit`) blast no ship when the
//!   weapon is planet-type (@0x35fb9, @0x36734) and never reach a
//!   planet-type ship (@0x3602e, @0x367a8), while a shot's on-hit blast
//!   (`_HandleShipHit` @0x36d4b-0x36efb) tests neither bit and reaches
//!   every ship. By the Bible's planet-type weapon, which "can only hit
//!   planet-type ships or destroyable stellars", every blast reaches only
//!   the ships whose planet type matches its weapon's.
//!
//! None of these rules plays a sound.

use super::weapon::WeaponSpec;
use crate::rulebook::RuleSource;

/// Which blast a ship may be reached by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlastKind {
    /// A shot's blast as it hits a ship (`_HandleShipHit`).
    OnHit,
    /// A shot's detonation at the end of its life (`_HandleShot`).
    Expiry,
    /// A shot's blast as it hits an asteroid (`_HandleAsteroidHit`).
    AsteroidHit,
}

/// Which ships a blast reaches by their planet type, as the rulebook's
/// [`RuleKey::PlanetBlasts`](crate::RuleKey::PlanetBlasts) says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlanetBlasts {
    /// The original's: an expiry's or an asteroid hit's blast from a
    /// planet-type weapon reaches no ship, and neither reaches a
    /// planet-type ship; an on-hit blast reaches every ship.
    #[default]
    Original,
    /// The Bible's: every blast reaches only the ships whose planet type
    /// matches its weapon's.
    Matching,
}

impl From<RuleSource> for PlanetBlasts {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Original,
            RuleSource::Bible => Self::Matching,
        }
    }
}

/// Whether a blast of `kind` from a weapon whose planet-type bit is
/// `weapon` reaches a ship whose class's is `ship`, as `reading` says
/// (see the module docs).
#[must_use]
pub const fn blast_reaches(
    kind: BlastKind,
    reading: PlanetBlasts,
    weapon: bool,
    ship: bool,
) -> bool {
    match (reading, kind) {
        (PlanetBlasts::Original, BlastKind::OnHit) => true,
        (PlanetBlasts::Original, BlastKind::Expiry | BlastKind::AsteroidHit) => !weapon && !ship,
        (PlanetBlasts::Matching, _) => matches(weapon, ship),
    }
}

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

    /// Whether a blast of `kind` by `reading` reaches each of an
    /// ordinary and a planet-type ship, from an ordinary weapon and from
    /// a planet-type one.
    fn reached(kind: BlastKind, reading: PlanetBlasts) -> [[bool; 2]; 2] {
        [false, true]
            .map(|weapon| [false, true].map(|ship| blast_reaches(kind, reading, weapon, ship)))
    }

    #[test]
    fn by_the_original_an_on_hit_blast_reaches_every_ship() {
        assert_eq!(
            reached(BlastKind::OnHit, PlanetBlasts::Original),
            [[true, true], [true, true]]
        );
    }

    #[test]
    fn by_the_original_an_expiry_or_asteroid_blast_reaches_only_ordinary_ships_from_an_ordinary_weapon()
     {
        for kind in [BlastKind::Expiry, BlastKind::AsteroidHit] {
            assert_eq!(
                reached(kind, PlanetBlasts::Original),
                [[true, false], [false, false]],
                "{kind:?}"
            );
        }
    }

    #[test]
    fn by_the_bible_every_blast_reaches_the_ships_of_its_weapons_planet_type() {
        for kind in [BlastKind::OnHit, BlastKind::Expiry, BlastKind::AsteroidHit] {
            assert_eq!(
                reached(kind, PlanetBlasts::Matching),
                [[true, false], [false, true]],
                "{kind:?}"
            );
        }
    }

    #[test]
    fn the_engine_reads_the_original_blasts_and_the_bible_the_matching_ones() {
        assert_eq!(
            PlanetBlasts::from(RuleSource::Engine),
            PlanetBlasts::Original
        );
        assert_eq!(
            PlanetBlasts::from(RuleSource::Bible),
            PlanetBlasts::Matching
        );
        assert_eq!(PlanetBlasts::default(), PlanetBlasts::Original);
    }
}
