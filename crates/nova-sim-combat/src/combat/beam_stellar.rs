//! Beams on stellars, by the Bible's reading of the rulebook's
//! [`RuleKey::BeamStellars`](crate::RuleKey::BeamStellars).
//!
//! The original's beams never touch a stellar: `_HandleBeams`
//! (@0x30295-0x31e1f) reads neither the stellar table (0x3b5010) nor
//! `_StellarIsDestroyed`, so by the engine's reading, the default, nothing
//! here runs. The Bible's planet-type weapon "can only hit planet-type
//! ships or destroyable stellars", which reads as a planet-type beam
//! (`Flags2` 0x0400) hitting a stellar too. By that reading a planet-type
//! beam, but a point-defence one, is stopped by the first stellar still
//! standing that its segment meets ([`met`]), when the stellar is nearer
//! along it than the ship it hits. Each tick it is held there it takes
//! its weapon's `MassDmg` plus `EnergyDmg`, times the pass's beam factor
//! ([`damage`]), off the stellar's strength, explodes where it meets it
//! with its `ExplodType`, and records a
//! [`StellarStrike`](super::stellar::StellarStrike) by its firer; a stellar
//! taken below 0 is shot down as by a shot, and the tick's later beams
//! pass it. No sound is raised: the original has none to copy.

use super::beam::{Aiming, Beam};
use super::projectile::contact;
use super::stellar::{self, StellarStrike, StellarTarget};
use super::weapon::WeaponSpec;
use super::{Combat, CombatEvent};
use crate::geometry::Vec2;

/// The stellar among `stellars`, by its index, that `beam`'s segment, from
/// its start to its full length, meets first, and where, when the beam is
/// planet-type, not point defence's, and the stellar is nearer along it
/// than `ship_at`, where it hits a ship, if it does. Shot-down stellars are
/// passed.
#[must_use]
pub fn met(
    beam: &Beam,
    ship_at: Option<Vec2>,
    stellars: &[StellarTarget],
) -> Option<(usize, Vec2)> {
    if !beam.weapon.planet_type() || matches!(beam.aiming, Aiming::Shot(_)) {
        return None;
    }
    let (start, end) = (beam.start, beam.end);
    let (t, index) = stellars
        .iter()
        .enumerate()
        .filter(|(_, target)| !stellar::shot_down(target.strength))
        .filter_map(|(index, target)| {
            Some((contact(start, end, target.position, target.radius)?, index))
        })
        .min_by(|(a, _), (b, _)| a.total_cmp(b))?;
    let at = start + (end - start) * t;
    let nearer = ship_at.is_none_or(|ship| (at - start).length() < (ship - start).length());
    nearer.then_some((index, at))
}

/// What a beam of `weapon` takes off a stellar's strength in a tick: its
/// mass and energy damage together, times the pass's beam `factor`,
/// truncated.
#[must_use]
pub fn damage(weapon: &WeaponSpec, factor: f32) -> i64 {
    // Within an `i64` for any damage an `i16` gives.
    #[allow(clippy::cast_possible_truncation)]
    {
        ((weapon.mass_damage + weapon.energy_damage) * factor).trunc() as i64
    }
}

impl Combat {
    /// Whether beam `index`, which hits the ship at `ship_at` if any, is
    /// stopped by a stellar among `stellars` (see the module docs), by the
    /// pass's beam `factor`: if so, the beam ends there, explodes, and
    /// takes its damage off the stellar.
    pub(super) fn beam_strikes_stellar(
        &mut self,
        index: usize,
        ship_at: Option<Vec2>,
        stellars: &mut [StellarTarget],
        factor: f32,
    ) -> bool {
        let beam = &mut self.beams[index];
        let Some((stellar, at)) = met(beam, ship_at, stellars) else {
            return false;
        };
        beam.end = at;
        let beam = *beam;
        if let Some(explosion) = beam.weapon.explosion {
            self.events.push(CombatEvent::Exploded {
                at,
                explosion,
                size: beam.weapon.blast_radius,
            });
        }
        let target = &mut stellars[stellar];
        let damage = damage(&beam.weapon, factor);
        target.strength -= damage;
        self.stellar_strikes.push(StellarStrike {
            stellar: target.stellar,
            by: Some(beam.firer),
            damage,
            destroyed: stellar::shot_down(target.strength),
        });
        true
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{StellarId, WeaponRecord};
    use crate::combat::ShipRef;
    use crate::combat::weapon::PLANET_TYPE;
    use crate::flight::ShipState;
    use nova_sim_catalog::fixture::weapon;

    /// A beam of `flags2`, 200 pixels long, from the centre to the right.
    fn beam(flags2: u16, aiming: Aiming) -> Beam {
        let record = WeaponRecord {
            guidance: 0,
            beam_length: 200,
            flags2,
            ..weapon(146)
        };
        let from = ShipState {
            heading: 90.0,
            ..ShipState::default()
        };
        Beam::launch(
            WeaponSpec::new(&record),
            ShipRef::Player,
            ShipRef::Player,
            &from,
            aiming,
        )
    }

    fn stellar(id: i16, x: f32, strength: i64) -> StellarTarget {
        StellarTarget {
            stellar: StellarId(id),
            position: Vec2::new(x, 0.0),
            radius: 10.0,
            strength,
        }
    }

    /// Whether `met` is stellar `index` met at (`x`, 0).
    fn met_at(met: Option<(usize, Vec2)>, index: usize, x: f32) -> bool {
        met.is_some_and(|(i, at)| i == index && (at - Vec2::new(x, 0.0)).length() < 1e-3)
    }

    #[test]
    fn a_planet_type_beam_meets_the_nearest_stellar_standing_along_it() {
        let planet = beam(PLANET_TYPE, Aiming::Ahead);
        let stellars = [stellar(128, 150.0, 5), stellar(129, 100.0, 5)];
        assert!(met_at(met(&planet, None, &stellars), 1, 90.0));
        let down = [stellar(128, 150.0, 5), stellar(129, 100.0, -1)];
        assert!(
            met_at(met(&planet, None, &down), 0, 140.0),
            "past one shot down"
        );
        let whole_at_none = [stellar(129, 100.0, 0)];
        assert_eq!(met(&planet, None, &whole_at_none).map(|m| m.0), Some(0));
        assert_eq!(
            met(&planet, None, &[stellar(128, 300.0, 5)]),
            None,
            "out of reach"
        );
    }

    #[test]
    fn only_a_planet_type_beam_not_point_defences_meets_a_stellar() {
        let stellars = [stellar(128, 100.0, 5)];
        assert_eq!(met(&beam(0, Aiming::Ahead), None, &stellars), None);
        let defence = beam(
            PLANET_TYPE,
            Aiming::Shot(crate::combat::projectile::ShotId(1)),
        );
        assert_eq!(met(&defence, None, &stellars), None);
    }

    #[test]
    fn a_ship_strictly_nearer_along_the_beam_takes_it_instead() {
        let planet = beam(PLANET_TYPE, Aiming::Ahead);
        let stellars = [stellar(128, 100.0, 5)];
        assert_eq!(met(&planet, Some(Vec2::new(89.0, 0.0)), &stellars), None);
        assert_eq!(
            met(&planet, Some(Vec2::new(90.0, 0.0)), &stellars),
            None,
            "a tie goes to the ship"
        );
        assert_eq!(
            met(&planet, Some(Vec2::new(91.0, 0.0)), &stellars).map(|m| m.0),
            Some(0)
        );
    }

    #[test]
    fn a_beam_off_the_centre_measures_from_its_own_start() {
        // From (100, 0) to the left, to (-100, 0): 128 at the centre is
        // met at (10, 0), 90 along.
        let from = ShipState {
            position: Vec2::new(100.0, 0.0),
            heading: 270.0,
            ..ShipState::default()
        };
        let planet = Beam {
            end: Vec2::new(-100.0, 0.0),
            ..Beam::launch(
                beam(PLANET_TYPE, Aiming::Ahead).weapon,
                ShipRef::Player,
                ShipRef::Player,
                &from,
                Aiming::Ahead,
            )
        };
        assert_eq!(planet.start, Vec2::new(100.0, 0.0));
        let stellars = [stellar(128, 0.0, 5)];
        let met_at = met(&planet, None, &stellars).expect("met");
        assert!(
            (met_at.1 - Vec2::new(10.0, 0.0)).length() < 1e-3,
            "{met_at:?}"
        );
        assert_eq!(
            met(&planet, Some(Vec2::new(20.0, 0.0)), &stellars),
            None,
            "a ship 80 along is nearer"
        );
        assert_eq!(
            met(&planet, Some(Vec2::new(5.0, 0.0)), &stellars).map(|m| m.0),
            Some(0),
            "one 95 along is not"
        );
    }

    #[test]
    fn a_beam_takes_its_mass_and_energy_times_the_factor_truncated() {
        let record = WeaponRecord {
            mass_dmg: 2,
            energy_dmg: 5,
            ..weapon(146)
        };
        let spec = WeaponSpec::new(&record);
        assert_eq!(damage(&spec, 1.0), 7);
        assert_eq!(damage(&spec, 0.5), 3, "3.5 truncated");
    }
}
