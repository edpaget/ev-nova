//! Direct writes of session state that no public setter makes, for
//! [`SessionBuilder`](crate::fixture::SessionBuilder): each puts the
//! session into a state play reaches some other way, without the rules on
//! that way (a load's, a landing's, a take-off's). Each is named for the
//! state it writes, and writes only that state and what the session
//! derives from it at once.

use super::Session;
use crate::catalog::SystemId;

impl Session {
    /// Makes `hops` the course, started at the system the ship is in, or
    /// clears it, its start included, when `hops` is empty: the course a
    /// kept load leaves, not cued, the nav selection untouched.
    pub(crate) fn set_course(&mut self, hops: Vec<SystemId>) {
        self.pilot.course_start = (!hops.is_empty()).then_some(self.pilot.system);
        self.pilot.course = hops;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::NavMode;
    use crate::testkit::catalog;

    #[test]
    fn setting_the_course_starts_it_here_and_cues_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.set_course(vec![SystemId(131), SystemId(132)]);
        assert_eq!(session.course(), [SystemId(131), SystemId(132)]);
        assert_eq!(session.pilot().course_start(), Some(SystemId(130)));
        assert_eq!(session.nav_mode(), NavMode::Off);
        session.set_course(Vec::new());
        assert_eq!(session.course(), []);
        assert_eq!(session.pilot().course_start(), None, "the start cleared");
    }
}
