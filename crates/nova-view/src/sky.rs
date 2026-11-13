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
//! A system's own sky look (its star density, its tint and its distant
//! backdrop, from its data) is not on the pose either: the renderer looks
//! it up by the seed (`nova_render::sky`), as it looks a planet up by its
//! key, and a seed it holds no look for draws the default sky. Flight's
//! seed rule is [`crate::flight::sky::sky_seed`].
//!
//! Through a jump's warp the pose also carries the rush
//! ([`SkyPose::rush`]): how far the eye moved over the last step. The
//! renderer draws each star as the streak it swept on screen over that
//! step, the rush times its layer's depth parallax, so near layers streak
//! long and far ones barely. Out of the warp the rush is zero and every
//! star is round.
//!
//! It carries a clock too ([`SkyPose::time`]), for what the renderer
//! turns over time in a sky (a decoration's slow spin). Flight sets it
//! from the elapsed time its planets turn by, so the same pose still
//! draws the same sky.
//!
//! Flight draws one under [`RuleKey::LayeredStarfield`]'s other reading
//! (see [`crate::flight::sky`]).
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
    /// How far the eye moved over the last step, in world units, while
    /// the sky rushes (a jump's warp); zero otherwise. The renderer
    /// streaks each star back along it by its layer's parallax (see the
    /// module docs).
    pub rush: Point,
    /// How long the sky has been watched, in seconds: the clock whatever
    /// in it turns over time is turned by (a spinning decoration, in the
    /// renderer). Flight gives its elapsed time, the clock its planets
    /// turn by; zero otherwise.
    pub time: f64,
}

impl SkyPose {
    /// Sky `seed` seen from over world point `eye`, drawn at `anchor`, at
    /// murk level `level`, not rushing, at time zero.
    #[must_use]
    pub const fn new(seed: u64, eye: Point, anchor: Point, level: u8) -> Self {
        Self {
            seed,
            eye,
            anchor,
            level,
            rush: Point::new(0.0, 0.0),
            time: 0.0,
        }
    }

    /// This pose rushing by `rush`, the eye's move over the last step.
    #[must_use]
    pub const fn with_rush(self, rush: Point) -> Self {
        Self { rush, ..self }
    }

    /// This pose `time` seconds in.
    #[must_use]
    pub const fn with_time(self, time: f64) -> Self {
        Self { time, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp, reason = "zero is exact")]
    fn a_new_pose_is_its_parts_and_no_rush() {
        let pose = SkyPose::new(9, Point::new(1.0, 2.0), Point::new(3.0, 4.0), 16);
        assert_eq!(pose.seed, 9);
        assert_eq!(pose.eye, Point::new(1.0, 2.0));
        assert_eq!(pose.anchor, Point::new(3.0, 4.0));
        assert_eq!(pose.level, 16);
        assert_eq!(pose.rush, Point::new(0.0, 0.0));
        assert_eq!(pose.time, 0.0);
    }

    #[test]
    #[allow(clippy::float_cmp, reason = "the time is handed on as given")]
    fn a_pose_at_a_time_keeps_the_rest_and_takes_its_time() {
        let pose = SkyPose::new(9, Point::new(1.0, 2.0), Point::new(3.0, 4.0), 16)
            .with_rush(Point::new(-30.0, 12.5));
        let later = pose.with_time(604_800.25);
        assert_eq!(later.time, 604_800.25);
        assert_eq!(
            SkyPose {
                time: pose.time,
                ..later
            },
            pose
        );
        assert_eq!(later.rush, Point::new(-30.0, 12.5));
        // A rush set after keeps the time.
        assert_eq!(later.with_rush(Point::new(1.0, 1.0)).time, 604_800.25);
    }

    #[test]
    fn a_rushing_pose_keeps_the_rest_and_takes_its_rush() {
        let pose = SkyPose::new(9, Point::new(1.0, 2.0), Point::new(3.0, 4.0), 16);
        let rushing = pose.with_rush(Point::new(-30.0, 12.5));
        assert_eq!(rushing.rush, Point::new(-30.0, 12.5));
        assert_eq!(
            SkyPose {
                rush: pose.rush,
                ..rushing
            },
            pose
        );
    }
}
