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
//!
//! **The entry** (`_EscortAI` @0x839fd-0x83a92, @0x83be3-0x83c51). A
//! ship's think decides it follows its leader's jump
//! ([`Decision::follows_jump`]): an NPC's escort while its lead
//! [draws its followers](draws_followers)
//! ([`fleet_escort`](crate::ai::fleet_escort)), and every ship the player
//! leads while the player's +0x50 is above 0, its brake included
//! ([`EscortAi`](crate::ai::EscortAi)). Its target goes, and right after
//! the goals are taken ([`enter`]) it follows the jump (sub 0xd,
//! [`Npc::follow_leaders_jump`]), or, an NPC's escort that flies
//! inertialess ([`Npc::flies_inertialess`]), leaves its fleet to jump on
//! its own ([`Npc::detach_for_own_jump`], +0x50 = 0).
//!
//! **`_EscortAI`'s -1.0 writes.** Its Defend, Attack and formation blocks
//! write +0x50 = -1.0 beside +0x4c (@0x83c6b-0x83c73, @0x83d62-0x83d6a,
//! @0x83ea8-0x83eb0), whoever the parent, the player or an NPC, and its
//! lost parent writes it too (@0x83974). The traffic gives up the stage
//! ([`Warp::give_up`](crate::traffic::warp_out::Warp::give_up)) as it takes
//! such a decision ([`Decision::ends_wait`], [`Decision::leaves_fleet`]),
//! for the player's escorts and the NPCs' alike. The hold then zeroes it
//! in the same tick (@0x8d513), as the original's does in the same
//! dispatch, so an escort rests at 0 between ticks.

use crate::ai::leave::{self, Leg};
#[cfg(doc)]
use crate::ai::{Decision, dispatch_gate};
use crate::ai::{Goal, sight};
use crate::combat::ShipRef;
use crate::flight::FlightRules;
#[cfg(doc)]
use crate::traffic::Traffic;
use crate::traffic::npc::{Npc, NpcId};
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

    /// It follows `leader`'s jump, sub 0xd: the one writer of
    /// [`WarpGoal::Follow`], letting go of its target (`_EscortAI`
    /// @0x83a71, `_MakeEscortsHyperWithParent` @0x3fd89-0x3fdd0).
    pub fn follow_leaders_jump(&mut self, leader: ShipRef) {
        self.warp.goal = Some(WarpGoal::Follow(leader));
        self.target = None;
    }

    /// It leaves its fleet to jump on its own: no leader, its class's AI
    /// type ([`Npc::leave_fleet`]), goal 2 and sub 4 (`_EscortAI`
    /// @0x83c11-0x83c51, sub 0xd's detach @0x85ab6-0x85af1). Its stage
    /// and desired heading are its caller's.
    pub fn detach_for_own_jump(&mut self) {
        self.leave_fleet();
        self.goal = Goal::JumpOut;
        self.warp.goal = Some(WarpGoal::Own);
    }
}

/// Whether `lead` draws its escorts onto its jump (`_EscortAI`
/// @0x83a1a-0x83a5f): in sub 4 or 0xd, in goal 2 braking for its jump
/// (sub 1, [`Leg::Brake`]), or with its +0x50 above 0. A lead running out
/// inside 1000 (sub 3) draws no one: its escorts keep formation until it
/// brakes or jumps.
#[must_use]
pub fn draws_followers(lead: &Npc) -> bool {
    lead.warp.goal.is_some() || brakes_for_jump(lead) || lead.warp.stage > 0.0
}

/// Whether `npc` is in goal 2 with no warp goal, braking for its jump
/// (sub 1).
fn brakes_for_jump(npc: &Npc) -> bool {
    npc.goal == Goal::JumpOut
        && npc.warp.goal.is_none()
        && leave::leg(&npc.state, leave::expedites(npc)) == Leg::Brake
}

/// How a ship whose think decided to follow its leader's jump enters it
/// (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    /// It follows the jump of this leader.
    Follow(ShipRef),
    /// It leaves its fleet to jump on its own.
    Detach,
}

/// The ships among `decisions` (one an NPC) that decided to follow their
/// leader's jump ([`Decision::follows_jump`]), by their place.
pub(super) fn entering(decisions: &[Option<crate::ai::Decision>]) -> Vec<usize> {
    decisions
        .iter()
        .enumerate()
        .filter(|(_, decision)| decision.is_some_and(|decision| decision.follows_jump))
        .map(|(at, _)| at)
        .collect()
}

/// The ships of `npcs` at `entering` enter their leader's jump under
/// `flight` (see the module docs).
pub(super) fn enter(npcs: &mut [Npc], entering: &[usize], flight: FlightRules) {
    let entries: Vec<(usize, Entry)> = entering
        .iter()
        .filter_map(|&at| entry(&npcs[at], npcs, flight).map(|entry| (at, entry)))
        .collect();
    for (at, entry) in entries {
        let npc = &mut npcs[at];
        match entry {
            Entry::Follow(leader) => npc.follow_leaders_jump(leader),
            Entry::Detach => {
                npc.detach_for_own_jump();
                npc.warp.stage = 0.0;
            }
        }
    }
}

/// How `npc` enters its leader's jump among `npcs` under `flight`: the
/// player's when the player leads it; else its NPC lead's, or a detach
/// when it flies inertialess beside it; none with no lead there.
fn entry(npc: &Npc, npcs: &[Npc], flight: FlightRules) -> Option<Entry> {
    if npc.escorts_player() {
        return Some(Entry::Follow(ShipRef::Player));
    }
    let lead = npc
        .leader
        .and_then(|id: NpcId| npcs.iter().find(|other| other.id == id))?;
    Some(
        if npc.flies_inertialess(Some(&lead.state), sight::npc_sees(npc, lead), flight) {
            Entry::Detach
        } else {
            Entry::Follow(ShipRef::Npc(lead.id))
        },
    )
}

/// Whether `npc`, a leader, is on its way out by its substate, as goal
/// 0xb's NPC-parent test reads it (sub 1 or 4, @0x8defc-0x8df13): on its
/// own jump (sub 4), or in goal 2 braking before it ([`Leg::Brake`]). A
/// leader running out inside 1000 (sub 3), or itself following a jump
/// (sub 0xd), is not, so its followers give it up.
#[must_use]
pub fn leaves_by_substate(npc: &Npc) -> bool {
    npc.warp.goal == Some(WarpGoal::Own) || brakes_for_jump(npc)
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

    // The entry.

    #[test]
    fn a_lead_draws_its_followers_on_a_jump_braking_for_one_or_under_way() {
        let mut lead = far(1);
        assert!(!draws_followers(&lead), "idle");
        lead.goal = Goal::JumpOut;
        lead.state.position = Vec2::new(500.0, 0.0);
        lead.state.velocity = Vec2::new(2.0, 0.0);
        assert!(!draws_followers(&lead), "sub 3");
        lead.state.position = Vec2::new(3000.0, 0.0);
        assert!(draws_followers(&lead), "sub 1");
        let mut own = far(1);
        own.begin_own_jump();
        assert!(draws_followers(&own), "sub 4");
        let mut following = far(1);
        following.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        assert!(draws_followers(&following), "sub 0xd");
        let mut under_way = far(1);
        under_way.warp.stage = 0.5;
        assert!(draws_followers(&under_way), "+0x50 above 0");
        under_way.warp.stage = 0.0;
        assert!(!draws_followers(&under_way));
    }

    /// A lead NPC 0, 3000 out, and the ship it leads beside it at
    /// (3000, 300), targeting the player, inertialess when `inertialess`,
    /// a fighter of the lead when `fighter`.
    fn led_pair(inertialess: bool, fighter: bool) -> Vec<Npc> {
        let mut follower = far(1);
        follower.state.position = Vec2::new(3000.0, 300.0);
        follower.leader = Some(NpcId(0));
        follower.goal = Goal::Follow {
            lead: NpcId(0),
            guard: None,
        };
        follower.target = Some(ShipRef::Player);
        follower.hull.inherent_ai = 3;
        follower.stats.inertialess_hull = inertialess;
        if fighter {
            follower.carrier = Some(crate::bay::Carrier {
                ship: ShipRef::Npc(NpcId(0)),
                window: 100.0,
                reach: 40.0,
            });
        }
        vec![far(0), follower]
    }

    #[test]
    fn an_npcs_escort_enters_its_leads_jump() {
        let mut npcs = led_pair(false, false);
        npcs[1].warp.stage = 0.5;
        enter(&mut npcs, &[1], crate::flight::FlightRules::default());
        assert_eq!(
            npcs[1].warp.goal,
            Some(WarpGoal::Follow(ShipRef::Npc(NpcId(0))))
        );
        assert_eq!(npcs[1].target, None);
        assert_eq!(npcs[1].leader, Some(NpcId(0)));
        assert_eq!(npcs[1].warp.stage, 0.5, "no +0x50 write");
        assert_eq!(npcs[0].warp.goal, None, "only the ships entering");
    }

    #[test]
    fn a_ship_the_player_leads_enters_the_players_jump() {
        let mut npcs = led_pair(true, false);
        npcs[1].leader = None;
        npcs[1].escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        enter(&mut npcs, &[1], crate::flight::FlightRules::default());
        assert_eq!(npcs[1].warp.goal, Some(WarpGoal::Follow(ShipRef::Player)));
        assert_eq!(npcs[1].target, None);
    }

    #[test]
    fn an_inertialess_npcs_escort_leaves_its_fleet_to_jump_on_its_own() {
        for fighter in [false, true] {
            let mut npcs = led_pair(true, fighter);
            npcs[1].warp.stage = 0.5;
            enter(&mut npcs, &[1], crate::flight::FlightRules::default());
            let led = &npcs[1];
            assert_eq!(led.leader, None, "+0x9a -1 (@0x83c11)");
            assert_eq!(led.carrier, None, "{fighter}");
            assert_eq!(
                led.ai_type,
                crate::traffic::npc::AiType::Warship,
                "its class's InherentAI"
            );
            assert_eq!(
                (led.goal, led.warp.goal),
                (Goal::JumpOut, Some(WarpGoal::Own))
            );
            assert_eq!(led.warp.stage, 0.0, "+0x50 0");
        }
    }

    #[test]
    fn a_think_that_follows_a_jump_enters_it_in_the_same_tick() {
        let mut traffic = Traffic::with_interval(1);
        let mut lead = far(0);
        lead.begin_own_jump();
        let lead = traffic.add_npc(lead);
        let mut escort = far(0);
        escort.state.position = Vec2::new(3000.0, 300.0);
        escort.leader = Some(lead);
        escort.target = Some(ShipRef::Player);
        let escort = traffic.add_npc(escort);
        traffic.tick_in(
            &crate::ai::FleetEscortAi::default(),
            jumping_world(0.0),
            &[],
            &mut NeverFires,
        );
        let ship = of(&traffic, escort);
        assert_eq!(ship.warp.goal, Some(WarpGoal::Follow(ShipRef::Npc(lead))));
        assert_eq!(ship.goal, Goal::Follow { lead, guard: None });
        assert_eq!(ship.target, None);
    }

    #[test]
    fn every_ship_the_player_leads_takes_its_jump_from_its_brake_on() {
        // A hired escort fighting, and a launched fighter, routed by
        // `NovaAi` to `EscortAi`.
        let duty = || {
            Some(crate::escort::EscortDuty {
                formation: crate::escort::FormationSlot::default(),
                order: Some(crate::escort::EscortOrder::Attack),
            })
        };
        let entered = |stage: f32| {
            let mut traffic = Traffic::with_interval(1);
            let raider = traffic.add_npc(far(0));
            let mut hired = far(0);
            hired.escort = duty();
            hired.goal = Goal::Attack(ShipRef::Npc(raider));
            hired.target = Some(ShipRef::Npc(raider));
            let hired = traffic.add_npc(hired);
            let mut fighter = far(0);
            fighter.escort = duty();
            fighter.carrier = Some(crate::bay::Carrier {
                ship: ShipRef::Player,
                window: 100.0,
                reach: 40.0,
            });
            fighter.target = Some(ShipRef::Npc(raider));
            let fighter = traffic.add_npc(fighter);
            let mut world = jumping_world(stage);
            world.player = Some(crate::ai::fixture::player(0.0, 0.0));
            traffic.tick_in(&crate::ai::NovaAi::default(), world, &[], &mut NeverFires);
            [hired, fighter].map(|id| {
                let ship = of(&traffic, id);
                (ship.warp.goal, ship.target)
            })
        };
        for (ship, target) in entered(1.0) {
            assert_eq!(ship, Some(WarpGoal::Follow(ShipRef::Player)));
            assert_eq!(target, None);
        }
        for (ship, target) in entered(0.0) {
            assert_eq!(ship, None);
            assert!(target.is_some());
        }
    }

    // `_EscortAI`'s -1.0 writes.

    /// The stages of the ships `ships` sets up, after the think half and
    /// then after the fly half of a tick that `behaviour` decides.
    fn stages(behaviour: &impl Behaviour, ships: Vec<Npc>) -> (Vec<f32>, Vec<f32>) {
        let mut traffic = Traffic::with_interval(1);
        for ship in ships {
            traffic.add_npc(ship);
        }
        let mut world = jumping_world(0.0);
        world.player = Some(crate::ai::fixture::player(0.0, 0.0));
        let read = |traffic: &Traffic| traffic.npcs().iter().map(|npc| npc.warp.stage).collect();
        traffic.think_holding(behaviour, world, &[], &mut NeverFires);
        let thought = read(&traffic);
        traffic.fly_holding(world, &std::collections::BTreeSet::new(), &mut NeverFires);
        (thought, read(&traffic))
    }

    #[test]
    fn a_fleet_escort_in_formation_or_leaving_its_fleet_gives_up_its_stage_until_the_hold() {
        // Formation's +0x50 = -1.0 (@0x83ea8-0x83eb0), the lost parent's
        // (@0x83974); the hold zeroes it in the same dispatch (@0x8d52e).
        // The lead thinks nothing of its own here.
        let mut lead = far(0);
        lead.no_ai = true;
        let mut escort = far(1);
        escort.leader = Some(NpcId(0));
        let mut lost = far(2);
        lost.carrier = Some(crate::bay::Carrier {
            ship: ShipRef::Npc(NpcId(0)),
            window: 100.0,
            reach: 40.0,
        });
        let (thought, flown) = stages(
            &crate::ai::FleetEscortAi::default(),
            vec![lead, escort, lost],
        );
        assert_eq!(thought, vec![0.0, -1.0, -1.0]);
        assert_eq!(flown, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn a_players_escort_under_defend_attack_or_formation_gives_up_its_stage_until_the_hold() {
        // @0x83c6b-0x83c73, @0x83d62-0x83d6a, @0x83ea8-0x83eb0, whoever
        // the parent; Hold Position writes none (@0x83e59-0x83e74).
        use crate::escort::{EscortDuty, EscortOrder, FormationSlot};
        let escort = |order: Option<EscortOrder>| {
            let mut ship = far(0);
            ship.escort = Some(EscortDuty {
                formation: FormationSlot::default(),
                order,
            });
            ship
        };
        let (thought, flown) = stages(
            &crate::ai::EscortAi::default(),
            vec![
                escort(Some(EscortOrder::Defend)),
                escort(Some(EscortOrder::Attack)),
                escort(None),
                escort(Some(EscortOrder::Hold)),
            ],
        );
        assert_eq!(thought, vec![-1.0, -1.0, -1.0, 0.0]);
        assert_eq!(flown, vec![0.0; 4]);
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
