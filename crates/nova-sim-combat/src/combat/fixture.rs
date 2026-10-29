//! Test support for the combat layer's tests and the tests above it.
//! Behind the `fixture` feature, for other crates' tests: a plain strike
//! ([`strike`]). For this crate's tests alone: a fight's sides with no
//! governments (`among`).

use super::{Nominal, ShipRef, Strike};

/// No governments at all, for `among`.
#[cfg(test)]
static NO_GOVERNMENTS: crate::govt::Governments = crate::govt::Governments::empty();

/// The fight's sides as `targets` alone make them: no governments, and
/// the original's pass-through and beam planet-type bit.
#[cfg(test)]
#[must_use]
pub fn among(targets: &[super::projectile::Target]) -> super::sides::Sides<'_> {
    super::sides::Sides {
        targets,
        govts: &NO_GOVERNMENTS,
        pass_through: super::sides::PassThrough::default(),
        beam_planet_flag: super::planet_type::BeamPlanetFlag::default(),
    }
}

/// A direct, intended and answered strike on `ship` by `by`, of 12
/// damage taken and 12 passed both ways, weighing nothing and downing
/// nothing: what a test that only needs some answered hit builds on
/// (`..strike(ship, by)`).
///
/// Kept for nova-sim-ai's and nova-sim's tests: a plain answered strike,
/// built from public fields, pinning no rule.
#[must_use]
pub fn strike(ship: ShipRef, by: ShipRef) -> Strike {
    Strike {
        ship,
        by: Some(by),
        damage: 12.0,
        downed: None,
        direct: true,
        intended: true,
        lock: false,
        nominal: Nominal {
            mass: 12.0,
            energy: 12.0,
        },
        weight: 0.0,
        answered: true,
    }
}
