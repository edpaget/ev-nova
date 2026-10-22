//! The ship slots the NPCs' fighters find as the traffic thinks, slot by
//! slot ([`LaunchRoom`]).
//!
//! `_LaunchFighter` gives a fighter the first slot whose active flag is
//! clear below the reserve it leaves (`_GenericRandomShipSpawn(govt, 8)`,
//! @0x3c8c6-0x3c8e9), and fails quietly with none (@0x3c8eb; its caller
//! `_AILaunchFighter` returns before the sound, the round and the reload,
//! @0x81421-0x81423). `_AIDispatch` runs each slot's AI in turn
//! (@0x44270-0x442ae), its launches with it, so the k free slots go to the
//! first k carriers in slot order whose launch reaches `_LaunchFighter`,
//! and a later carrier sees none left ([`Surroundings::fighter_room`]).
//!
//! The traffic decides the room once, as it asks for each NPC's trigger in
//! berth order ([`berth_order`]): before an NPC's trigger it reads whether
//! a berth is left ([`LaunchRoom::any`]), and its settled trigger takes
//! one for each launch it makes now, or with none left launches nothing
//! ([`LaunchRoom::settle`]). The fight then launches only what the room
//! let through. An NPC can take two berths a tick: a goal-4 sniper's
//! second launch on a snipe frame (@0x8732f), when its first left a bay
//! ready, takes the next free slot before any higher slot's AI runs.
//!
//! The room the think leaves is kept for the tick: a fighter launched in
//! the AI loop that thinks at its launch ([`Traffic::think_launched`])
//! claims from it, after every existing carrier's claim. The original
//! lets that fighter's launch take a slot before a higher existing
//! carrier's (rdm:phase/combat-ai-fidelity-cross-roadmap/phase-6-left-out-of-weapons-fidelity-phase-44);
//! no stock fighter carries a bay.
//!
//! [`Surroundings::fighter_room`]: crate::ai::Surroundings::fighter_room

use std::collections::BTreeSet;

use super::npc::{Mode, Npc};
use super::{MAX_SHIPS_IN_SYSTEM, Traffic};
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;

/// The berths left this tick for the fighters the NPCs launch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LaunchRoom {
    /// How many are left.
    left: u32,
}

impl LaunchRoom {
    /// A tick's room, of `free` berths ([`Traffic::free_berths`]).
    pub(crate) fn new(free: u32) -> Self {
        Self { left: free }
    }

    /// Whether a berth is left: the room a trigger asked for now sees
    /// ([`Surroundings::fighter_room`](crate::ai::Surroundings::fighter_room)).
    pub(crate) fn any(self) -> bool {
        self.left > 0
    }

    /// `npc`'s `trigger` as the room lets it launch: each launch it makes
    /// now ([`launches_now`]) takes a berth. With none left both its
    /// launches are cleared, as `_LaunchFighter` failing leaves no sound,
    /// round or reload (@0x81421-0x81423); with one left for two, the first
    /// takes it and only the second (`bays_after`) is cleared. Its pick
    /// stays either way, as `_AILaunchFighter` writes it before the call
    /// (@0x81411). Any other is as it was.
    pub(crate) fn settle(&mut self, npc: &Npc, trigger: Trigger) -> Trigger {
        let launches = launches_now(npc, trigger);
        if launches == 0 {
            return trigger;
        }
        if self.left == 0 {
            return Trigger {
                bays: false,
                bays_after: false,
                ..trigger
            };
        }
        if launches > self.left {
            self.left = 0;
            return Trigger {
                bays_after: false,
                ..trigger
            };
        }
        self.left -= launches;
        trigger
    }
}

/// How many fighters `npc`, holding `trigger`, launches in this tick's
/// fight, room allowing. A launch needs the ship intact (the high level's
/// `_IsDisabled` gate, @0x8e6eb-0x8e6f2), not waiting at its gate
/// ([`waits_at_gate`], whose gate wait replaces its trigger with one that
/// launches nothing), and a bay ready
/// ([`Armament::bay_to_launch`](crate::combat::armament::Armament::bay_to_launch)):
///
/// - the high level's ([`Trigger::bays`]), when those hold;
/// - the snipe's after it ([`Trigger::bays_after`]): after a first, when
///   that left a bay ready
///   ([`Armament::bay_after_launch`](crate::combat::armament::Armament::bay_after_launch):
///   a bay of `Reload` 0, or the next once the first is empty); alone (a
///   plunderer's, or a goal-4 launch the snipe's call alone makes), when
///   those hold.
fn launches_now(npc: &Npc, trigger: Trigger) -> u32 {
    let ready = npc.condition == Condition::Intact
        && !waits_at_gate(npc.mode)
        && npc.armament.bay_to_launch(&npc.rounds).is_some();
    let first = trigger.bays && ready;
    let second = trigger.bays_after
        && if first {
            npc.armament.bay_after_launch(&npc.rounds).is_some()
        } else {
            ready
        };
    u32::from(first) + u32::from(second)
}

/// Whether an NPC in `mode` waits at its gate: emerging from it or held
/// there, its trigger the gate wait's
/// ([`Behaviour::gate_wait`](crate::ai::Behaviour::gate_wait)).
pub(crate) fn waits_at_gate(mode: Mode) -> bool {
    matches!(mode, Mode::Emerging { .. } | Mode::HeldAtGate { .. })
}

/// The indices of `npcs` in berth order, lowest first: the slot order
/// `_AIDispatch`'s loop runs them in (@0x44270-0x442ae).
pub(crate) fn berth_order(npcs: &[Npc]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..npcs.len()).collect();
    order.sort_by_key(|&index| npcs[index].berth);
    order
}

impl Traffic {
    /// How many berths a spawn that leaves `reserve` slots free could
    /// take, one after another: the free berths ([`Traffic::free_berth`]:
    /// no NPC, ship left behind or reservation holds one) below
    /// [`MAX_SHIPS_IN_SYSTEM`] less `reserve`.
    #[must_use]
    pub fn free_berths(&self, reserve: u32) -> u32 {
        let bound = u32::try_from(MAX_SHIPS_IN_SYSTEM)
            .unwrap_or(u32::MAX)
            .saturating_sub(reserve);
        let held: BTreeSet<u32> = self
            .npcs
            .iter()
            .map(|npc| npc.berth)
            .chain(
                self.left_behind
                    .values()
                    .flatten()
                    .map(|left| left.npc.berth),
            )
            .chain(self.reserved.iter().copied())
            .collect();
        let free = (1..bound).filter(|berth| !held.contains(berth)).count();
        u32::try_from(free).unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::catalog::{SystemId, WeaponId, WeaponRecord};
    use crate::chance::Chance;
    use crate::combat::NpcId;
    use crate::combat::armament::{Armament, LauncherRounds};
    use crate::combat::weapon::WeaponSpec;
    use crate::stats::ShipStats;
    use crate::traffic::fixture;
    use crate::traffic::{LeftShip, World};
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::weapon;

    /// A carrier sniping at the player, its bay ready with two fighters.
    fn carrier() -> Npc {
        let bay = WeaponRecord {
            guidance: 99,
            ammo_type: 144,
            flags: 0x0002,
            reload: 60,
            ..weapon(149)
        };
        Npc {
            goal: Goal::Snipe(crate::combat::ShipRef::Player),
            armament: Armament::new([(WeaponSpec::new(&bay), 1)]),
            rounds: LauncherRounds::from([(WeaponId(149), 2)]),
            ..fixture::npc(0, ShipStats::default())
        }
    }

    /// A goal-4 sniper's launches: the high level's, blind to the room,
    /// and the snipe's own, asked for only when there is none.
    fn sniping(room: bool) -> Trigger {
        Trigger {
            bays: true,
            bays_after: !room,
            only: Some(WeaponId(149)),
            ..Trigger::default()
        }
    }

    #[test]
    fn with_no_berth_left_a_launch_is_cleared() {
        let mut room = LaunchRoom::new(1);
        assert!(room.any());
        assert_eq!(room.settle(&carrier(), sniping(true)), sniping(true));
        assert!(!room.any(), "taken");
        assert_eq!(
            room.settle(&carrier(), sniping(false)),
            Trigger {
                only: Some(WeaponId(149)),
                ..Trigger::default()
            },
            "both calls fail; the pick stays (@0x81411)"
        );
        let plunderer = Trigger {
            bays_after: true,
            ..Trigger::default()
        };
        assert_eq!(room.settle(&carrier(), plunderer), Trigger::default());
    }

    /// A fighter bay `id` of ship 144, reloading every `reload` ticks.
    fn bay(id: i16, reload: i16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance: 99,
            ammo_type: 144,
            flags: 0x0002,
            reload,
            ..weapon(id)
        })
    }

    /// [`carrier`] whose bay reloads in 0 ticks, with three fighters.
    fn reloading_in_zero() -> Npc {
        Npc {
            armament: Armament::new([(bay(149, 0), 1)]),
            rounds: LauncherRounds::from([(WeaponId(149), 3)]),
            ..carrier()
        }
    }

    /// Both a goal-4 sniper's launches asked for, its pick the bay.
    const BOTH: Trigger = Trigger {
        bays: true,
        bays_after: true,
        only: Some(WeaponId(149)),
        primary: false,
        secondary: None,
        point_defence: false,
        keep: Some(WeaponId(149)),
    };

    #[test]
    fn a_bay_reloading_in_zero_takes_a_berth_for_each_call() {
        // Its first launch leaves a reload of 0 (@0x81473-0x81491), so the
        // snipe's call (@0x8732f) launches too, in the next free slot.
        let mut room = LaunchRoom::new(2);
        assert_eq!(room.settle(&reloading_in_zero(), BOTH), BOTH);
        assert!(!room.any(), "both taken");
    }

    #[test]
    fn with_one_berth_left_the_second_call_fails_and_keeps_its_pick() {
        // The second `_LaunchFighter` fails (@0x81421-0x81423) after its
        // pick write (@0x81411).
        let mut room = LaunchRoom::new(1);
        assert_eq!(
            room.settle(&reloading_in_zero(), BOTH),
            Trigger {
                bays_after: false,
                ..BOTH
            }
        );
        assert!(!room.any());
    }

    #[test]
    fn a_launch_emptying_its_bay_takes_a_second_berth_for_the_next_bay() {
        // The walk passes over the emptied 149 (@0x813da) to 150, ready.
        let two_bays = Npc {
            armament: Armament::new([(bay(149, 60), 1), (bay(150, 60), 1)]),
            rounds: LauncherRounds::from([(WeaponId(149), 1), (WeaponId(150), 2)]),
            ..carrier()
        };
        let mut room = LaunchRoom::new(3);
        assert_eq!(room.settle(&two_bays, BOTH), BOTH);
        assert!(room.any(), "one of three left");
        let mut room = LaunchRoom::new(2);
        room.settle(&two_bays, BOTH);
        assert!(!room.any(), "two taken");
    }

    #[test]
    fn a_snipes_only_launch_takes_one_berth() {
        let snipe = Trigger {
            bays: false,
            ..BOTH
        };
        let mut room = LaunchRoom::new(2);
        assert_eq!(room.settle(&reloading_in_zero(), snipe), snipe);
        assert!(room.any(), "one of two left");
        room.settle(&reloading_in_zero(), snipe);
        assert!(!room.any());
    }

    #[test]
    fn a_second_call_finding_its_bay_reloading_takes_no_second_berth() {
        // A kept no-room trigger on a bay of `Reload` 60: the first launch
        // leaves it reloading (@0x81403-0x8140b).
        let mut room = LaunchRoom::new(2);
        assert_eq!(room.settle(&carrier(), BOTH), BOTH);
        assert!(room.any(), "one of two left");
    }

    #[test]
    fn a_carrier_whose_bay_reloads_takes_no_berth() {
        let mut reloading = carrier();
        reloading.armament.hold(WeaponId(149));
        let mut room = LaunchRoom::new(1);
        assert_eq!(room.settle(&reloading, sniping(true)), sniping(true));
        assert!(room.any(), "the next carrier still sees room");
        let mut empty = carrier();
        empty.rounds = LauncherRounds::default();
        assert_eq!(room.settle(&empty, sniping(true)), sniping(true));
        assert!(room.any());
    }

    #[test]
    fn a_carrier_held_at_its_gate_or_not_intact_takes_no_berth() {
        for mode in [
            Mode::HeldAtGate { ticks_left: 5 },
            Mode::Emerging { ticks_left: 5 },
        ] {
            let held = Npc { mode, ..carrier() };
            let mut room = LaunchRoom::new(1);
            room.settle(&held, sniping(true));
            assert!(room.any(), "{mode:?}");
        }
        let disabled = Npc {
            condition: Condition::Disabled,
            ..carrier()
        };
        let mut room = LaunchRoom::new(1);
        room.settle(&disabled, sniping(true));
        assert!(room.any());
        let coasting = Npc {
            mode: Mode::Coasting { ticks_left: 5 },
            ..carrier()
        };
        room.settle(&coasting, sniping(true));
        assert!(!room.any(), "its trigger launches through the wait gate");
    }

    #[test]
    fn a_trigger_launching_nothing_takes_no_berth() {
        let mut room = LaunchRoom::new(1);
        let pick = Trigger {
            only: Some(WeaponId(149)),
            ..Trigger::default()
        };
        assert_eq!(room.settle(&carrier(), pick), pick);
        assert!(room.any());
    }

    #[test]
    fn the_npcs_go_in_berth_order() {
        let at = |berth| Npc { berth, ..carrier() };
        assert_eq!(berth_order(&[at(5), at(2), at(9), at(1)]), [3, 1, 0, 2]);
    }

    /// The berths free, one after another, as [`Traffic::free_berth`]
    /// gives them.
    fn counted(traffic: &Traffic, reserve: u32) -> u32 {
        let mut skip = BTreeSet::new();
        while let Some(berth) = traffic.free_berth_skipping(reserve, &skip) {
            skip.insert(berth);
        }
        u32::try_from(skip.len()).expect("few")
    }

    #[test]
    fn free_berths_counts_what_free_berth_would_give() {
        let mut traffic = Traffic::new();
        for berth in [1, 2, 5] {
            traffic.add_npc_at(carrier(), berth);
        }
        traffic.left_behind.insert(
            SystemId(130),
            vec![LeftShip {
                npc: Npc {
                    berth: 3,
                    ..carrier()
                },
                person: None,
            }],
        );
        traffic.reserve(BTreeSet::from([4, 7]));
        // Below 64 - 56 = 8: only 6 is free.
        for reserve in [56, 57, 58, 8, 0, 63, 64, 70] {
            assert_eq!(
                traffic.free_berths(reserve),
                counted(&traffic, reserve),
                "{reserve}"
            );
        }
        assert_eq!(traffic.free_berths(56), 1);
        assert_eq!(traffic.free_berths(8), 55 - 6);
        assert_eq!(traffic.free_berths(64), 0);
    }

    /// Keeps each goal, and asks for both a goal-4 sniper's launches as
    /// it fires, recording each NPC's room as it asks.
    #[derive(Debug, Default)]
    struct Sniping(RefCell<Vec<(NpcId, bool)>>);

    impl Behaviour for Sniping {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            self.0.borrow_mut().push((npc.id, around.fighter_room));
            sniping(around.fighter_room)
        }
    }

    /// Thinks a tick of `traffic` by [`Sniping`], its spawns leaving
    /// `reserve` slots free: each NPC's room as it was asked, and its
    /// trigger after.
    fn think(traffic: &mut Traffic, reserve: u32) -> (Vec<(NpcId, bool)>, Vec<Trigger>) {
        let sniping = Sniping::default();
        let world = World {
            spawn_reserve: reserve,
            ..World::new(&[])
        };
        traffic.think_holding(&sniping, world, &[], &mut Draws::of(&[]));
        let triggers = traffic.npcs().iter().map(|npc| npc.trigger).collect();
        (sniping.0.into_inner(), triggers)
    }

    #[test]
    fn two_carriers_sharing_the_last_free_berth_the_lower_slot_launches() {
        // Below 64 - 60 = 4, berth 3 is the last free one. The higher
        // berth was added first: vector order is not berth order.
        let mut traffic = Traffic::new();
        let higher = traffic.add_npc_at(carrier(), 2);
        let lower = traffic.add_npc_at(carrier(), 1);
        let (rooms, triggers) = think(&mut traffic, 60);
        assert_eq!(rooms, [(lower, true), (higher, false)]);
        assert_eq!(triggers[1], sniping(true), "the lower launches");
        assert_eq!(
            triggers[0],
            Trigger {
                only: Some(WeaponId(149)),
                ..Trigger::default()
            },
            "the higher's calls both fail"
        );
    }

    #[test]
    fn a_carrier_whose_bay_reloads_leaves_the_berth_to_the_next() {
        let mut traffic = Traffic::new();
        traffic.add_npc_at(carrier(), 1);
        let next = traffic.add_npc_at(carrier(), 2);
        traffic.npcs_mut()[0].armament.hold(WeaponId(149));
        let (rooms, triggers) = think(&mut traffic, 60);
        assert_eq!(rooms[1], (next, true));
        assert_eq!(triggers[1], sniping(true));
    }

    #[test]
    fn a_kept_trigger_takes_its_berth_too() {
        // A disabled ship decides nothing; a coasting one keeps its
        // trigger, whose launch the wait gate lets through.
        let mut traffic = Traffic::new();
        let coasting = Npc {
            mode: Mode::Coasting { ticks_left: 5 },
            trigger: sniping(true),
            ..carrier()
        };
        traffic.add_npc_at(coasting, 1);
        let later = traffic.add_npc_at(carrier(), 2);
        let (rooms, triggers) = think(&mut traffic, 60);
        assert_eq!(rooms, [(later, false)], "only the flying one asks");
        assert_eq!(triggers[0], sniping(true));
        assert!(!triggers[1].launches());
    }
}
