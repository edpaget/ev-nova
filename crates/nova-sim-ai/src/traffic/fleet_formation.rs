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
    let parents = parents(npcs);
    (0..npcs.len())
        .flat_map(|at| formation(npcs, &parents, at))
        .collect()
}

/// Where among `npcs` each one's formation parent is: its leader by its
/// number, its swarm leader by its berth; none for the player, or a
/// parent not here.
fn parents(npcs: &[Npc]) -> Vec<Option<usize>> {
    npcs.iter()
        .map(|npc| match swarm::formation_parent(npc)? {
            FormationParent::Player => None,
            FormationParent::Leader(id) => npcs.iter().position(|other| other.id == id),
            FormationParent::Berth(berth) => npcs.iter().position(|other| other.berth == berth),
        })
        .collect()
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

/// Works `lead`'s formation out among `npcs` (as [`slots`] does) and
/// puts each of its followers on its slot as a fleet arrives (see the
/// module docs): each writes its [`Npc::fleet_slot`], and each not still
/// out of a gate ([`Mode::Emerging`]) is moved onto its slot's place from
/// `lead`'s state, its heading and velocity left as they are.
pub fn snap(npcs: &mut [Npc], lead: NpcId) {
    let Some(at) = npcs.iter().position(|npc| npc.id == lead) else {
        return;
    };
    let state = npcs[at].state;
    for (id, slot) in formation(npcs, &parents(npcs), at) {
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
                follower(3, 8, Vec2::ZERO, Mode::Emerging { ticks_left: 68 }),
                ship(9, 1, None, None),
                other,
            ];
            snap(&mut npcs, NpcId(0));
            // 64 wide with no sprite: spaced 38, four ships, by berth.
            let slot = |slot| FormationSlot {
                slot,
                ships: 4,
                spacing: 38.0,
            };
            for (at, number) in [(1, 4), (2, 2), (3, 3)] {
                assert_eq!(npcs[at].fleet_slot, Some(slot(number)), "NPC {at}");
            }
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
