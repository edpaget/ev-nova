//! What the new game's own landed screens ask of its storylines: the
//! news heard in the system the ship is in ([`ScreenAsk::HeardNews`]),
//! and the active storyline missions the mission info key lists
//! ([`Statics::story_rows`], on
//! [`MissionInfo::story`](crate::MissionInfo::story)). Each reads the
//! session's own state; a session whose data holds no storylines answers
//! empty.
//!
//! [`ScreenAsk::HeardNews`]: crate::flight_frame::ScreenAsk::HeardNews

use nova_sim_catalog::storylines::{MissionObjective, StoryMission};

use crate::flight_frame::{ScreenAnswer, StoryObjective, StoryRow};
use crate::pilot::Pilot;
use crate::storylines::state::MissionState;

use super::{Session, Statics};

impl Statics {
    /// One row for each of `pilot`'s active storyline missions, by
    /// catalog id: its title, its objective with names resolved (a
    /// point's gate from the storylines' points, its holder's government
    /// name through the factions), the cargo aboard, whether it is met,
    /// and where it completes (its completion's stellar, else a
    /// delivery's). Offered, declined and done missions, and one the data
    /// no longer holds, are no rows.
    pub(super) fn story_rows(&self, pilot: &Pilot) -> Vec<StoryRow> {
        self.storylines
            .missions
            .iter()
            .filter_map(|(id, mission)| {
                let Some(MissionState::Active { cargo, met }) = pilot.story.mission(id) else {
                    return None;
                };
                Some(StoryRow {
                    mission: id.clone(),
                    title: mission.title.clone(),
                    objective: self.story_objective(mission.objective.as_ref()?),
                    cargo,
                    met,
                    complete_at: complete_at(mission),
                    storyline: mission.storyline.clone(),
                    giver: mission.offer.as_ref().and_then(|offer| offer.giver.clone()),
                })
            })
            .collect()
    }

    /// `objective` with its point and holder resolved for the screen.
    fn story_objective(&self, objective: &MissionObjective) -> StoryObjective {
        match objective {
            MissionObjective::Deliver { cargo, tons, to } => StoryObjective::Deliver {
                cargo: cargo.clone(),
                tons: *tons,
                to: *to,
            },
            MissionObjective::TakePoint { point, held_by, .. } => StoryObjective::TakePoint {
                gate: self.storylines.points.get(point).map(|at| at.gate),
                holder: self
                    .storylines
                    .factions
                    .get(held_by)
                    .and_then(|govt| self.govts.get(*govt))
                    .map_or_else(|| held_by.clone(), |record| record.name.clone()),
            },
        }
    }
}

/// Where `mission` completes: its completion's stellar, else its
/// delivery's.
fn complete_at(mission: &StoryMission) -> Option<crate::catalog::StellarId> {
    let delivery = match mission.objective {
        Some(MissionObjective::Deliver { to, .. }) => Some(to),
        _ => None,
    };
    mission
        .complete
        .as_ref()
        .and_then(|complete| complete.at)
        .or(delivery)
}

impl Session {
    /// The news heard in the system the ship is in, landed or flying
    /// ([`Session::heard_news`] of the pilot's system).
    pub(super) fn heard_news_screen(&self) -> ScreenAnswer {
        ScreenAnswer::HeardNews(self.heard_news(self.pilot.system))
    }
}

#[cfg(test)]
pub(in crate::session) mod tests {
    use nova_sim_catalog::storylines::{
        MissionCompletion, MissionGiver, MissionObjective, MissionStoryline, StoryAct, StoryMission,
    };

    use super::*;
    use crate::catalog::{GovtId, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::flight_frame::{StoryObjective, StoryRow};
    use crate::session::storylines::tests::{DOCK, closing, flying, job, land, talking};
    use crate::storylines::state::MissionState;
    use crate::testkit::{FakePilotCatalog, catalog, govt};

    /// The job, titled, 10 tons of plate to the dock, completing at 129.
    pub(in crate::session) fn hauling() -> StoryMission {
        let mut mission = StoryMission {
            title: "Plate run".to_owned(),
            ..job(10)
        };
        if let Some(complete) = &mut mission.complete {
            complete.at = Some(StellarId(129));
        }
        mission
    }

    /// Taking the point `a/b` from the warden, with no completion.
    pub(in crate::session) fn taking(held_by: &str) -> StoryMission {
        StoryMission {
            title: "Break the gate".to_owned(),
            objective: Some(MissionObjective::TakePoint {
                point: "a/b".to_owned(),
                held_by: held_by.to_owned(),
                blockade: None,
            }),
            complete: None,
            ..job(0)
        }
    }

    /// [`talking`] with `missions`, the faction `warden` the government
    /// 140 named The Wardens.
    pub(in crate::session) fn storied(missions: &[(&str, StoryMission)]) -> FakePilotCatalog {
        let mut catalog = talking(missions);
        catalog
            .storylines
            .factions
            .insert("warden".to_owned(), GovtId(140));
        catalog.govts.push(crate::catalog::GovtRecord {
            name: "The Wardens".to_owned(),
            ..govt(140)
        });
        catalog
    }

    /// A session of `catalog`'s in flight, each of `states` set.
    pub(in crate::session) fn with_states(
        catalog: &FakePilotCatalog,
        states: &[(&str, MissionState)],
    ) -> Session {
        let mut session = flying(catalog);
        for (id, state) in states {
            session
                .pilot
                .story
                .missions
                .insert((*id).to_owned(), *state);
        }
        session
    }

    pub(in crate::session) fn active(cargo: u32, met: bool) -> MissionState {
        MissionState::Active { cargo, met }
    }

    #[test]
    fn an_active_delivery_is_a_row_with_its_objective_cargo_and_completion() {
        let catalog = storied(&[("job", hauling())]);
        let session = with_states(&catalog, &[("job", active(10, false))]);
        assert_eq!(
            session.statics.story_rows(&session.pilot),
            [StoryRow {
                mission: "job".to_owned(),
                title: "Plate run".to_owned(),
                objective: StoryObjective::Deliver {
                    cargo: "Plate".to_owned(),
                    tons: 10,
                    to: StellarId(DOCK),
                },
                cargo: 10,
                met: false,
                complete_at: Some(StellarId(129)),
                storyline: None,
                giver: None,
            }]
        );
        let session = with_states(&catalog, &[("job", active(0, true))]);
        let rows = session.statics.story_rows(&session.pilot);
        assert_eq!((rows[0].cargo, rows[0].met), (0, true));
    }

    #[test]
    fn a_row_carries_its_record_s_storyline_and_giver() {
        let storyline = MissionStoryline {
            id: "dock-work".to_owned(),
            title: "Dock work".to_owned(),
            act: StoryAct::Crossroads,
        };
        let giver = MissionGiver {
            name: "Ode".to_owned(),
            role: "dockmaster".to_owned(),
        };
        let mut mission = hauling();
        mission.storyline = Some(storyline.clone());
        mission.offer.as_mut().expect("offered").giver = Some(giver.clone());
        let catalog = storied(&[("job", mission)]);
        let session = with_states(&catalog, &[("job", active(10, false))]);
        let rows = session.statics.story_rows(&session.pilot);
        assert_eq!(rows[0].storyline, Some(storyline));
        assert_eq!(rows[0].giver, Some(giver));
    }

    #[test]
    fn a_delivery_with_no_completion_place_completes_where_it_delivers() {
        let mut mission = hauling();
        mission.complete = Some(MissionCompletion {
            at: None,
            ..mission.complete.expect("a completion")
        });
        let catalog = storied(&[("job", mission.clone())]);
        let session = with_states(&catalog, &[("job", active(10, false))]);
        let rows = session.statics.story_rows(&session.pilot);
        assert_eq!(rows[0].complete_at, Some(StellarId(DOCK)));
        mission.complete = None;
        let catalog = storied(&[("job", mission)]);
        let session = with_states(&catalog, &[("job", active(10, false))]);
        let rows = session.statics.story_rows(&session.pilot);
        assert_eq!(rows[0].complete_at, Some(StellarId(DOCK)), "none at all");
    }

    #[test]
    fn a_take_point_row_names_its_gate_and_its_holder() {
        let catalog = storied(&[("road", taking("warden"))]);
        let session = with_states(&catalog, &[("road", active(0, false))]);
        let rows = session.statics.story_rows(&session.pilot);
        assert_eq!(
            rows[0].objective,
            StoryObjective::TakePoint {
                gate: Some(StellarId(140)),
                holder: "The Wardens".to_owned(),
            }
        );
        assert_eq!(rows[0].complete_at, None);
    }

    #[test]
    fn a_holder_with_no_government_is_named_by_its_faction() {
        let catalog = storied(&[("road", taking("rebels"))]);
        let session = with_states(&catalog, &[("road", active(0, false))]);
        let rows = session.statics.story_rows(&session.pilot);
        assert_eq!(
            rows[0].objective,
            StoryObjective::TakePoint {
                gate: Some(StellarId(140)),
                holder: "rebels".to_owned(),
            }
        );
    }

    #[test]
    fn only_active_missions_are_rows_in_id_order() {
        let catalog = storied(&[
            ("a-offered", hauling()),
            ("b-declined", hauling()),
            ("c-done", hauling()),
            ("d-road", taking("warden")),
            ("e-job", hauling()),
        ]);
        let session = with_states(
            &catalog,
            &[
                ("e-job", active(10, false)),
                ("a-offered", MissionState::Offered),
                ("b-declined", MissionState::Declined),
                ("c-done", MissionState::Done),
                ("d-road", active(0, false)),
            ],
        );
        let ids: Vec<String> = session
            .statics
            .story_rows(&session.pilot)
            .into_iter()
            .map(|row| row.mission)
            .collect();
        assert_eq!(ids, ["d-road", "e-job"]);
    }

    #[test]
    fn a_session_with_no_storylines_has_no_rows() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.statics.story_rows(&session.pilot), []);
    }

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
