//! The session's side of the mission screens (see the
//! [`mission`](crate::mission) module's "Screens"): what each mission's
//! screens show of it, read as the session starts.

use super::Session;
use crate::catalog::{MissionId, MissionTexts};

impl Session {
    /// What the screens show of mission `id`, as the session read it as
    /// it started: `None` for a `mïsn` it has no texts of.
    #[must_use]
    pub fn mission_texts(&self, id: MissionId) -> Option<&MissionTexts> {
        self.mission_texts.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::catalog;

    #[test]
    fn the_session_reads_each_missions_texts_as_it_starts() {
        let mut catalog = catalog();
        let run = MissionTexts {
            offer: Some("Go.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        };
        catalog.mission_texts = vec![run.clone(), MissionTexts::named(MissionId(131), "Walk")];
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.mission_texts(MissionId(130)), Some(&run));
        assert_eq!(
            session
                .mission_texts(MissionId(131))
                .map(|texts| texts.name.as_str()),
            Some("Walk")
        );
        assert_eq!(session.mission_texts(MissionId(132)), None);
    }
}
