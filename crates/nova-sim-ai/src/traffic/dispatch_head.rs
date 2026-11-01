//! The head of each ship's `_AIDispatch`, run at that ship's own slot of
//! the loop (@0x8fcc8-0x8ff13 in the `EV Nova` executable).
//!
//! `_AIDispatch` (@0x8fb52-0x90200) runs, for each ship, before its type's
//! think:
//!
//! 1. the warp cancel: a ship `_IsDisabled` whose +0x50 is above 0 gives
//!    its stage up (@0x8fcc8-0x8fce9; [`disabled_reset`]);
//! 2. the jump path: a ship in sub 4 or 0xd goes to goal 0xb's cancels
//!    (@0x8fd83-0x8fd9a → @0x900d5-0x9019c;
//!    [`Traffic::cancel_jump_follows`]), then its high level, and skips
//!    the rest of this list and its type's think, the cancelled ones too;
//! 3. the escort orders, a leader's own on its frame (frame % 8 == slot
//!    >> 3, @0x8fdb3-0x8fde9; [`escort_orders`]);
//! 4. the regroup pause's draws (@0x8fe73-0x8febb; [`regroup`]);
//! 5. the disabled reset (@0x8fece-0x8ff13; [`disabled_reset`]).
//!
//! [`Traffic::dispatch_heads`] runs them at the head of each run of the
//! loop ([`Traffic::think_run`]), for the ships the run picks, pass by
//! pass in that order. A run is one slot, so each ship's head comes at its
//! own turn, and each pass that reads or writes another slot's ship reads
//! it as the slots below have left it:
//!
//! - a ship below its leader thinks on the old order word that frame, and
//!   one above on the new;
//! - a follower above its leader cancels its jump on the frame the leader
//!   turns to attack, and one below on the next;
//! - a lower slot reads a ship in the regroup pause, or disabled, as the
//!   frame began, and a higher slot reads it released or reset.
//!
//! Under [`RuleKey::LaunchFrameAi`](crate::RuleKey::LaunchFrameAi)'s other
//! reading the loop is one run, and each pass reads the tick as it began.
//!
//! Left before the first slot ([`Traffic::open_think`]): the lost leads'
//! succession and the formations, whose marking pass runs before every
//! dispatch in the original too (@0x4405e-0x441f7). Nothing here plays a
//! sound; the regroup draws are the only draws.
//!
//! [`disabled_reset`]: super::disabled_reset
//! [`escort_orders`]: super::escort_orders
//! [`regroup`]: super::regroup

use std::collections::BTreeSet;

use super::npc::{Npc, NpcId};
use super::{Traffic, World, decision_due, regroup};
use crate::ai::{dispatch_gate, swarm};
use crate::chance::Chance;

impl Traffic {
    /// The head of the dispatch of each NPC `picked` picks (one flag an
    /// NPC), in `world`, rolling on `chance`, pass by pass in the
    /// original's order (see the module docs). Whether each took the jump
    /// path, as its dispatch's head found it: those make no type's think
    /// this tick, cancelled or not.
    pub(super) fn dispatch_heads(
        &mut self,
        world: &World,
        picked: &[bool],
        chance: &mut (impl Chance + ?Sized),
    ) -> Vec<bool> {
        self.cancel_warps(picked);
        let jump_path: Vec<bool> = self
            .npcs
            .iter()
            .zip(picked)
            .map(|(npc, &picked)| picked && dispatch_gate::takes_jump_path(npc))
            .collect();
        self.cancel_jump_follows(world, picked);
        self.issue_escort_orders(world, picked);
        self.release_regroups(picked, chance);
        self.reset_disabled(world.npc_escort_ai, picked);
        jump_path
    }

    /// `_AIDispatch`'s swarm check (@0x8ff26-0x8ff35) for each NPC `picks`
    /// picks and on its AI timer, but one held in the regroup pause, one
    /// on a jump now, or one that took the jump path at its head
    /// (`jump_path`, one flag an NPC, from [`Traffic::dispatch_heads`]),
    /// as the jump path's branch goes past it.
    ///
    /// It comes before the type AI, reading the targets the frame's lower
    /// slots have just written. Each slot's run comes after theirs, so this
    /// reads them too; a ship's own target is the one its commands wrote
    /// on the last tick, as the swarm check reads it before its own think.
    pub(super) fn swarm_checks(&mut self, picks: &impl Fn(&Npc) -> bool, jump_path: &[bool]) {
        let jumped: BTreeSet<NpcId> = self
            .npcs
            .iter()
            .zip(jump_path)
            .filter(|&(_, &jump_path)| jump_path)
            .map(|(npc, _)| npc.id)
            .collect();
        let (ticks, interval) = (self.ticks, self.interval);
        swarm::pass(&mut self.npcs, |npc| {
            picks(npc)
                && !regroup::paused(npc)
                && npc.warp.goal.is_none()
                && !jumped.contains(&npc.id)
                && decision_due(ticks, npc.id, interval)
        });
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use crate::ai::regroup::Kept;
    use crate::ai::{Behaviour, Decision, Goal, Surroundings};
    use crate::catalog::{EscortClass, StellarId};
    use crate::chance::Chance;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::fleet::OrderWord;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{AiType, Npc, NpcId};
    use crate::traffic::regroup::RELEASE_DRAW;
    use crate::traffic::warp_out::{GIVEN_UP_STAGE, WarpGoal};
    use crate::traffic::{Traffic, World};
    use nova_sim_base::fixture::{Draws, FAST};

    /// Every ship keeps its goal but `turns`, which takes the goal paired
    /// with it; each think records the ship that thinks and `watched` as
    /// it reads them.
    #[derive(Debug, Default)]
    struct Watch {
        watched: Option<NpcId>,
        turns: Option<(NpcId, Goal)>,
        seen: RefCell<BTreeMap<NpcId, (Npc, Option<Npc>)>>,
    }

    impl Watch {
        fn of(watched: NpcId) -> Self {
            Self {
                watched: Some(watched),
                ..Self::default()
            }
        }

        /// What `id`'s think read of itself, and of the watched ship.
        fn seen(&self, id: NpcId) -> (Npc, Option<Npc>) {
            self.seen.borrow()[&id].clone()
        }

        fn thought(&self, id: NpcId) -> bool {
            self.seen.borrow().contains_key(&id)
        }
    }

    impl Behaviour for Watch {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.decision(npc, around, chance).goal
        }

        fn decision(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Decision {
            let watched = self.watched.and_then(|id| around.npc(id)).cloned();
            self.seen
                .borrow_mut()
                .insert(npc.id, (npc.clone(), watched));
            let goal = match self.turns {
                Some((id, goal)) if id == npc.id => goal,
                _ => npc.goal,
            };
            Decision {
                goal,
                ..Decision::default()
            }
        }
    }

    /// A ship at rest at the centre.
    fn ship() -> Npc {
        npc(0, ShipStats::new(FAST, &[]))
    }

    /// Traffic deciding every tick.
    fn traffic() -> Traffic {
        Traffic::with_interval(1)
    }

    /// One tick's think in a world of frame 0, drawing `draws`.
    fn think(traffic: &mut Traffic, behaviour: &Watch, draws: &[u32]) {
        let world = World::new(&[]);
        traffic.think_holding(behaviour, world, &[], &mut Draws::of(draws));
    }

    fn of(traffic: &Traffic, id: NpcId) -> &Npc {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("there")
    }

    #[test]
    fn a_follower_below_its_leader_thinks_on_the_old_order_word_and_one_above_on_the_new() {
        // Frame 0 is the order frame of berth 2 (frame % 8 == 2 >> 3).
        let mut traffic = traffic();
        let mut leader = ship();
        leader.ai_type = AiType::Warship;
        leader.goal = Goal::Attack(ShipRef::Npc(NpcId(9)));
        let leader = traffic.add_npc_at(leader, 2);
        let led = |traffic: &mut Traffic, berth| {
            let mut ship = ship();
            ship.ai_type = AiType::Warship;
            ship.class = EscortClass::Fighter;
            ship.leader = Some(leader);
            ship.state.position = Vec2::new(0.0, 100.0);
            traffic.add_npc_at(ship, berth)
        };
        let (below, above) = (led(&mut traffic, 1), led(&mut traffic, 3));
        let watch = Watch::default();
        think(&mut traffic, &watch, &[]);
        assert_eq!(watch.seen(below).0.order_word, OrderWord(-1), "the old");
        let new = of(&traffic, below).order_word;
        assert_ne!(new, OrderWord(-1), "written at the leader's slot");
        assert_eq!(watch.seen(above).0.order_word, new);
    }

    /// A ship at `berth` following `leader`'s jump, its stage not begun.
    fn following(traffic: &mut Traffic, leader: NpcId, berth: u32) -> NpcId {
        let mut ship = ship();
        ship.leader = Some(leader);
        ship.goal = Goal::Idle;
        ship.follow_leaders_jump(ShipRef::Npc(leader));
        traffic.add_npc_at(ship, berth)
    }

    #[test]
    fn a_follower_above_its_leader_cancels_its_jump_on_the_tick_the_leader_turns_to_attack_and_one_below_on_the_next()
     {
        // The cancel reads the lead's goal as the lower slots left it
        // (@0x90125-0x9019c).
        let mut traffic = traffic();
        let leader = traffic.add_npc_at(ship(), 2);
        let below = following(&mut traffic, leader, 1);
        let above = following(&mut traffic, leader, 3);
        let watch = Watch {
            turns: Some((leader, Goal::Attack(ShipRef::Npc(NpcId(9))))),
            ..Watch::default()
        };
        think(&mut traffic, &watch, &[]);
        assert_eq!(of(&traffic, above).warp.goal, None, "cancelled");
        assert_eq!(of(&traffic, above).goal, Goal::Idle, "goal 0");
        let held = Some(WarpGoal::Follow(ShipRef::Npc(leader)));
        assert_eq!(of(&traffic, below).warp.goal, held, "its head came first");
        think(&mut traffic, &watch, &[]);
        assert_eq!(of(&traffic, below).warp.goal, None, "the next tick");
    }

    #[test]
    fn a_follower_its_jumps_cancel_puts_back_makes_no_think_that_tick() {
        // The jump path goes from the cancels to the high level
        // (@0x9019c-0x901e2), past the type's think.
        let mut traffic = traffic();
        let leader = traffic.add_npc_at(ship(), 2);
        let follower = following(&mut traffic, leader, 3);
        traffic.npcs_mut()[0].goal = Goal::Attack(ShipRef::Npc(NpcId(9)));
        let watch = Watch::default();
        think(&mut traffic, &watch, &[]);
        assert_eq!(of(&traffic, follower).warp.goal, None, "cancelled");
        assert!(!watch.thought(follower), "no think on the jump path");
        think(&mut traffic, &watch, &[]);
        assert!(watch.thought(follower), "its next tick's");
    }

    #[test]
    fn a_follower_its_jumps_cancel_puts_back_runs_no_swarm_check_that_tick() {
        // The swarm check (@0x8ff26-0x8ff35) comes after the jump path's
        // branch (@0x8fd83-0x8fd9a), which goes past it.
        let mut traffic = traffic();
        let swarming = |mut ship: Npc| {
            ship.hull.swarms = true;
            ship.govt = Some(crate::catalog::GovtId(128));
            ship
        };
        traffic.add_npc_at(swarming(ship()), 1);
        let mut leader = ship();
        leader.goal = Goal::Attack(ShipRef::Npc(NpcId(9)));
        let leader = traffic.add_npc_at(leader, 2);
        let follower = following(&mut traffic, leader, 3);
        let follower_ship = traffic
            .npcs_mut()
            .iter_mut()
            .find(|npc| npc.id == follower)
            .expect("there");
        *follower_ship = swarming(follower_ship.clone());
        think(&mut traffic, &Watch::default(), &[]);
        assert_eq!(of(&traffic, follower).warp.goal, None, "cancelled");
        assert_eq!(of(&traffic, follower).swarm_leader, None, "no check");
        think(&mut traffic, &Watch::default(), &[]);
        assert_eq!(of(&traffic, follower).swarm_leader, Some(1), "its next");
    }

    #[test]
    fn a_lower_slot_reads_a_regrouping_ship_still_paused_and_a_higher_slot_reads_it_released() {
        let mut traffic = traffic();
        let below = traffic.add_npc_at(ship(), 1);
        let mut paused = ship();
        paused.goal = Goal::Regroup { kept: Kept::Coast };
        let paused = traffic.add_npc_at(paused, 3);
        let above = traffic.add_npc_at(ship(), 5);
        let watch = Watch::of(paused);
        let mut chance = Draws::of(&[]).keyed(RELEASE_DRAW, 0);
        traffic.think_holding(&watch, World::new(&[]), &[], &mut chance);
        assert_eq!(chance.times_asked(RELEASE_DRAW), 1);
        let goal = |id| watch.seen(id).1.expect("watched").goal;
        assert_eq!(goal(below), Goal::Regroup { kept: Kept::Coast });
        assert_eq!(goal(above), Goal::Idle, "drawn 0 at its slot");
    }

    /// A ship at berth 3, disabled on the last tick while it attacked NPC 9
    /// on its way to stellar 128, its jump's stage under way.
    fn disabled(traffic: &mut Traffic) -> NpcId {
        let mut ship = ship();
        ship.condition = Condition::Disabled;
        ship.goal = Goal::Attack(ShipRef::Npc(NpcId(9)));
        ship.target = Some(ShipRef::Npc(NpcId(9)));
        ship.travel = Some(StellarId(128));
        ship.warp.stage = 5.0;
        traffic.add_npc_at(ship, 3)
    }

    /// What a ship at berth 1 and one at berth 5 read of [`disabled`]'s
    /// ship in one tick.
    fn read_of_disabled() -> (Npc, Npc) {
        let mut traffic = traffic();
        let below = traffic.add_npc_at(ship(), 1);
        let down = disabled(&mut traffic);
        let above = traffic.add_npc_at(ship(), 5);
        let watch = Watch::of(down);
        think(&mut traffic, &watch, &[]);
        let read = |id| watch.seen(id).1.expect("watched");
        (read(below), read(above))
    }

    #[test]
    fn a_lower_slot_reads_a_ship_disabled_last_tick_as_it_was_and_a_higher_slot_reads_it_reset() {
        let (below, above) = read_of_disabled();
        assert_eq!(below.target, Some(ShipRef::Npc(NpcId(9))));
        assert_eq!(below.travel, Some(StellarId(128)));
        assert_eq!(below.goal, Goal::Attack(ShipRef::Npc(NpcId(9))));
        assert_eq!(above.target, None);
        assert_eq!(above.travel, None);
        assert_eq!(above.goal, Goal::Idle);
    }

    #[test]
    fn a_lower_slot_reads_a_disabled_ships_warp_stage_as_the_tick_began_and_a_higher_slot_reads_it_cancelled()
     {
        let (below, above) = read_of_disabled();
        assert_eq!(below.warp.stage, 5.0);
        assert_eq!(above.warp.stage, GIVEN_UP_STAGE);
    }
}
