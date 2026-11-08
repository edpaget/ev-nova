//! The floating stick's arithmetic: where the thumb is against where it
//! landed, as the raw offset the touch device reads
//! ([`TouchControl::Stick`](crate::input::TouchControl::Stick)), and
//! whether it holds Reverse Course.

use crate::geometry::Point;
use crate::keymap::gamepad::crossing;

/// The stick's offset with the thumb at `at`, having landed at `base`, on
/// a ring of `radius` logical units: `(at − base) / radius`, y negated so
/// forward (screen up) is positive, clamped to the unit circle by length,
/// so a push past the ring reads full along its direction. Raw: the
/// reader applies the flight dead zone.
#[must_use]
pub fn offset(base: Point, at: Point, radius: f32) -> (f32, f32) {
    let x = (at.x - base.x) / radius;
    let y = (base.y - at.y) / radius;
    let length = x.hypot(y);
    if length > 1.0 {
        (x / length, y / length)
    } else {
        (x, y)
    }
}

/// Whether the stick holds Reverse Course at forward offset `y`, having
/// held it (`held`) or not: pulled back to the pad's press threshold
/// it holds, and it lets go only under the release threshold, by the
/// pad's own rule ([`crossing`]), so a stick resting between the two
/// doesn't chatter and a stick pulled back does what the pad's throttle
/// does.
#[must_use]
pub fn reverse_after(held: bool, y: f32) -> bool {
    crossing(-y).unwrap_or(held)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::keymap::gamepad::{PRESS_THRESHOLD, RELEASE_THRESHOLD, dead_zoned};

    const BASE: Point = Point::new(200.0, 600.0);
    const RADIUS: f32 = 56.0;

    #[test]
    fn the_offset_is_a_fraction_of_the_radius_with_forward_up() {
        assert_eq!(offset(BASE, BASE, RADIUS), (0.0, 0.0));
        assert_eq!(offset(BASE, Point::new(228.0, 600.0), RADIUS), (0.5, 0.0));
        assert_eq!(offset(BASE, Point::new(200.0, 572.0), RADIUS), (0.0, 0.5));
        assert_eq!(offset(BASE, Point::new(172.0, 628.0), RADIUS), (-0.5, -0.5));
    }

    #[test]
    fn a_push_past_the_ring_reads_full_along_its_direction() {
        let (x, y) = offset(BASE, Point::new(200.0 + 3.0 * RADIUS, 600.0), RADIUS);
        assert_eq!((x, y), (1.0, 0.0));
        let (x, y) = offset(BASE, Point::new(300.0, 500.0), RADIUS);
        assert!((x.hypot(y) - 1.0).abs() < 1e-6, "{x} {y}");
        assert!((x - y).abs() < 1e-6, "diagonal: {x} {y}");
    }

    #[test]
    fn a_half_radius_push_sideways_is_about_half_the_turn() {
        let (x, y) = offset(BASE, Point::new(200.0 + RADIUS / 2.0, 600.0), RADIUS);
        let turn = dead_zoned(x, y).0;
        assert_eq!(turn, dead_zoned(0.5, 0.0).0);
        assert!((0.35..0.6).contains(&turn), "{turn}");
    }

    #[test]
    fn pulled_back_past_the_press_threshold_it_holds_reverse_and_lets_go_under_the_release_threshold()
     {
        assert!(!reverse_after(false, -PRESS_THRESHOLD + 0.01));
        assert!(reverse_after(false, -PRESS_THRESHOLD));
        assert!(reverse_after(false, -1.0));
        assert!(!reverse_after(true, -RELEASE_THRESHOLD + 0.01));
        assert!(!reverse_after(true, 0.8), "pushed forward");
    }

    #[test]
    fn a_stick_resting_between_the_thresholds_does_not_chatter() {
        let between = -f32::midpoint(PRESS_THRESHOLD, RELEASE_THRESHOLD);
        assert!(reverse_after(true, between), "kept held");
        assert!(!reverse_after(false, between), "kept let go");
        assert!(reverse_after(true, -RELEASE_THRESHOLD));
    }
}
