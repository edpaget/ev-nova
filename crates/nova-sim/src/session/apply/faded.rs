//! [`Session::apply`]'s arm for a key read on the pass a jump collapsed
//! under the white ([`FlightCommand::PressFaded`](crate::flight_frame::FlightCommand::PressFaded)):
//! the only way into the `_screenFaded` guards of `_DoPlayerInfoDialog`,
//! `_DoMissionInfoDialog`, `_DoCommDialog` and `_DoPlanetCommDialog`. The
//! session keeps no note of that pass past its step: the view carries it
//! in this command, so a press on any later frame is the plain command,
//! and acts.

use super::ApplyCollaborators;
use crate::flight_frame::{Answer, FadedPress};
use crate::session::Session;

impl Session {
    /// Runs the key `press` as that pass reads it, handed `collab`, and
    /// answers it. Each key's own refusals come first and ride their plain
    /// answers:
    ///
    /// - [`FadedPress::PlayerInfo`] runs [`Session::press_player_info`]:
    ///   [`Answer::PlayerInfoFaded`] taken, [`Answer::PlayerInfoPressed`]
    ///   refused, with its beep.
    /// - [`FadedPress::MissionInfo`] runs
    ///   [`Session::press_faded_mission_info`]: [`Answer::MissionInfo`].
    /// - [`FadedPress::Hail`] runs [`Session::hail_key_faded`] with the
    ///   collaborators' hail options: [`Answer::Hailed`] or
    ///   [`Answer::HailRefused`], as the plain key; refused without a
    ///   catalog.
    pub(super) fn apply_faded(
        &mut self,
        press: FadedPress,
        collab: &mut ApplyCollaborators<'_>,
    ) -> Answer {
        match press {
            FadedPress::PlayerInfo => {
                if self.press_player_info() {
                    Answer::PlayerInfoFaded
                } else {
                    Answer::PlayerInfoPressed(false)
                }
            }
            FadedPress::MissionInfo => Answer::MissionInfo(self.press_faded_mission_info()),
            FadedPress::Hail { option } => {
                let Some(catalog) = collab.catalog else {
                    return Answer::Refused;
                };
                let (options, chance) = (collab.hail_options, &mut *collab.chance);
                match self.hail_key_faded(catalog, options, option, chance) {
                    Ok(hailed) => Answer::Hailed(hailed),
                    Err(refusal) => Answer::HailRefused(refusal),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::flight_frame::FlightCommand;
    use crate::hail::HailRefusal;
    use crate::session::Hailed;
    use crate::session::apply::tests::apply;
    use crate::session::hail::tests::{hailable, targeting};
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, catalog};

    /// Runs `press` on `session`, handed `catalog`.
    fn pressed(session: &mut Session, catalog: &FakePilotCatalog, press: FadedPress) -> Answer {
        session.apply(
            FlightCommand::PressFaded(press),
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
    fn a_faded_p_is_taken_in_silence_and_opens_after_it_as_usual() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.take_sounds();
        assert_eq!(
            apply(
                &mut session,
                FlightCommand::PressFaded(FadedPress::PlayerInfo)
            ),
            Answer::PlayerInfoFaded
        );
        assert_eq!(session.take_sounds(), [], "the guard is silent");
        assert_eq!(
            apply(&mut session, FlightCommand::PressPlayerInfo),
            Answer::PlayerInfoPressed(true),
            "no note outlives the press"
        );
    }

    #[test]
    fn a_dying_player_still_beeps_a_faded_p() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.ship.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        session.take_sounds();
        assert_eq!(
            apply(
                &mut session,
                FlightCommand::PressFaded(FadedPress::PlayerInfo)
            ),
            Answer::PlayerInfoPressed(false),
            "@0x7031d comes first"
        );
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
    }

    #[test]
    fn a_faded_y_answers_as_the_key_does() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        let press = FadedPress::Hail { option: false };
        assert_eq!(
            pressed(&mut session, &catalog, press),
            Answer::Hailed(Hailed::Faded)
        );
        session.chart.target = None;
        assert_eq!(
            pressed(&mut session, &catalog, press),
            Answer::HailRefused(HailRefusal::NoTarget)
        );
    }

    #[test]
    fn a_faded_y_reads_option_as_the_plain_keys_do() {
        // Option held hails the stellar even with a ship targeted, and
        // this flight has no navigation stellar.
        let catalog = hailable();
        let mut session = targeting(&catalog);
        assert_eq!(
            pressed(&mut session, &catalog, FadedPress::Hail { option: true }),
            Answer::HailRefused(HailRefusal::NoTarget)
        );
    }

    #[test]
    fn a_faded_y_is_refused_without_a_catalog() {
        let mut session = Session::start(&catalog()).expect("starts");
        let before = session.clone();
        assert_eq!(
            apply(
                &mut session,
                FlightCommand::PressFaded(FadedPress::Hail { option: false })
            ),
            Answer::Refused
        );
        assert_eq!(session, before, "nothing changes");
    }

    #[test]
    fn a_faded_i_answers_as_the_key_does() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            apply(
                &mut session,
                FlightCommand::PressFaded(FadedPress::MissionInfo)
            ),
            Answer::MissionInfo(crate::session::MissionInfoPress::NoActiveMissions)
        );
    }
}
