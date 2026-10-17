use super::*;
use nova_sim::{MARKS_DESTINATION, MapMarks};

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

/// The arrows drawn on the open map at system `id`: red, then green.
fn arrows(view: &View, id: i16) -> Option<(bool, bool)> {
    view.course_map()
        .picture()
        .expect("drawn")
        .systems
        .iter()
        .find(|system| system.id == SystemId(id))
        .map(|system| (system.red_arrow, system.green_arrow))
}

/// Whether any system on the open map has the green arrow.
fn any_green(view: &View) -> bool {
    view.course_map()
        .picture()
        .expect("drawn")
        .systems
        .iter()
        .any(|system| system.green_arrow)
}

/// Closes the map as M does on it.
fn press_map(view: &mut View) {
    view.port_map_input(&key(MAP, true));
    view.port_map_input(&key(MAP, false));
    assert!(!view.map_open());
}

#[test]
fn an_offers_map_shows_the_green_arrow_at_its_destination() {
    let mut view = offering();
    view.open_dialog_map(MapOpener::Offer(MissionId(130)));
    assert!(view.map_open());
    assert!(view.dialog_map_open());
    assert_eq!(view.course_map().mode(), MapMode::Course);
    assert_eq!(arrows(&view, 131), Some((false, true)));
}

#[test]
fn the_computers_map_marks_its_selected_row_and_with_none_marks_nothing() {
    let mut view = offering();
    view.open_dialog_map(MapOpener::Computer(Some(MissionId(130))));
    assert_eq!(arrows(&view, 131), Some((false, true)));
    press_map(&mut view);
    view.open_dialog_map(MapOpener::Computer(None));
    assert!(!any_green(&view));
}

#[test]
fn a_briefings_map_draws_its_green_system_though_unknown() {
    let mut view = flight();
    let barnard = MapMarks {
        green: Some(SystemId(132)),
        multi_arrows: true,
    };
    view.open_dialog_map(MapOpener::Briefing(barnard));
    assert!(
        !view.session().expect("flying").is_known(SystemId(132)),
        "Barnard unknown and unexplored"
    );
    assert_eq!(arrows(&view, 132), Some((false, true)));
}

#[test]
fn mission_infos_map_clears_the_green_systems_red_arrow_where_a_briefings_keeps_it() {
    let mut view = holding();
    view.open_dialog_map(MapOpener::MissionInfo(Some(0)));
    assert_eq!(arrows(&view, 131), Some((false, true)), "mission info");
    press_map(&mut view);
    let marks = view
        .session()
        .expect("flying")
        .mission_info_map_marks(Some(0));
    view.open_dialog_map(MapOpener::Briefing(MapMarks {
        multi_arrows: true,
        ..marks
    }));
    assert_eq!(arrows(&view, 131), Some((true, true)), "both kept");
}

#[test]
fn a_dialogs_map_clicks_its_green_system_as_a_mission_system() {
    // Barnard, unknown and two jumps off, is refused on flight's map and
    // taken as the briefing's green system.
    let mut view = flight();
    tap(&mut view, MAP);
    let at = on_map(&mut view, 132);
    click(&mut view, at);
    assert_ne!(view.course_map().selected(), Some(SystemId(132)));
    tap(&mut view, MAP);
    view.open_dialog_map(MapOpener::Briefing(MapMarks {
        green: Some(SystemId(132)),
        multi_arrows: true,
    }));
    port_click(&mut view, 132);
    assert_eq!(view.course_map().selected(), Some(SystemId(132)));
}

#[test]
fn a_dialogs_map_opens_and_closes_in_silence_docked_and_in_flight() {
    let mut view = offering();
    view.take_sounds();
    view.open_dialog_map(MapOpener::Offer(MissionId(130)));
    assert_eq!(map_sounds(&mut view), [], "opened docked");
    press_map(&mut view);
    assert_eq!(map_sounds(&mut view), [], "closed docked");
    let mut view = holding();
    view.take_sounds();
    for opener in [
        MapOpener::MissionInfo(Some(0)),
        MapOpener::Offer(MissionId(130)),
        MapOpener::Computer(None),
        MapOpener::Briefing(MapMarks::NONE),
    ] {
        view.open_dialog_map(opener);
        assert_eq!(view.take_sounds(), [], "{opener:?}: opened in flight");
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
        view.session
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
fn after_a_dialogs_map_flights_map_marks_no_green_system_and_keeps_red_arrows() {
    for opener in [
        MapOpener::MissionInfo(Some(0)),
        MapOpener::Briefing(MapMarks {
            green: Some(SystemId(131)),
            multi_arrows: false,
        }),
    ] {
        let mut view = holding();
        view.open_dialog_map(opener);
        assert_eq!(arrows(&view, 131), Some((false, true)), "{opener:?}");
        press_map(&mut view);
        assert!(!view.dialog_map_open());
        tap(&mut view, MAP);
        assert!(view.map_open());
        assert!(!view.dialog_map_open(), "{opener:?}: flight's own");
        assert_eq!(arrows(&view, 131), Some((true, false)), "{opener:?}");
    }
}
