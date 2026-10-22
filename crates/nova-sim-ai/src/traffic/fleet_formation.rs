//! The formations of NPC parents: each ship whose formation parent is an
//! NPC holds a slot in that NPC's formation
//! ([`Npc::fleet_slot`](super::npc::Npc::fleet_slot)).
//!
//! **The parent** (`_DoPlayGameWork` @0x4415c-0x441de): a ship's
//! formation parent +0xc92e is its swarm leader while it attacks with a
//! target, else its leader ([`swarm::formation_parent`]). The slot that
//! +0xc92e names gets +0xc2, "is a formation parent", which makes its
//! dispatch work its followers' slots out. Neither is stored here: the
//! pass groups the NPCs by their parents each tick.
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
//! **When.** The think half of a tick works the slots out
//! ([`Traffic::form_fleets`]) after the answers to the last tick's hits,
//! which the original lands before its marking pass, and before any
//! think, as the marking pass precedes the `_AIDispatch` loop. Each slot
//! is a place relative to its parent ([`FormationSlot`]), so a follower
//! reads it from its parent's state as the fly half began. The original
//! writes the slots' positions in the parent's own dispatch
//! (@0x8fcaf-0x8fcc3): a follower in a lower slot than its parent is
//! nudged toward the frame before's, and a parent that never dispatches
//! leaves them fixed. Those two differences are filed whole as
//! rdm:phase/escort-fleet-fidelity-followups/phase-7-npc-formation-slots-at-the-parents-dispatch.
//!
//! **Arriving** (`_HyperSpawnFleet` @0x425e4-0x42602): once a fleet is
//! placed, `_AICalcFormationPositions(lead, 1)` puts each escort that
//! jumps in on its slot, position only (`_AIMaintainFormation(ship, 1)`
//! @0x84fc2-0x84fce), and leaves one out of a gate (goal 0x15,
//! @0x851a7-0x851af) at the gate's centre ([`snap`]). Both the pass and
//! the snap are the engine's reading of
//! [`RuleKey::NpcFormation`](crate::RuleKey::NpcFormation); by the other
//! the slots are still worked out, but nothing flies them.
//!
//! None of these routines plays a sound or draws a random number.

use super::Traffic;
use super::npc::{Mode, Npc, NpcId};
use crate::ai::swarm::{self, FormationParent};
use crate::escort::{self, FormationSlot};

/// The slot of each ship whose formation parent is an NPC here, in that
/// NPC's formation (see the module docs). A ship whose parent is the
/// player, or an NPC not here, holds none, nor does a ship that is its own
/// parent.
#[must_use]
pub fn slots(npcs: &[Npc]) -> Vec<(NpcId, FormationSlot)> {
    let parents: Vec<Option<usize>> = npcs.iter().map(|npc| parent_of(npc, npcs)).collect();
    let mut slots = Vec::new();
    for (at, parent) in npcs.iter().enumerate() {
        let mut followers: Vec<&Npc> = npcs
            .iter()
            .zip(&parents)
            .enumerate()
            .filter(|&(of, (_, &parent))| of != at && parent == Some(at))
            .map(|(_, (npc, _))| npc)
            .collect();
        if followers.is_empty() {
            continue;
        }
        followers.sort_by_key(|npc| npc.berth);
        let ships = u8::try_from(followers.len() + 1).unwrap_or(u8::MAX);
        let widths: Vec<Option<f32>> = followers.iter().map(|npc| npc.hull.sprite_size).collect();
        let spacing = escort::spacing(parent.hull.sprite_size, &widths);
        slots.extend(followers.iter().enumerate().map(|(place, npc)| {
            let slot = FormationSlot {
                slot: u8::try_from(place + 2).unwrap_or(u8::MAX),
                ships,
                spacing,
            };
            (npc.id, slot)
        }));
    }
    slots
}

/// Where among `npcs` the formation parent of `npc` is: its leader by its
/// number, its swarm leader by its berth; none for the player, or a
/// parent not here.
fn parent_of(npc: &Npc, npcs: &[Npc]) -> Option<usize> {
    match swarm::formation_parent(npc)? {
        FormationParent::Player => None,
        FormationParent::Leader(id) => npcs.iter().position(|other| other.id == id),
        FormationParent::Berth(berth) => npcs.iter().position(|other| other.berth == berth),
    }
}

/// Works `lead`'s formation out among `npcs` ([`slots`]) and puts each of
/// its followers on its slot as a fleet arrives (see the module docs):
/// each writes its [`Npc::fleet_slot`], and each not still out of a gate
/// ([`Mode::Emerging`]) is moved onto its slot's place from `lead`'s
/// state, its heading and velocity left as they are.
pub fn snap(npcs: &mut [Npc], lead: NpcId) {
    let Some(state) = npcs.iter().find(|npc| npc.id == lead).map(|npc| npc.state) else {
        return;
    };
    let formation: Vec<(NpcId, FormationSlot)> = slots(npcs)
        .into_iter()
        .filter(|&(id, _)| {
            npcs.iter()
                .find(|npc| npc.id == id)
                .and_then(|npc| parent_of(npc, npcs))
                .is_some_and(|at| npcs[at].id == lead)
        })
        .collect();
    for (id, slot) in formation {
        if let Some(npc) = npcs.iter_mut().find(|npc| npc.id == id) {
            npc.fleet_slot = Some(slot);
            if !matches!(npc.mode, Mode::Emerging { .. }) {
                npc.state.position = slot.position(&state);
            }
        }
    }
}

impl Traffic {
    /// Writes each NPC's place in its NPC parent's formation ([`slots`]),
    /// and none to every NPC that holds none (see the module docs).
    pub(crate) fn form_fleets(&mut self) {
        let slots = slots(&self.npcs);
        for npc in &mut self.npcs {
            npc.fleet_slot = slots
                .iter()
                .find(|&&(id, _)| id == npc.id)
                .map(|&(_, slot)| slot);
        }
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
        use crate::traffic::World;
        use nova_sim_base::fixture::Draws;

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
    }
}
