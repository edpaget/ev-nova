//! What the in-flight galaxy map's Find searches ([`Session::find_map_system`],
//! the session's side of [`map_search::search`]; see that module for the
//! match).
//!
//! `_DoSystemSearch` visits the 2048 system slots in order and takes a
//! system only when it is active (+0x1eb), its own explored level (+0x90)
//! is above 0, and it is known (+0x1ec) (@0x11f23-0x11f45). Here those are
//! a system on the star map the overlay shows
//! ([`OverlaidGalaxy::visible`](crate::story::OverlaidGalaxy::visible)),
//! explored at its own level under both readings of
//! [`RuleKey::ExploredByPlace`](crate::rulebook::RuleKey::ExploredByPlace),
//! and [`Session::is_known`], each named as the catalog names it, by
//! ascending ID.

use super::{Chart, Session, Statics};
use crate::map_search::{self, SearchOutcome};
use crate::pilot::{ExploredLevel, Pilot};

impl Session {
    /// The system the in-flight map's Find finds for `query` (see the
    /// module docs).
    #[must_use]
    pub fn find_map_system(&self, query: &str) -> SearchOutcome {
        self.chart
            .find_map_system(&self.pilot, &self.statics, query)
    }
}

impl Chart {
    /// The system the in-flight map's Find finds for `query` (see the
    /// module docs).
    #[must_use]
    fn find_map_system(&self, pilot: &Pilot, statics: &Statics, query: &str) -> SearchOutcome {
        let candidates = statics
            .place_names
            .systems
            .iter()
            .filter(|&(&id, _)| {
                self.galaxy.exists(id)
                    && self.galaxy.visible(id)
                    && pilot.explored_level(id) > ExploredLevel::Unexplored
                    && self.is_known(id)
            })
            .map(|(&id, name)| (id, name.as_str()));
        map_search::search(query, candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SystemId;
    use crate::session::tests::{alternates, hide};
    use crate::testkit::FakePilotCatalog;

    /// [`alternates`] with 130 "Sol", 131 "Kania", 132 "Kanto", 133
    /// "Nemor" and 134, at 131's place, "Kanius".
    fn named() -> FakePilotCatalog {
        let mut catalog = alternates();
        for (id, name) in [
            (130, "Sol"),
            (131, "Kania"),
            (132, "Kanto"),
            (133, "Nemor"),
            (134, "Kanius"),
        ] {
            catalog
                .place_names
                .systems
                .insert(SystemId(id), name.to_owned());
        }
        catalog
    }

    fn session(catalog: &FakePilotCatalog) -> Session {
        Session::start(catalog).expect("starts")
    }

    fn found(id: i16) -> SearchOutcome {
        SearchOutcome::Found(SystemId(id))
    }

    #[test]
    fn an_explored_known_visible_system_is_found_by_name() {
        let catalog = named();
        let session = session(&catalog);
        assert!(session.is_known(SystemId(130)));
        assert_eq!(session.find_map_system("SOL"), found(130));
    }

    #[test]
    fn a_hidden_system_is_not_found() {
        let catalog = named();
        let mut session = session(&catalog);
        session.pilot.explore(SystemId(132));
        session.mark_known([SystemId(132)]);
        assert_eq!(session.find_map_system("kanto"), found(132));
        hide(&mut session, &catalog, &[132]);
        session.mark_known([SystemId(132)]);
        assert_eq!(session.find_map_system("kanto"), SearchOutcome::NotFound);
    }

    #[test]
    fn an_unexplored_system_is_not_found_known_or_not() {
        let catalog = named();
        let mut session = session(&catalog);
        session.mark_known([SystemId(133)]);
        assert_eq!(
            session.pilot.explored_level(SystemId(133)),
            ExploredLevel::Unexplored
        );
        assert_eq!(session.find_map_system("nemor"), SearchOutcome::NotFound);
    }

    /// Raised after the last rebuild, as `X` does while docked, with no
    /// known pass: explored, but not known yet.
    #[test]
    fn an_explored_system_not_yet_known_is_not_found() {
        let catalog = named();
        let mut session = session(&catalog);
        session.pilot.explore(SystemId(133));
        assert!(!session.is_known(SystemId(133)));
        assert_eq!(session.find_map_system("nemor"), SearchOutcome::NotFound);
        session.mark_known([SystemId(133)]);
        assert_eq!(session.find_map_system("nemor"), found(133));
    }

    #[test]
    fn of_two_systems_with_one_name_the_lower_id_is_found_unless_hidden() {
        let mut catalog = named();
        for id in [131, 132] {
            catalog
                .place_names
                .systems
                .insert(SystemId(id), "Twin".to_owned());
        }
        let mut session = session(&catalog);
        for id in [131, 132] {
            session.pilot.explore(SystemId(id));
        }
        session.mark_known([SystemId(131), SystemId(132)]);
        assert_eq!(session.find_map_system("twin"), found(131));
        hide(&mut session, &catalog, &[131]);
        session.mark_known([SystemId(132)]);
        assert_eq!(session.find_map_system("twin"), found(132));
    }

    /// 134 sits at 131's place: its head explored, 134 itself not. The
    /// search reads each system's own level, under both readings of
    /// `ExploredByPlace`.
    #[test]
    fn the_level_read_is_the_systems_own_not_its_places() {
        let catalog = named();
        let mut session = session(&catalog);
        session.pilot.explore(SystemId(131));
        session.mark_known([SystemId(131), SystemId(134)]);
        assert_eq!(session.find_map_system("kania"), found(131));
        assert_eq!(session.find_map_system("kanius"), found(131), "m = 4");
        session.pilot.explore(SystemId(134));
        assert_eq!(session.find_map_system("kanius"), found(134));
    }
}
