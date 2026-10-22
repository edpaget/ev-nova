//! Escorts and fighters on their leader's jump: `_AIDispatch`'s jump path
//! and its cancels, `_EscortAI`'s entry into goal 0xb, and the high
//! level's fighter turn-back.
//!
//! **The jump path** (`_AIDispatch` @0x8fd83-0x8fd9a → @0x900d5): a ship
//! in sub 4 or 0xd ([`dispatch_gate::takes_jump_path`]) skips the
//! leader's orders, the think throttle, the regroup draw, the disabled
//! reset and its type's think, `_EscortAI` included. So
//! [`Traffic::think_holding`] makes it no decision: its goal and target
//! stay as its jump left them. Its high level and low level still run.
//!
//! **Goal 0xb** is a ship following its leader's jump
//! ([`Npc::follows_leaders_jump`]): sub 0xd's warp goal
//! ([`WarpGoal::Follow`]) under a goal other than its own jump's, a
//! flee's or the regroup pause's.
//!
//! **The high level's AI wait** (`_HighLevelAIHandler` @0x8d4b9-0x8d4d2):
//! with +0x4c above 0 and a goal other than 0xa and 0xe, the high level
//! returns at once ([`Npc::high_level_waits`]): no hold
//! ([`warp_out::hold_stage`]), fighter turn-back, goal 0xb check
//! ([`warp_out::follow_check`]) or push ([`warp_out::send_leaders_out`]).
//! So through sub 0xd's 180 wait (@0x85bc3) a follower keeps sub 0xd, and
//! a stage it starts runs past 30.0 to the detach (@0x85a8f).

use crate::ai::Goal;
#[cfg(doc)]
use crate::ai::dispatch_gate;
use crate::ai::leave::{self, Leg};
#[cfg(doc)]
use crate::traffic::Traffic;
use crate::traffic::npc::Npc;
#[cfg(doc)]
use crate::traffic::warp_out;
use crate::traffic::warp_out::WarpGoal;

impl Npc {
    /// Whether it is in goal 0xb, following its leader's jump: sub 0xd's
    /// warp goal ([`WarpGoal::Follow`]) under a goal other than its own
    /// jump's or a flee's (goals 2 and 3, which the hold writes) or the
    /// regroup pause's (0x13).
    #[must_use]
    pub fn follows_leaders_jump(&self) -> bool {
        matches!(self.warp.goal, Some(WarpGoal::Follow(_)))
            && !matches!(
                self.goal,
                Goal::JumpOut | Goal::Flee(_) | Goal::Regroup { .. }
            )
    }

    /// Whether its high level returns at once for its AI wait
    /// (`_HighLevelAIHandler` @0x8d4b9-0x8d4d2): +0x4c above 0 and a goal
    /// other than 0xa ([`Goal::flies_in_form`]) and 0xe ([`Goal::Drift`]).
    /// Goal 0xb ([`Npc::follows_leaders_jump`]) is neither, whatever goal
    /// the engine keeps beside it.
    #[must_use]
    pub fn high_level_waits(&self) -> bool {
        self.ai_wait > 0
            && (self.follows_leaders_jump()
                || !(self.goal.flies_in_form() || self.goal == Goal::Drift))
    }
}

/// Whether `npc`, a leader, is on its way out by its substate, as goal
/// 0xb's NPC-parent test reads it (sub 1 or 4, @0x8defc-0x8df13): on its
/// own jump (sub 4), or in goal 2 braking before it ([`Leg::Brake`]). A
/// leader running out inside 1000 (sub 3), or itself following a jump
/// (sub 0xd), is not, so its followers give it up.
#[must_use]
pub fn leaves_by_substate(npc: &Npc) -> bool {
    npc.warp.goal == Some(WarpGoal::Own)
        || (npc.goal == Goal::JumpOut
            && npc.warp.goal.is_none()
            && leave::leg(&npc.state, leave::expedites(npc)) == Leg::Brake)
}

/// Whether each of `npcs`, whose think is `due`, makes its type's think:
/// none on the jump path, sub 4 or 0xd (see the module docs). Its high
/// level, its board wait's lapse, its plunder drift and its fresh trigger
/// still read `due`, as `_AIDispatch` runs the high level on the jump path.
pub(super) fn deciding(npcs: &[Npc], due: &[bool]) -> Vec<bool> {
    npcs.iter()
        .zip(due)
        .map(|(npc, &due)| due && npc.warp.goal.is_none())
        .collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::{Behaviour, Surroundings};
    use crate::chance::{Chance, NeverFires};
    use crate::combat::ShipRef;
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{Npc, NpcId};
    use crate::traffic::warp_out::WarpGoal;
    use crate::traffic::{Traffic, World};
    use nova_sim_base::fixture::FAST;

    /// Every ship keeps its goal and target.
    #[derive(Debug)]
    struct Keep;

    impl Behaviour for Keep {
        fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    /// Every ship it decides for attacks the player, its target.
    #[derive(Debug)]
    struct Attacks;

    impl Behaviour for Attacks {
        fn decide(&self, _: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            Goal::Attack(ShipRef::Player)
        }

        fn target(&self, _: &Npc, _: &Surroundings) -> Option<ShipRef> {
            Some(ShipRef::Player)
        }
    }

    fn of(traffic: &Traffic, id: NpcId) -> &Npc {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("there")
    }

    /// A ship 3000 out east, at rest.
    fn far(id: u32) -> Npc {
        let mut ship = npc(id, ShipStats::new(FAST, &[]));
        ship.state = ShipState {
            position: Vec2::new(3000.0, 0.0),
            velocity: Vec2::ZERO,
            heading: 90.0,
        };
        ship
    }

    /// A world of no stellars, the warp sound 364 Mac ticks long, the
    /// player's stage at `stage`.
    fn jumping_world(stage: f32) -> World<'static> {
        let mut world = World::new(&[]);
        world.warp.durations = [364, 252];
        world.warp.player.stage = stage;
        world
    }

    // Goal 0xb and the high level's wait.

    #[test]
    fn goal_0xb_is_a_follow_of_a_leaders_jump_under_any_goal_but_2_3_and_0x13() {
        let mut ship = far(1);
        assert!(!ship.follows_leaders_jump(), "no warp goal");
        ship.warp.goal = Some(WarpGoal::Own);
        assert!(!ship.follows_leaders_jump(), "sub 4");
        ship.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(NpcId(4))));
        for (goal, follows) in [
            (Goal::Idle, true),
            (Goal::Formation { guard: None }, true),
            (Goal::JumpOut, false),
            (Goal::Flee(ShipRef::Player), false),
            (
                Goal::Regroup {
                    kept: crate::ai::regroup::Kept::Coast,
                },
                false,
            ),
        ] {
            ship.goal = goal;
            assert_eq!(ship.follows_leaders_jump(), follows, "{goal:?}");
        }
    }

    #[test]
    fn the_high_level_waits_on_its_ai_wait_but_in_goals_0xa_and_0xe() {
        let follow = Goal::Follow {
            lead: NpcId(4),
            guard: None,
        };
        let waits = |goal: Goal, ai_wait: u32, warp: Option<WarpGoal>| {
            let mut ship = far(1);
            ship.goal = goal;
            ship.ai_wait = ai_wait;
            ship.warp.goal = warp;
            ship.high_level_waits()
        };
        assert!(!waits(Goal::Idle, 0, None), "no wait");
        assert!(waits(Goal::Idle, 1, None));
        assert!(waits(Goal::JumpOut, 1, Some(WarpGoal::Own)));
        for goal in [
            follow,
            Goal::Formation { guard: None },
            Goal::Hold,
            Goal::Drift,
        ] {
            assert!(!waits(goal, 1, None), "{goal:?}: 0xa or 0xe");
        }
        let jump = Some(WarpGoal::Follow(ShipRef::Npc(NpcId(4))));
        assert!(waits(follow, 1, jump), "goal 0xb is neither");
        assert!(!waits(follow, 0, jump));
    }

    /// A lead at the centre, and its follower 3000 out east on its jump,
    /// facing out, its stage started `since` ticks ago, waiting `ai_wait`.
    fn following(since: u32, ai_wait: u32) -> (Traffic, NpcId, NpcId) {
        let mut traffic = Traffic::with_interval(1);
        let mut lead = npc(0, ShipStats::new(FAST, &[]));
        lead.state.position = Vec2::new(0.0, 0.0);
        let lead = traffic.add_npc(lead);
        let mut follower = far(0);
        follower.leader = Some(lead);
        follower.goal = Goal::Follow { lead, guard: None };
        follower.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(lead)));
        follower.warp.stage = 3.0;
        follower.warp.since = since;
        follower.warp.aim = 90;
        follower.ai_wait = ai_wait;
        let follower = traffic.add_npc(follower);
        (traffic, lead, follower)
    }

    #[test]
    fn a_follower_whose_stage_started_leaves_on_its_own_leg_and_clock() {
        let (mut traffic, _, follower) = following(137, 0);
        let world = jumping_world(0.0);
        traffic.tick_in(&Keep, world, &[], &mut NeverFires);
        let ship = of(&traffic, follower);
        assert_eq!(ship.goal, Goal::JumpOut, "goal 2 at its next high level");
        assert_eq!(ship.warp.goal, Some(WarpGoal::Own), "its leave's sub 4");
        assert!(
            ship.warp.stage > 1.0,
            "its stage runs on: {}",
            ship.warp.stage
        );
        traffic.tick_in(&Keep, world, &[], &mut NeverFires);
        assert!(traffic.departed().is_empty(), "139 ticks on its clock");
        traffic.tick_in(&Keep, world, &[], &mut NeverFires);
        assert_eq!(
            traffic.departed(),
            &[(follower, crate::traffic::Outcome::JumpedOut)],
            "140 ticks: 364 / 1.3 Mac ticks from sub 0xd's stamp"
        );
    }

    #[test]
    fn a_follower_in_its_ai_wait_keeps_its_leaders_jump() {
        let (mut traffic, lead, follower) = following(0, 100);
        traffic.tick_in(&Keep, jumping_world(0.0), &[], &mut NeverFires);
        let ship = of(&traffic, follower);
        assert_eq!(ship.goal, Goal::Follow { lead, guard: None }, "no hold");
        assert_eq!(ship.warp.goal, Some(WarpGoal::Follow(ShipRef::Npc(lead))));
    }

    // A leader's leave by its substate.

    /// Whether a follower of a lead that `setup` set, at rest 3000 out
    /// east, keeps following it through a tick.
    fn keeps_following(setup: impl Fn(&mut Npc)) -> bool {
        let mut traffic = Traffic::with_interval(1);
        let mut lead = far(0);
        setup(&mut lead);
        let lead = traffic.add_npc(lead);
        let mut follower = far(0);
        follower.state.position = Vec2::new(3000.0, 100.0);
        follower.leader = Some(lead);
        follower.goal = Goal::Follow { lead, guard: None };
        follower.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(lead)));
        let follower = traffic.add_npc(follower);
        traffic.tick_in(&Keep, jumping_world(0.0), &[], &mut NeverFires);
        of(&traffic, follower).warp.goal == Some(WarpGoal::Follow(ShipRef::Npc(lead)))
    }

    #[test]
    fn a_leader_braking_or_on_its_own_jump_keeps_its_followers() {
        assert!(keeps_following(|lead| {
            lead.goal = Goal::JumpOut;
            lead.state.velocity = Vec2::new(2.0, 0.0);
        }));
        assert!(keeps_following(Npc::begin_own_jump));
    }

    #[test]
    fn a_leader_running_out_inside_1000_or_following_another_gives_its_followers_up() {
        assert!(!keeps_following(|lead| {
            lead.goal = Goal::JumpOut;
            lead.state.position = Vec2::new(500.0, 0.0);
        }));
        assert!(!keeps_following(|lead| {
            lead.goal = Goal::Follow {
                lead: NpcId(77),
                guard: None,
            };
            lead.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(NpcId(77))));
        }));
        assert!(!keeps_following(|lead| lead.goal = Goal::Idle));
    }

    #[test]
    fn a_leaders_leave_reads_sub_4_or_goal_2s_brake() {
        let mut lead = far(1);
        assert!(!leaves_by_substate(&lead), "idle");
        lead.goal = Goal::JumpOut;
        assert!(
            !leaves_by_substate(&lead),
            "at rest beyond 1000: sub 4 next"
        );
        lead.state.velocity = Vec2::new(0.0, 0.4);
        assert!(leaves_by_substate(&lead), "sub 1");
        lead.stats.fast_jump = true;
        assert!(
            !leaves_by_substate(&lead),
            "an expediting ship doesn't brake"
        );
        lead.warp.goal = Some(WarpGoal::Own);
        assert!(leaves_by_substate(&lead), "sub 4");
        lead.goal = Goal::Flee(ShipRef::Player);
        assert!(leaves_by_substate(&lead), "goal 3's sub 4");
        let mut inside = far(1);
        inside.goal = Goal::JumpOut;
        inside.state.position = Vec2::new(500.0, 0.0);
        inside.state.velocity = Vec2::new(2.0, 0.0);
        assert!(!leaves_by_substate(&inside), "sub 3");
    }

    // The jump path skips the type think.

    #[test]
    fn a_ship_on_a_jump_makes_no_type_think_and_one_off_it_does() {
        let mut traffic = Traffic::with_interval(1);
        let mut own = far(0);
        own.begin_own_jump();
        let own = traffic.add_npc(own);
        let mut follower = far(0);
        follower.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        let follower = traffic.add_npc(follower);
        let free = traffic.add_npc(far(0));
        let world = jumping_world(1.0);
        traffic.tick_in(&Attacks, world, &[], &mut NeverFires);
        let own = of(&traffic, own);
        assert_eq!((own.goal, own.target), (Goal::JumpOut, None), "sub 4");
        let follower = of(&traffic, follower);
        assert_eq!(
            (follower.goal, follower.target),
            (Goal::Idle, None),
            "sub 0xd"
        );
        let free = of(&traffic, free);
        assert_eq!(
            (free.goal, free.target),
            (Goal::Attack(ShipRef::Player), Some(ShipRef::Player))
        );
    }
}
