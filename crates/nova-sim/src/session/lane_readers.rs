//! Tests of every link reader over a galaxy of jump points alone
//! ([`laned`]): the map's links, its clicks and Shift-clicks, the planner,
//! the cue, Hyper Select, the map's Tab cycle, the floating map, the known
//! pass, a new pilot's neighbours and J.
//!
//! Over the original's readings none of them finds a link, as each reads
//! the systems' Con slots and a gate-only galaxy has none: `_DrawMap`
//! (@0xe7a8-0xed31), `_DoSystemMap`'s click and Shift-click
//! (@0x13739-0x137b3, @0x13401-0x13466), `_CueNextHyperRouteDest`
//! (@0xe2dc-0xe35f), Hyper Select (`_HandlePlayer` @0x69c16-0x69c56), the
//! map's Tab (`_MapFilter` @0x13f0f-0x13fac), the floating map
//! (@0x6a1e9-0x6a332), the known pass (@0x32b91-0x32c10), `_DoNewPilot`'s
//! neighbours (@0x18f97-0x19003) and J (@0x6b11e-0x6b131). A gate's exit
//! restarting the course is pinned in the `course` module
//! (`a_hypergate_or_wormhole_exit_restarts_the_course_at_the_system_entered`).
//!
//! By [`RuleKey::GateLanes`]' other reading each reads the gate lanes
//! instead (see the `story` module's `galaxy_links`), and J never jumps:
//! each test of that reading stands beside the engine's.

use super::Session;
use super::map_click::MapPress;
use crate::catalog::SystemId;
use crate::galaxy_map::MapMarks;
use crate::hyperspace::{JumpReadiness, JumpRefusal, RouteError};
use crate::navigation::NavMode;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::sound::SimSound;
use crate::testkit::{DRIFT, HEARTH, VERGE, laned};

/// A new pilot over [`laned`], in drift, by the original's readings.
fn started() -> Session {
    Session::start(&laned()).expect("starts")
}

/// A new pilot over [`laned`], in drift, by [`RuleKey::GateLanes`]' other
/// reading: the links are the gate lanes.
fn laned_session() -> Session {
    let rules = Rulebook::default().with_override(RuleKey::GateLanes, RuleSource::Bible);
    Session::start_ruled(&laned(), &rules).expect("starts")
}

/// A press on the course map hitting `hit`, with Shift held or not.
fn press(hit: &[SystemId], shift: bool) -> MapPress<'_> {
    MapPress {
        hits: hit,
        shift,
        hypergate: None,
        missions: &[],
        green: None,
    }
}

#[test]
fn the_map_draws_no_link() {
    let mut session = started();
    session.chart.set_nav(NavMode::Hyperspace(Some(VERGE)));
    let drawing = session.map_drawing(None, &[], MapMarks::NONE);
    assert_eq!(drawing.current_links, []);
    assert_eq!(drawing.links, []);
    assert_eq!(drawing.mode3_link, None);
}

#[test]
fn by_the_lanes_the_map_draws_the_current_systems_lanes() {
    let session = laned_session();
    let drawing = session.map_drawing(None, &[], MapMarks::NONE);
    assert_eq!(drawing.current_links, [(DRIFT, HEARTH), (DRIFT, VERGE)]);
    let links: Vec<_> = drawing
        .links
        .iter()
        .map(|link| (link.from, link.to))
        .collect();
    assert_eq!(links, [(DRIFT, HEARTH), (DRIFT, VERGE)]);
}

#[test]
fn the_planner_finds_no_route() {
    let mut session = started();
    assert_eq!(session.plot_course(VERGE), Err(RouteError::Unreachable));
    assert_eq!(session.plot_course(HEARTH), Err(RouteError::Unreachable));
}

#[test]
fn by_the_lanes_the_planner_routes_over_them() {
    let mut session = laned_session();
    assert_eq!(session.plot_course(VERGE), Ok(&[VERGE][..]));
    assert_eq!(session.plot_course(HEARTH), Ok(&[HEARTH][..]));
}

#[test]
fn a_plain_click_on_a_known_neighbour_lets_go_of_the_selection() {
    let mut session = started();
    session.mark_known([VERGE]);
    session.chart.set_nav(NavMode::Hyperspace(Some(HEARTH)));
    let clicked = session.map_click(&press(&[VERGE], false)).expect("known");
    assert_eq!(clicked.selected, Some(VERGE));
    assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
}

#[test]
fn by_the_lanes_a_plain_click_on_a_lane_neighbour_selects_it_in_mode_3() {
    let mut session = laned_session();
    let clicked = session.map_click(&press(&[VERGE], false)).expect("linked");
    assert_eq!(clicked.selected, Some(VERGE));
    assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(VERGE)));
}

#[test]
fn a_shift_click_never_extends_the_course() {
    let mut session = started();
    session.mark_known([HEARTH, VERGE]);
    let clicked = session.map_click(&press(&[HEARTH], true)).expect("known");
    assert_eq!(clicked.selected, None, "nothing appended");
    assert_eq!(session.course(), []);
}

#[test]
fn by_the_lanes_a_shift_click_extends_the_course_a_lane_at_a_time() {
    let mut session = laned_session();
    let clicked = session.map_click(&press(&[HEARTH], true)).expect("linked");
    assert_eq!(clicked.selected, Some(HEARTH));
    let clicked = session.map_click(&press(&[VERGE], true)).expect("linked");
    assert_eq!(clicked.selected, Some(VERGE), "hearth's lane to verge");
    assert_eq!(session.course(), [HEARTH, VERGE]);
    assert_eq!(session.pilot.course_start(), Some(DRIFT));
}

#[test]
fn the_cue_selects_nothing() {
    let mut session = started();
    session.pilot.course = vec![HEARTH, VERGE];
    session.pilot.course_start = Some(DRIFT);
    session.cue_course();
    assert_eq!(session.nav_mode(), NavMode::Off);
}

#[test]
fn by_the_lanes_the_cue_selects_the_first_hop() {
    let mut session = laned_session();
    session.pilot.course = vec![HEARTH, VERGE];
    session.pilot.course_start = Some(DRIFT);
    session.cue_course();
    assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(HEARTH)));
}

#[test]
fn hyper_select_and_the_tab_cycle_find_no_slot() {
    let mut session = started();
    assert_eq!(session.select_next_system(), None);
    assert!(session.cycle_map_selection());
    assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
}

#[test]
fn the_floating_map_has_no_link() {
    let session = started();
    assert_eq!(session.floating_map_links(), []);
    assert_eq!(session.floating_map_places().links, []);
}

#[test]
fn the_known_pass_marks_only_the_explored_systems() {
    let session = started();
    assert_eq!(session.known().iter().copied().collect::<Vec<_>>(), [DRIFT]);
}

#[test]
fn a_new_pilot_explores_no_neighbour() {
    let session = started();
    assert!(session.pilot().has_explored(DRIFT));
    assert!(!session.pilot().has_explored(HEARTH));
    assert!(!session.pilot().has_explored(VERGE));
}

#[test]
fn j_toward_a_neighbour_refuses_silently_with_no_pre_jump() {
    let mut session = started();
    session.chart.set_nav(NavMode::Hyperspace(Some(VERGE)));
    session.take_sounds();
    assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
    assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    assert!(
        !session.take_sounds().contains(&SimSound::Refused),
        "silent with a selection"
    );
    assert_eq!(session.mode.jump(), None, "no pre-jump");
}

#[test]
fn by_the_lanes_j_toward_a_lane_neighbour_refuses_silently_with_no_pre_jump() {
    let mut session = laned_session();
    session.chart.set_nav(NavMode::Hyperspace(Some(VERGE)));
    session.take_sounds();
    assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
    assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    assert!(
        !session.take_sounds().contains(&SimSound::Refused),
        "silent with a selection"
    );
    assert_eq!(session.mode.jump(), None, "no pre-jump");
    assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(VERGE)), "kept");
}
