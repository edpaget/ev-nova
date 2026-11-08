//! Shots, cargo boxes and asteroids drawn between passes back along the
//! move they last made: `velocity × g` at the last pass's gain
//! ([`PassReport::speed_mult`]), as `flight::moved` moved them. At full
//! gain that is the whole velocity, as before the gain; at zero they are
//! drawn where they are.

use nova_sim::SpeedMult;
use nova_sim::flight_frame::{FrameAsteroid, FrameBox, FrameShot, PassReport, StepReport};

use super::stepping::{fight, frame, one_step, stepping};
use super::*;
use crate::system::camera::Camera;

const HALF: SpeedMult = SpeedMult::new(0.5);
const AT: Vec2 = Vec2::new(100.0, 50.0);
const VELOCITY: Vec2 = Vec2::new(8.0, -4.0);

/// A shot with no sheet, a box and an asteroid, all at [`AT`] moving
/// [`VELOCITY`].
fn moving() -> FlightFrame {
    FlightFrame {
        shots: vec![FrameShot {
            weapon: WeaponId(9999),
            position: AT,
            velocity: VELOCITY,
            ..record::shot()
        }],
        cargo_boxes: vec![FrameBox {
            velocity: VELOCITY,
            ..super::faked_world::box_at(AT)
        }],
        asteroids: vec![FrameAsteroid {
            velocity: VELOCITY,
            ..super::faked_world::asteroid_at(AT)
        }],
        ..sol()
    }
}

/// A pass at `gain`.
fn at(gain: SpeedMult) -> PassReport {
    PassReport {
        speed_mult: gain,
        ..fight(vec![])
    }
}

/// A view over [`moving`] after a frame of `report`.
fn after(report: StepReport) -> Faked {
    let mut view = stepping(armed(1, &[BLASTER]));
    view.sim.set_frame(moving());
    frame(&mut view, report);
    view
}

/// Asserts all three are drawn back `share` of [`VELOCITY`] per unit of
/// `1 − alpha`, for each alpha.
fn drawn_back(view: &mut Faked, share: f32) {
    let camera = Camera::centred_on_in(Point::new(0.0, 0.0), VIEW_SIZE);
    for alpha in [1.0_f32, 0.5, 0.0] {
        view.alpha = alpha;
        let back = share * (1.0 - alpha);
        let drawn = camera.whole_point(Point::new(
            AT.x - VELOCITY.x * back,
            AT.y - VELOCITY.y * back,
        ));
        assert_eq!(view.shown_shots(&camera)[0].at, drawn, "shot {alpha}");
        assert_eq!(view.shown_boxes(&camera)[0].at, drawn, "box {alpha}");
        assert_eq!(view.shown_asteroids(&camera)[0].at, drawn, "rock {alpha}");
    }
}

#[test]
fn below_full_gain_shots_boxes_and_asteroids_are_drawn_back_along_the_move_they_made() {
    let mut view = after(one_step(vec![at(HALF)]));
    drawn_back(&mut view, 0.5);
}

#[test]
fn at_full_gain_they_are_drawn_where_they_were() {
    let mut view = after(one_step(vec![at(SpeedMult::FULL)]));
    drawn_back(&mut view, 1.0);
}

#[test]
fn at_zero_gain_they_are_drawn_where_they_are() {
    let mut view = after(one_step(vec![at(SpeedMult::ZERO)]));
    drawn_back(&mut view, 0.0);
}

#[test]
fn the_last_pass_gain_holds_until_the_next_pass() {
    let mut view = after(one_step(vec![at(HALF)]));
    frame(&mut view, StepReport::default());
    drawn_back(&mut view, 0.5);
    frame(&mut view, one_step(vec![at(SpeedMult::FULL)]));
    drawn_back(&mut view, 1.0);
    let mut view = after(one_step(vec![at(SpeedMult::FULL), at(HALF)]));
    drawn_back(&mut view, 0.5);
}
