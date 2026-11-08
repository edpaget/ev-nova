//! The jump points' gates on the galaxy: each gate's stability and its
//! owner's government, as play leaves its point
//! ([`gate_points`](crate::storylines::gate_points::gate_points)).
//!
//! The session hands the galaxy every gate's point
//! ([`OverlaidGalaxy::set_gate_points`]), and the galaxy reads them in
//! three places:
//!
//! - **The lanes.** Each gate's stability is its point's
//!   ([`OverlaidGalaxy::gate_stability`]), stable for a gate with none,
//!   so the lanes, and the map built from them, follow the points
//!   (see the `gate_lanes` module): a closed lane is no link.
//! - **A gate's links.** A link is open when the
//!   [`worse`](GateStability::worse) of its two ends' stabilities is
//!   ([`OverlaidGalaxy::open_links`]); the hypergate entry offers and
//!   takes open links alone.
//! - **The stamps on a site** ([`OverlaidGalaxy::site`]): a hypergate
//!   with links, none of them open, is
//!   [`gate_closed`](crate::catalog::LandingSite::gate_closed), so the
//!   land key refuses it and no NPC uses it; a gate with no links at all,
//!   a wormhole and any other stellar never are. A point's owner's
//!   government is the gate's, over the overlay's.
//!
//! A change of the points rebuilds the lanes and the map; the same points
//! again rebuild nothing. The stock game has no points: every gate is
//! stable, none is closed and each keeps its government.

use std::collections::BTreeMap;

use super::OverlaidGalaxy;
use crate::catalog::{GateStability, GovtId, StellarId};
use crate::gate::{GateKind, has_links};
use crate::storylines::gate_points::GatePoint;

impl OverlaidGalaxy {
    /// Reads each jump point's gate as `points` gives it, by its `spöb`,
    /// instead, rebuilding the lanes and the map when they differ, and
    /// says whether it did (see the module docs).
    pub fn set_gate_points(&mut self, points: BTreeMap<StellarId, GatePoint>) -> bool {
        if points == self.gate_points {
            return false;
        }
        self.gate_points = points;
        self.rebuild();
        true
    }

    /// The stability of the gate `gate`: its point's, or stable for a
    /// gate with none, as every stock gate is.
    #[must_use]
    pub fn gate_stability(&self, gate: StellarId) -> GateStability {
        self.gate_points
            .get(&gate)
            .map_or(GateStability::Stable, |point| point.stability)
    }

    /// The government of the faction holding `gate`'s point, if it has
    /// one the data numbers: what the gate's site is stamped with.
    #[must_use]
    pub fn gate_govt(&self, gate: StellarId) -> Option<GovtId> {
        self.gate_points.get(&gate).and_then(|point| point.govt)
    }

    /// `links`, the links of the gate `gate`, with each closed one left
    /// out (`None`): a link is open when the worse of `gate`'s and the
    /// linked gate's stabilities [is open](GateStability::is_open).
    #[must_use]
    pub fn open_links(
        &self,
        gate: StellarId,
        links: [Option<StellarId>; 8],
    ) -> [Option<StellarId>; 8] {
        let own = self.gate_stability(gate);
        links.map(|link| link.filter(|&to| own.worse(self.gate_stability(to)).is_open()))
    }

    /// Whether the stellar `stellar` is a hypergate among the gates the
    /// lanes are built from whose links are all closed: it has links
    /// ([`has_links`]) and none of them is open. Never for a gate with no
    /// links, a wormhole or any other stellar.
    #[must_use]
    pub fn gate_closed(&self, stellar: StellarId) -> bool {
        self.gates
            .iter()
            .find(|site| site.id == stellar)
            .filter(|site| GateKind::of(site.flags2) == Some(GateKind::Hypergate))
            .is_some_and(|site| {
                has_links(&site.links) && !has_links(&self.open_links(site.id, site.links))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GateSite, LandingSite, ListedStellar, SystemId};
    use crate::gate::{HYPERGATE, WORMHOLE};
    use crate::geometry::Vec2;
    use crate::hyperspace::{HyperlinkRule, RouteError};
    use crate::story::{GalaxyOverlay, LinkSource};
    use nova_sim_catalog::fixture::{list_also, planet, star};

    fn ids(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    /// Gate `id` of `flags2` linking `to`.
    fn gate(id: i16, flags2: u16, to: &[i16]) -> GateSite {
        let mut links = [None; 8];
        for (slot, &link) in links.iter_mut().zip(to) {
            *slot = Some(StellarId(link));
        }
        GateSite {
            id: StellarId(id),
            position: Vec2::ZERO,
            flags2,
            links,
            exit_angle: -1,
        }
    }

    /// Systems 1 and 2, linked only by the hypergates 1001 (in 1) ↔ 2001
    /// (in 2), each gate in the system its ID's thousands give; a lone hypergate 1002 with no links and a wormhole 1003 to 2001,
    /// both in 1; links by the gates.
    fn galaxy() -> OverlaidGalaxy {
        let systems = vec![star(1, (0.0, 0.0), &[]), star(2, (10.0, 0.0), &[])];
        let sites = [
            gate(1001, HYPERGATE, &[2001]),
            gate(1002, HYPERGATE, &[]),
            gate(1003, WORMHOLE, &[2001]),
            gate(2001, HYPERGATE, &[1001]),
        ];
        let mut listed: Vec<ListedStellar> = Vec::new();
        list_also(
            &mut listed,
            sites
                .iter()
                .map(|site| (SystemId(site.id.0 / 1000), site.id)),
        );
        let mut galaxy =
            OverlaidGalaxy::new(systems, GalaxyOverlay::default()).with_gates(&sites, &listed);
        galaxy.set_link_source(LinkSource::Gates);
        galaxy
    }

    fn points(list: &[(i16, GateStability, Option<i16>)]) -> BTreeMap<StellarId, GatePoint> {
        list.iter()
            .map(|&(gate, stability, govt)| {
                (
                    StellarId(gate),
                    GatePoint {
                        stability,
                        govt: govt.map(GovtId),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn a_dead_far_end_closes_the_lane_and_a_stable_one_opens_it_again() {
        let mut galaxy = galaxy();
        assert_eq!(galaxy.lanes().listed(SystemId(1)), ids(&[2]));
        assert!(galaxy.set_gate_points(points(&[(2001, GateStability::Dead, None)])));
        assert_eq!(galaxy.lanes().listed(SystemId(1)), ids(&[]));
        assert_eq!(galaxy.lanes().closed(SystemId(1)), ids(&[2]));
        assert_eq!(galaxy.map().listed_links(SystemId(1)), ids(&[]));
        assert_eq!(
            galaxy
                .map()
                .route(SystemId(1), SystemId(2), HyperlinkRule::Engine),
            Err(RouteError::Unreachable)
        );
        assert!(galaxy.set_gate_points(points(&[(2001, GateStability::Stable, None)])));
        assert_eq!(galaxy.lanes().listed(SystemId(1)), ids(&[2]));
        assert_eq!(galaxy.map().listed_links(SystemId(1)), ids(&[2]));
    }

    #[test]
    fn the_same_points_again_or_none_rebuild_nothing() {
        let mut galaxy = galaxy();
        assert!(!galaxy.set_gate_points(BTreeMap::new()), "the stock case");
        let unstable = points(&[(1001, GateStability::Unstable, None)]);
        assert!(galaxy.set_gate_points(unstable.clone()));
        assert!(!galaxy.set_gate_points(unstable));
    }

    #[test]
    fn a_gate_with_no_point_is_stable_and_one_with_a_point_reads_it() {
        let mut galaxy = galaxy();
        assert_eq!(
            galaxy.gate_stability(StellarId(1001)),
            GateStability::Stable
        );
        galaxy.set_gate_points(points(&[(1001, GateStability::Unstable, Some(130))]));
        assert_eq!(
            galaxy.gate_stability(StellarId(1001)),
            GateStability::Unstable
        );
        assert_eq!(
            galaxy.gate_stability(StellarId(2001)),
            GateStability::Stable
        );
        assert_eq!(galaxy.gate_govt(StellarId(1001)), Some(GovtId(130)));
        assert_eq!(galaxy.gate_govt(StellarId(2001)), None);
    }

    #[test]
    fn a_link_is_open_only_while_both_its_ends_are_stable() {
        let mut galaxy = galaxy();
        let links = gate(1001, HYPERGATE, &[2001, 2002]).links;
        assert_eq!(galaxy.open_links(StellarId(1001), links), links);
        galaxy.set_gate_points(points(&[(2001, GateStability::Unstable, None)]));
        let open = galaxy.open_links(StellarId(1001), links);
        assert_eq!(open[..2], [None, Some(StellarId(2002))]);
        galaxy.set_gate_points(points(&[(1001, GateStability::Dead, None)]));
        assert_eq!(galaxy.open_links(StellarId(1001), links), [None; 8]);
    }

    fn site_of(id: i16, flags2: u16) -> LandingSite {
        LandingSite {
            flags2,
            ..planet(id, 0.0, 0.0)
        }
    }

    #[test]
    fn a_hypergate_whose_links_are_all_closed_is_stamped_closed() {
        let mut galaxy = galaxy();
        assert!(!galaxy.site(site_of(1001, HYPERGATE)).gate_closed, "open");
        galaxy.set_gate_points(points(&[(2001, GateStability::Dead, None)]));
        assert!(galaxy.gate_closed(StellarId(1001)));
        assert!(galaxy.site(site_of(1001, HYPERGATE)).gate_closed);
        assert!(
            galaxy.site(site_of(2001, HYPERGATE)).gate_closed,
            "its own end dead"
        );
    }

    #[test]
    fn a_gate_with_no_links_a_wormhole_and_a_planet_are_never_closed() {
        let mut galaxy = galaxy();
        galaxy.set_gate_points(points(&[
            (1002, GateStability::Dead, None),
            (1003, GateStability::Dead, None),
            (2001, GateStability::Dead, None),
            (128, GateStability::Dead, None),
        ]));
        assert!(
            !galaxy.site(site_of(1002, HYPERGATE)).gate_closed,
            "no links"
        );
        assert!(
            !galaxy.site(site_of(1003, WORMHOLE)).gate_closed,
            "a wormhole"
        );
        assert!(!galaxy.site(planet(128, 0.0, 0.0)).gate_closed, "a planet");
    }

    #[test]
    fn a_points_owner_is_the_gates_government_over_the_overlays() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_stellar_govt(StellarId(1001), Some(Some(GovtId(140))));
        overlay.set_stellar_govt(StellarId(2001), Some(Some(GovtId(141))));
        let mut galaxy = galaxy();
        galaxy.set_overlay(overlay);
        galaxy.set_gate_points(points(&[
            (1001, GateStability::Stable, Some(130)),
            (2001, GateStability::Stable, None),
        ]));
        let govt = |id| galaxy.site(site_of(id, HYPERGATE)).govt;
        assert_eq!(govt(1001), Some(GovtId(130)), "the owner's");
        assert_eq!(govt(2001), Some(GovtId(141)), "no owner: the overlay's");
        assert_eq!(
            galaxy.site(site_of(1002, HYPERGATE)).govt,
            None,
            "no point: its own"
        );
    }

    #[test]
    fn with_no_points_a_site_is_stamped_as_before() {
        let galaxy = galaxy();
        let site = site_of(1001, HYPERGATE);
        assert_eq!(galaxy.site(site), site);
    }
}
