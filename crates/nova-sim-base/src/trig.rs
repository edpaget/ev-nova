//! The original's table trigonometry in whole degrees: `_TableArcTan`
//! and the headings `_CalcInterceptAngle` and `_CalcLeadAngle` work out
//! with it, `_TableSin`, `_TableCos` and `_Accel`'s push, and
//! `_AngularDifference`.
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
///
/// A NaN or infinite coordinate stays in the table: the ratio is then NaN
/// and `cvttss2si`'s 0x80000000 has a low word of 0 (@0x25a0e-0x25a12),
/// index 0, as Rust's NaN cast gives. The quadrant tests are `ucomiss`
/// with `jbe`, `jb` and `cmova` (@0x25a50-0x25a7c), so a NaN coordinate
/// counts as below 0 on neither axis, and an x below 0 with a NaN y
/// keeps its entry, neither `180 − c` nor `180 + c`.
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
    // `jb` is taken below 0 and unordered; `cmova` only below 0.
    let below = |v: f32| v < 0.0;
    let below_or_nan = |v: f32| v < 0.0 || v.is_nan();
    if below(x) {
        if !below_or_nan(y) {
            c = 180 - c;
        }
        if below(y) {
            c += 180;
        }
    }
    if !below_or_nan(x) && below(y) {
        c = -c;
    }
    i16::try_from((c - 90).rem_euclid(360)).unwrap_or_default()
}

/// `_BuildTrigTable`'s angle for whole degree `degrees` normalized to
/// 0..360 as `_TableSin` and `_TableCos` do (@0x25913-0x25926): `(float)i
/// × 0.01745329`, in doubles, rounded to `f32` (@0x2583b-0x2584c).
fn table_radians(degrees: i16) -> f64 {
    let index = f32::from(degrees.rem_euclid(360));
    f64::from((f64::from(index) * 0.017_453_29) as f32)
}

/// `_TableSin` (@0x2590a): the `f32` sine of whole degree `degrees`.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn table_sin(degrees: i16) -> f32 {
    table_radians(degrees).sin() as f32
}

/// `_TableCos` (@0x2593c): the `f32` cosine of whole degree `degrees`.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn table_cos(degrees: i16) -> f32 {
    table_radians(degrees).cos() as f32
}

/// `_Accel(angle, magnitude, &v)`'s push (@0x2ade6): `magnitude` along
/// whole degree `angle`, x by `magnitude · sin` and y by `−magnitude ·
/// cos`, in `f32`. The original adds it to `v` in place (`v.x + m·sin`,
/// `v.y − m·cos`), which `v + accel(..)` matches exactly.
#[must_use]
pub fn accel(angle: i16, magnitude: f32) -> Vec2 {
    Vec2::new(
        magnitude * table_sin(angle),
        -(magnitude * table_cos(angle)),
    )
}

/// A heading truncated to the whole degree the original passes on:
/// `cvttss2si` then the low word (`cwtl`/`movswl`), as every `_Accel`
/// call takes `trunc(heading)`. NaN is 0, 0x80000000's low word.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn whole(heading: f32) -> i16 {
    // Rust's cast saturates where `cvttss2si` gives 0x80000000; only the
    // low word is kept either way, and headings stay far inside 16 bits.
    (heading as i32) as i16
}

/// A heading taken round once each way, as `_SpawnShot`, point defence
/// and `_SpawnShotSubmunitions` wrap theirs after the inaccuracy (`< 0` →
/// +360, then `≥ 360` → −360; @0x3ed7d-0x3edc1, @0x39834-0x3986e,
/// @0x3f50f-0x3f54f). A heading further out stays out by the rest.
#[must_use]
pub fn wrap_once(heading: f32) -> f32 {
    let heading = if heading < 0.0 {
        heading + 360.0
    } else {
        heading
    };
    if heading >= 360.0 {
        heading - 360.0
    } else {
        heading
    }
}

/// `_AngularDifference(a, b)` (@0x649e-0x64f4), in the original's 16
/// bits: |a − b|, taken from 360 when a and b lie either side of 179,
/// then from 360 again when above 180. For headings in 0..360 it is how
/// far apart they are, 0 to 180. `_TurretBlindSpot` folds the same way,
/// inline (@0xb341-0xb387).
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn angular_difference(a: i16, b: i16) -> i16 {
    let mut difference = (i32::from(a) - i32::from(b)).abs() as i16;
    if (a > 179) != (b > 179) {
        difference = 360_i16.wrapping_sub(difference);
    }
    if difference > 180 {
        difference = 360_i16.wrapping_sub(difference);
    }
    difference
}

/// The shot `_CalcLeadAngle` leads with: its speed (weapon +0x2c, in
/// pixels a tick) and whether it flies as a rocket (guidance 6). The
/// weapon's guidance decides whether there is one at all (the gate of -1,
/// 4, 6, 7, 8 and 9, @0x2b077-0x2b0a3), which the caller, who knows the
/// guidance, applies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LeadShot {
    /// Its speed, in pixels a tick.
    pub speed: f32,
    /// Whether it is a rocket, which closes on its speed.
    pub rocket: bool,
}

/// The rocket's reach in ticks at full speed (0xdd540).
const ROCKET_REACH: f64 = 19.59;
/// The share of its speed a rocket averages within its reach (0xdd548).
const ROCKET_AVERAGE: f64 = 0.316;
/// The ticks a rocket takes to cover its reach, past `ROCKET_REACH` at
/// full speed (0xdd558).
const ROCKET_REACH_TICKS: f64 = 2.066_666_666_666_667;

/// `_CalcLeadAngle(ship, target, weap, &from)` (@0x2b025-0x2b18d): the
/// whole-degree heading from `from` to where `target` (its position and
/// velocity) will be when `shot` gets there, its velocity taken relative
/// to the firer's `firer_velocity`. With no `shot` it is the table
/// bearing ([`table_arctan`] of `from` − the target's position).
///
/// The time is `d / speed` in `f32`, `d` the `f32` distance; a rocket's is
/// in doubles, `d / (speed × 0.316)` within `speed × 19.59` of the target
/// and `(d − 19.59 speed) / speed + 2.0666667` past it. The aim point is
/// the target's position plus its relative velocity times the time, in
/// `f32`. A shot of no speed has an infinite time (NaN at no distance),
/// which the table still reads ([`table_arctan`]).
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn lead_angle(
    from: Vec2,
    firer_velocity: Vec2,
    target: (Vec2, Vec2),
    shot: Option<LeadShot>,
) -> i16 {
    let (position, velocity) = target;
    let Some(shot) = shot else {
        return table_arctan(from.x - position.x, from.y - position.y);
    };
    let (dx, dy) = (position.x - from.x, position.y - from.y);
    let distance = (dx * dx + dy * dy).sqrt();
    let ticks = if shot.rocket {
        let (d, s) = (f64::from(distance), f64::from(shot.speed));
        if s * ROCKET_REACH >= d {
            (d / (s * ROCKET_AVERAGE)) as f32
        } else {
            ((d + s * -ROCKET_REACH) / s + ROCKET_REACH_TICKS) as f32
        }
    } else {
        distance / shot.speed
    };
    let aim_y = position.y + (velocity.y - firer_velocity.y) * ticks;
    let aim_x = position.x + (velocity.x - firer_velocity.x) * ticks;
    table_arctan(from.x - aim_x, from.y - aim_y)
}

/// `_CalcObjectLeadAngle(ship, object, velocity, weap, &from)`
/// (@0x2b18e-0x2b2ef): [`lead_angle`] at a point object, its position
/// and velocity `object`, from `from`, relative to the firer's
/// `firer_velocity`, with `shot` the shot its weapon's gate leads with
/// (guidance -1, 4, 7, 8 and 9, which the caller applies). Its rocket
/// branch (@0x2b248-0x2b28c) is dead, as guidance 6 fails that gate, so
/// any shot is led as a plain one. With no shot it is the table bearing.
#[must_use]
pub fn object_lead_angle(
    from: Vec2,
    firer_velocity: Vec2,
    object: (Vec2, Vec2),
    shot: Option<LeadShot>,
) -> i16 {
    let plain = shot.map(|shot| LeadShot {
        rocket: false,
        ..shot
    });
    lead_angle(from, firer_velocity, object, plain)
}

/// `_CalcInterceptAngle(from, to)` (@0x2af60-0x2af90): the heading from
/// `from` toward `to`, [`table_arctan`] of `from` − `to` in `f32`.
#[must_use]
pub fn intercept_angle(from: Vec2, to: Vec2) -> i16 {
    table_arctan(from.x - to.x, from.y - to.y)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
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

    #[test]
    fn a_nan_coordinate_is_below_zero_on_neither_axis_as_the_ucomiss_tests_read_it() {
        // Index 0, not steep: x < 0 with y NaN keeps the entry (180 − c
        // is skipped by `jb`, 180 + c by `cmova`), so 0 − 90 wraps to 270.
        assert_eq!(table_arctan(-1.0, f32::NAN), 270);
        // x NaN with y < 0: neither the x < 0 block nor the negation.
        assert_eq!(table_arctan(f32::NAN, -1.0), 270);
        assert_eq!(table_arctan(f32::NAN, f32::NAN), 270);
        // Infinities are ordered: (−inf, 5) is plain right of the origin.
        assert_eq!(table_arctan(f32::NEG_INFINITY, 5.0), 90);
        assert_eq!(table_arctan(f32::INFINITY, f32::INFINITY), 270);
    }

    #[test]
    fn sine_and_cosine_normalize_whole_degrees() {
        for (a, b) in [(-45, 315), (405, 45), (360, 0), (-360, 0), (720, 0)] {
            assert_eq!(table_sin(a), table_sin(b), "{a}");
            assert_eq!(table_cos(a), table_cos(b), "{a}");
        }
        assert_eq!(table_sin(90), 1.0);
        assert_eq!(table_cos(0), 1.0);
        assert_eq!(table_sin(0), 0.0);
        assert!(table_cos(90) > 0.0, "a float's 90 degrees is a hair short");
        assert!(table_cos(90) < 1e-6);
        assert!(table_sin(-90) < 0.0);
    }

    #[test]
    fn accel_pushes_along_the_whole_degree_with_y_down() {
        assert_eq!(accel(90, 10.0).x, 10.0);
        assert_eq!(accel(0, 10.0), Vec2::new(0.0, -10.0));
        assert_eq!(accel(180, 10.0).y, 10.0);
        assert_eq!(accel(270, 2.0).x, -2.0);
        assert_eq!(accel(-90, 2.0), accel(270, 2.0), "taken into 0..360");
        assert_eq!(
            accel(45, 10.0),
            Vec2::new(10.0 * table_sin(45), -(10.0 * table_cos(45)))
        );
    }

    #[test]
    fn whole_truncates_toward_zero_into_sixteen_bits() {
        assert_eq!(whole(45.9), 45);
        assert_eq!(whole(-0.5), 0);
        assert_eq!(whole(-1.5), -1);
        assert_eq!(whole(359.99), 359);
        assert_eq!(whole(65_536.0 + 7.0), 7, "cwtl keeps the low word");
        assert_eq!(whole(f32::NAN), 0, "0x80000000's low word");
    }

    #[test]
    fn wrap_once_takes_a_heading_round_once_each_way() {
        assert_eq!(wrap_once(-10.0), 350.0);
        assert_eq!(wrap_once(370.0), 10.0);
        assert_eq!(wrap_once(360.0), 0.0);
        assert_eq!(wrap_once(0.0), 0.0);
        assert_eq!(wrap_once(-400.0), -40.0, "only once");
        assert_eq!(wrap_once(800.0), 440.0, "only once");
        // `ucomiss` finds -0.0 equal to 0.0, so it is not taken round.
        assert_eq!(wrap_once(-0.0).to_bits(), (-0.0_f32).to_bits());
    }

    #[test]
    fn the_angular_difference_folds_as_the_originals() {
        assert_eq!(angular_difference(0, 0), 0);
        assert_eq!(angular_difference(0, 170), 170);
        assert_eq!(angular_difference(170, 0), 170);
        assert_eq!(angular_difference(10, 350), 20, "across north");
        assert_eq!(angular_difference(350, 10), 20);
        assert_eq!(angular_difference(170, 190), 20, "across south");
        assert_eq!(angular_difference(100, 200), 100);
        assert_eq!(angular_difference(0, 179), 179);
        assert_eq!(angular_difference(0, 180), 180, "180 is past 179");
        assert_eq!(angular_difference(0, 181), 179);
        assert_eq!(angular_difference(179, 180), 1);
        assert_eq!(angular_difference(180, 181), 1);
        assert_eq!(angular_difference(1, 181), 180);
        assert_eq!(angular_difference(359, 0), 1);
        // Out of 0..360 it folds by the same steps, as the original does
        // in 16 bits: 720 and 0 lie either side of 179, so 360 − 720.
        assert_eq!(angular_difference(720, 0), -360);
        assert_eq!(angular_difference(-90, 90), 180);
        assert_eq!(angular_difference(359, -1), 0, "a bomb's unwrapped −1");
    }

    const PLAIN: Option<LeadShot> = Some(LeadShot {
        speed: 10.0,
        rocket: false,
    });

    #[test]
    fn with_no_weapon_to_lead_with_the_lead_is_the_table_bearing() {
        let target = (Vec2::new(100.0, 0.0), Vec2::new(0.0, 5.0));
        assert_eq!(lead_angle(Vec2::ZERO, Vec2::ZERO, target, None), 90);
    }

    #[test]
    fn the_lead_reads_where_the_shot_leaves_and_both_velocities_on_both_axes() {
        let from = Vec2::new(30.0, 40.0);
        let firer = Vec2::new(2.0, 1.0);
        let target = (Vec2::new(130.0, -60.0), Vec2::new(-3.0, 4.0));
        assert_eq!(
            lead_angle(from, firer, target, None),
            table_arctan(-100.0, 100.0)
        );
        // 141.42 away at 10 a tick: 14.142 ticks, the target moving
        // (-5, 3) relative to the firer, so aimed at (59.29, -17.57).
        assert_eq!(
            lead_angle(from, firer, target, PLAIN),
            table_arctan(30.0 - 59.289_32, 40.0 + 17.573_59)
        );
        assert_eq!(table_arctan(30.0 - 59.289_32, 40.0 + 17.573_59), 26);
    }

    #[test]
    fn a_resting_target_is_led_to_its_table_bearing() {
        let target = (Vec2::new(100.0, 100.0), Vec2::ZERO);
        assert_eq!(lead_angle(Vec2::ZERO, Vec2::ZERO, target, PLAIN), 134);
    }

    #[test]
    fn a_crossing_target_is_led_by_its_distance_over_the_speed() {
        // 100 away at 10 a tick: 10 ticks, so aimed at (100, 50).
        let target = (Vec2::new(100.0, 0.0), Vec2::new(0.0, 5.0));
        let ahead = table_arctan(-100.0, -50.0);
        assert_eq!(lead_angle(Vec2::ZERO, Vec2::ZERO, target, PLAIN), ahead);
        assert_ne!(ahead, 90);
    }

    #[test]
    fn the_lead_reads_the_targets_velocity_relative_to_the_firers() {
        let target = (Vec2::new(100.0, 0.0), Vec2::new(0.0, 5.0));
        let along = Vec2::new(0.0, 5.0);
        assert_eq!(lead_angle(Vec2::ZERO, along, target, PLAIN), 90);
    }

    #[test]
    fn the_lead_is_measured_from_where_the_shot_leaves() {
        let target = (Vec2::new(100.0, 0.0), Vec2::ZERO);
        let from = Vec2::new(100.0, 100.0);
        assert_eq!(lead_angle(from, Vec2::ZERO, target, PLAIN), 0);
    }

    #[test]
    fn a_rocket_leads_by_its_two_doubles_branches() {
        let rocket = Some(LeadShot {
            speed: 10.0,
            rocket: true,
        });
        // 100 away, within 19.59 × 10: t = 100 / (10 × 0.316).
        let near = (Vec2::new(100.0, 0.0), Vec2::new(0.0, 1.0));
        let t = (100.0_f64 / (10.0 * 0.316)) as f32;
        assert_eq!(
            lead_angle(Vec2::ZERO, Vec2::ZERO, near, rocket),
            table_arctan(-100.0, -t)
        );
        // 300 away, past it: t = (300 − 195.9) / 10 + 2.0666667.
        let far = (Vec2::new(300.0, 0.0), Vec2::new(0.0, 10.0));
        let t = ((300.0_f64 + 10.0 * -19.59) / 10.0 + 2.066_666_666_666_667) as f32;
        assert_eq!(
            lead_angle(Vec2::ZERO, Vec2::ZERO, far, rocket),
            table_arctan(-300.0, -(10.0 * t))
        );
        assert_ne!(
            lead_angle(Vec2::ZERO, Vec2::ZERO, far, rocket),
            lead_angle(Vec2::ZERO, Vec2::ZERO, far, PLAIN),
            "a plain shot's t is 30"
        );
    }

    #[test]
    fn a_rocket_at_its_reach_takes_the_near_branch() {
        // 19.59 × 10 = 195.9 ≥ 195.0: near, t = 195 / 3.16 = 61.7; the far
        // branch would give (195 − 195.9) / 10 + 2.07 = 1.98.
        let rocket = Some(LeadShot {
            speed: 10.0,
            rocket: true,
        });
        let target = (Vec2::new(195.0, 0.0), Vec2::new(0.0, 1.0));
        let t = (195.0_f64 / (10.0 * 0.316)) as f32;
        assert_eq!(
            lead_angle(Vec2::ZERO, Vec2::ZERO, target, rocket),
            table_arctan(-195.0, -t)
        );
    }

    #[test]
    fn an_object_is_led_relative_to_the_firers_velocity() {
        let from = Vec2::new(30.0, 40.0);
        let firer = Vec2::new(2.0, 1.0);
        let object = (Vec2::new(130.0, -60.0), Vec2::new(-3.0, 4.0));
        assert_eq!(object_lead_angle(from, firer, object, PLAIN), 26);
        let along = Vec2::new(0.0, 5.0);
        let crossing = (Vec2::new(100.0, 0.0), Vec2::new(0.0, 5.0));
        assert_eq!(
            object_lead_angle(Vec2::ZERO, along, crossing, PLAIN),
            90,
            "moving with it"
        );
        assert_ne!(
            object_lead_angle(Vec2::ZERO, Vec2::ZERO, crossing, PLAIN),
            90,
            "moving across it"
        );
    }

    #[test]
    fn an_object_with_no_shot_to_lead_with_is_its_table_bearing() {
        let object = (Vec2::new(100.0, 0.0), Vec2::new(0.0, 5.0));
        assert_eq!(object_lead_angle(Vec2::ZERO, Vec2::ZERO, object, None), 90);
    }

    #[test]
    fn an_object_is_never_led_as_a_rocket() {
        // `_CalcObjectLeadAngle`'s rocket branch is dead (@0x2b248).
        let rocket = Some(LeadShot {
            speed: 10.0,
            rocket: true,
        });
        let far = (Vec2::new(300.0, 0.0), Vec2::new(0.0, 10.0));
        assert_eq!(
            object_lead_angle(Vec2::ZERO, Vec2::ZERO, far, rocket),
            lead_angle(Vec2::ZERO, Vec2::ZERO, far, PLAIN)
        );
    }

    #[test]
    fn a_shot_of_no_speed_leads_to_infinity_as_the_table_reads_it() {
        let shot = Some(LeadShot {
            speed: 0.0,
            rocket: false,
        });
        // t is infinite: the aim is (+inf, +inf), a NaN ratio, index 0,
        // both below 0 from the firer: 180 + 0, taken 90 back.
        let target = (Vec2::new(100.0, 0.0), Vec2::new(3.0, 5.0));
        assert_eq!(lead_angle(Vec2::ZERO, Vec2::ZERO, target, shot), 90);
        // Not moving across, 0 × inf is NaN on x: below 0 on neither.
        let along = (Vec2::new(100.0, 0.0), Vec2::new(0.0, 5.0));
        assert_eq!(lead_angle(Vec2::ZERO, Vec2::ZERO, along, shot), 270);
        // At rest, 0 × inf is NaN on both axes.
        let resting = (Vec2::new(100.0, 0.0), Vec2::ZERO);
        assert_eq!(lead_angle(Vec2::ZERO, Vec2::ZERO, resting, shot), 270);
    }
}
