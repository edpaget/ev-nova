use super::*;

use crate::galaxy::buttons::{HIDE_BORDERS_LABEL, SHOW_BORDERS_LABEL};
use crate::text::fixture::MonoMetrics;
use nova_sim::map_borders::Next;
use nova_sim::{RuleKey, RuleSource, Rulebook};

// The course map's borders in flight: the session's sources, the map's
// ticks, the rules, and the setting saved on an undocked close.

/// `catalog` with Sol (130) the Federation's (128) and lived in, and
/// Proxima Near (134, at (40, 30)) government 129's and lived in, known
/// from the start, as Sol lists it; [`known`] explores it too.
fn bordered() -> FakeCatalog {
    let mut catalog = FakeCatalog {
        sol_navs: vec![Some((StellarFlags::CAN_LAND, 0))],
        near: true,
        governed: vec![(130, 128), (134, 129)],
        lived: vec![134],
        ..catalog()
    };
    catalog
        .place_names
        .systems
        .insert(SystemId(130), "Sol".to_owned());
    catalog
}

/// `view` with Proxima Near explored and known.
fn known(mut view: View) -> View {
    run_set(&mut view, "X134");
    view.sim
        .session
        .as_mut()
        .expect("flying")
        .mark_known([SystemId(134)]);
    view
}

/// The governments of the cells the course map shows, once each.
fn govts_shown(view: &View) -> Vec<i16> {
    let mut govts: Vec<i16> = view
        .course_map()
        .border_cells()
        .iter()
        .map(|cell| cell.govt.0)
        .collect();
    govts.sort_unstable();
    govts.dedup();
    govts
}

fn passes(view: &View) -> &nova_sim::map_borders::BorderPasses {
    view.course_map().border_passes()
}

/// Whether the course map draws `label` on its Borders button.
fn labelled(view: &View, label: &str) -> bool {
    let mut list = DrawList::new();
    view.course_map().draw(&mut list);
    list.iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == label))
}

#[test]
fn the_course_map_opens_on_the_sessions_sources_and_runs_its_pass_on_the_ticks() {
    let mut view = known(FlightView::new(bordered()));
    tap(&mut view, MAP);
    assert_eq!(govts_shown(&view), [128, 129], "the coarse blocks");
    assert_eq!(passes(&view).step(), 8);
    assert_eq!(passes(&view).next(), Next::Unstarted);
    ticks(&mut view, 1);
    assert!(view.map_open());
    assert_eq!(passes(&view).next(), Next::Idle, "the map ticked");
    assert_eq!(passes(&view).step(), 1);
    assert_eq!(govts_shown(&view), [128, 129], "the fine cells");
}

#[test]
fn the_maps_next_drawing_takes_the_sources_again() {
    // Barnard (132, at (0, 600)), government 129's and lived in, is
    // explored but unknown as the map opens; it becomes known while the
    // map is open, as the hypergate map's arrows mark its offers known.
    // The map's next drawing takes the sources again, so the next pass
    // adds it.
    let mut view = FlightView::new(FakeCatalog {
        governed: vec![(130, 128), (132, 129)],
        lived: vec![132],
        sol_navs: vec![Some((StellarFlags::CAN_LAND, 0))],
        ..catalog()
    });
    run_set(&mut view, "X132");
    tap(&mut view, MAP);
    for _ in 0..5 {
        tap(&mut view, Key::Char('-'));
    }
    assert!(!view.course_map().place().expect("open").can_zoom_out());
    assert_eq!(govts_shown(&view), [128], "Barnard unknown");
    view.sim
        .session
        .as_mut()
        .expect("flying")
        .mark_known([SystemId(132)]);
    tap(&mut view, Key::Tab);
    tap(&mut view, Key::Left);
    assert_eq!(govts_shown(&view), [128, 129], "Barnard's blocks too");
}

#[test]
fn the_rules_reach_the_course_map() {
    let rules = Rulebook::default()
        .with_override(RuleKey::MapBorderPace, RuleSource::Bible)
        .with_override(RuleKey::MapFindBorders, RuleSource::Bible);
    let mut view = known(FlightView::new(bordered()))
        .with_rules(rules)
        .with_metrics(Rc::new(MonoMetrics));
    tap(&mut view, MAP);
    ticks(&mut view, 1);
    assert_eq!(
        passes(&view).next(),
        Next::Walking { after: None },
        "one null event a tick: the clear"
    );
    ticks(&mut view, 3);
    assert_eq!(passes(&view).next(), Next::Idle);
    view.input(&key(Key::Char('f'), true));
    view.input(&Input::Text('f'));
    view.input(&key(Key::Char('f'), false));
    for c in "Sol".chars() {
        view.input(&Input::Text(c));
    }
    tap(&mut view, Key::Enter);
    assert!(!view.course_map().finding());
    assert_eq!(
        passes(&view).step(),
        8,
        "Find's centring ran the coarse pass"
    );
    assert_eq!(passes(&view).next(), Next::Unstarted);
}

#[test]
fn by_default_the_rules_run_the_pass_in_a_tick_and_find_runs_none() {
    let mut view = known(FlightView::new(bordered())).with_metrics(Rc::new(MonoMetrics));
    tap(&mut view, MAP);
    ticks(&mut view, 1);
    assert_eq!(passes(&view).next(), Next::Idle);
    view.input(&key(Key::Char('f'), true));
    view.input(&Input::Text('f'));
    view.input(&key(Key::Char('f'), false));
    for c in "Sol".chars() {
        view.input(&Input::Text(c));
    }
    tap(&mut view, Key::Enter);
    assert_eq!(passes(&view).next(), Next::Idle, "no pass");
}

// The setting and its save.

#[test]
fn b_then_a_close_in_flight_saves_the_borders_off_once() {
    let mut view = flight();
    tap(&mut view, MAP);
    assert!(view.map_borders_shown());
    tap(&mut view, Key::Char('b'));
    assert!(!view.map_borders_shown());
    assert!(labelled(&view, SHOW_BORDERS_LABEL));
    assert_eq!(view.take_map_borders_save(), None, "not until the close");
    tap(&mut view, MAP);
    assert!(!view.map_open());
    assert_eq!(view.take_map_borders_save(), Some(false));
    assert_eq!(view.take_map_borders_save(), None, "taken");
    tap(&mut view, MAP);
    assert!(!view.course_map().borders_shown(), "the next open keeps it");
}

#[test]
fn b_twice_saves_nothing() {
    let mut view = flight();
    tap(&mut view, MAP);
    tap(&mut view, Key::Char('b'));
    tap(&mut view, Key::Char('b'));
    tap(&mut view, MAP);
    assert_eq!(view.take_map_borders_save(), None);
    assert!(view.map_borders_shown());
}

#[test]
fn a_change_on_the_bars_map_is_kept_but_not_saved() {
    let mut view = landed_view();
    view.open_bar_map();
    tap(&mut view, Key::Char('b'));
    view.close_map();
    assert_eq!(view.take_map_borders_save(), None, "docked");
    assert!(!view.map_borders_shown(), "kept for the run");
    view.open_bar_map();
    assert!(
        !view.course_map().borders_shown(),
        "the next open starts off"
    );
}

#[test]
fn the_map_opens_with_the_borders_setting_it_is_given() {
    let mut view = known(FlightView::new(bordered())).with_map_borders(false);
    assert!(!view.map_borders_shown());
    tap(&mut view, MAP);
    assert!(!view.course_map().borders_shown());
    assert!(labelled(&view, SHOW_BORDERS_LABEL));
    ticks(&mut view, 1);
    assert_eq!(view.course_map().border_cells(), [], "none drawn");
    tap(&mut view, Key::Char('b'));
    assert!(labelled(&view, HIDE_BORDERS_LABEL));
    assert_eq!(govts_shown(&view), [128, 129]);
    tap(&mut view, MAP);
    assert_eq!(view.take_map_borders_save(), Some(true));
}
