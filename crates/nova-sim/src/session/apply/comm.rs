//! [`Session::apply`]'s comm arms: the hail and its dialog's presses, the
//! board and the plunder dialog's, the escort keys and the escort
//! dialog's, and the message line's acknowledgement and count, each
//! matched to the session method that runs it and answered as
//! [`Session::apply_comm`] says. None of them runs in `_DoPlayGameWork`'s
//! fixed step, so none is the step's ([`Session::step`]): the view sends
//! each at its own point, the plunder dialog's [`FlightCommand::CommTicked`]
//! after the take's line it words and shows, as the dialog's loop runs
//! `_HandleComm` (@0x949a1) after `_DisplayComm`.

use super::ApplyCollaborators;
use crate::flight_frame::{Answer, FlightCommand};
use crate::session::Session;

impl Session {
    /// Runs the comm `command`, handed `collab`, and answers it:
    ///
    /// - [`FlightCommand::Hail`] runs [`Session::hail_key`] with the
    ///   collaborators' hail options: answered [`Answer::Hailed`], or
    ///   [`Answer::HailRefused`] with why, its beep raised.
    /// - [`FlightCommand::AnswerHail`] and [`FlightCommand::Haggle`] run
    ///   [`Session::answer`] and [`Session::haggle`]: [`Answer::Hail`],
    ///   the comm dialog's contents after, `None` with no hail under way.
    /// - [`FlightCommand::HangUp`] runs [`Session::hang_up`]: done.
    /// - [`FlightCommand::Board`] runs [`Session::board`] by the
    ///   collaborators' law and boarding rule: [`Answer::Boarded`], or
    ///   [`Answer::BoardRefused`] with why, its beep raised when it is
    ///   said.
    /// - [`FlightCommand::Plunder`] runs [`Session::plunder`] by the
    ///   boarding rule: [`Answer::Plundered`] with what it took and whether
    ///   the dialog is still open, or refused, changing nothing, with no
    ///   boarding under way ([`Session::boarding`] none).
    /// - [`FlightCommand::LetGoOfBoarding`] runs
    ///   [`Session::let_go_of_boarding`]: done.
    /// - [`FlightCommand::CommandEscorts`] runs [`Session::command_escorts`]:
    ///   [`Answer::Commanded`] with what it changed; done when it changed
    ///   no escort's order or target, as it still sets the standing order
    ///   of each class of the group, with no sound; refused while landed,
    ///   where nothing changes.
    /// - [`FlightCommand::PressEscort`] runs [`Session::press_escort`]:
    ///   [`Answer::EscortDialog`], what the dialog shows after, `None` once
    ///   it closes or when it was not open.
    /// - [`FlightCommand::CommAcknowledged`] and [`FlightCommand::CommTicked`]
    ///   run [`Session::comm_acknowledged`] and [`Session::comm_ticked`]:
    ///   done.
    ///
    /// The hail and its dialog's presses read the catalog
    /// ([`CommCatalog`](crate::CommCatalog)): with none they are refused.
    pub(super) fn apply_comm(
        &mut self,
        command: FlightCommand,
        collab: &mut ApplyCollaborators<'_>,
    ) -> Answer {
        let chance = &mut *collab.chance;
        match command {
            FlightCommand::HangUp => {
                self.hang_up();
                Answer::Done
            }
            FlightCommand::Board => match self.board(collab.law, collab.boarding_rule, chance) {
                Ok(boarding) => Answer::Boarded(boarding),
                Err(refusal) => Answer::BoardRefused(refusal),
            },
            FlightCommand::Plunder(take) => {
                if self.boarding().is_none() {
                    return Answer::Refused;
                }
                let taken = self.plunder(take, collab.boarding_rule, chance);
                Answer::Plundered {
                    taken,
                    open: self.boarding().is_some(),
                }
            }
            FlightCommand::LetGoOfBoarding => {
                self.let_go_of_boarding();
                Answer::Done
            }
            FlightCommand::CommandEscorts { group, command } => {
                if self.mode.landed().is_some() {
                    return Answer::Refused;
                }
                self.command_escorts(group, command, chance)
                    .map_or(Answer::Done, Answer::Commanded)
            }
            FlightCommand::PressEscort(button) => Answer::EscortDialog(self.press_escort(button)),
            FlightCommand::CommAcknowledged => {
                self.comm_acknowledged();
                Answer::Done
            }
            FlightCommand::CommTicked => {
                self.comm_ticked();
                Answer::Done
            }
            command => {
                let Some(catalog) = collab.catalog else {
                    return Answer::Refused;
                };
                let options = collab.hail_options;
                match command {
                    FlightCommand::Hail { option } => {
                        match self.hail_key(catalog, options, option, chance) {
                            Ok(hailed) => Answer::Hailed(hailed),
                            Err(refusal) => Answer::HailRefused(refusal),
                        }
                    }
                    FlightCommand::AnswerHail(pick) => {
                        Answer::Hail(self.answer(pick, catalog, options, chance))
                    }
                    FlightCommand::Haggle(choice) => {
                        Answer::Hail(self.haggle(choice, catalog, options))
                    }
                    _ => Answer::Refused,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{BoardRefusal, Boarding, BoardingRule, NovaBoarding, Take, Taken};
    use crate::chance::Chance;
    use crate::escort::{EscortCommand, EscortGroup, EscortOrder};
    use crate::escort_dialog::EscortButton;
    use crate::fleet::EscortMark;
    use crate::hail::{Haggle, HailOptions, HailRefusal};
    use crate::legal::{Crime, LegalCode, NovaLaw};
    use crate::session::Hailed;
    use crate::session::escort_dialog::tests::{dialog_open, escorting, upgradable};
    use crate::session::hail::tests::{hailable, opening, targeting};
    use crate::session::tests::{BOARD_DRAWS, ON_BOARD, Witness, aboard, alongside, boardable};
    use crate::sound::SimSound;
    use crate::testkit::{Draws, FakePilotCatalog};
    use crate::traffic::npc::NpcId;

    /// Runs `command` on `session`, handed `catalog` (none when `None`),
    /// Nova's law and boarding rule and hail options, drawing on `draws`.
    fn apply_on(
        session: &mut Session,
        catalog: Option<&FakePilotCatalog>,
        command: FlightCommand,
        draws: &[u32],
    ) -> Answer {
        apply_by(
            session,
            (catalog, &NovaLaw::default(), &NovaBoarding::default()),
            command,
            &mut Draws::of(draws),
        )
    }

    /// Runs `command` on `session`, handed `catalog`, `law` and `rule`,
    /// Nova's hail options, drawing on `chance`.
    fn apply_by(
        session: &mut Session,
        (catalog, law, rule): (Option<&FakePilotCatalog>, &dyn LegalCode, &dyn BoardingRule),
        command: FlightCommand,
        chance: &mut dyn Chance,
    ) -> Answer {
        session.apply(
            command,
            &mut ApplyCollaborators {
                catalog: catalog.map(|catalog| catalog as &dyn crate::session::SimCatalog),
                chance,
                law,
                boarding_rule: rule,
                hail_options: &HailOptions::default(),
            },
        )
    }

    /// Always repels boarders, and notes each ask; never asked anything
    /// more.
    #[derive(Debug, Default)]
    struct Repels {
        asked: std::cell::Cell<u32>,
    }

    impl BoardingRule for Repels {
        fn repels(&self, _booty: u16) -> bool {
            self.asked.set(self.asked.get() + 1);
            true
        }

        fn capture_odds(&self, _crew: &crate::board::CaptureCrew, _chance: &mut dyn Chance) -> u8 {
            unreachable!("repelled")
        }

        fn captures(&self, _odds: u8, _chance: &mut dyn Chance) -> bool {
            unreachable!("repelled")
        }

        fn person_credits(&self, _credits: i32, _chance: &mut dyn Chance) -> i64 {
            unreachable!("repelled")
        }

        fn grant(
            &self,
            _grant: &crate::grant::PersonGrant,
            _stock: &[crate::grant::GrantStock],
            _free_mass: i64,
            _grant_max: crate::RuleSource,
            _chance: &mut dyn Chance,
        ) -> Option<crate::grant::Granted> {
            unreachable!("repelled")
        }
    }

    // The hail and its dialog.

    #[test]
    fn y_answers_the_comm_dialog_it_opens() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        let answer = apply_on(
            &mut session,
            Some(&catalog),
            FlightCommand::Hail { option: false },
            &opening(0),
        );
        let Answer::Hailed(Hailed::Comm(view)) = answer else {
            panic!("the comm dialog: {answer:?}");
        };
        assert_eq!(view.npc, NpcId(0));
        assert_eq!(
            session.hailing(&catalog, &HailOptions::default()),
            Some(view),
            "under way"
        );
    }

    #[test]
    fn y_on_a_fleet_escort_answers_its_escort_dialog() {
        let catalog = upgradable();
        let mut session = escorting(&catalog);
        let answer = apply_on(
            &mut session,
            Some(&catalog),
            FlightCommand::Hail { option: false },
            &[],
        );
        let Answer::Hailed(Hailed::Escort(view)) = answer else {
            panic!("the escort dialog: {answer:?}");
        };
        assert_eq!(session.escort_dialog(), Some(view));
    }

    #[test]
    fn a_refused_hail_answers_why_with_its_beep() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        session.chart.target = None;
        session.take_sounds();
        assert_eq!(
            apply_on(
                &mut session,
                Some(&catalog),
                FlightCommand::Hail { option: false },
                &[],
            ),
            Answer::HailRefused(HailRefusal::NoTarget)
        );
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
    }

    #[test]
    fn the_hail_lists_the_collaborators_options() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        let answer = session.apply(
            FlightCommand::Hail { option: false },
            &mut ApplyCollaborators {
                catalog: Some(&catalog),
                chance: &mut Draws::of(&opening(0)),
                law: &NovaLaw::default(),
                boarding_rule: &NovaBoarding::default(),
                hail_options: &HailOptions::empty(),
            },
        );
        let Answer::Hailed(Hailed::Comm(view)) = answer else {
            panic!("the comm dialog: {answer:?}");
        };
        assert_eq!(view.options, []);
    }

    #[test]
    fn an_answer_a_haggle_and_the_hang_up_go_to_the_hail_under_way() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        apply_on(
            &mut session,
            Some(&catalog),
            FlightCommand::Hail { option: false },
            &opening(0),
        );
        let answered = apply_on(
            &mut session,
            Some(&catalog),
            FlightCommand::AnswerHail(0),
            &[],
        );
        let Answer::Hail(Some(view)) = answered else {
            panic!("still open: {answered:?}");
        };
        assert_eq!(
            session.hailing(&catalog, &HailOptions::default()),
            Some(view)
        );
        assert_eq!(
            apply_on(
                &mut session,
                Some(&catalog),
                FlightCommand::Haggle(Haggle::Accept),
                &[]
            ),
            Answer::Hail(None),
            "no price asked"
        );
        assert_eq!(
            apply_on(&mut session, None, FlightCommand::HangUp, &[]),
            Answer::Done
        );
        assert_eq!(session.hailing(&catalog, &HailOptions::default()), None);
        assert_eq!(
            apply_on(
                &mut session,
                Some(&catalog),
                FlightCommand::AnswerHail(0),
                &[]
            ),
            Answer::Hail(None),
            "hung up"
        );
    }

    #[test]
    fn the_hail_and_its_presses_are_refused_with_no_catalog() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        let before = session.clone();
        for command in [
            FlightCommand::Hail { option: false },
            FlightCommand::AnswerHail(0),
            FlightCommand::Haggle(Haggle::Accept),
        ] {
            assert_eq!(
                apply_on(&mut session, None, command.clone(), &opening(0)),
                Answer::Refused,
                "{command:?}"
            );
        }
        assert_eq!(session, before, "nothing changed");
    }

    // The board and the plunder.

    #[test]
    fn b_answers_the_boarding_by_the_collaborators_law() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        let witness = Witness::default();
        assert_eq!(
            apply_by(
                &mut session,
                (None, &witness, &NovaBoarding::default()),
                FlightCommand::Board,
                &mut Draws::of(&BOARD_DRAWS),
            ),
            Answer::Boarded(Boarding::Opened(ON_BOARD))
        );
        assert_eq!(
            witness.seen.borrow().first().map(|(crime, _)| *crime),
            Some(Crime::Board),
            "judged by the law handed"
        );
        assert_eq!(session.boarding(), Some(ON_BOARD));
    }

    #[test]
    fn b_is_repelled_by_the_collaborators_boarding_rule() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        let rule = Repels::default();
        assert_eq!(
            apply_by(
                &mut session,
                (None, &NovaLaw::default(), &rule),
                FlightCommand::Board,
                &mut Draws::of(&BOARD_DRAWS),
            ),
            Answer::Boarded(Boarding::Repelled)
        );
        assert_eq!(rule.asked.get(), 1);
    }

    #[test]
    fn a_refused_board_answers_why_with_its_beep() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.space.traffic.npcs_mut()[0].condition = crate::combat::hull::Condition::Intact;
        session.take_sounds();
        assert_eq!(
            apply_on(&mut session, None, FlightCommand::Board, &BOARD_DRAWS),
            Answer::BoardRefused(BoardRefusal::CantBoard)
        );
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
        assert_eq!(session.boarding(), None);
    }

    #[test]
    fn a_take_answers_what_it_took_and_whether_the_dialog_stays_open() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        let cash = session.pilot.cash;
        assert_eq!(
            apply_on(
                &mut session,
                None,
                FlightCommand::Plunder(Take::Credits),
                &[0]
            ),
            Answer::Plundered {
                taken: Taken::Credits(ON_BOARD.credits),
                open: true,
            }
        );
        assert_eq!(session.boarding().map(|view| view.credits), Some(0));
        assert_eq!(session.pilot.cash, cash + ON_BOARD.credits, "stolen");
        assert_eq!(
            apply_on(&mut session, None, FlightCommand::Plunder(Take::Abort), &[]),
            Answer::Plundered {
                taken: Taken::Aborted,
                open: false,
            }
        );
        assert_eq!(session.boarding(), None);
    }

    #[test]
    fn a_take_with_no_boarding_is_refused_and_changes_nothing() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.take_sounds();
        let before = session.clone();
        assert_eq!(
            apply_on(
                &mut session,
                None,
                FlightCommand::Plunder(Take::Cargo),
                &[0]
            ),
            Answer::Refused
        );
        assert_eq!(session, before);
    }

    #[test]
    fn letting_go_of_the_boarding_closes_it() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        assert_eq!(
            apply_on(&mut session, None, FlightCommand::LetGoOfBoarding, &[]),
            Answer::Done
        );
        assert_eq!(session.boarding(), None);
    }

    // The escorts.

    #[test]
    fn an_escort_key_answers_what_it_commanded() {
        let catalog = upgradable();
        let mut session = escorting(&catalog);
        let hold = FlightCommand::CommandEscorts {
            group: EscortGroup::All,
            command: EscortCommand::Hold,
        };
        let answer = apply_on(&mut session, None, hold.clone(), &[0]);
        let Answer::Commanded(commanded) = answer else {
            panic!("commanded: {answer:?}");
        };
        assert_eq!(commanded.command, EscortCommand::Hold);
        assert_eq!(session.pilot.escorts[0].order, Some(EscortOrder::Hold));
        assert!(session.take_sounds().contains(&SimSound::EscortsCommanded));
        assert_eq!(
            apply_on(&mut session, None, hold, &[0]),
            Answer::Done,
            "held already: it ran, changing no escort"
        );
        assert_eq!(session.take_sounds(), [], "and raised no sound");
    }

    #[test]
    fn an_escort_key_while_landed_is_refused_and_changes_nothing() {
        let catalog = upgradable();
        let mut session = escorting(&catalog);
        session.mode = crate::session::Mode::Landed(crate::catalog::StellarId(128));
        let before = session.clone();
        assert_eq!(
            apply_on(
                &mut session,
                None,
                FlightCommand::CommandEscorts {
                    group: EscortGroup::All,
                    command: EscortCommand::Hold,
                },
                &[0],
            ),
            Answer::Refused
        );
        assert_eq!(session, before);
    }

    #[test]
    fn an_escort_dialogs_press_answers_what_it_shows_after() {
        let catalog = upgradable();
        let mut session = dialog_open(&catalog);
        let answer = apply_on(
            &mut session,
            None,
            FlightCommand::PressEscort(EscortButton::Sell),
            &[],
        );
        let Answer::EscortDialog(Some(view)) = answer else {
            panic!("still open: {answer:?}");
        };
        assert_eq!(view.mark, Some(EscortMark::Sell));
        assert_eq!(
            apply_on(
                &mut session,
                None,
                FlightCommand::PressEscort(EscortButton::Close),
                &[],
            ),
            Answer::EscortDialog(None),
            "closed"
        );
        assert_eq!(session.escort_dialog(), None);
    }

    // The message line.

    #[test]
    fn return_and_the_plunder_loops_count_go_to_the_message_line() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.comm_shown(240);
        assert_eq!(
            apply_on(&mut session, None, FlightCommand::CommTicked, &[]),
            Answer::Done
        );
        let mut line = crate::CommLine::default();
        line.show(240);
        line.tick();
        assert_eq!(session.comm_line(), line, "counted a tick");
        assert_eq!(
            apply_on(&mut session, None, FlightCommand::CommAcknowledged, &[]),
            Answer::Done
        );
        let mut line = crate::CommLine::default();
        line.show(240);
        line.tick();
        line.acknowledge();
        assert_eq!(session.comm_line(), line, "acknowledged");
    }
}
