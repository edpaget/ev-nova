//! When a save is written in flight: the saves that come due in space
//! ([`Session::take_save_now`]) and ending play, by quitting the app or
//! going back to the main menu with the game still going
//! ([`Session::saves_on_ending_play`]).
//!
//! The original saves only on creation (`_DoNewPilot` @0x1935f), the
//! take-off after the port dialog (`_PlayerLandOnStellar` @0x6354b) and
//! the escape pod's rescue (@0x692c4), so a pilot ended in flight reopens
//! as it was at the last of those, never past a jump made since.
//! By [`RuleKey::InFlightSave`]'s engine reading, the default:
//!
//! - A save that comes due in flight (a fighter lost to a jump, a
//!   mission, cron, story, rank or escort change) is held, not written:
//!   it stays due, and the next landing writes it, or the next take-off
//!   if it comes first. The take-off's save and a Strict Play rescue's
//!   ([`Outbox::save_now`]) are written in flight, as the original's are,
//!   and so is a developer desk edit's, the desk not being play. Landed,
//!   every save due is written when it comes.
//! - Ending play in flight saves nothing; ending it landed saves.
//!
//! By the other reading, the port's earlier behaviour, every save due is
//! written when it comes and ending play saves wherever the pilot is. A
//! pilot that may not be saved ([`Session::may_save`]) is never saved on
//! ending play, under either reading. No sound.

use super::{Outbox, Session};
use crate::rulebook::{RuleKey, RuleSource};

impl Session {
    /// Whether ending play now saves the pilot (see the module docs): it
    /// may be saved, and it is landed or [`RuleKey::InFlightSave`]
    /// reads the other reading.
    #[must_use]
    pub fn saves_on_ending_play(&self) -> bool {
        self.may_save() && (self.landed().is_some() || self.saves_anywhere())
    }

    /// Whether a save due should be written now (see the module docs),
    /// taking it if so; a save held in flight stays due.
    pub(crate) fn take_save_now(&mut self) -> bool {
        let out = &self.out;
        let now = out.save_due
            && (out.save_in_flight || self.landed().is_some() || self.saves_anywhere());
        now && self.out.take_save_due()
    }

    /// Whether [`RuleKey::InFlightSave`] reads the port's earlier
    /// save-anywhere behaviour.
    fn saves_anywhere(&self) -> bool {
        self.setup.rules.source_for(RuleKey::InFlightSave) == RuleSource::Bible
    }
}

impl Outbox {
    /// A save is due that is written even in flight: the take-off's, a
    /// Strict Play rescue's or a developer desk edit's (see the
    /// `in_flight_save` module).
    pub(super) fn save_now(&mut self) {
        self.save_due = true;
        self.save_in_flight = true;
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::fixture::SessionBuilder;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::testkit::{catalog, land_now};

    fn rules(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::InFlightSave, source)
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

    #[test]
    fn by_the_engine_a_save_due_in_flight_is_held_until_the_next_landing() {
        let catalog = catalog();
        let mut session = SessionBuilder::flying(&catalog).build();
        session.out.save_due = true;
        assert!(!session.take_save_now(), "held in flight");
        assert!(!session.take_save_now(), "still held");
        land_now(&mut session).expect("lands");
        assert!(session.take_save_now(), "the landing writes it");
        assert!(!session.take_save_now(), "once");
    }

    #[test]
    fn a_held_save_stays_due_until_written() {
        let catalog = catalog();
        let mut session = SessionBuilder::flying(&catalog).build();
        session.out.save_due = true;
        assert!(!session.take_save_now());
        assert!(session.take_save_due(), "still due");
    }

    #[test]
    fn by_the_engine_the_take_off_save_is_written_in_flight() {
        let catalog = catalog();
        let mut session = SessionBuilder::flying(&catalog)
            .landed_at(SystemId(131), StellarId(140))
            .build();
        let _ = session.take_save_due();
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.landed(), None);
        assert!(session.take_save_now());
        session.out.save_due = true;
        assert!(!session.take_save_now(), "the next one in flight is held");
    }

    #[test]
    fn by_the_engine_a_developer_edit_in_flight_is_written() {
        let catalog = catalog();
        let mut session = SessionBuilder::flying(&catalog).build();
        session.set_credits(5);
        assert!(session.take_save_now());
    }

    #[test]
    fn landed_a_save_due_is_written_under_either_reading() {
        let catalog = catalog();
        for source in RuleSource::ALL {
            let mut session = SessionBuilder::flying(&catalog)
                .with_rules(rules(source))
                .landed_at(SystemId(131), StellarId(140))
                .build();
            let _ = session.take_save_due();
            assert!(session.transact(|pilot| pilot.set_cash(5)));
            assert!(session.take_save_now(), "{source:?}");
        }
    }

    #[test]
    fn by_the_other_reading_a_save_due_in_flight_is_written() {
        let catalog = catalog();
        let mut session = SessionBuilder::flying(&catalog)
            .with_rules(rules(RuleSource::Bible))
            .build();
        session.out.save_due = true;
        assert!(session.take_save_now());
        assert!(!session.take_save_now(), "once");
    }

    #[test]
    fn nothing_due_writes_nothing_under_either_reading() {
        let catalog = catalog();
        for source in RuleSource::ALL {
            let mut session = SessionBuilder::flying(&catalog)
                .with_rules(rules(source))
                .landed_at(SystemId(131), StellarId(140))
                .build();
            let _ = session.take_save_due();
            assert!(!session.take_save_now(), "{source:?}");
        }
    }
}
