//! The player's course on the session at the rebuild points and after an
//! arrival, as [`RuleKey::RouteRewrite`] reads it (see
//! [`course`](crate::course) for the rules themselves).
//!
//! The course's start (`Pilot::course_start`, the original's slot 0) is
//! rewritten and kept with it.
//!
//! **Rebuild points** ([`Chart::rewrite_course`]):
//! `_HandleStellarSystemVisibility` rewrites the course first thing, its
//! start included (@0x32aaa-0x32b0d), before it shares exploration
//! (@0x32b13). The session rewrites first thing at each rebuild point,
//! docked or not (see the `stellars` module's "Rebuild points"), raising
//! no event: a hop's rewrite never changes the course shown at that
//! moment, as `Session::course_shown` already resolves each hop by place
//! and cuts at the first with none. It shows only later, when a hop it
//! rewrote or cut shows again. A start cut is the exception: it empties
//! the course shown at once. A galaxy change is no rebuild point, so between rebuild points
//! the course keeps its IDs and each hop is resolved by place when it is
//! used. A load's rebuild rewrites under the default reading, as
//! `Session::fly` runs before `Session::with_rules` (task
//! load-reads-rules-before-fly).
//!
//! **An arrival** ([`Chart::course_after_arrival`]): the jump's entry
//! drops the hops made, the start then the last one made, and an arrival
//! that made none clears the course or drops its first hop by the engine
//! (@0x6c472-0x6c4c6; see the
//! `entry` module's `CourseStep::Arrival`). The entry tells
//! [`SessionEvent::SystemChanged`](super::SessionEvent::SystemChanged),
//! from which the view lays the course out again.
//!
//! **A load** ([`course_on_load`]): by [`RuleKey::LoadedCourse`]'s engine
//! reading a pilot opened from a save flies with no course, as the
//! original's open clears `_hyperRoute` and never restores it; by the
//! other reading the saved course is kept.

use super::{Chart, Session, Setup};
use crate::catalog::SystemId;
use crate::course;
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

impl Session {}

/// A load's course, as [`RuleKey::LoadedCourse`] reads it: by the engine
/// the hops and the start are cleared, as `_OpenPilotFile` (@0x7673e) and
/// `_OpenLastPilotFile` (@0x76502) call `_ResetPlayer`, which sets all 32
/// words of `_hyperRoute` to -1 (@0x1d6bb-0x1d6cd), and `_LoadPilotData`
/// never writes them back. By the other reading the saved course is kept.
pub(super) fn course_on_load(pilot: &mut Pilot, rules: &Rulebook) {
    if rules.source_for(RuleKey::LoadedCourse) == RuleSource::Engine {
        pilot.clear_course();
    }
}

impl Setup {
    /// How the session rewrites the course ([`RuleKey::RouteRewrite`]).
    pub(super) fn route_rewrite(&self) -> RuleSource {
        self.rules.source_for(RuleKey::RouteRewrite)
    }
}

impl Chart {
    /// A rebuild point's rewrite (see the module docs): by the engine,
    /// a hidden start and each hidden hop rewritten to the system at its
    /// place, and the course cut at the first with none, whole when it is
    /// the start. Raises no event.
    pub(super) fn rewrite_course(&mut self, pilot: &mut Pilot, setup: &Setup) {
        (pilot.course_start, pilot.course) = course::rewritten(
            pilot.course_start,
            &pilot.course,
            self.galaxy.map().places(),
            setup.route_rewrite(),
        );
    }

    /// The course after a jump's arrival at `arrived` that made `hops` of
    /// its hops (see the module docs).
    pub(super) fn course_after_arrival(
        &mut self,
        pilot: &mut Pilot,
        setup: &Setup,
        hops: usize,
        arrived: SystemId,
    ) {
        (pilot.course_start, pilot.course) = course::after_arrival(
            pilot.course_start,
            &pilot.course,
            hops,
            arrived,
            self.galaxy.map().places(),
            setup.route_rewrite(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::hyperspace::MultiJumpRule;
    use crate::session::tests::{
        alternates, at_gate, at_wormhole, bound_for_134, chained, gated, hide, unhide, wormholes,
    };
    use crate::testkit::{FakePilotCatalog, Scripted, begin_jump_now, catalog, land_now};

    fn ids(route: &[i16]) -> Vec<SystemId> {
        route.iter().copied().map(SystemId).collect()
    }

    fn rules(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::RouteRewrite, source)
    }

    /// What `engine` and `bible` give by `source`.
    fn by<T>(source: RuleSource, engine: T, bible: T) -> T {
        match source {
            RuleSource::Engine => engine,
            RuleSource::Bible => bible,
        }
    }

    /// A pilot with `course` from 130.
    fn coursed(course: &[i16]) -> Pilot {
        let catalog = catalog();
        let mut pilot = Pilot::new(&catalog, "").expect("a pilot");
        pilot.course = ids(course);
        pilot.course_start = (!course.is_empty()).then_some(SystemId(130));
        pilot
    }

    fn loaded_course(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::LoadedCourse, source)
    }

    #[test]
    fn a_load_clears_the_hops_and_the_start_by_the_engine_and_keeps_them_by_the_other_reading() {
        for source in RuleSource::ALL {
            let mut pilot = coursed(&[131, 132]);
            course_on_load(&mut pilot, &loaded_course(source));
            assert_eq!(
                pilot.course,
                by(source, ids(&[]), ids(&[131, 132])),
                "{source:?}"
            );
            assert_eq!(
                pilot.course_start,
                by(source, None, Some(SystemId(130))),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_saved_course_loads_as_the_rule_reads_and_the_save_keeps_it() {
        let catalog = catalog();
        for source in RuleSource::ALL {
            let mut pilot = coursed(&[131, 132]);
            pilot.course_start = Some(pilot.system);
            let start = pilot.course_start;
            let saved = crate::save::decode(&crate::save::encode(&pilot)).expect("loads");
            assert_eq!(saved.course, ids(&[131, 132]), "the save keeps the course");
            assert_eq!(saved.course_start, start, "the save keeps the start");
            let session = Session::load(&catalog, saved, &loaded_course(source)).expect("flies");
            assert_eq!(
                session.course(),
                by(source, ids(&[]), ids(&[131, 132])),
                "{source:?}"
            );
            assert_eq!(
                session.pilot().course_start(),
                by(source, None, start),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_load_leaves_an_empty_course_empty_under_both_readings() {
        for source in RuleSource::ALL {
            let mut pilot = coursed(&[]);
            course_on_load(&mut pilot, &loaded_course(source));
            assert_eq!(pilot.course, ids(&[]), "{source:?}");
            assert_eq!(pilot.course_start, None, "{source:?}");
        }
    }

    #[test]
    fn a_landing_rewrites_a_hidden_hop_to_the_system_at_its_place() {
        let catalog = alternates();
        for source in RuleSource::ALL {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(rules(source));
            session.plot_course(SystemId(132)).expect("a route");
            assert_eq!(session.course(), ids(&[131, 132]));
            hide(&mut session, &catalog, &[131]);
            assert_eq!(session.course(), ids(&[131, 132]), "no rebuild point");
            land_now(&mut session).expect("lands");
            assert_eq!(
                session.course(),
                by(source, ids(&[134, 132]), ids(&[131, 132])),
                "{source:?}"
            );
            assert_eq!(session.course_shown(), ids(&[134, 132]), "{source:?}");
        }
    }

    #[test]
    fn a_hop_with_no_stand_in_cuts_the_course_for_good() {
        let catalog = alternates();
        for source in RuleSource::ALL {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(rules(source));
            session.plot_course(SystemId(132)).expect("a route");
            hide(&mut session, &catalog, &[132]);
            land_now(&mut session).expect("lands");
            assert_eq!(
                session.course(),
                by(source, ids(&[131]), ids(&[131, 132])),
                "{source:?}"
            );
            assert_eq!(session.course_shown(), ids(&[131]), "{source:?}");
            unhide(&mut session, &catalog, &[132]);
            assert_eq!(
                session.course_shown(),
                by(source, ids(&[131]), ids(&[131, 132])),
                "{source:?}"
            );
        }
    }

    #[test]
    fn the_take_off_rewrites_the_course_in_flight() {
        let catalog = alternates();
        for source in RuleSource::ALL {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(rules(source));
            session.plot_course(SystemId(132)).expect("a route");
            land_now(&mut session).expect("lands");
            hide(&mut session, &catalog, &[131]);
            assert_eq!(session.course(), ids(&[131, 132]), "{source:?}");
            session.take_off(&catalog, &mut NeverFires);
            assert_eq!(
                session.course(),
                by(source, ids(&[134, 132]), ids(&[131, 132])),
                "{source:?}"
            );
        }
    }

    #[test]
    fn an_arrival_off_the_course_clears_it() {
        let catalog = chained(catalog(), 1);
        for source in RuleSource::ALL {
            let mut session =
                bound_for_134(&catalog, MultiJumpRule::default()).with_rules(rules(source));
            assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
            session.plot_course(SystemId(133)).expect("a route");
            assert_eq!(session.course(), ids(&[133]));
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(131))
            );
            assert_eq!(
                session.course(),
                by(source, ids(&[]), ids(&[133])),
                "{source:?}"
            );
        }
    }

    #[test]
    fn an_arrival_along_the_course_drops_only_the_hops_made() {
        let catalog = chained(catalog(), 1);
        for source in RuleSource::ALL {
            let mut session =
                bound_for_134(&catalog, MultiJumpRule::default()).with_rules(rules(source));
            assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(131))
            );
            assert_eq!(session.course(), ids(&[132, 134]), "{source:?}");
        }
    }

    /// Runs the set expression `text` and settles it, as play does.
    fn settle(session: &mut Session, catalog: &FakePilotCatalog, text: &str) {
        session.run_set(
            &SetExpr::parse(text).expect("parses"),
            &mut Scripted::default(),
        );
        let _ = session.settle_script(catalog, &mut Scripted::default());
    }

    #[test]
    fn a_plotted_course_starts_at_the_pilots_system() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.pilot.course_start(), Some(SystemId(130)));
        assert!(session.plot_course(SystemId(999)).is_err());
        assert_eq!(
            session.pilot.course_start(),
            None,
            "a failed plot clears it"
        );
    }

    #[test]
    fn a_course_whose_start_hides_with_nothing_at_its_place_is_cut_whole_at_the_next_rebuild() {
        let catalog = alternates();
        for source in RuleSource::ALL {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(rules(source));
            session.plot_course(SystemId(132)).expect("a route");
            settle(&mut session, &catalog, "M131");
            assert_eq!(session.system(), SystemId(131));
            assert_eq!(session.course(), ids(&[131, 132]), "the move keeps it");
            assert_eq!(session.pilot.course_start(), Some(SystemId(130)));
            hide(&mut session, &catalog, &[130]);
            land_now(&mut session).expect("lands");
            assert_eq!(
                session.course(),
                by(source, ids(&[]), ids(&[131, 132])),
                "{source:?}"
            );
            assert_eq!(
                session.pilot.course_start(),
                by(source, None, Some(SystemId(130))),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_hidden_start_with_a_system_at_its_place_becomes_it_and_keeps_the_course() {
        let catalog = alternates();
        for source in RuleSource::ALL {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(rules(source));
            settle(&mut session, &catalog, "M131");
            session.plot_course(SystemId(132)).expect("a route");
            assert_eq!(session.pilot.course_start(), Some(SystemId(131)));
            settle(&mut session, &catalog, "M130");
            hide(&mut session, &catalog, &[131]);
            land_now(&mut session).expect("lands");
            assert_eq!(session.course(), ids(&[132]), "{source:?}");
            assert_eq!(
                session.pilot.course_start(),
                by(source, Some(SystemId(134)), Some(SystemId(131))),
                "{source:?}"
            );
        }
    }

    #[test]
    fn the_take_off_after_a_landed_move_cuts_a_course_whose_start_hid() {
        let catalog = alternates();
        for source in RuleSource::ALL {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(rules(source));
            session.plot_course(SystemId(132)).expect("a route");
            land_now(&mut session).expect("lands");
            settle(&mut session, &catalog, "M131");
            assert_eq!(session.system(), SystemId(131));
            hide(&mut session, &catalog, &[130]);
            assert_eq!(session.course(), ids(&[131, 132]), "{source:?}");
            session.take_off(&catalog, &mut NeverFires);
            assert_eq!(
                session.course(),
                by(source, ids(&[]), ids(&[131, 132])),
                "{source:?}"
            );
        }
    }

    #[test]
    fn an_in_flight_move_that_keeps_the_course_draws_it_from_its_start() {
        // `_DrawMap` pairs `_hyperRoute` from slot 0 (@0xe661-0xe7a2),
        // never from the player's system.
        let catalog = alternates();
        for source in RuleSource::ALL {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(Rulebook::default().with_override(RuleKey::MoveArrival, source));
            session.plot_course(SystemId(132)).expect("a route");
            settle(&mut session, &catalog, "M131");
            assert_eq!(session.system(), SystemId(131), "{source:?}");
            let drawn = session
                .map_drawing(None, &[], crate::galaxy_map::MapMarks::NONE)
                .course;
            let kept = [(130, 131), (131, 132)].map(|(a, b)| (SystemId(a), SystemId(b)));
            assert_eq!(drawn, by(source, kept.to_vec(), vec![]), "{source:?}");
        }
    }

    #[test]
    fn an_arrival_along_the_course_starts_it_at_the_last_hop_made() {
        let catalog = chained(catalog(), 1);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        session.arrive(&catalog, &mut NeverFires);
        assert_eq!(session.pilot.course_start(), Some(SystemId(131)));
    }

    #[test]
    fn a_hypergate_or_wormhole_exit_restarts_the_course_at_the_system_entered() {
        let gates = gated();
        let mut session = at_gate(&gates);
        session.pilot.course = ids(&[132]);
        session.pilot.course_start = Some(SystemId(133));
        session.open_hypergate(&gates).expect("opens");
        session
            .enter_hypergate(Some(SystemId(131)), &gates, &mut NeverFires)
            .expect("enters");
        assert_eq!(session.course(), ids(&[]));
        assert_eq!(session.pilot.course_start(), Some(SystemId(131)));

        let holes = wormholes();
        let mut session = at_wormhole(&holes);
        session.pilot.course = ids(&[132]);
        session
            .enter_wormhole(&holes, &mut Scripted::rolling(&[0]))
            .expect("enters");
        assert_eq!(session.course(), ids(&[]));
        assert_eq!(session.pilot.course_start(), Some(SystemId(131)));
    }
}
