//! The player's and the NPCs' fighter bays in flight: the session's side
//! of [`bay`](crate::bay).

use super::Session;
use crate::rulebook::RuleSource;

impl Session {
    /// This session with the fighters the player launches doing first as
    /// `source` says (see the module docs): the engine's by default.
    #[must_use]
    pub fn with_fighter_launch(mut self, source: RuleSource) -> Self {
        self.fighter_launch = source;
        self
    }

    /// What a fighter the player launches does first: by the engine, its
    /// class's standing order; otherwise it attacks the player's target.
    #[must_use]
    pub fn fighter_launch(&self) -> RuleSource {
        self.fighter_launch
    }

    /// This session with the player's fighters out, as it leaves the
    /// system, following `source` (see the module docs): the engine's by
    /// default.
    #[must_use]
    pub fn with_fighter_recall(mut self, source: RuleSource) -> Self {
        self.fighter_recall = source;
        self
    }

    /// What becomes of the player's fighters out as it leaves the system:
    /// by the engine, they follow a jump if they can and stay out while
    /// it is landed; otherwise they go back into their bays.
    #[must_use]
    pub fn fighter_recall(&self) -> RuleSource {
        self.fighter_recall
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::testkit::catalog;

    #[test]
    fn both_fighter_rules_follow_the_engine_by_default_and_either_is_chosen() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.fighter_launch(), RuleSource::Engine);
        assert_eq!(session.fighter_recall(), RuleSource::Engine);
        for source in RuleSource::ALL {
            let session = Session::start(&catalog())
                .expect("starts")
                .with_fighter_launch(source);
            assert_eq!(session.fighter_launch(), source);
            assert_eq!(session.fighter_recall(), RuleSource::Engine);
            let session = Session::start(&catalog())
                .expect("starts")
                .with_fighter_recall(source);
            assert_eq!(session.fighter_recall(), source);
            assert_eq!(session.fighter_launch(), RuleSource::Engine);
        }
    }
}
