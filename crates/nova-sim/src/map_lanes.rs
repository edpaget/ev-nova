//! How the galaxy map draws each link by
//! [`RuleKey::MapLaneStyle`](crate::RuleKey::MapLaneStyle)'s other reading,
//! the new game's: each way of a link in its gate lane's stability's look,
//! and a one-way link marked ([`LaneLook`], [`lane_look`]).
//!
//! The original has no reading to port: `_DrawMap` draws every link in one
//! grey (`_rgbLessDarkGrey` @0x1bf26, `_rgbRealDarkGrey` @0x1befa on the
//! hypergate map) and draws neither stability nor direction
//! (@0xe7a8-0xed31). That stays the engine's reading
//! ([`galaxy_map`](crate::galaxy_map)).
//!
//! The map draws each pair of systems once, so a drawn link's look carries
//! both directions, each read from the galaxy's gate lanes
//! ([`OverlaidGalaxy::lanes`]):
//!
//! - the way out, `from` → `to`: the lane's stability, or `None` where no
//!   gate serves that way (a Con link, drawn grey);
//! - the way back, `to` → `from` ([`Back`]): the same, where the far
//!   system lists the link back (its links on the map,
//!   [`StarMap::listed_links`](crate::StarMap::listed_links)) or its lane
//!   back is closed, else [`Back::OneWay`]. So a lane whose way back is
//!   closed reads that way's stability, not one-way.
//!
//! A closed lane ([`GateLanes::closed`](crate::gate_lanes::GateLanes::closed)),
//! its gates all unstable or dead, is no link: no reader takes it, so the
//! three link sets leave it out. The drawing keeps it, in its look, apart
//! from the links ([`MapDrawing::closed_lanes`]): each from the current
//! system or a visible, explored and known one, as the links are drawn
//! from, each pair once and none drawn as a link either way, and none on
//! the hypergate map.
//!
//! The view draws a link whose two ways look the same as one line in that
//! look, one whose ways differ in two halves, the half nearer each system
//! in its way out, and a one-way link in the way out's look with an
//! arrowhead at `to`. These are data on the drawing ([`MapDrawing::lanes`]),
//! beside the original's links, so the view needs no catalog to draw them.
//! Making one plays no sound and draws nothing random.
//!
//! [`MapDrawing::lanes`]: crate::galaxy_map::MapDrawing::lanes
//! [`MapDrawing::closed_lanes`]: crate::galaxy_map::MapDrawing::closed_lanes

use crate::catalog::{GateStability, SystemId};
use crate::story::OverlaidGalaxy;

/// The way back along a drawn link, `to` → `from`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Back {
    /// The far system doesn't list the link back: one-way, arrowed.
    OneWay,
    /// It does: the way back's stability, or `None` where no gate serves
    /// it, drawn grey.
    Way(Option<GateStability>),
}

/// How one drawn link looks (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaneLook {
    /// The system the link is drawn from.
    pub from: SystemId,
    /// The system it is drawn to.
    pub to: SystemId,
    /// The way out, `from` → `to`: its lane's stability, or `None` where
    /// no gate serves it, drawn grey.
    pub out: Option<GateStability>,
    /// The way back.
    pub back: Back,
}

/// The look of the link `galaxy`'s map draws from `from` to `to` (see the
/// module docs).
#[must_use]
pub fn lane_look(galaxy: &OverlaidGalaxy, from: SystemId, to: SystemId) -> LaneLook {
    let lanes = galaxy.lanes();
    let back = if galaxy.map().listed_links(to).contains(&from) || lanes.closed(to).contains(&from)
    {
        Back::Way(lanes.stability(to, from))
    } else {
        Back::OneWay
    };
    LaneLook {
        from,
        to,
        out: lanes.stability(from, to),
        back,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GateSite, ListedStellar, StellarId};
    use crate::gate::HYPERGATE;
    use crate::geometry::Vec2;
    use crate::story::{GalaxyOverlay, LinkSource};
    use crate::storylines::gate_points::GatePoint;
    use crate::testkit::star;
    use GateStability::{Dead, Stable, Unstable};

    /// Gate `id`, in the system its ID's hundreds give, linking `to`, its
    /// jump point at `stability`.
    fn gate(id: i16, to: &[i16], stability: GateStability) -> (GateSite, GateStability) {
        let mut links = [None; 8];
        for (slot, &link) in links.iter_mut().zip(to) {
            *slot = Some(StellarId(link));
        }
        let site = GateSite {
            id: StellarId(id),
            position: Vec2::ZERO,
            flags2: HYPERGATE,
            links,
            exit_angle: -1,
        };
        (site, stability)
    }

    /// `systems` with `gates` among them, each listed by the system its
    /// ID's hundreds give, each gate's point at its stability, its links
    /// by `source`.
    fn gated(
        systems: Vec<crate::catalog::StarSystem>,
        gates: &[(GateSite, GateStability)],
        source: LinkSource,
    ) -> OverlaidGalaxy {
        let sites: Vec<GateSite> = gates.iter().map(|&(site, _)| site).collect();
        let mut listed: Vec<ListedStellar> = Vec::new();
        nova_sim_catalog::fixture::list_also(
            &mut listed,
            sites
                .iter()
                .map(|site| (SystemId(site.id.0 / 100), site.id)),
        );
        let mut galaxy =
            OverlaidGalaxy::new(systems, GalaxyOverlay::default()).with_gates(&sites, &listed);
        galaxy.set_gate_points(
            gates
                .iter()
                .map(|&(site, stability)| {
                    let point = GatePoint {
                        stability,
                        govt: None,
                    };
                    (site.id, point)
                })
                .collect(),
        );
        galaxy.set_link_source(source);
        galaxy
    }

    /// Systems 1-4, 4 listing 1 by Con, with the nova-data fixture's
    /// gates between drift 1, hearth 2 and verge 3, and a one-way gate
    /// from 1 to 4; its links by `source`.
    fn galaxy(source: LinkSource) -> OverlaidGalaxy {
        let systems = vec![
            star(1, (0.0, 0.0), &[]),
            star(2, (10.0, 0.0), &[]),
            star(3, (20.0, 0.0), &[]),
            star(4, (30.0, 0.0), &[1]),
        ];
        let sites = [
            gate(101, &[201], Stable),
            gate(102, &[303], Dead),
            gate(103, &[302], Unstable),
            gate(104, &[401], Stable),
            gate(201, &[101], Stable),
            gate(202, &[301], Stable),
            gate(203, &[302], Stable),
            gate(301, &[202], Unstable),
            gate(302, &[203], Stable),
            gate(303, &[102], Dead),
            gate(401, &[], Stable),
        ];
        gated(systems, &sites, source)
    }

    fn look(galaxy: &OverlaidGalaxy, from: i16, to: i16) -> LaneLook {
        lane_look(galaxy, SystemId(from), SystemId(to))
    }

    #[test]
    fn two_ways_of_the_same_stability_look_the_same() {
        let galaxy = galaxy(LinkSource::Gates);
        let look = look(&galaxy, 1, 2);
        assert_eq!(look.out, Some(Stable));
        assert_eq!(look.back, Back::Way(Some(Stable)));
        assert_eq!((look.from, look.to), (SystemId(1), SystemId(2)));
    }

    #[test]
    fn the_fixtures_drift_verge_link_is_unstable_out_of_drift_and_dead_out_of_verge() {
        let galaxy = galaxy(LinkSource::Gates);
        let from_drift = look(&galaxy, 1, 3);
        assert_eq!(from_drift.out, Some(Unstable));
        assert_eq!(from_drift.back, Back::Way(Some(Dead)));
        let from_verge = look(&galaxy, 3, 1);
        assert_eq!(from_verge.out, Some(Dead));
        assert_eq!(from_verge.back, Back::Way(Some(Unstable)));
    }

    #[test]
    fn a_one_way_lane_has_no_way_back() {
        let galaxy = galaxy(LinkSource::Gates);
        let look = look(&galaxy, 1, 4);
        assert_eq!(look.out, Some(Stable));
        assert_eq!(look.back, Back::OneWay);
    }

    #[test]
    fn a_stable_way_out_whose_way_back_is_closed_reads_the_way_backs_stability() {
        // 1 → 5 by gate 105, stable at both ends; 5 → 1 by gate 501, dead.
        let systems = vec![star(1, (0.0, 0.0), &[]), star(5, (40.0, 0.0), &[])];
        let sites = [
            gate(105, &[502], Stable),
            gate(502, &[], Stable),
            gate(501, &[106], Dead),
            gate(106, &[], Stable),
        ];
        let galaxy = gated(systems, &sites, LinkSource::Gates);
        let look = look(&galaxy, 1, 5);
        assert_eq!(look.out, Some(Stable));
        assert_eq!(look.back, Back::Way(Some(Dead)), "not one-way");
    }

    #[test]
    fn a_con_link_no_gate_serves_is_grey_and_one_way_when_listed_by_one_end() {
        let galaxy = galaxy(LinkSource::Con);
        let look = look(&galaxy, 4, 1);
        assert_eq!(look.out, None);
        assert_eq!(look.back, Back::OneWay, "1 doesn't list 4 by Con");
        let mut both = galaxy.clone();
        both.set_link_source(LinkSource::Gates);
        assert_eq!(lane_look(&both, SystemId(4), SystemId(1)).out, None);
    }

    #[test]
    fn a_con_link_a_gate_also_serves_is_drawn_in_its_stability() {
        let galaxy = galaxy(LinkSource::Con);
        let look = look(&galaxy, 1, 4);
        assert_eq!(look.out, Some(Stable), "gate 104");
        assert_eq!(look.back, Back::Way(None), "4 lists 1 by Con, no gate back");
    }

    // The drawing's looks (`galaxy_map::draw_map`).

    use std::collections::BTreeSet;

    use crate::galaxy_map::{MapDrawing, MapState, draw_map};
    use crate::iff::SystemColorCode;
    use crate::rulebook::RuleSource;

    /// What the map draws of [`galaxy`] by `source`'s lanes, in 1, with 1
    /// and 3 explored and known, by `style`.
    fn drawn(source: LinkSource, style: RuleSource) -> MapDrawing {
        let galaxy = galaxy(source);
        let seen: BTreeSet<SystemId> = [1, 3].map(SystemId).into();
        let state = MapState {
            galaxy: &galaxy,
            explored: &seen,
            known: &seen,
            current: SystemId(1),
            course_start: None,
            course: &[],
            hyper_slot: None,
            hypergate: None,
            missions: &[],
            green: None,
            multi_arrows: true,
            pairing: RuleSource::Bible,
            lane_style: style,
        };
        draw_map(&state, |_| SystemColorCode::Uninhabited)
    }

    #[test]
    fn by_the_engine_the_drawing_has_no_looks_and_is_otherwise_the_same() {
        for source in [LinkSource::Con, LinkSource::Gates] {
            let engine = drawn(source, RuleSource::Engine);
            assert_eq!(engine.lanes, [], "{source:?}");
            assert_eq!(engine.closed_lanes, [], "{source:?}");
            let other = drawn(source, RuleSource::Bible);
            assert_eq!(
                MapDrawing {
                    lanes: Vec::new(),
                    closed_lanes: Vec::new(),
                    ..other
                },
                engine,
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_closed_lane_is_drawn_in_its_look_but_is_no_link() {
        let drawing = drawn(LinkSource::Gates, RuleSource::Bible);
        let pair = (SystemId(1), SystemId(3));
        assert!(!drawing.current_links.contains(&pair));
        assert!(
            !drawing
                .links
                .iter()
                .any(|link| [(link.from, link.to), (link.to, link.from)].contains(&pair))
        );
        assert_eq!(
            drawing.closed_lanes,
            [LaneLook {
                from: SystemId(1),
                to: SystemId(3),
                out: Some(Unstable),
                back: Back::Way(Some(Dead)),
            }],
            "drift–verge once, from the current system"
        );
    }

    #[test]
    fn a_closed_lane_from_an_unknown_system_is_not_drawn() {
        let galaxy = galaxy(LinkSource::Gates);
        let seen: BTreeSet<SystemId> = [2].map(SystemId).into();
        let state = MapState {
            galaxy: &galaxy,
            explored: &seen,
            known: &seen,
            current: SystemId(2),
            course_start: None,
            course: &[],
            hyper_slot: None,
            hypergate: None,
            missions: &[],
            green: None,
            multi_arrows: true,
            pairing: RuleSource::Bible,
            lane_style: RuleSource::Bible,
        };
        let drawing = draw_map(&state, |_| SystemColorCode::Uninhabited);
        assert_eq!(drawing.closed_lanes, [], "neither drift nor verge known");
    }

    #[test]
    fn a_closed_lane_is_never_on_the_hypergate_map() {
        let galaxy = galaxy(LinkSource::Gates);
        let seen: BTreeSet<SystemId> = [1, 3].map(SystemId).into();
        let offered = [SystemId(2)];
        let state = MapState {
            galaxy: &galaxy,
            explored: &seen,
            known: &seen,
            current: SystemId(1),
            course_start: None,
            course: &[],
            hyper_slot: None,
            hypergate: Some(&offered),
            missions: &[],
            green: None,
            multi_arrows: true,
            pairing: RuleSource::Bible,
            lane_style: RuleSource::Bible,
        };
        let drawing = draw_map(&state, |_| SystemColorCode::Uninhabited);
        assert_eq!(drawing.closed_lanes, []);
    }

    #[test]
    fn by_the_other_reading_each_drawn_link_has_its_look_current_links_first() {
        let drawing = drawn(LinkSource::Gates, RuleSource::Bible);
        let drawn: Vec<(SystemId, SystemId)> = drawing
            .current_links
            .iter()
            .copied()
            .chain(drawing.links.iter().map(|link| (link.from, link.to)))
            .collect();
        assert!(!drawing.links.is_empty());
        let galaxy = galaxy(LinkSource::Gates);
        let looks: Vec<LaneLook> = drawn
            .iter()
            .map(|&(from, to)| lane_look(&galaxy, from, to))
            .collect();
        assert_eq!(drawing.lanes, looks);
        assert_eq!(
            drawing.lanes[0],
            LaneLook {
                from: SystemId(1),
                to: SystemId(2),
                out: Some(Stable),
                back: Back::Way(Some(Stable)),
            }
        );
    }
}
