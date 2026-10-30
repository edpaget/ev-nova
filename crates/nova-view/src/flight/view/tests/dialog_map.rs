use super::*;
use nova_sim::{MapMarks, SavedNav};

// A mission dialog's map (`_OfferOneMission`, `_DoMissionDialog`,
// `_DoMissionInfoDialog`, `_ShowTextDialog` for the accept's briefing).

/// Closes `view`'s map as M does on it, over the port.
fn faked_press_map(view: &mut Faked) {
    view.port_map_input(&key(MAP, true));
    view.port_map_input(&key(MAP, false));
    assert!(!view.map_open, "closed");
}

/// Queues on `view`'s flight the answers to the next map's opening, its
/// nav as it opens `nav`, marked `marks` (a dialog's, or a briefing's
/// own), preselecting Sol, with the mission systems `missions`, and to
/// its first redraw, [`sol_drawing`].
fn queue_opening(view: &mut Faked, marks: MapMarks, nav: NavMode, missions: &[i16]) {
    view.sim.queue_screen(ScreenAnswer::MapOpening(MapOpen {
        nav: SavedNav::of(nav),
        marks,
        borders: nova_sim::map_borders::BorderSources::default(),
        preselection: Some(SystemId(130)),
        missions: missions.iter().copied().map(SystemId).collect(),
    }));
    view.sim.queue_screen(redrawn(sol_drawing()));
}

/// A view over a scripted flight in `frame`, its next map's opening
/// answered as [`queue_opening`] says, marked `marks`, its nav as it
/// opens `nav`.
fn faked_dialogs(frame: FlightFrame, marks: MapMarks, nav: NavMode) -> Faked {
    let mut view = faked(frame);
    queue_opening(&mut view, marks, nav, &[]);
    view
}

/// [`faked_dialogs`] in flight in Sol, with no marks and Earth's
/// neighbour, the Moon (129), selected.
fn faked_in_flight() -> Faked {
    faked_dialogs(in_sol(), MapMarks::NONE, MOON)
}

/// The nav the scripted flights fly with as a dialog's map opens.
const MOON: NavMode = NavMode::Stellar(Some(StellarId(129)));

/// Green marks on system `id`, with multiple arrows `multi_arrows`.
fn green_on(id: i16, multi_arrows: bool) -> MapMarks {
    MapMarks {
        green: Some(SystemId(id)),
        multi_arrows,
    }
}

/// The marks each map redraw was asked with, in order.
fn marks_drawn(view: &Faked) -> Vec<MapMarks> {
    view.sim
        .screens_asked()
        .iter()
        .filter_map(|ask| match ask {
            ScreenAsk::Map { marks, .. } => Some(*marks),
            _ => None,
        })
        .collect()
}

/// Where each map's opening asked its marks from, in order.
fn marks_asked(view: &Faked) -> Vec<MarksFrom> {
    view.sim
        .screens_asked()
        .iter()
        .filter_map(|ask| match ask {
            ScreenAsk::MapOpening { marks } => Some(*marks),
            _ => None,
        })
        .collect()
}

/// Ends the map as [`FlightView::close_map`] ends it: its opener's
/// restore of what it opened with.
fn restore_after(view: &mut Faked) {
    let opened = view.map_opened.take();
    view.restore_opener_nav(opened);
}

#[test]
fn an_offers_map_asks_the_offers_marks_and_draws_with_them() {
    let marks = green_on(131, false);
    let mut view = faked(in_sol());
    let mut drawing = sol_drawing();
    drawing.systems[1].green_arrow = true;
    view.sim.queue_screen(opening_with(MOON, Some(130), marks));
    view.sim.queue_screen(redrawn(drawing));
    view.open_dialog_map(MapOpener::Offer(MissionId(130)));
    assert!(view.map_open);
    assert!(view.dialog_map_open());
    assert_eq!(view.map.mode(), MapMode::Course);
    assert_eq!(marks_asked(&view), [MarksFrom::Offer(MissionId(130))]);
    assert_eq!(marks_drawn(&view), [marks]);
    let alpha = view.map.picture().expect("drawn").systems[1];
    assert_eq!(
        (alpha.id, alpha.red_arrow, alpha.green_arrow),
        (SystemId(131), false, true)
    );
}

#[test]
fn the_computers_map_asks_the_marks_of_its_selected_row_or_none() {
    let marks = green_on(131, false);
    let mut view = faked_dialogs(in_sol(), marks, MOON);
    view.open_dialog_map(MapOpener::Computer(Some(MissionId(130))));
    restore_after(&mut view);
    queue_opening(&mut view, MapMarks::NONE, MOON, &[]);
    view.open_dialog_map(MapOpener::Computer(None));
    assert_eq!(
        marks_asked(&view),
        [
            MarksFrom::Computer(Some(MissionId(130))),
            MarksFrom::Computer(None)
        ]
    );
    assert_eq!(marks_drawn(&view), [marks, MapMarks::NONE]);
}

#[test]
fn a_briefings_map_draws_with_the_marks_its_text_carries() {
    let barnard = green_on(132, true);
    let mut view = faked_dialogs(in_sol(), barnard, MOON);
    view.open_dialog_map(MapOpener::Briefing(barnard));
    assert_eq!(
        marks_asked(&view),
        [MarksFrom::Given(barnard)],
        "the text carries them"
    );
    assert_eq!(marks_drawn(&view), [barnard]);
}

#[test]
fn mission_infos_map_asks_its_slots_marks_and_a_briefing_draws_its_own() {
    let marks = green_on(131, false);
    let mut view = faked_dialogs(in_sol(), marks, MOON);
    view.open_dialog_map(MapOpener::MissionInfo(Some(0)));
    assert_eq!(marks_asked(&view), [MarksFrom::MissionInfo(Some(0))]);
    restore_after(&mut view);
    let kept = MapMarks {
        multi_arrows: true,
        ..marks
    };
    queue_opening(&mut view, kept, MOON, &[]);
    view.open_dialog_map(MapOpener::Briefing(kept));
    assert_eq!(marks_drawn(&view), [marks, kept]);
}

#[test]
fn a_dialogs_map_a_flight_answers_nothing_marks_only_a_briefings_own() {
    // A flight that answers the opening nothing, as one that failed to
    // start does: the briefing's marks are its text's, the others none,
    // as such a flight's marks were.
    let barnard = green_on(132, true);
    for (opener, marks) in [
        (MapOpener::Offer(MissionId(130)), MapMarks::NONE),
        (MapOpener::Computer(Some(MissionId(130))), MapMarks::NONE),
        (MapOpener::MissionInfo(Some(0)), MapMarks::NONE),
        (MapOpener::Briefing(barnard), barnard),
    ] {
        let mut view = faked(in_sol());
        view.sim.fail("no chär");
        view.open_dialog_map(opener);
        assert!(view.dialog_map_open(), "{opener:?}");
        assert_eq!(marks_drawn(&view), [marks], "{opener:?}");
        assert_eq!(
            view.map_opened.map(|opened| (opened.nav, opened.marks)),
            Some((None, marks)),
            "{opener:?}: no nav noted"
        );
    }
}

#[test]
fn a_dialogs_map_sends_its_green_system_with_a_click_and_flights_map_none() {
    let mut view = faked(in_sol());
    queue_opening(&mut view, MapMarks::NONE, MOON, &[131]);
    view.open_map();
    let at = faked_on_map(&mut view, 132);
    faked_map_click(&mut view, at);
    restore_after(&mut view);
    queue_opening(&mut view, green_on(132, true), MOON, &[131]);
    view.open_dialog_map(MapOpener::Briefing(green_on(132, true)));
    let at = faked_on_map(&mut view, 132);
    faked_map_click(&mut view, at);
    let greens: Vec<Option<SystemId>> = sent(&view)
        .iter()
        .filter_map(|command| match command {
            FlightCommand::MapClick(click) => {
                assert_eq!(click.hits, [SystemId(132)]);
                assert_eq!(click.missions, [SystemId(131)]);
                Some(click.green)
            }
            _ => None,
        })
        .collect();
    assert_eq!(greens, [None, Some(SystemId(132))]);
}

#[test]
fn a_dialogs_map_opens_in_silence_docked_and_in_flight() {
    for docked in [true, false] {
        for opener in [
            MapOpener::MissionInfo(Some(0)),
            MapOpener::Offer(MissionId(130)),
            MapOpener::Computer(None),
            MapOpener::Briefing(MapMarks::NONE),
        ] {
            let mut view = faked_dialogs(
                FlightFrame {
                    landed: docked.then_some(StellarId(128)),
                    ..in_sol()
                },
                MapMarks::NONE,
                MOON,
            );
            view.open_dialog_map(opener);
            assert!(view.map_open, "{opener:?}");
            assert_eq!(view.ui_sounds, [], "{opener:?}, docked {docked}");
            assert_eq!(sent(&view), [], "{opener:?}, docked {docked}");
        }
    }
}

#[test]
fn a_dialogs_map_closes_in_silence_docked_and_in_flight() {
    for landed in [Some(StellarId(128)), None] {
        for opener in [
            MapOpener::MissionInfo(Some(0)),
            MapOpener::Offer(MissionId(130)),
            MapOpener::Computer(None),
            MapOpener::Briefing(MapMarks::NONE),
        ] {
            let mut view = faked_dialogs(FlightFrame { landed, ..in_sol() }, MapMarks::NONE, MOON);
            view.open_dialog_map(opener);
            faked_press_map(&mut view);
            assert_eq!(view.ui_sounds, [], "{opener:?}, docked at {landed:?}");
        }
    }
}
#[test]
fn in_flight_an_offers_map_puts_back_the_nav_it_opened_with_keeping_the_course() {
    // The clicks' nav and course are the flight's
    // (`nova_sim::session::map_click::tests`).
    for opener in [
        MapOpener::Offer(MissionId(130)),
        MapOpener::Computer(None),
        MapOpener::Briefing(MapMarks::NONE),
    ] {
        let mut view = faked_in_flight();
        view.open_dialog_map(opener);
        faked_press_map(&mut view);
        assert_eq!(
            sent(&view),
            [
                FlightCommand::CueCourse,
                FlightCommand::RestoreMapNav(SavedNav::of(MOON))
            ],
            "{opener:?}: the close's cue, then the nav as the map opened"
        );
    }
}
#[test]
fn in_flight_mission_infos_map_keeps_the_clicks_nav_and_cues_the_course() {
    let mut view = faked_in_flight();
    view.open_dialog_map(MapOpener::MissionInfo(Some(0)));
    faked_press_map(&mut view);
    assert_eq!(
        sent(&view),
        [FlightCommand::CueCourse],
        "the close's cue stands, and nothing is put back"
    );
}
#[test]
fn a_dialogs_map_close_sends_back_the_nav_it_opened_with() {
    for opener in [
        MapOpener::Offer(MissionId(130)),
        MapOpener::Computer(None),
        MapOpener::Briefing(MapMarks::NONE),
    ] {
        let mut view = faked_in_flight();
        view.open_dialog_map(opener);
        let saved = SavedNav::of(MOON);
        assert_eq!(
            view.map_opened.and_then(|opened| opened.nav),
            Some(saved),
            "{opener:?}: noted as it opened"
        );
        view.sim.set_frame(FlightFrame {
            nav_mode: NavMode::Hyperspace(Some(SystemId(131))),
            ..in_sol()
        });
        restore_after(&mut view);
        assert_eq!(
            sent(&view),
            [FlightCommand::RestoreMapNav(saved)],
            "{opener:?}"
        );
    }
}

#[test]
fn a_computers_map_close_retakes_the_nav_area_after_its_restore_and_an_offers_not() {
    let stale = (MOON, NavDisplay::Stellar(Some("Stale".to_owned())));
    for (opener, retaken) in [
        (MapOpener::Computer(Some(MissionId(130))), true),
        (MapOpener::Offer(MissionId(130)), false),
    ] {
        let mut view = faked_dialogs(
            FlightFrame {
                landed: Some(StellarId(128)),
                nav_mode: MOON,
                ..in_sol()
            },
            MapMarks::NONE,
            MOON,
        );
        view.open_dialog_map(opener);
        view.nav_scene = Some(stale.clone());
        restore_after(&mut view);
        let shown = if retaken {
            Some((MOON, view.nav_now()))
        } else {
            Some(stale.clone())
        };
        assert_eq!(view.nav_scene, shown, "{opener:?}");
    }
}

#[test]
fn mission_infos_map_close_restores_nothing() {
    let mut view = faked_in_flight();
    view.open_dialog_map(MapOpener::MissionInfo(Some(0)));
    let stale = Some((MOON, NavDisplay::Stellar(Some("Stale".to_owned()))));
    view.nav_scene = stale.clone();
    restore_after(&mut view);
    assert_eq!(sent(&view), [], "its restore is dead (@0x9e45f-0x9e476)");
    assert_eq!(view.nav_scene, stale);
}

#[test]
fn docked_the_close_puts_back_the_selection_the_map_opened_with_on_every_map() {
    type Open = fn(&mut Faked);
    let earth = NavMode::Stellar(Some(StellarId(128)));
    let openers: [(&str, Open); 4] = [
        ("the bar's", |view| view.open_bar_map()),
        ("an offer's", |view| {
            view.open_dialog_map(MapOpener::Offer(MissionId(130)));
        }),
        ("mission info's", |view| {
            view.open_dialog_map(MapOpener::MissionInfo(None));
        }),
        ("another", |view| view.open_map()),
    ];
    for (what, open) in openers {
        let mut view = faked_dialogs(
            FlightFrame {
                landed: Some(StellarId(128)),
                nav_mode: earth,
                ..in_sol()
            },
            MapMarks::NONE,
            earth,
        );
        open(&mut view);
        view.port_map_input(&key(Key::Char('c'), true));
        assert_eq!(sent(&view), [FlightCommand::ClearRoute], "{what}");
        faked_press_map(&mut view);
        assert_eq!(
            sent(&view).get(1),
            Some(&FlightCommand::RestoreMapSelection(SavedNav::of(earth))),
            "{what}: the selection put back, undoing the Clear Route"
        );
        assert!(!sent(&view).contains(&FlightCommand::CueCourse), "{what}");
    }
}
#[test]
fn docked_the_computers_map_redraws_the_nav_area_after_its_restore_and_an_offers_not() {
    let earth = NavMode::Stellar(Some(StellarId(128)));
    for (opener, after) in [
        (MapOpener::Offer(MissionId(130)), MOON),
        (MapOpener::Computer(Some(MissionId(130))), earth),
    ] {
        let docked = FlightFrame {
            landed: Some(StellarId(128)),
            nav_mode: MOON,
            ..in_sol()
        };
        let mut view = faked_dialogs(docked.clone(), MapMarks::NONE, earth);
        view.open_dialog_map(opener);
        // The selection's put-back changes nothing; the dialog's restore
        // puts Earth back.
        view.sim.queue_answer(Answer::Done);
        view.sim.queue_answer_showing(
            Answer::Done,
            FlightFrame {
                nav_mode: earth,
                ..docked
            },
        );
        faked_press_map(&mut view);
        assert_eq!(
            view.nav_scene.as_ref().map(|(nav, _)| *nav),
            Some(after),
            "{opener:?}: the close's redraw, then the computer's after the restore"
        );
    }
}
#[test]
fn a_borders_change_on_a_dialogs_map_saves_in_flight_and_not_docked() {
    for (landed, saved) in [(None, Some(false)), (Some(StellarId(128)), None)] {
        let mut view = faked_dialogs(FlightFrame { landed, ..in_sol() }, MapMarks::NONE, MOON);
        view.open_dialog_map(MapOpener::MissionInfo(Some(0)));
        view.port_map_input(&key(Key::Char('b'), true));
        faked_press_map(&mut view);
        assert_eq!(view.take_map_borders_save(), saved, "docked at {landed:?}");
    }
}
#[test]
fn after_a_dialogs_map_flights_map_draws_with_no_marks() {
    for opener in [
        MapOpener::MissionInfo(Some(0)),
        MapOpener::Briefing(green_on(131, false)),
    ] {
        let mut view = faked_dialogs(in_sol(), green_on(131, false), MOON);
        view.open_dialog_map(opener);
        assert_eq!(marks_drawn(&view), [green_on(131, false)], "{opener:?}");
        restore_after(&mut view);
        queue_opening(&mut view, MapMarks::NONE, MOON, &[]);
        view.open_map();
        assert!(!view.dialog_map_open(), "{opener:?}: flight's own");
        assert_eq!(
            marks_drawn(&view).last(),
            Some(&MapMarks::NONE),
            "{opener:?}"
        );
    }
}

#[test]
fn a_dialogs_map_closed_before_m_is_let_go_waits_for_its_release() {
    // `_DoSystemMap`'s wait (@0x13a2e-0x13a46): each dialog opens it on
    // M, so a close while M is still down waits, its restore with it.
    let closes: [fn(&mut Faked); 2] = [
        |view| view.port_map_input(&key(Key::Enter, true)),
        Faked::close_map,
    ];
    for landed in [Some(StellarId(128)), None] {
        for (n, close) in closes.into_iter().enumerate() {
            let mut view = faked_dialogs(FlightFrame { landed, ..in_sol() }, MapMarks::NONE, MOON);
            view.open_dialog_map(MapOpener::Offer(MissionId(130)));
            close(&mut view);
            assert!(view.map_open, "close {n} waits, docked at {landed:?}");
            assert_eq!(sent(&view), [], "close {n}: no restore yet");
            view.port_map_input(&key(MAP, false));
            assert!(!view.map_open, "close {n} on the release");
            assert!(
                sent(&view).contains(&FlightCommand::RestoreMapNav(SavedNav::of(MOON))),
                "close {n}: restored on the release"
            );
        }
    }
}

#[test]
fn a_dialogs_map_closed_by_m_pressed_again_waits_for_its_release() {
    // M's own close reads M held (`_MapFilter` @0x144db-0x14587), so a
    // second press after the first was let go waits as any close does.
    for landed in [Some(StellarId(128)), None] {
        let mut view = faked_dialogs(FlightFrame { landed, ..in_sol() }, MapMarks::NONE, MOON);
        view.open_dialog_map(MapOpener::Offer(MissionId(130)));
        view.port_map_input(&key(MAP, false));
        view.port_map_input(&key(MAP, true));
        assert!(view.map_open, "waits for M's release, docked at {landed:?}");
        assert_eq!(sent(&view), [], "no restore yet, docked at {landed:?}");
        assert_eq!(view.ui_sounds, [], "no beep yet, docked at {landed:?}");
        view.port_map_input(&key(MAP, false));
        assert!(
            !view.map_open,
            "closed on the release, docked at {landed:?}"
        );
        assert!(
            sent(&view).contains(&FlightCommand::RestoreMapNav(SavedNav::of(MOON))),
            "restored on the release, docked at {landed:?}"
        );
        assert_eq!(view.ui_sounds, [], "a dialog's map closes in silence");
    }
}

#[test]
fn a_dialogs_map_closes_at_once_by_return_once_m_is_let_go() {
    let mut view = faked_in_flight();
    view.open_dialog_map(MapOpener::Offer(MissionId(130)));
    view.port_map_input(&key(MAP, false));
    view.port_map_input(&key(Key::Enter, true));
    assert!(!view.map_open);
}
