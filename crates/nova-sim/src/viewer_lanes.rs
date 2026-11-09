//! The links the developer galaxy viewer draws, by
//! [`RuleKey::GateLanes`](crate::RuleKey::GateLanes) and
//! [`RuleKey::MapLaneStyle`](crate::RuleKey::MapLaneStyle).
//!
//! The viewer (`nova_view::galaxy::GalaxyMap` in viewer mode) has no
//! session: it shows every system of the catalog. It is built before any
//! rulebook is known, so its catalog hands it both sets of links: the
//! `sÿst` Con links, and a fresh pilot's gate lanes ([`GateLinks`],
//! [`gate_links`]). [`viewer_links`] then picks one by the rulebook.
//!
//! - By `GateLanes`' engine reading (stock), the viewer draws the Con
//!   links, grey, and no closed lanes, whatever `MapLaneStyle` reads. The
//!   in-flight map would draw each Con link in its look and show the
//!   closed lanes under `MapLaneStyle`'s other reading; the viewer
//!   simplifies both away, as the stock data has no gate lane to show.
//! - By its other reading (the new game's), the viewer draws the gate
//!   lanes. By `MapLaneStyle`'s other reading too, each in its look
//!   ([`lane_look`]), with every closed lane in its look, each pair once
//!   and none drawn as a link. The viewer has no explored or known filter,
//!   so it draws every closed lane, not only those the in-flight map
//!   draws from.
//!
//! The original has no reading to port: it has no viewer. Building one
//! plays no sound and draws nothing random.

use std::collections::BTreeSet;

use crate::catalog::{PilotCatalog, SystemId};
use crate::hyperspace::CoLocated;
use crate::map_lanes::{LaneLook, lane_look};
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::story::{GalaxyOverlay, LinkSource, OverlaidGalaxy};
use crate::storylines::state::StoryState;

/// A fresh pilot's gate lanes, rule-free: the data [`viewer_links`]
/// chooses from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GateLinks {
    /// The gate-lane links, each pair once, lower ID first, sorted.
    pub links: Vec<(SystemId, SystemId)>,
    /// One look per link, in the same order.
    pub looks: Vec<LaneLook>,
    /// The closed lanes' looks: each pair once, none a link.
    pub closed: Vec<LaneLook>,
}

/// What the viewer draws: its links, each link's look (empty to draw
/// them all grey) and the closed lanes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewerLinks {
    /// The links, each pair once.
    pub links: Vec<(SystemId, SystemId)>,
    /// One look per link, in order, or none.
    pub lanes: Vec<LaneLook>,
    /// The closed lanes' looks.
    pub closed_lanes: Vec<LaneLook>,
}

/// A fresh pilot's gate lanes over `catalog`: the default overlay, gate
/// points over a fresh story, the engine's co-located reading and the
/// gate lanes as the links.
#[must_use]
pub fn gate_links(catalog: &impl PilotCatalog) -> GateLinks {
    let listed = catalog.listed_stellars();
    let mut galaxy = OverlaidGalaxy::new(catalog.star_map(), GalaxyOverlay::default())
        .with_gates(&catalog.gate_sites(), &listed);
    galaxy.set_gate_points(crate::storylines::gate_points::gate_points(
        &catalog.storylines(),
        &StoryState::default(),
    ));
    galaxy.set_colocated(CoLocated::Lowest);
    galaxy.set_link_source(LinkSource::Gates);
    let links = galaxy.map().links();
    let looks = links
        .iter()
        .map(|&(from, to)| lane_look(&galaxy, from, to))
        .collect();
    let mut paired: BTreeSet<(SystemId, SystemId)> = links.iter().copied().collect();
    let mut closed = Vec::new();
    for from in galaxy.catalog_systems() {
        for to in galaxy.lanes().closed(from) {
            if paired.insert((from.min(to), from.max(to))) {
                closed.push(lane_look(&galaxy, from, to));
            }
        }
    }
    GateLinks {
        links,
        looks,
        closed,
    }
}

/// What the viewer draws by `rules`, from the Con links `con` and the
/// gate lanes `gates` (see the module docs).
#[must_use]
pub fn viewer_links(
    con: &[(SystemId, SystemId)],
    gates: &GateLinks,
    rules: &Rulebook,
) -> ViewerLinks {
    if rules.source_for(RuleKey::GateLanes) == RuleSource::Engine {
        return ViewerLinks {
            links: con.to_vec(),
            ..ViewerLinks::default()
        };
    }
    let styled = rules.source_for(RuleKey::MapLaneStyle) == RuleSource::Bible;
    ViewerLinks {
        links: gates.links.clone(),
        lanes: if styled {
            gates.looks.clone()
        } else {
            Vec::new()
        },
        closed_lanes: if styled {
            gates.closed.clone()
        } else {
            Vec::new()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::GateStability;
    use crate::map_lanes::Back;
    use crate::testkit::FakePilotCatalog;
    use crate::testkit::{DRIFT, VERGE, laned};

    fn id(n: i16) -> SystemId {
        SystemId(n)
    }

    fn look(from: i16, to: i16) -> LaneLook {
        LaneLook {
            from: id(from),
            to: id(to),
            out: Some(GateStability::Stable),
            back: Back::OneWay,
        }
    }

    fn gates() -> GateLinks {
        GateLinks {
            links: vec![(id(1), id(2))],
            looks: vec![look(1, 2)],
            closed: vec![look(2, 3)],
        }
    }

    const CON: [(SystemId, SystemId); 1] = [(SystemId(5), SystemId(6))];

    fn rules(lanes: RuleSource, style: RuleSource) -> Rulebook {
        Rulebook::default()
            .with_override(RuleKey::GateLanes, lanes)
            .with_override(RuleKey::MapLaneStyle, style)
    }

    #[test]
    fn the_engine_reading_draws_the_con_links_grey() {
        let drawn = viewer_links(&CON, &gates(), &Rulebook::default());
        assert_eq!(
            drawn,
            ViewerLinks {
                links: CON.to_vec(),
                ..ViewerLinks::default()
            }
        );
        let explicit = viewer_links(
            &CON,
            &gates(),
            &rules(RuleSource::Engine, RuleSource::Engine),
        );
        assert_eq!(explicit, drawn);
    }

    #[test]
    fn the_lane_style_alone_still_draws_the_con_links_and_no_closed_lanes() {
        let drawn = viewer_links(
            &CON,
            &gates(),
            &rules(RuleSource::Engine, RuleSource::Bible),
        );
        assert_eq!(
            drawn,
            ViewerLinks {
                links: CON.to_vec(),
                ..ViewerLinks::default()
            }
        );
    }

    #[test]
    fn the_gate_lanes_alone_draw_the_gate_links_grey() {
        let drawn = viewer_links(
            &CON,
            &gates(),
            &rules(RuleSource::Bible, RuleSource::Engine),
        );
        assert_eq!(
            drawn,
            ViewerLinks {
                links: gates().links,
                ..ViewerLinks::default()
            }
        );
    }

    #[test]
    fn both_keys_draw_the_gate_links_in_their_looks_and_the_closed_lanes() {
        let drawn = viewer_links(&CON, &gates(), &rules(RuleSource::Bible, RuleSource::Bible));
        assert_eq!(
            drawn,
            ViewerLinks {
                links: gates().links,
                lanes: gates().looks,
                closed_lanes: gates().closed,
            }
        );
    }

    /// The session's galaxy for `catalog`, as `Session` builds it at a
    /// load, under the gate lanes.
    fn session_galaxy(catalog: &FakePilotCatalog) -> OverlaidGalaxy {
        let listed = catalog.listed_stellars();
        let mut galaxy = OverlaidGalaxy::new(catalog.star_map(), GalaxyOverlay::default())
            .with_gates(&catalog.gate_sites(), &listed);
        galaxy.set_gate_points(crate::storylines::gate_points::gate_points(
            &catalog.storylines(),
            &StoryState::default(),
        ));
        galaxy.set_colocated(CoLocated::from(RuleSource::Engine));
        galaxy.set_link_source(LinkSource::Gates);
        galaxy
    }

    #[test]
    fn a_fresh_pilots_gate_links_are_the_sessions_in_their_looks() {
        let catalog = laned();
        let galaxy = session_galaxy(&catalog);
        let gates = gate_links(&catalog);
        assert_eq!(gates.links, galaxy.map().links());
        assert!(!gates.links.is_empty());
        let looks: Vec<LaneLook> = gates
            .links
            .iter()
            .map(|&(from, to)| lane_look(&galaxy, from, to))
            .collect();
        assert_eq!(gates.looks, looks);
    }

    #[test]
    fn each_closed_lane_is_drawn_once_and_is_no_link() {
        let catalog = laned();
        let galaxy = session_galaxy(&catalog);
        let gates = gate_links(&catalog);
        let pairs: Vec<(SystemId, SystemId)> = gates
            .closed
            .iter()
            .map(|look| (look.from.min(look.to), look.from.max(look.to)))
            .collect();
        assert_eq!(pairs, vec![(DRIFT, VERGE)]);
        assert!(!gates.links.contains(&(DRIFT, VERGE)));
        let closed = &gates.closed[0];
        assert_eq!(*closed, lane_look(&galaxy, closed.from, closed.to));
    }
}
