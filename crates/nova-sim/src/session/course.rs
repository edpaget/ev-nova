//! The player's course on the session at the rebuild points and after an
//! arrival, as [`RuleKey::RouteRewrite`] reads it (see
//! [`course`](crate::course) for the rules themselves).
//!
//! **Rebuild points** ([`Session::rewrite_course`]):
//! `_HandleStellarSystemVisibility` rewrites the course first thing
//! (@0x32ab3-0x32b0d), before it shares exploration (@0x32b13). The
//! session rewrites first thing at each rebuild point, docked or not (see
//! the `stellars` module's "Rebuild points"), raising no event: the
//! rewrite never changes the course shown at that moment, as
//! `Session::course_shown` already resolves each hop by place and cuts at
//! the first with none. It shows only later, when a hop it rewrote or cut
//! shows again. A galaxy change is no rebuild point, so between rebuild
//! points the course keeps its IDs and each hop is resolved by place when
//! it is used. A load's rebuild rewrites under the default reading, as
//! `Session::fly` runs before `Session::with_rules` (task
//! load-reads-rules-before-fly).
//!
//! **An arrival** ([`Session::course_after_arrival`]): the jump's entry
//! drops the hops made, and an arrival that made none clears the course
//! or drops its first hop by the engine (@0x6c472-0x6c4c6; see the
//! `entry` module's `CourseStep::Arrival`). The entry tells
//! [`SessionEvent::SystemChanged`](super::SessionEvent::SystemChanged),
//! from which the view lays the course out again.

use super::{Session, Setup};
use crate::catalog::SystemId;
use crate::course;
use crate::rulebook::{RuleKey, RuleSource};

impl Session {
    /// A rebuild point's rewrite (see the module docs): by the engine,
    /// each hidden hop rewritten to the system at its place and the
    /// course cut at the first with none. Raises no event.
    pub(super) fn rewrite_course(&mut self) {
        self.pilot.course = course::rewritten(
            &self.pilot.course,
            self.galaxy.map().places(),
            self.setup.route_rewrite(),
        );
    }

    /// The course after a jump's arrival at `arrived` that made `hops` of
    /// its hops (see the module docs).
    pub(super) fn course_after_arrival(&mut self, hops: usize, arrived: SystemId) {
        self.pilot.course = course::after_arrival(
            &self.pilot.course,
            hops,
            arrived,
            self.galaxy.map().places(),
            self.setup.route_rewrite(),
        );
    }
}

impl Setup {
    /// How the session rewrites the course ([`RuleKey::RouteRewrite`]).
    fn route_rewrite(&self) -> RuleSource {
        self.rules.source_for(RuleKey::RouteRewrite)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::hyperspace::MultiJumpRule;
    use crate::rulebook::Rulebook;
    use crate::session::tests::{alternates, bound_for_134, chained, hide, unhide};
    use crate::testkit::{begin_jump_now, catalog, land_now};

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
}
