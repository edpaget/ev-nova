//! What a miner's goals decide each tick, as `_HighLevelAIHandler`'s
//! blocks for states 16 and 17 (@0x8f832-0x8f903, @0x8faf3-0x8fafd in the
//! `EV Nova` executable) and `_LowLevelAIHandler`'s orders 0x13-0x15
//! (@0x88140-0x88642) work them out. The traffic flies them
//! ([`mine_flight`](crate::traffic::mine_flight)), and the fire picks its
//! gun by them ([`fire_untargeted`](crate::ai::fire_untargeted)).
//!
//! - **The order** ([`order`], state 16's high level): with asteroid slot
//!   0 empty, order 1, a stop ([`MineOrder::Stop`], @0x8f879); within
//!   [`mine_box`] of slot 0 on both axes, inclusive, order 0x14
//!   ([`MineOrder::Kill`], @0x8f8ec); otherwise order 0x13
//!   ([`MineOrder::Chase`], @0x8f8fa). The box has no inertialess factor,
//!   unlike the plunder's. The original then takes the box as a 16-bit
//!   word (`movswl` @0x8f89b), which isn't replicated. State 17 always
//!   gives order 0x15 (@0x8fafd).
//! - **Order 0x14's hold** ([`hold`], @0x881d8-0x883b1): its velocity eased
//!   toward slot 0's, then its position nudged into the band
//!   [`HOLD_NEAR`]-[`HOLD_FAR`] off slot 0 on each axis, both by one
//!   [`kill_step`].
//! - **Order 0x14's heading** ([`kill_heading`], @0x883b6-0x883ee):
//!   `_CalcObjectLeadAngle` at slot 0 for the weapon it holds (+0x72),
//!   read after the hold, and **its fire** ([`fires`], @0x88401-0x88443):
//!   facing that heading within its turn and [`FIRE_SLACK`], strictly, it
//!   clears its target and runs `_AIFireGunUntargeted`.
//! - **Order 0x15's box** ([`nearest_box`], @0x88459-0x884f0): the nearest
//!   scoopable box with life left, by whole squared distance.
//!
//! No `PlaySound` or `_beepSnd` call is in any of these blocks; a shot
//! fired sounds as the fight's.

use crate::cargo_box::CargoBox;
use crate::combat::weapon::WeaponSpec;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::traffic::npc::Npc;
use crate::trig;
use nova_sim_combat::asteroid::Asteroid;

/// The turns the mining box is worked out from, less the class's turn
/// (the float 10.0 @0xdd65c).
pub const BOX_TURNS: f32 = 10.0;
/// The pixels of mining box per turn (the float 15.0 @0xdda30).
pub const BOX_PER_TURN: f32 = 15.0;
/// How far beyond its turn, in degrees, a chasing or scooping miner still
/// thrusts (the float 15.0 @0xdda30).
pub const THRUST_SLACK: f32 = 15.0;
/// How far beyond its turn, in degrees, a miner holding by slot 0 still
/// fires (the float 10.0 @0xdd65c).
pub const FIRE_SLACK: f32 = 10.0;
/// How many accelerations a tick order 0x14 eases and nudges by (the
/// double 1.5 @0xdd640).
pub const KILL_EASE: f64 = 1.5;
/// The farthest a miner holds off slot 0 on each axis, in pixels (the
/// float 150.0 @0xdd724).
pub const HOLD_FAR: f32 = 150.0;
/// The nearest a miner holds off slot 0 on each axis, in pixels (the
/// float 80.0 @0xdda44).
pub const HOLD_NEAR: f32 = 80.0;
/// How many accelerations a tick a scooping miner off its heading brakes
/// by (the double 1.75 @0xdd6e8).
pub const SCOOP_BRAKE: f64 = 1.75;

/// The low-level order state 16's high level gives a miner (see the
/// module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MineOrder {
    /// Order 1: slot 0 is empty, so it stops.
    Stop,
    /// Order 0x13: outside the box, it chases slot 0.
    Chase,
    /// Order 0x14: inside the box, it holds by slot 0 and fires at it.
    Kill,
}

/// The mining box of a class whose turn (+0x38) is `class_turn`:
/// trunc((10 − turn) × 15), in floats (@0x8f840-0x8f873).
#[must_use]
pub fn mine_box(class_turn: f32) -> i32 {
    // Within an i32 for any class's turn.
    #[allow(clippy::cast_possible_truncation)]
    let side = ((BOX_TURNS - class_turn) * BOX_PER_TURN) as i32;
    side
}

/// The order state 16's high level gives `npc` with `rock` in slot 0
/// (see the module docs).
#[must_use]
pub fn order(npc: &Npc, rock: Option<&Asteroid>) -> MineOrder {
    let Some(rock) = rock else {
        return MineOrder::Stop;
    };
    // f32: the box is converted back to a float for the test.
    #[allow(clippy::cast_precision_loss)]
    let side = mine_box(npc.hull.class_turn) as f32;
    let off = npc.state.position - rock.position;
    if off.x.abs() <= side && off.y.abs() <= side {
        MineOrder::Kill
    } else {
        MineOrder::Chase
    }
}

/// Order 0x14's step for a ship of acceleration `accel`: accel ×
/// [`KILL_EASE`] × the frame scale, 1, in doubles (@0x881db-0x88207).
#[must_use]
pub fn kill_step(accel: f32) -> f32 {
    // Back to the float the original keeps.
    #[allow(clippy::cast_possible_truncation)]
    let step = (f64::from(accel) * KILL_EASE) as f32;
    step
}

/// Order 0x14's hold of a ship at `state` by `rock`, by `step`
/// ([`kill_step`]), in the original's order (@0x8820e-0x883b1): its
/// velocity, x then y, eased toward `rock`'s by `step`, snapping to it
/// within a step; then its position, x then y, pulled in by `step` when
/// more than [`HOLD_FAR`] off on that axis, strictly; then, on the
/// nudged position, pushed out by `step` when less than [`HOLD_NEAR`]
/// off, strictly, and not on `rock`'s own line.
pub fn hold(state: &mut ShipState, rock: &Asteroid, step: f32) {
    state.velocity = Vec2::new(
        ease(state.velocity.x, rock.velocity.x, step),
        ease(state.velocity.y, rock.velocity.y, step),
    );
    let pulled = Vec2::new(
        pull_in(state.position.x, rock.position.x, step),
        pull_in(state.position.y, rock.position.y, step),
    );
    state.position = Vec2::new(
        push_out(pulled.x, rock.position.x, step),
        push_out(pulled.y, rock.position.y, step),
    );
}

/// `v` eased toward `to` by `step`: down at `to + step` or more, up at
/// `to − step` or less, else `to` (@0x8821c-0x88249).
fn ease(v: f32, to: f32, step: f32) -> f32 {
    if v >= to + step {
        v - step
    } else if to - step >= v {
        v + step
    } else {
        to
    }
}

/// `p` pulled toward `at` by `step` when more than [`HOLD_FAR`] off,
/// strictly (@0x882ad-0x882da).
fn pull_in(p: f32, at: f32, step: f32) -> f32 {
    if p > at + HOLD_FAR {
        p - step
    } else if at - HOLD_FAR > p {
        p + step
    } else {
        p
    }
}

/// `p` pushed away from `at` by `step` when less than [`HOLD_NEAR`] off
/// and not on it, strictly (@0x88326-0x88369).
fn push_out(p: f32, at: f32, step: f32) -> f32 {
    if p > at && at + HOLD_NEAR > p {
        p + step
    } else if at > p && p > at - HOLD_NEAR {
        p - step
    } else {
        p
    }
}

/// The whole-degree heading order 0x14 wants of a ship at `state`, held
/// by `rock` ([`hold`]): `_CalcObjectLeadAngle` from its position at
/// `rock`, relative to its velocity, for `pick`, the weapon it holds
/// (+0x72), whose global record leads by its gate
/// ([`WeaponSpec::object_lead_shot`]); the table bearing with none
/// (@0x883b6-0x883ee).
#[must_use]
pub fn kill_heading(state: &ShipState, rock: &Asteroid, pick: Option<&WeaponSpec>) -> i16 {
    trig::object_lead_angle(
        state.position,
        state.velocity,
        (rock.position, rock.velocity),
        pick.and_then(WeaponSpec::object_lead_shot),
    )
}

/// Whether a ship heading `heading` faces `wanted` within its turn
/// `turn_rate` and `slack`, strictly: `_AngularDifference` of its whole
/// degree and `wanted`, as orders 0x13-0x15 test it (`jbe` @0x881b4,
/// @0x88435, @0x88573).
#[must_use]
pub fn faces(heading: f32, wanted: i16, turn_rate: f32, slack: f32) -> bool {
    let off = f32::from(trig::angular_difference(wanted, trig::whole(heading)).abs());
    off < turn_rate + slack
}

/// Whether a ship heading `heading`, its turn `turn_rate`, fires on order
/// 0x14 at the heading it wants `wanted`: within its turn and
/// [`FIRE_SLACK`], strictly (@0x88401-0x88435).
#[must_use]
pub fn fires(heading: f32, wanted: i16, turn_rate: f32) -> bool {
    faces(heading, wanted, turn_rate, FIRE_SLACK)
}

/// Where order 0x15 steers a ship at `at` among `boxes`: the scoopable
/// box with life above none nearest it, by its squared distance
/// truncated to a whole number, the earlier box on a tie, with no range
/// limit (@0x88459-0x884e6); none when no box qualifies, which leaves it
/// on order 0, coasting (@0x88636).
#[must_use]
pub fn nearest_box(at: Vec2, boxes: &[CargoBox]) -> Option<Vec2> {
    let mut nearest: Option<(Vec2, i32)> = None;
    for cargo_box in boxes {
        if cargo_box.life <= 0.0 || !cargo_box.scoopable {
            continue;
        }
        let off = cargo_box.position - at;
        // `cvttss2si` of `_XYFloatSquaredDist` (@0x884b2).
        #[allow(clippy::cast_possible_truncation)]
        let squared = (off.x * off.x + off.y * off.y) as i32;
        if nearest.is_none_or(|(_, best)| squared < best) {
            nearest = Some((cargo_box.position, squared));
        }
    }
    nearest.map(|(position, _)| position)
}

/// Order 0x15's brake step for a ship of acceleration `accel`: accel ×
/// [`SCOOP_BRAKE`] × the frame scale, 1, in doubles (@0x8858b-0x885ba).
#[must_use]
pub fn scoop_brake_step(accel: f32) -> f32 {
    // Back to the float the original keeps.
    #[allow(clippy::cast_possible_truncation)]
    let step = (f64::from(accel) * SCOOP_BRAKE) as f32;
    step
}

/// Order 0x15's brake of `velocity` by `step` ([`scoop_brake_step`]),
/// each axis toward none: down at `step` or more, up at `−step` or less,
/// else none (@0x885a1-0x8862d).
#[must_use]
pub fn brake(velocity: Vec2, step: f32) -> Vec2 {
    Vec2::new(ease(velocity.x, 0.0, step), ease(velocity.y, 0.0, step))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::combat::hull::class_turn;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use nova_sim_base::fixture::FAST;

    /// A live asteroid at (`x`, `y`), moving by (`vx`, `vy`).
    fn rock(x: f32, y: f32, vx: f32, vy: f32) -> Asteroid {
        Asteroid {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            frame: 0.0,
            spin: 0.0,
            strength: 30,
            kind: 0,
        }
    }

    /// A miner at (`x`, `y`), its class turn 2 (a box of 120).
    fn miner(x: f32, y: f32) -> Npc {
        let mut ship = npc(1, ShipStats::new(FAST, &[]));
        ship.hull.class_turn = 2.0;
        ship.state.position = Vec2::new(x, y);
        ship
    }

    #[test]
    fn the_box_is_ten_less_the_class_turn_times_fifteen_in_floats() {
        assert_eq!(mine_box(2.0), 120);
        assert_eq!(mine_box(8.0), 30);
        // trunc((10 − 3.3) × 15), in floats.
        assert_eq!(mine_box(class_turn(33)), 100);
        assert_eq!(mine_box(10.5), -7);
    }

    #[test]
    fn with_slot_0_empty_the_order_is_a_stop() {
        assert_eq!(order(&miner(0.0, 0.0), None), MineOrder::Stop);
    }

    #[test]
    fn within_the_box_on_both_axes_inclusive_it_kills_and_else_chases() {
        let at = rock(1000.0, 1000.0, 0.0, 0.0);
        let order_at = |x: f32, y: f32| order(&miner(x, y), Some(&at));
        assert_eq!(order_at(1120.0, 880.0), MineOrder::Kill, "on the edge");
        assert_eq!(order_at(1000.0, 1000.0), MineOrder::Kill);
        assert_eq!(order_at(1120.5, 1000.0), MineOrder::Chase);
        assert_eq!(order_at(1000.0, 879.5), MineOrder::Chase);
        assert_eq!(order_at(0.0, 0.0), MineOrder::Chase);
    }

    #[test]
    fn the_kill_step_is_one_and_a_half_accelerations_in_doubles() {
        assert_eq!(kill_step(0.2), (0.2_f64 * 1.5) as f32);
        assert_eq!(kill_step(1.0), 1.5);
    }

    fn held(state: ShipState, at: &Asteroid, step: f32) -> ShipState {
        let mut state = state;
        hold(&mut state, at, step);
        state
    }

    fn at(x: f32, y: f32, vx: f32, vy: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading: 0.0,
        }
    }

    #[test]
    fn the_hold_eases_each_axis_of_the_velocity_and_snaps_within_a_step() {
        let slot0 = rock(0.0, 0.0, 1.0, -1.0);
        let state = held(at(100.0, 100.0, 3.0, -3.0), &slot0, 0.5);
        assert_eq!(state.velocity, Vec2::new(2.5, -2.5));
        let state = held(at(100.0, 100.0, -1.0, 1.0), &slot0, 0.5);
        assert_eq!(state.velocity, Vec2::new(-0.5, 0.5));
        let state = held(at(100.0, 100.0, 1.4, -0.6), &slot0, 0.5);
        assert_eq!(state.velocity, Vec2::new(1.0, -1.0), "snapped");
        let state = held(at(100.0, 100.0, 1.5, -1.5), &slot0, 0.5);
        assert_eq!(state.velocity, Vec2::new(1.0, -1.0), "a step off eases on");
    }

    #[test]
    fn the_hold_pulls_in_beyond_150_and_leaves_150_itself() {
        let slot0 = rock(0.0, 0.0, 0.0, 0.0);
        let state = held(at(151.0, -151.0, 0.0, 0.0), &slot0, 0.5);
        assert_eq!(state.position, Vec2::new(150.5, -150.5));
        let state = held(at(-150.0, 150.0, 0.0, 0.0), &slot0, 0.5);
        assert_eq!(state.position, Vec2::new(-150.0, 150.0), "strict");
        let state = held(at(100.0, 120.0, 0.0, 0.0), &slot0, 0.5);
        assert_eq!(state.position, Vec2::new(100.0, 120.0), "in the band");
    }

    #[test]
    fn the_hold_pushes_out_within_80_off_its_line_and_leaves_80_and_the_line() {
        let slot0 = rock(10.0, 10.0, 0.0, 0.0);
        let state = held(at(50.0, -30.0, 0.0, 0.0), &slot0, 0.5);
        assert_eq!(state.position, Vec2::new(50.5, -30.5));
        let state = held(at(90.0, -70.0, 0.0, 0.0), &slot0, 0.5);
        assert_eq!(state.position, Vec2::new(90.0, -70.0), "80 off: strict");
        let state = held(at(10.0, 100.0, 0.0, 0.0), &slot0, 0.5);
        assert_eq!(state.position, Vec2::new(10.0, 100.0), "on its line");
    }

    #[test]
    fn the_hold_pushes_on_the_position_it_pulled() {
        // x is pulled from 150.5 to 50.5, then pushed out from there;
        // y, inside 80 and not pulled, is pushed out.
        let slot0 = rock(0.0, 0.0, 0.0, 0.0);
        let state = held(at(150.5, 79.5, 0.0, 0.0), &slot0, 100.0);
        // x: 150.5 → 50.5, then within 80: pushed back out to 150.5.
        assert_eq!(state.position, Vec2::new(150.5, 179.5));
    }

    #[test]
    fn the_kill_heading_leads_slot_0_for_the_pick_and_else_takes_its_bearing() {
        let slot0 = rock(100.0, 0.0, 0.0, 5.0);
        let state = at(0.0, 0.0, 0.0, 0.0);
        assert_eq!(kill_heading(&state, &slot0, None), 90);
        let gun = WeaponSpec::new(&crate::catalog::WeaponRecord {
            guidance: -1,
            speed: 1000,
            ..nova_sim_catalog::fixture::weapon(128)
        });
        assert_eq!(
            kill_heading(&state, &slot0, Some(&gun)),
            trig::table_arctan(-100.0, -50.0)
        );
        let beam = WeaponSpec::new(&crate::catalog::WeaponRecord {
            guidance: 0,
            speed: 1000,
            ..nova_sim_catalog::fixture::weapon(128)
        });
        assert_eq!(kill_heading(&state, &slot0, Some(&beam)), 90, "a beam");
        let along = at(0.0, 0.0, 0.0, 5.0);
        assert_eq!(
            kill_heading(&along, &slot0, Some(&gun)),
            90,
            "moving with it"
        );
    }

    #[test]
    fn it_fires_within_its_turn_and_ten_strictly() {
        assert!(fires(0.0, 12, 3.0));
        assert!(!fires(0.0, 13, 3.0), "13 is not under 13");
        assert!(fires(359.9, 9, 0.5), "its whole degree, across north");
        assert!(!fires(359.9, 11, 0.5));
    }

    #[test]
    fn it_thrusts_within_its_turn_and_fifteen_strictly() {
        assert!(faces(90.0, 107, 3.0, THRUST_SLACK));
        assert!(!faces(90.0, 108, 3.0, THRUST_SLACK));
        assert!(!faces(90.0, 72, 3.0, THRUST_SLACK));
    }

    fn cargo(x: f32, y: f32, life: f32, scoopable: bool) -> CargoBox {
        CargoBox {
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            life,
            frame: 0,
            spin: 0,
            graphic: 1,
            scoopable,
            contents: Some(0),
        }
    }

    #[test]
    fn the_nearest_box_is_scoopable_with_life_left_at_any_range() {
        let boxes = [
            cargo(10.0, 0.0, 50.0, false),
            cargo(0.0, 20.0, 0.0, true),
            cargo(5000.0, 0.0, 5.0, true),
            cargo(-3000.0, 0.0, 5.0, true),
        ];
        assert_eq!(
            nearest_box(Vec2::ZERO, &boxes),
            Some(Vec2::new(-3000.0, 0.0))
        );
        assert_eq!(nearest_box(Vec2::ZERO, &boxes[..2]), None);
        assert_eq!(nearest_box(Vec2::ZERO, &[]), None);
    }

    #[test]
    fn a_tie_on_the_whole_squared_distance_goes_to_the_earlier_box() {
        // 10.01² = 100.2 and 10.03² = 100.6: both 100.
        let boxes = [cargo(0.0, 10.03, 5.0, true), cargo(10.01, 0.0, 5.0, true)];
        assert_eq!(nearest_box(Vec2::ZERO, &boxes), Some(Vec2::new(0.0, 10.03)));
        let boxes = [cargo(0.0, 11.0, 5.0, true), cargo(10.01, 0.0, 5.0, true)];
        assert_eq!(
            nearest_box(Vec2::ZERO, &boxes),
            Some(Vec2::new(10.01, 0.0)),
            "121 then 100"
        );
    }

    #[test]
    fn the_brake_takes_each_axis_toward_none_by_its_step() {
        assert_eq!(scoop_brake_step(0.2), (0.2_f64 * 1.75) as f32);
        assert_eq!(brake(Vec2::new(2.0, -2.0), 0.5), Vec2::new(1.5, -1.5));
        assert_eq!(brake(Vec2::new(0.4, -0.5), 0.5), Vec2::new(0.0, 0.0));
        assert_eq!(brake(Vec2::new(0.5, -0.4), 0.5), Vec2::new(0.0, 0.0));
    }
}
