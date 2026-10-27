//! [`Session::apply`]'s travel arms: the jump, the land key and the gates,
//! the arrival, the take-offs, the escorts' landing, the settling, the
//! fade and the end of play, each matched to the session method that runs
//! it and answered as [`Session::apply_travel`] says. None of them runs in
//! `_DoPlayGameWork`'s fixed step, so none is the step's
//! ([`Session::step`]): the view sends each at its own point.

use super::{ApplyCollaborators, done_if};
use crate::flight_frame::{Answer, FlightCommand};
use crate::session::Session;

impl Session {
    /// Runs the travel `command`, handed `collab`, and answers it:
    ///
    /// - [`FlightCommand::Jump`] runs [`Session::begin_jump`]: answered
    ///   [`Answer::Jumped`] with the system bound for, or
    ///   [`Answer::JumpRefused`] with why and whether the player is told
    ///   ([`Session::jump_refusal_told`]), its beep raised when told.
    /// - [`FlightCommand::Land`] runs [`Session::land`]: answered
    ///   [`Answer::LandPress`], or [`Answer::LandRefused`] with why and
    ///   the gate the refusal's stellar is ([`Session::gate_kind`]).
    /// - [`FlightCommand::OpenHypergate`] runs [`Session::open_hypergate`]:
    ///   [`Answer::Offered`] or [`Answer::GateRefused`].
    /// - [`FlightCommand::EnterHypergate`] and
    ///   [`FlightCommand::EnterWormhole`] run [`Session::enter_hypergate`]
    ///   and [`Session::enter_wormhole`]: [`Answer::Arrived`] or
    ///   [`Answer::GateRefused`].
    /// - [`FlightCommand::Arrive`] runs [`Session::arrive`] once the warp
    ///   has ended ([`Session::warp_end_due`]): [`Answer::Arrived`], and
    ///   refused before then, the pending move's settling with it.
    /// - [`FlightCommand::TakeOff`] and [`FlightCommand::LeaveIfDue`] run
    ///   [`Session::leave_port`] and [`Session::leave_if_due`]:
    ///   [`Answer::Departed`], or refused when the ship is not landed or no
    ///   leave is due.
    /// - [`FlightCommand::LandEscorts`] runs [`Session::land_escorts`]:
    ///   [`Answer::EscortReport`], never refused.
    /// - [`FlightCommand::SettleScript`] runs [`Session::settle_script`]:
    ///   [`Answer::Settled`].
    /// - [`FlightCommand::BeginFade`] and [`FlightCommand::EndPlay`] run
    ///   [`Session::begin_fade`] and [`Session::end_play`]: done when the
    ///   fade began or play ended, refused at any other stage.
    ///
    /// An arrival answers, with the system, the fight's events still
    /// pending ([`Session::take_combat_events`]): the explosions the jump
    /// left, a crön's `Y` on its days among them, which the view shows on
    /// the arrival's first frame. The gates, the arrival, the take-offs and
    /// the settling read the catalog: with none they are refused.
    pub(super) fn apply_travel(
        &mut self,
        command: FlightCommand,
        collab: &mut ApplyCollaborators<'_>,
    ) -> Answer {
        let chance = &mut *collab.chance;
        match command {
            FlightCommand::Jump => match self.begin_jump() {
                Ok(to) => Answer::Jumped(to),
                Err(refusal) => Answer::JumpRefused {
                    told: self.jump_refusal_told(refusal),
                    refusal,
                },
            },
            FlightCommand::Land => match self.land() {
                Ok(press) => Answer::LandPress(press),
                Err(refusal) => Answer::LandRefused {
                    gate: refusal
                        .stellar()
                        .and_then(|stellar| self.chart.gate_kind(stellar)),
                    refusal,
                },
            },
            FlightCommand::EndPlay => {
                let before = self.player_end;
                self.end_play();
                done_if(self.player_end != before)
            }
            FlightCommand::BeginFade => {
                let before = self.player_end;
                self.begin_fade();
                done_if(self.player_end != before)
            }
            FlightCommand::LandEscorts => Answer::EscortReport(self.land_escorts(chance)),
            command => {
                let Some(catalog) = collab.catalog else {
                    return Answer::Refused;
                };
                match command {
                    FlightCommand::OpenHypergate => self
                        .open_hypergate(catalog)
                        .map_or_else(Answer::GateRefused, Answer::Offered),
                    FlightCommand::EnterHypergate(to) => {
                        match self.enter_hypergate(to, catalog, chance) {
                            Ok(system) => self.arrived(system),
                            Err(refusal) => Answer::GateRefused(refusal),
                        }
                    }
                    FlightCommand::EnterWormhole => match self.enter_wormhole(catalog, chance) {
                        Ok(system) => self.arrived(system),
                        Err(refusal) => Answer::GateRefused(refusal),
                    },
                    FlightCommand::Arrive if self.warp_end_due() => self
                        .arrive(catalog, chance)
                        .map_or(Answer::Refused, |system| self.arrived(system)),
                    FlightCommand::TakeOff => self
                        .leave_port(catalog, chance)
                        .map_or(Answer::Refused, Answer::Departed),
                    FlightCommand::LeaveIfDue => self
                        .leave_if_due(catalog, chance)
                        .map_or(Answer::Refused, Answer::Departed),
                    FlightCommand::SettleScript => {
                        Answer::Settled(self.settle_script(catalog, chance))
                    }
                    _ => Answer::Refused,
                }
            }
        }
    }

    /// The answer to an arrival in `system`: it, and the fight's events
    /// still pending.
    fn arrived(&mut self, system: crate::catalog::SystemId) -> Answer {
        Answer::Arrived {
            system,
            combat: self.take_combat_events(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::flight::Controls;
    use crate::gate::{GateKind, GateRefusal};
    use crate::hyperspace::JumpRefusal;
    use crate::landing::{LandOutcome, LandingRefusal};
    use crate::player_fate::PlayerFate;
    use crate::session::tests::{at_gate, at_wormhole, expired_hold, gated, wormholes};
    use crate::session::{LandPress, StellarPick};
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, begin_jump_now, catalog, fly_out, land_now, work};

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
            },
        )
    }

    /// Runs `command` on `session` with no catalog shared.
    fn apply_bare(session: &mut Session, command: FlightCommand) -> Answer {
        super::super::tests::apply(session, command)
    }

    /// A session on `catalog` flown out with a course to Alpha Centauri
    /// (131), ready to jump, its sounds so far taken.
    fn ready(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, catalog);
        session.take_sounds();
        session
    }

    /// [`ready`]'s session jumped, worked to the warp's end.
    fn at_the_warps_end(catalog: &FakePilotCatalog) -> Session {
        let mut session = ready(catalog);
        begin_jump_now(&mut session, catalog).expect("jumps");
        for _ in 0..1000 {
            if session.warp_end_due() {
                return session;
            }
            work(&mut session, catalog, Controls::default());
        }
        panic!("the warp never ended");
    }

    #[test]
    fn j_answers_the_system_the_jump_is_bound_for() {
        let catalog = catalog();
        let mut session = ready(&catalog);
        assert_eq!(
            apply_bare(&mut session, FlightCommand::Jump),
            Answer::Jumped(SystemId(131))
        );
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
    }

    #[test]
    fn a_refused_jump_answers_why_and_whether_the_player_is_told() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        session.take_sounds();
        let answer = apply_bare(&mut session, FlightCommand::Jump);
        let Answer::JumpRefused { refusal, told } = answer else {
            panic!("refused: {answer:?}");
        };
        assert!(
            matches!(refusal, JumpRefusal::TooClose { .. }),
            "{refusal:?}"
        );
        assert!(told, "too close is always told");
        assert_eq!(session.take_sounds(), [SimSound::Refused], "with its beep");
        assert_eq!(session.preparing_jump(), None, "no jump under way");
        crate::fixture::set_player_condition(
            &mut session,
            crate::combat::hull::Condition::Disabled,
        );
        assert_eq!(
            apply_bare(&mut session, FlightCommand::Jump),
            Answer::JumpRefused {
                refusal: JumpRefusal::Disabled,
                told: false,
            },
            "a disabled ship is told nothing"
        );
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn l_answers_what_the_press_did() {
        let mut session = Session::start(&crate::session::tests::three_stellars()).expect("starts");
        session.select_stellar(StellarPick::Slot(1));
        let answer = apply_bare(&mut session, FlightCommand::Land);
        assert!(
            matches!(
                answer,
                Answer::LandPress(LandPress::Outcome(LandOutcome::Requested { .. }))
            ),
            "{answer:?}"
        );
    }

    #[test]
    fn a_refused_landing_answers_why_and_the_gate_its_stellar_is() {
        // Over the hypergate, a second L before the approach clears is too
        // far, at the gate.
        let catalog = gated();
        let mut session = Session::start(&catalog).expect("starts");
        assert!(matches!(
            apply_bare(&mut session, FlightCommand::Land),
            Answer::LandPress(_)
        ));
        session.take_sounds();
        let answer = apply_bare(&mut session, FlightCommand::Land);
        let Answer::LandRefused { refusal, gate } = answer else {
            panic!("refused: {answer:?}");
        };
        assert!(
            matches!(refusal, LandingRefusal::TooFar { .. }),
            "{refusal:?}"
        );
        assert_eq!(refusal.stellar(), Some(StellarId(300)));
        assert_eq!(gate, Some(GateKind::Hypergate));
        assert_eq!(session.take_sounds(), [SimSound::Refused], "with its beep");
        let mut session = super::super::tests::jumping();
        assert_eq!(
            apply_bare(&mut session, FlightCommand::Land),
            Answer::LandRefused {
                refusal: LandingRefusal::Jumping,
                gate: None,
            },
            "a refusal naming no stellar names no gate"
        );
    }

    #[test]
    fn a_hypergate_opened_offers_its_links_and_entered_arrives_with_the_fights_events() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        assert_eq!(
            apply_with(&mut session, &catalog, FlightCommand::OpenHypergate),
            Answer::Offered(vec![SystemId(131), SystemId(132)])
        );
        assert_eq!(
            apply_with(
                &mut session,
                &catalog,
                FlightCommand::EnterHypergate(Some(SystemId(131)))
            ),
            Answer::Arrived {
                system: SystemId(131),
                combat: Vec::new(),
            }
        );
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(
            apply_with(
                &mut session,
                &catalog,
                FlightCommand::EnterHypergate(Some(SystemId(130)))
            ),
            Answer::GateRefused(GateRefusal::NotAtGate),
            "the entry is over"
        );
        assert_eq!(
            apply_with(&mut session, &catalog, FlightCommand::OpenHypergate),
            Answer::GateRefused(GateRefusal::NotAtGate)
        );
    }

    #[test]
    fn a_wormhole_entered_arrives_or_is_refused() {
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog);
        let answer = apply_with(&mut session, &catalog, FlightCommand::EnterWormhole);
        let Answer::Arrived { system, .. } = answer else {
            panic!("arrived: {answer:?}");
        };
        assert_eq!(system, session.system());
        assert_ne!(system, SystemId(130));
        assert_eq!(
            apply_with(&mut session, &catalog, FlightCommand::EnterWormhole),
            Answer::GateRefused(GateRefusal::NotAtGate)
        );
    }

    #[test]
    fn the_gates_the_arrival_the_take_offs_and_the_settling_are_refused_with_no_catalog() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        for command in [
            FlightCommand::OpenHypergate,
            FlightCommand::EnterHypergate(Some(SystemId(131))),
            FlightCommand::EnterWormhole,
            FlightCommand::Arrive,
            FlightCommand::TakeOff,
            FlightCommand::LeaveIfDue,
            FlightCommand::SettleScript,
        ] {
            assert_eq!(
                apply_bare(&mut session, command.clone()),
                Answer::Refused,
                "{command:?}"
            );
        }
        assert_eq!(
            apply_with(&mut session, &catalog, FlightCommand::OpenHypergate),
            Answer::Offered(vec![SystemId(131), SystemId(132)]),
            "the entry still pending"
        );
    }

    #[test]
    fn the_arrival_waits_for_the_warps_end_then_answers_the_system_and_the_fights_events() {
        let catalog = catalog();
        let mut session = ready(&catalog);
        assert_eq!(
            apply_with(&mut session, &catalog, FlightCommand::Arrive),
            Answer::Refused,
            "no jump"
        );
        begin_jump_now(&mut session, &catalog).expect("jumps");
        assert!(!session.warp_end_due());
        let before = session.clone();
        assert_eq!(
            apply_with(&mut session, &catalog, FlightCommand::Arrive),
            Answer::Refused,
            "the warp hasn't ended"
        );
        assert_eq!(session, before, "nothing settled");
        let mut session = at_the_warps_end(&catalog);
        let boom = crate::catalog::Explosion {
            boom: crate::catalog::BoomId(130),
            extra: false,
        };
        session
            .space
            .combat
            .explode(crate::geometry::Vec2::ZERO, boom);
        let answer = apply_with(&mut session, &catalog, FlightCommand::Arrive);
        let Answer::Arrived { system, combat } = answer else {
            panic!("arrived: {answer:?}");
        };
        assert_eq!(system, SystemId(131));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(combat.len(), 1, "the event pending: {combat:?}");
        assert_eq!(session.take_combat_events(), [], "taken");
    }

    #[test]
    fn take_off_answers_the_departure_and_is_refused_in_flight() {
        let catalog = crate::session::tests::three_stellars();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(
            apply_with(&mut session, &catalog, FlightCommand::TakeOff),
            Answer::Refused
        );
        session.select_stellar(StellarPick::Slot(1));
        land_now(&mut session).expect("lands");
        let answer = apply_with(&mut session, &catalog, FlightCommand::TakeOff);
        let Answer::Departed(departure) = answer else {
            panic!("departed: {answer:?}");
        };
        assert_eq!(departure.stellar, StellarId(128));
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn a_held_leave_takes_off_and_none_is_refused() {
        let catalog = crate::session::script_effects::tests::moving();
        let mut session = crate::session::script_effects::tests::leave_held(&catalog);
        let answer = apply_with(&mut session, &catalog, FlightCommand::LeaveIfDue);
        let Answer::Departed(departure) = answer else {
            panic!("departed: {answer:?}");
        };
        assert_eq!(departure.stellar, StellarId(128));
        assert_eq!(session.landed(), None);
        assert!(!session.leave_due(), "the leave taken");
        let mut landed = super::super::tests::landed();
        let before = landed.clone();
        assert_eq!(
            apply_with(&mut landed, &catalog, FlightCommand::LeaveIfDue),
            Answer::Refused,
            "no leave due"
        );
        assert_eq!(landed, before);
    }

    #[test]
    fn the_escorts_landing_reports_nothing_with_nothing_to_settle() {
        let mut session = super::super::tests::landed();
        assert_eq!(
            apply_bare(&mut session, FlightCommand::LandEscorts),
            Answer::EscortReport(None)
        );
    }

    #[test]
    fn the_settling_answers_what_it_settled() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(
            apply_with(&mut session, &catalog, FlightCommand::SettleScript),
            Answer::Settled(crate::session::Settled::default())
        );
    }

    #[test]
    fn the_fade_begins_only_after_the_hold_and_play_ends_only_after_the_fade() {
        let catalog = catalog();
        let mut flying = Session::start(&catalog).expect("starts");
        assert_eq!(
            apply_bare(&mut flying, FlightCommand::BeginFade),
            Answer::Refused
        );
        assert_eq!(
            apply_bare(&mut flying, FlightCommand::EndPlay),
            Answer::Refused
        );
        let mut session = expired_hold();
        assert_eq!(
            apply_bare(&mut session, FlightCommand::EndPlay),
            Answer::Refused
        );
        assert_eq!(session.player_fate(), PlayerFate::Holding { ticks_left: 0 });
        assert_eq!(
            apply_bare(&mut session, FlightCommand::BeginFade),
            Answer::Done
        );
        assert_eq!(session.player_fate(), PlayerFate::Fading);
        assert_eq!(
            apply_bare(&mut session, FlightCommand::BeginFade),
            Answer::Refused,
            "begun once"
        );
        assert_eq!(
            apply_bare(&mut session, FlightCommand::EndPlay),
            Answer::Done
        );
        assert_eq!(session.player_fate(), PlayerFate::GameOver);
        assert_eq!(
            apply_bare(&mut session, FlightCommand::EndPlay),
            Answer::Refused
        );
    }
}
