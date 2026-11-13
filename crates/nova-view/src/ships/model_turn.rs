//! A ship class's 3D model turning in the ship browser, for a class with
//! no sprite sheet whose model the world models' table holds
//! (rdm:phase/rendering-3d/phase-17-ship-browser-models).
//!
//! The browser checks how a class looks in flight, so the model turns in
//! flight's own top-down pose ([`ship_orientation`]), about the view axis
//! with no tilt, rather than the tilted still pose of a screen's picture.
//! It steps as a sheet steps here: one of [`MODEL_STEPS`] headings a tick
//! of Nova's 1/30 s clock, the frames flight gives a class whose `shän`
//! has none (a new game's neutral one), by the engine reading
//! ([`HullTurn::ByFrame`]). Selecting a ship restarts the turn, nose up.
//!
//! It is fitted, as every model picture is
//! ([`ShipPortrait::draw_posed_in`]), to the placeholder's square on
//! [`SHIP_CENTER`] ([`model_area`]), and drawn whole, with its class's
//! as-built parts; a sprite's glow and lights have no counterpart on a
//! model. Under it, where the placeholder's message would go, a dim line
//! says why the class has no sprite.

use std::time::Duration;

use nova_sim::combat::hull_turn::HullTurn;
use nova_sim::combat::ship_frames::FrameSpec;

use super::browser::{
    MESSAGE_SIZE, PLACEHOLDER_SIZE, SHIP_CENTER, SPRITE_ERROR_WRAP, message_origin,
};
use crate::flight::ship_orientation;
use crate::geometry::{Bounds, Point};
use crate::model::Orientation;
use crate::model_picture::ShipPortrait;
use crate::time::ticks;
use crate::{Color, DrawList};

/// How many headings a model turns through: the frames flight reads for a
/// class with no frames of its own (`FrameSpec::NONE`), so a model here
/// steps as it does in flight.
pub(super) const MODEL_STEPS: u16 = FrameSpec::NONE.frames.get();

/// The step shown `elapsed` after the selection: one per tick of Nova's
/// 1/30 s clock, wrapping round after [`MODEL_STEPS`].
pub(super) fn model_step(elapsed: Duration) -> u16 {
    (ticks(elapsed) % u128::from(MODEL_STEPS)) as u16
}

/// The model's pose at `step`: flight's, at the heading `step` is of
/// [`MODEL_STEPS`] equal shares of a turn clockwise from nose up.
pub(super) fn model_orientation(step: u16) -> Orientation {
    let heading = f32::from(step) * 360.0 / f32::from(MODEL_STEPS);
    ship_orientation(&FrameSpec::NONE, heading, HullTurn::ByFrame)
}

/// Where the model is fitted: the placeholder's square, centred on
/// [`SHIP_CENTER`].
pub(super) fn model_area() -> Bounds {
    let half = PLACEHOLDER_SIZE / 2.0;
    Bounds::at(
        Point::new(SHIP_CENTER.x - half, SHIP_CENTER.y - half),
        PLACEHOLDER_SIZE,
        PLACEHOLDER_SIZE,
    )
}

/// Draws `portrait` turned to `step` in [`model_area`], its parts after
/// it, then, dim, why the class has no sprite (`message`); says whether it
/// drew. A portrait that can't be fitted draws nothing, not even the
/// message, so the caller draws the placeholder instead.
pub(super) fn draw_model(
    portrait: &ShipPortrait,
    step: u16,
    message: &str,
    list: &mut DrawList,
) -> bool {
    if !portrait.draw_posed_in(list, model_area(), model_orientation(step)) {
        return false;
    }
    list.text(
        format!("3D model (sprite unavailable: {message})"),
        message_origin(),
        MESSAGE_SIZE,
        Some(SPRITE_ERROR_WRAP),
        Color::DIM,
    );
    true
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use nova_data::store::{Footing, Mount, Mounts, Shown};
    use nova_sim::ShipId;
    use nova_sim::combat::hull_turn::HullTurn;
    use nova_sim::combat::ship_frames::FrameSpec;

    use super::*;
    use crate::flight::ship_orientation;
    use crate::flight::world_models::{ShipModel, WorldModels};
    use crate::geometry::{Bounds, Point};
    use crate::model::ModelKey;
    use crate::model_picture::{FIT, ShipPortrait};
    use crate::ships::browser::SHIP_CENTER;
    use crate::{Color, DrawCommand, DrawList};

    /// `n` ticks of Nova's 1/30 s clock.
    fn ticks(n: u32) -> Duration {
        Duration::from_secs(1) / 30 * n
    }

    #[track_caller]
    fn assert_near(a: [f32; 3], b: [f32; 3]) {
        for (p, q) in a.iter().zip(&b) {
            assert!((p - q).abs() < 1e-5, "{a:?} is not {b:?}");
        }
    }

    #[test]
    fn a_model_turns_in_flight_s_steps_for_a_class_with_no_sheet() {
        assert_eq!(MODEL_STEPS, FrameSpec::NONE.frames.get());
    }

    #[test]
    fn the_step_advances_one_per_tick_and_wraps() {
        let steps = u32::from(MODEL_STEPS);
        assert_eq!(model_step(Duration::ZERO), 0);
        assert_eq!(model_step(Duration::from_millis(20)), 0);
        assert_eq!(model_step(ticks(1)), 1);
        assert_eq!(model_step(ticks(steps - 1)), MODEL_STEPS - 1);
        assert_eq!(model_step(ticks(steps)), 0);
        assert_eq!(model_step(ticks(steps + 1)), 1);
        assert_eq!(model_step(ticks(3001)), (3001 % steps) as u16);
    }

    #[test]
    fn each_step_is_flight_s_pose_at_its_share_of_a_turn() {
        for step in 0..MODEL_STEPS {
            let heading = f32::from(step) * 360.0 / f32::from(MODEL_STEPS);
            assert_eq!(
                model_orientation(step),
                ship_orientation(&FrameSpec::NONE, heading, HullTurn::ByFrame),
                "{step}"
            );
        }
    }

    #[test]
    fn each_step_turns_the_nose_clockwise_by_its_share_of_a_turn_with_no_tilt() {
        for step in 0..MODEL_STEPS {
            let turn = (f32::from(step) * 360.0 / f32::from(MODEL_STEPS)).to_radians();
            let pose = model_orientation(step);
            let nose = pose.rotate([0.0, -1.0, 0.0]).expect("a turn");
            // Up the screen at step 0, right a quarter turn on, down half.
            assert_near(nose, [turn.sin(), -turn.cos(), 0.0]);
            let deck = pose.rotate([0.0, 0.0, 1.0]).expect("a turn");
            assert_near(deck, [0.0, 0.0, 1.0]);
        }
        let quarter = model_orientation(MODEL_STEPS / 4);
        assert_near(
            quarter.rotate([0.0, -1.0, 0.0]).expect("a turn"),
            [1.0, 0.0, 0.0],
        );
        let half = model_orientation(MODEL_STEPS / 2);
        assert_near(
            half.rotate([0.0, -1.0, 0.0]).expect("a turn"),
            [0.0, 1.0, 0.0],
        );
    }

    #[test]
    fn the_area_is_the_placeholder_s_square_on_the_ship_s_centre() {
        assert_eq!(
            model_area(),
            Bounds::at(Point::new(176.0, 256.0), 128.0, 128.0)
        );
        assert_eq!(model_area().center(), SHIP_CENTER);
    }

    /// Ship 128's model, one hull mount showing a gun.
    fn portrait() -> ShipPortrait {
        let ship = ShipId(128);
        let models = WorldModels {
            ships: BTreeMap::from([(
                ship,
                ShipModel {
                    key: ModelKey(7),
                    scale: 3.0,
                    base_transp: 12,
                    radius: 4.0,
                },
            )]),
            mounts: Mounts {
                hulls: BTreeMap::from([(
                    ship,
                    vec![Mount {
                        hardpoint: "nose".to_owned(),
                        at: [0.0, 2.0, 0.5],
                        footing: Footing::Surface,
                        turns: false,
                        shows: Some(Shown {
                            asset: "gun".to_owned(),
                            lift: 0.25,
                        }),
                    }],
                )]),
                kits: BTreeMap::new(),
            },
            outfits: BTreeMap::from([("gun".to_owned(), ModelKey(9))]),
            ..WorldModels::default()
        };
        ShipPortrait::of(&models, ship).expect("a portrait")
    }

    #[test]
    fn the_model_draws_its_hull_fitted_in_the_area_then_its_parts_then_why_no_sprite() {
        let portrait = portrait();
        let step = MODEL_STEPS / 4;
        let mut list = DrawList::new();
        assert!(draw_model(&portrait, step, "no shän", &mut list));
        let mut want = DrawList::new();
        assert!(portrait.draw_posed_in(&mut want, model_area(), model_orientation(step)));
        want.text(
            "3D model (sprite unavailable: no shän)",
            Point::new(48.0, 412.0),
            16.0,
            Some(384.0),
            Color::DIM,
        );
        let got: Vec<_> = list.iter().cloned().collect();
        assert_eq!(got, want.iter().cloned().collect::<Vec<_>>());
        assert_eq!(got.len(), 3, "the hull, the gun and the text");
        let DrawCommand::Model {
            model,
            pose,
            alpha,
            colorize,
        } = &got[0]
        else {
            panic!("the hull first: {got:?}");
        };
        assert_eq!(*model, ModelKey(7));
        assert_eq!(pose.scale, FIT * 64.0 / 4.0);
        assert_eq!(pose.center, SHIP_CENTER);
        assert_eq!(*alpha, 1.0, "its BaseTransp is ignored");
        assert_eq!(*colorize, None);
        assert!(matches!(
            got[1],
            DrawCommand::Mounted {
                model: ModelKey(9),
                ..
            }
        ));
        assert!(
            !got.iter().any(|c| matches!(c, DrawCommand::Line { .. })),
            "no crossed box"
        );
    }

    #[test]
    fn a_model_that_cannot_be_fitted_draws_nothing() {
        let huge = ShipPortrait::bare(ModelKey(7), f32::MIN_POSITIVE);
        let mut list = DrawList::new();
        assert!(!draw_model(&huge, 0, "no shän", &mut list));
        assert_eq!(list.iter().count(), 0);
    }
}
