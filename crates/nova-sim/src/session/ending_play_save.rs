//! Whether ending play saves the pilot: quitting the app, or going back
//! to the main menu, with the game still going.
//!
//! The original saves only on creation (`_DoNewPilot` @0x1935f), the
//! take-off after the port dialog (`_PlayerLandOnStellar` @0x6354b) and
//! the escape pod's rescue (@0x692c4), so a pilot ended in flight reopens
//! as it was at the last of those, never past a jump made since. By
//! [`RuleKey::EndingPlaySave`]'s engine reading, the default, ending play
//! in flight saves nothing; ending it landed saves, under both readings,
//! as the spaceport's own saves already left the file so. By the other
//! reading the pilot is saved wherever it is. A pilot that may not be
//! saved ([`Session::may_save`]) is never saved here either. No sound.

use super::Session;
use crate::rulebook::{RuleKey, RuleSource};

impl Session {
    /// Whether ending play now saves the pilot (see the module docs): it
    /// may be saved, and it is landed or [`RuleKey::EndingPlaySave`]
    /// reads the other reading.
    #[must_use]
    pub fn saves_on_ending_play(&self) -> bool {
        self.may_save()
            && (self.landed().is_some()
                || self.setup.rules.source_for(RuleKey::EndingPlaySave) == RuleSource::Bible)
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{StellarId, SystemId};
    use crate::combat::hull::Condition;
    use crate::fixture::SessionBuilder;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::testkit::catalog;

    fn rules(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::EndingPlaySave, source)
    }

    #[test]
    fn by_the_engine_ending_play_in_flight_saves_nothing() {
        let catalog = catalog();
        let session = SessionBuilder::flying(&catalog).named("Ada").build();
        assert!(session.may_save());
        assert!(!session.saves_on_ending_play());
    }

    #[test]
    fn landed_ending_play_saves_under_either_reading() {
        let catalog = catalog();
        for source in RuleSource::ALL {
            let session = SessionBuilder::flying(&catalog)
                .with_rules(rules(source))
                .landed_at(SystemId(131), StellarId(140))
                .build();
            assert!(session.saves_on_ending_play(), "{source:?}");
        }
    }

    #[test]
    fn by_the_other_reading_ending_play_in_flight_saves() {
        let catalog = catalog();
        let session = SessionBuilder::flying(&catalog)
            .with_rules(rules(RuleSource::Bible))
            .build();
        assert!(session.saves_on_ending_play());
    }

    #[test]
    fn a_pilot_that_may_not_be_saved_is_not_saved_under_either_reading() {
        let catalog = catalog();
        for source in RuleSource::ALL {
            let mut session = SessionBuilder::flying(&catalog)
                .with_rules(rules(source))
                .build();
            session.ship.condition = Condition::Destroyed;
            assert!(!session.may_save());
            assert!(!session.saves_on_ending_play(), "{source:?}");
        }
    }
}
