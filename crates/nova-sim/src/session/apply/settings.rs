//! [`Session::apply`]'s settings arms: the view's game window's width, the
//! double-time key and the sound's quiet, each matched to the session
//! method that takes it and answered as [`Session::apply_setting`] says.
//! None is a player's command: the view sends each as it is built, resized
//! or told a preference, and none is gated.

use crate::flight_frame::{Answer, FlightCommand};
use crate::session::Session;

impl Session {
    /// Takes the setting `command` and answers it [`Answer::Done`], as a
    /// setting can't be refused:
    ///
    /// - [`FlightCommand::SetWindowWidth`] runs
    ///   [`Session::set_window_width`];
    /// - [`FlightCommand::SetDoubleTimeKey`] runs
    ///   [`Session::set_double_time_key`];
    /// - [`FlightCommand::SetSoundQuiet`] runs [`Session::set_sound_quiet`].
    #[allow(clippy::needless_pass_by_value)] // the commands come by value, as `Session::apply` takes them
    pub(super) fn apply_setting(&mut self, command: FlightCommand) -> Answer {
        match command {
            FlightCommand::SetWindowWidth(width) => self.set_window_width(width),
            FlightCommand::SetDoubleTimeKey(on) => self.set_double_time_key(on),
            FlightCommand::SetSoundQuiet(quiet) => self.set_sound_quiet(quiet),
            _ => return Answer::Refused,
        }
        Answer::Done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::apply::tests::apply;
    use crate::testkit::catalog;

    fn session() -> Session {
        Session::start(&catalog()).expect("starts")
    }

    #[test]
    fn the_window_width_is_taken_and_done() {
        let mut session = session();
        assert_eq!(
            apply(&mut session, FlightCommand::SetWindowWidth(1430)),
            Answer::Done
        );
        assert_eq!(session.space.window_width, 1430);
    }

    #[test]
    fn the_double_time_key_is_taken_and_done() {
        let mut session = session();
        assert_eq!(
            apply(&mut session, FlightCommand::SetDoubleTimeKey(true)),
            Answer::Done
        );
        assert!(session.double_time(), "latched at once");
        assert_eq!(
            apply(&mut session, FlightCommand::SetDoubleTimeKey(false)),
            Answer::Done
        );
        assert!(!session.double_time());
    }

    #[test]
    fn the_sounds_quiet_is_taken_and_done() {
        let mut session = session();
        assert_eq!(
            apply(&mut session, FlightCommand::SetSoundQuiet(true)),
            Answer::Done
        );
        assert!(session.setup.sound_quiet);
        assert_eq!(
            apply(&mut session, FlightCommand::SetSoundQuiet(false)),
            Answer::Done
        );
        assert!(!session.setup.sound_quiet);
    }

    #[test]
    fn a_command_not_a_setting_is_refused_here() {
        let mut session = session();
        let before = session.clone();
        assert_eq!(
            session.apply_setting(FlightCommand::CueCourse),
            Answer::Refused
        );
        assert_eq!(session, before);
    }
}
