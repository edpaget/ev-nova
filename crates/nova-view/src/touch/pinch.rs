//! Pinch to zoom: the fingers' spread, counted in the map's own zoom
//! steps.
//!
//! The reference is the two fingers' distance as the second finger
//! lands, floored at one logical unit. With `x = ln(d / d0) / ln(4/3)`
//! and `n` the steps taken in this pinch (0 at its start), a move emits
//! [`Pinch::ZoomIn`] while `x ≥ n + 1` and [`Pinch::ZoomOut`] while
//! `x ≤ n − 1`, one step each, several for one move if the fingers moved
//! that far.
//!
//! - **One step per 4/3 of spread**, because both maps zoom in steps of
//!   4/3 (the viewer's [`SCALES`](crate::galaxy::view::SCALES), the
//!   in-flight maps' [`ZOOM_OUT_STEP`](crate::galaxy::place::ZOOM_OUT_STEP)),
//!   so the map never zooms further than the fingers moved.
//! - **One step of hysteresis**: a finger trembling at a boundary steps
//!   once and never toggles, and bringing the fingers back to where they
//!   started brings the count back to zero.
//! - **Spreading apart zooms in** (the map grows), as `=` does.
//!
//! The steps are counted for the gesture. The map clamps them at its own
//! zoom limits (`galaxy::touch`), so a step it refuses is not undone by
//! the pinch turning round.

use crate::input::Pinch;

/// The spread, as a ratio, that makes one zoom step: the maps' own zoom
/// step.
pub const PINCH_STEP: f64 = 4.0 / 3.0;

/// How far short of a whole step a spread may fall and still take it,
/// so a spread of exactly a power of 4/3 isn't lost to rounding.
const ROUNDING: f64 = 1e-6;

/// A pinch's zoom steps: its reference spread and the steps taken.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinchSteps {
    /// The fingers' distance as the pinch began, at least one unit.
    reference: f64,
    /// The steps taken, in less out.
    taken: i32,
}

impl PinchSteps {
    /// A pinch whose fingers were `distance` apart as it began.
    #[must_use]
    pub fn new(distance: f32) -> Self {
        Self {
            reference: f64::from(distance).max(1.0),
            taken: 0,
        }
    }

    /// The steps the fingers spreading or closing to `distance` apart
    /// make, in order.
    pub fn spread(&mut self, distance: f32) -> Vec<Pinch> {
        let x = (f64::from(distance).max(1.0) / self.reference).ln() / PINCH_STEP.ln();
        let mut steps = Vec::new();
        while x >= f64::from(self.taken + 1) - ROUNDING {
            self.taken += 1;
            steps.push(Pinch::ZoomIn);
        }
        while x <= f64::from(self.taken - 1) + ROUNDING {
            self.taken -= 1;
            steps.push(Pinch::ZoomOut);
        }
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::galaxy::{place::ZOOM_OUT_STEP, view::SCALES};

    const D0: f32 = 90.0;

    /// `D0` spread by `steps` pinch steps.
    #[allow(clippy::cast_possible_truncation)]
    fn spread_by(steps: f64) -> f32 {
        (f64::from(D0) * PINCH_STEP.powf(steps)) as f32
    }

    #[test]
    fn spreading_by_four_thirds_zooms_in_once() {
        let mut pinch = PinchSteps::new(D0);
        assert_eq!(pinch.spread(120.0), [Pinch::ZoomIn]);
        assert_eq!(
            pinch.spread(120.0),
            [],
            "no further step at the same spread"
        );
    }

    #[test]
    fn just_short_of_four_thirds_does_nothing() {
        let mut pinch = PinchSteps::new(D0);
        assert_eq!(pinch.spread(119.9), []);
        assert_eq!(pinch.spread(67.6), [], "just short of three quarters");
    }

    #[test]
    fn spreading_two_steps_in_one_move_zooms_in_twice() {
        let mut pinch = PinchSteps::new(D0);
        assert_eq!(pinch.spread(spread_by(2.0)), [Pinch::ZoomIn, Pinch::ZoomIn]);
    }

    #[test]
    fn pinching_together_by_three_quarters_zooms_out_once() {
        let mut pinch = PinchSteps::new(D0);
        assert_eq!(pinch.spread(67.5), [Pinch::ZoomOut]);
        assert_eq!(
            pinch.spread(spread_by(-3.0)),
            [Pinch::ZoomOut, Pinch::ZoomOut]
        );
    }

    #[test]
    fn back_to_the_starting_spread_zooms_back_to_the_start() {
        let mut pinch = PinchSteps::new(D0);
        assert_eq!(pinch.spread(spread_by(2.5)), [Pinch::ZoomIn, Pinch::ZoomIn]);
        assert_eq!(pinch.spread(D0), [Pinch::ZoomOut, Pinch::ZoomOut]);
        assert_eq!(pinch.spread(spread_by(-1.5)), [Pinch::ZoomOut]);
        assert_eq!(pinch.spread(D0), [Pinch::ZoomIn]);
    }

    #[test]
    fn trembling_at_a_boundary_steps_once() {
        let mut pinch = PinchSteps::new(D0);
        let steps: Vec<_> = [0.99, 1.0, 0.99, 1.01, 0.99]
            .into_iter()
            .flat_map(|x| pinch.spread(spread_by(x)))
            .collect();
        assert_eq!(steps, [Pinch::ZoomIn]);
    }

    #[test]
    fn a_start_closer_than_one_unit_counts_as_one() {
        let mut pinch = PinchSteps::new(0.0);
        assert_eq!(pinch.spread(0.5), [], "a spread under one unit is one");
        assert_eq!(pinch.spread(1.34), [Pinch::ZoomIn]);
        let mut close = PinchSteps::new(0.2);
        assert_eq!(close.spread(1.32), [], "the reference is one unit, not 0.2");
    }

    #[test]
    fn the_pinch_step_is_the_maps_zoom_step() {
        for pair in SCALES.windows(2) {
            assert!(
                (f64::from(pair[1] / pair[0]) - PINCH_STEP).abs() < 1e-6,
                "{pair:?}"
            );
        }
        assert!((ZOOM_OUT_STEP - PINCH_STEP).abs() < 1e-6);
    }
}
