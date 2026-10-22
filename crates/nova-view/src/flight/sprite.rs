//! Which frame of a ship's sprite sheet shows its heading.
//!
//! A ship's sheet holds one frame per rotation in each set, as many as
//! its `shän`'s frames per rotation. Frame 0 faces up and the frames turn
//! clockwise in equal steps, as Nova's ship sheets do. Flight draws the
//! set the session's display picked on top of it (see the
//! [`ship_layers`](super::ship_layers) module).
//!
//! The frame is the simulation's ([`drawn_frame`]): the heading's share of
//! a turn truncated, as `_HandleShipDisplay` picks it, so the ship is
//! drawn on the frame its weapon exit points turn with.

use std::num::NonZeroU16;

use nova_sim::combat::exits::drawn_frame;

/// The frame of set 0 a ship facing `heading_deg` (degrees clockwise from
/// up, any value) is drawn on, of a sheet of `rotations` frames a turn.
#[must_use]
pub fn rotation_frame(heading_deg: f32, rotations: NonZeroU16) -> u16 {
    drawn_frame(heading_deg, rotations, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rotations(n: u16) -> NonZeroU16 {
        NonZeroU16::new(n).expect("non-zero")
    }

    fn frames(headings: &[f32], n: u16) -> Vec<u16> {
        headings
            .iter()
            .map(|&h| rotation_frame(h, rotations(n)))
            .collect()
    }

    #[test]
    fn a_36_frame_ship_turns_a_frame_every_10_degrees() {
        assert_eq!(
            frames(&[0.0, 10.0, 90.0, 180.0, 270.0, 350.0], 36),
            [0, 1, 9, 18, 27, 35]
        );
    }

    #[test]
    fn the_frame_truncates_the_heading_as_the_original_draws_it() {
        assert_eq!(
            frames(&[4.9, 5.1, 9.9, 10.0, 14.9, 15.1, 354.9, 355.0, 359.9], 36),
            [0, 0, 0, 1, 1, 1, 35, 35, 35]
        );
    }

    #[test]
    fn a_4_frame_ship_turns_a_frame_every_quarter() {
        assert_eq!(
            frames(
                &[
                    0.0, 44.0, 46.0, 89.9, 90.0, 180.0, 270.0, 314.0, 316.0, 359.9
                ],
                4
            ),
            [0, 0, 0, 0, 1, 2, 3, 3, 3, 3]
        );
        assert_eq!(frames(&[0.0, 90.0, 270.0], 1), [0, 0, 0]);
    }

    #[test]
    fn headings_outside_0_to_360_wrap() {
        assert_eq!(frames(&[-10.0, 370.0, 720.0, -350.0], 36), [35, 1, 0, 1]);
    }

    #[test]
    fn the_view_draws_the_frame_the_exits_turn_with() {
        for n in [1, 4, 36, 48, 64] {
            for tenth in 0..3600 {
                let heading = tenth as f32 / 10.0;
                assert_eq!(
                    rotation_frame(heading, rotations(n)),
                    nova_sim::combat::exits::drawn_frame(heading, rotations(n), None),
                    "{heading} of {n}"
                );
            }
        }
    }
}
