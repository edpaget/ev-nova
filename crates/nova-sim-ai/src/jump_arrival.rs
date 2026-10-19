//! The original's jump arrival: where the player comes out of hyperspace,
//! and how fast, as `_HandlePlayer`'s arrival block (@0x6bc0d-0x6bd20)
//! places it. It is
//! [`RuleKey::JumpArrival`](crate::RuleKey::JumpArrival)'s engine
//! reading; the other reading is the at-rest arrival of
//! [`hyperspace::arrival`](crate::hyperspace::arrival).
//!
//! - **Where.** The position is zeroed (@0x6bc0d-0x6bc15), then moved
//!   [`ARRIVAL_RADIUS`] along the bearing back toward the system the ship
//!   left: `_Accel`(bearing + 180 mod 360, 1350.0) (@0x6bc8c-0x6bccd),
//!   where the bearing is `_CalcInterceptAngle` of the departure's and the
//!   first hop's map positions (@0x6bc1b-0x6bc87,
//!   [`intercept_angle`](crate::trig::intercept_angle)), the direction of
//!   travel. A multi-jump runs the block once, before its hop loop
//!   (@0x6bdb4-0x6bdfc), so the side is the first hop's, not the last
//!   leg's. Two systems at one map position give bearing 0, so the ship
//!   arrives 1350 below the centre.
//! - **The heading** is not written: the ship keeps the one the warp left
//!   it, turned toward that same bearing.
//! - **The speed.** The velocity is zeroed (@0x6bcd2-0x6bcd8). An
//!   inertialess ship (`_ShipIsInertialess` @0x6bcde) keeps it at zero,
//!   its speed along its heading set to its top speed (@0x6bce7-0x6bcf5),
//!   which its velocity follows from the next tick. Any other ship moves
//!   at its top speed along its whole-degree heading,
//!   `_Accel`(trunc heading, `_ShipMaxSpeed`) (@0x6bcfa-0x6bd1b).
//!
//! The block plays no sound, and nothing of it is left out.

use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::trig::{accel, intercept_angle, whole};

/// How far from the centre the player arrives: 1350 pixels, the f32
/// 0x44a8c000 `_HandlePlayer` passes to `_Accel` (@0x6bc94).
pub const ARRIVAL_RADIUS: f32 = 1350.0;

/// The player's ship as it arrives from `departure` bound first for
/// `first_hop` (both map positions), facing `heading` with `top_speed`,
/// and its speed along its heading: [`ARRIVAL_RADIUS`] out on the
/// departure's side, at top speed (see the module docs). An inertialess
/// ship arrives still, its speed `top_speed`; any other's speed is its
/// velocity's length.
#[must_use]
pub fn top_speed_arrival(
    departure: Vec2,
    first_hop: Vec2,
    heading: f32,
    top_speed: f32,
    inertialess: bool,
) -> (ShipState, f32) {
    let bearing = intercept_angle(departure, first_hop);
    let position = accel((bearing + 180) % 360, ARRIVAL_RADIUS);
    let (velocity, speed) = if inertialess {
        (Vec2::ZERO, top_speed)
    } else {
        let velocity = accel(whole(heading), top_speed);
        (velocity, velocity.length())
    };
    (
        ShipState {
            position,
            velocity,
            heading,
        },
        speed,
    )
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn near(a: Vec2, b: Vec2, within: f32) -> bool {
        (a - b).length() <= within
    }

    #[test]
    fn the_arrival_radius_is_the_originals() {
        assert_eq!(ARRIVAL_RADIUS.to_bits(), 0x44a8_c000);
    }

    #[test]
    fn the_ship_arrives_1350_out_on_the_side_it_came_from() {
        let (ship, _) = top_speed_arrival(Vec2::ZERO, Vec2::new(600.0, 0.0), 0.0, 6.0, false);
        assert!(
            near(ship.position, Vec2::new(-1350.0, 0.0), 1e-3),
            "{ship:?}"
        );
        assert!((ship.position.length() - 1350.0).abs() < 1e-3);
        let (ship, _) = top_speed_arrival(
            Vec2::new(600.0, 0.0),
            Vec2::new(600.0, 600.0),
            0.0,
            6.0,
            false,
        );
        assert!(
            near(ship.position, Vec2::new(0.0, -1350.0), 1e-3),
            "{ship:?}"
        );
        // A bearing whose opposite wraps past 360: travelling west.
        let (ship, _) = top_speed_arrival(Vec2::ZERO, Vec2::new(-600.0, 0.0), 0.0, 6.0, false);
        assert!(
            near(ship.position, Vec2::new(1350.0, 0.0), 1e-3),
            "{ship:?}"
        );
    }

    #[test]
    fn two_systems_at_one_place_arrive_below_the_centre() {
        let at = Vec2::new(40.0, -70.0);
        let (ship, _) = top_speed_arrival(at, at, 0.0, 6.0, false);
        assert!(
            near(ship.position, Vec2::new(0.0, 1350.0), 1e-3),
            "{ship:?}"
        );
    }

    #[test]
    fn an_inertial_ship_moves_at_top_speed_along_its_whole_degree_heading() {
        let (ship, speed) = top_speed_arrival(Vec2::ZERO, Vec2::new(600.0, 0.0), 90.7, 6.0, false);
        assert!(near(ship.velocity, Vec2::new(6.0, 0.0), 1e-5), "{ship:?}");
        assert_eq!(ship.heading, 90.7);
        assert!((speed - 6.0).abs() < 1e-5, "{speed}");
        let (ship, _) = top_speed_arrival(Vec2::ZERO, Vec2::new(600.0, 0.0), 0.0, 6.0, false);
        assert!(near(ship.velocity, Vec2::new(0.0, -6.0), 1e-5), "{ship:?}");
    }

    #[test]
    fn an_inertialess_ship_arrives_still_with_its_speed_at_top() {
        let (ship, speed) = top_speed_arrival(Vec2::ZERO, Vec2::new(600.0, 0.0), 90.7, 6.0, true);
        assert_eq!(ship.velocity, Vec2::ZERO);
        assert_eq!(speed, 6.0);
        assert_eq!(ship.heading, 90.7);
        assert!(
            near(ship.position, Vec2::new(-1350.0, 0.0), 1e-3),
            "{ship:?}"
        );
    }

    #[test]
    fn the_heading_is_kept_whatever_the_bearing() {
        let (ship, _) = top_speed_arrival(Vec2::ZERO, Vec2::new(600.0, 0.0), 200.0, 6.0, false);
        assert_eq!(ship.heading, 200.0);
        assert!(near(ship.velocity, accel(200, 6.0), 1e-6), "{ship:?}");
    }
}
