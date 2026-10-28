//! [`Session::screen`]: the screen query, what a screen of the flight view
//! asks as it opens or as its input changes
//! ([`ScreenAsk`](crate::flight_frame::ScreenAsk)), answered with the
//! plain-data snapshot it draws from
//! ([`ScreenAnswer`](crate::flight_frame::ScreenAnswer)). Each answer is
//! the session's own reads, the ones its tests pin, put together; none
//! changes anything a frame shows, which the tests here pin, so the view
//! keeps its frame across them.

use super::Session;
use super::apply::ApplyCollaborators;
use crate::flight_frame::{FloatingMap, MapOpen, MapScreen, MarksFrom, ScreenAnswer, ScreenAsk};
use crate::galaxy_map::{GalaxyShown, MapMarks};

impl Session {
    /// Answers the screen's `ask`, handed `collab`, changing nothing a
    /// frame shows:
    ///
    /// - To [`ScreenAsk::Galaxy`], [`GalaxyShown::of`] the galaxy
    ///   ([`Session::galaxy`]).
    /// - To [`ScreenAsk::MapOpening`], the nav ([`Session::map_nav`]), the
    ///   marks its [`MarksFrom`] names ([`Session::offer_map_marks`] and
    ///   its kin), the border sources ([`Session::map_border_sources`]),
    ///   the preselection ([`Session::map_preselection`]) and the mission
    ///   systems ([`Session::map_mission_systems`]).
    /// - To [`ScreenAsk::Map`], the drawing ([`Session::map_drawing`]), the
    ///   border sources, on the hypergate map with the drawing's gate
    ///   arrows counted as known, as the view marks them known
    ///   ([`Session::mark_known`]) before it shows the borders, and the
    ///   panels ([`Session::map_panel`]).
    /// - To [`ScreenAsk::MapPanel`], the panels alone.
    /// - To [`ScreenAsk::MapPreselection`], [`Session::map_preselection`].
    /// - To [`ScreenAsk::HypergateCycle`], [`Session::hypergate_cycle`].
    /// - To [`ScreenAsk::Find`], [`Session::find_map_system`].
    /// - To [`ScreenAsk::FloatingMap`], the drawing off the hypergate map
    ///   with the mission systems now and no marks ([`MapMarks::NONE`]),
    ///   and [`Session::floating_preselection`].
    /// - To [`ScreenAsk::ComingTraffic`], [`Session::coming_traffic_ships`].
    /// - To [`ScreenAsk::PlainText`], [`Session::plain_text`].
    ///
    /// The map's redraw and panels, the hypergate cycle and the coming
    /// traffic read `collab`'s catalog, and answer
    /// [`ScreenAnswer::Nothing`] without one. It takes `&mut self` and
    /// the apply collaborators for the screens whose reads roll on the
    /// chance or list the hail's options; the maps read only the catalog.
    #[allow(clippy::needless_pass_by_value)] // the asks come by value, as the port hands them on
    pub fn screen(&mut self, ask: ScreenAsk, collab: &mut ApplyCollaborators<'_>) -> ScreenAnswer {
        let catalog = collab.catalog;
        match ask {
            ScreenAsk::Galaxy => ScreenAnswer::Galaxy(GalaxyShown::of(self.galaxy())),
            ScreenAsk::MapOpening { marks } => ScreenAnswer::MapOpening(MapOpen {
                nav: self.map_nav(),
                marks: self.marks_from(marks),
                borders: self.map_border_sources(),
                preselection: self.map_preselection(),
                missions: self.map_mission_systems(),
            }),
            ScreenAsk::Map {
                hypergate,
                missions,
                marks,
                selected,
            } => {
                let Some(catalog) = catalog else {
                    return ScreenAnswer::Nothing;
                };
                let drawing = self.map_drawing(hypergate.as_deref(), &missions, marks);
                let borders = self.chart.map_border_sources_knowing(
                    &self.pilot,
                    &self.statics,
                    &self.setup,
                    &drawing.gate_arrows,
                );
                let panel = self.map_panel(&catalog, selected, hypergate.as_deref());
                ScreenAnswer::Map(Box::new(MapScreen {
                    drawing,
                    borders,
                    panel,
                }))
            }
            ScreenAsk::MapPanel {
                selected,
                hypergate,
            } => catalog.map_or(ScreenAnswer::Nothing, |catalog| {
                ScreenAnswer::MapPanel(Box::new(self.map_panel(
                    &catalog,
                    selected,
                    hypergate.as_deref(),
                )))
            }),
            ScreenAsk::MapPreselection => ScreenAnswer::MapPreselection(self.map_preselection()),
            ScreenAsk::HypergateCycle => catalog.map_or(ScreenAnswer::Nothing, |catalog| {
                ScreenAnswer::HypergateCycle(self.hypergate_cycle(&catalog))
            }),
            ScreenAsk::Find(query) => ScreenAnswer::Found(self.find_map_system(&query)),
            ScreenAsk::FloatingMap => {
                let missions = self.map_mission_systems();
                ScreenAnswer::FloatingMap(Box::new(FloatingMap {
                    drawing: self.map_drawing(None, &missions, MapMarks::NONE),
                    preselection: self.floating_preselection(),
                }))
            }
            ScreenAsk::ComingTraffic => catalog.map_or(ScreenAnswer::Nothing, |catalog| {
                ScreenAnswer::ComingTraffic(self.coming_traffic_ships(catalog))
            }),
            ScreenAsk::PlainText(text) => ScreenAnswer::PlainText(self.plain_text(&text)),
        }
    }

    /// The marks `from` names: those given, or the dialog's
    /// ([`Session::offer_map_marks`], [`Session::computer_map_marks`],
    /// [`Session::mission_info_map_marks`]).
    fn marks_from(&self, from: MarksFrom) -> MapMarks {
        match from {
            MarksFrom::Given(marks) => marks,
            MarksFrom::Offer(mission) => self.offer_map_marks(mission),
            MarksFrom::Computer(selected) => self.computer_map_marks(selected),
            MarksFrom::MissionInfo(slot) => self.mission_info_map_marks(slot),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtId, MissionId, StarSystem, SystemId};
    use crate::chance::NeverFires;
    use crate::flight_frame::FlightFrame;
    use crate::hail::HailOptions;
    use crate::landing::StellarFlags;
    use crate::map_search::SearchOutcome;
    use crate::session::frame::tests::filled;
    use crate::session::mission_map::tests::{holding, landed, marking};
    use crate::session::tests::{at_gate, gated, hide, trafficked, with_alternate};
    use crate::testkit::{FakePilotCatalog, catalog, govt};

    /// `session`'s answer to `ask`, handed `catalog` if any, on a chance
    /// that never fires.
    fn screen(
        session: &mut Session,
        catalog: Option<&FakePilotCatalog>,
        ask: ScreenAsk,
    ) -> ScreenAnswer {
        session.screen(
            ask,
            &mut ApplyCollaborators {
                catalog: catalog.map(|catalog| catalog as &dyn crate::SimCatalog),
                chance: &mut NeverFires,
                law: &crate::legal::NovaLaw::default(),
                boarding_rule: &crate::board::NovaBoarding::default(),
                hail_options: &HailOptions::default(),
            },
        )
    }

    /// What `session` reads as a map opens marked `marks`, by hand.
    fn opening(session: &Session, marks: MapMarks) -> ScreenAnswer {
        ScreenAnswer::MapOpening(MapOpen {
            nav: session.map_nav(),
            marks,
            borders: session.map_border_sources(),
            preselection: session.map_preselection(),
            missions: session.map_mission_systems(),
        })
    }

    #[test]
    fn the_galaxy_is_what_the_map_shows_of_the_sessions_galaxy() {
        let mut catalog = catalog();
        with_alternate(&mut catalog);
        let mut session = Session::start(&catalog).expect("starts");
        assert!(hide(&mut session, &catalog, &[131]), "precondition: hidden");
        let shown = GalaxyShown::of(session.galaxy());
        assert!(
            !shown.visible(SystemId(131)),
            "precondition: the overlay read"
        );
        assert_eq!(
            screen(&mut session, None, ScreenAsk::Galaxy),
            ScreenAnswer::Galaxy(shown),
            "no catalog read"
        );
    }

    #[test]
    fn the_plain_maps_opening_is_the_sessions_reads_with_the_marks_given() {
        let catalog = marking();
        let mut session = landed(&catalog);
        holding(&mut session, 300, Some(140), Some(150));
        assert!(
            !session.map_mission_systems().is_empty(),
            "precondition: a mission system"
        );
        for marks in [
            MapMarks::NONE,
            MapMarks {
                green: Some(SystemId(132)),
                multi_arrows: false,
            },
        ] {
            assert_eq!(
                screen(
                    &mut session,
                    None,
                    ScreenAsk::MapOpening {
                        marks: MarksFrom::Given(marks),
                    }
                ),
                opening(&session, marks),
                "{marks:?}"
            );
        }
    }

    #[test]
    fn a_dialogs_maps_opening_takes_its_marks_from_the_session() {
        let catalog = marking();
        let mut session = landed(&catalog);
        let slot = holding(&mut session, 300, Some(140), Some(150));
        let offer = session.offer_map_marks(MissionId(300));
        assert_ne!(offer, MapMarks::NONE, "precondition: an offer marked");
        let info = session.mission_info_map_marks(Some(slot));
        assert_ne!(info, MapMarks::NONE, "precondition: mission info marked");
        for (from, marks) in [
            (MarksFrom::Offer(MissionId(300)), offer),
            (
                MarksFrom::Computer(Some(MissionId(300))),
                session.computer_map_marks(Some(MissionId(300))),
            ),
            (MarksFrom::Computer(None), session.computer_map_marks(None)),
            (MarksFrom::MissionInfo(Some(slot)), info),
            (
                MarksFrom::MissionInfo(None),
                session.mission_info_map_marks(None),
            ),
        ] {
            assert_eq!(
                screen(&mut session, None, ScreenAsk::MapOpening { marks: from }),
                opening(&session, marks),
                "{from:?}"
            );
        }
    }

    #[test]
    fn the_course_maps_redraw_is_the_sessions_drawing_borders_and_panels() {
        let catalog = marking();
        let mut session = landed(&catalog);
        holding(&mut session, 300, Some(140), Some(150));
        session.plot_course(SystemId(131)).expect("a route");
        let missions = session.map_mission_systems();
        let marks = session.offer_map_marks(MissionId(300));
        let selected = Some(SystemId(131));
        let expected = MapScreen {
            drawing: session.map_drawing(None, &missions, marks),
            borders: session.map_border_sources(),
            panel: session.map_panel(&catalog, selected, None),
        };
        assert_eq!(
            screen(
                &mut session,
                Some(&catalog),
                ScreenAsk::Map {
                    hypergate: None,
                    missions,
                    marks,
                    selected,
                }
            ),
            ScreenAnswer::Map(Box::new(expected))
        );
    }

    /// [`gated`] with 131, the hypergate's first offer, the Federation's
    /// (128) and inhabited, so its borders add once it is known.
    fn gated_federation() -> FakePilotCatalog {
        let mut catalog = gated();
        let home = Some((StellarFlags::CAN_LAND, 0));
        let at = catalog
            .star_map
            .iter()
            .position(|system| system.id == SystemId(131))
            .expect("131");
        catalog.star_map[at] = StarSystem {
            govt: Some(GovtId(128)),
            stellars: vec![home],
            ..catalog.star_map[at].clone()
        };
        catalog.govts = vec![govt(128)];
        catalog
    }

    #[test]
    fn the_hypergate_maps_redraw_counts_its_gate_arrows_known_and_marks_none() {
        let catalog = gated_federation();
        let mut session = at_gate(&catalog);
        session.pilot.explore(SystemId(131));
        session.chart.known.remove(&SystemId(131));
        let offers = vec![SystemId(131), SystemId(132)];
        let drawing = session.map_drawing(Some(&offers), &[], MapMarks::NONE);
        let mut marked = session.clone();
        marked.mark_known(drawing.gate_arrows.clone());
        assert_ne!(
            marked.map_border_sources(),
            session.map_border_sources(),
            "precondition: 131 adds once known"
        );
        let selected = Some(SystemId(131));
        let expected = MapScreen {
            drawing,
            borders: marked.map_border_sources(),
            panel: marked.map_panel(&catalog, selected, Some(&offers)),
        };
        let known = session.known().clone();
        assert_eq!(
            screen(
                &mut session,
                Some(&catalog),
                ScreenAsk::Map {
                    hypergate: Some(offers),
                    missions: Vec::new(),
                    marks: MapMarks::NONE,
                    selected,
                }
            ),
            ScreenAnswer::Map(Box::new(expected))
        );
        assert_eq!(*session.known(), known, "the view marks them, not the ask");
    }

    #[test]
    fn the_panels_alone_are_the_sessions_for_the_system_selected() {
        let catalog = gated_federation();
        let mut session = at_gate(&catalog);
        session.pilot.explore(SystemId(131));
        let offers = vec![SystemId(131), SystemId(132)];
        for (selected, hypergate) in [
            (Some(SystemId(131)), None),
            (Some(SystemId(131)), Some(offers.clone())),
            (None, None),
        ] {
            let expected = session.map_panel(&catalog, selected, hypergate.as_deref());
            assert_eq!(
                screen(
                    &mut session,
                    Some(&catalog),
                    ScreenAsk::MapPanel {
                        selected,
                        hypergate: hypergate.clone(),
                    }
                ),
                ScreenAnswer::MapPanel(Box::new(expected)),
                "{selected:?} {hypergate:?}"
            );
        }
    }

    #[test]
    fn the_preselection_after_a_cycle_is_the_sessions() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        let preselection = session.map_preselection();
        assert_eq!(preselection, Some(SystemId(131)), "precondition: mode 3's");
        assert_eq!(
            screen(&mut session, None, ScreenAsk::MapPreselection),
            ScreenAnswer::MapPreselection(preselection)
        );
    }

    #[test]
    fn the_hypergate_cycle_is_the_sessions_over_the_catalog() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        let cycle = session.hypergate_cycle(&catalog);
        assert!(!cycle.is_empty(), "precondition: destinations");
        assert_eq!(
            screen(&mut session, Some(&catalog), ScreenAsk::HypergateCycle),
            ScreenAnswer::HypergateCycle(cycle)
        );
    }

    #[test]
    fn find_finds_what_the_session_finds() {
        let mut session = Session::start(&catalog()).expect("starts");
        for query in ["", "Sol", "nowhere"] {
            assert_eq!(
                screen(&mut session, None, ScreenAsk::Find(query.to_owned())),
                ScreenAnswer::Found(session.find_map_system(query)),
                "{query}"
            );
        }
        assert_ne!(
            session.find_map_system("nowhere"),
            SearchOutcome::Found(SystemId(130)),
            "precondition: not the current system"
        );
    }

    #[test]
    fn the_floating_map_draws_the_mission_systems_unmarked_with_its_preselection() {
        let catalog = marking();
        let mut session = landed(&catalog);
        holding(&mut session, 300, Some(140), Some(150));
        session.plot_course(SystemId(131)).expect("a route");
        let missions = session.map_mission_systems();
        assert!(!missions.is_empty(), "precondition: a mission system");
        let preselection = session.floating_preselection();
        assert_eq!(preselection, Some(SystemId(131)), "precondition: mode 3's");
        let expected = FloatingMap {
            drawing: session.map_drawing(None, &missions, MapMarks::NONE),
            preselection,
        };
        assert_eq!(
            screen(&mut session, None, ScreenAsk::FloatingMap),
            ScreenAnswer::FloatingMap(Box::new(expected)),
            "no catalog read"
        );
    }

    #[test]
    fn the_coming_traffic_answers_the_ships_due_and_then_those_the_system_was_filled_with() {
        let trafficked = trafficked(130, 2, 1);
        let mut session = Session::start(&trafficked).expect("starts");
        let due = session.coming_traffic_ships(&trafficked);
        assert!(!due.is_empty(), "precondition: the table's ships");
        assert_ne!(due, session.traffic_ships(), "precondition: not yet filled");
        assert_eq!(
            screen(&mut session, Some(&trafficked), ScreenAsk::ComingTraffic),
            ScreenAnswer::ComingTraffic(due)
        );
        session.populate(&trafficked, &mut NeverFires);
        assert_eq!(
            screen(&mut session, Some(&catalog()), ScreenAsk::ComingTraffic),
            ScreenAnswer::ComingTraffic(session.traffic_ships()),
            "once filled, the system's own"
        );
    }

    #[test]
    fn a_plain_text_is_filled_as_the_session_fills_it_for_the_pilot() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.ship_name = Some("Kestrel".to_owned());
        let raw = "Hello, <PSN>.";
        let expected = session.plain_text(raw);
        assert_eq!(expected, "Hello, Kestrel.", "precondition: a tag filled");
        assert_eq!(
            screen(&mut session, None, ScreenAsk::PlainText(raw.to_owned())),
            ScreenAnswer::PlainText(expected)
        );
    }

    /// One ask of each kind, over `gated`'s offers.
    fn every_ask() -> Vec<ScreenAsk> {
        let offers = vec![SystemId(131), SystemId(132)];
        vec![
            ScreenAsk::Galaxy,
            ScreenAsk::MapOpening {
                marks: MarksFrom::Offer(MissionId(300)),
            },
            ScreenAsk::Map {
                hypergate: None,
                missions: vec![SystemId(131)],
                marks: MapMarks::NONE,
                selected: Some(SystemId(131)),
            },
            ScreenAsk::Map {
                hypergate: Some(offers.clone()),
                missions: Vec::new(),
                marks: MapMarks::NONE,
                selected: Some(SystemId(131)),
            },
            ScreenAsk::MapPanel {
                selected: Some(SystemId(131)),
                hypergate: Some(offers),
            },
            ScreenAsk::MapPreselection,
            ScreenAsk::HypergateCycle,
            ScreenAsk::Find("Sol".to_owned()),
            ScreenAsk::FloatingMap,
            ScreenAsk::ComingTraffic,
            ScreenAsk::PlainText("Hello, <PSN>.".to_owned()),
        ]
    }

    #[test]
    fn the_asks_that_read_a_catalog_answer_nothing_without_one() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        for ask in [
            ScreenAsk::Map {
                hypergate: None,
                missions: Vec::new(),
                marks: MapMarks::NONE,
                selected: None,
            },
            ScreenAsk::MapPanel {
                selected: None,
                hypergate: None,
            },
            ScreenAsk::HypergateCycle,
            ScreenAsk::ComingTraffic,
        ] {
            let sent = format!("{ask:?}");
            assert_eq!(
                screen(&mut session, None, ask),
                ScreenAnswer::Nothing,
                "{sent}"
            );
        }
    }

    #[test]
    fn a_screen_ask_changes_nothing_a_frame_shows() {
        let catalog = gated_federation();
        let mut session = at_gate(&catalog);
        session.pilot.explore(SystemId(131));
        session.chart.known.remove(&SystemId(131));
        for ask in every_ask() {
            let before: FlightFrame = filled(&session);
            let sent = format!("{ask:?}");
            let answer = screen(&mut session, Some(&catalog), ask.clone());
            assert!(answer.answers(&ask), "{sent}: {answer:?}");
            assert_ne!(answer, ScreenAnswer::Nothing, "{sent}");
            assert_eq!(filled(&session), before, "{sent}");
        }
    }
}
