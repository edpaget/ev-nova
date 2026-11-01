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
//! substate, sub 0, 4 or 0xd ([`regroup`](crate::ai::regroup) says why no
//! other reaches the pause): the fly half flies its kept substate's goal
//! ([`Npc::fly_as_flown`]). In sub 0 its trigger and target come from the
//! behaviour's [`gate_wait`](crate::ai::Behaviour::gate_wait) ([`fresh`]),
//! settled against the launch room, a find kept as its pick, as a
//! decision's trigger is. Subs 4 and 0xd take the jump path, firing
//! nothing.
//!
//! None of this plays a sound.

use super::Traffic;
use super::npc::Npc;
use crate::ai::regroup::Kept;
use crate::ai::{Behaviour, GateWait, Goal, Surroundings, dispatch_gate};
use crate::chance::Chance;

/// The range of the pause's release draw, `trunc(100.0 / _gSpeedMult)` at
/// the engine's speed factor of 1: released on a 0.
pub const RELEASE_DRAW: u32 = 100;

impl Traffic {
    /// Each NPC in the regroup pause that passes the dispatch's gates draws
    /// its release, rolling on `chance`: released, it idles (see the module
    /// docs).
    pub(crate) fn release_regroups(&mut self, chance: &mut (impl Chance + ?Sized)) {
        for npc in self
            .npcs
            .iter_mut()
            .filter(|npc| paused(npc) && dispatch_gate::reaches_think(npc))
        {
            if chance.below(RELEASE_DRAW) == 0 {
                npc.goal = Goal::Idle;
            }
        }
    }
}

/// Whether `npc` is held in the regroup pause ([`Goal::Regroup`]).
#[must_use]
pub(super) fn paused(npc: &Npc) -> bool {
    matches!(npc.goal, Goal::Regroup { .. })
}

/// What a held `npc` holds on one tick (see the module docs): its gate
/// wait's in sub 0, as `behaviour` says among `around`; none on the jump
/// path.
pub(super) fn held_fire(
    behaviour: &(impl Behaviour + ?Sized),
    npc: &Npc,
    around: &Surroundings,
) -> GateWait {
    if matches!(npc.goal, Goal::Regroup { kept: Kept::Coast }) {
        behaviour.gate_wait(npc, around)
    } else {
        GateWait::default()
    }
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
    use crate::catalog::WeaponId;
    use crate::chance::NeverFires;
    use crate::cloak::Cloak;
    use crate::combat::ShipRef;
    use crate::combat::armament::Trigger;
    use crate::escort::FORMATION_NUDGE;
    use crate::flight::FlightRules;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::World;
    use crate::traffic::emerge::exit_speed;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{Mode, NpcId};
    use crate::traffic::ship_display::Thrust;
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
        let mut jumping = paused_with(Kept::OwnJump);
        jumping.warp.goal = Some(WarpGoal::Own);
        let gliding = Npc {
            mode: Mode::JumpingIn { tick: 0 },
            ..paused_with(Kept::Coast)
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
        let mut held = paused_with(Kept::Coast);
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

    // In its gate.

    /// A ship paused coasting (sub 0) where it waited in its gate, led out
    /// as `led_out` says, `ticks_left` of its wait left.
    fn paused_in_gate(ticks_left: u32, led_out: bool) -> Npc {
        Npc {
            mode: Mode::HeldAtGate { ticks_left },
            led_out,
            ..paused_with(Kept::Coast)
        }
    }

    #[test]
    fn paused_in_its_gate_it_never_bursts_and_owes_its_burst_once_its_wait_is_out() {
        for led_out in [false, true] {
            let mut traffic = Traffic::with_interval(1);
            let id = traffic.add_npc(paused_in_gate(3, led_out));
            let behaviour = Recording::default();
            for _ in 0..6 {
                ticked(&mut traffic, &behaviour, 1);
                assert!(
                    !matches!(of(&traffic, id).mode, Mode::LeavingGate { .. }),
                    "no burst"
                );
            }
            let out = of(&traffic, id);
            assert_eq!(out.mode, Mode::Flying);
            assert!(paused(out), "still paused");
            assert_eq!(out.owed_burst, Some(exit_speed(led_out, 0)));
            assert_eq!(out.state.position, Vec2::ZERO, "at its gate");
            assert!(behaviour.decided.borrow().is_empty());
        }
    }

    #[test]
    fn released_in_its_gate_it_decides_in_the_same_tick() {
        let mut traffic = Traffic::with_interval(1);
        let id = traffic.add_npc(paused_in_gate(30, false));
        let behaviour = Recording::default();
        let mut chance = Draws::of(&[]).keyed(RELEASE_DRAW, 0);
        traffic.tick_in(&behaviour, World::new(&[]), &[], &mut chance);
        assert_eq!(of(&traffic, id).goal, Goal::Idle);
        assert_eq!(*behaviour.decided.borrow(), [id]);
    }

    // On the jump path.

    /// A world of no stellars whose warp sound runs 364 Mac ticks.
    fn warp_world() -> World<'static> {
        let mut world = World::new(&[]);
        world.warp.durations = [364, 252];
        world
    }

    /// A lead at the centre at stage 0, facing east, moving east at 2 and
    /// glowing, and a follower paused on its jump at (300, 400), facing
    /// north at rest.
    fn following_a_lead() -> (Traffic, NpcId, NpcId) {
        let mut traffic = Traffic::with_interval(1);
        let mut lead = ship(0.0, 0.0);
        lead.goal = Goal::Hold;
        lead.state.velocity = Vec2::new(2.0, 0.0);
        lead.state.heading = 90.0;
        lead.display.glow = 20;
        let lead = traffic.add_npc(lead);
        let mut follower = paused_with(Kept::FollowsJump);
        follower.state.position = Vec2::new(300.0, 400.0);
        follower.leader = Some(lead);
        follower.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(lead)));
        let id = traffic.add_npc(follower);
        (traffic, lead, id)
    }

    /// The nudge `AIMaintainFormation` gives `npc` toward its slot on each
    /// axis: its thrust's.
    fn nudge(npc: &Npc) -> f32 {
        FlightRules::default().handling(npc.stats.handling).accel * FORMATION_NUDGE
    }

    #[test]
    fn a_kept_follow_takes_its_leads_velocity_and_heading_with_no_thrust_and_is_nudged() {
        let (mut traffic, lead, id) = following_a_lead();
        let behaviour = Recording::default();
        ticked(&mut traffic, &behaviour, 1);
        let lead_velocity = of(&traffic, lead).state.velocity;
        let follower = of(&traffic, id);
        assert_eq!(follower.state.velocity, lead_velocity);
        assert_eq!(
            follower.state.heading,
            FlightRules::default()
                .handling(follower.stats.handling)
                .turn_rate,
            "turning by its turn"
        );
        // Its slot is well off on both axes.
        let slot = follower.parent_word().expect("a slot");
        let step = nudge(follower);
        let off = follower.state.position - (Vec2::new(300.0, 400.0) + lead_velocity);
        let towards = |at: f32, to: f32| if at < to { step } else { -step };
        assert!((off.x - towards(300.0, slot.x)).abs() < 1e-3, "{off:?}");
        assert!((off.y - towards(400.0, slot.y)).abs() < 1e-3, "{off:?}");
        assert_eq!(follower.display.drive.thrust, Thrust::None);
        assert!(!follower.close_form, "sub 0xd isn't 0xc (@0x82f54)");
        assert!(paused(follower));
        assert_eq!(follower.display.glow, 19, "the lead's 20, faded");
        ticked(&mut traffic, &behaviour, 60);
        let follower = of(&traffic, id);
        assert_eq!(follower.state.heading, 90.0, "the lead's heading");
        assert_eq!(follower.warp.stage, 0.0, "the lead's isn't over 1.0");
        assert_eq!(follower.display.drive.thrust, Thrust::None);
    }

    #[test]
    fn a_kept_follow_sends_its_lead_nowhere() {
        // The push (@0x8dedf-0x8def0) is goal 0xb's: on the jump path goal
        // 0x13 runs the high level's head, never that block.
        let (mut traffic, lead, _) = following_a_lead();
        ticked(&mut traffic, &Recording::default(), 3);
        assert_eq!(of(&traffic, lead).goal, Goal::Hold);
    }

    #[test]
    fn double_times_second_pass_nudges_a_kept_follow_not() {
        let (mut traffic, _, id) = following_a_lead();
        ticked(&mut traffic, &Recording::default(), 1);
        let before = of(&traffic, id).state;
        traffic.move_in(World::new(&[]), &mut NeverFires);
        let after = of(&traffic, id).state;
        assert_eq!(after.position, before.position + before.velocity);
    }

    #[test]
    fn a_kept_own_jump_turns_outward_and_never_leaves() {
        let mut traffic = Traffic::with_interval(1);
        let mut own = paused_with(Kept::OwnJump);
        own.state.position = Vec2::new(30_000.0, 0.0);
        own.state.velocity = Vec2::new(0.0, 1.0);
        own.warp.goal = Some(WarpGoal::Own);
        let id = traffic.add_npc(own);
        // A release draw of 0 would release it.
        let mut chance = Draws::of(&[0; 16]).keyed(RELEASE_DRAW, 0);
        let behaviour = Recording::default();
        for tick in 0..300 {
            traffic.tick_in(&behaviour, warp_world(), &[], &mut chance);
            assert!(traffic.departed().is_empty(), "tick {tick}");
            let ship = of(&traffic, id);
            assert!(ship.warp.since <= 1, "tick {tick}: {}", ship.warp.since);
            assert_eq!(ship.state.velocity, Vec2::new(0.0, 1.0), "tick {tick}");
        }
        assert_eq!(chance.times_asked(RELEASE_DRAW), 0, "no release draw");
        let ship = of(&traffic, id);
        assert_eq!(
            ship.goal,
            Goal::Regroup {
                kept: Kept::OwnJump
            }
        );
        assert_eq!(ship.warp.goal, Some(WarpGoal::Own));
        assert_eq!(ship.warp.stage, 0.0, "the warp-out's not-jumping write");
        assert_eq!(ship.state.heading, 90.0, "outward");
        assert_eq!(ship.state.position.y, 300.0);
        assert_eq!(ship.display.drive.thrust, Thrust::None);
    }

    #[test]
    fn on_the_jump_path_it_fires_nothing_whatever_its_gate_wait_holds() {
        for (kept, warp) in [
            (Kept::FollowsJump, WarpGoal::Follow(ShipRef::Player)),
            (Kept::OwnJump, WarpGoal::Own),
        ] {
            let mut traffic = Traffic::with_interval(1);
            let mut held = paused_with(kept);
            held.warp.goal = Some(warp);
            held.target = Some(ShipRef::Player);
            held.trigger = MARKED;
            let id = traffic.add_npc(held);
            traffic.think_holding(&Recording::default(), World::new(&[]), &[], &mut NeverFires);
            let held = of(&traffic, id);
            assert_eq!(held.trigger, Trigger::default(), "{kept:?}");
            assert_eq!(held.target, Some(ShipRef::Player), "{kept:?}");
        }
    }
}
