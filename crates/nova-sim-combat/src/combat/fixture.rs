//! Test support for the combat layer's tests and the tests above it,
//! behind the `fixture` feature: a fight's sides with no governments
//! ([`among`]).

use super::projectile::Target;
use super::sides::{PassThrough, Sides};
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
