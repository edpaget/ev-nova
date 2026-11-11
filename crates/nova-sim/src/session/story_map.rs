//! The marks the new game's story log gives the course map it opens on
//! the map key ([`MarksFrom::StoryLog`](crate::flight_frame::MarksFrom::StoryLog)),
//! as stock mission info gives its map (see the `mission_map` module):
//! the selected mission's place as the green system, with the green
//! system's red arrow cleared, as mission info clears it.
//!
//! - **The mission** is the row at the log's index in the rows the log
//!   was given (`Statics::story_rows`, active missions by id), so the
//!   index and the list agree.
//! - **Its place** is the row's
//!   [`complete_at`](crate::flight_frame::StoryRow::complete_at) (its
//!   completion's stellar, else a delivery's), else a `take_point`'s gate:
//!   a `take_point` names no completion stellar, and without its gate the
//!   map would mark nothing for it.
//! - **The green system** is the place's holder, as mission info resolves
//!   a stellar (`Chart::green_system`), with no `MARKS_DESTINATION` test:
//!   a storyline mission has no `mïsn` flags. A place with no holder, an
//!   index past the rows, or a session with no storylines marks nothing.
//!
//! The original has no story log, so there is no routine to port. None of
//! this plays a sound or makes a random draw.

use super::{Chart, Statics};
use crate::flight_frame::{StoryObjective, StoryRow};
use crate::galaxy_map::MapMarks;
use crate::pilot::Pilot;

impl Chart {
    /// The marks of the map the story log opens with the row at `index`
    /// of its rows selected (see the module docs).
    pub(super) fn story_map_marks(
        &self,
        pilot: &Pilot,
        statics: &Statics,
        index: usize,
    ) -> MapMarks {
        let green = statics
            .story_rows(pilot)
            .get(index)
            .and_then(place)
            .and_then(|stellar| {
                self.galaxy
                    .holder(stellar, statics.listers(stellar), pilot.system)
            });
        MapMarks {
            green,
            multi_arrows: false,
        }
    }
}

/// Where `row`'s mission is done: where it completes, else a
/// `take_point`'s gate.
fn place(row: &StoryRow) -> Option<crate::catalog::StellarId> {
    row.complete_at.or(match row.objective {
        StoryObjective::TakePoint { gate, .. } => gate,
        StoryObjective::Deliver { .. } => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{StellarId, SystemId};
    use crate::session::Session;
    use crate::session::story_screens::tests::{active, hauling, storied, taking, with_states};
    use crate::testkit::{FakePilotCatalog, catalog};

    /// The system `catalog`'s sites list `stellar` in.
    fn listed_in(catalog: &FakePilotCatalog, stellar: i16) -> SystemId {
        catalog
            .sites
            .iter()
            .find(|(_, sites)| sites.iter().any(|site| site.id == StellarId(stellar)))
            .map(|(system, _)| *system)
            .expect("listed")
    }

    fn marks(session: &Session, index: usize) -> MapMarks {
        session
            .chart
            .story_map_marks(&session.pilot, &session.statics, index)
    }

    fn green(system: SystemId) -> MapMarks {
        MapMarks {
            green: Some(system),
            multi_arrows: false,
        }
    }

    /// [`hauling`], completing at `at` (none for its delivery's place).
    fn completing_at(at: Option<i16>) -> nova_sim_catalog::storylines::StoryMission {
        let mut mission = hauling();
        if let Some(complete) = &mut mission.complete {
            complete.at = at.map(StellarId);
        }
        mission
    }

    #[test]
    fn a_delivery_marks_its_completion_stellar_s_system() {
        let catalog = storied(&[("job", completing_at(Some(140)))]);
        let session = with_states(&catalog, &[("job", active(10, false))]);
        assert_ne!(
            listed_in(&catalog, 140),
            session.pilot.system,
            "precondition: elsewhere"
        );
        assert_eq!(marks(&session, 0), green(listed_in(&catalog, 140)));
    }

    #[test]
    fn with_no_completion_stellar_a_delivery_marks_where_it_delivers() {
        let catalog = storied(&[("job", completing_at(None))]);
        let session = with_states(&catalog, &[("job", active(10, false))]);
        let to = match &session.statics.story_rows(&session.pilot)[0].objective {
            StoryObjective::Deliver { to, .. } => to.0,
            StoryObjective::TakePoint { .. } => unreachable!("a delivery"),
        };
        assert_eq!(marks(&session, 0), green(listed_in(&catalog, to)));
    }

    #[test]
    fn a_take_point_marks_its_gate_s_system() {
        let catalog = storied(&[("road", taking("warden"))]);
        let session = with_states(&catalog, &[("road", active(0, false))]);
        let gate = catalog.storylines.points["a/b"].gate.0;
        assert_eq!(marks(&session, 0), green(listed_in(&catalog, gate)));
    }

    #[test]
    fn the_index_picks_the_row_in_the_order_the_log_lists_them() {
        let catalog = storied(&[
            ("a-near", completing_at(None)),
            ("b-far", completing_at(Some(140))),
        ]);
        let session = with_states(
            &catalog,
            &[("a-near", active(10, false)), ("b-far", active(10, false))],
        );
        assert_eq!(marks(&session, 1), green(listed_in(&catalog, 140)));
        assert_eq!(marks(&session, 0), green(listed_in(&catalog, 128)));
    }

    #[test]
    fn an_index_past_the_rows_or_no_storylines_marks_nothing() {
        let none = MapMarks {
            green: None,
            multi_arrows: false,
        };
        let storied = storied(&[("job", completing_at(Some(140)))]);
        let session = with_states(&storied, &[("job", active(10, false))]);
        assert_eq!(marks(&session, 1), none);
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(marks(&session, 0), none);
    }
}
