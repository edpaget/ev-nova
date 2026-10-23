//! A click on the in-flight course and hypergate maps, and Clear Route,
//! as `_DoSystemMap` takes item 3 (@0x12dd4-0x137e8) and item 8
//! (@0x12c69-0x12dbf): [`Session::map_click`] and [`Session::clear_route`].
//! All addresses are in `EV Nova.app`'s `EV Nova` (i386).
//!
//! **The hit test** is the view's: the visible systems whose 20 × 20 box
//! (the point inset by −10, @0x12f3e, `PtInRect` @0x12f53) holds the
//! press, by ascending ID ([`MapPress::hits`]). The first of them the
//! acceptance passes decides the click; one it refuses is passed over
//! (@0x1311e-0x13133). With none accepted the press drags the map
//! (@0x137e9).
//!
//! **The acceptance** (@0x12e84-0x13133), for a hit `h`:
//!
//! - its gate flag: on the hypergate map, the gate offers `h`
//!   (@0x12f8f-0x13017);
//! - on the hypergate map a hit neither flagged nor the current system is
//!   passed over (@0x13023-0x13040);
//! - it is accepted when it is known (+0x1ec, @0x13051), a mission system
//!   (`_missionSyst`, @0x12f60-0x12f7c), the green mission system its
//!   opener marked (`_greenMissionSyst`, raw, @0x12f82-0x12f8a, taken
//!   as a mission system is @0x13061-0x1306b; [`MapPress::green`]) or
//!   flagged;
//! - otherwise the source is, with Shift (`_KeyCheck(0x38)` @0x13071) and
//!   a course, its last hop, raw, and a hit at that hop's position is
//!   accepted (@0x1308b-0x130af); without, the current system;
//! - then each of the source's first 16 Con slots that is set and has a
//!   visible system at its place (`_FindActiveCoLocatedSystem`): a hit
//!   that system is, or stands at the position of, is accepted as that
//!   system (@0x130c0-0x13125). By [`HyperlinkRule::BothWays`], a hit
//!   whose stand-in links to the source is accepted too, as its jumps
//!   read the link both ways.
//!
//! The pick is the accepted hit's stand-in (@0x1314b); with none the
//! press drags (@0x13154), which no visible hit gives.
//!
//! **What the click does**, by [`RuleKey::MapPlotting`]'s engine reading:
//!
//! - On the hypergate map, under both readings, and Shift never read
//!   (@0x13272): `_selSystem` is the pick, and the gate's pick the pick
//!   when the gate offers the hit, none otherwise (@0x136fc-0x13734).
//!   Ours writes no nav: it has no mode 4.
//! - A Shift-click (@0x1325e-0x136d2): the course as
//!   [`map_route::shift_click`] writes it, the link test being the
//!   source's explored level above unexplored (+0x90 > 0) or the pick
//!   known (@0x1340f-0x1342c), then a Con slot of the source whose system
//!   stands for the pick (@0x1343f-0x13462). `_selSystem` is the pick only
//!   when it is written (@0x136c6). No nav is written, in flight and
//!   docked alike, as the Shift-click never tests `_onPlanet`.
//! - A plain click (@0x136d7-0x137c7): `_selSystem` is the pick, docked
//!   too (@0x136e9). Docked, nothing more (@0x136ec). In flight, mode 3
//!   with the pick when the ship can jump there, the slot noted the first
//!   at its place (@0x13739-0x137bb); otherwise the selection let go with
//!   the mode kept (@0x13788-0x13790). The nav area is redrawn at once
//!   (`_DrawStatus`). The course is never touched.
//!
//! By the other reading, a click off the hypergate map plots the shortest
//! course to the pick ([`Session::plot_course`]), Shift or not.
//!
//! **Clear Route** (@0x12c69-0x12dbf), from its button or C, under both
//! readings: the course cleared, `showClearRouteButton` cleared and the
//! selection let go with the mode kept (+0x6c = −1), then the nav area
//! redrawn (`_DrawStatus` @0x12dba).
//!
//! Nothing here plays a sound or makes a random draw.
//!
//! **Left out** (rdm:task/do-system-map-left-out-branches): the length
//! the acceptance's Shift branch reads is a stale local of `_DoSystemMap`
//! (-0x9c(%ebp), @0x13081-0x13090), where ours reads the course's real
//! last hop; the hypergate map's mode 4 writes (@0x13702-0x13734); and
//! the position compare of a link that resolves to −1, which reads system
//! −1 out of bounds (@0x130f0-0x13113): ours skips the link.

use super::nav::CON_SLOTS;
use super::{Chart, Session, Setup};
use crate::catalog::{StellarId, SystemId};
use crate::hyperspace::HyperlinkRule;
use crate::map_route;
use crate::pilot::{ExploredLevel, Pilot};
use crate::rulebook::{RuleKey, RuleSource};

/// A left press in an in-flight map's item 3, as `_DoSystemMap` reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapPress<'a> {
    /// The visible systems whose hit box holds the press, by ascending ID.
    pub hits: &'a [SystemId],
    /// Whether Shift was held at the press (`_KeyCheck(0x38)`, @0x13071,
    /// @0x1325e).
    pub shift: bool,
    /// On the hypergate map, the systems the gate offers; `None` on the
    /// course map.
    pub hypergate: Option<&'a [SystemId]>,
    /// The mission systems taken as the map opened (`_missionSyst`,
    /// [`Session::map_mission_systems`]).
    pub missions: &'a [SystemId],
    /// The green mission system the map's opener marked
    /// (`_greenMissionSyst`, [`MapMarks::green`](crate::MapMarks::green)),
    /// accepted raw as a mission system is (@0x12f82-0x12f8a); `None` on
    /// every map but a mission dialog's.
    pub green: Option<SystemId>,
}

/// What an accepted click did ([`Session::map_click`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapClicked {
    /// The map's new selection (`_selSystem`); `None` leaves it as it is.
    pub selected: Option<SystemId>,
    /// On the hypergate map, its new pick, `Some(None)` for none; `None`
    /// off it.
    pub gate_pick: Option<Option<SystemId>>,
    /// What the click writes to `showClearRouteButton`; `None` leaves it.
    /// By the engine's reading only a hop written sets it, and no click
    /// clears it; by the other, the click writes it outright, on while
    /// the plotted course has a hop.
    pub clear_route: Option<bool>,
    /// Whether the click touched the course, which the map shows again.
    pub course_changed: bool,
    /// Whether the nav area redraws at once (`_DrawStatus`).
    pub nav_redraw: bool,
}

impl Session {
    /// A press on an in-flight map (see the module docs): `None` when no
    /// hit is accepted, and the map drags; otherwise what the first
    /// accepted hit did.
    pub fn map_click(&mut self, press: &MapPress<'_>) -> Option<MapClicked> {
        self.chart
            .map_click(&mut self.pilot, &self.setup, self.mode.landed(), press)
    }

    /// Clear Route, from its button or C (@0x12c69-0x12dbf): the course
    /// cleared and started at the current system (@0x12c74-0x12c98), and
    /// the selection let go with the mode kept, in flight, in
    /// port and on the hypergate map alike. The caller turns
    /// `showClearRouteButton` off and redraws the nav area at once
    /// (`_DrawStatus` @0x12dba).
    pub fn clear_route(&mut self) {
        self.chart.clear_route(&mut self.pilot);
    }
}

impl Chart {
    /// A press on an in-flight map (see the module docs): `None` when no
    /// hit is accepted, and the map drags; otherwise what the first
    /// accepted hit did.
    fn map_click(
        &mut self,
        pilot: &mut Pilot,
        setup: &Setup,
        landed: Option<StellarId>,
        press: &MapPress<'_>,
    ) -> Option<MapClicked> {
        let (hit, flagged) = press
            .hits
            .iter()
            .find_map(|&hit| self.accepted(pilot, setup, press, hit))?;
        let pick = self.galaxy.same_place(hit)?;
        if press.hypergate.is_some() {
            return Some(MapClicked {
                selected: Some(pick),
                gate_pick: Some(flagged.then_some(pick)),
                clear_route: None,
                course_changed: false,
                nav_redraw: false,
            });
        }
        let docked = landed.is_some();
        if setup.rules.source_for(RuleKey::MapPlotting) == RuleSource::Bible {
            let _ = self.plot_course(pilot, setup, pick);
            return Some(MapClicked {
                selected: Some(pick),
                gate_pick: None,
                clear_route: Some(!pilot.course.is_empty()),
                course_changed: true,
                nav_redraw: !docked,
            });
        }
        if press.shift {
            return Some(self.shift_click(pilot, setup, pick));
        }
        if !docked {
            if self
                .galaxy
                .map()
                .jumps(pilot.system, setup.hyperlinks)
                .contains(&pick)
            {
                self.select_at_place(pilot, pick);
            } else {
                self.deselect();
            }
        }
        Some(MapClicked {
            selected: Some(pick),
            gate_pick: None,
            clear_route: None,
            course_changed: false,
            nav_redraw: !docked,
        })
    }

    /// Clear Route, from its button or C (@0x12c69-0x12dbf): the course
    /// cleared and the selection let go with the mode kept, in flight, in
    /// port and on the hypergate map alike. The caller turns
    /// `showClearRouteButton` off and redraws the nav area at once
    /// (`_DrawStatus` @0x12dba).
    fn clear_route(&mut self, pilot: &mut Pilot) {
        pilot.restart_course(pilot.system);
        self.deselect();
    }

    /// The system `hit` is accepted as, and its gate flag, or `None` when
    /// it is passed over (see the module docs' acceptance).
    fn accepted(
        &self,
        pilot: &Pilot,
        setup: &Setup,
        press: &MapPress<'_>,
        hit: SystemId,
    ) -> Option<(SystemId, bool)> {
        let flagged = press
            .hypergate
            .is_some_and(|offered| offered.contains(&hit));
        if press.hypergate.is_some() && !flagged && hit != pilot.system {
            return None;
        }
        if flagged
            || self.is_known(hit)
            || press.missions.contains(&hit)
            || press.green == Some(hit)
        {
            return Some((hit, flagged));
        }
        let places = self.galaxy.map().places();
        let source = match pilot.course.last() {
            Some(&last) if press.shift => {
                if places.together(last, hit) {
                    return Some((hit, false));
                }
                last
            }
            _ => pilot.system,
        };
        let linked = self
            .links(source)
            .find(|&to| to == hit || places.together(to, hit));
        let back = || {
            let stood = self.galaxy.same_place(hit)?;
            self.links_back(setup, source, stood).then_some(stood)
        };
        linked.or_else(back).map(|to| (to, false))
    }

    /// The systems standing for `source`'s first [`CON_SLOTS`] Con slots,
    /// in order: each set, in the catalog and with a visible system at its
    /// place (`_FindActiveCoLocatedSystem`).
    fn links(&self, source: SystemId) -> impl Iterator<Item = SystemId> + '_ {
        self.galaxy
            .con_slots(source)
            .iter()
            .take(CON_SLOTS)
            .filter(|&&slot| self.galaxy.exists(slot))
            .filter_map(|&slot| self.galaxy.same_place(slot))
    }

    /// Whether, by [`HyperlinkRule::BothWays`], `to` links to `source`, so
    /// its jumps reach `to` from `source`.
    fn links_back(&self, setup: &Setup, source: SystemId, to: SystemId) -> bool {
        setup.hyperlinks == HyperlinkRule::BothWays
            && self
                .galaxy
                .map()
                .jumps(source, HyperlinkRule::BothWays)
                .contains(&to)
    }

    /// A Shift-click on the course map picking `pick`, by the engine's
    /// reading (see the module docs).
    fn shift_click(&mut self, pilot: &mut Pilot, setup: &Setup, pick: SystemId) -> MapClicked {
        let edit = map_route::shift_click(
            pilot.course_start,
            &pilot.course,
            pilot.system,
            pick,
            self.galaxy.map().places(),
            |source| {
                let gate = pilot.explored_level(source) != ExploredLevel::Unexplored
                    || self.is_known(pick);
                gate && (self.links(source).any(|to| to == pick)
                    || self.links_back(setup, source, pick))
            },
        );
        pilot.course = edit.course;
        pilot.course_start = edit.start;
        MapClicked {
            selected: edit.appended.then_some(pick),
            gate_pick: None,
            clear_route: edit.appended.then_some(true),
            course_changed: true,
            nav_redraw: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_route::ROUTE_HOPS;
    use crate::navigation::NavMode;
    use crate::session::tests::{alternates, hide, hub, with_alternate};
    use crate::testkit::{FakePilotCatalog, catalog, land_now, star};
    use crate::{Rulebook, StellarPick};

    fn ids(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    fn hyperspace(system: i16) -> NavMode {
        NavMode::Hyperspace(Some(SystemId(system)))
    }

    /// A press on the course map hitting `hits`.
    fn press(hits: &[SystemId], shift: bool) -> MapPress<'_> {
        MapPress {
            hits,
            shift,
            hypergate: None,
            missions: &[],
            green: None,
        }
    }

    /// A plain click on the course map hitting `hits`.
    fn click(session: &mut Session, hits: &[i16]) -> Option<MapClicked> {
        session.map_click(&press(&ids(hits), false))
    }

    /// A Shift-click on the course map hitting `hits`.
    fn shift_click(session: &mut Session, hits: &[i16]) -> Option<MapClicked> {
        session.map_click(&press(&ids(hits), true))
    }

    /// What a plain click selecting `selected` in flight gives.
    #[allow(clippy::unnecessary_wraps)] // Compared with `map_click`'s answer.
    fn selects(selected: i16) -> Option<MapClicked> {
        Some(MapClicked {
            selected: Some(SystemId(selected)),
            gate_pick: None,
            clear_route: None,
            course_changed: false,
            nav_redraw: true,
        })
    }

    /// What a Shift-click gives: `selected` when it appended.
    #[allow(clippy::unnecessary_wraps)] // Compared with `map_click`'s answer.
    fn shifted(selected: Option<i16>) -> Option<MapClicked> {
        Some(MapClicked {
            selected: selected.map(SystemId),
            gate_pick: None,
            clear_route: selected.is_some().then_some(true),
            course_changed: true,
            nav_redraw: false,
        })
    }

    /// [`hub`]'s session: 130 lists 134, 131 (twice), itself, a missing
    /// 999 and 135, and knows them; 132, 133 and 136 are unknown.
    fn hub_session() -> Session {
        Session::start(&hub()).expect("starts")
    }

    // The acceptance.

    #[test]
    fn the_lowest_accepted_hit_wins() {
        let mut session = hub_session();
        assert_eq!(click(&mut session, &[131, 134]), selects(131));
        assert_eq!(
            click(&mut session, &[132, 134]),
            selects(134),
            "132, unknown and unlinked, is passed over"
        );
    }

    #[test]
    fn no_hit_accepted_gives_none_and_changes_nothing() {
        let mut session = hub_session();
        session.pilot.course = ids(&[131]);
        assert_eq!(click(&mut session, &[]), None);
        assert_eq!(click(&mut session, &[132, 133, 136]), None);
        assert_eq!(session.nav_mode(), NavMode::Off);
        assert_eq!(session.course(), ids(&[131]));
    }

    #[test]
    fn a_known_system_is_accepted_linked_or_not() {
        let mut session = hub_session();
        session.mark_known([SystemId(133)]);
        assert_eq!(click(&mut session, &[133]), selects(133));
    }

    #[test]
    fn a_mission_system_is_accepted() {
        let mut session = hub_session();
        let missions = ids(&[133]);
        let hits = ids(&[133]);
        let press = MapPress {
            missions: &missions,
            ..press(&hits, false)
        };
        assert_eq!(session.map_click(&press), selects(133));
    }

    #[test]
    fn the_green_system_is_accepted_raw_known_or_not_linked_or_not() {
        // 133 is unknown and unlinked: refused with no green system, or
        // another one, and accepted as the green one.
        let mut session = hub_session();
        assert!(!session.is_known(SystemId(133)));
        let hits = ids(&[133]);
        for green in [None, Some(SystemId(132))] {
            let press = MapPress {
                green,
                ..press(&hits, false)
            };
            assert_eq!(session.map_click(&press), None, "{green:?}");
        }
        session.chart.nav = hyperspace(134);
        let press = MapPress {
            green: Some(SystemId(133)),
            ..press(&hits, false)
        };
        assert_eq!(session.map_click(&press), selects(133));
        assert_eq!(
            session.nav_mode(),
            NavMode::Hyperspace(None),
            "no jump there: the selection let go in mode 3"
        );
    }

    #[test]
    fn the_green_system_selects_in_mode_3_in_flight_when_the_ship_can_jump_there() {
        let mut session = hub_session();
        session.chart.known.clear();
        session.pilot.course = ids(&[131]);
        let hits = ids(&[134]);
        let press = MapPress {
            green: Some(SystemId(134)),
            ..press(&hits, false)
        };
        assert_eq!(session.map_click(&press), selects(134));
        assert_eq!(session.nav_mode(), hyperspace(134));
        assert_eq!(session.course(), ids(&[131]), "the course untouched");
    }

    #[test]
    fn an_unknown_system_one_link_from_the_current_one_is_accepted() {
        let mut session = hub_session();
        session.chart.known.clear();
        assert_eq!(click(&mut session, &[131]), selects(131));
    }

    #[test]
    fn an_unknown_system_two_jumps_away_is_refused() {
        let mut session = hub_session();
        assert!(!session.is_known(SystemId(132)));
        assert_eq!(click(&mut session, &[132]), None);
        assert_eq!(shift_click(&mut session, &[132]), None, "no course");
    }

    #[test]
    fn with_shift_and_a_course_an_unknown_system_the_last_hop_links_to_is_accepted() {
        let mut session = hub_session();
        session.pilot.course = ids(&[131]);
        assert_eq!(click(&mut session, &[132]), None, "not without Shift");
        assert!(shift_click(&mut session, &[132]).is_some());
    }

    /// [`catalog`] with an unknown 137 at 132's place: 130 lists 131, 131
    /// lists 132.
    fn with_137() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.star_map.push(star(137, (600.0, 600.0), &[]));
        catalog
    }

    #[test]
    fn with_shift_and_a_course_an_unknown_system_at_the_last_hops_position_is_accepted() {
        let mut session = Session::start(&with_137()).expect("starts");
        session.pilot.course = ids(&[131, 132]);
        assert!(!session.is_known(SystemId(137)));
        assert_eq!(click(&mut session, &[137]), None, "not without Shift");
        assert!(shift_click(&mut session, &[137]).is_some());
    }

    #[test]
    fn a_shadowed_system_at_a_links_place_is_accepted_as_the_link() {
        // 134 shares 131's place, and 131, the lower, stands for it.
        let mut session = Session::start(&alternates()).expect("starts");
        session.chart.known.clear();
        assert_eq!(click(&mut session, &[134]), selects(131));
        assert_eq!(session.nav_mode(), hyperspace(131));
    }

    #[test]
    fn a_link_with_no_visible_system_at_its_place_accepts_nothing() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        assert!(hide(&mut session, &catalog, &[131, 134]));
        session.chart.known.clear();
        assert_eq!(click(&mut session, &[132]), None);
    }

    #[test]
    fn a_link_only_the_hit_lists_is_accepted_both_ways_alone() {
        // 136 lists 130, one way.
        let mut session = hub_session();
        assert_eq!(click(&mut session, &[136]), None, "by the engine");
        let mut session = hub_session().with_hyperlinks(HyperlinkRule::BothWays);
        assert_eq!(click(&mut session, &[136]), selects(136));
        assert_eq!(session.nav_mode(), hyperspace(136), "a jump both ways");
    }

    // The plain click.

    #[test]
    fn a_plain_click_on_a_linked_system_selects_it_in_mode_3_and_keeps_the_course() {
        let mut session = hub_session();
        session.pilot.course = ids(&[131, 132]);
        assert_eq!(click(&mut session, &[134]), selects(134));
        assert_eq!(session.nav_mode(), hyperspace(134));
        assert_eq!(session.course(), ids(&[131, 132]), "the course untouched");
    }

    #[test]
    fn a_plain_click_notes_the_first_slot_at_its_place_for_the_cycle() {
        // 130 lists 131 (hidden behind 134), 134 and 132: the click on 134
        // notes slot 0, so the cycle steps on to slot 1, 134 again.
        let mut catalog = catalog();
        catalog.star_map[0] = star(130, (0.0, 0.0), &[131, 134, 132]);
        with_alternate(&mut catalog);
        let mut session = Session::start(&catalog).expect("starts");
        assert!(hide(&mut session, &catalog, &[131]));
        assert_eq!(click(&mut session, &[134]), selects(134));
        assert!(session.cycle_map_selection());
        assert_eq!(session.chart.nav, hyperspace(134), "slot 1, after slot 0");
    }

    #[test]
    fn a_plain_click_on_an_unlinked_system_lets_go_of_the_selection_and_keeps_the_mode() {
        for (before, after) in [
            (hyperspace(134), NavMode::Hyperspace(None)),
            (NavMode::Stellar(None), NavMode::Stellar(None)),
            (NavMode::Off, NavMode::Off),
        ] {
            let mut session = hub_session();
            session.mark_known([SystemId(133)]);
            session.chart.nav = before;
            session.pilot.course = ids(&[131]);
            assert_eq!(click(&mut session, &[133]), selects(133), "{before:?}");
            assert_eq!(session.nav_mode(), after, "{before:?}");
            assert_eq!(session.course(), ids(&[131]), "{before:?}");
        }
    }

    #[test]
    fn a_plain_click_from_a_stellar_selects_a_linked_system_in_mode_3() {
        let mut session = hub_session();
        session.select_stellar(StellarPick::Slot(0));
        click(&mut session, &[135]);
        assert_eq!(session.nav_mode(), hyperspace(135));
    }

    #[test]
    fn docked_a_plain_click_only_selects() {
        let mut session = hub_session();
        land_now(&mut session).expect("lands");
        let nav = session.nav_mode();
        assert_eq!(
            click(&mut session, &[134]),
            Some(MapClicked {
                nav_redraw: false,
                ..selects(134).expect("a click")
            })
        );
        assert_eq!(session.nav_mode(), nav);
        assert_eq!(session.course(), []);
    }

    // The hypergate map.

    /// A press on the hypergate map offering `offered`, hitting `hits`.
    fn gate_click(
        session: &mut Session,
        hits: &[i16],
        offered: &[i16],
        shift: bool,
    ) -> Option<MapClicked> {
        let (hits, offered) = (ids(hits), ids(offered));
        session.map_click(&MapPress {
            hypergate: Some(&offered),
            ..press(&hits, shift)
        })
    }

    #[allow(clippy::unnecessary_wraps)] // Compared with `map_click`'s answer.
    fn gate_picks(selected: i16, pick: Option<i16>) -> Option<MapClicked> {
        Some(MapClicked {
            selected: Some(SystemId(selected)),
            gate_pick: Some(pick.map(SystemId)),
            clear_route: None,
            course_changed: false,
            nav_redraw: false,
        })
    }

    #[test]
    fn the_hypergate_map_picks_an_offered_system_known_or_not() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert!(!session.is_known(SystemId(132)));
        assert_eq!(
            gate_click(&mut session, &[132], &[131, 132], false),
            gate_picks(132, Some(132))
        );
        assert_eq!(session.nav_mode(), NavMode::Off, "no nav written");
    }

    #[test]
    fn the_hypergate_maps_current_system_is_selected_with_no_pick() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            gate_click(&mut session, &[130], &[131, 132], false),
            gate_picks(130, None)
        );
    }

    #[test]
    fn the_hypergate_map_passes_over_a_known_system_the_gate_does_not_offer() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert!(session.is_known(SystemId(131)));
        assert_eq!(gate_click(&mut session, &[131], &[132], false), None);
        assert_eq!(
            gate_click(&mut session, &[131, 132], &[132], false),
            gate_picks(132, Some(132))
        );
    }

    #[test]
    fn the_hypergate_maps_current_system_still_needs_the_acceptance() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.chart.known.clear();
        assert_eq!(gate_click(&mut session, &[130], &[131], false), None);
    }

    #[test]
    fn the_hypergate_map_ignores_shift() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.course = ids(&[131]);
        assert_eq!(
            gate_click(&mut session, &[131], &[131, 132], true),
            gate_picks(131, Some(131))
        );
        assert_eq!(session.course(), ids(&[131]), "the course untouched");
    }

    // The Shift-click.

    #[test]
    fn a_shift_click_appends_a_linked_system_and_writes_no_nav() {
        let mut session = hub_session();
        assert_eq!(shift_click(&mut session, &[131]), shifted(Some(131)));
        assert_eq!(session.course(), ids(&[131]));
        assert_eq!(
            session.pilot.course_start(),
            Some(session.system()),
            "the new start written back"
        );
        assert_eq!(session.nav_mode(), NavMode::Off, "nothing cued");
        session.pilot.explore(SystemId(131));
        assert_eq!(shift_click(&mut session, &[132]), shifted(Some(132)));
        assert_eq!(session.course(), ids(&[131, 132]));
        assert_eq!(session.nav_mode(), NavMode::Off);
    }

    #[test]
    fn a_shift_click_on_an_unlinked_system_changes_nothing() {
        let mut session = hub_session();
        session.mark_known([SystemId(133)]);
        session.pilot.course = ids(&[134]);
        assert_eq!(shift_click(&mut session, &[133]), shifted(None));
        assert_eq!(session.course(), ids(&[134]));
    }

    #[test]
    fn a_shift_click_appends_a_link_only_the_pick_lists_both_ways_alone() {
        // 136 lists 130, one way.
        let mut session = hub_session();
        assert_eq!(shift_click(&mut session, &[136]), None, "by the engine");
        assert_eq!(session.course(), []);
        let mut session = hub_session().with_hyperlinks(HyperlinkRule::BothWays);
        assert_eq!(shift_click(&mut session, &[136]), shifted(Some(136)));
        assert_eq!(session.course(), ids(&[136]), "a jump both ways");
    }

    #[test]
    fn a_shift_click_is_refused_from_an_unexplored_hop_to_an_unknown_system() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.course = ids(&[131]);
        assert!(!session.is_known(SystemId(132)));
        assert_eq!(shift_click(&mut session, &[132]), shifted(None));
        assert_eq!(session.course(), ids(&[131]));
        session.pilot.explore(SystemId(131));
        assert_eq!(shift_click(&mut session, &[132]), shifted(Some(132)));
        session.pilot.course = ids(&[131]);
        session.pilot.explored.clear();
        session.mark_known([SystemId(132)]);
        assert_eq!(
            shift_click(&mut session, &[132]),
            shifted(Some(132)),
            "the pick known lets it through"
        );
    }

    #[test]
    fn a_shift_click_on_a_hop_cuts_the_course_and_puts_it_back() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.course = ids(&[131, 132]);
        assert_eq!(shift_click(&mut session, &[131]), shifted(Some(131)));
        assert_eq!(session.course(), ids(&[131]));
    }

    #[test]
    fn a_shift_click_on_the_last_hop_leaves_the_course_as_it_was() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.explore(SystemId(131));
        session.pilot.course = ids(&[131, 132]);
        assert_eq!(shift_click(&mut session, &[132]), shifted(Some(132)));
        assert_eq!(session.course(), ids(&[131, 132]));
    }

    #[test]
    fn a_shift_click_at_the_current_systems_place_clears_the_course() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.course = ids(&[131, 132]);
        session.pilot.course_start = Some(SystemId(130));
        session.chart.nav = hyperspace(131);
        assert_eq!(shift_click(&mut session, &[130]), shifted(None));
        assert_eq!(session.course(), []);
        assert_eq!(session.pilot.course_start(), None, "the start cleared");
        assert_eq!(session.nav_mode(), hyperspace(131), "no nav written");
    }

    /// 130 lists 131 and 200; 131 to 162 in a line, each listing the
    /// next; all known.
    fn line() -> Session {
        let mut catalog = catalog();
        catalog.star_map = (0..=32)
            .map(|step: i16| {
                let id = 130 + step;
                let links = if id == 130 {
                    vec![131, 200]
                } else {
                    vec![id + 1]
                };
                star(id, (f32::from(step) * 10.0, 0.0), &links)
            })
            .chain([star(200, (0.0, 50.0), &[])])
            .collect();
        let mut session = Session::start(&catalog).expect("starts");
        session.mark_known((131..=162).chain([200]).map(SystemId));
        for id in 131..=161 {
            session.pilot.explore(SystemId(id));
        }
        session
    }

    #[test]
    fn a_shift_click_on_a_full_course_overwrites_its_first_hop_from_the_current_system() {
        let mut session = line();
        let full: Vec<i16> = (131..=161).collect();
        assert_eq!(full.len(), ROUTE_HOPS);
        session.pilot.course = ids(&full);
        assert_eq!(
            shift_click(&mut session, &[162]),
            shifted(None),
            "161's link"
        );
        assert_eq!(session.course(), ids(&full));
        assert_eq!(shift_click(&mut session, &[200]), shifted(Some(200)));
        let mut wrapped = ids(&full);
        wrapped[0] = SystemId(200);
        assert_eq!(session.course(), wrapped);
    }

    #[test]
    fn docked_a_shift_click_builds_the_course_and_writes_no_nav() {
        let mut session = hub_session();
        land_now(&mut session).expect("lands");
        let nav = session.nav_mode();
        assert_eq!(shift_click(&mut session, &[131]), shifted(Some(131)));
        assert_eq!(session.course(), ids(&[131]));
        assert_eq!(session.nav_mode(), nav);
    }

    // Clear Route.

    #[test]
    fn clear_route_empties_the_course_and_lets_go_of_the_selection_keeping_the_mode() {
        let mut session = hub_session();
        session.pilot.course = ids(&[131, 132]);
        session.pilot.course_start = Some(SystemId(133));
        session.chart.nav = hyperspace(131);
        session.clear_route();
        assert_eq!(session.course(), []);
        assert_eq!(
            session.pilot.course_start(),
            Some(session.system()),
            "started at the current system"
        );
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));

        let mut session = hub_session();
        land_now(&mut session).expect("lands");
        session.pilot.course = ids(&[131]);
        session.clear_route();
        assert_eq!(session.course(), []);
        assert_eq!(session.nav_mode(), NavMode::Stellar(None), "docked");
    }

    // The readings.

    fn by_the_planner(session: Session) -> Session {
        session
            .with_rules(Rulebook::default().with_override(RuleKey::MapPlotting, RuleSource::Bible))
    }

    #[test]
    fn by_the_other_reading_a_click_plots_the_shortest_course_and_cues_it_shift_or_not() {
        for shift in [false, true] {
            let mut session = by_the_planner(Session::start(&catalog()).expect("starts"));
            session.mark_known([SystemId(132)]);
            assert_eq!(
                session.map_click(&press(&ids(&[132]), shift)),
                Some(MapClicked {
                    selected: Some(SystemId(132)),
                    gate_pick: None,
                    clear_route: Some(true),
                    course_changed: true,
                    nav_redraw: true,
                }),
                "shift {shift}"
            );
            assert_eq!(session.course(), ids(&[131, 132]), "shift {shift}");
            assert_eq!(session.nav_mode(), hyperspace(131), "shift {shift}");
        }
    }

    #[test]
    fn by_the_other_reading_a_click_with_no_route_clears_the_course() {
        let mut session = by_the_planner(hub_session());
        session.mark_known([SystemId(133)]);
        session.pilot.course = ids(&[131]);
        let clicked = click(&mut session, &[133]).expect("accepted");
        assert_eq!(clicked.clear_route, Some(false), "turned off");
        assert!(clicked.course_changed);
        assert_eq!(session.course(), []);
    }

    #[test]
    fn by_the_other_reading_docked_a_click_plots_without_a_nav_redraw() {
        let mut session = by_the_planner(hub_session());
        land_now(&mut session).expect("lands");
        let clicked = click(&mut session, &[131]).expect("accepted");
        assert!(!clicked.nav_redraw);
        assert_eq!(session.course(), ids(&[131]));
    }

    #[test]
    fn by_the_other_reading_the_hypergate_map_is_the_engines() {
        let mut session = by_the_planner(Session::start(&catalog()).expect("starts"));
        assert_eq!(
            gate_click(&mut session, &[132], &[132], false),
            gate_picks(132, Some(132))
        );
        assert_eq!(session.course(), []);
    }

    #[test]
    fn by_the_default_a_plain_click_leaves_the_course_and_a_shift_click_cues_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.mark_known([SystemId(132)]);
        click(&mut session, &[132]);
        assert_eq!(session.course(), []);
        shift_click(&mut session, &[131]);
        assert_eq!(session.course(), ids(&[131]));
        assert_eq!(session.nav_mode(), NavMode::Off, "nothing cued");
    }
}
