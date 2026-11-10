//! A ship's 3D model's pose: [`ship_orientation`], how a ship's model is
//! turned for the heading the view shows it at.
//!
//! The model turns about the view axis (z, toward the player) by its
//! hull's angle, which the simulation decides
//! (`nova_sim::combat::ship_frames::FrameSpec::hull_angle`, as the frame's
//! `hull_turn` says): by the engine reading of the rulebook's
//! `RuleKey::ContinuousHeadings`, the angle of the frame its sprite is
//! drawn on, so the model steps a frame at a time; by the other reading
//! the heading itself, so it turns smoothly. A positive turn about z is
//! clockwise on screen, as headings turn (see [`model`](crate::model)), so
//! the model's nose, up the screen (−y) at heading 0, faces the heading.
//!
//! This is the pose a screen draws a ship's [`DrawCommand::Model`] at,
//! and its outfits on their hardpoints with it
//! ([`mounted_outfits`](super::mounted_outfits())): the flight screen
//! draws a new game's ships so (its view's `ship_models` module).
//!
//! [`DrawCommand::Model`]: crate::DrawCommand::Model

use nova_sim::combat::hull_turn::HullTurn;
use nova_sim::combat::ship_frames::FrameSpec;

use crate::model::Orientation;

/// The view axis, toward the player, that a ship's model turns about.
const VIEW_AXIS: [f32; 3] = [0.0, 0.0, 1.0];

/// How the model of a ship of display `spec`, shown facing `heading`, is
/// turned, its hull turning as `turn` says: a turn about the view axis by
/// its hull's angle (see the module docs).
#[must_use]
pub fn ship_orientation(spec: &FrameSpec, heading: f32, turn: HullTurn) -> Orientation {
    Orientation::about(VIEW_AXIS, spec.hull_angle(heading, turn).to_radians())
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use super::*;

    /// Hand-built displays: the stock 36 frames, and 64.
    fn specs() -> [FrameSpec; 2] {
        [
            FrameSpec::NONE,
            FrameSpec {
                frames: NonZeroU16::new(64).expect("64"),
                ..FrameSpec::NONE
            },
        ]
    }

    #[test]
    fn a_model_turns_about_the_view_axis_by_its_hulls_angle_under_either_reading() {
        for spec in specs() {
            for turn in [HullTurn::ByFrame, HullTurn::Continuous] {
                for heading in [0.0, 4.5, 12.5, 97.3, 181.0, 359.9] {
                    let angle = spec.hull_angle(heading, turn);
                    assert_eq!(
                        ship_orientation(&spec, heading, turn),
                        Orientation::about(VIEW_AXIS, angle.to_radians()),
                        "{} frames, {turn:?}, {heading}",
                        spec.frames
                    );
                }
            }
        }
    }

    #[test]
    fn a_positive_hull_angle_turns_the_models_nose_clockwise_on_screen() {
        // Facing right: the nose, up the screen at 0, points along +x.
        let turned = ship_orientation(&FrameSpec::NONE, 90.0, HullTurn::Continuous);
        let nose = turned.rotate([0.0, -1.0, 0.0]).expect("turns");
        for (got, want) in nose.iter().zip([1.0, 0.0, 0.0]) {
            assert!((got - want).abs() < 1e-5, "{nose:?}");
        }
    }
}
