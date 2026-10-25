//! [`Session::apply`]: the player's [`FlightCommand`]s, as the flight
//! view's `FlightSim` port sends them, each matched to the session method
//! that runs it, and its [`Answer`] (see
//! [`flight_frame`](crate::flight_frame)). The match lives here, on the
//! session's side, so the view's adapter only forwards.

use super::Session;
use crate::flight_frame::{Answer, FlightCommand};

impl Session {
    /// Runs `command` and answers it: [`Answer::Refused`] when it did
    /// nothing.
    ///
    /// - [`FlightCommand::SelectStellar`] runs [`Session::select_stellar`],
    ///   answered with the stellar selected, or refused during a jump and
    ///   when the pick finds nothing.
    #[allow(clippy::needless_pass_by_value)] // the port sends commands by value, and later ones carry owned data
    pub fn apply(&mut self, command: FlightCommand) -> Answer {
        match command {
            FlightCommand::SelectStellar(pick) => self
                .select_stellar(pick)
                .map_or(Answer::Refused, Answer::Stellar),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{StellarId, SystemId};
    use crate::navigation::NavMode;
    use crate::session::StellarPick;
    use crate::session::tests::three_stellars;
    use crate::sound::SimSound;
    use crate::testkit::{begin_jump_now, catalog, fly_out};

    #[test]
    fn a_stellar_key_answers_the_stellar_it_selects() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.take_sounds();
        assert_eq!(
            session.apply(FlightCommand::SelectStellar(StellarPick::Slot(0))),
            Answer::Stellar(StellarId(131))
        );
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(StellarId(131))));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
    }

    #[test]
    fn an_empty_slot_is_refused_and_changes_nothing() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.take_sounds();
        let before = session.nav_mode();
        assert_eq!(
            session.apply(FlightCommand::SelectStellar(StellarPick::Slot(3))),
            Answer::Refused
        );
        assert_eq!(session.nav_mode(), before);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn a_stellar_key_is_refused_during_a_jump_and_changes_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        session.take_sounds();
        let before = session.nav_mode();
        assert_eq!(
            session.apply(FlightCommand::SelectStellar(StellarPick::Slot(0))),
            Answer::Refused
        );
        assert_eq!(session.nav_mode(), before);
        assert_eq!(session.take_sounds(), []);
    }
}
