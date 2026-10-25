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
    /// - [`FlightCommand::SelectNextSystem`] runs
    ///   [`Session::select_next_system`], answered with the system
    ///   selected, `None` when it let go of the selection, or refused
    ///   during a jump.
    /// - [`FlightCommand::ClearTarget`] runs [`Session::clear_target`],
    ///   refused when there was no target.
    /// - [`FlightCommand::SelectTarget`] runs [`Session::select_target`],
    ///   answered with the target it leaves, or refused while landed,
    ///   where the keys change nothing and are silent.
    /// - [`FlightCommand::NavModeKeys`] runs [`Session::nav_mode_keys`],
    ///   refused during a jump.
    #[allow(clippy::needless_pass_by_value)] // the port sends commands by value, and later ones carry owned data
    pub fn apply(&mut self, command: FlightCommand) -> Answer {
        match command {
            FlightCommand::SelectStellar(pick) => self
                .select_stellar(pick)
                .map_or(Answer::Refused, Answer::Stellar),
            FlightCommand::SelectNextSystem => {
                if self.mode.jump().is_some() {
                    Answer::Refused
                } else {
                    Answer::System(self.select_next_system())
                }
            }
            FlightCommand::ClearTarget => done_if(self.clear_target()),
            FlightCommand::SelectTarget(pick) => {
                if self.mode.landed().is_some() {
                    Answer::Refused
                } else {
                    Answer::Target(self.select_target(pick))
                }
            }
            FlightCommand::NavModeKeys { nav_off, h } => done_if(self.nav_mode_keys(nav_off, h)),
        }
    }
}

/// [`Answer::Done`] when the command ran, [`Answer::Refused`] when it did
/// nothing.
fn done_if(ran: bool) -> Answer {
    if ran { Answer::Done } else { Answer::Refused }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{StellarId, SystemId};
    use crate::navigation::NavMode;
    use crate::session::StellarPick;
    use crate::session::tests::three_stellars;
    use crate::sound::SimSound;
    use crate::stats::ShipStats;
    use crate::targeting::TargetPick;
    use crate::testkit::{begin_jump_now, catalog, fly_out, land_now, npc};
    use crate::traffic::npc::NpcId;

    /// A session started on `catalog`, its sounds so far taken.
    fn quiet(catalog: &crate::testkit::FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.take_sounds();
        session
    }

    /// A session flying a jump to Alpha Centauri, its sounds so far taken.
    fn jumping() -> Session {
        let mut session = Session::start(&catalog()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        session.take_sounds();
        session
    }

    /// A session landed on Earth (128), its sounds so far taken.
    fn landed() -> Session {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_stellar(StellarPick::Slot(1));
        land_now(&mut session).expect("lands");
        session.take_sounds();
        session
    }

    /// Adds an idle NPC at (50, 0), giving its number.
    fn ship(session: &mut Session) -> NpcId {
        let mut ship = npc(0, ShipStats::default());
        ship.state.position = crate::geometry::Vec2::new(50.0, 0.0);
        session.space.traffic.add_npc(ship)
    }

    #[test]
    fn hyper_select_answers_the_system_it_selects() {
        let mut session = quiet(&catalog());
        assert_eq!(
            session.apply(FlightCommand::SelectNextSystem),
            Answer::System(Some(SystemId(131)))
        );
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(131))));
        assert_eq!(session.take_sounds(), [SimSound::HyperSelected]);
    }

    #[test]
    fn hyper_select_with_nothing_to_offer_answers_none_and_lets_go() {
        let mut catalog = catalog();
        catalog.star_map[0].links.clear();
        let mut session = quiet(&catalog);
        session.select_stellar(StellarPick::Slot(0));
        session.take_sounds();
        assert_eq!(
            session.apply(FlightCommand::SelectNextSystem),
            Answer::System(None)
        );
        assert_eq!(session.nav_mode(), NavMode::Stellar(None));
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn hyper_select_is_refused_during_a_jump_and_changes_nothing() {
        let mut session = jumping();
        let before = session.nav_mode();
        assert_eq!(
            session.apply(FlightCommand::SelectNextSystem),
            Answer::Refused
        );
        assert_eq!(session.nav_mode(), before);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn option_backquote_lets_go_of_the_target() {
        let mut session = quiet(&catalog());
        let id = ship(&mut session);
        session.chart.target = Some(id);
        assert_eq!(session.apply(FlightCommand::ClearTarget), Answer::Done);
        assert_eq!(session.chart.target, None);
        assert_eq!(session.take_sounds(), [SimSound::TargetCleared]);
    }

    #[test]
    fn option_backquote_with_no_target_is_refused_and_silent() {
        let mut session = quiet(&catalog());
        assert_eq!(session.apply(FlightCommand::ClearTarget), Answer::Refused);
        assert_eq!(session.chart.target, None);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn a_target_key_answers_the_target_it_picks() {
        let mut session = quiet(&catalog());
        let id = ship(&mut session);
        assert_eq!(
            session.apply(FlightCommand::SelectTarget(TargetPick::Next)),
            Answer::Target(Some(id))
        );
        assert_eq!(session.chart.target, Some(id));
        assert_eq!(session.take_sounds(), [SimSound::TargetSelected]);
    }

    #[test]
    fn a_target_key_is_refused_while_landed_and_changes_nothing() {
        let mut session = landed();
        assert_eq!(
            session.apply(FlightCommand::SelectTarget(TargetPick::Next)),
            Answer::Refused
        );
        assert_eq!(session.chart.target, None);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn the_nav_block_turns_the_nav_off_or_to_hyperspace() {
        let mut session = quiet(&three_stellars());
        session.select_stellar(StellarPick::Slot(0));
        session.take_sounds();
        assert_eq!(
            session.apply(FlightCommand::NavModeKeys {
                nav_off: true,
                h: false
            }),
            Answer::Done
        );
        assert_eq!(session.nav_mode(), NavMode::Off);
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        session.apply(FlightCommand::NavModeKeys {
            nav_off: false,
            h: true,
        });
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
    }

    #[test]
    fn the_nav_block_is_refused_during_a_jump_and_changes_nothing() {
        let mut session = jumping();
        let before = session.nav_mode();
        assert_eq!(
            session.apply(FlightCommand::NavModeKeys {
                nav_off: true,
                h: true
            }),
            Answer::Refused
        );
        assert_eq!(session.nav_mode(), before);
        assert_eq!(session.take_sounds(), []);
    }

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
