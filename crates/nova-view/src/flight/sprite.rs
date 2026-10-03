//! Which frame of a ship's sprite sheet shows its heading.
//!
//! A ship's sheet holds one frame per rotation in each set, as many as
//! its `shän`'s frames per rotation. Frame 0 faces up and the frames turn
//! clockwise in equal steps, as Nova's ship sheets do. Flight shows set 0.

use std::num::NonZeroU16;

/// The frame of set 0 nearest to heading `heading_deg` (degrees clockwise
/// from up, any value) on a sheet of `rotations` frames a turn.
#[must_use]
pub fn rotation_frame(heading_deg: f32, rotations: NonZeroU16) -> u16 {
    let count = u32::from(rotations.get());
    let per_frame = 360.0 / count as f32;
    let nearest = (heading_deg.rem_euclid(360.0) / per_frame).round() as u32;
    // Just short of a full turn rounds up to `count`: frame 0 again.
    (nearest % count) as u16
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
    fn the_nearest_frame_shows_wrapping_back_to_0() {
        assert_eq!(
            frames(&[4.9, 5.1, 14.9, 15.1, 354.9, 355.0, 359.9], 36),
            [0, 1, 1, 2, 35, 0, 0]
        );
    }

    #[test]
    fn a_4_frame_ship_turns_a_frame_every_quarter() {
        assert_eq!(
            frames(&[0.0, 44.0, 46.0, 90.0, 180.0, 270.0, 314.0, 316.0], 4),
            [0, 0, 1, 1, 2, 3, 3, 0]
        );
        assert_eq!(frames(&[0.0, 90.0, 270.0], 1), [0, 0, 0]);
    }

    #[test]
    fn headings_outside_0_to_360_wrap() {
        assert_eq!(frames(&[-10.0, 370.0, 720.0, -350.0], 36), [35, 1, 0, 1]);
    }
}
