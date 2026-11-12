//! How a ship's hull turns with its heading, [`HullTurn`], from
//! `nova_sim_combat::combat::hull_turn`, which re-exports it and keeps
//! `hull_angle`, the angle it reads.

use nova_sim_base::rulebook::RuleSource;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engine_reads_by_frame_and_the_other_reading_continuously() {
        assert_eq!(HullTurn::from(RuleSource::Engine), HullTurn::ByFrame);
        assert_eq!(HullTurn::from(RuleSource::Bible), HullTurn::Continuous);
        assert_eq!(HullTurn::default(), HullTurn::ByFrame);
    }
}
