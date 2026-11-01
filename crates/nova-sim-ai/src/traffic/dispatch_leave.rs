//! The low level's leaves in `_AIDispatch`'s loop: a ship's own jump-out
//! and a fighter's dock, each at that ship's own slot (in the `EV Nova`
//! executable).
//!
//! `_LowLevelAIHandler` is the last thing in a ship's `_AIDispatch`
//! (@0x901f4). Two of its substates free the ship's slot there:
//!
//! - **sub 4, the ship's own jump** (@0x8585b-0x859c5;
//!   [`warp_out::own_jump`]): its aim, the stamp and stage, and once
//!   t ≥ d/w on its clock, +0x50 = 0, +0xb8 = 0 and +0x74 = -1
//!   (@0x859ae-0x859bf), its substate left at 4;
//! - **the carrier dock** (@0x87f0b-0x87fef): a fighter within its
//!   carrier's reach on both axes, the reach read from the carrier's live
//!   place (@0x87f0b-0x87fb9), calls `_AIAddFighterToParent`
//!   (@0x802a5-0x80424), which, with a bay for it, raises the bay's count
//!   (@0x803e6) and frees the slot (+0xb8 = 0 @0x80404, +0x9a and +0x88
//!   -1). With no bay it writes nothing and the fighter stays out
//!   ([`RuleKey::FighterNoBay`](crate::RuleKey::FighterNoBay), whose
//!   fighters the session names).
//!
//! [`Traffic::leave_in_dispatch`] decides both at the end of each run of
//! the loop ([`Traffic::think_run`]), for the ships the run picks: each
//! leaver is recorded among the ships gone ([`Freed::InDispatch`]) and
//! taken out of the traffic at once. So:
//!
//! - a higher slot's think that tick reads the gone ship's record
//!   ([`gone`](super::gone)), and a lower slot's has run before it;
//! - the slot is free to a higher slot's launch that tick
//!   ([`launch_room`](super::launch_room)), as `_GenericRandomShipSpawn`
//!   rereads +0xb8 (@0x3c8c6-0x3c8e9);
//! - every slot's `_HandleShip` (@0x33865-0x33888, the ship loop's
//!   [`Traffic::let_go_at_step`]) finds the slot inactive, the earlier
//!   slots' too, before their fire;
//! - the fighter's bay is restocked at its slot by the session, so a
//!   carrier above it launches the round that tick.
//!
//! Neither fires or launches anything after it: `_AIFireTurret`
//! (@0x80ad3) runs only from the fight substates. Neither plays a sound.
//! The fly half docks nothing and jumps nothing out (see
//! [`fly_pass`](super::fly_pass)): double time's second pass runs no
//! `_AIDispatch`, so its low level can't dock or jump either.
//!
//! **A leader's warp, by slot.** A follower on its leader's jump reads
//! the leader's +0x50 live in its own low level (sub 0xd @0x859c5-0x85c5f):
//! one below its leader reads it as the tick began, one above after the
//! leader's own step. On the leader's departure tick, one above reads the
//! freed record: +0x50 0, sub 4 kept. The fly half reads each follower's
//! leader so ([`FlyPass`](super::FlyPass)'s two views).

use std::collections::BTreeSet;

use super::gone::Freed;
use super::npc::{Mode, Npc, NpcId};
use super::warp_out::{self, WarpGoal};
use super::{Outcome, Traffic, World, autopilot, jump_with_leader, launch_room};
use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;

impl Traffic {
    /// The loop's opening, after the passes before the first slot: every
    /// ship's warp clock moves on, as `TickCount` does between advance
    /// passes; each ship's warp is kept as the tick opened, for the
    /// followers below it; and the last tick's leavers are forgotten.
    pub(super) fn open_dispatch(&mut self) {
        for npc in &mut self.npcs {
            npc.warp.tick_clock();
        }
        self.open_leaders = warp_out::leaders(&self.npcs);
        self.left_in_dispatch.clear();
    }

    /// The low level's leaves of each NPC `picks` picks, in berth order, in
    /// `world` (see the module docs): its own jump-out, and a fighter's
    /// dock with its carrier unless `no_bay` names it. Each leaver is
    /// recorded among the ships gone, freed in the AI loop, and taken out
    /// of the traffic at once. The ships that left, and how, in order.
    pub(super) fn leave_in_dispatch(
        &mut self,
        world: &World,
        picks: &impl Fn(&Npc) -> bool,
        no_bay: &NoBay,
    ) -> Vec<(NpcId, Outcome)> {
        let order: Vec<NpcId> = launch_room::berth_order(&self.npcs)
            .into_iter()
            .filter(|&index| picks(&self.npcs[index]))
            .map(|index| self.npcs[index].id)
            .collect();
        let mut left = Vec::new();
        for id in order {
            let Some(index) = self.npcs.iter().position(|npc| npc.id == id) else {
                continue;
            };
            self.hold_and_turn_back(index, world);
            let Some(outcome) = self.leaves(index, world, no_bay) else {
                continue;
            };
            let mut record = self.npcs.remove(index);
            if outcome == Outcome::JumpedOut {
                // Its substate stays 4 (@0x859ae-0x859bf).
                record.warp.goal = Some(WarpGoal::Own);
            }
            self.left_the_system(record.clone(), Freed::InDispatch);
            self.left_in_dispatch.push(record);
            left.push((id, outcome));
        }
        self.freed.extend(left.iter().copied());
        left
    }

    /// The high level's hold on the warp of the NPC at `index` and its
    /// fighter's turn-back, in `world` (`_HighLevelAIHandler`
    /// @0x8d513-0x8d56b and @0x8d575-0x8d625; [`warp_out::hold_stage`],
    /// [`jump_with_leader::turn_back`]), before its low level: none for a
    /// disabled ship, nor past its AI wait (see `jump_with_leader`). A
    /// stellar's defence ship breaking up is not disabled (@0x2d6f), but it
    /// holds no warp and no carrier, so neither changes it. A fighter
    /// turning back reads its carrier's slot.
    fn hold_and_turn_back(&mut self, index: usize, world: &World) {
        let npc = &self.npcs[index];
        if npc.condition != Condition::Intact || npc.high_level_waits() {
            return;
        }
        let carrier_slot = npc.carrier.and_then(|carrier| match carrier.ship {
            ShipRef::Player => Some(0),
            ShipRef::Npc(id) => (self.npcs.iter())
                .chain(self.gone.ships().iter().map(|gone| &gone.ship))
                .find(|npc| npc.id == id)
                .map(|npc| npc.berth),
        });
        let npc = &mut self.npcs[index];
        warp_out::hold_stage(npc);
        jump_with_leader::turn_back(npc, world.npc_jump_fuel, carrier_slot);
    }

    /// How the NPC at `index` leaves at its slot in `world`, if it does:
    /// a flying, intact ship on its own jump whose clock has run out, or a
    /// fighter out of its AI wait within its carrier's reach with a bay for
    /// it (not in `no_bay`).
    fn leaves(&mut self, index: usize, world: &World, no_bay: &NoBay) -> Option<Outcome> {
        let npc = &self.npcs[index];
        if npc.mode != Mode::Flying || npc.condition != Condition::Intact {
            return None;
        }
        if npc.warp.goal == Some(WarpGoal::Own) && npc.jumping() {
            let npc = &mut self.npcs[index];
            return warp_out::own_jump(npc, world.warp.duration()).then_some(Outcome::JumpedOut);
        }
        let (Goal::Dock(ship), Some(dock)) = (npc.goal, npc.carrier) else {
            return None;
        };
        let waiting = npc.ai_wait > 0 && !npc.count_wait();
        if waiting || npc.jumping() || npc.docking.is_some() || no_bay.contains(&npc.id) {
            return None;
        }
        let carrier = match ship {
            ShipRef::Player => world.player.map(|player| player.state),
            ShipRef::Npc(id) => self.npcs.iter().find(|n| n.id == id).map(|n| n.state),
        };
        autopilot::docks_now(&npc.state, carrier.as_ref(), dock).then_some(Outcome::Docked {
            carrier: dock.ship,
            ship: npc.ship,
        })
    }
}

/// The fighters that stay out at their carrier, having no bay there: a
/// set the session names.
pub type NoBay = BTreeSet<NpcId>;

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use crate::ai::fixture::{PIRATES, TRADERS, ship};
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::bay::Carrier;
    use crate::chance::{Chance, NeverFires};
    use crate::combat::ShipRef;
    use crate::geometry::Vec2;
    use crate::traffic::npc::{AiType, Npc, NpcId};
    use crate::traffic::warp_out::WarpGoal;
    use crate::traffic::{NoBay, Outcome, Traffic, World};
    use crate::trig::intercept_angle;
    use nova_sim_base::fixture::Draws;

    /// Every ship keeps its goal; each think records the ships gone it
    /// reads, and whether one of them is held by it.
    #[derive(Debug, Default)]
    struct Watch {
        seen: RefCell<BTreeMap<NpcId, (Vec<NpcId>, bool)>>,
    }

    impl Watch {
        fn seen(&self, id: NpcId) -> (Vec<NpcId>, bool) {
            self.seen.borrow()[&id].clone()
        }
    }

    impl Behaviour for Watch {
        fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            let gone = around.gone.iter().map(|gone| gone.ship.id).collect();
            let held = around.held_gone(npc).is_some();
            self.seen.borrow_mut().insert(npc.id, (gone, held));
            npc.goal
        }
    }

    fn of(traffic: &Traffic, id: NpcId) -> Option<&Npc> {
        traffic.npcs().iter().find(|npc| npc.id == id)
    }

    /// A world whose warp sound runs 600 Mac ticks, so a jump's clock
    /// takes a while to run out.
    fn world() -> World<'static> {
        let mut world = World::new(&[]);
        world.warp.durations = [600, 600];
        world
    }

    /// A ship in slot `berth` on the last pass of its own warp out, out
    /// east of the centre and facing away from it, as its warp aims it.
    fn warping_out(traffic: &mut Traffic, berth: u32) -> NpcId {
        let mut runner = ship(0, TRADERS, AiType::WimpyTrader, 2000.0, 0.0);
        runner.goal = Goal::JumpOut;
        runner.begin_own_jump();
        runner.warp.aim = intercept_angle(Vec2::ZERO, runner.state.position);
        runner.state.heading = f32::from(runner.warp.aim);
        runner.warp.stage = 3.0;
        runner.warp.since = 1000;
        traffic.add_npc_at(runner, berth)
    }

    /// A ship in slot `berth`, at the centre.
    fn idle(traffic: &mut Traffic, berth: u32) -> NpcId {
        traffic.add_npc_at(ship(0, PIRATES, AiType::Warship, 0.0, 0.0), berth)
    }

    /// One tick's think of `traffic`, as `watch` decides.
    fn think(traffic: &mut Traffic, watch: &Watch) {
        traffic.think_holding(watch, world(), &[], &mut Draws::of(&[]));
    }

    #[test]
    fn a_ship_whose_own_jump_ends_leaves_at_its_slot_and_a_higher_slots_think_reads_its_record() {
        let mut traffic = Traffic::with_interval(1);
        let below = idle(&mut traffic, 2);
        let runner = warping_out(&mut traffic, 3);
        let above = idle(&mut traffic, 6);
        for id in [below, above] {
            let npc = traffic.npcs_mut().iter_mut().find(|npc| npc.id == id);
            npc.expect("there").target = Some(ShipRef::Npc(runner));
        }
        let watch = Watch::default();
        think(&mut traffic, &watch);
        assert!(of(&traffic, runner).is_none(), "gone at its slot");
        assert_eq!(watch.seen(below), (vec![], false), "its run came first");
        assert_eq!(watch.seen(above), (vec![runner], true), "@0x859b8");
    }

    /// A fighter in slot `berth` at its carrier, in slot `carrier_berth`,
    /// within its reach: the fighter and the carrier.
    fn docking(traffic: &mut Traffic, berth: u32, carrier_berth: u32) -> (NpcId, NpcId) {
        let carrier =
            traffic.add_npc_at(ship(0, PIRATES, AiType::Warship, 100.0, 0.0), carrier_berth);
        let mut fighter = ship(2, PIRATES, AiType::Interceptor, 110.0, 0.0);
        let bay = ShipRef::Npc(carrier);
        fighter.goal = Goal::Dock(bay);
        fighter.carrier = Some(Carrier {
            ship: bay,
            window: 100.0,
            reach: 30.0,
        });
        (traffic.add_npc_at(fighter, berth), carrier)
    }

    #[test]
    fn a_fighter_inside_its_carriers_reach_docks_at_its_slot() {
        let mut traffic = Traffic::with_interval(1);
        let (fighter, carrier) = docking(&mut traffic, 3, 8);
        let above = idle(&mut traffic, 5);
        let watch = Watch::default();
        think(&mut traffic, &watch);
        assert!(of(&traffic, fighter).is_none(), "aboard at its slot");
        assert_eq!(watch.seen(above).0, [fighter], "@0x80404");
        traffic.fly_holding(world(), &BTreeSet::new(), &mut NeverFires);
        let docked = Outcome::Docked {
            carrier: ShipRef::Npc(carrier),
            ship: crate::catalog::ShipId(128),
        };
        assert_eq!(traffic.departed(), [(fighter, docked)]);
    }

    #[test]
    fn a_fighter_with_no_bay_stays_out_at_its_slot() {
        // `_AIAddFighterToParent` finds no bay and writes nothing (@0x80384).
        let mut traffic = Traffic::with_interval(1);
        let (fighter, _) = docking(&mut traffic, 3, 8);
        let watch = Watch::default();
        traffic.open_think(&watch, world(), &[], &mut NeverFires);
        let no_bay = NoBay::from([fighter]);
        let left = traffic.think_run(&watch, world(), |_| true, &no_bay, &mut NeverFires);
        assert_eq!(left, []);
        assert!(of(&traffic, fighter).is_some(), "still out");
        let left = traffic.think_run(&watch, world(), |_| true, &NoBay::new(), &mut NeverFires);
        assert_eq!(left.len(), 1, "with a bay it docks");
    }

    #[test]
    fn a_fighter_in_reach_on_double_times_second_pass_stays_out_and_docks_at_its_next_slot() {
        // The second pass runs `_HandleShip` alone (@0x44350), no
        // `_AIDispatch`, so its low level docks nothing.
        let mut traffic = Traffic::with_interval(1);
        let (fighter, _) = docking(&mut traffic, 3, 8);
        traffic.move_in(world(), &mut NeverFires);
        assert!(of(&traffic, fighter).is_some(), "still out");
        traffic.tick_in(&Watch::default(), world(), &[], &mut NeverFires);
        assert!(of(&traffic, fighter).is_none(), "docked at its slot");
    }

    /// A ship in slot `berth` following `leader`'s jump, its stage not
    /// begun, beside it.
    fn following(traffic: &mut Traffic, leader: NpcId, berth: u32) -> NpcId {
        let mut follower = ship(0, TRADERS, AiType::WimpyTrader, 2000.0, 50.0);
        follower.leader = Some(leader);
        follower.goal = Goal::Idle;
        follower.follow_leaders_jump(ShipRef::Npc(leader));
        traffic.add_npc_at(follower, berth)
    }

    #[test]
    fn a_follower_below_its_jumping_leader_reads_its_stage_as_the_tick_began_and_one_above_after_its_step()
     {
        // Sub 0xd reads the leader's +0x50 live (@0x859c5-0x85c5f): at 1.0
        // as the tick began it keeps formation, at 2.0 after the leader's
        // sub 4 step it starts its own stage.
        let mut traffic = Traffic::with_interval(1);
        let leader = warping_out(&mut traffic, 3);
        let npc = traffic.npcs_mut().iter_mut().find(|npc| npc.id == leader);
        let npc = npc.expect("there");
        npc.warp.stage = 1.0;
        npc.warp.since = 0;
        let below = following(&mut traffic, leader, 2);
        let above = following(&mut traffic, leader, 4);
        traffic.tick_in(&Watch::default(), world(), &[], &mut NeverFires);
        let stage = |id| of(&traffic, id).expect("there").warp.stage;
        assert_eq!(stage(leader), 2.0, "its own step");
        assert_eq!(stage(below), 0.0, "it read 1.0: formation");
        assert_eq!(stage(above), 2.0, "it read 2.0: its stage begun");
    }

    #[test]
    fn on_its_leaders_departure_tick_a_follower_above_it_reads_the_freed_record() {
        // @0x859ae-0x859bf: +0x50 0, sub 4 kept, so goal 0xb's test still
        // finds it leaving and sub 0xd keeps formation on it.
        let mut traffic = Traffic::with_interval(1);
        let leader = warping_out(&mut traffic, 3);
        let below = following(&mut traffic, leader, 2);
        let above = following(&mut traffic, leader, 4);
        traffic.tick_in(&Watch::default(), world(), &[], &mut NeverFires);
        assert!(of(&traffic, leader).is_none(), "jumped out");
        let follower = of(&traffic, above).expect("there");
        let follows = Some(WarpGoal::Follow(ShipRef::Npc(leader)));
        assert_eq!(follower.warp.goal, follows, "not on its own jump");
        assert_eq!(follower.warp.stage, 0.0, "formation");
        let follower = of(&traffic, below).expect("there");
        assert_eq!(follower.warp.goal, follows);
        assert_eq!(follower.warp.stage, 2.0, "it read 3.0: its stage begun");
    }

    #[test]
    fn a_departure_at_a_slot_is_read_only_in_that_ticks_think() {
        // Every slot's `_HandleShip` lets go of it after the loop
        // (@0x33865-0x33888), so no later tick's think reads it.
        let mut traffic = Traffic::with_interval(1);
        let runner = warping_out(&mut traffic, 2);
        let above = idle(&mut traffic, 5);
        let npc = traffic.npcs_mut().iter_mut().find(|npc| npc.id == above);
        npc.expect("there").target = Some(ShipRef::Npc(runner));
        let watch = Watch::default();
        traffic.tick_in(&watch, world(), &[], &mut NeverFires);
        assert_eq!(watch.seen(above).0, [runner], "that tick's");
        traffic.tick_in(&watch, world(), &[], &mut NeverFires);
        assert_eq!(watch.seen(above).0, [], "the next tick's");
    }

    use std::collections::BTreeSet;
}
