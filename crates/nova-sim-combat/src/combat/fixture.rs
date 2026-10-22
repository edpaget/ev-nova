//! Test support for the combat layer's tests and the tests above it,
//! behind the `fixture` feature: a fight's sides with no governments
//! ([`among`]), and a plain strike ([`strike`]).

use super::projectile::Target;
use super::sides::{PassThrough, Sides};
use super::{Nominal, ShipRef, Strike};
use crate::govt::Governments;

/// No governments at all, for [`among`].
static NO_GOVERNMENTS: Governments = Governments::empty();

/// The fight's sides as `targets` alone make them: no governments, and
/// the original's pass-through.
#[must_use]
pub fn among(targets: &[Target]) -> Sides<'_> {
    Sides {
        targets,
        govts: &NO_GOVERNMENTS,
        pass_through: PassThrough::default(),
    }
}

/// A direct, intended and answered strike on `ship` by `by`, of 12
/// damage taken and 12 passed both ways, weighing nothing and downing
/// nothing: what a test that only needs some answered hit builds on
/// (`..strike(ship, by)`).
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
