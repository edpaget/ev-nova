//! [`Session::apply`]'s player info arms: the player info key's press and
//! the dialog's opening, each matched to the session method that runs it
//! and answered as [`Session::apply_player_info`] says. Neither runs in
//! `_DoPlayGameWork`'s fixed step: the view sends the press from its key
//! and the opening as the router opens the screen, from flight and from
//! the spaceport alike.

use super::ApplyCollaborators;
use crate::flight_frame::{Answer, FlightCommand};
use crate::session::Session;

impl Session {
    /// Runs the player info `command`, reading `collab`'s catalog where
    /// its method does, and answers it:
    ///
    /// - [`FlightCommand::PressPlayerInfo`] runs
    ///   [`Session::press_player_info`]: [`Answer::PlayerInfoPressed`],
    ///   the session's own refusal (in a jump, the take-off's grace or
    ///   while the ship breaks up, with its beep) among them, so the view
    ///   tells it from a flight with no session.
    /// - [`FlightCommand::OpenPlayerInfo`] runs
    ///   [`Session::open_player_info`] over the catalog:
    ///   [`Answer::PlayerInfo`], none in a jump or while the ship breaks
    ///   up; refused without a catalog, as it reads the stellars' records.
    #[allow(clippy::needless_pass_by_value)] // the commands come by value, as `Session::apply` takes them
    pub(super) fn apply_player_info(
        &mut self,
        command: FlightCommand,
        collab: &mut ApplyCollaborators<'_>,
    ) -> Answer {
        match command {
            FlightCommand::PressPlayerInfo => Answer::PlayerInfoPressed(self.press_player_info()),
            FlightCommand::OpenPlayerInfo => match collab.catalog {
                Some(catalog) => Answer::PlayerInfo(self.open_player_info(catalog).map(Box::new)),
                None => Answer::Refused,
            },
            _ => Answer::Refused,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::session::apply::tests::{apply, jumping};
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, catalog};

    /// Runs `command` on `session`, handed `catalog`, on a chance that
    /// never fires.
    fn apply_with(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        command: FlightCommand,
    ) -> Answer {
        session.apply(
            command,
            &mut ApplyCollaborators {
                catalog: Some(catalog),
                chance: &mut NeverFires,
                law: &crate::legal::NovaLaw::default(),
                boarding_rule: &crate::board::NovaBoarding::default(),
                hail_options: &crate::hail::HailOptions::default(),
            },
        )
    }

    #[test]
    fn the_player_info_key_answers_its_press_and_its_own_refusal() {
        let mut flying = Session::start(&catalog()).expect("starts");
        flying.take_sounds();
        assert_eq!(
            apply(&mut flying, FlightCommand::PressPlayerInfo),
            Answer::PlayerInfoPressed(true)
        );
        assert_eq!(flying.take_sounds(), [], "taken in silence");
        let mut jumping = jumping();
        assert_eq!(
            apply(&mut jumping, FlightCommand::PressPlayerInfo),
            Answer::PlayerInfoPressed(false),
            "the session's own refusal rides its answer"
        );
        assert_eq!(jumping.take_sounds(), [SimSound::Refused], "with its beep");
    }

    #[test]
    fn the_player_info_opens_as_the_session_opens_it_over_the_catalog() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let mut direct = session.clone();
        let info = direct.open_player_info(&catalog);
        assert!(info.is_some(), "flying, it opens");
        assert_eq!(
            apply_with(&mut session, &catalog, FlightCommand::OpenPlayerInfo),
            Answer::PlayerInfo(info.map(Box::new))
        );
        assert_eq!(session, direct, "as the opening leaves it");
        let mut jumping = jumping();
        let before = jumping.clone();
        assert_eq!(
            apply_with(&mut jumping, &catalog, FlightCommand::OpenPlayerInfo),
            Answer::PlayerInfo(None),
            "nothing opens in a jump"
        );
        assert_eq!(jumping, before, "and nothing changes");
    }

    #[test]
    fn the_player_info_is_refused_without_a_catalog() {
        let mut session = Session::start(&catalog()).expect("starts");
        let before = session.clone();
        assert_eq!(
            apply(&mut session, FlightCommand::OpenPlayerInfo),
            Answer::Refused
        );
        assert_eq!(session, before, "nothing changes");
    }
}
