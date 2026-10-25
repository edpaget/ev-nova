use super::*;

use crate::flight::sim_port::fixture::MapAsk;
use crate::galaxy::GalaxyMap;
use crate::galaxy::find::{CANCEL_ITEM, PROMPT};
use crate::text::fixture::MonoMetrics;
use crate::ui::ButtonStyle;
use crate::ui::dialog::ItemSpec;
use nova_sim::MapClicked;
use nova_sim::map_panel::{Column, ColumnBody, MapPanel, PanelWords, Ports, Strip};
use nova_sim::map_search::SearchOutcome;

// Space, Find and Done on the in-flight maps (`_MapFilter`,
// `_DoSystemSearch`).

/// `catalog` with Sol (130), Alpha Centauri (131) and Barnard (132) named
/// as the session names them.
fn named(mut catalog: FakeCatalog) -> FakeCatalog {
    for (id, name) in [(130, "Sol"), (131, "Alpha Centauri"), (132, "Barnard")] {
        catalog
            .place_names
            .systems
            .insert(SystemId(id), name.to_owned());
    }
    catalog
}

fn measured(view: View) -> View {
    view.with_metrics(Rc::new(MonoMetrics))
}

/// A measured flight from Sol over [`named`]'s [`catalog`], with the
/// course map open and its sounds taken.
fn on_the_map() -> View {
    let mut view = measured(FlightView::new(named(catalog())));
    tap(&mut view, MAP);
    view.take_sounds();
    view
}

/// Types `text` into the map as the platform sends it.
fn type_in(view: &mut View, text: &str) {
    for c in text.chars() {
        let k = if c == ' ' {
            Key::Space
        } else {
            Key::Char(c.to_ascii_lowercase())
        };
        view.input(&key(k, true));
        view.input(&Input::Text(c));
        view.input(&key(k, false));
    }
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
    let (docked, current) = (frame.docked, frame.system);
    let mut view = faked_in(named(catalog()), frame);
    view.map = view.map.clone().with_metrics(Rc::new(MonoMetrics));
    let map = view.sim.map_mut();
    map.drawing = Some(sol_drawing());
    map.map_preselection = Some(current);
    if docked {
        view.open_bar_map();
    } else {
        view.open_map();
    }
    view
}

/// A panel titled `title` with nothing else known.
fn panel_titled(title: &str) -> MapPanel {
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

/// The last panel `view`'s flight was asked for.
fn last_panel_asked(view: &Faked) -> Option<MapAsk> {
    view.sim
        .asked()
        .into_iter()
        .rfind(|ask| matches!(ask, MapAsk::Panel { .. }))
}

/// Types `text` into `view`'s open map as the platform sends it.
fn faked_type_in(view: &mut Faked, text: &str) {
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
fn faked_find(view: &mut Faked, text: &str) {
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
    faked_map_click(&mut view, alpha);
    assert_eq!(view.map.selected(), Some(SystemId(131)));
    view.ui_sounds.clear();
    let sols = panel_titled("Current System:");
    view.sim.map_mut().panel = Some(sols.clone());
    assert!(!faked_map_tap(&mut view, Key::Space));
    assert_eq!(view.ui_sounds, [UiSound::MapCentred]);
    assert_eq!(view.map.selected(), Some(SystemId(130)));
    assert_eq!(view.map.place().expect("in flight").scroll(), (0, 0));
    assert_eq!(
        last_panel_asked(&view),
        Some(MapAsk::Panel {
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
    let map = view.sim.map_mut();
    map.search = Some(SearchOutcome::Found(SystemId(130)));
    map.panel = Some(sols.clone());
    faked_find(&mut view, "sol");
    assert!(view.sim.asked().contains(&MapAsk::Find("sol".to_owned())));
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
    view.sim.map_mut().search = Some(SearchOutcome::NotFound);
    view.open_map();
    let selected = view.map.selected();
    let panel = view.map.panel().cloned();
    for name in ["sol", "barnard", "zzz"] {
        faked_find(&mut view, name);
        assert_eq!(
            view.sim.asked().last(),
            Some(&MapAsk::Find(name.to_owned())),
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
fn with_find_open_m_types_into_it_and_leaves_the_map_open() {
    let mut view = on_the_map();
    type_in(&mut view, "f");
    type_in(&mut view, "m");
    assert!(view.map_open(), "M typed");
    let typed = view
        .course_map()
        .find_dialog()
        .expect("open")
        .field()
        .text();
    assert_eq!(typed, "m");
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
fn done_on_the_hypergate_map_enters_the_pick_or_cancels_in_silence() {
    let mut view = measured(FlightView::new(gated()));
    land_now(&mut view);
    view.take_sounds();
    click(&mut view, map_button(true));
    assert!(!view.map_open());
    assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
    assert_eq!(map_sounds(&mut view), []);
    let mut view = measured(FlightView::new(gated()));
    land_now(&mut view);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    view.take_sounds();
    click(&mut view, map_button(true));
    assert_eq!(system_of(&view), SystemId(131));
    assert_eq!(map_sounds(&mut view), []);
}

#[test]
fn space_on_the_hypergate_map_keeps_the_destination_picked() {
    let mut view = faked_in(named(catalog()), in_sol());
    view.map = view.map.clone().with_metrics(Rc::new(MonoMetrics));
    view.sim.map_mut().map_preselection = Some(SystemId(130));
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
fn done_closes_the_bars_map_silently_and_puts_back_the_bars_nav() {
    let mut view = measured(in_the_bar(None));
    view.open_bar_map();
    port_click(&mut view, 131);
    view.take_sounds();
    for pressed in [true, false] {
        view.port_map_input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: map_button(true),
        });
    }
    assert!(!view.map_open());
    assert_eq!(map_sounds(&mut view), []);
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(128)))),
        "the bar's nav put back"
    );
}

/// Types `text` into the map open in port, a key and its character at
/// a time.
fn port_type(view: &mut View, text: &str) {
    for c in text.chars() {
        let k = Key::Char(c);
        view.port_map_input(&key(k, true));
        view.port_map_input(&Input::Text(c));
        view.port_map_input(&key(k, false));
    }
}

#[test]
fn find_works_on_the_bars_map_where_m_types() {
    let mut view = faked_on_the_map(FlightFrame {
        docked: true,
        ..in_sol()
    });
    let alpha = faked_on_map(&mut view, 131);
    view.sim.queue_answer(selects(131));
    faked_map_click(&mut view, alpha);
    assert_eq!(view.map.selected(), Some(SystemId(131)));
    view.sim.map_mut().search = Some(SearchOutcome::Found(SystemId(130)));
    faked_type_in(&mut view, "f");
    assert!(view.map.finding());
    faked_type_in(&mut view, "m");
    faked_map_input(&mut view, &key(Key::Backspace, true));
    faked_type_in(&mut view, "sol");
    faked_map_input(&mut view, &key(Key::Enter, true));
    assert!(view.sim.asked().contains(&MapAsk::Find("sol".to_owned())));
    assert_eq!(view.ui_sounds, [UiSound::MapSystemFound]);
    assert_eq!(view.map.selected(), Some(SystemId(130)));
}

#[test]
fn with_find_open_on_the_bars_map_m_types_into_it() {
    let mut view = measured(in_the_bar_of(named(port()), None));
    view.open_bar_map();
    port_type(&mut view, "f");
    assert!(view.map_finding());
    port_type(&mut view, "m");
    assert!(view.map_open(), "M typed");
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
    let mut view = on_the_map();
    type_in(&mut view, "f");
    view.close_map();
    tap(&mut view, MAP);
    assert!(view.map_open());
    assert!(!view.map_finding());
}

#[test]
fn find_is_laid_out_from_the_template_and_style_the_flight_gives() {
    let mut template = crate::galaxy::find::fallback();
    template.items[CANCEL_ITEM - 1].kind = ItemSpec::Button("Never mind".to_owned());
    let view = measured(FlightView::new(named(catalog())));
    let mut view = view.with_find(Some(template), ButtonStyle::STOCK);
    tap(&mut view, MAP);
    type_in(&mut view, "f");
    let mut list = DrawList::new();
    view.draw(&mut list);
    assert!(texts(&list).contains(&"Never mind".to_owned()));
}
