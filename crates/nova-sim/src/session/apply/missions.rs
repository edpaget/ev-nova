//! [`Session::apply`]'s mission arms: the mission computer and its rows,
//! the offers and their venues, the accept, the refusal and the abort,
//! the mission info's rows and its key, each matched to the session
//! method that runs it and answered as [`Session::apply_missions`] says.
//! None of them runs in `_DoPlayGameWork`'s fixed step, so none is the
//! step's ([`Session::step`]): the view sends each from its screen or
//! key, and stages the bar's redraws around the scripts they run.

use super::ApplyCollaborators;
use crate::flight_frame::{Answer, FlightCommand};
use crate::session::Session;

impl Session {
    /// Runs the mission `command`, drawing on `collab`'s chance where its
    /// method does, and answers it:
    ///
    /// - [`FlightCommand::OpenMissionComputer`] runs
    ///   [`Session::open_mission_computer`]: [`Answer::MissionComputer`],
    ///   its refusal among them.
    /// - [`FlightCommand::ShowComputerRow`] and [`FlightCommand::ShowInfoRow`]
    ///   run [`Session::show_computer_row`] and [`Session::show_info_row`]:
    ///   [`Answer::ComputerRow`] and [`Answer::InfoRow`].
    /// - [`FlightCommand::AcceptMission`], [`FlightCommand::RefuseMission`]
    ///   and [`FlightCommand::AbortMission`] run [`Session::accept_mission`],
    ///   [`Session::refuse_mission`] and [`Session::abort_mission`]:
    ///   [`Answer::Accepted`], [`Answer::RefuseShown`] and
    ///   [`Answer::Aborted`], each with its refusal.
    /// - [`FlightCommand::NextOffer`] runs [`Session::next_offer`]:
    ///   [`Answer::Offer`].
    /// - [`FlightCommand::PressMissionInfo`] runs
    ///   [`Session::press_mission_info`]: [`Answer::MissionInfo`], the
    ///   session's own refusal (during a jump, or under a refused gate)
    ///   among them, so the view tells it from a flight with no session.
    /// - [`FlightCommand::EnterOfferVenue`] and
    ///   [`FlightCommand::ReloadOfferDesc`] run [`Session::enter_offer_venue`]
    ///   and [`Session::reload_offer_desc`]: done.
    ///
    /// None reads the catalog, so none is refused for want of one.
    #[allow(clippy::needless_pass_by_value)] // the commands come by value, as `Session::apply` takes them
    pub(super) fn apply_missions(
        &mut self,
        command: FlightCommand,
        collab: &mut ApplyCollaborators<'_>,
    ) -> Answer {
        let chance = &mut *collab.chance;
        match command {
            FlightCommand::OpenMissionComputer => {
                Answer::MissionComputer(self.open_mission_computer())
            }
            FlightCommand::ShowComputerRow(mission) => {
                Answer::ComputerRow(self.show_computer_row(mission, chance))
            }
            FlightCommand::ShowInfoRow(slot) => Answer::InfoRow(self.show_info_row(slot, chance)),
            FlightCommand::AcceptMission(mission) => {
                Answer::Accepted(self.accept_mission(mission, chance))
            }
            FlightCommand::RefuseMission(mission) => {
                Answer::RefuseShown(self.refuse_mission(mission, chance))
            }
            FlightCommand::AbortMission(slot) => Answer::Aborted(self.abort_mission(slot, chance)),
            FlightCommand::NextOffer(loc) => Answer::Offer(self.next_offer(loc, chance)),
            FlightCommand::EnterOfferVenue(loc) => {
                self.enter_offer_venue(loc);
                Answer::Done
            }
            FlightCommand::ReloadOfferDesc(mission) => {
                self.reload_offer_desc(mission, chance);
                Answer::Done
            }
            FlightCommand::PressMissionInfo => Answer::MissionInfo(self.press_mission_info()),
            _ => Answer::Refused,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{MissionId, MissionTexts, StellarId};
    use crate::chance::NeverFires;
    use crate::mission::screens::SPACEPORT;
    use crate::session::MissionInfoPress;
    use crate::session::apply::tests::{apply, jumping};
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, catalog, land_now, mission};

    /// [`catalog`] with missions 130 and 131 at the mission computer and
    /// 132 at the spaceport, 130's info screen text "Quick.".
    fn errands() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.missions = vec![
            mission(130),
            mission(131),
            crate::catalog::MissionRecord {
                avail_loc: SPACEPORT,
                ..mission(132)
            },
        ];
        catalog.mission_texts = vec![MissionTexts {
            quick_brief: Some("Quick.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        }];
        catalog
    }

    /// A session of `catalog`'s landed on its planet 128 and settled.
    fn docked(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(catalog, &mut NeverFires);
        session.take_sounds();
        session
    }

    /// Checks that `command` answers `session` as `run` answers a copy of
    /// it, drawing on a chance that never fires, and leaves the session as
    /// `run` leaves the copy: the arm runs the method `run` calls.
    fn runs(
        session: &mut Session,
        command: FlightCommand,
        run: impl FnOnce(&mut Session) -> Answer,
    ) {
        let mut direct = session.clone();
        let expected = run(&mut direct);
        let sent = format!("{command:?}");
        assert_eq!(apply(session, command), expected, "{sent}");
        assert_eq!(*session, direct, "{sent}: as the method leaves it");
    }

    #[test]
    fn the_mission_computer_opens_and_selects_its_rows() {
        let mut session = docked(&errands());
        runs(
            &mut session,
            FlightCommand::OpenMissionComputer,
            |session| Answer::MissionComputer(session.open_mission_computer()),
        );
        let row = FlightCommand::ShowComputerRow(MissionId(131));
        runs(&mut session, row.clone(), |session| {
            Answer::ComputerRow(session.show_computer_row(MissionId(131), &mut NeverFires))
        });
        assert!(matches!(
            apply(&mut session, row),
            Answer::ComputerRow(Some(_))
        ));
        let mut flying = Session::start(&errands()).expect("starts");
        assert!(matches!(
            apply(&mut flying, FlightCommand::OpenMissionComputer),
            Answer::MissionComputer(Err(_))
        ));
    }

    #[test]
    fn an_accept_a_refusal_and_an_abort_answer_as_the_missions_do() {
        let mut session = docked(&errands());
        let accept = FlightCommand::AcceptMission(MissionId(130));
        runs(&mut session, accept, |session| {
            Answer::Accepted(session.accept_mission(MissionId(130), &mut NeverFires))
        });
        assert_eq!(session.pilot().missions().iter().count(), 1, "accepted");
        runs(
            &mut session,
            FlightCommand::RefuseMission(MissionId(131)),
            |session| Answer::RefuseShown(session.refuse_mission(MissionId(131), &mut NeverFires)),
        );
        runs(&mut session, FlightCommand::ShowInfoRow(0), |session| {
            Answer::InfoRow(session.show_info_row(0, &mut NeverFires))
        });
        assert_eq!(
            apply(&mut session, FlightCommand::ShowInfoRow(0)),
            Answer::InfoRow(Some("Quick.".to_owned()))
        );
        runs(&mut session, FlightCommand::AbortMission(0), |session| {
            Answer::Aborted(session.abort_mission(0, &mut NeverFires))
        });
        assert_eq!(session.pilot().missions().iter().count(), 0, "aborted");
        assert_eq!(
            apply(&mut session, FlightCommand::AbortMission(0)),
            Answer::Aborted(Err(crate::session::AbortError::NotActive)),
            "none active"
        );
        assert_eq!(
            apply(&mut session, FlightCommand::AcceptMission(MissionId(999))),
            Answer::Accepted(Err(crate::mission::AcceptError::NotOffered))
        );
    }

    #[test]
    fn the_venues_offers_answer_their_offer() {
        let mut session = docked(&errands());
        runs(
            &mut session,
            FlightCommand::EnterOfferVenue(SPACEPORT),
            |session| {
                session.enter_offer_venue(SPACEPORT);
                Answer::Done
            },
        );
        runs(
            &mut session,
            FlightCommand::NextOffer(SPACEPORT),
            |session| Answer::Offer(session.next_offer(SPACEPORT, &mut NeverFires)),
        );
        runs(
            &mut session,
            FlightCommand::ReloadOfferDesc(MissionId(132)),
            |session| {
                session.reload_offer_desc(MissionId(132), &mut NeverFires);
                Answer::Done
            },
        );
    }

    #[test]
    fn the_mission_info_key_answers_its_press_and_its_own_refusal() {
        let mut session = docked(&errands());
        assert_eq!(
            apply(&mut session, FlightCommand::PressMissionInfo),
            Answer::MissionInfo(MissionInfoPress::NoActiveMissions)
        );
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
        apply(&mut session, FlightCommand::AcceptMission(MissionId(130)));
        let info = session.mission_info();
        assert_eq!(
            apply(&mut session, FlightCommand::PressMissionInfo),
            Answer::MissionInfo(MissionInfoPress::Opens(info))
        );
        let mut jumping = jumping();
        let before = jumping.clone();
        assert_eq!(
            apply(&mut jumping, FlightCommand::PressMissionInfo),
            Answer::MissionInfo(MissionInfoPress::Refused),
            "the session's own refusal rides its answer"
        );
        assert_eq!(jumping, before, "and changes nothing");
    }
}
