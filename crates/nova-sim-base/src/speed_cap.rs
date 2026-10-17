//! The original's per-axis speed caps, which flight, launches and
//! knockback share: `_AdjustedAccel` ([`adjusted_accel`]), a push soft-capped
//! on each axis at the top speed's share there, and `_SpeedLimit`
//! ([`speed_limit`]), a velocity clamped to the top speed on each axis.
//!
//! [`RuleKey::SpeedCap`] chooses between them and the circle the port
//! used before ([`circle`]): [`SpeedCap`] is its reading.

use crate::flight::facing;
use crate::geometry::Vec2;
#[cfg(any(doc, test))]
use crate::rulebook::RuleKey;
use crate::rulebook::RuleSource;

/// How a ship's speed is capped, by [`RuleKey::SpeedCap`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpeedCap {
    /// The original's: thrust soft-capped per axis ([`adjusted_accel`]),
    /// the player's velocity and a push clamped per axis
    /// ([`speed_limit`]), an NPC's velocity not at all.
    #[default]
    PerAxis,
    /// The port's earlier reading: every speed rescaled onto the circle of
    /// the top speed ([`circle`]).
    Circle,
}

impl From<RuleSource> for SpeedCap {
    /// The engine's reading is the original's per-axis cap; the other is
    /// the circle.
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::PerAxis,
            RuleSource::Bible => Self::Circle,
        }
    }
}

impl SpeedCap {
    /// `velocity` clamped to `top_speed` as this reading clamps a push:
    /// each axis ([`speed_limit`]), or the circle ([`circle`]).
    #[must_use]
    pub fn clamp(self, velocity: Vec2, top_speed: f32) -> Vec2 {
        match self {
            Self::PerAxis => speed_limit(velocity, top_speed),
            Self::Circle => circle(velocity, top_speed),
        }
    }
}

/// `velocity` rescaled to `top_speed` when it is faster, keeping its
/// direction: the circle the port capped every ship's speed to before it
/// ported `_AdjustedAccel`. A velocity at or below the top speed is left
/// as it is, bit for bit.
#[must_use]
pub fn circle(velocity: Vec2, top_speed: f32) -> Vec2 {
    let speed = velocity.length();
    if speed > top_speed {
        // Divided first, so a velocity along an axis lands on the top
        // speed exactly.
        let direction = Vec2::new(velocity.x / speed, velocity.y / speed);
        direction * top_speed
    } else {
        velocity
    }
}

/// `velocity` pushed `push` pixels a tick along `heading`, soft-capped
/// at `top_speed`: `_AdjustedAccel` (@0x2ae4b-0x2af43). On each axis the
/// push's share is added while the velocity is short of the top speed's
/// share, when both shares are above none or both below; when they differ
/// in sign, or one is none, it is added whatever the velocity
/// (@0x2aecd-0x2aeee), so a negative push always pulls. Nothing clamps
/// it, so the last push can carry an axis past its share.
#[must_use]
pub fn adjusted_accel(velocity: Vec2, heading: f32, push: f32, top_speed: f32) -> Vec2 {
    let along = facing(heading);
    let gain = |velocity: f32, share: f32| {
        let push = share * push;
        let top = share * top_speed;
        let past = if push > 0.0 && top > 0.0 {
            velocity >= top
        } else if push < 0.0 && top < 0.0 {
            velocity <= top
        } else {
            false
        };
        if past { velocity } else { velocity + push }
    };
    Vec2::new(gain(velocity.x, along.x), gain(velocity.y, along.y))
}

/// `velocity` with each axis clamped to `top_speed` either way
/// (`_SpeedLimit` @0x2af91-0x2afda).
#[must_use]
pub fn speed_limit(velocity: Vec2, top_speed: f32) -> Vec2 {
    // Down to the top speed first, then up to its negative, as the
    // original tests them; `f32::clamp` would panic on a negative one.
    let clamp = |axis: f32| axis.min(top_speed).max(-top_speed);
    Vec2::new(clamp(velocity.x), clamp(velocity.y))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn the_engines_reading_is_per_axis_and_the_other_the_circle() {
        assert_eq!(SpeedCap::from(RuleSource::Engine), SpeedCap::PerAxis);
        assert_eq!(SpeedCap::from(RuleSource::Bible), SpeedCap::Circle);
        assert_eq!(SpeedCap::default(), SpeedCap::PerAxis);
        assert_eq!(RuleKey::SpeedCap.default_source(), RuleSource::Engine);
    }

    #[test]
    fn the_circle_rescales_a_faster_velocity_onto_the_top_speed_and_keeps_a_slower_one() {
        assert_eq!(circle(Vec2::new(0.0, -6.0), 3.0), Vec2::new(0.0, -3.0));
        assert_eq!(circle(Vec2::new(6.0, 8.0), 5.0), Vec2::new(3.0, 4.0));
        assert_eq!(circle(Vec2::new(1.0, 1.0), 3.0), Vec2::new(1.0, 1.0));
        let at = Vec2::new(1.3, -2.9);
        assert_eq!(circle(at, at.length()), at, "exactly at it");
    }

    #[test]
    fn a_reading_clamps_a_push_by_its_own_shape() {
        let fast = Vec2::new(12.0, -15.0);
        assert_eq!(SpeedCap::PerAxis.clamp(fast, 10.0), Vec2::new(10.0, -10.0));
        let circled = SpeedCap::Circle.clamp(fast, 10.0);
        assert!((circled.length() - 10.0).abs() < 1e-5, "{circled:?}");
        assert!((circled.x / circled.y - fast.x / fast.y).abs() < 1e-5);
        let slow = Vec2::new(3.0, -4.0);
        assert_eq!(SpeedCap::PerAxis.clamp(slow, 10.0), slow);
        assert_eq!(SpeedCap::Circle.clamp(slow, 10.0), slow);
    }

    #[test]
    fn a_carrier_at_rest_gives_the_fighter_the_push_along_its_heading() {
        let up = adjusted_accel(Vec2::ZERO, 0.0, 4.0, 10.0);
        assert_eq!(up, facing(0.0) * 4.0);
        let right = adjusted_accel(Vec2::ZERO, 90.0, 4.0, 10.0);
        assert!((right - Vec2::new(4.0, 0.0)).length() < 1e-5, "{right:?}");
    }

    #[test]
    fn an_axis_at_or_past_the_top_speed_gains_nothing_and_the_other_still_gains() {
        // Heading 45: the push is (2.83, -2.83), the top speed's share
        // (7.07, -7.07).
        let push = facing(45.0) * 4.0;
        let top = facing(45.0) * 10.0;
        let fast_x = adjusted_accel(Vec2::new(8.0, 0.0), 45.0, 4.0, 10.0);
        assert_eq!(fast_x, Vec2::new(8.0, push.y), "x past its share");
        let at_y = adjusted_accel(Vec2::new(0.0, top.y), 45.0, 4.0, 10.0);
        assert_eq!(at_y, Vec2::new(push.x, top.y), "y at its share");
        let backwards = adjusted_accel(Vec2::new(-20.0, 20.0), 45.0, 4.0, 10.0);
        assert_eq!(
            backwards,
            Vec2::new(-20.0 + push.x, 20.0 + push.y),
            "moving the other way, both gain"
        );
        let down = adjusted_accel(Vec2::new(0.0, 12.0), 180.0, 4.0, 10.0);
        assert_eq!(down.y, 12.0, "downwards past its share");
        let down = adjusted_accel(Vec2::new(0.0, 9.0), 180.0, 4.0, 10.0);
        assert!((down.y - 13.0).abs() < 1e-5, "{down:?}");
    }

    #[test]
    fn a_push_against_the_top_speeds_share_always_gains() {
        // `_AdjustedAccel` @0x2aec2-0x2aeee tests an axis only while its
        // push and its share of the top speed point the same way: a
        // negative push (a recoil below -1) gains whatever the velocity.
        let push = facing(0.0) * -4.0;
        let fast = adjusted_accel(Vec2::new(0.0, 30.0), 0.0, -4.0, 10.0);
        assert!((fast.y - (30.0 + push.y)).abs() < 1e-5, "{fast:?}");
        let slow = adjusted_accel(Vec2::new(0.0, -30.0), 0.0, -4.0, 10.0);
        assert!((slow.y - (-30.0 + push.y)).abs() < 1e-5, "{slow:?}");
    }

    #[test]
    fn a_push_that_crosses_the_top_speed_is_not_clamped() {
        let v = adjusted_accel(Vec2::new(0.0, -8.0), 0.0, 4.0, 10.0);
        assert!((v.y + 12.0).abs() < 1e-5, "{v:?}");
        assert!(v.x.abs() < 1e-5);
    }

    #[test]
    fn a_push_against_the_top_speeds_sign_is_added_whatever_the_velocity() {
        // A negative push with a positive top speed: the signs differ on
        // each axis, so `_AdjustedAccel` adds it unchecked (@0x2aecd-0x2aeee).
        let pulled = adjusted_accel(Vec2::ZERO, 90.0, -0.5, 2.0);
        assert!(
            (pulled - Vec2::new(-0.5, 0.0)).length() < 1e-5,
            "{pulled:?}"
        );
        let further = adjusted_accel(Vec2::new(-5.0, 0.0), 90.0, -0.5, 2.0);
        assert!((further.x + 5.5).abs() < 1e-5, "{further:?}");
        // A negative push with no top speed: added too.
        let stopped = adjusted_accel(Vec2::new(-1.0, 0.0), 90.0, -0.5, 0.0);
        assert!((stopped.x + 1.5).abs() < 1e-5, "{stopped:?}");
        // A positive push with no top speed: added too.
        let unbounded = adjusted_accel(Vec2::new(3.0, 0.0), 90.0, 0.5, 0.0);
        assert!((unbounded.x - 3.5).abs() < 1e-5, "{unbounded:?}");
    }

    #[test]
    fn a_speed_limit_clamps_each_axis_either_way() {
        assert_eq!(
            speed_limit(Vec2::new(12.0, -15.0), 10.0),
            Vec2::new(10.0, -10.0)
        );
        assert_eq!(
            speed_limit(Vec2::new(3.0, -4.0), 10.0),
            Vec2::new(3.0, -4.0)
        );
    }
}
