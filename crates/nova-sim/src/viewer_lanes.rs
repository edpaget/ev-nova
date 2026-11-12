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

pub use nova_sim_frame::viewer_lanes::*;

use std::collections::BTreeSet;

use crate::catalog::{PilotCatalog, SystemId};
use crate::hyperspace::CoLocated;
use crate::map_lanes::lane_look;
use crate::story::{GalaxyOverlay, LinkSource, OverlaidGalaxy};
use crate::storylines::state::StoryState;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_lanes::LaneLook;
    use crate::rulebook::RuleSource;
    use crate::testkit::FakePilotCatalog;
    use crate::testkit::{DRIFT, VERGE, laned};

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
