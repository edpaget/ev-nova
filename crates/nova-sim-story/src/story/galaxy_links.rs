//! Where the galaxy's links between systems come from: the systems' `sÿst`
//! Con slots, as the original reads them everywhere, or the jump points'
//! gate lanes ([`GateLanes`]), as
//! [`RuleKey::GateLanes`](crate::RuleKey::GateLanes)' other reading
//! (the new game's) reads them. The session reads the key once into
//! [`OverlaidGalaxy::set_link_source`], beside the resolution of a shared
//! place, and every link reader follows it with no branch of its own:
//!
//! - the readers of the overlaid star map ([`OverlaidGalaxy::map`]): the
//!   planner, the clicks' acceptance, the cue, J's selection, Hyper
//!   Select's one-jump reading, the map outfits' walk, the view's galaxy
//!   model and the mission pickers' adjacency: a mission stellar's
//!   distance ([`StarMap::listed_links`] or [`StarMap::neighbours`] by
//!   [`RuleKey::MissionStellarDistance`](crate::RuleKey::MissionStellarDistance))
//!   and `AvailStel` 5000-9998 ([`StarMap::listed_links`]), under both of
//!   their readings. By [`LinkSource::Gates`] the map's links are the
//!   lanes ([`StarMap::with_lanes`]).
//! - the readers of a system's slots ([`OverlaidGalaxy::link_slots`]): the
//!   galaxy map's three link sets, the click's and Shift-click's link
//!   tests, Hyper Select, the Tab cycle, the floating map, the floating
//!   map's preselection, the known pass ([`known_from`]) and a new
//!   pilot's neighbours. By [`LinkSource::Gates`] a system's slots are the
//!   systems its lanes reach, each once, by gate ID then slot order.
//!
//! The raw Con slots ([`OverlaidGalaxy::con_slots`],
//! [`StarMap::con_slots`]) stay the catalog's under both: no lane is ever
//! a Con link, so no lane is a hyperspace link either. The readers that
//! must read the raw Con keep them: the cue's −1-slot quirk, a hyperspace
//! arrival's target and a crime's walk. Two mission pickers read the raw
//! Con by the engine's reading of their own keys and the map's links,
//! so the lanes by [`LinkSource::Gates`], by the other: a mission's
//! `ShipSyst` -5 ships
//! ([`RuleKey::MissionShipLinks`](crate::RuleKey::MissionShipLinks),
//! [`select_ship_system`]) and its `AuxShipSyst` 5000-9998 auxiliary
//! ships ([`RuleKey::AuxShipLinks`](crate::RuleKey::AuxShipLinks),
//! [`aux_system_allows`]).
//!
//! The lanes are built under both sources, as the map's lane looks read
//! them by [`RuleKey::MapLaneStyle`](crate::RuleKey::MapLaneStyle) over
//! either, and rebuilt with the map: at [`OverlaidGalaxy::set_overlay`],
//! [`OverlaidGalaxy::set_colocated`], [`OverlaidGalaxy::with_gates`] and a
//! change of [`OverlaidGalaxy::set_link_source`].
//!
//! [`known_from`]: crate::exploration::known_from
//! [`select_ship_system`]: crate::mission::ships::select_ship_system
//! [`aux_system_allows`]: crate::mission::aux_ships::aux_system_allows

use super::OverlaidGalaxy;
use crate::catalog::{GateSite, ListedStellar, SystemId};
use crate::gate::GateKind;
use crate::gate_lanes::GateLanes;
#[cfg(doc)]
use crate::hyperspace::StarMap;
use crate::rulebook::RuleSource;

/// Where every link reader takes the links between systems from
/// ([`RuleKey::GateLanes`](crate::RuleKey::GateLanes)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LinkSource {
    /// The original's reading: the systems' `sÿst` Con slots.
    #[default]
    Con,
    /// The other reading, the new game's: the jump points' gate lanes.
    Gates,
}

impl From<RuleSource> for LinkSource {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Con,
            RuleSource::Bible => Self::Gates,
        }
    }
}

impl OverlaidGalaxy {
    /// This galaxy with the gates its lanes are built from: the
    /// hypergates among `sites`, and the gates they link to, each found in
    /// the systems `listed` says list it (see the `gate_lanes` module).
    /// The lanes and the map are built again.
    #[must_use]
    pub fn with_gates(mut self, sites: &[GateSite], listed: &[ListedStellar]) -> Self {
        let hypergates: Vec<&GateSite> = sites
            .iter()
            .filter(|site| GateKind::of(site.flags2) == Some(GateKind::Hypergate))
            .collect();
        self.gates = sites
            .iter()
            .filter(|site| {
                hypergates
                    .iter()
                    .any(|gate| gate.id == site.id || gate.links.contains(&Some(site.id)))
            })
            .copied()
            .collect();
        self.listed = listed.to_vec();
        self.rebuild();
        self
    }

    /// Where the links between systems come from.
    #[must_use]
    pub fn link_source(&self) -> LinkSource {
        self.link_source
    }

    /// Takes the links between systems from `source` instead, rebuilding
    /// the map when it differs, and says whether it did.
    pub fn set_link_source(&mut self, source: LinkSource) -> bool {
        if source == self.link_source {
            return false;
        }
        self.link_source = source;
        self.rebuild();
        true
    }

    /// The lanes the galaxy's hypergates make on it as overlaid, under
    /// either link source.
    #[must_use]
    pub fn lanes(&self) -> &GateLanes {
        &self.lanes
    }

    /// The links `system` lists, as the slot readers walk them: by
    /// [`LinkSource::Con`] its raw Con slots ([`OverlaidGalaxy::con_slots`]),
    /// which each reader resolves and filters itself; by
    /// [`LinkSource::Gates`] the systems its lanes reach on the map, each
    /// once, already standing for their places
    /// ([`StarMap::listed_links`]). Empty for a system not in the catalog,
    /// and by [`LinkSource::Gates`] for a hidden one.
    #[must_use]
    pub fn link_slots(&self, system: SystemId) -> &[SystemId] {
        match self.link_source {
            LinkSource::Con => self.con_slots(system),
            LinkSource::Gates => self.map().listed_links(system),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::{GateStability, StellarId};
    use crate::exploration::known_from;
    use crate::gate::{HYPERGATE, WORMHOLE};
    use crate::geometry::Vec2;
    use crate::story::GalaxyOverlay;
    use nova_sim_catalog::fixture::{list_also, star};

    fn ids(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    fn pair(a: i16, b: i16) -> (SystemId, SystemId) {
        (SystemId(a), SystemId(b))
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
            stability: GateStability::Stable,
        }
    }

    /// Systems 1, 2 and 3, 1 listing 3 by Con; gates 101 (in 1) ↔ 201 (in
    /// 2), 202 (in 2) → 301 (in 3), and a wormhole 102 (in 1) to 301.
    fn gated(overlay: GalaxyOverlay) -> OverlaidGalaxy {
        let systems = vec![
            star(1, (0.0, 0.0), &[3]),
            star(2, (10.0, 0.0), &[]),
            star(3, (20.0, 0.0), &[]),
        ];
        let sites = [
            gate(101, HYPERGATE, &[201]),
            gate(102, WORMHOLE, &[301]),
            gate(201, HYPERGATE, &[101]),
            gate(202, HYPERGATE, &[301]),
            gate(301, 0, &[]),
        ];
        let listed = listed_by_hundreds(&sites);
        OverlaidGalaxy::new(systems, overlay).with_gates(&sites, &listed)
    }

    /// Each of `sites` listed by the system its ID's hundreds give.
    fn listed_by_hundreds(sites: &[GateSite]) -> Vec<ListedStellar> {
        let mut listed = Vec::new();
        list_also(
            &mut listed,
            sites
                .iter()
                .map(|site| (SystemId(site.id.0 / 100), site.id)),
        );
        listed
    }

    fn by(source: LinkSource) -> OverlaidGalaxy {
        let mut galaxy = gated(GalaxyOverlay::default());
        galaxy.set_link_source(source);
        galaxy
    }

    #[test]
    fn the_key_reads_con_by_the_engine_and_gates_by_the_other_reading() {
        assert_eq!(LinkSource::from(RuleSource::Engine), LinkSource::Con);
        assert_eq!(LinkSource::from(RuleSource::Bible), LinkSource::Gates);
        assert_eq!(LinkSource::default(), LinkSource::Con);
        assert_eq!(
            gated(GalaxyOverlay::default()).link_source(),
            LinkSource::Con
        );
    }

    #[test]
    fn by_con_the_map_and_slots_are_the_con_links_and_the_lanes_are_still_built() {
        let galaxy = by(LinkSource::Con);
        assert_eq!(galaxy.map().links(), [pair(1, 3)]);
        assert_eq!(galaxy.link_slots(SystemId(1)), ids(&[3]));
        assert_eq!(galaxy.link_slots(SystemId(2)), ids(&[]));
        assert_eq!(galaxy.lanes().listed(SystemId(2)), ids(&[1, 3]));
    }

    #[test]
    fn by_gates_the_map_and_slots_are_the_lanes_and_the_con_slots_stay_raw() {
        let galaxy = by(LinkSource::Gates);
        assert_eq!(galaxy.map().links(), [pair(1, 2), pair(2, 3)]);
        assert_eq!(galaxy.link_slots(SystemId(1)), ids(&[2]), "no wormhole");
        assert_eq!(galaxy.link_slots(SystemId(2)), ids(&[1, 3]));
        assert_eq!(galaxy.link_slots(SystemId(9)), ids(&[]));
        assert_eq!(galaxy.con_slots(SystemId(1)), ids(&[3]));
        assert_eq!(galaxy.map().con_slots(SystemId(1)), ids(&[3]));
        assert_eq!(galaxy.con_slots(SystemId(2)), ids(&[]));
    }

    #[test]
    fn a_lone_hypergate_linking_a_stellar_that_is_no_gate_makes_its_lane() {
        let systems = vec![star(1, (0.0, 0.0), &[]), star(3, (20.0, 0.0), &[])];
        let sites = [gate(101, HYPERGATE, &[301]), gate(301, 0, &[])];
        let mut galaxy = OverlaidGalaxy::new(systems, GalaxyOverlay::default())
            .with_gates(&sites, &listed_by_hundreds(&sites));
        galaxy.set_link_source(LinkSource::Gates);
        assert_eq!(galaxy.link_slots(SystemId(1)), ids(&[3]));
    }

    #[test]
    fn hiding_a_system_drops_its_lanes() {
        let mut galaxy = by(LinkSource::Gates);
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(3), Some(false));
        assert!(galaxy.set_overlay(overlay));
        assert_eq!(galaxy.link_slots(SystemId(2)), ids(&[1]));
        assert_eq!(galaxy.lanes().listed(SystemId(2)), ids(&[1]));
        assert_eq!(galaxy.map().links(), [pair(1, 2)]);
    }

    #[test]
    fn the_link_source_rebuilds_only_when_it_changes() {
        let mut galaxy = by(LinkSource::Con);
        assert!(!galaxy.set_link_source(LinkSource::Con));
        assert!(galaxy.set_link_source(LinkSource::Gates));
        assert_eq!(galaxy.link_source(), LinkSource::Gates);
        assert!(!galaxy.set_link_source(LinkSource::Gates));
        assert_eq!(galaxy.map().links(), [pair(1, 2), pair(2, 3)]);
        assert!(galaxy.set_link_source(LinkSource::Con));
        assert_eq!(galaxy.map().links(), [pair(1, 3)]);
    }

    #[test]
    fn the_known_pass_marks_the_lane_neighbours_by_gates_and_the_explored_alone_by_con() {
        let systems = vec![star(1, (0.0, 0.0), &[]), star(2, (10.0, 0.0), &[])];
        let sites = [gate(101, HYPERGATE, &[201]), gate(201, HYPERGATE, &[])];
        let listed = listed_by_hundreds(&sites);
        let mut galaxy =
            OverlaidGalaxy::new(systems, GalaxyOverlay::default()).with_gates(&sites, &listed);
        let explored = BTreeSet::from([SystemId(1)]);
        let known = |galaxy: &OverlaidGalaxy| -> Vec<SystemId> {
            known_from(galaxy, &explored).into_iter().collect()
        };
        assert_eq!(known(&galaxy), ids(&[1]));
        galaxy.set_link_source(LinkSource::Gates);
        assert_eq!(known(&galaxy), ids(&[1, 2]));
    }
}
