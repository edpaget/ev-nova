use super::*;
use crate::flight::sim_port::fixture::MapAsk;
use nova_sim::{MARKS_DESTINATION, MapMarks, SavedNav};

// A mission dialog's map (`_OfferOneMission`, `_DoMissionDialog`,
// `_DoMissionInfoDialog`, `_ShowTextDialog` for the accept's briefing).

/// Errand `id` to Proxima (140, in Alpha Centauri, 131), marking its
/// destination green (`Flags` 0x0100).
fn to_proxima(id: i16) -> MissionRecord {
    MissionRecord {
        travel_stel: 140,
        flags: MARKS_DESTINATION,
        ..errand(id)
    }
}

/// [`in_the_bar`] with mission 130 [`to_proxima`], offered at Earth's
/// mission computer.
fn offering() -> View {
    let mut catalog = port();
    catalog.missions[0] = to_proxima(130);
    in_the_bar_of(catalog, None)
}

/// [`flight`] holding mission 130 [`to_proxima`] in slot 0.
fn holding() -> View {
    let mut catalog = catalog();
    catalog.missions = vec![to_proxima(130)];
    let mut view = FlightView::new(catalog);
    run_set(&mut view, "S130");
    view.settle_script();
    assert_eq!(
        view.session().expect("flying").map_mission_systems(),
        [SystemId(131)],
        "Alpha Centauri arrowed red"
    );
    view
}

#[test]
fn the_flights_mission_systems_are_its_sessions() {
    use crate::flight::sim_port::MapSource as _;
    assert_eq!(holding().sim.map_mission_systems(), [SystemId(131)]);
}

/// Closes the map as M does on it.
fn press_map(view: &mut View) {
    view.port_map_input(&key(MAP, true));
    view.port_map_input(&key(MAP, false));
    assert!(!view.map_open());
}

/// A view over a scripted flight in `frame`, drawing the map as
/// [`sol_drawing`] and preselecting Sol, its dialogs' marks `marks` and
/// its nav as a map opens `nav`.
fn faked_dialogs(frame: FlightFrame, marks: MapMarks, nav: NavMode) -> Faked {
    let mut view = faked_sol(frame);
    let map = view.sim.map_mut();
    map.marks = Some(marks);
    map.map_nav = Some(SavedNav::of(nav));
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

/// The marks each map drawing was asked with, in order.
fn marks_drawn(view: &Faked) -> Vec<MapMarks> {
    view.sim
        .asked()
        .into_iter()
        .filter_map(|ask| match ask {
            MapAsk::Drawing { marks, .. } => Some(marks),
            _ => None,
        })
        .collect()
}

/// The marks questions `view`'s flight was asked, in order.
fn marks_asked(view: &Faked) -> Vec<MapAsk> {
    view.sim
        .asked()
        .into_iter()
        .filter(|ask| {
            matches!(
                ask,
                MapAsk::OfferMarks(_) | MapAsk::ComputerMarks(_) | MapAsk::MissionInfoMarks(_)
            )
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
    let mut view = faked_dialogs(in_sol(), marks, MOON);
    let mut drawing = sol_drawing();
    drawing.systems[1].green_arrow = true;
    view.sim.map_mut().drawing = Some(drawing);
    view.open_dialog_map(MapOpener::Offer(MissionId(130)));
    assert!(view.map_open);
    assert!(view.dialog_map_open());
    assert_eq!(view.map.mode(), MapMode::Course);
    assert_eq!(marks_asked(&view), [MapAsk::OfferMarks(MissionId(130))]);
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
    view.sim.map_mut().marks = Some(MapMarks::NONE);
    view.open_dialog_map(MapOpener::Computer(None));
    assert_eq!(
        marks_asked(&view),
        [
            MapAsk::ComputerMarks(Some(MissionId(130))),
            MapAsk::ComputerMarks(None)
        ]
    );
    assert_eq!(marks_drawn(&view), [marks, MapMarks::NONE]);
}

#[test]
fn a_briefings_map_draws_with_the_marks_its_text_carries() {
    let mut view = faked_in_flight();
    let barnard = green_on(132, true);
    view.open_dialog_map(MapOpener::Briefing(barnard));
    assert_eq!(marks_asked(&view), [], "the text carries them");
    assert_eq!(marks_drawn(&view), [barnard]);
}

#[test]
fn mission_infos_map_asks_its_slots_marks_and_a_briefing_draws_its_own() {
    let marks = green_on(131, false);
    let mut view = faked_dialogs(in_sol(), marks, MOON);
    view.open_dialog_map(MapOpener::MissionInfo(Some(0)));
    assert_eq!(marks_asked(&view), [MapAsk::MissionInfoMarks(Some(0))]);
    restore_after(&mut view);
    let kept = MapMarks {
        multi_arrows: true,
        ..marks
    };
    view.open_dialog_map(MapOpener::Briefing(kept));
    assert_eq!(marks_drawn(&view), [marks, kept]);
}

#[test]
fn a_dialogs_map_sends_its_green_system_with_a_click_and_flights_map_none() {
    let mut view = faked_in_flight();
    view.sim.map_mut().mission_systems = vec![SystemId(131)];
    view.open_map();
    let at = faked_on_map(&mut view, 132);
    faked_map_click(&mut view, at);
    restore_after(&mut view);
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
    let mut view = offering();
    view.open_dialog_map(MapOpener::Offer(MissionId(130)));
    view.take_sounds();
    press_map(&mut view);
    assert_eq!(map_sounds(&mut view), [], "closed docked");
    let mut view = holding();
    for opener in [
        MapOpener::MissionInfo(Some(0)),
        MapOpener::Offer(MissionId(130)),
        MapOpener::Computer(None),
        MapOpener::Briefing(MapMarks::NONE),
    ] {
        view.open_dialog_map(opener);
        view.take_sounds();
        press_map(&mut view);
        assert_eq!(view.take_sounds(), [], "{opener:?}: closed in flight");
    }
}

#[test]
fn in_flight_an_offers_map_puts_back_the_nav_it_opened_with_keeping_the_course() {
    for opener in [
        MapOpener::Offer(MissionId(130)),
        MapOpener::Computer(None),
        MapOpener::Briefing(MapMarks::NONE),
    ] {
        let mut view = flight();
        let before = NavMode::Stellar(Some(StellarId(129)));
        view.sim
            .session
            .as_mut()
            .expect("flying")
            .select_stellar(nova_sim::StellarPick::Slot(1));
        assert_eq!(nav_mode(&view), before, "{opener:?}");
        view.open_dialog_map(opener);
        port_click(&mut view, 131);
        assert_eq!(
            nav_mode(&view),
            NavMode::Hyperspace(Some(SystemId(131))),
            "{opener:?}: a plain click writes mode 3 in flight"
        );
        view.port_map_input(&key(Key::Shift, true));
        port_click(&mut view, 131);
        view.port_map_input(&key(Key::Shift, false));
        press_map(&mut view);
        assert_eq!(nav_mode(&view), before, "{opener:?}: put back");
        assert_eq!(
            view.session().expect("flying").course(),
            [SystemId(131)],
            "{opener:?}: the Shift-click's course kept"
        );
    }
}

#[test]
fn in_flight_mission_infos_map_keeps_the_clicks_nav_and_cues_the_course() {
    let mut view = holding();
    view.open_dialog_map(MapOpener::MissionInfo(Some(0)));
    view.port_map_input(&key(Key::Shift, true));
    port_click(&mut view, 131);
    view.port_map_input(&key(Key::Shift, false));
    assert_eq!(nav_mode(&view), NavMode::Off, "a Shift-click writes no nav");
    press_map(&mut view);
    assert_eq!(
        nav_mode(&view),
        NavMode::Hyperspace(Some(SystemId(131))),
        "the close's cue stands"
    );
    // A plain click's mode 3 stands too, with no course to cue.
    let mut view = holding();
    view.sim
        .session
        .as_mut()
        .expect("flying")
        .select_stellar(nova_sim::StellarPick::Slot(1));
    view.open_dialog_map(MapOpener::MissionInfo(Some(0)));
    port_click(&mut view, 131);
    press_map(&mut view);
    assert_eq!(
        nav_mode(&view),
        NavMode::Hyperspace(Some(SystemId(131))),
        "the click's nav stands"
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
        view.sim.map_mut().map_nav = Some(SavedNav::of(NavMode::Hyperspace(Some(SystemId(131)))));
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
    type Open = fn(&mut View);
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
        let mut view = offering();
        open(&mut view);
        view.port_map_input(&key(Key::Char('c'), true));
        assert_eq!(
            nav_mode(&view),
            NavMode::Stellar(None),
            "{what}: Clear Route lets go of it"
        );
        press_map(&mut view);
        assert_eq!(
            nav_mode(&view),
            NavMode::Stellar(Some(StellarId(128))),
            "{what}: put back"
        );
    }
}

#[test]
fn docked_the_computers_map_redraws_the_nav_area_after_its_restore_and_an_offers_not() {
    let mut view = offering();
    view.open_dialog_map(MapOpener::Offer(MissionId(130)));
    view.port_map_input(&key(Key::Char('c'), true));
    press_map(&mut view);
    assert_eq!(nav_mode(&view), NavMode::Stellar(Some(StellarId(128))));
    assert_ne!(
        port_nav(&view),
        naming("Earth"),
        "the close's redraw, before the put-back"
    );
    let mut view = offering();
    view.open_dialog_map(MapOpener::Computer(Some(MissionId(130))));
    view.port_map_input(&key(Key::Char('c'), true));
    press_map(&mut view);
    assert_eq!(port_nav(&view), naming("Earth"), "_DrawStatus after it");
}

#[test]
fn a_borders_change_on_a_dialogs_map_saves_in_flight_and_not_docked() {
    let mut view = holding();
    view.open_dialog_map(MapOpener::MissionInfo(Some(0)));
    view.port_map_input(&key(Key::Char('b'), true));
    press_map(&mut view);
    assert_eq!(view.take_map_borders_save(), Some(false), "in flight");
    let mut view = offering();
    view.open_dialog_map(MapOpener::Offer(MissionId(130)));
    view.port_map_input(&key(Key::Char('b'), true));
    press_map(&mut view);
    assert_eq!(view.take_map_borders_save(), None, "docked");
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
        view.open_map();
        assert!(!view.dialog_map_open(), "{opener:?}: flight's own");
        assert_eq!(
            marks_drawn(&view).last(),
            Some(&MapMarks::NONE),
            "{opener:?}"
        );
    }
}
