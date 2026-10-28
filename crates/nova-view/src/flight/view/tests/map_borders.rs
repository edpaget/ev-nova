use super::*;

use crate::galaxy::GalaxyMap;
use crate::text::fixture::MonoMetrics;
use nova_sim::GovtId;
use nova_sim::map_borders::{BorderSources, Next};
use nova_sim::{RuleKey, RuleSource, Rulebook};

// The course map's borders in flight: the session's sources, the map's
// ticks, the rules, and the setting saved on an undocked close.

/// The sources a flight in Sol paints the borders from: Sol (130, at the
/// origin) the Federation's (128), and with `near`, Proxima Near (134,
/// at (40, 30)) government 129's.
pub(super) fn sources(near: bool) -> BorderSources {
    let mut systems = vec![(SystemId(130), (0, 0), Some(GovtId(128)))];
    if near {
        systems.push((SystemId(134), (40, 30), Some(GovtId(129))));
    }
    BorderSources::unfiltered(systems, nova_sim::govt::Governments::empty())
}

/// A view over a scripted flight in Sol over [`FakeCatalog::near`]'s
/// galaxy, drawing the map as [`sol_drawing`], painting the borders from
/// `sources`.
pub(super) fn faked_bordered(sources: BorderSources) -> Faked {
    let catalog = FakeCatalog {
        near: true,
        ..catalog()
    };
    let mut view = faked_in(catalog, in_sol());
    let map = view.sim.map_mut();
    map.drawing = Some(sol_drawing());
    map.border_sources = Some(sources);
    view
}

/// The governments of the cells `map` shows, once each.
pub(super) fn govts_on(map: &GalaxyMap) -> Vec<i16> {
    let mut govts: Vec<i16> = map.border_cells().iter().map(|cell| cell.govt.0).collect();
    govts.sort_unstable();
    govts.dedup();
    govts
}

#[test]
fn the_course_map_opens_on_the_flights_sources_with_the_coarse_pass() {
    let mut view = faked_bordered(sources(true));
    view.open_map();
    assert_eq!(govts_on(&view.map), [128, 129], "the coarse blocks");
    assert_eq!(view.map.border_passes().step(), 8);
    assert_eq!(view.map.border_passes().next(), Next::Unstarted);
}

#[test]
fn the_maps_next_drawing_takes_the_flights_sources_again() {
    // Proxima Near is unknown as the map opens; it becomes known while
    // the map is open, as the hypergate map's arrows mark its offers
    // known. The map's next drawing takes the sources again, so the
    // next pass adds it.
    let mut view = faked_bordered(sources(false));
    view.open_map();
    for _ in 0..5 {
        faked_map_tap(&mut view, Key::Char('-'));
    }
    assert!(!view.map.place().expect("open").can_zoom_out());
    assert_eq!(govts_on(&view.map), [128], "Proxima Near unknown");
    view.sim.map_mut().border_sources = Some(sources(true));
    view.sim.queue_answer(Answer::Done);
    faked_map_tap(&mut view, Key::Tab);
    faked_map_tap(&mut view, Key::Left);
    assert_eq!(govts_on(&view.map), [128, 129], "Proxima Near's blocks too");
}

/// [`faked_bordered`] by `rules`, measured, its course map open, its
/// flight's Find finding Sol.
fn faked_bordered_by(rules: &Rulebook) -> Faked {
    let catalog = FakeCatalog {
        near: true,
        ..catalog()
    };
    let mut fake = FakeFlight::new();
    fake.set_frame(in_sol());
    let map = fake.map_mut();
    map.drawing = Some(sol_drawing());
    map.border_sources = Some(sources(true));
    map.search = Some(nova_sim::map_search::SearchOutcome::Found(SystemId(130)));
    let mut view = scripted_over(catalog, fake, rules);
    view.map = view.map.clone().with_metrics(Rc::new(MonoMetrics));
    view.open_map();
    view
}

#[test]
fn by_default_the_rules_run_the_pass_in_a_tick_and_find_runs_none() {
    let mut view = faked_bordered_by(&Rulebook::default());
    view.map.tick(TICK);
    assert_eq!(view.map.border_passes().next(), Next::Idle);
    super::map_find::faked_find(&mut view, "Sol");
    assert_eq!(view.map.border_passes().next(), Next::Idle, "no pass");
}

#[test]
fn by_the_other_readings_the_pass_walks_a_tick_at_a_time_and_finds_centring_runs_the_coarse_one() {
    let rules = Rulebook::default()
        .with_override(RuleKey::MapBorderPace, RuleSource::Bible)
        .with_override(RuleKey::MapFindBorders, RuleSource::Bible);
    let mut view = faked_bordered_by(&rules);
    view.map.tick(TICK);
    assert_eq!(
        view.map.border_passes().next(),
        Next::Walking { after: None },
        "one null event a tick: the clear"
    );
    for _ in 0..3 {
        view.map.tick(TICK);
    }
    assert_eq!(view.map.border_passes().next(), Next::Idle);
    super::map_find::faked_find(&mut view, "Sol");
    assert_eq!(
        view.map.border_passes().step(),
        8,
        "Find's centring ran the coarse pass"
    );
    assert_eq!(view.map.border_passes().next(), Next::Unstarted);
}

// The setting and its save.
