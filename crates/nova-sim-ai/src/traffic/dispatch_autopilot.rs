//! The autopilot's low level at the ship's own slot of `_AIDispatch`'s
//! loop, which leaves the word `_HandleShip` flies at the ship's step (in
//! the `EV Nova` executable; see [`slot_word`](super::slot_word)).
//!
//! `_AIDispatch` calls `_HighLevelAIHandler` (@0x901e5) and tail-jumps to
//! `_LowLevelAIHandler` (@0x901f4) for every ship the AI loop reaches but
//! an escape pod, which writes its own word in `_AIDispatch`
//! (@0x8fb62-0x8fbba), and while the frozen flag *0x3b523c is set
//! (@0x8fbbf-0x8fbc7, never in play). No other return comes before the
//! tail jump: `_AIDispatch`'s +0x4c reads (@0x8fca1, @0x90063) skip only
//! the cloak think and the bay's launch, and the high level's own wait
//! gate (@0x8d4b4-0x8d4d8) returns into the tail jump, so the low level
//! runs under the AI wait too, flying whatever sub-goal +0xc8ca kept.
//!
//! [`Traffic::autopilot_in_dispatch`] runs it at the ship's slot, after
//! the docking's low level and the ship's leaves
//! ([`dispatch_leave`](super::dispatch_leave)), for every reached ship
//! that stays, isn't held still and flew no docking:
//!
//! - It reads at the slot, live: the ship its goal is about as the lower
//!   slots left it (their low levels' writes in, no ship moved yet), a
//!   ship gone by its record for a ship whose high level was skipped
//!   ([`skipped_high_level`](super::skipped_high_level)), the player as
//!   the tick began, its own handling as it stands (its charge before the
//!   shot loop's `_IonizeShip` @0x3612d and the step's shed), its swarm
//!   lead (a freed slot by its record), its sight of its leader (one that
//!   left in the loop seen as it left, as `_ShipVisibleToShip` tests no
//!   +0xb8), its guard, asteroid slot 0, the nearest box and the regroup
//!   pause.
//! - It writes there what the low level writes: the word, kept per ship
//!   ([`Traffic::keep_word`]), the berth's last heading with it, the
//!   high level's sub-goal +0xc8ca as close formation and the mining
//!   order read it ([`Npc::close_form`], [`Npc::mine_order`]), and the
//!   direct writes of each flight (see
//!   [`autopilot::low_level`]). So a higher slot's think reads them that
//!   tick, and a lower one's as the tick began.
//! - A ship whose flight is `_HandleShip`'s own (a gate's waits, glide and
//!   burst, a disabled ship's drift, a warp the warp-out flies,
//!   [`warp_out::flies_jump`]) or whose AI wait runs leaves the head's
//!   word ([`SlotWord::none`]).
//! - A flight that ends at the slot (a landing, state 1 @0x8d8b8 on, or a
//!   jump begun, state 2 @0x8dd99) is noted for its step, which reports it
//!   and flies nothing on the advance pass. The landing's mode is written
//!   at the slot, so a higher slot's think reads it landed that tick; the
//!   traffic's report of it lags to the ship's step within the tick, which
//!   crosses no boundary the player sees.
//!
//! Double time's second pass runs no `_AIDispatch` (`je 0x44350`
//! @0x4402a): it flies the advance pass's word again.
//!
//! **Left out:** under its AI wait the original's low level flies the
//! sub-goal +0xc8ca kept, and its direct writes (sub 1's settle, say)
//! show, though `_HandleShip` skips the turn and thrust; the engine
//! doesn't keep +0xc8ca for an autopilot goal, so a waiting ship leaves
//! the head's word and writes nothing
//! (rdm:phase/npc-ai/phase-6-left-out-of-phase-19).
//! Nothing here plays a sound.

use super::autopilot::{self, Beside, Readings, TopSpeeds};
use super::npc::Npc;
use super::slot_word::SlotWord;
use super::warp_out;
use super::{Traffic, World};
use crate::ai::mine_orders;
use crate::ai::sight;
use crate::ai::{Goal, Surroundings, swarm};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::flight::ShipState;
use crate::traffic::inertialess::Inertia;
use crate::trig;

impl Traffic {
    /// The word NPC `id`'s low level left at its last slot, which its step
    /// flies ([`slot_word`](super::slot_word)); none since its spawn.
    #[must_use]
    pub fn word_of(&self, id: super::npc::NpcId) -> Option<SlotWord> {
        self.words.get(&id).copied()
    }

    /// The autopilot's low level of the NPC at `index` in `world`, at its
    /// slot, rolling on `chance` (see the module docs): the word it leaves
    /// is kept for both of the tick's fly halves, and a flight that ended
    /// at the slot is noted for its step.
    pub(super) fn autopilot_in_dispatch(
        &mut self,
        index: usize,
        world: &World,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let id = self.npcs[index].id;
        // By the goal it flies, as its step's warp-out reads it: the regroup
        // pause's kept own jump is the warp-out's too.
        if self.npcs[index].fly_as_flown(|npc| warp_out::flies_jump(npc)) {
            self.keep_head_word(index, world);
            return;
        }
        let (other, other_disabled, beside) = self.reads_at_slot(index, world);
        #[cfg(test)]
        self.slot_reads.insert(id, (other, other_disabled, beside));
        let npc = &self.npcs[index];
        let speeds = TopSpeeds {
            own: npc.top_speed(world.led_speed),
            lead: None,
        };
        let readings = Readings {
            snipe_lead: world.snipe_brake_lead,
            jump_fuel: world.npc_jump_fuel,
            held_burst: world.held_burst,
            npc_landing: world.npc_landing,
            speed_mult: world.speed_mult,
        };
        let rules = (world.flight, readings, world.weapons);
        let npc = &mut self.npcs[index];
        let (left, outcome) = npc.fly_as_flown(|npc| {
            // The sub-goal this low level flies, stored for the next frame's
            // sight and cloak think (`_HighLevelAIHandler` writes +0xc8ca).
            npc.close_form =
                npc.keeps_close_formation(other.as_ref(), beside.leader_seen, world.flight);
            // And goal 0x10's order, from where the flight starts (state 16
            // @0x8f832-0x8f903): order 1 in any other goal.
            npc.mine_order = if npc.goal == Goal::Mine {
                mine_orders::order(npc, beside.rock.as_ref())
            } else {
                mine_orders::MineOrder::Stop
            };
            let quarry = (other.as_ref(), other_disabled, beside);
            let sites = world.flown_among(npc.goal);
            autopilot::low_level(npc, sites, world.type_widths, quarry, speeds, rules, chance)
        });
        self.keep_word(index, left);
        if let Some(outcome) = outcome {
            self.slot_outcomes.insert(id, outcome);
        }
    }

    /// What the low level of the NPC at `index` reads at its slot in
    /// `world` (see the module docs): the ship its flight is about and
    /// whether it is disabled ([`Traffic::quarry_at_slot`]), and what it
    /// reads beside it ([`Beside`]). The swarm lead and the sight read the
    /// goal the ship holds; the rest the one it flies
    /// ([`Npc::fly_as_flown`]).
    pub(super) fn reads_at_slot(
        &self,
        index: usize,
        world: &World,
    ) -> (Option<ShipState>, bool, Beside) {
        let npc = &self.npcs[index];
        let around = Surroundings {
            gone: self.gone.ships(),
            ..world.around(&self.npcs)
        };
        let swarm = swarm::lead(npc, &around);
        let seen = self.sees_leader_at_slot(npc, world);
        let mut flown = npc.clone();
        flown.goal = npc.flown_goal();
        let (other, other_disabled) = self.quarry_at_slot(&flown, world);
        let beside = Beside {
            swarm,
            rock: world.mining_rock,
            scoop_box: mine_orders::nearest_box(npc.state.position, world.boxes),
            ..self.guard_at_slot(&flown, seen, world)
        };
        (other, other_disabled, beside)
    }

    /// Each NPC's low level at its slot, in berth order, in `world`, rolling
    /// on `chance`, with none of the high level's work: the docking's
    /// ([`dispatch_dock`](super::dispatch_dock)) or else the autopilot's.
    /// For the tests of a flight that would otherwise run a think.
    #[cfg(test)]
    pub(crate) fn low_levels(&mut self, world: &World, chance: &mut (impl Chance + ?Sized)) {
        self.open_dispatch();
        for index in super::launch_room::berth_order(&self.npcs) {
            if !self.dock_in_dispatch(index, world, chance) {
                self.autopilot_in_dispatch(index, world, chance);
            }
        }
    }

    /// A tick of `world` with no think: each NPC's low level at its slot
    /// ([`Traffic::low_levels`]) then the fly half
    /// ([`Traffic::fly_holding`]), rolling on `chance`.
    #[cfg(test)]
    pub(crate) fn fly_tick(&mut self, world: &World, chance: &mut (impl Chance + ?Sized)) {
        self.low_levels(world, chance);
        self.fly_holding(*world, &std::collections::BTreeSet::new(), chance);
    }

    /// Keeps `word`, the NPC at `index`'s low level's, for its flight
    /// until its next slot, and its heading wanted (+0x68) as its berth's
    /// last ([`slot_word::spawn_heading`](super::slot_word::spawn_heading));
    /// a word that keeps the ship's heading, the port's earlier formation,
    /// writes the ship's whole heading.
    pub(super) fn keep_word(&mut self, index: usize, word: SlotWord) {
        let npc = &self.npcs[index];
        let heading = word
            .wanted()
            .unwrap_or_else(|| trig::whole(npc.state.heading));
        self.berth_headings.write(npc.berth, Some(heading));
        self.words.insert(npc.id, word);
    }

    /// Keeps the head's word for the NPC at `index` in `world`
    /// ([`SlotWord::none`]): a ship whose low level wrote no other, as
    /// sub 0xd's detach and sub 4 under way, which the warp-out flies.
    pub(super) fn keep_head_word(&mut self, index: usize, world: &World) {
        let npc = &self.npcs[index];
        let handling = npc.flying_handling(npc.top_speed(world.led_speed), world.flight);
        let inertia = Inertia::of(npc, None, true, world.flight);
        let head = SlotWord::none(npc, &handling, inertia);
        self.keep_word(index, head);
    }

    /// Where `ship` is for `reader`'s low level at its slot in `world`: the
    /// player as the tick began, an NPC as it stands, a gone one by its
    /// record for a reader whose high level was skipped and that held it
    /// at its think ([`skipped_high_level`](super::skipped_high_level)).
    fn state_at_slot(&self, ship: ShipRef, reader: &Npc, world: &World) -> Option<ShipState> {
        match ship {
            ShipRef::Player => world.player.map(|player| player.state),
            ShipRef::Npc(id) => (self.npcs.iter())
                .find(|npc| npc.id == id)
                .map(|npc| npc.state)
                .or_else(|| self.skipped.state_of(reader.id, id)),
        }
    }

    /// Whether `ship` is disabled at the slot, in `world`.
    fn disabled_at_slot(&self, ship: ShipRef, world: &World) -> bool {
        match ship {
            ShipRef::Player => world
                .player
                .is_some_and(|player| player.condition == Condition::Disabled),
            ShipRef::Npc(id) => {
                (self.npcs.iter()).any(|npc| npc.id == id && npc.condition == Condition::Disabled)
            }
        }
    }

    /// The ship `npc`'s flight is about (the one it docks with, follows,
    /// escorts, fights or inspects), where its slot reads it in `world`,
    /// and whether it is disabled. A docking's ship gone from the system
    /// is read by its slot's record, as the nav (+0x6c) still names the
    /// slot (@0x86a19-0x86a51; see [`repair_dock`](super::repair_dock)).
    fn quarry_at_slot(&self, npc: &Npc, world: &World) -> (Option<ShipState>, bool) {
        if let Some(docking) = npc.docking {
            let state =
                self.state_at_slot(docking.ship, npc, world)
                    .or_else(|| match docking.ship {
                        ShipRef::Npc(id) => (self.gone.ships().iter())
                            .find(|gone| gone.ship.id == id)
                            .map(|gone| gone.ship.state),
                        ShipRef::Player => None,
                    });
            return (state, self.disabled_at_slot(docking.ship, world));
        }
        let other = match npc.goal {
            Goal::Follow { lead, .. } => Some(ShipRef::Npc(lead)),
            Goal::Assist(_) | Goal::Formation { .. } => Some(ShipRef::Player),
            Goal::Dock(carrier) => Some(carrier),
            goal => goal.quarry().or(goal.inspecting()),
        };
        let disabled = other.is_some_and(|ship| self.disabled_at_slot(ship, world));
        (
            other.and_then(|ship| self.state_at_slot(ship, npc, world)),
            disabled,
        )
    }

    /// Whether `npc` sees its leader at its slot in `world`
    /// (`_ShipVisibleToShip(leader, ship)` @0x8ed5f), or, returning to its
    /// carrier ([`Goal::Dock`]), its carrier (@0x8ed00), read after the
    /// turn-back, which comes first in the one high level (@0x8d608): a
    /// leader that left at its slot of this tick's loop seen as it left, as
    /// `_ShipVisibleToShip` tests no +0xb8 (@0x971c-0x97fe).
    pub(super) fn sees_leader_at_slot(&self, npc: &Npc, world: &World) -> bool {
        let player = world.player;
        let with_left: Vec<Npc>;
        let npcs: &[Npc] = if self.left_in_dispatch.is_empty() {
            &self.npcs
        } else {
            with_left = (self.npcs.iter())
                .chain(&self.left_in_dispatch)
                .cloned()
                .collect();
            &with_left
        };
        if let Goal::Dock(carrier) = npc.goal {
            return world.around(npcs).sees(npc, carrier);
        }
        sight::sees_leader(npc, npcs, player.as_ref())
    }

    /// What `npc`'s flight reads of its guard and its leader at its slot in
    /// `world`, its leader seen as `seen` says ([`Beside`]): its guard while
    /// it keeps formation ([`Goal::Formation`]) or keeps with a lead
    /// ([`Goal::Follow`]), and its sight of its leader, or of its carrier
    /// in [`Goal::Dock`]; neither for any other goal. The coast and the inertia
    /// ([`Inertia::of`]) read the same sight.
    fn guard_at_slot(&self, npc: &Npc, seen: bool, world: &World) -> Beside {
        let (guard, leader_seen) = match npc.goal {
            Goal::Formation { guard } | Goal::Follow { guard, .. } => (guard, seen),
            Goal::Dock(_) => (None, seen),
            _ => (None, true),
        };
        Beside {
            guard: guard.and_then(|guard| self.state_at_slot(guard, npc, world)),
            leader_seen,
            ..Beside::default()
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeSet;

    use crate::ai::fixture::{PIRATES, TRADERS, ship};
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::chance::{Chance, NeverFires};
    use crate::combat::ShipRef;
    use crate::traffic::npc::{AiType, Npc, NpcId};
    use crate::traffic::{Traffic, World};

    /// Every ship keeps its goal.
    #[derive(Debug)]
    struct Keeps;

    impl Behaviour for Keeps {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }
    }

    fn of(traffic: &Traffic, id: NpcId) -> &Npc {
        (traffic.npcs().iter())
            .find(|npc| npc.id == id)
            .expect("there")
    }

    /// A trader 2000 up the screen in slot 5, and an attacker at the
    /// centre in slot 3 facing 90° off it, moving (1, 0): the attacker.
    fn off_its_target(traffic: &mut Traffic) -> NpcId {
        let target = traffic.add_npc_at(ship(0, TRADERS, AiType::WimpyTrader, 0.0, -2000.0), 5);
        let mut attacker = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
        attacker.goal = Goal::Attack(ShipRef::Npc(target));
        attacker.target = Some(ShipRef::Npc(target));
        attacker.state.heading = 90.0;
        attacker.state.velocity = crate::geometry::Vec2::new(1.0, 0.0);
        traffic.add_npc_at(attacker, 3)
    }

    #[test]
    fn the_low_level_leaves_its_word_at_its_slot_and_moves_nothing() {
        let mut traffic = Traffic::with_interval(1);
        let attacker = off_its_target(&mut traffic);
        let before = of(&traffic, attacker).state;
        traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
        assert_eq!(of(&traffic, attacker).state, before, "unmoved, unturned");
        let word = traffic.word_of(attacker).expect("kept");
        assert_eq!(word.wanted(), Some(0), "the bearing of its target");
        assert!(!word.thrusts(), "90° off: outside its window");
    }

    #[test]
    fn a_ship_the_loop_doesnt_reach_keeps_its_last_word() {
        // A ship held still runs no low level: the word its last slot
        // left stands, as +0x30 and +0x68 do.
        let mut traffic = Traffic::with_interval(1);
        let attacker = off_its_target(&mut traffic);
        traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
        let kept = traffic.word_of(attacker);
        let still = BTreeSet::from([attacker]);
        traffic.tick_holding(&Keeps, World::new(&[]), &[], &still, &mut NeverFires);
        traffic.npcs_mut()[1].state.heading = 270.0;
        traffic.tick_holding(&Keeps, World::new(&[]), &[], &still, &mut NeverFires);
        assert_eq!(traffic.word_of(attacker), kept);
    }

    #[test]
    fn each_spawn_has_no_word_until_its_first_slot() {
        let mut traffic = Traffic::with_interval(1);
        let attacker = off_its_target(&mut traffic);
        assert_eq!(traffic.word_of(attacker), None);
    }

    #[test]
    fn a_ship_whose_warp_the_warp_out_flies_leaves_the_heads_word() {
        // Sub 4 under way writes no +0x68 of its own here: the head's own
        // whole heading and no thrust stand, where its goal's brake would
        // want its retro heading.
        let mut traffic = Traffic::with_interval(1);
        let mut runner = ship(0, TRADERS, AiType::WimpyTrader, 2000.0, 0.0);
        runner.goal = Goal::JumpOut;
        runner.begin_own_jump();
        runner.state.heading = 90.4;
        runner.state.velocity = crate::geometry::Vec2::new(1.0, 0.0);
        let runner = traffic.add_npc_at(runner, 3);
        let index = traffic.npcs().iter().position(|npc| npc.id == runner);
        traffic.autopilot_in_dispatch(index.expect("there"), &World::new(&[]), &mut NeverFires);
        let word = traffic.word_of(runner).expect("kept");
        assert_eq!(word.wanted(), Some(90));
        assert!(!word.thrusts());
    }

    #[test]
    fn the_low_level_stores_the_order_its_high_level_gave_a_miner() {
        use crate::ai::mine_orders::MineOrder;
        use crate::geometry::Vec2;
        use nova_sim_combat::asteroid::Asteroid;
        // `_HighLevelAIHandler` writes the sub-goal +0xc8ca (@0x901e5) that
        // the next think's terms read (@0x834d6, @0x835e9-0x835fb), from
        // where the ship stood before its flight: a box of 120 here.
        let rock = Asteroid {
            position: Vec2::new(1000.0, 0.0),
            velocity: Vec2::ZERO,
            frame: 0.0,
            spin: 0.0,
            strength: 30,
            kind: 0,
        };
        let stored = |goal: Goal, x: f32, rock: Option<Asteroid>| {
            let mut traffic = Traffic::with_interval(1);
            let mut miner = ship(0, TRADERS, AiType::WimpyTrader, x, 0.0);
            miner.goal = goal;
            miner.hull.class_turn = 2.0;
            miner.mine_order = MineOrder::Chase;
            let miner = traffic.add_npc_at(miner, 2);
            let world = World {
                mining_rock: rock,
                ..World::new(&[])
            };
            traffic.autopilot_in_dispatch(0, &world, &mut NeverFires);
            of(&traffic, miner).mine_order
        };
        assert_eq!(stored(Goal::Mine, 0.0, Some(rock)), MineOrder::Chase);
        assert_eq!(stored(Goal::Mine, 880.0, Some(rock)), MineOrder::Kill);
        assert_eq!(stored(Goal::Mine, 0.0, None), MineOrder::Stop, "no rock");
        assert_eq!(
            stored(Goal::Idle, 0.0, Some(rock)),
            MineOrder::Stop,
            "not goal 0x10"
        );
        assert_eq!(
            stored(Goal::Scoop, 0.0, Some(rock)),
            MineOrder::Stop,
            "goal 0x11"
        );
    }

    #[test]
    fn a_docking_ships_gone_victim_is_read_by_its_own_record() {
        use crate::traffic::gone::Freed;
        use crate::traffic::plunder_dock::{DockSub, Docking};
        let mut traffic = Traffic::with_interval(1);
        let mut other = ship(0, TRADERS, AiType::WimpyTrader, -700.0, 0.0);
        other.id = NpcId(40);
        let mut victim = ship(0, TRADERS, AiType::WimpyTrader, 300.0, 0.0);
        victim.id = NpcId(41);
        traffic.left_the_system(other, Freed::InDispatch);
        traffic.left_the_system(victim.clone(), Freed::InDispatch);
        let mut pirate = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
        pirate.docking = Some(Docking {
            ship: ShipRef::Npc(victim.id),
            sub: DockSub::Approach,
        });
        traffic.add_npc_at(pirate, 2);
        let (read, ..) = traffic.reads_at_slot(0, &World::new(&[]));
        assert_eq!(read, Some(victim.state));
    }
}
