//! Gate lanes: the links a galaxy's hypergates make between its systems,
//! for a galaxy whose systems are joined by gates rather than by `sÿst`
//! Con links, as the new game's are (its jump points are one-link
//! hypergates, mapped in `nova_data::new_game`).
//!
//! The original has no such graph: its galaxy map, clicks, cue, nav cycles,
//! known pass and new pilot read the Con slots alone (see
//! [`hyperspace`](crate::hyperspace)), and a hypergate's links show only
//! on the hypergate map, as one gate's arrows ([`gate`](crate::gate)). By
//! [`RuleKey::GateLanes`](crate::RuleKey::GateLanes)' other reading every
//! one of those readers reads these lanes instead, through the star map
//! built from them ([`StarMap::with_lanes`](crate::hyperspace::StarMap::with_lanes));
//! no Con link is made, and no
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
//! of its own and its link's gate's stability, each as the resolver
//! [`GateLanes::new`] is given says (the galaxy gives the jump points'
//! as play leaves them, `nova_sim::story::OverlaidGalaxy::gate_stability`),
//! and of parallel gates the best wins, as the traveller can take it. A
//! lane is one-way ([`GateLanes::one_way`]) when no lane leads back.
//!
//! **Open and closed.** A gate serving a lane is open when its
//! stability [is open](GateStability::is_open), and a lane is open when
//! one of its gates is. Every link reader reads open lanes alone:
//! [`GateLanes::listed`], so the star map, and [`GateLanes::gates`],
//! give only open lanes and gates. [`GateLanes::stability`] and
//! [`GateLanes::one_way`] read every lane, and
//! [`GateLanes::closed`] gives the closed ones, for the map's looks. The
//! lanes change with the points' stabilities too: the galaxy builds them
//! again when play changes one.
//!
//! With the `fixture` feature, `GateLanes::fixture` builds lanes as
//! given, for the tests of the crates above (the story's news spreads
//! over them).

use std::collections::BTreeMap;

use crate::catalog::{GateSite, GateStability, StellarId, SystemId};
use crate::gate::{GateKind, exits};
use crate::hyperspace::Places;

/// One lane: the gates serving it and how well each works.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Lane {
    /// The gates serving it, by ID, each once, with the
    /// [`worse`](GateStability::worse) of its two ends.
    gates: Vec<(StellarId, GateStability)>,
    /// Those of [`gates`](Self::gates) that are
    /// [open](GateStability::is_open), by ID.
    open: Vec<StellarId>,
}

impl Lane {
    /// The best of its gates' stabilities.
    fn stability(&self) -> GateStability {
        self.gates
            .iter()
            .map(|&(_, stability)| stability)
            .min()
            .unwrap_or_default()
    }
}

/// The lanes a galaxy's hypergates make (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GateLanes {
    /// Each lane, open or closed, by (from, to).
    lanes: BTreeMap<(SystemId, SystemId), Lane>,
    /// The systems each system's lanes reach, open or closed, by gate ID
    /// then slot order, each once.
    reached: BTreeMap<SystemId, Vec<SystemId>>,
    /// Those of [`reached`](Self::reached) whose lane has an open gate,
    /// in the same order: what [`GateLanes::listed`] gives.
    listed: BTreeMap<SystemId, Vec<SystemId>>,
}

impl GateLanes {
    /// The lanes the hypergates among `sites` make, each gate's system,
    /// and each link's gate's, given by `system_of` and resolved by place
    /// on `places`, each gate's own stability by `stability_of` (see the
    /// module docs).
    #[must_use]
    pub fn new(
        sites: &[GateSite],
        system_of: impl Fn(StellarId) -> Option<SystemId>,
        stability_of: impl Fn(StellarId) -> GateStability,
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
                let stability = stability_of(gate.id).worse(stability_of(exit.id));
                lanes.serve(from, to, gate.id, stability);
            }
        }
        lanes.list_open();
        lanes
    }

    /// Notes `gate` serving the lane (`from`, `to`) at `stability`: a
    /// gate serving it twice keeps the better.
    fn serve(&mut self, from: SystemId, to: SystemId, gate: StellarId, stability: GateStability) {
        let reached = self.reached.entry(from).or_default();
        if !reached.contains(&to) {
            reached.push(to);
        }
        let lane = self.lanes.entry((from, to)).or_insert(Lane {
            gates: Vec::new(),
            open: Vec::new(),
        });
        match lane.gates.iter_mut().find(|(id, _)| *id == gate) {
            Some((_, kept)) => *kept = (*kept).min(stability),
            None => lane.gates.push((gate, stability)),
        }
    }

    /// Works out each lane's open gates, and each system's open lanes,
    /// once every gate is served.
    fn list_open(&mut self) {
        for lane in self.lanes.values_mut() {
            lane.gates.sort_by_key(|&(id, _)| id);
            lane.open = lane
                .gates
                .iter()
                .filter(|(_, stability)| stability.is_open())
                .map(|&(id, _)| id)
                .collect();
        }
        self.listed = self
            .reached
            .iter()
            .map(|(&from, reached)| {
                let open = reached
                    .iter()
                    .copied()
                    .filter(|&to| !self.gates(from, to).is_empty())
                    .collect();
                (from, open)
            })
            .collect();
    }

    /// The systems `from`'s open lanes reach, by gate ID then slot order,
    /// each once: empty for a system with none. A lane whose every gate
    /// is closed is left out ([`GateLanes::closed`]).
    #[must_use]
    pub fn listed(&self, from: SystemId) -> &[SystemId] {
        self.listed.get(&from).map_or(&[], Vec::as_slice)
    }

    /// The open gates serving the lane from `from` to `to`, by ID: empty
    /// when there is no such lane or every gate serving it is closed.
    #[must_use]
    pub fn gates(&self, from: SystemId, to: SystemId) -> &[StellarId] {
        self.lanes
            .get(&(from, to))
            .map_or(&[], |lane| lane.open.as_slice())
    }

    /// The systems `from`'s closed lanes reach, by gate ID then slot
    /// order, each once: each lane from `from` that exists but has no
    /// open gate, as the map draws it.
    #[must_use]
    pub fn closed(&self, from: SystemId) -> Vec<SystemId> {
        self.reached.get(&from).map_or_else(Vec::new, |reached| {
            reached
                .iter()
                .copied()
                .filter(|&to| self.gates(from, to).is_empty())
                .collect()
        })
    }

    /// How well the lane from `from` to `to` works, open or closed: the
    /// best of its gates' (see the module docs), or `None` when there is
    /// no such lane.
    #[must_use]
    pub fn stability(&self, from: SystemId, to: SystemId) -> Option<GateStability> {
        self.lanes.get(&(from, to)).map(Lane::stability)
    }

    /// Whether the lane from `from` to `to` is one-way: it exists, and no
    /// lane leads back, open or closed.
    #[must_use]
    pub fn one_way(&self, from: SystemId, to: SystemId) -> bool {
        self.lanes.contains_key(&(from, to)) && !self.lanes.contains_key(&(to, from))
    }

    /// Lanes as given, for other crates' tests: each `(from, to,
    /// stability)` a lane, in order, served by one gate numbered by its
    /// place in `lanes` (from 0). A lane given twice is served by two
    /// gates, so it keeps the better stability, as parallel gates do. A
    /// lane given only unstable or dead is closed.
    #[cfg(any(test, feature = "fixture"))]
    #[must_use]
    pub fn fixture(lanes: &[(i16, i16, GateStability)]) -> Self {
        let mut built = Self::default();
        for (gate, &(from, to, stability)) in (0..).zip(lanes) {
            built.serve(SystemId(from), SystemId(to), StellarId(gate), stability);
        }
        built.list_open();
        built
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

    /// A gate's site and its own stability, as the galaxy resolves it.
    type Gate = (GateSite, GateStability);

    /// Hypergate `id` linking `links`, at `stability`.
    fn gate(id: i16, links: &[i16], stability: GateStability) -> Gate {
        let mut slots = [None; 8];
        for (slot, &link) in slots.iter_mut().zip(links) {
            *slot = Some(StellarId(link));
        }
        let site = GateSite {
            id: StellarId(id),
            position: Vec2::ZERO,
            flags2: HYPERGATE,
            links: slots,
            exit_angle: -1,
        };
        (site, stability)
    }

    /// The lanes `gates` make, each gate's system by `system_of`, on
    /// `places`, each at its own stability: [`GateLanes::new`] over their
    /// sites, stable for a gate not among them.
    fn built(
        gates: &[Gate],
        system_of: impl Fn(StellarId) -> Option<SystemId>,
        places: &Places,
    ) -> GateLanes {
        let sites: Vec<GateSite> = gates.iter().map(|&(site, _)| site).collect();
        let stability_of = |id: StellarId| {
            gates
                .iter()
                .find(|(site, _)| site.id == id)
                .map_or(Stable, |&(_, stability)| stability)
        };
        GateLanes::new(&sites, system_of, stability_of, places)
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

    fn lanes(gates: &[Gate]) -> GateLanes {
        built(gates, by_hundreds, &places(&[]))
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
            ..gate(101, &[201], Stable).0
        };
        let lanes = lanes(&[(wormhole, Stable), gate(201, &[], Stable)]);
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
        let lanes = built(&sites, by_hundreds, &places(&[2]));
        assert_eq!(lanes.listed(SystemId(1)), ids(&[3]));
    }

    #[test]
    fn a_link_to_a_hidden_system_reaches_the_one_standing_at_its_place() {
        let sites = [gate(201, &[501], Stable), gate(501, &[], Stable)];
        let lanes = built(&sites, by_hundreds, &places(&[5]));
        assert_eq!(lanes.listed(SystemId(2)), ids(&[1]));
    }

    #[test]
    fn a_gate_with_no_system_makes_no_lane() {
        let sites = [gate(101, &[201], Stable), gate(201, &[], Stable)];
        let lanes = built(&sites, |_| None, &places(&[]));
        assert_eq!(lanes.listed(SystemId(1)), ids(&[]));
    }

    #[test]
    fn a_link_to_the_gates_own_system_makes_no_lane() {
        let lanes = lanes(&[gate(101, &[102], Stable), gate(102, &[], Stable)]);
        assert_eq!(lanes.listed(SystemId(1)), ids(&[]));
        let lanes = built(
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

    #[test]
    fn a_gate_whose_either_end_is_unstable_or_dead_is_closed_but_its_lane_kept() {
        for (own, far) in [
            (Stable, Unstable),
            (Stable, Dead),
            (Unstable, Stable),
            (Dead, Stable),
        ] {
            let lanes = lanes(&[gate(101, &[201], own), gate(201, &[], far)]);
            let (from, to) = (SystemId(1), SystemId(2));
            assert_eq!(lanes.listed(from), ids(&[]), "{own:?} {far:?}");
            assert_eq!(lanes.gates(from, to), stellars(&[]), "{own:?} {far:?}");
            assert_eq!(
                lanes.stability(from, to),
                Some(own.worse(far)),
                "{own:?} {far:?}"
            );
            assert_eq!(lanes.closed(from), ids(&[2]), "{own:?} {far:?}");
            assert!(lanes.one_way(from, to), "{own:?} {far:?}");
        }
    }

    #[test]
    fn an_open_lane_is_listed_and_not_closed() {
        let lanes = lanes(&[gate(101, &[201], Stable), gate(201, &[], Stable)]);
        assert_eq!(lanes.listed(SystemId(1)), ids(&[2]));
        assert_eq!(lanes.gates(SystemId(1), SystemId(2)), stellars(&[101]));
        assert_eq!(lanes.closed(SystemId(1)), ids(&[]));
        assert_eq!(lanes.closed(SystemId(9)), ids(&[]), "no lane at all");
    }

    #[test]
    fn of_parallel_gates_only_the_open_ones_serve_and_the_best_is_the_lanes() {
        let lanes = lanes(&[
            gate(101, &[201], Dead),
            gate(102, &[202], Stable),
            gate(103, &[201], Unstable),
            gate(104, &[301], Stable),
            gate(201, &[], Stable),
            gate(202, &[], Stable),
            gate(301, &[], Stable),
        ]);
        let (from, to) = (SystemId(1), SystemId(2));
        assert_eq!(lanes.listed(from), ids(&[2, 3]));
        assert_eq!(lanes.gates(from, to), stellars(&[102]));
        assert_eq!(lanes.stability(from, to), Some(Stable));
        assert_eq!(lanes.closed(from), ids(&[]));
    }

    #[test]
    fn closed_and_open_lanes_keep_the_order_of_their_gates() {
        let lanes = lanes(&[
            gate(101, &[401], Dead),
            gate(102, &[201], Stable),
            gate(103, &[301], Unstable),
            gate(104, &[501], Stable),
            gate(201, &[], Stable),
            gate(301, &[], Stable),
            gate(401, &[], Stable),
        ]);
        assert_eq!(lanes.listed(SystemId(1)), ids(&[2]));
        assert_eq!(lanes.closed(SystemId(1)), ids(&[4, 3]));
    }

    /// The nova-data new-game fixture's gates, at `stability` of each
    /// gate's own: drift 1, hearth 2, verge 3.
    fn fixture_gates(stability: impl Fn(GateStability) -> GateStability) -> Vec<Gate> {
        use GateStability::{Dead, Stable, Unstable};
        let at = |id, links: &[i16], own| gate(id, links, stability(own));
        vec![
            // drift: hearth-road, verge-road, back-door.
            at(101, &[201], Stable),
            at(102, &[303], Dead),
            at(103, &[302], Unstable),
            // hearth: drift-road, verge-road, side-road.
            at(201, &[101], Stable),
            at(202, &[301], Stable),
            at(203, &[302], Stable),
            // verge: hearth-road, side-road, drift-road.
            at(301, &[202], Unstable),
            at(302, &[203], Stable),
            at(303, &[102], Dead),
        ]
    }

    /// The fixture's lanes with every gate stable: each lane open.
    fn fixture_stable() -> GateLanes {
        lanes(&fixture_gates(|_| Stable))
    }

    /// The nova-data new-game fixture's gates: drift 1, hearth 2, verge 3.
    fn fixture_shaped() -> GateLanes {
        lanes(&fixture_gates(|own| own))
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
        assert_eq!(
            lanes.gates(drift, verge),
            stellars(&[]),
            "both ways from drift to verge closed"
        );
        assert_eq!(lanes.closed(drift), ids(&[3]));
        assert_eq!(lanes.closed(verge), ids(&[1]));
        assert_eq!(
            lanes.gates(hearth, verge),
            stellars(&[203]),
            "the side road alone"
        );
        assert_eq!(lanes.listed(hearth), ids(&[1, 3]));
        assert_eq!(lanes.closed(hearth), ids(&[]));
        assert_eq!(
            fixture_stable().gates(drift, verge),
            stellars(&[102, 103]),
            "every gate open"
        );
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

    // The star map of lanes (`StarMap::with_lanes`).

    use crate::hyperspace::{HyperlinkRule, RouteError, StarMap};

    /// [`systems`] with 1 listing 2 and 4 by Con, mapped by `lanes`.
    fn mapped(lanes: &GateLanes) -> StarMap {
        let mut systems = systems();
        systems[0].links = ids(&[2, 4, 4]);
        StarMap::with_lanes(systems, &BTreeSet::new(), CoLocated::Lowest, lanes)
    }

    /// [`systems`] with 1 listing 2 and 4 by Con, mapped by
    /// [`fixture_stable`]'s lanes.
    fn laned_map() -> StarMap {
        mapped(&fixture_stable())
    }

    #[test]
    fn a_closed_lane_is_no_link_on_the_map() {
        let map = mapped(&fixture_shaped());
        assert_eq!(map.listed_links(SystemId(1)), ids(&[2]));
        assert_eq!(map.listed_links(SystemId(3)), ids(&[2]));
        assert_eq!(
            map.links(),
            [(1, 2), (2, 3)].map(|(a, b)| (SystemId(a), SystemId(b)))
        );
        assert_eq!(
            map.route(SystemId(1), SystemId(3), HyperlinkRule::Engine),
            Ok(ids(&[2, 3])),
            "round by hearth"
        );
    }

    #[test]
    fn a_lane_maps_con_slots_stay_the_systems_own() {
        let map = laned_map();
        assert_eq!(map.con_slots(SystemId(1)), ids(&[2, 4, 4]));
        assert_eq!(map.usable_con_slots(SystemId(1)), ids(&[2, 4, 4]));
        assert_eq!(map.con_slots(SystemId(2)), ids(&[]));
        assert_eq!(map.usable_con_slots(SystemId(3)), ids(&[]));
    }

    #[test]
    fn a_lane_maps_links_are_the_lanes() {
        let map = laned_map();
        assert_eq!(map.listed_links(SystemId(1)), ids(&[2, 3]));
        assert_eq!(map.listed_links(SystemId(3)), ids(&[2, 1]));
        assert_eq!(map.neighbours(SystemId(1)), ids(&[2, 3]));
        assert_eq!(
            map.neighbours(SystemId(4)),
            ids(&[]),
            "no lane, its Con aside"
        );
        assert_eq!(
            map.links(),
            [(1, 2), (1, 3), (2, 3)].map(|(a, b)| (SystemId(a), SystemId(b))),
            "each pair once"
        );
        assert_eq!(map.jumps(SystemId(1), HyperlinkRule::Engine), ids(&[2, 3]));
        assert_eq!(
            map.ordered_jumps(SystemId(3), HyperlinkRule::Engine),
            ids(&[2, 1])
        );
    }

    #[test]
    fn the_engines_hyperlinks_never_go_back_along_a_one_way_lane_and_both_ways_do() {
        let lanes = lanes(&[
            gate(101, &[201], Stable),
            gate(201, &[], Stable),
            gate(301, &[202], Stable),
            gate(202, &[], Stable),
        ]);
        let map = StarMap::with_lanes(systems(), &BTreeSet::new(), CoLocated::Lowest, &lanes);
        assert_eq!(map.jumps(SystemId(2), HyperlinkRule::Engine), ids(&[]));
        assert_eq!(
            map.jumps(SystemId(2), HyperlinkRule::BothWays),
            ids(&[1, 3])
        );
        assert_eq!(
            map.route(SystemId(2), SystemId(1), HyperlinkRule::Engine),
            Err(RouteError::Unreachable)
        );
        assert_eq!(
            map.route(SystemId(2), SystemId(1), HyperlinkRule::BothWays),
            Ok(ids(&[1]))
        );
    }

    #[test]
    fn a_route_crosses_two_lanes() {
        let lanes = lanes(&[
            gate(101, &[201], Stable),
            gate(201, &[101], Stable),
            gate(202, &[301], Stable),
            gate(301, &[202], Stable),
        ]);
        let map = StarMap::with_lanes(systems(), &BTreeSet::new(), CoLocated::Lowest, &lanes);
        assert_eq!(
            map.route(SystemId(1), SystemId(3), HyperlinkRule::Engine),
            Ok(ids(&[2, 3]))
        );
    }

    #[test]
    fn a_fixture_s_lanes_answer_as_given() {
        let lanes = GateLanes::fixture(&[
            (1, 2, GateStability::Stable),
            (2, 1, GateStability::Dead),
            (1, 3, GateStability::Unstable),
            (1, 3, GateStability::Stable),
        ]);
        assert_eq!(lanes.listed(SystemId(1)), ids(&[2, 3]));
        assert_eq!(lanes.listed(SystemId(2)), ids(&[]), "dead: closed");
        assert_eq!(lanes.closed(SystemId(2)), ids(&[1]));
        assert_eq!(lanes.listed(SystemId(3)), ids(&[]));
        assert_eq!(
            lanes.stability(SystemId(1), SystemId(2)),
            Some(GateStability::Stable)
        );
        assert_eq!(
            lanes.stability(SystemId(2), SystemId(1)),
            Some(GateStability::Dead)
        );
        assert_eq!(
            lanes.stability(SystemId(1), SystemId(3)),
            Some(GateStability::Stable),
            "the better of two"
        );
        assert_eq!(lanes.stability(SystemId(3), SystemId(1)), None);
        assert!(lanes.one_way(SystemId(1), SystemId(3)));
        assert_eq!(
            lanes.gates(SystemId(1), SystemId(3)),
            stellars(&[3]),
            "the stable one alone"
        );
    }

    #[test]
    fn a_hidden_system_has_no_lane_on_the_map() {
        let lanes = fixture_shaped();
        let hidden = BTreeSet::from([SystemId(3)]);
        let map = StarMap::with_lanes(systems(), &hidden, CoLocated::Lowest, &lanes);
        assert_eq!(map.listed_links(SystemId(3)), ids(&[]));
        assert_eq!(map.position(SystemId(3)), None);
    }
}
