//! The router's facts in [`Session::fill_frame`]: whether the spaceport's
//! Recharge button is enabled and whether the pilot has seen the intro
//! screen. The router reads each between commands (after every spaceport
//! input, and as the game held is entered from the main menu), any command
//! may change them, and each is a few comparisons with no catalog, chance
//! or allocation, so they are frame fields, kept current by the view's
//! refill after every command.

use super::super::Session;

impl Session {
    /// Writes the router's facts: whether Recharge is enabled
    /// ([`Session::recharge_enabled`]) and whether the pilot has seen the
    /// intro ([`Session::seen_intro`]).
    pub(super) fn fill_router_facts(&self, recharge_enabled: &mut bool, seen_intro: &mut bool) {
        *recharge_enabled = self.recharge_enabled();
        *seen_intro = self.seen_intro();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::session::frame::tests::filled;
    use crate::session::tests::{alternates, exchange, half_empty_at_port};

    #[test]
    fn a_fill_enables_recharge_as_the_session_does() {
        let session = half_empty_at_port();
        assert!(session.recharge_enabled(), "precondition: room and cash");
        assert!(filled(&session).recharge_enabled);
        let mut full = half_empty_at_port();
        full.pilot.reserves.fuel.now = 300.0;
        assert!(!full.recharge_enabled(), "precondition: full");
        assert!(!filled(&full).recharge_enabled, "a full tank");
        let mut flying = half_empty_at_port();
        flying.take_off(&exchange(), &mut NeverFires);
        assert!(!flying.recharge_enabled(), "precondition: in flight");
        assert!(!filled(&flying).recharge_enabled, "in flight");
    }

    #[test]
    fn a_fill_says_whether_the_pilot_has_seen_the_intro_and_follows_the_mark() {
        let mut session = Session::start(&alternates()).expect("starts");
        assert!(!filled(&session).seen_intro);
        session.mark_intro_seen();
        assert!(filled(&session).seen_intro);
        assert_eq!(filled(&session).seen_intro, session.seen_intro());
    }
}
