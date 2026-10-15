//! The original's table trigonometry in whole degrees: `_TableArcTan`
//! and the headings `_CalcInterceptAngle` works out with it.
//!
//! Headings are on Nova's compass: 0 up the screen (−y), growing
//! clockwise, as the [`flight`](crate::flight) module's are, but whole
//! degrees in 0..360, as the original stores them in 16 bits.

use crate::geometry::Vec2;

/// The arctangent table's last index (`_BuildTrigTable` @0x258c1-0x25900
/// builds 0x400 entries).
const ARCTAN_LAST: i32 = 0x3FF;

/// `_BuildTrigTable`'s arctangent entry `index`: `trunc(atan(index ×
/// 0.01) × 57.2957795)`, in doubles (@0x258c9-0x258f4).
#[allow(clippy::cast_possible_truncation)]
fn arctan_entry(index: i32) -> i32 {
    ((f64::from(index) * 0.01).atan() * 57.295_779_5) as i32
}

/// The heading, in whole degrees on Nova's compass (0 up, clockwise, y
/// down), from (`x`, `y`) toward (0, 0), as `_TableArcTan` (@0x259a0)
/// works it: the smaller of |x| and |y| over the larger, in `f32`, times
/// 100 (0xdd060), truncated and clamped to the table, its entry turned
/// into the octant and the quadrant, then taken 90 back and wrapped to
/// 0..360. (0, 0) is 0 (@0x259b3-0x259c1). The table truncates, so a
/// diagonal is a degree short: (1, 1) is 314, not 315.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn table_arctan(x: f32, y: f32) -> i16 {
    if x == 0.0 && y == 0.0 {
        return 0;
    }
    let (ax, ay) = (x.abs(), y.abs());
    let steep = ay > ax;
    let ratio = if steep { ax / ay } else { ay / ax };
    let index = ((ratio * 100.0) as i32).clamp(0, ARCTAN_LAST);
    let mut c = arctan_entry(index);
    if steep {
        c = 90 - c;
    }
    if x < 0.0 {
        c = if y < 0.0 { 180 + c } else { 180 - c };
    } else if y < 0.0 {
        c = -c;
    }
    i16::try_from((c - 90).rem_euclid(360)).unwrap_or_default()
}

/// `_CalcInterceptAngle(from, to)` (@0x2af60-0x2af90): the heading from
/// `from` toward `to`, [`table_arctan`] of `from` − `to` in `f32`.
#[must_use]
pub fn intercept_angle(from: Vec2, to: Vec2) -> i16 {
    table_arctan(from.x - to.x, from.y - to.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_heading_points_from_the_point_to_the_origin_on_novas_compass() {
        assert_eq!(table_arctan(1.0, 0.0), 270);
        assert_eq!(table_arctan(0.0, 1.0), 0);
        assert_eq!(table_arctan(-1.0, 0.0), 90);
        assert_eq!(table_arctan(0.0, -1.0), 180);
        assert_eq!(table_arctan(0.0, 0.0), 0);
    }

    #[test]
    fn a_diagonal_is_a_degree_short_as_the_table_truncates() {
        assert_eq!(table_arctan(1.0, 1.0), 314);
        assert_eq!(table_arctan(-1.0, -1.0), 134);
        assert_eq!(table_arctan(1.0, -1.0), 226);
        assert_eq!(table_arctan(-1.0, 1.0), 46);
    }

    #[test]
    fn the_ratio_truncates_to_a_hundredth_and_the_entry_to_a_degree() {
        // 0.555 is index 55, whose atan(0.55) is 28.81 degrees: 28.
        assert_eq!(table_arctan(1000.0, 555.0), 298);
        assert_eq!(table_arctan(555.0, 1000.0), 332);
        assert_eq!(table_arctan(-1000.0, 555.0), 62);
        assert_eq!(table_arctan(-1000.0, -555.0), 118);
        assert_eq!(table_arctan(1000.0, -555.0), 242);
        assert_eq!(table_arctan(-555.0, -1000.0), 152);
    }

    #[test]
    fn the_intercept_angle_is_the_heading_from_one_point_to_the_other() {
        let at = Vec2::new(10.0, 10.0);
        assert_eq!(intercept_angle(at, Vec2::new(10.0, 0.0)), 0, "up");
        assert_eq!(intercept_angle(at, Vec2::new(20.0, 10.0)), 90, "right");
        assert_eq!(intercept_angle(at, Vec2::new(10.0, 20.0)), 180, "down");
        assert_eq!(intercept_angle(at, Vec2::new(0.0, 10.0)), 270, "left");
        assert_eq!(
            intercept_angle(at, Vec2::new(20.0, 20.0)),
            134,
            "a degree short"
        );
        assert_eq!(intercept_angle(at, at), 0);
    }
}
