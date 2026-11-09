//! What the new game's own landed screens ask of its storylines: the
//! news heard in the system the ship is in ([`ScreenAsk::HeardNews`]).
//! Each forwards to the session's own read; a session whose data holds
//! no storylines answers empty.
//!
//! [`ScreenAsk::HeardNews`]: crate::flight_frame::ScreenAsk::HeardNews

use crate::flight_frame::ScreenAnswer;

use super::Session;

impl Session {
    /// The news heard in the system the ship is in, landed or flying
    /// ([`Session::heard_news`] of the pilot's system).
    pub(super) fn heard_news_screen(&self) -> ScreenAnswer {
        ScreenAnswer::HeardNews(self.heard_news(self.pilot.system))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SystemId;
    use crate::chance::NeverFires;
    use crate::session::storylines::tests::{closing, flying, land, talking};
    use crate::testkit::catalog;

    #[test]
    fn the_news_asked_is_what_the_ship_s_system_has_heard_in_flight_and_landed() {
        let catalog = talking(&closing());
        let mut session = flying(&catalog);
        session.let_days_pass(30, &mut NeverFires);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        session.pilot.system = SystemId(130);
        let heard = session.heard_news(SystemId(130));
        assert!(!heard.is_empty(), "heard where it broke");
        assert_eq!(session.heard_news_screen(), ScreenAnswer::HeardNews(heard));
        land(&mut session, &catalog);
        let here = session.pilot.system;
        assert_eq!(
            session.heard_news_screen(),
            ScreenAnswer::HeardNews(session.heard_news(here))
        );
        assert_eq!(
            session.heard_news_screen(),
            session.heard_news_screen(),
            "asked twice, the same"
        );
    }

    #[test]
    fn a_session_with_no_storylines_hears_nothing() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            session.heard_news_screen(),
            ScreenAnswer::HeardNews(Vec::new())
        );
    }
}
