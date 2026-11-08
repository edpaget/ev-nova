//! Keeping the galaxy's gates in step with the jump points as play leaves
//! them (`play-the-sample` phase 1).
//!
//! The new game's story sets a jump point's stability and owner
//! ([`StoryState::points`](crate::storylines::state::StoryState::points)).
//! The galaxy reads each point's gate from them
//! ([`gate_points`]: its lanes, its gates' links, the closed-gate and
//! government stamps on the sites; see `nova_sim_story`'s
//! `galaxy_points` module), and [`Chart::sync_points`] hands them over:
//! on a change it rebuilds the lanes and the map, restamps the sites the
//! chart holds and tells [`SessionEvent::GalaxyChanged`]; with no change
//! it does nothing and tells nothing. Either way it then freezes the
//! reach of each news item released since, over the lanes as they now
//! are ([`news::freeze`]): a news item released in the effect block that
//! closes a road never crosses it, and a system that has heard an item
//! keeps it when a lane closes later. A saved item with no reach, from a
//! save made before reaches were kept, is frozen at the load.
//!
//! **When.** At the load, before the first frame, so a saved pilot's
//! lanes and gates follow its saved points; and wherever the storylines
//! play, as they settle ([`Session::settle_storylines`]): the end of every
//! entry that plays them, where their outfit changes are made, so the
//! points settle before any frame, dialog or save. A course that crosses
//! a lane that closes is kept, not cut: the autopilot lets go at that hop
//! (no gate the system shows serves it), and the planner won't plot over
//! it again.
//!
//! **What the player meets.** A hypergate whose links are all closed is
//! refused at the land key as a rank-barred one (`LandingRefusal::Denied`,
//! with [`SimSound::TrafficControl`](crate::sound::SimSound::TrafficControl)
//! and "Hypergate usage denied."), which ends the autopilot; the
//! hypergate entry offers and takes open links alone
//! ([`Session::open_hypergate`], [`Session::enter_hypergate`]), so a point
//! that closes between the clearance and the entry cancels it.
//!
//! The stock game has no storylines, so no points: nothing changes.

use super::{Chart, Outbox, Session, Statics};
use crate::chance::Chance;
use crate::pilot::Pilot;
use crate::session::events::SessionEvent;
use crate::storylines::gate_points::gate_points;
use crate::storylines::news;

impl Chart {
    /// Hands the galaxy `pilot`'s jump points over `statics`' storylines
    /// and says whether that changed anything: on a change the sites are
    /// restamped and [`SessionEvent::GalaxyChanged`] told on `out`. Then,
    /// over the lanes as they now are, it freezes the reach of each news
    /// item released with none ([`news::freeze`]; see the module docs).
    pub(super) fn sync_points(
        &mut self,
        statics: &Statics,
        pilot: &mut Pilot,
        out: &mut Outbox,
    ) -> bool {
        let points = gate_points(&statics.storylines, &pilot.story);
        let changed = self.galaxy.set_gate_points(points);
        if changed {
            self.stamp_sites();
            out.emit(SessionEvent::GalaxyChanged);
        }
        news::freeze(&statics.storylines, &mut pilot.story, self.galaxy.lanes());
        changed
    }
}

impl Session {
    /// Settles what the storylines played in the entry just run: the jump
    /// points ([`Chart::sync_points`]), then the outfit changes they
    /// queued ([`Session::apply_story_outfits`]), drawing on `chance`.
    pub(super) fn settle_storylines(&mut self, chance: &mut dyn Chance) {
        self.chart
            .sync_points(&self.statics, &mut self.pilot, &mut self.out);
        self.apply_story_outfits(chance);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtId, StellarId};
    use crate::flight::Controls;
    use crate::flight_frame::{Answer, FlightCommand};
    use crate::gate::{GateKind, GateRefusal};
    use crate::landing::{LandOutcome, LandingRefusal};
    use crate::navigation::NavMode;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::LandPress;
    use crate::sound::SimSound;
    use crate::storylines::state::PointChange;
    use crate::testkit::{DRIFT, FakePilotCatalog, HEARTH, VERGE, catalog, laned, laned_stable};
    use nova_sim_catalog::storylines::PointStability;

    /// The gate in drift to hearth, which the pilot starts over.
    const TO_HEARTH: StellarId = StellarId(300);

    /// The rules of the new game's travel: the gate lanes, the gate
    /// course and the gate autopilot by their other readings.
    fn rules() -> Rulebook {
        Rulebook::default()
            .with_override(RuleKey::GateLanes, RuleSource::Bible)
            .with_override(RuleKey::GateCourse, RuleSource::Bible)
            .with_override(RuleKey::GateAutopilot, RuleSource::Bible)
    }

    /// A new pilot over `catalog`, in drift over gate 300, by [`rules`].
    fn started(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start_ruled(catalog, &rules()).expect("starts");
        session.take_events();
        session.take_sounds();
        session
    }

    /// `session`'s pilot with the point `point` (`system/point`) changed
    /// to `stability`.
    fn set(session: &mut Session, point: &str, stability: PointStability) {
        session.pilot.story.points.insert(
            point.to_owned(),
            PointChange {
                owner: None,
                stability: Some(stability),
            },
        );
    }

    fn sync(session: &mut Session) -> bool {
        session
            .chart
            .sync_points(&session.statics, &mut session.pilot, &mut session.out)
    }

    fn site(session: &Session, id: StellarId) -> crate::catalog::LandingSite {
        *session
            .chart
            .sites
            .iter()
            .find(|site| site.id == id)
            .expect("shown")
    }

    fn apply(session: &mut Session, command: FlightCommand) -> Answer {
        session.apply(
            command,
            &mut crate::session::apply::ApplyCollaborators {
                catalog: Some(&laned_stable()),
                chance: &mut crate::chance::NeverFires,
                law: &crate::legal::NovaLaw::default(),
                boarding_rule: &crate::board::NovaBoarding::default(),
                hail_options: &crate::hail::HailOptions::default(),
            },
        )
    }

    fn is_denied(answer: &Answer) -> bool {
        matches!(
            answer,
            Answer::LandRefused {
                refusal: LandingRefusal::Denied { stellar, .. },
                ..
            } if *stellar == TO_HEARTH
        )
    }

    #[test]
    fn a_change_tells_the_galaxy_changed_once_and_none_tells_nothing() {
        let mut session = started(&laned_stable());
        assert!(!sync(&mut session), "nothing changed");
        assert_eq!(session.take_events(), []);
        set(&mut session, "hearth/drift-road", PointStability::Dead);
        assert!(sync(&mut session));
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
        assert!(!sync(&mut session), "the same points again");
        assert_eq!(session.take_events(), []);
        assert_eq!(session.galaxy().lanes().gates(DRIFT, HEARTH), []);
        assert!(site(&session, TO_HEARTH).gate_closed, "restamped");
    }

    #[test]
    fn the_storylines_settling_syncs_the_points() {
        let mut session = started(&laned_stable());
        set(&mut session, "hearth/drift-road", PointStability::Unstable);
        session.settle_storylines(&mut crate::chance::NeverFires);
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
        assert!(site(&session, TO_HEARTH).gate_closed);
    }

    #[test]
    fn a_load_reads_the_saved_points_before_the_first_frame() {
        let catalog = laned_stable();
        let mut pilot = crate::pilot::Pilot::new(&catalog, "").expect("a pilot");
        pilot.story.points.insert(
            "hearth/drift-road".to_owned(),
            PointChange {
                owner: None,
                stability: Some(PointStability::Dead),
            },
        );
        let mut session = Session::load(&catalog, pilot, &rules()).expect("loads");
        assert!(site(&session, TO_HEARTH).gate_closed);
        assert_eq!(session.galaxy().lanes().closed(DRIFT), [HEARTH]);
        assert!(
            !session.take_events().contains(&SessionEvent::GalaxyChanged),
            "a load tells nothing"
        );
    }

    /// [`laned_stable`] with the news item `word`, breaking in drift.
    fn with_news() -> FakePilotCatalog {
        let mut catalog = laned_stable();
        catalog.storylines.news.insert(
            "word".to_owned(),
            nova_sim_catalog::storylines::NewsItem {
                headline: "Word".to_owned(),
                text: "It spreads.".to_owned(),
                from: DRIFT,
            },
        );
        catalog
    }

    #[test]
    fn news_released_with_a_closing_is_frozen_after_it() {
        let mut session = started(&with_news());
        set(&mut session, "hearth/drift-road", PointStability::Dead);
        session.pilot.story.news.insert("word".to_owned(), 0);
        session.settle_storylines(&mut crate::chance::NeverFires);
        assert_eq!(
            session.pilot.story.reach["word"],
            std::collections::BTreeMap::from([(DRIFT, 0), (VERGE, 1), (HEARTH, 2)]),
            "round by verge, not the road just closed"
        );
        assert_eq!(session.heard_news(HEARTH), [], "two lanes off");
    }

    #[test]
    fn a_load_freezes_a_saved_item_with_no_reach() {
        let catalog = with_news();
        let mut pilot = crate::pilot::Pilot::new(&catalog, "").expect("a pilot");
        pilot.story.news.insert("word".to_owned(), 0);
        let session = Session::load(&catalog, pilot, &rules()).expect("loads");
        assert_eq!(
            session.pilot.story.reach["word"],
            std::collections::BTreeMap::from([(DRIFT, 0), (HEARTH, 1), (VERGE, 1)])
        );
    }

    #[test]
    fn the_fixtures_closed_points_close_their_gates_from_the_start() {
        let session = started(&laned());
        assert_eq!(
            session.galaxy().lanes().gates(DRIFT, VERGE),
            [],
            "the dead pair and the unstable back door"
        );
        assert!(site(&session, StellarId(301)).gate_closed);
        assert!(site(&session, StellarId(302)).gate_closed);
        assert!(!site(&session, TO_HEARTH).gate_closed);
    }

    #[test]
    fn the_player_is_denied_a_closed_gate_and_cleared_once_it_opens() {
        let mut session = started(&laned_stable());
        set(&mut session, "hearth/drift-road", PointStability::Dead);
        sync(&mut session);
        session.chart.set_nav(NavMode::Stellar(Some(TO_HEARTH)));
        let pressed = apply(&mut session, FlightCommand::Land);
        assert!(is_denied(&pressed), "{pressed:?}");
        assert!(session.take_sounds().contains(&SimSound::TrafficControl));
        assert_eq!(session.dock_request(), None, "the request dropped");
        set(&mut session, "hearth/drift-road", PointStability::Stable);
        sync(&mut session);
        assert!(matches!(
            apply(&mut session, FlightCommand::Land),
            Answer::LandPress(LandPress::Outcome(LandOutcome::Requested { .. }))
        ));
        session.dock_approach();
        assert_eq!(
            apply(&mut session, FlightCommand::Land),
            Answer::LandPress(LandPress::AtGate {
                stellar: TO_HEARTH,
                kind: GateKind::Hypergate,
            })
        );
        let entered = session.enter_hypergate(
            Some(HEARTH),
            &laned_stable(),
            &mut crate::chance::NeverFires,
        );
        assert_eq!(entered, Ok(HEARTH));
    }

    /// `session` at gate 300's entry: requested, approached and pressed
    /// again, the entry awaiting its pick.
    fn at_the_entry(session: &mut Session) {
        session.chart.set_nav(NavMode::Stellar(Some(TO_HEARTH)));
        apply(session, FlightCommand::Land);
        session.dock_approach();
        assert_eq!(
            apply(session, FlightCommand::Land),
            Answer::LandPress(LandPress::AtGate {
                stellar: TO_HEARTH,
                kind: GateKind::Hypergate,
            })
        );
    }

    #[test]
    fn a_point_closing_between_the_clearance_and_the_entry_cancels_it() {
        let mut session = started(&laned_stable());
        at_the_entry(&mut session);
        set(&mut session, "drift/hearth-road", PointStability::Unstable);
        sync(&mut session);
        assert_eq!(session.open_hypergate(&laned_stable()), Ok(Vec::new()));
        assert_eq!(
            session.enter_hypergate(
                Some(HEARTH),
                &laned_stable(),
                &mut crate::chance::NeverFires
            ),
            Err(GateRefusal::Cancelled)
        );
        assert_eq!(session.pilot().system, DRIFT, "still in drift");
    }

    #[test]
    fn the_entry_offers_the_open_links_alone() {
        let mut session = started(&laned_stable());
        at_the_entry(&mut session);
        assert_eq!(session.open_hypergate(&laned_stable()), Ok(vec![HEARTH]));
    }

    /// [`started`] over [`laned_stable`] with the course drift → hearth →
    /// verge cued, J pressed: the autopilot flies to gate 300.
    fn along_the_course() -> Session {
        let mut session = started(&laned_stable());
        session.pilot.course = vec![HEARTH, VERGE];
        session.pilot.course_start = Some(DRIFT);
        session.cue_course();
        session.take_sounds();
        assert_eq!(
            apply(&mut session, FlightCommand::Jump),
            Answer::GatePilot(HEARTH)
        );
        session
    }

    #[test]
    fn the_autopilot_lets_go_when_its_hops_lane_closes_and_keeps_the_course() {
        let mut session = along_the_course();
        session.pilot_controls(Controls::default());
        assert!(session.gate_pilot_engaged());
        set(&mut session, "hearth/drift-road", PointStability::Dead);
        sync(&mut session);
        session.pilot_controls(Controls::default());
        assert!(!session.gate_pilot_engaged());
        assert_eq!(session.course(), [HEARTH, VERGE]);
    }

    #[test]
    fn the_autopilot_lets_go_when_its_own_press_is_denied() {
        let mut session = along_the_course();
        session.pilot_controls(Controls::default());
        set(&mut session, "drift/hearth-road", PointStability::Dead);
        sync(&mut session);
        let pressed = apply(&mut session, FlightCommand::Land);
        assert!(is_denied(&pressed), "{pressed:?}");
        assert!(!session.gate_pilot_engaged());
    }

    #[test]
    fn the_autopilot_lets_go_when_its_entry_is_cancelled() {
        let mut session = along_the_course();
        session.pilot_controls(Controls::default());
        apply(&mut session, FlightCommand::Land);
        session.dock_approach();
        session.pilot_controls(Controls::default());
        apply(&mut session, FlightCommand::Land);
        set(&mut session, "drift/hearth-road", PointStability::Dead);
        sync(&mut session);
        assert_eq!(
            apply(&mut session, FlightCommand::EnterHypergate(Some(HEARTH))),
            Answer::GateRefused(GateRefusal::Cancelled)
        );
        assert!(!session.gate_pilot_engaged());
        assert_eq!(session.pilot().system, DRIFT);
    }

    #[test]
    fn a_points_owner_restamps_its_gates_government() {
        let mut catalog = laned_stable();
        catalog
            .storylines
            .factions
            .insert("warden".to_owned(), GovtId(129));
        let mut session = started(&catalog);
        assert_eq!(site(&session, TO_HEARTH).govt, None);
        session.pilot.story.points.insert(
            "drift/hearth-road".to_owned(),
            PointChange {
                owner: Some("warden".to_owned()),
                stability: None,
            },
        );
        assert!(sync(&mut session));
        assert_eq!(site(&session, TO_HEARTH).govt, Some(GovtId(129)));
        assert!(!site(&session, TO_HEARTH).gate_closed);
    }

    #[test]
    fn with_no_storylines_nothing_changes() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.take_events();
        let before = session.chart.clone();
        assert!(!sync(&mut session));
        assert_eq!(session.take_events(), []);
        assert_eq!(session.chart, before);
        assert!(session.chart.sites.iter().all(|site| !site.gate_closed));
    }
}
