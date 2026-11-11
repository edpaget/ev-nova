//! [`Session::apply`]'s news arm: [`FlightCommand::ReadNews`], the player
//! closing a news log, which marks read what it listed (see the story
//! layer's `news` module).
//!
//! [`FlightCommand::ReadNews`]: crate::flight_frame::FlightCommand::ReadNews

use super::done_if;
use crate::flight_frame::Answer;
use crate::session::Session;
use crate::storylines::news;

impl Session {
    /// Marks read each of `ids` the system the ship is in has heard and
    /// the pilot hasn't read ([`news::mark_read`]): [`Answer::Done`] when
    /// it marked any, [`Answer::Refused`] when it marked none (each read
    /// already, or not heard here). It isn't gated on being landed, as
    /// the story layer's filter is enough, and it makes no save due, like
    /// the intro's mark: the next save keeps it.
    pub(super) fn apply_read_news(&mut self, ids: &[String]) -> Answer {
        done_if(news::mark_read(
            &self.statics.storylines,
            &mut self.pilot.story,
            self.pilot.system,
            ids,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::SystemId;
    use crate::chance::NeverFires;
    use crate::flight_frame::{Answer, FlightCommand, ScreenAnswer};
    use crate::session::Session;
    use crate::session::apply::tests::apply;
    use crate::session::storylines::tests::{closing, flying, talking};

    /// A session of [`talking`] over [`closing`], in system 130 once the
    /// days have released `word` there, with no save due.
    fn heard_here() -> Session {
        let catalog = talking(&closing());
        let mut session = flying(&catalog);
        session.let_days_pass(5, &mut NeverFires);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        session.pilot.system = SystemId(130);
        session.out.save_due = false;
        session
    }

    /// The ids of the news heard here, each with whether it is read, as
    /// the screen's ask answers them.
    fn asked(session: &Session) -> Vec<(String, bool)> {
        match session.heard_news_screen() {
            ScreenAnswer::HeardNews(news) => {
                news.into_iter().map(|item| (item.id, item.read)).collect()
            }
            other => panic!("{other:?}"),
        }
    }

    fn read(session: &mut Session, ids: &[&str]) -> Answer {
        let ids = ids.iter().map(|&id| id.to_owned()).collect();
        apply(session, FlightCommand::ReadNews(ids))
    }

    #[test]
    fn reading_a_heard_item_is_done_and_the_next_ask_says_it_is_read() {
        let mut session = heard_here();
        assert_eq!(asked(&session), [("word".to_owned(), false)]);
        assert_eq!(read(&mut session, &["word"]), Answer::Done);
        assert_eq!(asked(&session), [("word".to_owned(), true)]);
    }

    #[test]
    fn reading_it_again_or_what_is_not_heard_here_is_refused_and_marks_nothing() {
        let mut session = heard_here();
        assert_eq!(read(&mut session, &["gone", "elsewhere"]), Answer::Refused);
        assert!(session.pilot().story.read.is_empty());
        session.pilot.system = SystemId(999);
        assert_eq!(read(&mut session, &["word"]), Answer::Refused, "not here");
        assert!(session.pilot().story.read.is_empty());
        session.pilot.system = SystemId(130);
        assert_eq!(read(&mut session, &["word"]), Answer::Done);
        let before = session.clone();
        assert_eq!(read(&mut session, &["word"]), Answer::Refused, "read");
        assert_eq!(session, before);
    }

    #[test]
    fn reading_makes_no_save_due() {
        let mut session = heard_here();
        assert_eq!(read(&mut session, &["word"]), Answer::Done);
        assert!(!session.out.save_due);
    }
}
