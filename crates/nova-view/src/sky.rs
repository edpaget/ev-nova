//! The layered sky in the draw list: where a [`DrawCommand::Sky`] looks
//! from and how bright it is ([`SkyPose`]).
//!
//! The sky is a field of generated stars in layers at depth behind the
//! play plane. The command carries only the pose: which sky (its seed),
//! where the eye is over the world, where the eye is drawn, and the
//! system's murk level. How many layers there are, at what depths, and
//! the stars' density, sizes and tints belong to the renderer
//! (`nova_render::sky`), as a planet's look belongs to its bake. The same
//! pose always draws the same sky.
//!
//! Flight draws one under [`RuleKey::LayeredStarfield`]'s other reading
//! (see `flight::sky`).
//!
//! [`DrawCommand::Sky`]: crate::DrawCommand::Sky
//! [`RuleKey::LayeredStarfield`]: nova_sim::RuleKey::LayeredStarfield

use crate::geometry::Point;

/// Where a [`DrawCommand::Sky`](crate::DrawCommand::Sky) looks from, and
/// how bright it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyPose {
    /// Which sky: each seed scatters its own stars, the same ones every
    /// time.
    pub seed: u64,
    /// The world point the camera centres on, in world units. It moves
    /// continuously with the ship, so a wrap of the play area doesn't jump
    /// the sky.
    pub eye: Point,
    /// The logical point the eye is drawn at.
    pub anchor: Point,
    /// How bright the sky is, out of 32 (the original's murk level, see
    /// [`crate::flight::stars::murk_level`]): 32 draws it at full
    /// brightness, 16 at half, and 0 draws nothing.
    pub level: u8,
}
