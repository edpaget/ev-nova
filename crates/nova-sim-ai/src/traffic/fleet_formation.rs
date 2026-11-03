//! The formations of NPC parents: each ship whose formation parent is an
//! NPC holds a slot in that NPC's formation
//! ([`Npc::fleet_slot`](super::npc::Npc::fleet_slot)).
//!
//! **The parent** (`_DoPlayGameWork` @0x4415c-0x441de): a ship of AI type
//! above 4 has its formation parent +0xc92e rewritten each pass, to its
//! swarm leader while it attacks with a target, else its leader
//! ([`swarm::formation_parent`]). The pass skips any other ship
//! (@0x440d8), which keeps the word it holds ([`Npc::form_parent`]), so
//! a ship whose AI type dropped holds a stale parent, for as long as it
//! stays. Stale or not, a ship counts in its parent's formation.
//!
//! **The mark.** The slot that +0xc92e names gets +0xc2, "is a formation
//! parent", which makes its dispatch work its followers' slots out; a stale
//! ship sets it only on the pass the check drops it (@0x441c7-0x441de,
//! [`Traffic::succeed_leaders`] hands those ships over). So a formation is
//! worked out only for a marked parent: its followers, stale or not, take
//! [`Npc::fleet_slot`] and the absolute place that gives them,
//! [`Npc::slot_place`], the original's +0x28/+0x2c. A follower of an
//! unmarked parent keeps both: the place stays where it was while the
//! parent moves. So does a follower of a word naming an empty slot: the
//! mark tests no +0xb8 (@0x441d1-0x441de), but the dispatch loop does
//! (@0x44275-0x4427c), so no calc runs for that slot, and the word stands
//! too. `_HandlePlayer` tests the player's mark the same way
//! (@0x6f1cb-0x6f1df), so a ship on the player takes its place only while
//! the player is marked, here from the slot the session numbered it. The
//! formations group the NPCs by their parents each tick, and only a ship
//! with no parent has its [`Npc::fleet_slot`] cleared.
//!
//! **The slots** (`_AICalcFormationPositions` @0x85040-0x851d9): the
//! parent's followers are every other ship whose parent it is, disabled
//! ones included (no condition test, @0x8508d-0x850ef), taking slots 2,
//! 3, … in slot (berth) order out of `ships`, the followers and the
//! parent. The spacing is the widest sprite's, the parent's included (64
//! for a parent with none), × 0.6, truncated and held to 24-60
//! ([`escort::spacing`]). A ship attacking with a swarm leader so counts
//! in that ship's formation and moves the slots of the swarm leader's own
//! escorts on. The player's formation is the session's (`Session::reform`).
//!
//! **When.** The think half of a tick works the numbers out
//! ([`Traffic::form_fleets`]) after the answers to the last tick's hits,
//! which the original lands before its marking pass, and before any
//! think, as the marking pass precedes the `_AIDispatch` loop. A slot's
//! place is the absolute word of the follower's berth (+0x28/+0x2c), which
//! the parent's own dispatch writes from where the parent then stands
//! (@0x8fcaf-0x8fcc3): before its own think, so before a drift of its own.
//! The same pass stands for those dispatches
//! ([`write_places`](super::formation_places)): a follower in a higher
//! berth than its parent reads this frame's place, one in a lower berth
//! the frame before's, and the followers of a parent whose dispatch never
//! gets there ([`dispatch_gate::reaches_formation_calc`]) keep the place
//! last written, though the parent moves. A disabled or breaking-up parent
//! still writes.
//!
//! **Arriving** (`_HyperSpawnFleet` @0x425e4-0x42602): once a fleet is
//! placed, `_AICalcFormationPositions(lead, 1)` writes each follower's
//! word and puts each escort that jumps in on its slot, position only
//! (`_AIMaintainFormation(ship, 1)` @0x84fc2-0x84fce), and leaves one out
//! of a gate (goal 0x15, @0x851a7-0x851af) at the gate's centre
//! ([`snap`]). The snap, and flying the slots, are the engine's reading of
//! [`RuleKey::NpcFormation`](crate::RuleKey::NpcFormation); by the other
//! the slots are still worked out, but nothing flies them, but for a
//! returning swarm fighter
//! ([`return_flight`](crate::traffic::return_flight)).
//!
//! None of these routines plays a sound or draws a random number.

use std::collections::BTreeSet;

use super::Traffic;
use super::formation_places::BerthPlaces;
use super::npc::{Mode, Npc, NpcId};
use super::pod_flight;
use crate::ai::dispatch_gate;
use crate::ai::swarm::{self, FormationParent};
use crate::escort::{self, FormationSlot};
use crate::flight::ShipState;
use crate::geometry::Vec2;

/// The word +0xc92e holds for the player: its slot.
const PLAYER_WORD: u32 = 0;

/// The slot of each ship whose formation parent is an NPC here and marked
/// (see the module docs), in that NPC's formation: `dropped` names the
/// ships the check dropped this pass ([`Traffic::succeed_leaders`]). A
/// ship whose parent is the player, or a berth with no ship, gets no
/// entry, nor does a ship that is its own parent, and nor does the
/// follower of an unmarked parent: no entry writes nothing, so each keeps
/// what it holds ([`Traffic::form_fleets`]).
#[must_use]
pub fn slots(npcs: &[Npc], dropped: &BTreeSet<NpcId>) -> Vec<(NpcId, FormationSlot)> {
    let parents = parents(npcs);
    let marked = marked(npcs, &parents, dropped);
    (0..npcs.len())
        .filter(|&at| marked[at])
        .flat_map(|at| formation(npcs, &parents, at))
        .collect()
}

/// Where among `npcs` each one's formation parent is: its leader by its
/// number, its swarm leader or stored word by its berth; none for the
/// player, or a parent not here.
fn parents(npcs: &[Npc]) -> Vec<Option<usize>> {
    npcs.iter()
        .map(|npc| match swarm::formation_parent(npc)? {
            FormationParent::Player => None,
            FormationParent::Leader(id) => npcs.iter().position(|other| other.id == id),
            FormationParent::Berth(berth) => npcs.iter().position(|other| other.berth == berth),
        })
        .collect()
}

/// Whether `npc` sets its parent's +0xc2 (@0x441c7-0x441de): the marking
/// pass reaches it, above AI type 4 or dropped by the check this pass.
fn marks(npc: &Npc, dropped: &BTreeSet<NpcId>) -> bool {
    swarm::marks_its_parent(npc) || dropped.contains(&npc.id)
}

/// Whether each of `npcs` is marked: a formation parent, among `parents`,
/// of a ship that marks (see the module docs).
fn marked(npcs: &[Npc], parents: &[Option<usize>], dropped: &BTreeSet<NpcId>) -> Vec<bool> {
    let mut marked = vec![false; npcs.len()];
    for (npc, parent) in npcs.iter().zip(parents) {
        if let Some(&at) = parent.as_ref().filter(|_| marks(npc, dropped)) {
            marked[at] = true;
        }
    }
    marked
}

/// `npc`'s parent as the word +0xc92e holds it, among `npcs`: the player's
/// slot, [`PLAYER_WORD`], a berth, or none, for no parent or a leader not
/// here.
fn word(npc: &Npc, npcs: &[Npc]) -> Option<u32> {
    match swarm::formation_parent(npc)? {
        FormationParent::Player => Some(PLAYER_WORD),
        FormationParent::Berth(berth) => Some(berth),
        FormationParent::Leader(id) => npcs
            .iter()
            .find(|other| other.id == id)
            .map(|other| other.berth),
    }
}

/// Whether some ship among `npcs` marks the player as its formation
/// parent (`_HandlePlayer` @0x6f1cb-0x6f1df), `dropped` as [`slots`] reads
/// it.
fn player_marked(npcs: &[Npc], dropped: &BTreeSet<NpcId>) -> bool {
    npcs.iter()
        .any(|npc| word(npc, npcs) == Some(PLAYER_WORD) && marks(npc, dropped))
}

/// The slots of the formation of `npcs[at]`, whose followers are the
/// others whose parent, among `parents`, it is (see the module docs),
/// numbered as the player's are ([`escort::number_formation`]).
fn formation(npcs: &[Npc], parents: &[Option<usize>], at: usize) -> Vec<(NpcId, FormationSlot)> {
    let followers = npcs
        .iter()
        .zip(parents)
        .enumerate()
        .filter(|&(of, (_, &parent))| of != at && parent == Some(at))
        .map(|(_, (npc, _))| npc);
    escort::number_formation(npcs[at].hull.sprite_size, followers)
}

/// One word a parent's formation calc writes (see the module docs): the
/// place of `follower`, in `berth`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Written {
    /// The follower.
    pub follower: NpcId,
    /// Its berth, which keeps the word.
    pub berth: u32,
    /// The place worked out from its parent's state.
    pub place: Vec2,
    /// Whether its dispatch comes before its parent's, so it reads the
    /// word only on the next frame.
    pub late: bool,
}

/// The words the formation calcs among `npcs` write: of every marked
/// parent (+0xc2, @0x8fcaf, `dropped` as [`slots`] reads it) whose
/// dispatch reaches the calc ([`dispatch_gate::reaches_formation_calc`]),
/// or, for a dispatch of its own, of `only` alone, marked and not a pod.
/// Numbers come from the parents and followers as they stand: a pod
/// follower below its parent is freed at its own dispatch (@0x8fbb3) and
/// out of the original's count that frame, but it needs a plug-in fleet
/// naming shïp 895, and lives one frame, in which its fellows are jumping
/// in and take no nudge, so no boundary shows it.
#[must_use]
pub fn written(npcs: &[Npc], only: Option<NpcId>, dropped: &BTreeSet<NpcId>) -> Vec<Written> {
    let parents = parents(npcs);
    let marked = marked(npcs, &parents, dropped);
    let writes = |npc: &Npc| match only {
        Some(parent) => npc.id == parent && !pod_flight::is_pod(npc),
        None => dispatch_gate::reaches_formation_calc(npc),
    };
    (0..npcs.len())
        .filter(|&at| marked[at] && writes(&npcs[at]))
        .flat_map(|at| {
            let parent = &npcs[at];
            formation(npcs, &parents, at)
                .into_iter()
                .filter_map(move |(follower, slot)| {
                    let berth = npcs.iter().find(|npc| npc.id == follower)?.berth;
                    Some(Written {
                        follower,
                        berth,
                        place: slot.position(&parent.state),
                        late: berth < parent.berth,
                    })
                })
        })
        .collect()
}

/// Works `lead`'s formation out among `npcs` (as [`slots`] does) and
/// puts each of its followers on its slot as a fleet arrives (see the
/// module docs): each writes its [`Npc::fleet_slot`], its
/// [`Npc::slot_place`] and its word in `places`, and each not still out of
/// a gate ([`Mode::Emerging`]) is moved onto that place, its heading and
/// velocity left as they are.
pub fn snap(npcs: &mut [Npc], lead: NpcId, places: &mut BerthPlaces) {
    let Some(at) = npcs.iter().position(|npc| npc.id == lead) else {
        return;
    };
    let state = npcs[at].state;
    for (id, slot) in formation(npcs, &parents(npcs), at) {
        if let Some(npc) = npcs.iter_mut().find(|npc| npc.id == id) {
            let place = slot.position(&state);
            npc.fleet_slot = Some(slot);
            npc.slot_place = Some(place);
            places.write(npc.berth, Some(place));
            if !matches!(npc.mode, Mode::Emerging { .. }) {
                npc.state.position = place;
            }
        }
    }
}

impl Npc {
    /// `_AISetup`'s +0xc906 and +0xc92e (@0x7f415-0x7f41e), for a ship
    /// that stays in its slot and runs it: a release, a capture, or a
    /// former escort boarded back. No swarm leader and no formation
    /// parent, so no formation numbers it while its AI type stays down.
    pub fn clear_swarm_words(&mut self) {
        self.swarm_leader = None;
        self.form_parent = None;
    }
}

impl Traffic {
    /// Writes each NPC's formation parent ([`Npc::form_parent`]) and its
    /// number in each marked NPC parent's formation ([`slots`]), then the
    /// places ([`Traffic::write_places`], with `player` the player's
    /// state, if it is here and marked; see the module docs). `dropped`
    /// names the ships the check dropped this pass
    /// ([`Traffic::succeed_leaders`]). A follower of an unmarked parent
    /// keeps its number and its place, and only a ship with no parent has
    /// its [`Npc::fleet_slot`] cleared.
    pub(crate) fn form_fleets(&mut self, player: Option<&ShipState>, dropped: &BTreeSet<NpcId>) {
        let npcs = &self.npcs;
        let words: Vec<Option<u32>> = npcs.iter().map(|npc| word(npc, npcs)).collect();
        let slots = slots(npcs, dropped);
        let player = player.filter(|_| player_marked(npcs, dropped));
        for (npc, word) in self.npcs.iter_mut().zip(words) {
            npc.form_parent = word;
            if word.is_none() {
                npc.fleet_slot = None;
            }
            if let Some(&(_, slot)) = slots.iter().find(|&&(id, _)| id == npc.id) {
                npc.fleet_slot = Some(slot);
            }
        }
        self.write_places(player, dropped);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort::EscortDuty;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;

    /// NPC `id` at berth `berth`, led by `leader` and `width` wide.
    fn ship(id: u32, berth: u32, leader: Option<u32>, width: Option<f32>) -> Npc {
        let mut ship = npc(id, ShipStats::default());
        ship.berth = berth;
        ship.leader = leader.map(NpcId);
        ship.hull.sprite_size = width;
        ship
    }

    /// The slots of `npcs` with no ship dropped ([`super::slots`]).
    fn slots(npcs: &[Npc]) -> Vec<(NpcId, FormationSlot)> {
        super::slots(npcs, &BTreeSet::new())
    }

    /// The slot numbers of `slots`, each with its ship's number, sorted by
    /// ship.
    fn numbers(slots: &[(NpcId, FormationSlot)]) -> Vec<(u32, u8)> {
        let mut numbers: Vec<(u32, u8)> =
            slots.iter().map(|(id, slot)| (id.0, slot.slot)).collect();
        numbers.sort_unstable();
        numbers
    }

    #[test]
    fn followers_take_slots_by_berth_not_by_their_order_among_the_ships() {
        let ships = [
            ship(1, 9, Some(0), None),
            ship(0, 5, None, None),
            ship(2, 6, Some(0), None),
            ship(3, 7, Some(0), None),
        ];
        let slots = slots(&ships);
        assert_eq!(numbers(&slots), [(1, 4), (2, 2), (3, 3)]);
        assert!(slots.iter().all(|(_, slot)| slot.ships == 4));
    }

    #[test]
    fn the_spacing_is_the_widest_sprites_the_parents_included() {
        let spacing = |lead: Option<f32>, widths: &[Option<f32>]| {
            let mut ships = vec![ship(0, 1, None, lead)];
            for (n, &width) in (1..).zip(widths) {
                ships.push(ship(n, n + 1, Some(0), width));
            }
            let slots = slots(&ships);
            assert_eq!(slots.len(), widths.len());
            slots[0].1.spacing
        };
        assert_eq!(spacing(None, &[Some(40.0), Some(70.0)]), 42.0, "70 × 0.6");
        assert_eq!(spacing(None, &[Some(20.0)]), 38.0, "64 with no sprite");
        assert_eq!(spacing(Some(30.0), &[Some(20.0), Some(30.0)]), 24.0);
        assert_eq!(spacing(Some(30.0), &[Some(120.0)]), 60.0);
    }

    #[test]
    fn a_disabled_follower_takes_its_slot_and_counts() {
        let mut down = ship(1, 2, Some(0), None);
        down.condition = Condition::Disabled;
        let ships = [ship(0, 1, None, None), down, ship(2, 3, Some(0), None)];
        let slots = slots(&ships);
        assert_eq!(numbers(&slots), [(1, 2), (2, 3)]);
        assert!(slots.iter().all(|(_, slot)| slot.ships == 3));
    }

    #[test]
    fn a_ship_attacking_with_a_swarm_leader_forms_on_it_by_its_berth() {
        // NPC 4 escorts NPC 3, but attacks with NPC 0 (berth 5) as its
        // swarm leader: it forms on NPC 0, between NPC 0's own escorts.
        let mut swarmer = ship(4, 7, Some(3), None);
        swarmer.goal = Goal::Attack(ShipRef::Npc(NpcId(9)));
        swarmer.target = Some(ShipRef::Npc(NpcId(9)));
        swarmer.swarm_leader = Some(5);
        let ships = [
            ship(0, 5, None, None),
            ship(1, 9, Some(0), None),
            ship(2, 6, Some(0), None),
            ship(3, 2, None, None),
            swarmer,
        ];
        let slots = slots(&ships);
        assert_eq!(numbers(&slots), [(1, 4), (2, 2), (4, 3)]);
        assert!(slots.iter().all(|(_, slot)| slot.ships == 4));
    }

    #[test]
    fn the_players_escorts_and_a_ship_whose_leader_is_gone_hold_none() {
        let mut hired = ship(1, 2, None, None);
        hired.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        let orphan = ship(2, 3, Some(7), None);
        let ships = [ship(0, 1, None, None), hired, orphan];
        assert!(slots(&ships).is_empty());
    }

    #[test]
    fn a_ship_that_is_its_own_swarm_leader_holds_no_slot_on_itself() {
        let mut lead = ship(0, 1, Some(5), None);
        lead.goal = Goal::Attack(ShipRef::Npc(NpcId(9)));
        lead.target = Some(ShipRef::Npc(NpcId(9)));
        lead.swarm_leader = Some(1);
        let ships = [lead, ship(5, 2, None, None)];
        assert!(slots(&ships).is_empty());
    }

    mod in_traffic {
        use super::*;
        use crate::ai::{Behaviour, Surroundings};
        use crate::chance::Chance;
        use crate::flight::ShipState;
        use crate::geometry::Vec2;
        use crate::traffic::World;
        use nova_sim_base::fixture::Draws;
        use std::collections::BTreeSet;

        /// Every NPC keeps its goal.
        #[derive(Debug)]
        struct Keep;

        impl Behaviour for Keep {
            fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
                npc.goal
            }
        }

        #[test]
        fn a_think_comes_out_with_each_fleet_slot_set_and_the_rest_cleared() {
            let mut traffic = Traffic::with_interval(1);
            let lead = traffic.add_npc(ship(0, 0, None, Some(50.0)));
            for _ in 0..2 {
                let mut escort = ship(0, 0, None, None);
                escort.leader = Some(lead);
                traffic.add_npc(escort);
            }
            let mut stale = ship(0, 0, None, None);
            stale.fleet_slot = Some(FormationSlot::default());
            traffic.add_npc(stale);
            traffic.think_holding(&Keep, World::new(&[]), &[], &mut Draws::of(&[0]));
            let slots: Vec<Option<FormationSlot>> =
                traffic.npcs().iter().map(|npc| npc.fleet_slot).collect();
            let at = |slot| {
                Some(FormationSlot {
                    slot,
                    ships: 3,
                    spacing: 30.0,
                })
            };
            assert_eq!(slots, [None, at(2), at(3), None]);
        }

        /// Runs the pass with no ship dropped, the player at `player`.
        fn formed(traffic: &mut Traffic, player: Option<ShipState>) {
            traffic.form_fleets(player.as_ref(), &BTreeSet::new());
        }

        fn of(traffic: &Traffic, id: NpcId) -> &Npc {
            traffic
                .npcs()
                .iter()
                .find(|npc| npc.id == id)
                .expect("there")
        }

        /// A ship of AI type 4 or below holding the parent word `word`.
        fn stale(word: u32, width: Option<f32>) -> Npc {
            let mut ship = ship(0, 0, None, width);
            ship.form_parent = Some(word);
            ship
        }

        const FAR: ShipState = ShipState {
            position: Vec2::new(300.0, -100.0),
            velocity: Vec2::new(1.0, 0.0),
            heading: 90.0,
        };

        /// A traffic of a parent at berth 1, [`FAR`], 20 wide.
        fn parented() -> (Traffic, NpcId) {
            let mut traffic = Traffic::with_interval(1);
            let mut parent = ship(0, 0, None, Some(20.0));
            parent.state = FAR;
            let id = traffic.add_npc_at(parent, 1);
            (traffic, id)
        }

        #[test]
        fn a_stale_follower_takes_a_number_and_a_width_in_a_marked_formation() {
            // @0x850bc, @0x85166: neither loop tests the AI type.
            let (mut traffic, parent) = parented();
            let was_stale = traffic.add_npc_at(stale(1, Some(100.0)), 3);
            let mut real = ship(0, 0, Some(parent.0), Some(10.0));
            real.state = FAR;
            let real = traffic.add_npc_at(real, 5);
            formed(&mut traffic, None);
            let slot = |number| FormationSlot {
                slot: number,
                ships: 3,
                spacing: 60.0,
            };
            assert_eq!(of(&traffic, was_stale).fleet_slot, Some(slot(2)));
            assert_eq!(of(&traffic, real).fleet_slot, Some(slot(3)));
            assert_eq!(
                of(&traffic, was_stale).slot_place,
                Some(slot(2).position(&FAR)),
                "on the parent's slot after the pass"
            );
            assert_eq!(of(&traffic, real).slot_place, Some(slot(3).position(&FAR)));
        }

        #[test]
        fn with_no_ship_above_ai_4_naming_the_parent_a_stale_follower_keeps_both() {
            let (mut traffic, parent) = parented();
            let mut held = stale(1, None);
            let slot = FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            };
            held.fleet_slot = Some(slot);
            held.slot_place = Some(Vec2::new(7.0, 8.0));
            let held = traffic.add_npc_at(held, 3);
            formed(&mut traffic, None);
            if let Some(npc) = traffic.npcs_mut().iter_mut().find(|npc| npc.id == parent) {
                npc.state.position = Vec2::new(900.0, 900.0);
            }
            formed(&mut traffic, None);
            let npc = of(&traffic, held);
            assert_eq!(npc.fleet_slot, Some(slot));
            assert_eq!(npc.slot_place, Some(Vec2::new(7.0, 8.0)), "fixed");
            assert_eq!(npc.form_parent, Some(1));
        }

        #[test]
        fn a_follower_of_a_berth_with_no_ship_keeps_its_parent_slot_and_place() {
            // The mark tests no +0xb8 (@0x441d1-0x441de), but the dispatch
            // loop does (@0x44275-0x4427c): no calc runs for an empty slot.
            const EMPTY: u32 = 7;
            let slot = FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            };
            let place = Vec2::new(7.0, 8.0);
            let holding = |mut npc: Npc| {
                npc.form_parent = Some(EMPTY);
                npc.fleet_slot = Some(slot);
                npc.slot_place = Some(place);
                npc
            };
            let (mut traffic, parent) = parented();
            let mut attacking = ship(0, 0, Some(parent.0), None);
            attacking.goal = Goal::Attack(ShipRef::Npc(NpcId(9)));
            attacking.target = Some(ShipRef::Npc(NpcId(9)));
            attacking.swarm_leader = Some(EMPTY);
            assert!(swarm::marks_its_parent(&attacking), "above AI 4");
            let attacking = traffic.add_npc_at(holding(attacking), 3);
            let held = traffic.add_npc_at(holding(stale(EMPTY, None)), 4);
            formed(&mut traffic, None);
            for id in [attacking, held] {
                let npc = of(&traffic, id);
                assert_eq!(npc.form_parent, Some(EMPTY));
                assert_eq!(npc.fleet_slot, Some(slot));
                assert_eq!(npc.slot_place, Some(place));
            }
        }

        #[test]
        fn the_pass_writes_the_word_the_rule_gives() {
            let (mut traffic, parent) = parented();
            let led = traffic.add_npc_at(ship(0, 0, Some(parent.0), None), 3);
            let mut swarming = ship(0, 0, Some(parent.0), None);
            swarming.goal = Goal::Attack(ShipRef::Npc(NpcId(9)));
            swarming.target = Some(ShipRef::Npc(NpcId(9)));
            swarming.swarm_leader = Some(7);
            let swarming = traffic.add_npc_at(swarming, 4);
            let mut escort = ship(0, 0, None, None);
            escort.escort = Some(EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            });
            let escort = traffic.add_npc_at(escort, 5);
            let mut orphan = ship(0, 0, Some(40), None);
            orphan.form_parent = Some(9);
            let orphan = traffic.add_npc_at(orphan, 6);
            let mut none = ship(0, 0, None, None);
            none.form_parent = None;
            none.fleet_slot = Some(FormationSlot::default());
            let none = traffic.add_npc_at(none, 8);
            formed(&mut traffic, None);
            let word = |id| of(&traffic, id).form_parent;
            assert_eq!(word(led), Some(1), "its leader as its berth");
            assert_eq!(word(swarming), Some(7));
            assert_eq!(word(escort), Some(0), "the player");
            assert_eq!(word(orphan), None, "its leader not here");
            assert_eq!(word(none), None);
            assert_eq!(of(&traffic, none).fleet_slot, None);
        }

        #[test]
        fn a_berth_used_again_reads_as_its_new_ship() {
            let (mut traffic, parent) = parented();
            let held = traffic.add_npc_at(stale(1, None), 3);
            let driver = traffic.add_npc_at(ship(0, 0, Some(parent.0), None), 4);
            formed(&mut traffic, None);
            traffic.remove(parent);
            let mut new = ship(0, 0, None, Some(40.0));
            new.state.position = Vec2::new(-500.0, 40.0);
            let new_state = new.state;
            let new = traffic.add_npc_at(new, 1);
            if let Some(npc) = traffic.npcs_mut().iter_mut().find(|npc| npc.id == driver) {
                npc.leader = Some(new);
            }
            formed(&mut traffic, None);
            let npc = of(&traffic, held);
            assert_eq!(npc.form_parent, Some(1));
            assert_eq!(
                npc.slot_place,
                npc.fleet_slot.map(|slot| slot.position(&new_state))
            );
            assert_eq!(npc.fleet_slot.map(|slot| slot.ships), Some(3));
        }

        #[test]
        fn a_ship_that_regains_a_leader_is_rewritten() {
            let (mut traffic, parent) = parented();
            let back = traffic.add_npc_at(stale(9, None), 3);
            if let Some(npc) = traffic.npcs_mut().iter_mut().find(|npc| npc.id == back) {
                npc.leader = Some(parent);
            }
            formed(&mut traffic, None);
            assert_eq!(of(&traffic, back).form_parent, Some(1));
        }

        #[test]
        fn clearing_the_swarm_words_leaves_a_ship_with_no_parent() {
            let mut held = stale(1, None);
            held.swarm_leader = Some(1);
            held.clear_swarm_words();
            assert_eq!((held.swarm_leader, held.form_parent), (None, None));
            assert_eq!(swarm::formation_parent(&held), None);
        }

        /// A ship the player leads, in `slot`.
        fn escorting(slot: FormationSlot) -> Npc {
            let mut hired = ship(0, 0, None, None);
            hired.escort = Some(EscortDuty {
                formation: slot,
                order: None,
            });
            hired
        }

        #[test]
        fn a_ship_on_the_player_takes_its_place_from_the_slot_it_holds() {
            let held = FormationSlot {
                slot: 2,
                ships: 3,
                spacing: 40.0,
            };
            let leftover = FormationSlot {
                slot: 4,
                ships: 5,
                spacing: 60.0,
            };
            let mut traffic = Traffic::with_interval(1);
            let mut rejoined = escorting(held);
            rejoined.fleet_slot = Some(leftover);
            let rejoined = traffic.add_npc_at(rejoined, 1);
            let mut lost = stale(0, None);
            lost.fleet_slot = Some(leftover);
            let lost = traffic.add_npc_at(lost, 2);
            formed(&mut traffic, Some(FAR));
            let npc = of(&traffic, rejoined);
            assert_eq!(npc.slot_place, Some(held.position(&FAR)), "its duty's");
            assert_eq!(npc.fleet_slot, Some(leftover), "left alone");
            assert_eq!(npc.form_parent, Some(0));
            let npc = of(&traffic, lost);
            assert_eq!(npc.slot_place, Some(leftover.position(&FAR)));
            assert_eq!(npc.fleet_slot, Some(leftover));
        }

        #[test]
        fn a_lone_stale_ship_on_the_player_keeps_its_place_as_the_player_moves() {
            // `_HandlePlayer` works its formation out under +0xc2 alone
            // (@0x6f1cb-0x6f1df), which a stale ship doesn't set (@0x440d8).
            let mut traffic = Traffic::with_interval(1);
            let mut lost = stale(0, None);
            lost.fleet_slot = Some(FormationSlot::default());
            lost.slot_place = Some(Vec2::new(7.0, 8.0));
            let lost = traffic.add_npc_at(lost, 2);
            formed(&mut traffic, Some(FAR));
            assert_eq!(of(&traffic, lost).slot_place, Some(Vec2::new(7.0, 8.0)));
        }
    }

    mod arriving {
        use super::*;
        use crate::catalog::{EscortRecord, FleetId, FleetRecord, ShipId};
        use crate::control::Test;
        use crate::flight::{FlightRules, NpcFormation, ShipState};
        use crate::geometry::Vec2;
        use crate::traffic::spawn::FleetCall;
        use crate::traffic::table::SpawnTable;
        use crate::traffic::tests::{gated, table};
        use crate::traffic::{Leave, World};
        use nova_sim_base::fixture::Draws;
        use std::collections::BTreeSet;

        #[test]
        fn a_snap_puts_each_follower_not_out_of_a_gate_on_its_slot_by_the_leads_state() {
            let mut lead = ship(0, 5, None, None);
            lead.state = ShipState {
                position: Vec2::new(100.0, -200.0),
                velocity: Vec2::new(3.0, 4.0),
                heading: 37.0,
            };
            let lead_state = lead.state;
            let follower = |id, berth, off: Vec2, mode| {
                let mut ship = ship(id, berth, Some(0), None);
                ship.state = ShipState {
                    position: lead_state.position + off,
                    ..lead_state
                };
                ship.mode = mode;
                ship
            };
            let jumping = Mode::JumpingIn { tick: 0 };
            let mut other = ship(4, 2, Some(9), None);
            other.state.position = Vec2::new(7.0, 7.0);
            let mut npcs = vec![
                lead,
                follower(1, 9, Vec2::new(-150.0, 149.0), jumping),
                follower(2, 7, Vec2::new(120.0, -30.0), jumping),
                follower(3, 8, Vec2::ZERO, Mode::Emerging { ticks_left: 68.0 }),
                ship(9, 1, None, None),
                other,
            ];
            let mut places = BerthPlaces::default();
            snap(&mut npcs, NpcId(0), &mut places);
            // 64 wide with no sprite: spaced 38, four ships, by berth.
            let slot = |slot| FormationSlot {
                slot,
                ships: 4,
                spacing: 38.0,
            };
            for (at, number) in [(1, 4), (2, 2), (3, 3)] {
                assert_eq!(npcs[at].fleet_slot, Some(slot(number)), "NPC {at}");
                let place = Some(slot(number).position(&lead_state));
                assert_eq!(npcs[at].slot_place, place, "NPC {at}");
                assert_eq!(places.left(npcs[at].berth), place, "NPC {at}'s word");
            }
            assert_eq!(places.left(npcs[5].berth), None, "not its");
            for at in [1, 2] {
                let on = slot(npcs[at].fleet_slot.map_or(0, |slot| slot.slot));
                assert_eq!(
                    npcs[at].state,
                    ShipState {
                        position: on.position(&lead_state),
                        ..lead_state
                    },
                    "NPC {at}: position only"
                );
                assert_eq!(npcs[at].slot_place, Some(on.position(&lead_state)));
            }
            assert_eq!(npcs[3].state.position, lead_state.position, "out of a gate");
            assert_eq!(npcs[5].state.position, Vec2::new(7.0, 7.0), "not its");
            assert_eq!(npcs[5].fleet_slot, None);
        }

        /// [`table`] of `avg_ships`, with fleet 141: ship 200 leading
        /// `escorts` more, linked here when `linked`, and named among the
        /// arrivals when `named`.
        fn fleet_table(avg_ships: u32, escorts: i16, linked: bool, named: bool) -> SpawnTable {
            let mut fleets = table(avg_ships);
            if linked {
                fleets.link_fleets.insert(FleetId(141));
            }
            if named {
                fleets.dude_fleets = vec![(FleetId(141), 100)];
            }
            fleets.fleets.insert(
                FleetId(141),
                FleetRecord {
                    id: FleetId(141),
                    lead: Some(ShipId(200)),
                    escorts: vec![EscortRecord {
                        ship: ShipId(200),
                        min: escorts,
                        max: escorts,
                    }],
                    govt: None,
                    link_syst: -1,
                    appear_on: Test::default(),
                    quote: None,
                    flags: 0,
                },
            );
            fleets
        }

        /// A world among `sites` by `reading` of the NPC formation.
        fn by(reading: NpcFormation, sites: &[crate::catalog::LandingSite]) -> World<'_> {
            World {
                flight: FlightRules {
                    npc_formation: reading,
                    ..FlightRules::default()
                },
                ..World::new(sites)
            }
        }

        /// Each escort of the fleet `traffic` holds (its first NPC the
        /// lead): its fleet slot and its place off its lead.
        fn arrived(traffic: &Traffic) -> Vec<(Option<FormationSlot>, Vec2)> {
            let lead = &traffic.npcs()[0];
            traffic.npcs()[1..]
                .iter()
                .map(|escort| {
                    assert_eq!(escort.leader, Some(lead.id));
                    assert_eq!(
                        (escort.state.heading, escort.state.velocity),
                        (lead.state.heading, lead.state.velocity)
                    );
                    (
                        escort.fleet_slot,
                        escort.state.position - lead.state.position,
                    )
                })
                .collect()
        }

        /// Slot `slot` of the three of a fleet of ship 200 (no sprite:
        /// spaced 38), and its place off its lead at `lead`.
        fn on_slot(slot: u8, lead: &ShipState) -> (Option<FormationSlot>, Vec2) {
            let slot = FormationSlot {
                slot,
                ships: 3,
                spacing: 38.0,
            };
            (Some(slot), slot.position(lead) - lead.position)
        }

        /// Where the ±150 draws left each escort (the draws run out: 299,
        /// so 149 each way).
        const SCATTERED: (Option<FormationSlot>, Vec2) = (None, Vec2::new(149.0, 149.0));

        /// `traffic` arrived as `reading` says: on its slots, or scattered.
        fn assert_arrived(traffic: &Traffic, reading: NpcFormation) {
            let lead = traffic.npcs()[0].state;
            assert_eq!(traffic.npcs().len(), 3);
            let expected = match reading {
                NpcFormation::Original => vec![on_slot(2, &lead), on_slot(3, &lead)],
                NpcFormation::Earlier => vec![SCATTERED; 2],
            };
            assert_eq!(arrived(traffic), expected, "{reading:?}");
        }

        const READINGS: [NpcFormation; 2] = [NpcFormation::Original, NpcFormation::Earlier];

        #[test]
        fn a_fleet_set_up_jumping_in_arrives_on_its_slots() {
            for reading in READINGS {
                let mut traffic = Traffic::new();
                // Not a person, a fleet, slot 13.
                traffic.enter_in(
                    fleet_table(1, 2, true, false),
                    by(reading, &[]),
                    &mut Draws::of(&[6, 0, 13]),
                );
                assert!(matches!(traffic.npcs()[0].mode, Mode::JumpingIn { .. }));
                assert_arrived(&traffic, reading);
            }
        }

        #[test]
        fn a_fleet_arriving_by_the_arrivals_pass_arrives_on_its_slots() {
            for reading in READINGS {
                let mut traffic = Traffic::new();
                traffic.retarget(
                    fleet_table(1, 2, false, true),
                    &BTreeSet::new(),
                    Leave::Dropped,
                );
                // A fleet's arrival, the 100 % fleet named, fleet 141.
                traffic.enter_more_ships(by(reading, &[]), &mut Draws::of(&[1, 0, 0]));
                assert_arrived(&traffic, reading);
            }
        }

        #[test]
        fn a_strength_or_reinforcement_fleet_arrives_on_its_slots() {
            for reading in READINGS {
                let mut traffic = Traffic::new();
                traffic.retarget(
                    fleet_table(0, 2, false, false),
                    &BTreeSet::new(),
                    Leave::Dropped,
                );
                let world = by(reading, &[]);
                traffic.bring_fleet(
                    FleetId(141),
                    FleetCall::ANNOUNCED_INTERCEPTOR,
                    &world,
                    world.persons.gate(),
                    &mut Draws::of(&[]),
                );
                assert_arrived(&traffic, reading);
            }
        }

        #[test]
        fn a_fleet_out_of_a_gate_takes_its_slots_at_the_gates_centre() {
            let sites = gated();
            for reading in READINGS {
                let mut traffic = Traffic::new();
                // Not a person, a fleet, slot 13, the gate, the lead's
                // aggression, its escort's offsets and extra wait.
                traffic.enter_in(
                    fleet_table(1, 1, true, false),
                    by(reading, &sites),
                    &mut Draws::of(&[6, 0, 13, 1, 0, 0, 0, 3]),
                );
                let (lead, escort) = (&traffic.npcs()[0], &traffic.npcs()[1]);
                assert!(matches!(escort.mode, Mode::Emerging { .. }));
                assert_eq!(escort.state, lead.state, "at the gate's centre");
                let slot = (reading == NpcFormation::Original).then_some(FormationSlot {
                    slot: 2,
                    ships: 2,
                    spacing: 38.0,
                });
                assert_eq!(escort.fleet_slot, slot, "{reading:?}");
            }
        }
    }
}
