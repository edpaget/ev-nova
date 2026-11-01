//! A follower's warp at its own slot of `_AIDispatch`'s loop: goal 0xb's
//! block and its push of the leader (`_HighLevelAIHandler`
//! @0x8de63-0x8df75 in the `EV Nova` executable), then sub-goal 0xd's step
//! (`_LowLevelAIHandler` @0x859c5-0x85c5f; see [`warp_out`]).
//!
//! Both run inside the follower's own dispatch, the high level's goal
//! block (goal 0x14's @0x8dc5a comes before goal 0xb's @0x8de63) before
//! the low level, so:
//!
//! - **The push** (@0x8dedf-0x8defa, [`warp_out::pushes_leader`]) writes
//!   goal 2 on the leader's slot at the follower's slot: a leader above
//!   its follower thinks on goal 2 that tick, one below it on the next.
//!   It writes the slot whether or not it is active, so on the leader's
//!   departure tick a follower above it writes goal 2 on the freed slot's
//!   record, which a successor copies from the next tick
//!   (@0x893b0-0x893e1, see [`succession`](super::succession)).
//! - **The check and sub 0xd** read the leader live, by +0x9a, which no
//!   slot test clears: a leader in a higher slot as the tick began, one in
//!   a lower slot after its own step, and, on its departure tick, the
//!   freed slot's record for a follower above it (+0x50 0, sub 4 kept,
//!   @0x859ae-0x859bf), so it follows and keeps formation at the record's
//!   velocity. From the next tick the marking pass's succession
//!   (`_AICheckParentExists` @0x44101) has run first.
//! - **What it writes** (its goal and sub on a give-up, its own jump, a
//!   detach's AI type and leader, its stage, clock, velocity, glow, aim and
//!   travel stellar) a higher slot's think reads that tick.
//!
//! A follower whose sub 0xd keeps formation is flown by the autopilot as
//! its goal says ([`warp_out::FollowStep::Formation`]); the fly half
//! flies the rest of the jump ([`warp_out::fly_jump`]) and decides
//! nothing. Double time's second pass runs no `_AIDispatch`, so neither
//! the check nor sub 0xd. A follower that detaches at sub 0xd runs no sub
//! 4 that frame, as sub 4's block comes first in the low level
//! (@0x8585b); one the check sets on its own jump runs it at once, as the
//! low level follows the high level. Nothing here plays a sound.

use super::npc::NpcId;
use super::warp_out::{self, FollowStep, LeaderWarp, WarpGoal};
use super::{Traffic, World, regroup};
use crate::ai::Goal;
use crate::combat::ShipRef;

impl Traffic {
    /// Goal 0xb's block, its push, and sub 0xd's step of the NPC at
    /// `index` in `world` (see the module docs), sub 0xd held when `still`:
    /// whether it detached from its leader there.
    pub(super) fn follow_in_dispatch(&mut self, index: usize, world: &World, still: bool) -> bool {
        let npc = &self.npcs[index];
        let id = npc.id;
        self.followed.remove(&id);
        let paused = regroup::paused(npc);
        if let Some(lead) = warp_out::pushes_leader(npc, paused) {
            self.push_out(lead);
        }
        let npc = &self.npcs[index];
        let leader = match npc.warp.goal {
            Some(WarpGoal::Follow(ShipRef::Player)) => Some((true, world.warp.player)),
            Some(WarpGoal::Follow(ShipRef::Npc(_))) => (npc.leader)
                .and_then(|lead| self.leader_view(lead))
                .map(|view| (false, view)),
            _ => None,
        };
        let npc = &mut self.npcs[index];
        let Some((player, leader)) = warp_out::goal_block(npc, leader, paused) else {
            return false;
        };
        if still {
            return false;
        }
        let step = warp_out::follow_jump(npc, leader, player);
        self.followed.insert(id, step == FollowStep::Formation);
        step == FollowStep::Detached
    }

    /// The warp NPC `lead` shows its followers at this point of the AI
    /// loop: as it stands, or as its record left it at its slot of this
    /// tick's loop.
    fn leader_view(&self, lead: NpcId) -> Option<LeaderWarp> {
        (self.npcs.iter())
            .chain(&self.left_in_dispatch)
            .find(|npc| npc.id == lead)
            .map(warp_out::view)
    }

    /// Goal 0xb's push of NPC `lead` onto goal 2 (@0x8def0): live, or on
    /// its slot's record, but not on a ship the player leads (see
    /// [`warp_out::pushes_leader`]).
    fn push_out(&mut self, lead: NpcId) {
        let ship = match self.npcs.iter_mut().find(|npc| npc.id == lead) {
            Some(npc) => Some(npc),
            None => self.gone.ship_mut(lead),
        };
        if let Some(npc) = ship.filter(|npc| !npc.escorts_player()) {
            npc.goal = Goal::JumpOut;
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use crate::ai::fixture::{TRADERS, ship};
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::chance::{Chance, NeverFires};
    use crate::combat::ShipRef;
    use crate::geometry::Vec2;
    use crate::traffic::npc::{AiType, Npc, NpcId};
    use crate::traffic::warp_out::WarpGoal;
    use crate::traffic::{Traffic, World};

    /// What a think read of the ship it watches: its goal, its warp goal
    /// and its AI type.
    type Read = (Goal, Option<WarpGoal>, AiType);

    /// Every ship keeps its goal; each think records its own goal and what
    /// it reads of `watched`.
    #[derive(Debug)]
    struct Reads {
        watched: NpcId,
        seen: RefCell<BTreeMap<NpcId, (Goal, Option<Read>)>>,
    }

    impl Reads {
        fn of(watched: NpcId) -> Self {
            Self {
                watched,
                seen: RefCell::default(),
            }
        }

        fn own(&self, id: NpcId) -> Goal {
            self.seen.borrow()[&id].0
        }

        fn read(&self, id: NpcId) -> Option<Read> {
            self.seen.borrow()[&id].1
        }
    }

    impl Behaviour for Reads {
        fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            let read = around
                .npc(self.watched)
                .map(|ship| (ship.goal, ship.warp.goal, ship.ai_type));
            self.seen.borrow_mut().insert(npc.id, (npc.goal, read));
            npc.goal
        }
    }

    fn of(traffic: &Traffic, id: NpcId) -> &Npc {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("there")
    }

    /// A world whose warp sound runs 600 Mac ticks.
    fn world() -> World<'static> {
        let mut world = World::new(&[]);
        world.warp.durations = [600, 600];
        world
    }

    /// An idle ship in slot `berth`, at the centre.
    fn idle(traffic: &mut Traffic, berth: u32) -> NpcId {
        traffic.add_npc_at(ship(0, TRADERS, AiType::Warship, 0.0, 0.0), berth)
    }

    /// A ship in slot `berth` following `leader`'s jump, its stage `stage`.
    fn following(traffic: &mut Traffic, leader: NpcId, berth: u32, stage: f32) -> NpcId {
        let mut follower = ship(0, TRADERS, AiType::WimpyTrader, 2000.0, 50.0);
        follower.leader = Some(leader);
        follower.goal = Goal::Idle;
        follower.follow_leaders_jump(ShipRef::Npc(leader));
        follower.warp.stage = stage;
        traffic.add_npc_at(follower, berth)
    }

    #[test]
    fn a_follower_below_its_leader_puts_it_on_goal_2_before_the_leaders_think_that_tick() {
        // Goal 0xb's push writes the leader's slot inside the follower's own
        // dispatch (@0x8def0).
        let mut traffic = Traffic::with_interval(1);
        let leader = idle(&mut traffic, 3);
        following(&mut traffic, leader, 2, 0.0);
        let reads = Reads::of(leader);
        traffic.think_holding(&reads, world(), &[], &mut NeverFires);
        assert_eq!(reads.own(leader), Goal::JumpOut, "goal 2 at its think");
    }

    #[test]
    fn one_above_puts_it_on_goal_2_for_the_next_tick() {
        let mut traffic = Traffic::with_interval(1);
        let leader = idle(&mut traffic, 3);
        following(&mut traffic, leader, 4, 0.0);
        let reads = Reads::of(leader);
        traffic.think_holding(&reads, world(), &[], &mut NeverFires);
        assert_eq!(reads.own(leader), Goal::Idle, "its think came first");
        assert_eq!(of(&traffic, leader).goal, Goal::JumpOut, "pushed after");
    }

    #[test]
    fn a_leader_the_player_leads_is_not_pushed() {
        // Its own next hold would put it back on goal 0xb (@0x8d542-0x8d54c).
        let mut traffic = Traffic::with_interval(1);
        let mut lead = ship(0, TRADERS, AiType::Warship, 0.0, 0.0);
        lead.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        let leader = traffic.add_npc_at(lead, 3);
        following(&mut traffic, leader, 2, 0.0);
        let reads = Reads::of(leader);
        traffic.think_holding(&reads, world(), &[], &mut NeverFires);
        assert_ne!(reads.own(leader), Goal::JumpOut);
    }

    /// A ship in slot `berth` well into its own warp: stage 5 and facing
    /// its aim, east of the centre, on goal 2's jump.
    fn warping(traffic: &mut Traffic, berth: u32) -> NpcId {
        let mut leader = ship(0, TRADERS, AiType::Warship, 2000.0, 0.0);
        leader.goal = Goal::JumpOut;
        leader.begin_own_jump();
        leader.warp.aim = crate::trig::intercept_angle(Vec2::ZERO, leader.state.position);
        leader.state.heading = f32::from(leader.warp.aim);
        leader.warp.stage = 5.0;
        traffic.add_npc_at(leader, berth)
    }

    #[test]
    fn a_higher_slots_think_reads_a_followers_detach_that_tick() {
        // Past 30.0 sub 0xd detaches it: goal 2, sub 4, its class's AI type
        // (@0x85a8f-0x85af1), at its slot. In its AI wait, so its high
        // level's hold doesn't take it off its leader's jump first
        // (@0x8d4b9-0x8d4d2).
        let mut traffic = Traffic::with_interval(1);
        let leader = warping(&mut traffic, 5);
        let follower = following(&mut traffic, leader, 2, 30.5);
        let npc = traffic.npcs_mut().iter_mut().find(|npc| npc.id == follower);
        let npc = npc.expect("there");
        npc.ai_type = AiType::Interceptor;
        npc.ai_wait = crate::traffic::warp_out::FOLLOW_WAIT;
        let below = idle(&mut traffic, 1);
        let above = idle(&mut traffic, 6);
        let reads = Reads::of(follower);
        traffic.think_holding(&reads, world(), &[], &mut NeverFires);
        let follows = Some(WarpGoal::Follow(ShipRef::Npc(leader)));
        assert_eq!(reads.read(below).map(|read| read.1), Some(follows));
        let (goal, warp, ai_type) = reads.read(above).expect("there");
        assert_eq!((goal, warp), (Goal::JumpOut, Some(WarpGoal::Own)));
        let class = of(&traffic, follower).ai_type;
        assert_eq!(ai_type, class, "its class's, at slot 2");
        assert_eq!(of(&traffic, follower).leader, None);
        // Sub 4's block comes first in the low level (@0x8585b): no sub 4
        // step that frame.
        assert_eq!(of(&traffic, follower).warp.stage, 30.5);
    }

    #[test]
    fn a_follower_keeping_formation_at_its_slot_is_flown_by_its_goal() {
        // Sub 0xd's formation branch takes its leader's velocity and leaves
        // the flight to its goal's (@0x85a0b-0x85a51): an idle follower
        // brakes, where a jump's flight wouldn't thrust.
        let mut traffic = Traffic::with_interval(1);
        let mut lead = ship(0, TRADERS, AiType::Warship, 0.0, 0.0);
        lead.goal = Goal::JumpOut;
        lead.begin_own_jump();
        lead.state.velocity = Vec2::new(3.0, 0.0);
        lead.state.heading = 270.0;
        let leader = traffic.add_npc_at(lead, 5);
        let follower = following(&mut traffic, leader, 2, 0.0);
        let npc = traffic.npcs_mut().iter_mut().find(|npc| npc.id == follower);
        npc.expect("there").state.heading = 270.0;
        traffic.tick_in(&Reads::of(leader), world(), &[], &mut NeverFires);
        let npc = of(&traffic, follower);
        assert!(
            npc.state.velocity.x < 3.0,
            "braking: {:?}",
            npc.state.velocity
        );
    }

    #[test]
    fn a_follower_whose_leader_is_not_leaving_gives_up_at_its_slot_and_a_higher_slot_reads_goal_0()
    {
        // Goal 0, sub 0 (@0x8df31-0x8df49).
        let mut traffic = Traffic::with_interval(1);
        let leader = idle(&mut traffic, 5);
        let follower = following(&mut traffic, leader, 2, 0.0);
        let npc = traffic.npcs_mut().iter_mut().find(|npc| npc.id == follower);
        npc.expect("there").goal = Goal::Follow {
            lead: leader,
            guard: None,
        };
        let below = idle(&mut traffic, 1);
        let above = idle(&mut traffic, 6);
        let reads = Reads::of(follower);
        traffic.think_holding(&reads, world(), &[], &mut NeverFires);
        let (goal, warp, _) = reads.read(below).expect("there");
        assert!(matches!(goal, Goal::Follow { .. }), "{goal:?}");
        assert!(warp.is_some());
        let (goal, warp, _) = reads.read(above).expect("there");
        assert_eq!((goal, warp), (Goal::Idle, None));
    }

    #[test]
    fn a_follower_above_a_leader_that_left_at_its_slot_pushes_goal_2_onto_its_record() {
        // The push writes +0x9a's slot whether or not it is active
        // (@0x8def0); a successor copies the record's goal next tick
        // (@0x893b0-0x893e1).
        let mut traffic = Traffic::with_interval(1);
        let attacker = idle(&mut traffic, 1);
        let mut lead = ship(0, TRADERS, AiType::Warship, 2000.0, 0.0);
        lead.goal = Goal::Flee(ShipRef::Npc(attacker));
        lead.target = Some(ShipRef::Npc(attacker));
        lead.begin_own_jump();
        lead.warp.aim = crate::trig::intercept_angle(Vec2::ZERO, lead.state.position);
        lead.state.heading = f32::from(lead.warp.aim);
        lead.warp.stage = 3.0;
        lead.warp.since = 1000;
        let leader = traffic.add_npc_at(lead, 3);
        following(&mut traffic, leader, 4, 0.0);
        let reads = Reads::of(leader);
        traffic.think_holding(&reads, world(), &[], &mut NeverFires);
        let record = traffic
            .gone
            .ships()
            .iter()
            .find(|gone| gone.ship.id == leader);
        assert_eq!(record.expect("left at slot 3").ship.goal, Goal::JumpOut);
    }

    /// A ship in slot `berth` on the last pass of its own warp out, bound
    /// for stellar 140, its record left at its slot this tick.
    fn last_pass(traffic: &mut Traffic, berth: u32) -> NpcId {
        let mut lead = ship(0, TRADERS, AiType::Warship, 2000.0, 0.0);
        lead.goal = Goal::JumpOut;
        lead.begin_own_jump();
        lead.warp.aim = crate::trig::intercept_angle(Vec2::ZERO, lead.state.position);
        lead.state.heading = f32::from(lead.warp.aim);
        lead.warp.stage = 3.0;
        lead.warp.since = 1000;
        lead.travel = Some(crate::catalog::StellarId(140));
        traffic.add_npc_at(lead, berth)
    }

    #[test]
    fn a_follower_of_a_leader_that_jumped_out_at_its_slot_takes_over_by_its_record_the_next_tick() {
        // The departure tick it follows the record (sub 4 kept). Before the
        // next tick's loop the marking pass's succession runs (@0x44101): a
        // lone follower of its government takes the record's goal and sub,
        // its target and travel stellar (@0x893b0-0x893e1), and +0x50 =
        // -1.0, so its own jump restarts its stage at its slot. It takes
        // none of goal 0xb's no-leader clears (@0x8df57-0x8df5d).
        let mut traffic = Traffic::with_interval(1);
        let leader = last_pass(&mut traffic, 3);
        let follower = following(&mut traffic, leader, 4, 0.0);
        let keep = Reads::of(leader);
        traffic.tick_in(&keep, world(), &[], &mut NeverFires);
        assert!(traffic.npcs().iter().all(|npc| npc.id != leader), "gone");
        let follows = Some(WarpGoal::Follow(ShipRef::Npc(leader)));
        assert_eq!(of(&traffic, follower).warp.goal, follows, "that tick");
        traffic.tick_in(&keep, world(), &[], &mut NeverFires);
        let npc = of(&traffic, follower);
        assert_eq!(npc.leader, None, "+0x9a -1 (@0x894b4)");
        assert_eq!(
            (npc.goal, npc.warp.goal),
            (Goal::JumpOut, Some(WarpGoal::Own))
        );
        assert_eq!(
            npc.travel,
            Some(crate::catalog::StellarId(140)),
            "the record's"
        );
        assert_eq!((npc.warp.stage, npc.warp.since), (2.0, 0), "restarted");
    }

    #[test]
    fn a_fellow_follower_of_a_leader_that_jumped_out_at_its_slot_follows_its_successor() {
        // "Another leads" (@0x89479-0x894af): the heavier follower takes
        // over, the other's +0x9a names it.
        let mut traffic = Traffic::with_interval(1);
        let leader = last_pass(&mut traffic, 3);
        let heavy = following(&mut traffic, leader, 4, 0.0);
        let light = following(&mut traffic, leader, 5, 0.0);
        let npc = traffic.npcs_mut().iter_mut().find(|npc| npc.id == heavy);
        npc.expect("there").hull.signed_mass = 900;
        let keep = Reads::of(leader);
        traffic.tick_in(&keep, world(), &[], &mut NeverFires);
        traffic.tick_in(&keep, world(), &[], &mut NeverFires);
        assert_eq!(of(&traffic, heavy).leader, None, "it leads");
        assert_eq!(of(&traffic, light).leader, Some(heavy));
    }

    /// Every ship keeps its goal; each think records the velocity it reads
    /// of the ship in slot 2.
    #[derive(Debug, Default)]
    struct Velocity(RefCell<BTreeMap<NpcId, Vec2>>);

    impl Behaviour for Velocity {
        fn decide(&self, npc: &Npc, around: &Surroundings, _: &mut dyn Chance) -> Goal {
            if let Some(ship) = around.npcs.iter().find(|ship| ship.berth == 2) {
                self.0.borrow_mut().insert(npc.id, ship.state.velocity);
            }
            npc.goal
        }
    }

    #[test]
    fn a_followers_formation_at_its_slot_is_read_above_it_that_tick() {
        // Sub 0xd takes its leader's velocity while the leader's +0x50 is
        // 1.0 or below (@0x85a0b-0x85a1a), at its slot.
        let mut traffic = Traffic::with_interval(1);
        let mut lead = ship(0, TRADERS, AiType::Warship, 0.0, 0.0);
        lead.goal = Goal::JumpOut;
        lead.begin_own_jump();
        lead.state.velocity = Vec2::new(3.0, 0.0);
        lead.warp.stage = 0.0;
        let leader = traffic.add_npc_at(lead, 5);
        following(&mut traffic, leader, 2, 0.0);
        let above = idle(&mut traffic, 6);
        let reads = Velocity::default();
        traffic.think_holding(&reads, world(), &[], &mut NeverFires);
        assert_eq!(reads.0.borrow()[&above], Vec2::new(3.0, 0.0));
    }
}
