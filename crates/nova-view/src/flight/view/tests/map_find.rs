use super::*;

use crate::galaxy::GalaxyMap;
use crate::galaxy::find::PROMPT;
use crate::text::fixture::MonoMetrics;
use nova_sim::MapClicked;
use nova_sim::map_panel::{Column, ColumnBody, MapPanel, PanelWords, Ports, Strip};
use nova_sim::map_search::SearchOutcome;

// Space, Find and Done on the in-flight maps (`_MapFilter`,
// `_DoSystemSearch`).

/// `catalog` with Sol (130), Alpha Centauri (131) and Barnard (132) named
/// as the session names them.
pub(super) fn named(mut catalog: FakeCatalog) -> FakeCatalog {
    for (id, name) in [(130, "Sol"), (131, "Alpha Centauri"), (132, "Barnard")] {
        catalog
            .place_names
            .systems
            .insert(SystemId(id), name.to_owned());
    }
    catalog
}

/// Where the map's Done or Find button is.
fn map_button(done: bool) -> Point {
    use crate::galaxy::buttons::{MapButton, MapButtons};
    let buttons = MapButtons::new(crate::galaxy::dialog::DIALOG_ORIGIN);
    let button = if done {
        MapButton::Done
    } else {
        MapButton::Find
    };
    buttons.rect(button).center()
}

/// A view over a scripted flight in `frame` over [`named`]'s
/// [`catalog`], measured, its flight drawing the map as [`sol_drawing`]
/// and preselecting the current system, with the course map open (opened
/// in port when the frame is docked).
fn faked_on_the_map(frame: FlightFrame) -> Faked {
    let (docked, current) = (frame.docked(), frame.system);
    let mut view = faked_in(named(catalog()), frame);
    view.map = view.map.clone().with_metrics(Rc::new(MonoMetrics));
    queue_map(&mut view, Some(current.0), sol_drawing());
    if docked {
        view.open_bar_map();
    } else {
        view.open_map();
    }
    view
}

/// A panel titled `title` with nothing else known.
pub(super) fn panel_titled(title: &str) -> MapPanel {
    MapPanel {
        strip: Strip::Info {
            ports: Ports::Unknown,
            hazards: String::new(),
            date: String::new(),
        },
        column: Column {
            title: title.to_owned(),
            body: ColumnBody::Unknown,
        },
        words: PanelWords::default(),
    }
}

/// The last panels `view`'s flight was asked for, as the panels' ask
/// ([`panels_asked`](super::maps::panels_asked)).
fn last_panel_asked(view: &Faked) -> Option<ScreenAsk> {
    super::maps::panels_asked(view).pop()
}

/// Types `text` into `view`'s open map as the platform sends it.
pub(super) fn faked_type_in(view: &mut Faked, text: &str) {
    for c in text.chars() {
        let k = if c == ' ' {
            Key::Space
        } else {
            Key::Char(c.to_ascii_lowercase())
        };
        faked_map_input(view, &key(k, true));
        faked_map_input(view, &Input::Text(c));
        faked_map_input(view, &key(k, false));
    }
}

/// F, `text` and Return on `view`'s open map.
pub(super) fn faked_find(view: &mut Faked, text: &str) {
    faked_type_in(view, "f");
    assert!(view.map.finding(), "Find opened");
    faked_type_in(view, text);
    faked_map_tap(view, Key::Enter);
    assert!(!view.map.finding(), "Find closed");
}

/// What the flight answers a plain click that selects `selected`.
fn selects(selected: i16) -> Answer {
    Answer::Clicked(MapClicked {
        selected: Some(SystemId(selected)),
        gate_pick: None,
        clear_route: None,
        course_changed: false,
        nav_redraw: true,
    })
}

#[test]
fn space_on_the_course_map_beeps_centres_on_the_current_system_and_selects_it() {
    let mut view = faked_on_the_map(in_sol());
    let alpha = faked_on_map(&mut view, 131);
    view.sim.queue_answer(selects(131));
    view.sim.queue_screen(redrawn(sol_drawing()));
    view.sim
        .queue_screen(ScreenAnswer::MapPanel(Box::new(blank_panel())));
    faked_map_click(&mut view, alpha);
    assert_eq!(view.map.selected(), Some(SystemId(131)));
    view.ui_sounds.clear();
    let sols = panel_titled("Current System:");
    view.sim
        .queue_screen(ScreenAnswer::MapPanel(Box::new(sols.clone())));
    assert!(!faked_map_tap(&mut view, Key::Space));
    assert_eq!(view.ui_sounds, [UiSound::MapCentred]);
    assert_eq!(view.map.selected(), Some(SystemId(130)));
    assert_eq!(view.map.place().expect("in flight").scroll(), (0, 0));
    assert_eq!(
        last_panel_asked(&view),
        Some(ScreenAsk::MapPanel {
            selected: Some(SystemId(130)),
            hypergate: None,
        })
    );
    assert_eq!(view.map.panel(), Some(&sols), "the panel follows");
}

#[test]
fn f_then_a_name_then_return_selects_what_the_flights_find_found_and_its_panel() {
    let mut view = faked_on_the_map(FlightFrame {
        system: SystemId(131),
        ..in_sol()
    });
    let sols = panel_titled("Selected System:");
    view.sim
        .queue_screen(ScreenAnswer::Found(SearchOutcome::Found(SystemId(130))));
    view.sim
        .queue_screen(ScreenAnswer::MapPanel(Box::new(sols.clone())));
    faked_find(&mut view, "sol");
    assert!(
        view.sim
            .screens_asked()
            .contains(&ScreenAsk::Find("sol".to_owned()))
    );
    assert_eq!(view.ui_sounds, [UiSound::MapSystemFound]);
    assert_eq!(view.map.selected(), Some(SystemId(130)));
    assert_eq!(view.map.place().expect("in flight").scroll(), (0, 0));
    assert_eq!(view.map.panel(), Some(&sols));
    assert_eq!(sent(&view), [], "no course nor nav mode");
}

#[test]
fn a_system_the_flights_find_doesnt_find_leaves_the_selection_and_panel() {
    // Sol hidden by its `Visibility` once bit 50 is set: a viewer, which
    // has no flight, finds it anyway.
    let catalog = FakeCatalog {
        visibility: vec![nova_sim::catalog::SystemVisibility {
            id: SystemId(130),
            visibility: nova_sim::Test::parse("!b50"),
        }],
        ..named(catalog())
    };
    let mut view = faked_in(
        catalog,
        FlightFrame {
            system: SystemId(131),
            ..in_sol()
        },
    );
    view.map = view.map.clone().with_metrics(Rc::new(MonoMetrics));
    view.open_map();
    let selected = view.map.selected();
    let panel = view.map.panel().cloned();
    for name in ["sol", "barnard", "zzz"] {
        view.sim
            .queue_screen(ScreenAnswer::Found(SearchOutcome::NotFound));
        faked_find(&mut view, name);
        assert_eq!(
            view.sim.screens_asked().last(),
            Some(&ScreenAsk::Find(name.to_owned())),
            "{name}"
        );
        assert_eq!(view.ui_sounds, [UiSound::MapSystemNotFound], "{name}");
        view.ui_sounds.clear();
        assert_eq!(view.map.selected(), selected, "{name}");
        assert_eq!(view.map.panel().cloned(), panel, "{name}");
    }
    // A viewer over the same galaxy finds both.
    let mut viewer = GalaxyMap::new(&view.catalog).with_metrics(Rc::new(MonoMetrics));
    for (name, id) in [("sol", 130), ("barnard", 132)] {
        viewer.input(&key(Key::Char('s'), true));
        viewer.input(&Input::Text('s'));
        for c in name.chars() {
            viewer.input(&Input::Text(c));
        }
        viewer.input(&key(Key::Enter, true));
        assert_eq!(viewer.selected(), Some(SystemId(id)), "{name}");
        assert_eq!(viewer.take_ui_sounds(), [UiSound::MapSystemFound]);
    }
}

#[test]
fn with_find_open_m_types_and_closes_nothing() {
    let mut view = faked_on_the_map(in_sol());
    faked_type_in(&mut view, "f");
    faked_type_in(&mut view, "m");
    faked_map_input(&mut view, &key(MAP, true));
    assert!(
        !faked_map_input(&mut view, &key(Key::Char('q'), true)),
        "a key with M held"
    );
    faked_map_input(&mut view, &Input::Text('q'));
    faked_map_input(&mut view, &key(Key::Char('q'), false));
    faked_map_input(&mut view, &key(MAP, false));
    let typed = view.map.find_dialog().expect("open").field().text();
    assert_eq!(typed, "mq", "M held types nothing more");
    assert!(
        !faked_map_tap(&mut view, Key::Enter),
        "Return confirms Find alone"
    );
    assert!(!view.map.finding());
    assert!(faked_map_tap(&mut view, Key::Enter), "then closes the map");
}

#[test]
fn a_click_on_done_closes_the_course_map() {
    let mut view = faked_on_the_map(in_sol());
    assert!(faked_map_click(&mut view, map_button(true)));
}

#[test]
fn a_click_on_find_opens_find_silently() {
    let mut view = faked_on_the_map(in_sol());
    assert!(!faked_map_click(&mut view, map_button(false)));
    assert!(view.map.finding());
    assert_eq!(view.ui_sounds, []);
    let mut list = DrawList::new();
    view.draw_open_map(&mut list);
    assert!(texts(&list).contains(&PROMPT.to_owned()));
}

#[test]
fn space_on_the_hypergate_map_keeps_the_destination_picked() {
    let mut view = faked_in(named(catalog()), in_sol());
    view.map = view.map.clone().with_metrics(Rc::new(MonoMetrics));
    let opening = map_opening(&view.frame, Some(130));
    view.sim.queue_screen(opening);
    view.map.offer_gates(vec![SystemId(131)]);
    view.open_map();
    let alpha = faked_on_map(&mut view, 131);
    view.sim.queue_answer(Answer::Clicked(MapClicked {
        selected: Some(SystemId(131)),
        gate_pick: Some(Some(SystemId(131))),
        clear_route: None,
        course_changed: false,
        nav_redraw: false,
    }));
    faked_map_click(&mut view, alpha);
    faked_map_tap(&mut view, Key::Space);
    assert_eq!(view.map.selected(), Some(SystemId(130)));
    assert_eq!(view.ui_sounds, [UiSound::MapCentred]);
    assert_eq!(
        view.map.take_gate_choice(),
        Some(SystemId(131)),
        "the pick to enter"
    );
}

#[test]
fn find_works_on_the_bars_map_where_m_types() {
    let mut view = faked_on_the_map(FlightFrame {
        landed: Some(StellarId(128)),
        ..in_sol()
    });
    let alpha = faked_on_map(&mut view, 131);
    view.sim.queue_answer(selects(131));
    faked_map_click(&mut view, alpha);
    assert_eq!(view.map.selected(), Some(SystemId(131)));
    view.sim
        .queue_screen(ScreenAnswer::Found(SearchOutcome::Found(SystemId(130))));
    faked_type_in(&mut view, "f");
    assert!(view.map.finding());
    faked_type_in(&mut view, "m");
    faked_map_input(&mut view, &key(Key::Backspace, true));
    faked_type_in(&mut view, "sol");
    faked_map_input(&mut view, &key(Key::Enter, true));
    assert!(
        view.sim
            .screens_asked()
            .contains(&ScreenAsk::Find("sol".to_owned()))
    );
    assert_eq!(view.ui_sounds, [UiSound::MapSystemFound]);
    assert_eq!(view.map.selected(), Some(SystemId(130)));
}

#[test]
fn with_find_open_on_the_bars_map_m_types_into_it() {
    let mut view = faked_on_the_map(FlightFrame {
        landed: Some(StellarId(128)),
        ..in_sol()
    });
    let mut port_type = |text: &str| {
        for c in text.chars() {
            let k = Key::Char(c);
            view.port_map_input(&key(k, true));
            view.port_map_input(&Input::Text(c));
            view.port_map_input(&key(k, false));
        }
    };
    port_type("f");
    port_type("m");
    assert!(view.map_open(), "M typed, not the close");
    assert!(view.map_finding());
    let typed = view
        .course_map()
        .find_dialog()
        .expect("open")
        .field()
        .text();
    assert_eq!(typed, "m");
}

#[test]
fn closing_the_map_with_find_open_drops_it() {
    let mut view = faked_on_the_map(in_sol());
    faked_type_in(&mut view, "f");
    assert!(view.map.finding(), "Find opened");
    view.close_map();
    view.open_map();
    assert!(view.map_open);
    assert!(!view.map.finding(), "dropped by the close");
}
