//! How a ship's hull turns with its heading, [`HullTurn`], and the angle
//! it stands at ([`hull_angle`]), from
//! `nova_sim_combat::combat::hull_turn`, which re-exports them.

use std::num::NonZeroU16;

use nova_sim_base::flight::normalized;
use nova_sim_base::rulebook::RuleSource;

use super::exits::{drawn_frame, frame_angle};
use super::ship_frames::FrameTable;

/// How a ship's hull turns with its heading (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HullTurn {
    /// The original's: by the frame its sprite is drawn on.
    #[default]
    ByFrame,
    /// The new game's: with the heading itself.
    Continuous,
}

impl From<RuleSource> for HullTurn {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::ByFrame,
            RuleSource::Bible => Self::Continuous,
        }
    }
}

/// The angle, in degrees clockwise from up, at which the hull of a ship
/// facing `heading`, of `frames` a turn and its frame-angle `table` if it
/// has one, stands as `turn` says: by frame its drawn frame's whole-degree
/// angle, continuously the heading itself, wrapped into 0-360.
#[must_use]
pub fn hull_angle(
    heading: f32,
    frames: NonZeroU16,
    table: Option<&FrameTable>,
    turn: HullTurn,
) -> f32 {
    match turn {
        HullTurn::ByFrame => frame_angle(drawn_frame(heading, frames, table), frames),
        HullTurn::Continuous => normalized(heading),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim_base::geometry::Vec2;

    use super::*;

    #[test]
    fn the_engine_reads_by_frame_and_the_other_reading_continuously() {
        assert_eq!(HullTurn::from(RuleSource::Engine), HullTurn::ByFrame);
        assert_eq!(HullTurn::from(RuleSource::Bible), HullTurn::Continuous);
        assert_eq!(HullTurn::default(), HullTurn::ByFrame);
    }

    fn frames(n: u16) -> NonZeroU16 {
        NonZeroU16::new(n).expect("non-zero")
    }

    /// Headings over a turn and a little past it, in 0.7-degree steps.
    fn sweep() -> impl Iterator<Item = f32> {
        (0..520).map(|i| i as f32 * 0.7)
    }

    #[test]
    fn by_frame_the_hull_angle_is_the_drawn_frames() {
        let table = FrameTable::new(frames(8), Vec2::new(1.0, 1.0), Vec2::new(1.0, 0.5));
        for (n, table) in [(36, None), (64, None), (8, Some(&table))] {
            for heading in sweep() {
                assert_eq!(
                    hull_angle(heading, frames(n), table, HullTurn::ByFrame),
                    frame_angle(drawn_frame(heading, frames(n), table), frames(n)),
                    "{n} frames at {heading}"
                );
            }
        }
        // A frame's angle holds within the frame, then steps.
        let at = |h| hull_angle(h, frames(36), None, HullTurn::ByFrame);
        assert_eq!((at(1.0), at(9.9), at(10.0)), (0.0, 0.0, 10.0));
    }

    #[test]
    fn continuously_the_hull_angle_is_the_heading() {
        let table = FrameTable::new(frames(8), Vec2::new(1.0, 1.0), Vec2::new(1.0, 0.5));
        let mut last = None;
        for i in 0..40 {
            let heading = i as f32 * 0.5;
            for table in [None, Some(&table)] {
                let angle = hull_angle(heading, frames(36), table, HullTurn::Continuous);
                assert_eq!(angle, heading);
            }
            assert_ne!(Some(heading), last);
            last = Some(heading);
        }
        let at = |h| hull_angle(h, frames(36), None, HullTurn::Continuous);
        assert_eq!(at(-10.0), 350.0);
        assert_eq!(at(370.0), 10.0);
        assert_eq!(at(97.3), 97.3);
    }
}
