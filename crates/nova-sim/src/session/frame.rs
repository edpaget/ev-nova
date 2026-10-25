//! [`Session::fill_frame`]: what the flight view draws, filled into a
//! [`FlightFrame`] for the view's `FlightSim` port
//! (see [`flight_frame`](crate::flight_frame)).

use super::Session;
use crate::flight_frame::{FlightFrame, MapKey};

impl Session {
    /// Fills `into` with what the flight view draws now. Every field is
    /// written, so a frame kept from an earlier fill, or from another
    /// session, shows nothing stale: the destructuring names every field,
    /// so a field added to [`FlightFrame`] doesn't compile until it is
    /// filled here.
    pub fn fill_frame(&self, into: &mut FlightFrame) {
        let FlightFrame {
            nav_mode,
            system,
            course_shown,
            course_plotted,
            docked,
            jump_under_way,
            jump_readiness,
            texts_queued,
            map_key,
        } = into;
        *nav_mode = self.nav_mode();
        *system = self.system();
        *course_shown = self.course_shown();
        *course_plotted = !self.course().is_empty();
        *docked = self.landed().is_some();
        *jump_under_way = self.jumping().is_some() || self.preparing_jump().is_some();
        *jump_readiness = self.jump_readiness();
        *texts_queued = self.texts_queued();
        *map_key = MapKey {
            system: self.system(),
            explored: self.pilot.explored().collect(),
            charted: self.pilot.charted().collect(),
            known: self.known().clone(),
            course: self.course().to_vec(),
            nav: self.nav_mode(),
        };
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::{StellarId, SystemId};
    use crate::hyperspace::JumpReadiness;
    use crate::navigation::NavMode;
    use crate::session::StellarPick;
    use crate::session::tests::three_stellars;
    use crate::testkit::{begin_jump_now, catalog, fly_out, land_now};

    /// A frame unlike any a test session fills, in every field.
    fn stale() -> FlightFrame {
        FlightFrame {
            nav_mode: NavMode::Hyperspace(Some(SystemId(999))),
            system: SystemId(999),
            course_shown: vec![SystemId(998)],
            course_plotted: true,
            docked: true,
            jump_under_way: true,
            jump_readiness: JumpReadiness::Underway,
            texts_queued: u64::MAX,
            map_key: MapKey {
                system: SystemId(999),
                explored: vec![SystemId(997)],
                charted: vec![SystemId(997)],
                known: BTreeSet::from([SystemId(996)]),
                course: vec![SystemId(995)],
                nav: NavMode::Hyperspace(Some(SystemId(999))),
            },
        }
    }

    /// `session`'s frame, filled over [`stale`].
    fn filled(session: &Session) -> FlightFrame {
        let mut frame = stale();
        session.fill_frame(&mut frame);
        frame
    }

    #[test]
    fn a_fill_writes_the_sessions_nav_mode_over_a_stale_one() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_stellar(StellarPick::Slot(0));
        let frame = filled(&session);
        assert_eq!(frame.nav_mode, NavMode::Stellar(Some(StellarId(131))));
        assert_eq!(frame.nav_mode, session.nav_mode());
    }

    #[test]
    fn a_fill_writes_the_current_system() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(filled(&session).system, SystemId(130));
    }

    #[test]
    fn a_fill_writes_the_course_as_it_is_flown_and_whether_one_is_plotted() {
        let mut session = Session::start(&catalog()).expect("starts");
        let frame = filled(&session);
        assert_eq!(
            (frame.course_shown, frame.course_plotted),
            (Vec::new(), false)
        );
        session.plot_course(SystemId(132)).expect("a route");
        let frame = filled(&session);
        assert_eq!(frame.course_shown, [SystemId(131), SystemId(132)]);
        assert_eq!(frame.course_shown, session.course_shown());
        assert!(frame.course_plotted);
    }

    #[test]
    fn a_fill_writes_whether_the_player_is_docked() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        assert!(!filled(&session).docked);
        session.select_stellar(StellarPick::Slot(1));
        land_now(&mut session).expect("lands");
        let mut frame = FlightFrame::default();
        session.fill_frame(&mut frame);
        assert!(frame.docked);
    }

    #[test]
    fn a_fill_writes_whether_a_jump_is_under_way_and_the_ships_readiness() {
        let mut session = Session::start(&catalog()).expect("starts");
        let frame = filled(&session);
        assert_eq!(
            (frame.jump_under_way, frame.jump_readiness),
            (false, JumpReadiness::Blocked)
        );
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        let mut frame = FlightFrame::default();
        session.fill_frame(&mut frame);
        assert_eq!(
            (frame.jump_under_way, frame.jump_readiness),
            (false, JumpReadiness::Clear)
        );
        session.begin_jump().expect("accepted");
        assert!(session.preparing_jump().is_some());
        let mut frame = FlightFrame::default();
        session.fill_frame(&mut frame);
        assert!(frame.jump_under_way, "the pre-jump stage counts");
        assert_eq!(frame.jump_readiness, JumpReadiness::Underway);
    }

    #[test]
    fn a_fill_counts_the_jump_itself_under_way() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        assert!(session.jumping().is_some());
        let mut frame = FlightFrame::default();
        session.fill_frame(&mut frame);
        assert!(frame.jump_under_way);
    }

    #[test]
    fn a_fill_writes_how_many_texts_are_queued() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(filled(&session).texts_queued, session.texts_queued());
    }

    #[test]
    fn a_fill_writes_what_the_course_map_is_drawn_from() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.mark_known([SystemId(132)]);
        session.plot_course(SystemId(131)).expect("a route");
        let frame = filled(&session);
        assert_eq!(
            frame.map_key,
            MapKey {
                system: SystemId(130),
                explored: session.pilot().explored().collect(),
                charted: session.pilot().charted().collect(),
                known: session.known().clone(),
                course: vec![SystemId(131)],
                nav: session.nav_mode(),
            }
        );
        assert!(frame.map_key.known.contains(&SystemId(132)));
        assert!(frame.map_key.explored.contains(&SystemId(130)));
    }

    #[test]
    fn a_change_to_any_of_the_maps_inputs_changes_its_key() {
        let mut session = Session::start(&catalog()).expect("starts");
        let before = filled(&session).map_key;
        assert_eq!(filled(&session).map_key, before, "nothing changed");
        session.mark_known([SystemId(133)]);
        let known = filled(&session).map_key;
        assert_ne!(known, before);
        session.plot_course(SystemId(131)).expect("a route");
        assert_ne!(filled(&session).map_key, known);
    }
}
