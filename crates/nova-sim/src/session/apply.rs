//! [`Session::apply`]: the player's [`FlightCommand`]s, as the flight
//! view's `FlightSim` port sends them, each matched to the session method
//! that runs it, and its [`Answer`] (see
//! [`flight_frame`](crate::flight_frame)). The match lives here, on the
//! session's side, so the view's adapter only forwards.

use super::Session;
use crate::flight_frame::{Answer, FlightCommand};

impl Session {
    /// Runs `command` and answers it: [`Answer::Refused`] when it did
    /// nothing.
    ///
    /// - [`FlightCommand::SelectStellar`] runs [`Session::select_stellar`],
    ///   answered with the stellar selected, or refused during a jump and
    ///   when the pick finds nothing.
    /// - [`FlightCommand::SelectNextSystem`] runs
    ///   [`Session::select_next_system`], answered with the system
    ///   selected, `None` when it let go of the selection, or refused
    ///   during a jump.
    /// - [`FlightCommand::ClearTarget`] runs [`Session::clear_target`],
    ///   refused when there was no target.
    /// - [`FlightCommand::SelectTarget`] runs [`Session::select_target`],
    ///   answered with the target it leaves, or refused while landed,
    ///   where the keys change nothing and are silent.
    /// - [`FlightCommand::NavModeKeys`] runs [`Session::nav_mode_keys`],
    ///   refused during a jump.
    /// - [`FlightCommand::MapClick`] runs [`Session::map_click`], answered
    ///   with what it did, or refused when no hit is accepted and the map
    ///   drags.
    /// - [`FlightCommand::CycleMapSelection`] runs
    ///   [`Session::cycle_map_selection`], refused docked.
    /// - [`FlightCommand::SelectOnFloatingMap`] runs
    ///   [`Session::select_on_floating_map`], refused during a jump.
    /// - [`FlightCommand::ClickInSpace`] runs [`Session::click_in_space`],
    ///   answered with what it took, or refused when it changed nothing:
    ///   landed, or a click on nothing it takes.
    /// - The rest are never refused, and answer [`Answer::Done`]:
    ///   [`FlightCommand::ClearRoute`] ([`Session::clear_route`]),
    ///   [`FlightCommand::MarkKnown`] ([`Session::mark_known`]),
    ///   [`FlightCommand::CueCourse`] ([`Session::cue_course`]), and the
    ///   nav's saves and restores: [`Session::save_port_nav`],
    ///   [`Session::restore_port_nav`], [`Session::restore_bar_nav`],
    ///   [`Session::put_back_port_nav`], [`Session::restore_map_nav`] and
    ///   [`Session::restore_map_selection`].
    #[allow(clippy::needless_pass_by_value)] // the port sends commands by value, and later ones carry owned data
    pub fn apply(&mut self, command: FlightCommand) -> Answer {
        match command {
            FlightCommand::SelectStellar(pick) => self
                .select_stellar(pick)
                .map_or(Answer::Refused, Answer::Stellar),
            FlightCommand::SelectNextSystem => {
                if self.mode.jump().is_some() {
                    Answer::Refused
                } else {
                    Answer::System(self.select_next_system())
                }
            }
            FlightCommand::ClearTarget => done_if(self.clear_target()),
            FlightCommand::SelectTarget(pick) => {
                if self.mode.landed().is_some() {
                    Answer::Refused
                } else {
                    Answer::Target(self.select_target(pick))
                }
            }
            FlightCommand::NavModeKeys { nav_off, h } => done_if(self.nav_mode_keys(nav_off, h)),
            FlightCommand::MapClick(click) => self
                .map_click(&click.press())
                .map_or(Answer::Refused, Answer::Clicked),
            FlightCommand::CycleMapSelection => done_if(self.cycle_map_selection()),
            FlightCommand::ClearRoute => {
                self.clear_route();
                Answer::Done
            }
            FlightCommand::SelectOnFloatingMap(link) => done_if(self.select_on_floating_map(link)),
            FlightCommand::MarkKnown(systems) => {
                self.mark_known(systems);
                Answer::Done
            }
            FlightCommand::CueCourse => {
                self.cue_course();
                Answer::Done
            }
            FlightCommand::SavePortNav => {
                self.save_port_nav();
                Answer::Done
            }
            FlightCommand::RestorePortNav => {
                self.restore_port_nav();
                Answer::Done
            }
            FlightCommand::RestoreBarNav => {
                self.restore_bar_nav();
                Answer::Done
            }
            FlightCommand::PutBackPortNav => {
                self.put_back_port_nav();
                Answer::Done
            }
            FlightCommand::RestoreMapNav(saved) => {
                self.restore_map_nav(saved);
                Answer::Done
            }
            FlightCommand::RestoreMapSelection(saved) => {
                self.restore_map_selection(saved);
                Answer::Done
            }
            FlightCommand::ClickInSpace { at, drawn } => self
                .click_in_space(at, &drawn)
                .map_or(Answer::Refused, Answer::InSpace),
        }
    }
}

/// [`Answer::Done`] when the command ran, [`Answer::Refused`] when it did
/// nothing.
fn done_if(ran: bool) -> Answer {
    if ran { Answer::Done } else { Answer::Refused }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{StellarId, SystemId};
    use crate::flight_frame::MapClick;
    use crate::navigation::NavMode;
    use crate::session::MapPress;
    use crate::session::StellarPick;
    use crate::session::tests::three_stellars;
    use crate::sound::SimSound;
    use crate::stats::ShipStats;
    use crate::targeting::TargetPick;
    use crate::testkit::{begin_jump_now, catalog, fly_out, land_now, npc};
    use crate::traffic::npc::NpcId;

    /// A session started on `catalog`, its sounds so far taken.
    fn quiet(catalog: &crate::testkit::FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.take_sounds();
        session
    }

    /// A session flying a jump to Alpha Centauri, its sounds so far taken.
    fn jumping() -> Session {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        session.take_sounds();
        session
    }

    /// A session landed on Earth (128), its sounds so far taken.
    fn landed() -> Session {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_stellar(StellarPick::Slot(1));
        land_now(&mut session).expect("lands");
        session.take_sounds();
        session
    }

    /// Adds an idle NPC at (50, 0), giving its number.
    fn ship(session: &mut Session) -> NpcId {
        let mut ship = npc(0, ShipStats::default());
        ship.state.position = crate::geometry::Vec2::new(50.0, 0.0);
        session.space.traffic.add_npc(ship)
    }

    #[test]
    fn hyper_select_answers_the_system_it_selects() {
        let mut session = quiet(&catalog());
        assert_eq!(
            session.apply(FlightCommand::SelectNextSystem),
            Answer::System(Some(SystemId(131)))
        );
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(131))));
        assert_eq!(session.take_sounds(), [SimSound::HyperSelected]);
    }

    #[test]
    fn hyper_select_with_nothing_to_offer_answers_none_and_lets_go() {
        let mut catalog = catalog();
        catalog.star_map[0].links.clear();
        let mut session = quiet(&catalog);
        session.select_stellar(StellarPick::Slot(0));
        session.take_sounds();
        assert_eq!(
            session.apply(FlightCommand::SelectNextSystem),
            Answer::System(None)
        );
        assert_eq!(session.nav_mode(), NavMode::Stellar(None));
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn hyper_select_is_refused_during_a_jump_and_changes_nothing() {
        let mut session = jumping();
        let before = session.nav_mode();
        assert_eq!(
            session.apply(FlightCommand::SelectNextSystem),
            Answer::Refused
        );
        assert_eq!(session.nav_mode(), before);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn option_backquote_lets_go_of_the_target() {
        let mut session = quiet(&catalog());
        let id = ship(&mut session);
        session.chart.target = Some(id);
        assert_eq!(session.apply(FlightCommand::ClearTarget), Answer::Done);
        assert_eq!(session.chart.target, None);
        assert_eq!(session.take_sounds(), [SimSound::TargetCleared]);
    }

    #[test]
    fn option_backquote_with_no_target_is_refused_and_silent() {
        let mut session = quiet(&catalog());
        assert_eq!(session.apply(FlightCommand::ClearTarget), Answer::Refused);
        assert_eq!(session.chart.target, None);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn a_target_key_answers_the_target_it_picks() {
        let mut session = quiet(&catalog());
        let id = ship(&mut session);
        assert_eq!(
            session.apply(FlightCommand::SelectTarget(TargetPick::Next)),
            Answer::Target(Some(id))
        );
        assert_eq!(session.chart.target, Some(id));
        assert_eq!(session.take_sounds(), [SimSound::TargetSelected]);
    }

    #[test]
    fn a_target_key_is_refused_while_landed_and_changes_nothing() {
        let mut session = landed();
        assert_eq!(
            session.apply(FlightCommand::SelectTarget(TargetPick::Next)),
            Answer::Refused
        );
        assert_eq!(session.chart.target, None);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn the_nav_block_turns_the_nav_off_or_to_hyperspace() {
        let mut session = quiet(&three_stellars());
        session.select_stellar(StellarPick::Slot(0));
        session.take_sounds();
        assert_eq!(
            session.apply(FlightCommand::NavModeKeys {
                nav_off: true,
                h: false
            }),
            Answer::Done
        );
        assert_eq!(session.nav_mode(), NavMode::Off);
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        session.apply(FlightCommand::NavModeKeys {
            nav_off: false,
            h: true,
        });
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
    }

    #[test]
    fn the_nav_block_is_refused_during_a_jump_and_changes_nothing() {
        let mut session = jumping();
        let before = session.nav_mode();
        assert_eq!(
            session.apply(FlightCommand::NavModeKeys {
                nav_off: true,
                h: true
            }),
            Answer::Refused
        );
        assert_eq!(session.nav_mode(), before);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn a_stellar_key_answers_the_stellar_it_selects() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.take_sounds();
        assert_eq!(
            session.apply(FlightCommand::SelectStellar(StellarPick::Slot(0))),
            Answer::Stellar(StellarId(131))
        );
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(StellarId(131))));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
    }

    #[test]
    fn an_empty_slot_is_refused_and_changes_nothing() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.take_sounds();
        let before = session.nav_mode();
        assert_eq!(
            session.apply(FlightCommand::SelectStellar(StellarPick::Slot(3))),
            Answer::Refused
        );
        assert_eq!(session.nav_mode(), before);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn a_stellar_key_is_refused_during_a_jump_and_changes_nothing() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        session.take_sounds();
        let before = session.nav_mode();
        assert_eq!(
            session.apply(FlightCommand::SelectStellar(StellarPick::Slot(0))),
            Answer::Refused
        );
        assert_eq!(session.nav_mode(), before);
        assert_eq!(session.take_sounds(), []);
    }

    /// A plain click on the course map hitting `hits`.
    fn click(hits: &[i16]) -> FlightCommand {
        FlightCommand::MapClick(MapClick {
            hits: hits.iter().map(|&id| SystemId(id)).collect(),
            shift: false,
            hypergate: None,
            missions: Vec::new(),
            green: None,
        })
    }

    #[test]
    fn an_accepted_map_click_answers_what_it_did() {
        let mut session = quiet(&catalog());
        let answer = session.apply(click(&[131]));
        let Answer::Clicked(clicked) = answer else {
            panic!("accepted: {answer:?}");
        };
        assert_eq!(clicked.selected, Some(SystemId(131)));
        assert!(clicked.nav_redraw);
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(131))));
    }

    #[test]
    fn a_map_click_hitting_nothing_it_accepts_is_refused_and_changes_nothing() {
        let mut session = quiet(&catalog());
        let before = (session.nav_mode(), session.course().to_vec());
        for hits in [&[][..], &[133]] {
            assert_eq!(session.apply(click(hits)), Answer::Refused, "{hits:?}");
        }
        assert_eq!((session.nav_mode(), session.course().to_vec()), before);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn a_map_click_reads_the_press_it_carries() {
        let missions = vec![SystemId(133)];
        let click = MapClick {
            hits: vec![SystemId(133)],
            shift: true,
            hypergate: Some(vec![SystemId(131)]),
            missions: missions.clone(),
            green: Some(SystemId(132)),
        };
        assert_eq!(
            click.press(),
            MapPress {
                hits: &[SystemId(133)],
                shift: true,
                hypergate: Some(&[SystemId(131)]),
                missions: &missions,
                green: Some(SystemId(132)),
            }
        );
    }

    #[test]
    fn the_maps_cycle_selects_the_next_link_in_mode_3() {
        let mut session = quiet(&catalog());
        assert_eq!(
            session.apply(FlightCommand::CycleMapSelection),
            Answer::Done
        );
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(131))));
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn the_maps_cycle_is_refused_docked_and_changes_nothing() {
        let mut session = landed();
        let before = session.nav_mode();
        assert_eq!(
            session.apply(FlightCommand::CycleMapSelection),
            Answer::Refused
        );
        assert_eq!(session.nav_mode(), before);
        assert_eq!(session.take_sounds(), []);
    }

    /// Never refused: clearing a route with none still lets go of the
    /// selection.
    #[test]
    fn clear_route_clears_the_course_and_lets_go_of_the_selection() {
        let mut session = quiet(&catalog());
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.apply(FlightCommand::ClearRoute), Answer::Done);
        assert_eq!(session.course(), []);
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
    }

    #[test]
    fn a_floating_map_hit_selects_its_link_in_mode_3() {
        let mut session = quiet(&catalog());
        assert_eq!(
            session.apply(FlightCommand::SelectOnFloatingMap(Some(SystemId(131)))),
            Answer::Done
        );
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(131))));
        assert_eq!(session.take_sounds(), [SimSound::HyperSelected]);
    }

    #[test]
    fn a_floating_map_hit_is_refused_during_a_jump_and_changes_nothing() {
        let mut session = jumping();
        let before = session.nav_mode();
        assert_eq!(
            session.apply(FlightCommand::SelectOnFloatingMap(None)),
            Answer::Refused
        );
        assert_eq!(session.nav_mode(), before);
        assert_eq!(session.take_sounds(), []);
    }

    /// Never refused.
    #[test]
    fn marking_known_marks_the_systems() {
        let mut session = quiet(&catalog());
        assert!(!session.is_known(SystemId(133)));
        assert_eq!(
            session.apply(FlightCommand::MarkKnown(vec![SystemId(133)])),
            Answer::Done
        );
        assert!(session.is_known(SystemId(133)));
    }

    /// Never refused: with nothing to cue it changes nothing.
    #[test]
    fn the_cue_selects_the_courses_next_hop() {
        let mut session = quiet(&catalog());
        session.plot_course(SystemId(132)).expect("a route");
        session.apply(FlightCommand::NavModeKeys {
            nav_off: true,
            h: false,
        });
        assert_eq!(session.apply(FlightCommand::CueCourse), Answer::Done);
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(131))));
    }

    /// Never refused: with nothing saved the restore changes nothing.
    #[test]
    fn a_port_dialog_saves_the_nav_and_restores_it_once() {
        let mut session = landed();
        let saved = session.nav_mode();
        assert_eq!(session.apply(FlightCommand::SavePortNav), Answer::Done);
        session.chart.nav = NavMode::Off;
        assert_eq!(session.apply(FlightCommand::RestorePortNav), Answer::Done);
        assert_eq!(session.nav_mode(), saved);
        session.chart.nav = NavMode::Off;
        session.apply(FlightCommand::RestorePortNav);
        assert_eq!(session.nav_mode(), NavMode::Off, "restored once");
    }

    /// Never refused.
    #[test]
    fn the_exchanges_map_puts_back_the_port_nav_and_keeps_the_save() {
        let mut session = landed();
        let saved = session.nav_mode();
        session.apply(FlightCommand::SavePortNav);
        session.chart.nav = NavMode::Off;
        assert_eq!(session.apply(FlightCommand::PutBackPortNav), Answer::Done);
        assert_eq!(session.nav_mode(), saved);
        session.chart.nav = NavMode::Off;
        session.apply(FlightCommand::RestorePortNav);
        assert_eq!(session.nav_mode(), saved, "still saved");
    }

    /// Never refused.
    #[test]
    fn the_bars_map_puts_back_the_nav_the_bar_opened_with() {
        let mut session = landed();
        let saved = session.nav_mode();
        session.chart.bar_nav = Some(saved);
        session.chart.nav = NavMode::Off;
        assert_eq!(session.apply(FlightCommand::RestoreBarNav), Answer::Done);
        assert_eq!(session.nav_mode(), saved);
    }

    /// Never refused.
    #[test]
    fn a_dialogs_map_puts_back_the_nav_saved_before_it() {
        let mut session = quiet(&three_stellars());
        session.select_stellar(StellarPick::Slot(0));
        let saved = session.map_nav();
        session.chart.nav = NavMode::Off;
        assert_eq!(
            session.apply(FlightCommand::RestoreMapNav(saved)),
            Answer::Done
        );
        assert_eq!(session.map_nav(), saved);
    }

    /// Never refused.
    #[test]
    fn the_maps_close_in_port_puts_back_the_selection_keeping_the_mode() {
        let mut session = quiet(&catalog());
        session.apply(FlightCommand::SelectNextSystem);
        let saved = session.map_nav();
        session.clear_route();
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
        assert_eq!(
            session.apply(FlightCommand::RestoreMapSelection(saved)),
            Answer::Done
        );
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(131))));
    }

    /// The boxes of a click's ships: NPC `id` drawn in a 40-pixel square
    /// at (50, 0), and the player at the centre when `player` says.
    fn boxes(id: NpcId, player: bool) -> crate::flight_frame::DrawnBoxes {
        let square = |centre| crate::hover::DrawnBox {
            centre,
            width: 40,
            height: 40,
        };
        crate::flight_frame::DrawnBoxes {
            player: player.then(|| square(crate::geometry::Vec2::ZERO)),
            npcs: vec![(id, square(crate::geometry::Vec2::new(50.0, 0.0)))],
        }
    }

    #[test]
    fn a_click_in_space_on_a_ship_drawn_answers_the_ship_it_targets() {
        let mut session = quiet(&catalog());
        let id = ship(&mut session);
        let click = FlightCommand::ClickInSpace {
            at: crate::geometry::Vec2::new(55.0, 5.0),
            drawn: boxes(id, false),
        };
        assert_eq!(
            session.apply(click),
            Answer::InSpace(crate::hover::Hover::Ship(id))
        );
        assert_eq!(session.target().map(|npc| npc.id), Some(id));
        assert_eq!(session.take_sounds(), [SimSound::TargetSelected]);
    }

    #[test]
    fn a_click_in_space_on_the_players_ship_lets_go_of_the_target() {
        let mut session = quiet(&catalog());
        let id = ship(&mut session);
        session.select_target(TargetPick::Next);
        session.take_sounds();
        let click = FlightCommand::ClickInSpace {
            at: crate::geometry::Vec2::ZERO,
            drawn: boxes(id, true),
        };
        assert_eq!(
            session.apply(click),
            Answer::InSpace(crate::hover::Hover::Player)
        );
        assert_eq!(session.target(), None);
    }

    #[test]
    fn a_click_in_space_on_nothing_is_refused_and_changes_nothing() {
        let mut session = quiet(&catalog());
        let id = ship(&mut session);
        let before = session.clone();
        let click = FlightCommand::ClickInSpace {
            at: crate::geometry::Vec2::new(-3000.0, 3000.0),
            drawn: boxes(id, true),
        };
        assert_eq!(session.apply(click), Answer::Refused);
        assert_eq!(session, before);
    }

    #[test]
    fn a_click_in_space_is_refused_landed() {
        let mut session = landed();
        let before = session.clone();
        let click = FlightCommand::ClickInSpace {
            at: crate::geometry::Vec2::ZERO,
            drawn: boxes(NpcId(1), true),
        };
        assert_eq!(session.apply(click), Answer::Refused);
        assert_eq!(session, before);
    }
}
