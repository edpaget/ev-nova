//! [`Session::apply`]'s conversation arm: [`FlightCommand::Converse`],
//! answered with the landed stellar's conversation page (see the
//! session's `conversation` module).
//!
//! [`FlightCommand::Converse`]: crate::flight_frame::FlightCommand::Converse

use super::done_if;
use crate::flight_frame::{Answer, Converse};
use crate::session::Session;

impl Session {
    /// Runs `converse` on the landed stellar's conversation, and answers
    /// it:
    ///
    /// - [`Converse::Open`] opens it at its knot, with the pilot's state of
    ///   its Ink file and its flags: [`Answer::Conversation`] with its
    ///   first page, or [`Answer::Refused`] in flight or where the stellar
    ///   names none.
    /// - [`Converse::Choose`] takes a choice, keeping the story's state in
    ///   the pilot: [`Answer::Conversation`] with the next page, or [`Answer::Refused`] with none open, on an ended
    ///   page or past its choices.
    /// - [`Converse::Close`] closes it: [`Answer::Done`], or
    ///   [`Answer::Refused`] with none open.
    pub(super) fn apply_conversation(&mut self, converse: Converse) -> Answer {
        let landed = self.mode.landed();
        let page = match converse {
            Converse::Open => {
                self.port
                    .open_conversation(&self.statics, &self.setup, &mut self.pilot, landed)
            }
            Converse::Choose(index) => self.port.choose_in_conversation(
                &self.statics,
                &self.setup,
                &mut self.pilot,
                landed,
                index,
            ),
            Converse::Close => return done_if(self.port.close_conversation()),
        };
        page.map_or(Answer::Refused, Answer::Conversation)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::sync::Arc;

    use super::*;
    use crate::catalog::StellarId;
    use crate::chance::NeverFires;
    use crate::dialog::Value;
    use crate::dialog::conversation::ConversationPage;
    use crate::dialog::fake::{DialogCall, FakeChoice, FakeDialog, FakeKnot, FakeLoader};
    use crate::flight_frame::FlightCommand;
    use crate::session::apply::tests::apply;
    use crate::testkit::{FakePilotCatalog, catalog, land_now};

    const STORY: &str = "dock story";
    const FILE: &str = "dialog/offer.ink";

    /// An offer at planet 128: a line and two choices, `Ask` leading on to
    /// another page and `Leave` ending it.
    /// Its first line, `You again.`, shows only while its flag `regular`
    /// is set.
    fn loader() -> FakeLoader {
        let offer = FakeKnot::new("offer")
            .line_when("regular", Value::Bool(true), "You again.", &[])
            .line("A job?", &[])
            .choice(
                FakeChoice::sticky("Ask", &[])
                    .line("Two gates.", &[])
                    .then("route"),
            )
            .choice(FakeChoice::sticky("Leave", &[]).line("Bye.", &["outcome: decline"]));
        let route = FakeKnot::new("route")
            .line("Either will do.", &[])
            .choice(FakeChoice::sticky("Take it", &[]).line("Go.", &[]));
        let dialog = FakeDialog::default()
            .declare("regular", Value::Bool(false))
            .knot(offer)
            .knot(route);
        FakeLoader::default().story(STORY, dialog)
    }

    /// [`catalog`] with planet 128, where the ship starts, naming the
    /// offer; planet 129 names none.
    fn talking() -> FakePilotCatalog {
        FakePilotCatalog {
            stellar_dialogs: nova_sim_catalog::dialogs::StellarDialogs::default().with(
                StellarId(128),
                FILE,
                Arc::from(STORY),
                "offer",
            ),
            ..catalog()
        }
    }

    fn page(lines: &[&str], choices: &[&str], ended: bool) -> Answer {
        Answer::Conversation(ConversationPage {
            lines: lines.iter().map(|&l| l.to_owned()).collect(),
            choices: choices.iter().map(|&c| c.to_owned()).collect(),
            ended,
            problem: None,
        })
    }

    fn converse(session: &mut Session, converse: Converse) -> Answer {
        apply(session, FlightCommand::Converse(converse))
    }

    /// A session of [`talking`]'s, wired to `loader`, landed on planet
    /// 128.
    fn landed(loader: &FakeLoader) -> Session {
        let mut session = Session::start(&talking())
            .expect("starts")
            .with_dialog_loader(Rc::new(loader.clone()));
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        session
    }

    #[test]
    fn landed_at_the_stellar_open_answers_the_first_page() {
        let mut session = landed(&loader());
        assert_eq!(
            converse(&mut session, Converse::Open),
            page(&["A job?"], &["Ask", "Leave"], false)
        );
    }

    #[test]
    fn flying_or_landed_elsewhere_open_is_refused() {
        let loader = loader();
        let mut flying = Session::start(&talking())
            .expect("starts")
            .with_dialog_loader(Rc::new(loader.clone()));
        assert_eq!(converse(&mut flying, Converse::Open), Answer::Refused);
        let mut elsewhere = landed(&loader);
        elsewhere.statics = Rc::new(crate::session::Statics {
            stellar_dialogs: nova_sim_catalog::dialogs::StellarDialogs::default().with(
                StellarId(129),
                "dialog/offer.ink",
                Arc::from(STORY),
                "offer",
            ),
            ..(*elsewhere.statics).clone()
        });
        assert_eq!(converse(&mut elsewhere, Converse::Open), Answer::Refused);
        assert_eq!(loader.log(), [], "nothing was opened");
    }

    #[test]
    fn choose_answers_the_next_page_and_the_last_is_ended() {
        let mut session = landed(&loader());
        converse(&mut session, Converse::Open);
        assert_eq!(
            converse(&mut session, Converse::Choose(0)),
            page(&["Two gates.", "Either will do."], &["Take it"], false)
        );
        assert_eq!(
            converse(&mut session, Converse::Choose(0)),
            page(&["Go."], &[], true)
        );
        assert_eq!(converse(&mut session, Converse::Choose(0)), Answer::Refused);
    }

    #[test]
    fn a_choice_past_the_end_is_refused_and_changes_nothing() {
        let mut session = landed(&loader());
        converse(&mut session, Converse::Open);
        let before = session.clone();
        assert_eq!(converse(&mut session, Converse::Choose(2)), Answer::Refused);
        assert_eq!(session, before);
        assert_eq!(
            converse(&mut session, Converse::Choose(1)),
            page(&["Bye."], &[], true)
        );
    }

    #[test]
    fn close_drops_the_conversation_and_a_later_choice_is_refused() {
        let mut session = landed(&loader());
        assert_eq!(converse(&mut session, Converse::Close), Answer::Refused);
        converse(&mut session, Converse::Open);
        assert_eq!(converse(&mut session, Converse::Close), Answer::Done);
        assert_eq!(converse(&mut session, Converse::Choose(0)), Answer::Refused);
        assert_eq!(converse(&mut session, Converse::Close), Answer::Refused);
    }

    #[test]
    fn opening_again_starts_from_the_knot() {
        let mut session = landed(&loader());
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(0));
        assert_eq!(
            converse(&mut session, Converse::Open),
            page(&["A job?"], &["Ask", "Leave"], false)
        );
    }

    #[test]
    fn taking_off_drops_it() {
        let mut session = landed(&loader());
        converse(&mut session, Converse::Open);
        assert_eq!(
            session.take_off(&talking(), &mut NeverFires),
            Some(StellarId(128))
        );
        assert_eq!(session.port.conversation, None);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(converse(&mut session, Converse::Choose(0)), Answer::Refused);
    }

    #[test]
    fn a_conversation_left_open_at_another_stellar_takes_no_choice() {
        let mut session = landed(&loader());
        converse(&mut session, Converse::Open);
        let open = session.port.conversation.clone();
        session.take_off(&talking(), &mut NeverFires);
        session.port.conversation = open;
        assert_eq!(converse(&mut session, Converse::Choose(0)), Answer::Refused);
    }

    #[test]
    fn the_session_setup_s_loader_reaches_the_session() {
        let loader = loader();
        let mut session =
            Session::start(&talking())
                .expect("starts")
                .set_up(crate::session::SessionSetup {
                    dialogs: Some(Rc::new(loader.clone())),
                    ..crate::session::SessionSetup::default()
                });
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(1));
        assert_eq!(
            loader.log(),
            [DialogCall::Enter("offer".to_owned()), DialogCall::Choose(1)]
        );
    }

    #[test]
    fn with_no_loader_the_conversation_ends_showing_why() {
        let mut session = Session::start(&talking()).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(
            converse(&mut session, Converse::Open),
            Answer::Conversation(ConversationPage {
                ended: true,
                problem: Some("not a compiled Ink story: no dialog runtime is wired".to_owned()),
                ..ConversationPage::default()
            })
        );
    }

    #[test]
    fn the_stock_catalog_names_no_conversation() {
        let mut session = Session::start(&catalog())
            .expect("starts")
            .with_dialog_loader(Rc::new(loader()));
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(converse(&mut session, Converse::Open), Answer::Refused);
    }

    // The story's state and the flags, in the pilot.

    #[test]
    fn a_choice_keeps_the_story_s_state_in_the_pilot_under_its_file() {
        let mut session = landed(&loader());
        assert_eq!(session.pilot().dialogs.get(FILE), None);
        converse(&mut session, Converse::Open);
        let opened = session.pilot().dialogs.get(FILE).cloned();
        assert!(opened.is_some(), "the first page is kept");
        converse(&mut session, Converse::Choose(0));
        let chosen = session.pilot().dialogs.get(FILE).cloned();
        assert!(chosen.is_some());
        assert_ne!(chosen, opened, "the chosen page is kept");
        assert_eq!(session.pilot().dialogs.iter().count(), 1);
    }

    #[test]
    fn a_flag_set_in_the_pilot_shows_in_the_page() {
        let mut session = landed(&loader());
        assert_eq!(
            converse(&mut session, Converse::Open),
            page(&["A job?"], &["Ask", "Leave"], false)
        );
        session.pilot.flags.set("regular");
        assert_eq!(
            converse(&mut session, Converse::Open),
            page(&["You again.", "A job?"], &["Ask", "Leave"], false)
        );
    }

    /// An offer whose `Ask` is once-only.
    fn once_loader() -> FakeLoader {
        let offer = FakeKnot::new("offer")
            .line("A job?", &[])
            .choice(FakeChoice::once("Ask", &[]).line("Two gates.", &[]))
            .choice(FakeChoice::sticky("Leave", &[]).line("Bye.", &[]));
        FakeLoader::default().story(STORY, FakeDialog::default().knot(offer))
    }

    #[test]
    fn a_spent_choice_is_still_spent_after_a_save_and_load() {
        let loader = once_loader();
        let mut session = landed(&loader);
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(0));
        converse(&mut session, Converse::Close);

        let pilot = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let mut resumed = Session::fly(&talking(), pilot)
            .expect("flies")
            .with_dialog_loader(Rc::new(loader.clone()));
        let landed_again = land_now(&mut resumed).map(|_| converse(&mut resumed, Converse::Open));

        let again = converse(&mut session, Converse::Open);
        assert_eq!(again, page(&["A job?"], &["Leave"], false));
        assert_eq!(landed_again, Ok(again));
    }
}
