//! Shots at stellars: which shots can hit a destroyable stellar, where
//! they meet it, and what they take off it, as the stellar branch of the
//! original's `_CheckShotProximities` (@0x371bc-0x37775) reads them (see
//! the `story` module's "Checked for phase 8").
//!
//! Only a shot of a planet-type weapon (`Flags2` 0x0400) can hit one,
//! and only once it has flown past its `ProxSafety`. A stellar is a
//! circle of the radius the session gives it: the original tests the
//! shot's sprite against the stellar's, pixel by pixel, and the engine
//! takes the disc the player sees instead. A hit takes `MassDmg` plus
//! `EnergyDmg` off its strength, with no shield and no kind of damage.
//! Beams never hit a stellar.

use super::ShipRef;
use super::projectile::{Shot, contact};
use super::weapon::WeaponSpec;
use crate::catalog::StellarId;
use crate::geometry::Vec2;

/// A stellar shots may hit this tick, its strength borrowed from the
/// session, which writes it back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StellarTarget {
    /// Which stellar.
    pub stellar: StellarId,
    /// Its centre.
    pub position: Vec2,
    /// Its hit radius.
    pub radius: f32,
    /// Its current strength: below 0 once shot down.
    pub strength: i64,
}

/// A shot's hit on a stellar, for the session to act on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StellarStrike {
    /// The stellar hit.
    pub stellar: StellarId,
    /// The ship whose shot it was ([`Shot::attacker`]): none for a shot
    /// no ship launched, or one turned on its firer, whose source slot
    /// `_CheckShotProximities` reads as no ship and convicts no one for
    /// (@0x374e1-0x374e7).
    pub by: Option<ShipRef>,
    /// The strength it took.
    pub damage: i64,
    /// Whether it shot the stellar down: its strength is below 0 now.
    pub destroyed: bool,
}

/// Whether a stellar with `strength` left after a shot's damage is shot
/// down: below 0, as `_StellarIsDestroyed` reads the current strength
/// (@0x4e14-0x4e3c, after the hit @0x37409). One left at exactly 0 is
/// still whole.
#[must_use]
pub fn shot_down(strength: i64) -> bool {
    strength < 0
}

/// What a hit of `weapon` takes off a stellar's strength: its mass and
/// energy damage together (@0x372dc-0x372eb).
#[must_use]
pub fn damage(weapon: &WeaponSpec) -> i64 {
    (weapon.mass_damage + weapon.energy_damage) as i64
}

/// Whether `shot` can hit a stellar: a planet-type weapon's (@0x37224),
/// flown past its proximity safety (@0x371fc-0x3721e).
#[must_use]
pub fn can_hit(shot: &Shot) -> bool {
    shot.weapon.planet_type() && shot.age > shot.weapon.prox_safety
}

/// The stellar among `stellars`, by its index, that `shot`, having flown
/// here from `was`, met first, and where: the nearest along its way of
/// those still standing; none when the shot cannot hit one.
#[must_use]
pub fn met(shot: &Shot, was: Vec2, stellars: &[StellarTarget]) -> Option<(usize, Vec2)> {
    if !can_hit(shot) {
        return None;
    }
    stellars
        .iter()
        .enumerate()
        .filter(|(_, stellar)| !shot_down(stellar.strength))
        .filter_map(|(index, stellar)| {
            Some((
                contact(was, shot.position, stellar.position, stellar.radius)?,
                index,
            ))
        })
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
        .map(|(t, index)| (index, was + (shot.position - was) * t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stellar_is_shot_down_below_none_and_not_at_none() {
        for (left, down) in [
            (-1, true),
            (-50, true),
            (0, false),
            (1, false),
            (100, false),
        ] {
            assert_eq!(shot_down(left), down, "{left}");
        }
    }
}
