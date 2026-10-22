//! The traffic's regroup pause, goal 0x13
//! ([`regroup`](crate::ai::regroup)): its release,
//! what it skips and what it fires.
//!
//! **The release** ([`Traffic::release_regroups`], `_AIDispatch`
//! @0x8fe73-0x8febb). Each ship in the pause whose dispatch passes the gates
//! ([`dispatch_gate::reaches_think`]) draws, every dispatch,
//! `Rand(trunc(100.0 / _gSpeedMult))` (doubles 0xdd0e0 = 100.0, 0xdd118 =
//! 0.0): [`RELEASE_DRAW`], as the engine's speed factor is always 1. At 0
//! its goal becomes 0 ([`Goal::Idle`]), its substate kept, and the dispatch
//! goes on: it thinks in the same tick. The think half runs the draws
//! after the leaders' orders and before the disabled reset, as the
//! dispatch does (@0x8fdb3, @0x8fe73, @0x8fece).
//!
//! **What it skips.** Held, a ship skips the disabled reset, the type
//! think and `_HighLevelAIHandler` (@0x9002d → @0x900c6 → @0x901ea): in
//! [`Traffic::think_holding`] it is never due ([`paused`]), so it decides
//! nothing and runs no high level, and no swarm check. Its cloak still
//! thinks, as `_AIHandleCloaking` comes first (@0x8fcaa).
//!
//! **What it flies and fires.** `_LowLevelAIHandler` runs its kept
//! substate: the fly half flies its kept substate's goal
//! ([`Npc::fly_as_flown`]), and its trigger and target come from the
//! behaviour's [`gate_wait`](crate::ai::Behaviour::gate_wait) ([`fresh`]),
//! settled against the launch room, a find kept as its pick, as a
//! decision's trigger is. A paused fight's frame is worked out afresh each
//! tick from where its target is, as the low level reads its target each
//! frame, the high level's stored frame being stale.
//!
//! None of this plays a sound.

use super::Traffic;
use super::npc::Npc;
use crate::ai::regroup::Kept;
use crate::ai::{Goal, dispatch_gate};
use crate::chance::Chance;

/// The range of the pause's release draw, `trunc(100.0 / _gSpeedMult)` at
/// the engine's speed factor of 1: released on a 0.
pub const RELEASE_DRAW: u32 = 100;

impl Traffic {
    /// Each NPC in the regroup pause that passes the dispatch's gates draws
    /// its release, rolling on `chance`: released, it idles (see the module
    /// docs); held, its fight's frame is worked out afresh.
    pub(crate) fn release_regroups(&mut self, chance: &mut (impl Chance + ?Sized)) {
        for npc in self
            .npcs
            .iter_mut()
            .filter(|npc| paused(npc) && dispatch_gate::reaches_think(npc))
        {
            if chance.below(RELEASE_DRAW) == 0 {
                npc.goal = Goal::Idle;
            } else if matches!(
                npc.goal,
                Goal::Regroup {
                    kept: Kept::Attack(_) | Kept::Snipe(_)
                }
            ) {
                npc.pursuit.frame = None;
            }
        }
    }
}

/// Whether `npc` is held in the regroup pause ([`Goal::Regroup`]).
#[must_use]
pub(super) fn paused(npc: &Npc) -> bool {
    matches!(npc.goal, Goal::Regroup { .. })
}

/// Whether each of `npcs` took its trigger this tick: each whose decision
/// is `due`, and each `picked` that is paused, which holds its gate wait
/// in place of a decision's ([`gate_wait`](crate::ai::Behaviour::gate_wait)),
/// whose find is then its pick.
pub(super) fn fresh(npcs: &[Npc], due: &[bool], picked: &[bool]) -> Vec<bool> {
    npcs.iter()
        .zip(due)
        .zip(picked)
        .map(|((npc, &due), &picked)| due || (picked && paused(npc)))
        .collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::ai::{Behaviour, GateWait, Surroundings};
    use crate::catalog::WeaponId;
    use crate::chance::NeverFires;
    use crate::cloak::Cloak;
    use crate::combat::ShipRef;
    use crate::combat::armament::Trigger;
    use crate::escort::KEEP_FORMATION;
    use crate::flight::{FlightRules, ShipState};
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::World;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{Mode, NpcId};
    use crate::traffic::warp_out::WarpGoal;
    use nova_sim_base::fixture::{Draws, FAST};

    /// Keeps every goal, logging the ships it decides for; while waiting
    /// or paused it holds [`MARKED`], dropping its target.
    #[derive(Debug, Default)]
    struct Recording {
        decided: RefCell<Vec<NpcId>>,
    }

    /// The trigger [`Recording`]'s gate wait holds.
    const MARKED: Trigger = Trigger {
        primary: false,
        secondary: None,
        only: Some(WeaponId(150)),
        bays: false,
        bays_after: false,
        point_defence: true,
        keep: Some(WeaponId(150)),
    };

    impl Behaviour for Recording {
        fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            self.decided.borrow_mut().push(npc.id);
            npc.goal
        }

        fn gate_wait(&self, _: &Npc, _: &Surroundings) -> GateWait {
            GateWait {
                trigger: MARKED,
                drops_target: true,
            }
        }
    }

    /// A [`FAST`] ship at rest at `(x, y)`.
    fn ship(x: f32, y: f32) -> Npc {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.state.position = Vec2::new(x, y);
        ship
    }

    /// A ship at the centre paused, keeping `kept`.
    fn paused_with(kept: Kept) -> Npc {
        Npc {
            goal: Goal::Regroup { kept },
            ..ship(0.0, 0.0)
        }
    }

    /// `traffic` ticked `ticks` times by `behaviour`, never released.
    fn ticked(traffic: &mut Traffic, behaviour: &impl Behaviour, ticks: usize) {
        for _ in 0..ticks {
            traffic.tick_in(behaviour, World::new(&[]), &[], &mut NeverFires);
        }
    }

    fn of(traffic: &Traffic, id: NpcId) -> &Npc {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("there")
    }

    // The release and what the pause skips.

    #[test]
    fn released_on_a_0_it_idles_and_decides_in_the_same_tick() {
        let mut traffic = Traffic::with_interval(1);
        let id = traffic.add_npc(paused_with(Kept::Coast));
        let behaviour = Recording::default();
        let mut chance = Draws::of(&[]).keyed(RELEASE_DRAW, 0);
        traffic.tick_in(&behaviour, World::new(&[]), &[], &mut chance);
        assert_eq!(chance.times_asked(RELEASE_DRAW), 1);
        assert_eq!(of(&traffic, id).goal, Goal::Idle);
        assert_eq!(*behaviour.decided.borrow(), [id]);
    }

    #[test]
    fn held_it_stays_paused_and_decides_nothing_but_its_cloak_thinks() {
        let mut traffic = Traffic::with_interval(1);
        let mut cloaked = paused_with(Kept::Coast);
        cloaked.cloak.cloak();
        let id = traffic.add_npc(cloaked);
        let behaviour = Recording::default();
        let mut chance = Draws::of(&[]).keyed(RELEASE_DRAW, 99);
        traffic.think_holding(&behaviour, World::new(&[]), &[], &mut chance);
        assert_eq!(chance.times_asked(RELEASE_DRAW), 1);
        let held = of(&traffic, id);
        assert_eq!(held.goal, Goal::Regroup { kept: Kept::Coast });
        assert!(behaviour.decided.borrow().is_empty());
        let mut uncloaking = Cloak::default();
        uncloaking.cloak();
        uncloaking.uncloak();
        assert_eq!(held.cloak, uncloaking, "no device: its think uncloaks it");
    }

    #[test]
    fn a_paused_ship_past_the_dispatchs_gates_draws_nothing() {
        let mut jumping = paused_with(Kept::Stop);
        jumping.warp.goal = Some(WarpGoal::Own);
        let gliding = Npc {
            mode: Mode::JumpingIn { tick: 0 },
            ..paused_with(Kept::Stop)
        };
        for ship in [jumping, gliding] {
            let mut traffic = Traffic::with_interval(1);
            let id = traffic.add_npc(ship);
            let mut chance = Draws::of(&[]).keyed(RELEASE_DRAW, 0);
            traffic.think_holding(&Recording::default(), World::new(&[]), &[], &mut chance);
            assert_eq!(chance.times_asked(RELEASE_DRAW), 0);
            assert!(paused(of(&traffic, id)));
        }
    }

    #[test]
    fn held_it_runs_no_swarm_check() {
        let mut swarmer = paused_with(Kept::Coast);
        swarmer.hull.swarms = true;
        swarmer.swarm_leader = Some(9);
        let mut traffic = Traffic::with_interval(1);
        let id = traffic.add_npc_at(swarmer, 5);
        traffic.think_holding(&Recording::default(), World::new(&[]), &[], &mut NeverFires);
        assert_eq!(of(&traffic, id).swarm_leader, Some(9));
        // Released, it checks: no swarm leader above its berth.
        let mut chance = Draws::of(&[]).keyed(RELEASE_DRAW, 0);
        traffic.think_holding(&Recording::default(), World::new(&[]), &[], &mut chance);
        assert_eq!(of(&traffic, id).swarm_leader, None);
    }

    #[test]
    fn held_it_fires_what_its_gate_wait_holds_keeping_the_find_as_its_pick() {
        let mut traffic = Traffic::with_interval(1);
        let mut held = paused_with(Kept::Formation {
            guard: Some(ShipRef::Player),
        });
        held.target = Some(ShipRef::Player);
        let id = traffic.add_npc(held);
        traffic.think_holding(&Recording::default(), World::new(&[]), &[], &mut NeverFires);
        let held = of(&traffic, id);
        assert_eq!(held.trigger, MARKED);
        assert_eq!(held.pick, Some(WeaponId(150)));
        assert_eq!(held.target, None, "dropped as its gate wait says");
    }

    #[test]
    fn a_ship_neither_deciding_nor_paused_keeps_its_pick() {
        // Its kept trigger's find is no new pick: the pick a fired weapon
        // cleared stays clear.
        let mut traffic = Traffic::with_interval(1);
        let mut down = ship(0.0, 0.0);
        down.condition = crate::combat::hull::Condition::Disabled;
        down.trigger = MARKED;
        down.pick = None;
        let id = traffic.add_npc(down);
        traffic
            .npcs_mut()
            .iter_mut()
            .find(|npc| npc.id == id)
            .expect("there")
            .pick = None;
        traffic.think_holding(&Recording::default(), World::new(&[]), &[], &mut NeverFires);
        assert_eq!(of(&traffic, id).pick, None);
    }

    // What it flies.

    #[test]
    fn a_kept_formation_keeps_its_slot_in_its_new_leads_formation() {
        let mut traffic = Traffic::with_interval(1);
        let mut lead = ship(0.0, 0.0);
        lead.goal = Goal::Hold;
        lead.state.velocity = Vec2::new(2.0, 0.0);
        let lead_id = traffic.add_npc(lead);
        traffic.form_fleets();
        let mut follower = paused_with(Kept::Formation { guard: None });
        follower.leader = Some(lead_id);
        let id = traffic.add_npc(follower);
        traffic.form_fleets();
        let slot = of(&traffic, id).fleet_slot.expect("a slot");
        let lead_state = of(&traffic, lead_id).state;
        let follower = traffic.npcs_mut().iter_mut().find(|npc| npc.id == id);
        let follower = follower.expect("there");
        follower.state = ShipState {
            position: slot.position(&lead_state),
            ..lead_state
        };
        ticked(&mut traffic, &Recording::default(), 60);
        let lead = of(&traffic, lead_id).state;
        let follower = of(&traffic, id);
        assert!(paused(follower));
        let off = slot.position(&lead) - follower.state.position;
        assert!(off.length() < 1.0, "{off:?}");
        assert!(follower.keeps_close_formation(Some(&lead), true, FlightRules::default()));
        assert!(off.length() < KEEP_FORMATION);
    }

    #[test]
    fn a_kept_coast_moves_on_with_no_turn_or_thrust() {
        let mut traffic = Traffic::with_interval(1);
        let mut coasting = paused_with(Kept::Coast);
        coasting.state.velocity = Vec2::new(1.0, 2.0);
        coasting.state.heading = 30.0;
        let id = traffic.add_npc(coasting);
        ticked(&mut traffic, &Recording::default(), 1);
        let state = of(&traffic, id).state;
        assert_eq!(state.velocity, Vec2::new(1.0, 2.0));
        assert_eq!(state.heading, 30.0);
        assert_eq!(state.position, Vec2::new(1.0, 2.0));
    }

    #[test]
    fn a_kept_stop_brakes() {
        let mut traffic = Traffic::with_interval(1);
        let mut stopping = paused_with(Kept::Stop);
        stopping.state.velocity = Vec2::new(2.0, 0.0);
        stopping.state.heading = 270.0;
        let id = traffic.add_npc(stopping);
        ticked(&mut traffic, &Recording::default(), 1);
        assert!(of(&traffic, id).state.velocity.x < 2.0);
    }

    #[test]
    fn a_kept_attack_closes_on_its_target() {
        let mut traffic = Traffic::with_interval(1);
        let target = traffic.add_npc(ship(0.0, -1000.0));
        let mut attacker = paused_with(Kept::Attack(ShipRef::Npc(target)));
        attacker.target = Some(ShipRef::Npc(target));
        let id = traffic.add_npc(attacker);
        ticked(&mut traffic, &Recording::default(), 30);
        let off = of(&traffic, target).state.position - of(&traffic, id).state.position;
        assert!(off.length() < 950.0, "{off:?}");
        assert!(paused(of(&traffic, id)));
    }
}
