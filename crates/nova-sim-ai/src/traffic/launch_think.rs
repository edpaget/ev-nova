//! The think of a fighter launched in `_AIDispatch`'s loop
//! ([`Traffic::think_launched`]).
//!
//! `_LaunchFighter` marks the fighter's slot active at once
//! (`_GenericRandomShipSpawn` @0x3c90b, `_LaunchFighter` @0x3d680), in its
//! carrier's system (@0x3d694), of AI type 5 (@0x3d771), and
//! `_DoPlayGameWork`'s loop rereads each slot as it comes
//! (@0x44270-0x442ae). So a fighter put in a slot above its carrier's is
//! reached by the same loop in its launch frame, and its `_AIDispatch`
//! runs its whole AI: its fields on entry (mission mark -1 @0x3c9a9, fade
//! 0 @0x3c9c7, +0x50 0 @0x3d79d, substate and goal 0 @0x3d7a3/@0x3d7af)
//! take it past every gate to `_AIHandleCloaking` (@0x8fcaa), its type AI
//! whatever the frame counter (@0x8fe20, @0x8fe25), `_EscortAI`
//! (@0x90011-0x90016), `_HighLevelAIHandler` (@0x901e5) and
//! `_LowLevelAIHandler` (@0x901f4).
//!
//! The traffic thinks every existing NPC in one batch before the loop
//! ([`Traffic::think_holding`]); a launched fighter thinks here, alone,
//! by the same per-NPC think, every other NPC left exactly as the batch
//! left it. Its launches take only the room the batch left
//! (see `launch_room`), as every existing carrier's claim was settled
//! there; the original lets the fighter's launch take a slot before a
//! higher existing carrier's
//! (rdm:phase/combat-ai-fidelity/phase-32-left-out-of-weapons-fidelity-phase-44).

use std::collections::BTreeSet;

use super::launch_room::LaunchRoom;
use super::{Traffic, World};
use crate::ai::Behaviour;
use crate::chance::Chance;
use crate::combat::NpcId;

impl Traffic {
    /// The think of each NPC in `ids` alone, fighters just launched, in
    /// `world`, deciding as `behaviour` says, rolling on `chance` (see the
    /// module docs): the decisions to the cloak thinks, as
    /// [`Traffic::think_holding`] runs them for every NPC, every other NPC
    /// left as it was. Their launches settle against the room this tick's
    /// think left, or with none yet the berths free now.
    pub fn think_launched(
        &mut self,
        ids: &BTreeSet<NpcId>,
        behaviour: &(impl Behaviour + ?Sized),
        world: World,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let room = self
            .launch_room
            .unwrap_or_else(|| LaunchRoom::new(self.free_berths(world.spawn_reserve)));
        let left = self.think_npcs(behaviour, &world, room, |npc| ids.contains(&npc.id), chance);
        self.launch_room = Some(left);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::ai::{GateWait, Goal, Surroundings};
    use crate::catalog::{WeaponId, WeaponRecord};
    use crate::cloak::Cloak;
    use crate::combat::ShipRef;
    use crate::combat::armament::{Armament, LauncherRounds, Trigger};
    use crate::combat::weapon::WeaponSpec;
    use crate::stats::ShipStats;
    use crate::traffic::fixture;
    use crate::traffic::npc::{Mode, Npc};
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::weapon;

    /// What [`Marking`] has every thinking NPC fire.
    const MARKED: Trigger = Trigger {
        primary: true,
        secondary: None,
        only: None,
        turrets_only: false,
        bays: false,
        bays_after: false,
        point_defence: true,
        keep: Some(WeaponId(128)),
    };

    /// Attacks the player, fires [`MARKED`] at it, and holds a gate wait
    /// that drops its target: each NPC that thinks is marked by it.
    #[derive(Debug, Default)]
    struct Marking(RefCell<Vec<NpcId>>);

    impl Behaviour for Marking {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            self.0.borrow_mut().push(npc.id);
            Goal::Attack(ShipRef::Player)
        }

        fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
            MARKED
        }

        fn target(&self, _npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            Some(ShipRef::Player)
        }

        fn gate_wait(&self, _npc: &Npc, _around: &Surroundings) -> GateWait {
            GateWait {
                trigger: MARKED,
                drops_target: true,
            }
        }
    }

    /// A carrier whose bay is ready with two fighters.
    fn carrier(id: u32) -> Npc {
        let bay = WeaponRecord {
            guidance: 99,
            ammo_type: 144,
            flags: 0x0002,
            reload: 60,
            ..weapon(149)
        };
        Npc {
            armament: Armament::new([(WeaponSpec::new(&bay), 1)]),
            rounds: LauncherRounds::from([(WeaponId(149), 2)]),
            ..fixture::npc(id, ShipStats::default())
        }
    }

    /// The goal-4 launch with the bay as the pick.
    const LAUNCHING: Trigger = Trigger {
        primary: false,
        secondary: None,
        only: Some(WeaponId(149)),
        turrets_only: false,
        bays: true,
        bays_after: false,
        point_defence: false,
        keep: None,
    };

    /// NPC `id` of `traffic`.
    fn find(traffic: &Traffic, id: NpcId) -> &Npc {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("there")
    }

    /// A world whose spawns leave `reserve` slots free.
    fn world(reserve: u32) -> World<'static> {
        World {
            spawn_reserve: reserve,
            ..World::new(&[])
        }
    }

    #[test]
    fn a_launched_fighter_thinks_alone_and_leaves_every_other_npc_as_it_was() {
        let mut traffic = Traffic::new();
        // Cloaked with no cloak: a think would uncloak it.
        let flying = Npc {
            cloak: Cloak {
                fade: 20.0,
                direction: 0,
            },
            ..fixture::npc(0, ShipStats::default())
        };
        traffic.add_npc_at(flying, 1);
        let held = Npc {
            mode: Mode::HeldAtGate { ticks_left: 5 },
            target: Some(ShipRef::Player),
            ..fixture::npc(0, ShipStats::default())
        };
        traffic.add_npc_at(held, 3);
        // Coasting with a kept launch: settled again with no room, it
        // would lose it.
        let coasting = Npc {
            mode: Mode::Coasting { ticks_left: 5 },
            trigger: LAUNCHING,
            ..carrier(0)
        };
        traffic.add_npc_at(coasting, 4);
        let fighter = Npc {
            cloak: Cloak {
                fade: 20.0,
                direction: 0,
            },
            ..fixture::npc(0, ShipStats::default())
        };
        let fighter = traffic.add_npc_at(fighter, 2);
        let others: Vec<Npc> = traffic
            .npcs()
            .iter()
            .filter(|npc| npc.id != fighter)
            .cloned()
            .collect();
        let marking = Marking::default();
        // No berth free below 64 - 63.
        traffic.think_launched(
            &BTreeSet::from([fighter]),
            &marking,
            world(63),
            &mut Draws::of(&[]),
        );
        assert_eq!(marking.0.into_inner(), [fighter], "it alone decides");
        let after: Vec<Npc> = traffic
            .npcs()
            .iter()
            .filter(|npc| npc.id != fighter)
            .cloned()
            .collect();
        assert_eq!(after, others);
        let thought = find(&traffic, fighter);
        assert_eq!(thought.goal, Goal::Attack(ShipRef::Player));
        assert_eq!(thought.target, Some(ShipRef::Player));
        assert_eq!(thought.trigger, MARKED);
        assert_eq!(thought.pick, Some(WeaponId(128)));
        assert_eq!(thought.cloak.direction, -1, "its cloak think");
    }

    /// Keeps each goal, and asks for the goal-4 launch, recording each
    /// NPC's room as it asks.
    #[derive(Debug, Default)]
    struct Launching(RefCell<Vec<(NpcId, bool)>>);

    impl Behaviour for Launching {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            self.0.borrow_mut().push((npc.id, around.fighter_room));
            LAUNCHING
        }
    }

    #[test]
    fn a_launched_fighters_launch_takes_only_the_room_the_batch_left() {
        // Below 64 - 61 = 3, berths 1 and 2 are free: carriers 3 and 5
        // claim both. Carrier 3's fighter is put in berth 1, carrier 5's
        // not yet: one berth is free, but none is left to claim.
        let mut traffic = Traffic::new();
        traffic.add_npc_at(carrier(0), 3);
        traffic.add_npc_at(carrier(0), 5);
        traffic.think_holding(&Launching::default(), world(61), &[], &mut Draws::of(&[]));
        let fighter = traffic.add_npc_at(carrier(0), 1);
        assert_eq!(traffic.free_berths(61), 1);
        let launching = Launching::default();
        traffic.think_launched(
            &BTreeSet::from([fighter]),
            &launching,
            world(61),
            &mut Draws::of(&[]),
        );
        assert_eq!(launching.0.into_inner(), [(fighter, false)], "no room");
        assert_eq!(
            find(&traffic, fighter).trigger,
            Trigger {
                only: Some(WeaponId(149)),
                ..Trigger::default()
            },
            "its launch cleared, its pick kept"
        );
    }

    #[test]
    fn with_room_left_a_launched_fighters_launch_takes_it() {
        let mut traffic = Traffic::new();
        traffic.add_npc_at(carrier(0), 3);
        traffic.think_holding(&Launching::default(), world(60), &[], &mut Draws::of(&[]));
        let fighter = traffic.add_npc_at(carrier(0), 1);
        let launching = Launching::default();
        traffic.think_launched(
            &BTreeSet::from([fighter]),
            &launching,
            world(60),
            &mut Draws::of(&[]),
        );
        assert_eq!(launching.0.into_inner(), [(fighter, true)]);
        assert_eq!(find(&traffic, fighter).trigger, LAUNCHING);
        // It took the last berth the batch left.
        let next = traffic.add_npc_at(carrier(0), 2);
        let launching = Launching::default();
        traffic.think_launched(
            &BTreeSet::from([next]),
            &launching,
            world(60),
            &mut Draws::of(&[]),
        );
        assert_eq!(launching.0.into_inner(), [(next, false)]);
    }

    #[test]
    fn before_any_batch_a_launched_fighter_reads_the_berths_free_now() {
        let mut traffic = Traffic::new();
        let fighter = traffic.add_npc_at(carrier(0), 1);
        let launching = Launching::default();
        traffic.think_launched(
            &BTreeSet::from([fighter]),
            &launching,
            world(61),
            &mut Draws::of(&[]),
        );
        assert_eq!(launching.0.into_inner(), [(fighter, true)], "berth 2 free");
    }
}
