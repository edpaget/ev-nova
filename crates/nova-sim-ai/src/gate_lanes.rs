//! Gate lanes: the links a galaxy's hypergates make between its systems,
//! for a galaxy whose systems are joined by gates rather than by `sÿst`
//! Con links, as the new game's are (its jump points are one-link
//! hypergates, mapped in `nova_data::new_game`).
//!
//! The original has no such graph: its galaxy map, clicks, cue, nav cycles,
//! known pass and new pilot read the Con slots alone (see
//! [`hyperspace`](crate::hyperspace)), and a hypergate's links show only
//! on the hypergate map, as one gate's arrows ([`gate`](crate::gate)). By
//! `RuleKey::GateLanes`' other reading every
//! one of those readers reads these lanes instead, through the star map
//! built from them (`StarMap::with_lanes`); no Con link is made, and no
//! hyperspace jump with it.
//!
//! **A lane** is directional, (from, to): each hypergate in `from` whose
//! link resolves to `to` serves it ([`GateLanes::gates`]). Wormholes make
//! no lane. A link resolves as [`hypergate_offers`](crate::gate::hypergate_offers)
//! resolves it: to a gate some system lists, then that gate's system,
//! then the visible system standing for it ([`Places::stand_in`]); a link
//! to a gate no system lists, one with no visible system at its place, and
//! one to the gate's own system make no lane.
//!
//! **Where a gate is.** A gate's own system, and the system its link's
//! gate is in, are each resolved by the resolver [`GateLanes::new`] is
//! given, then by place. The galaxy gives the owner, else
//! `_FindSystemFromStellar`: the owner-first readers' resolution
//! (`nova_sim::story::OverlaidGalaxy::system_of`) without the holder's
//! rescue, which reads the player's system (@0x32c58-0x32cb3). So the lanes
//! change only when the star map does, and are built once with it. A stock
//! gate that only hidden or shadowed systems list, held by the player's
//! system, is offered by the hypergate map but makes no lane; the new game
//! lists every gate in one visible system, so it never meets this.
//!
//! **Stability**, per direction ([`GateLanes::stability`]): each gate
//! serving the lane works as well as the [`worse`](GateStability::worse)
//! of its own and its link's gate's stability, and of parallel gates the
//! best wins, as the traveller can take it. A lane is one-way
//! ([`GateLanes::one_way`]) when no lane leads back.

use std::collections::BTreeMap;

use crate::catalog::{GateSite, GateStability, StellarId, SystemId};
use crate::gate::{GateKind, exits};
use crate::hyperspace::Places;

/// One lane: the gates serving it and how well it works.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Lane {
    /// The gates serving it, by ID, each once.
    gates: Vec<StellarId>,
    /// The best of its gates' stabilities.
    stability: GateStability,
}

/// The lanes a galaxy's hypergates make (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GateLanes {
    /// Each lane, by (from, to).
    lanes: BTreeMap<(SystemId, SystemId), Lane>,
    /// The systems each system's lanes reach, by gate ID then slot order,
    /// each once.
    listed: BTreeMap<SystemId, Vec<SystemId>>,
}

impl GateLanes {
    /// The lanes the hypergates among `sites` make, each gate's system,
    /// and each link's gate's, given by `system_of` and resolved by place
    /// on `places` (see the module docs).
    #[must_use]
    pub fn new(
        sites: &[GateSite],
        system_of: impl Fn(StellarId) -> Option<SystemId>,
        places: &Places,
    ) -> Self {
        let standing = |gate: StellarId| system_of(gate).and_then(|system| places.stand_in(system));
        let mut gates: Vec<&GateSite> = sites
            .iter()
            .filter(|site| GateKind::of(site.flags2) == Some(GateKind::Hypergate))
            .collect();
        gates.sort_by_key(|site| site.id);
        let mut lanes = Self::default();
        for gate in gates {
            let Some(from) = standing(gate.id) else {
                continue;
            };
            for exit in exits(&gate.links, sites) {
                let Some(to) = standing(exit.id).filter(|&to| to != from) else {
                    continue;
                };
                lanes.serve(from, to, gate.id, gate.stability.worse(exit.stability));
            }
        }
        lanes
    }

    /// Notes `gate` serving the lane (`from`, `to`) at `stability`.
    fn serve(&mut self, from: SystemId, to: SystemId, gate: StellarId, stability: GateStability) {
        let listed = self.listed.entry(from).or_default();
        if !listed.contains(&to) {
            listed.push(to);
        }
        let lane = self.lanes.entry((from, to)).or_insert(Lane {
            gates: Vec::new(),
            stability,
        });
        if !lane.gates.contains(&gate) {
            lane.gates.push(gate);
        }
        lane.stability = lane.stability.min(stability);
    }

    /// The systems `from`'s lanes reach, by gate ID then slot order, each
    /// once: empty for a system with none.
    #[must_use]
    pub fn listed(&self, from: SystemId) -> &[SystemId] {
        self.listed.get(&from).map_or(&[], Vec::as_slice)
    }

    /// The gates serving the lane from `from` to `to`, by ID: empty when
    /// there is no such lane.
    #[must_use]
    pub fn gates(&self, from: SystemId, to: SystemId) -> &[StellarId] {
        self.lanes
            .get(&(from, to))
            .map_or(&[], |lane| lane.gates.as_slice())
    }

    /// How well the lane from `from` to `to` works: the best of its gates'
    /// (see the module docs), or `None` when there is no such lane.
    #[must_use]
    pub fn stability(&self, from: SystemId, to: SystemId) -> Option<GateStability> {
        self.lanes.get(&(from, to)).map(|lane| lane.stability)
    }

    /// Whether the lane from `from` to `to` is one-way: it exists, and no
    /// lane leads back.
    #[must_use]
    pub fn one_way(&self, from: SystemId, to: SystemId) -> bool {
        self.lanes.contains_key(&(from, to)) && !self.lanes.contains_key(&(to, from))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::StarSystem;
    use crate::gate::{HYPERGATE, WORMHOLE};
    use crate::geometry::Vec2;
    use crate::hyperspace::CoLocated;
    use GateStability::{Dead, Stable, Unstable};
    use nova_sim_catalog::fixture::star;

    fn ids(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    fn stellars(list: &[i16]) -> Vec<StellarId> {
        list.iter().copied().map(StellarId).collect()
    }

    /// Hypergate `id` linking `links`, at `stability`.
    fn gate(id: i16, links: &[i16], stability: GateStability) -> GateSite {
        let mut slots = [None; 8];
        for (slot, &link) in slots.iter_mut().zip(links) {
            *slot = Some(StellarId(link));
        }
        GateSite {
            id: StellarId(id),
            position: Vec2::ZERO,
            flags2: HYPERGATE,
            links: slots,
            exit_angle: -1,
            stability,
        }
    }

    /// Systems 1-4, each at its own place, 5 at 1's.
    fn systems() -> Vec<StarSystem> {
        vec![
            star(1, (0.0, 0.0), &[]),
            star(2, (10.0, 0.0), &[]),
            star(3, (20.0, 0.0), &[]),
            star(4, (30.0, 0.0), &[]),
            star(5, (0.0, 0.0), &[]),
        ]
    }

    /// The places of [`systems`], with `hidden` hidden.
    fn places(hidden: &[i16]) -> Places {
        let hidden: BTreeSet<SystemId> = ids(hidden).into_iter().collect();
        Places::new(&systems(), &hidden, CoLocated::Lowest)
    }

    /// Each gate in the system its ID's hundreds give: gate 101 in 1.
    #[allow(clippy::unnecessary_wraps)] // A resolver, as `GateLanes::new` takes it.
    fn by_hundreds(gate: StellarId) -> Option<SystemId> {
        Some(SystemId(gate.0 / 100))
    }

    fn lanes(sites: &[GateSite]) -> GateLanes {
        GateLanes::new(sites, by_hundreds, &places(&[]))
    }

    #[test]
    fn a_two_way_pair_gives_both_directions_and_a_one_way_gate_only_its_own() {
        let lanes = lanes(&[
            gate(101, &[201], Stable),
            gate(201, &[101], Stable),
            gate(102, &[301], Stable),
            gate(301, &[202], Stable),
            gate(202, &[], Stable),
        ]);
        assert_eq!(lanes.listed(SystemId(1)), ids(&[2, 3]));
        assert_eq!(lanes.listed(SystemId(2)), ids(&[1]));
        assert_eq!(lanes.listed(SystemId(3)), ids(&[2]));
        assert_eq!(lanes.gates(SystemId(1), SystemId(3)), stellars(&[102]));
        assert_eq!(lanes.gates(SystemId(3), SystemId(1)), stellars(&[]));
    }

    #[test]
    fn a_wormhole_makes_no_lane() {
        let wormhole = GateSite {
            flags2: WORMHOLE,
            ..gate(101, &[201], Stable)
        };
        let lanes = lanes(&[wormhole, gate(201, &[], Stable)]);
        assert_eq!(lanes.listed(SystemId(1)), ids(&[]));
        assert_eq!(lanes.stability(SystemId(1), SystemId(2)), None);
    }

    #[test]
    fn a_link_to_a_gate_no_system_lists_makes_no_lane() {
        let lanes = lanes(&[gate(101, &[201, 999], Stable)]);
        assert_eq!(lanes.listed(SystemId(1)), ids(&[]));
    }

    #[test]
    fn a_link_with_no_visible_system_at_its_place_makes_no_lane() {
        let sites = [
            gate(101, &[201, 301], Stable),
            gate(201, &[], Stable),
            gate(301, &[], Stable),
        ];
        let lanes = GateLanes::new(&sites, by_hundreds, &places(&[2]));
        assert_eq!(lanes.listed(SystemId(1)), ids(&[3]));
    }

    #[test]
    fn a_link_to_a_hidden_system_reaches_the_one_standing_at_its_place() {
        let sites = [gate(201, &[501], Stable), gate(501, &[], Stable)];
        let lanes = GateLanes::new(&sites, by_hundreds, &places(&[5]));
        assert_eq!(lanes.listed(SystemId(2)), ids(&[1]));
    }

    #[test]
    fn a_gate_with_no_system_makes_no_lane() {
        let sites = [gate(101, &[201], Stable), gate(201, &[], Stable)];
        let lanes = GateLanes::new(&sites, |_| None, &places(&[]));
        assert_eq!(lanes.listed(SystemId(1)), ids(&[]));
    }

    #[test]
    fn a_link_to_the_gates_own_system_makes_no_lane() {
        let lanes = lanes(&[gate(101, &[102], Stable), gate(102, &[], Stable)]);
        assert_eq!(lanes.listed(SystemId(1)), ids(&[]));
        let lanes = GateLanes::new(
            &[gate(101, &[501], Stable), gate(501, &[], Stable)],
            by_hundreds,
            &places(&[]),
        );
        assert_eq!(
            lanes.listed(SystemId(1)),
            ids(&[]),
            "5 stands for 1 at its place"
        );
    }

    #[test]
    fn listed_is_in_gate_id_then_slot_order_each_once() {
        let lanes = lanes(&[
            gate(103, &[301, 201], Stable),
            gate(101, &[401, 301], Stable),
            gate(102, &[201], Stable),
            gate(201, &[], Stable),
            gate(301, &[], Stable),
            gate(401, &[], Stable),
        ]);
        assert_eq!(lanes.listed(SystemId(1)), ids(&[4, 3, 2]));
        assert_eq!(lanes.listed(SystemId(9)), ids(&[]));
    }

    #[test]
    fn gates_lists_every_gate_serving_the_lane_by_id_each_once() {
        let lanes = lanes(&[
            gate(103, &[201], Stable),
            gate(101, &[201, 202], Stable),
            gate(102, &[301], Stable),
            gate(201, &[], Stable),
            gate(202, &[], Stable),
            gate(301, &[], Stable),
        ]);
        assert_eq!(lanes.gates(SystemId(1), SystemId(2)), stellars(&[101, 103]));
        assert_eq!(lanes.gates(SystemId(1), SystemId(3)), stellars(&[102]));
        assert_eq!(lanes.gates(SystemId(1), SystemId(4)), stellars(&[]));
    }

    #[test]
    fn a_gates_stability_is_the_worse_of_its_two_ends() {
        for (own, far, lane) in [
            (Stable, Stable, Stable),
            (Stable, Unstable, Unstable),
            (Dead, Unstable, Dead),
            (Unstable, Stable, Unstable),
        ] {
            let lanes = lanes(&[gate(101, &[201], own), gate(201, &[], far)]);
            assert_eq!(
                lanes.stability(SystemId(1), SystemId(2)),
                Some(lane),
                "{own:?} {far:?}"
            );
        }
    }

    #[test]
    fn a_lane_of_parallel_gates_is_as_good_as_the_best() {
        for (first, second, lane) in [
            (Dead, Unstable, Unstable),
            (Unstable, Dead, Unstable),
            (Dead, Stable, Stable),
            (Dead, Dead, Dead),
        ] {
            let lanes = lanes(&[
                gate(101, &[201], first),
                gate(102, &[202], second),
                gate(201, &[], Stable),
                gate(202, &[], Stable),
            ]);
            assert_eq!(
                lanes.stability(SystemId(1), SystemId(2)),
                Some(lane),
                "{first:?} {second:?}"
            );
        }
    }

    /// The nova-data new-game fixture's gates: drift 1, hearth 2, verge 3.
    fn fixture_shaped() -> GateLanes {
        lanes(&[
            // drift: hearth-road, verge-road, back-door.
            gate(101, &[201], Stable),
            gate(102, &[303], Dead),
            gate(103, &[302], Unstable),
            // hearth: drift-road, verge-road, side-road.
            gate(201, &[101], Stable),
            gate(202, &[301], Stable),
            gate(203, &[302], Stable),
            // verge: hearth-road, side-road, drift-road.
            gate(301, &[202], Unstable),
            gate(302, &[203], Stable),
            gate(303, &[102], Dead),
        ])
    }

    #[test]
    fn the_fixtures_two_ways_between_drift_and_verge_differ() {
        let lanes = fixture_shaped();
        let (drift, hearth, verge) = (SystemId(1), SystemId(2), SystemId(3));
        assert_eq!(
            lanes.stability(drift, verge),
            Some(Unstable),
            "the back door"
        );
        assert_eq!(lanes.stability(verge, drift), Some(Dead));
        assert_eq!(lanes.stability(drift, hearth), Some(Stable));
        assert_eq!(lanes.stability(hearth, drift), Some(Stable));
        assert_eq!(
            lanes.stability(hearth, verge),
            Some(Stable),
            "the side road"
        );
        assert_eq!(lanes.stability(verge, hearth), Some(Stable));
        assert_eq!(lanes.gates(drift, verge), stellars(&[102, 103]));
    }

    #[test]
    fn a_lane_is_one_way_only_with_no_lane_back() {
        let lanes = lanes(&[
            gate(101, &[201], Stable),
            gate(201, &[101], Stable),
            gate(102, &[301], Stable),
            gate(301, &[], Stable),
        ]);
        assert!(!lanes.one_way(SystemId(1), SystemId(2)));
        assert!(!lanes.one_way(SystemId(2), SystemId(1)));
        assert!(lanes.one_way(SystemId(1), SystemId(3)));
        assert!(!lanes.one_way(SystemId(3), SystemId(1)), "no lane at all");
        assert!(
            !fixture_shaped().one_way(SystemId(1), SystemId(3)),
            "the dead pair leads back"
        );
    }
}
