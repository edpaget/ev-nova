use super::*;

use crate::galaxy::GalaxyMap;
use crate::galaxy::find::{CANCEL_ITEM, PROMPT};
use crate::text::fixture::MonoMetrics;
use crate::ui::ButtonStyle;
use crate::ui::dialog::ItemSpec;

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

/// The panel the session gives the map for `selected`.
fn panel_for(view: &View, selected: i16) -> nova_sim::map_panel::MapPanel {
    view.session()
        .expect("flying")
        .map_panel(&view.catalog, Some(SystemId(selected)), None)
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

/// F, `text` and Return on the map open in flight.
fn find(view: &mut View, text: &str) {
    type_in(view, "f");
    assert!(view.map_finding(), "Find opened");
    type_in(view, text);
    tap(view, Key::Enter);
    assert!(!view.map_finding(), "Find closed");
}

/// Where the map's Done or Find button is.
fn map_button(done: bool) -> Point {
    let buttons = crate::galaxy::buttons::MapButtons::new();
    if done {
        buttons.done().rect.center()
    } else {
        buttons.find().rect.center()
    }
}

#[test]
fn space_on_the_course_map_beeps_centres_on_the_current_system_and_selects_it() {
    let mut view = on_the_map();
    let alpha = on_map(&view, 131);
    click(&mut view, alpha);
    view.take_sounds();
    tap(&mut view, Key::Space);
    assert!(view.map_open());
    assert_eq!(ui_sounds(&mut view), [UiSound::MapCentred]);
    let map = view.course_map();
    assert_eq!(map.selected(), Some(SystemId(130)));
    assert_eq!(map.view().center(), Point::new(0.0, 0.0));
    assert_eq!(
        map.panel(),
        Some(&panel_for(&view, 130)),
        "the panel follows"
    );
}

#[test]
fn f_then_a_name_then_return_selects_a_searchable_system_and_its_panel() {
    let mut view = measured(FlightView::new(named(catalog())));
    jump_to_alpha(&mut view);
    tap(&mut view, MAP);
    view.take_sounds();
    let course = view.session().expect("flying").course_shown();
    let nav = nav_mode(&view);
    find(&mut view, "sol");
    assert_eq!(ui_sounds(&mut view), [UiSound::MapSystemFound]);
    let map = view.course_map();
    assert_eq!(map.selected(), Some(SystemId(130)));
    assert_eq!(map.view().center(), Point::new(0.0, 0.0));
    assert_eq!(map.panel(), Some(&panel_for(&view, 130)));
    assert_eq!(
        view.session().expect("flying").course_shown(),
        course,
        "no course"
    );
    assert_eq!(nav_mode(&view), nav, "nor nav mode");
    assert!(view.map_open());
}

#[test]
fn an_unexplored_or_hidden_system_or_no_match_is_not_found_in_flight() {
    // Sol hidden by its `Visibility` once bit 50 is set.
    let catalog = FakeCatalog {
        visibility: vec![nova_sim::catalog::SystemVisibility {
            id: SystemId(130),
            visibility: nova_sim::Test::parse("!b50"),
        }],
        ..named(catalog())
    };
    let mut view = measured(FlightView::new(catalog));
    // Set at Sol, hiding it from the arrival's refresh on.
    run_set(&mut view, "b50");
    jump_to_alpha(&mut view);
    assert!(
        !view
            .session()
            .expect("flying")
            .galaxy()
            .visible(SystemId(130))
    );
    tap(&mut view, MAP);
    let selected = view.course_map().selected();
    let panel = view.course_map().panel().cloned();
    view.take_sounds();
    for name in ["sol", "barnard", "zzz"] {
        find(&mut view, name);
        assert_eq!(ui_sounds(&mut view), [UiSound::MapSystemNotFound], "{name}");
        assert_eq!(view.course_map().selected(), selected, "{name}");
        assert_eq!(view.course_map().panel().cloned(), panel, "{name}");
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
    let mut view = on_the_map();
    type_in(&mut view, "f");
    type_in(&mut view, "m");
    assert!(view.map_open(), "M typed");
    view.input(&key(MAP, true));
    type_in(&mut view, "q");
    view.input(&key(MAP, false));
    assert!(view.map_open(), "a key with M held");
    let typed = view
        .course_map()
        .find_dialog()
        .expect("open")
        .field()
        .text();
    assert_eq!(typed, "mq", "M held types nothing more");
    tap(&mut view, Key::Enter);
    assert!(view.map_open(), "Return confirms Find alone");
    assert!(!view.map_finding());
    tap(&mut view, Key::Enter);
    assert!(!view.map_open(), "then closes the map");
}

#[test]
fn a_click_on_done_closes_the_course_map_with_the_beep_and_the_cue() {
    let mut view = measured(FlightView::new(named(catalog())));
    plot(&mut view, 131);
    tap(&mut view, NAV_OFF_KEY);
    tap(&mut view, MAP);
    view.take_sounds();
    click(&mut view, map_button(true));
    assert!(!view.map_open());
    assert_eq!(map_sounds(&mut view), [UiSound::MapClosed]);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
}

#[test]
fn a_click_on_find_opens_find_silently() {
    let mut view = on_the_map();
    click(&mut view, map_button(false));
    assert!(view.map_finding());
    assert_eq!(view.take_sounds(), []);
    let mut list = DrawList::new();
    view.draw(&mut list);
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
    let alpha = on_map(&view, 131);
    click(&mut view, alpha);
    view.take_sounds();
    click(&mut view, map_button(true));
    assert_eq!(system_of(&view), SystemId(131));
    assert_eq!(map_sounds(&mut view), []);
}

#[test]
fn space_on_the_hypergate_map_keeps_the_destination_picked() {
    let mut view = measured(FlightView::new(gated()));
    land_now(&mut view);
    let alpha = on_map(&view, 131);
    click(&mut view, alpha);
    tap(&mut view, Key::Space);
    assert_eq!(view.course_map().selected(), Some(SystemId(130)));
    assert_eq!(ui_sounds(&mut view), [UiSound::MapCentred]);
    click(&mut view, map_button(true));
    assert_eq!(system_of(&view), SystemId(131), "the pick entered");
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

#[test]
fn find_works_on_the_bars_map_where_m_types() {
    let mut view = measured(in_the_bar_of(named(port()), None));
    view.open_bar_map();
    view.take_sounds();
    let port_type = |view: &mut View, text: &str| {
        for c in text.chars() {
            let k = Key::Char(c);
            view.port_map_input(&key(k, true));
            view.port_map_input(&Input::Text(c));
            view.port_map_input(&key(k, false));
        }
    };
    port_click(&mut view, 131);
    assert_eq!(view.course_map().selected(), Some(SystemId(131)));
    port_type(&mut view, "f");
    assert!(view.map_finding());
    port_type(&mut view, "m");
    assert!(view.map_open(), "M typed");
    view.port_map_input(&key(Key::Backspace, true));
    port_type(&mut view, "sol");
    view.port_map_input(&key(Key::Enter, true));
    assert_eq!(ui_sounds(&mut view), [UiSound::MapSystemFound]);
    assert_eq!(view.course_map().selected(), Some(SystemId(130)));
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
