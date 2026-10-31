//! The ships gone from the system whose slots another ship's target may
//! still name, as the original's ship table keeps a freed slot's record in
//! the `EV Nova` executable.
//!
//! A ship that jumps out, docks, enters a gate or is destroyed has its
//! slot freed (+0xb8 cleared), but its record, its place (+0x18, +0x1c)
//! among the rest, stays. A ship whose target (+0x70) names that slot keeps
//! it until a slot test clears it: `_HighLevelAIHandler`'s, after the
//! ship's think (@0x8d4e4-0x8d50d), and `_HandleShip`'s, on every frame of
//! every pass (@0x33865-0x33888). Both clear +0x70 when its slot's +0xb8 is
//! clear. So the think reads the gone slot only when it comes before the
//! ship's next `_HandleShip`, and `_DoPlayGameWork` runs the AI for every
//! slot (@0x44270-0x442ae) before `_HandleShip` and `_HandleShipDisplay`
//! for every slot (@0x44468-0x444c4). Which think reads it turns on where
//! the slot was freed ([`Freed`]) and on the slots' order
//! ([`GoneShip::held_by`]). The original's AI interval is 1 at full speed,
//! so every ship thinks on that frame.
//!
//! The brave trader reads the place
//! ([`BraveTrader`](crate::ai::brave::BraveTrader), `_BraveTraderAI`
//! @0x8b616-0x8b68d): a provoked trader attacks a gone attacker within 1250
//! on both axes, or flees it beyond, and the high level then ends that
//! fight ([`fight_on_gone`](crate::traffic::provocation::fight_on_gone)).
//!
//! [`Traffic`](crate::traffic::Traffic) keeps the gone ships in
//! [`GoneShips`]: each departure and destruction is recorded with its
//! record; every run of a tick's think reads them all, and the think's
//! close forgets them
//! ([`Traffic::close_think`](crate::traffic::Traffic::close_think)), as its
//! thinks are the last that can read them; a ship pass with no decisions
//! before it, double
//! time's second ([`Traffic::move_in`](crate::traffic::Traffic::move_in)),
//! forgets those freed in a ship pass or after one, as its `_HandleShip`
//! clears every target that names them.
//!
//! **The let-go** ([`Traffic::let_go_at_step`]) is `_HandleShip`'s test, at
//! each ship's own turn of the ship loop (`nova_sim::Session::ship_loop`),
//! so [`Npc::target`] is what +0x70 is at every point of a pass. It writes
//! +0x70 alone: a fight's goal names the gone ship until the ship's next
//! think. A ship in a later slot than a ship destroyed or entering a gate
//! lets go in that pass, and one in an earlier slot reads the slot in the
//! next pass's think and lets go at its step there. A ship killed by a
//! deadly stellar ([`Traffic::remove_killed`]) is read by every other slot
//! in that next think.
//!
//! **A departure in the AI loop** is read one tick late: the original frees
//! a jump-out in the low level, in the same frame's AI loop, so a later slot
//! reads it on that frame, where the engine flies after the decisions and
//! reads it on the next tick (filed in
//! rdm:phase/combat-ai-fidelity-cross-roadmap/phase-9-slot-order-left-out-of-phase-6).
//! Two things stand in for moving the departure there, and that phase
//! retires them. An earlier slot lets go as the ship leaves
//! ([`Traffic::left_the_system`]), as its turn of the engine's loop has
//! passed. A later slot lets go at its step, but is noted on the record
//! ([`GoneShip::let_go`]), so the think that stands for that frame's still
//! reads the slot.
//!
//! **A slot taken again** ([`Traffic::slot_taken`]) leaves every other
//! ship's +0x70 as it was, so a ship still holding the slot finds it active
//! at its next test and now fights the newcomer, friend or not. It changes
//! what is drawn and breaks no stock mission, so it has no rulebook key.
//!
//! The reads of a stale +0x70 not built here (a cloak's think, a skipped
//! high level, goal 2's write, goal 0xf, the slots taken that
//! [`Traffic::slot_taken`] can't see and the frees in a dialog) are
//! rdm:phase/combat-ai-fidelity-cross-roadmap/phase-11-left-out-of-phase-7's.
//!
//! A ship that lands is not recorded: the original keeps a landed ship in
//! its slot, waiting at the stellar (`_HighLevelAIHandler`
//! @0x8dadc-0x8db5a), and so does the traffic
//! ([`land_flight::stay`](crate::traffic::land_flight::stay)). Under
//! [`RuleKey::LandedLeaves`](crate::RuleKey::LandedLeaves)' other reading
//! a mission ship that lands leaves the system, unrecorded too, as no
//! slot of the original's is freed that way. An escape pod freed
//! (`_AIDispatch` @0x8fbb3) fires nothing, so no target names it.

use crate::combat::ShipRef;
use crate::traffic::Traffic;
use crate::traffic::autopilot::Outcome;
use crate::traffic::npc::{Npc, NpcId};

/// Where a gone ship's slot was freed, which decides which thinks still
/// read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freed {
    /// In the AI loop, by the ship's own dispatch: a jump out at the end
    /// of its warp (`_LowLevelAIHandler` @0x859b8), a fighter docking
    /// (`_LowLevelAIHandler` @0x87fef → `_AIAddFighterToParent` @0x80404).
    /// The ships in later slots think after it on that frame, before their
    /// `_HandleShip`.
    InDispatch,
    /// In the ship loop, by `_HandleShipDisplay`: a gate entry as its fade
    /// ends (@0x2c078), a ship destroyed as its last explosion ends
    /// (@0x2dee0). The ships in earlier slots have run their `_HandleShip`
    /// on that frame, and think next frame before running it again.
    InShipPass,
    /// After the ship loop: a deadly stellar's kill (@0x39dac, from
    /// `_HandleDeadlyStellars` @0x4450e), which never reaches
    /// `_HandleShipDisplay`. No `_HandleShip` runs before the next frame's
    /// thinks, so every other slot reads it there.
    AfterShipLoop,
}

impl Freed {
    /// Where the slot of a ship that left by `outcome` was freed: `None`
    /// for one still flying, and for one that landed, which the original
    /// keeps in its slot (see the module docs).
    #[must_use]
    pub fn by(outcome: Outcome) -> Option<Self> {
        match outcome {
            Outcome::JumpedOut | Outcome::Docked { .. } | Outcome::Freed => Some(Self::InDispatch),
            Outcome::EnteredGate(_) => Some(Self::InShipPass),
            Outcome::Flying | Outcome::Landed(_) => None,
        }
    }
}

/// A ship gone from the system, its record as it left.
#[derive(Clone, Debug, PartialEq)]
pub struct GoneShip {
    /// The ship as it was when its slot was freed: its place, its slot
    /// ([`Npc::berth`]) and its AI type among the rest.
    pub ship: Npc,
    /// Where its slot was freed.
    pub freed: Freed,
    /// The ships that let go of it at their step
    /// ([`Traffic::let_go_at_step`]) though their think still reads it:
    /// see [`Freed::InDispatch`]. The engine flies a departure in the ship
    /// loop, where the original frees it in the AI loop, so the think that
    /// stands for that frame's comes after those ships' step, not before
    /// it. Moving the departure into the AI loop retires this
    /// (rdm:phase/combat-ai-fidelity-cross-roadmap/phase-9-slot-order-left-out-of-phase-6).
    pub let_go: Vec<NpcId>,
}

impl GoneShip {
    /// Whether `reader`'s think still finds this slot in its target
    /// (+0x70), no slot test having cleared it by then: its target names
    /// the ship, or it let go at a step that comes after that think in the
    /// engine ([`GoneShip::let_go`]); and its slot comes after the gone
    /// one's when that was freed in the AI loop ([`Freed::InDispatch`]),
    /// before it when in the ship loop ([`Freed::InShipPass`]), and is any
    /// other when after it ([`Freed::AfterShipLoop`]).
    #[must_use]
    pub fn held_by(&self, reader: &Npc) -> bool {
        (reader.target == Some(ShipRef::Npc(self.ship.id)) || self.let_go.contains(&reader.id))
            && match self.freed {
                Freed::InDispatch => reader.berth > self.ship.berth,
                Freed::InShipPass => reader.berth < self.ship.berth,
                Freed::AfterShipLoop => reader.berth != self.ship.berth,
            }
    }
}

/// The ships gone since the last think closed, whose slots a think may
/// still read (see the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GoneShips(Vec<GoneShip>);

impl GoneShips {
    /// Records `ship`, its slot freed as `freed` says.
    pub fn record(&mut self, ship: Npc, freed: Freed) {
        self.0.push(GoneShip {
            ship,
            freed,
            let_go: Vec::new(),
        });
    }

    /// A ship pass with no decisions before it runs `_HandleShip` for
    /// every slot, clearing each target that names a slot freed before it
    /// in a ship pass or after one. One freed in the AI loop stands for a
    /// read the original made on that frame, before the pass, so it is
    /// kept for the next think (see the module docs).
    pub fn ship_pass(&mut self) {
        self.0.retain(|gone| gone.freed == Freed::InDispatch);
    }

    /// Notes on every record freed in the AI loop that `reader` holds that
    /// it let go at its step ([`GoneShip::let_go`]).
    fn note_let_go(&mut self, reader: &Npc) {
        for gone in &mut self.0 {
            if gone.freed == Freed::InDispatch && gone.held_by(reader) {
                gone.let_go.push(reader.id);
            }
        }
    }

    /// Forgets every ship recorded: a think's close, whose runs are the
    /// last that can read them, or a new system, whose slots hold none of
    /// them.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// The ships recorded, in order.
    #[must_use]
    pub fn ships(&self) -> &[GoneShip] {
        &self.0
    }
}

impl Traffic {
    /// `_HandleShip`'s slot test at NPC `id`'s own step (@0x33865-0x33888):
    /// with a target whose slot is inactive, its target is none. It writes
    /// nothing else: no goal, no provocation, no travel stellar. The player
    /// is active as `player_active` says; an NPC in the system as
    /// [`Condition::active`](crate::combat::hull::Condition::active) says,
    /// so one destroyed at its display is inactive though the take-out
    /// comes later; one left behind by a move ([`Leave::Held`](super::Leave::Held))
    /// for a ship in a lower slot, as its own display frees it; anything
    /// else is inactive.
    ///
    /// A ship still holding a record freed in the AI loop is noted on it
    /// ([`GoneShip::let_go`]), so the think that stands for that frame's
    /// still reads it. Nothing happens for an NPC not in the system.
    pub fn let_go_at_step(&mut self, id: NpcId, player_active: bool) {
        let Some(at) = self.npcs.iter().position(|npc| npc.id == id) else {
            return;
        };
        let reader = &self.npcs[at];
        let active = match reader.target {
            None => return,
            Some(ShipRef::Player) => player_active,
            Some(ShipRef::Npc(ship)) => self.slot_active(ship, reader.berth),
        };
        if !active {
            self.gone.note_let_go(reader);
            self.npcs[at].target = None;
        }
    }

    /// Whether NPC `ship`'s slot is active as a ship in slot `berth` finds
    /// it at its step (see [`Traffic::let_go_at_step`]).
    fn slot_active(&self, ship: NpcId, berth: u32) -> bool {
        match self.npcs.iter().find(|npc| npc.id == ship) {
            Some(npc) => npc.condition.active(),
            None => self
                .left_behind
                .values()
                .flatten()
                .any(|left| left.npc.id == ship && berth < left.npc.berth),
        }
    }

    /// `ship`, flown out of the system as `freed` says, is recorded. One
    /// freed in the AI loop ([`Freed::InDispatch`]) lets go of every ship in
    /// a lower slot at once, standing for the `_HandleShip` the original
    /// runs after that loop (@0x33865-0x33888): their turn of the engine's
    /// ship loop has passed. Moving the departure into the AI loop retires
    /// this sweep
    /// (rdm:phase/combat-ai-fidelity-cross-roadmap/phase-9-slot-order-left-out-of-phase-6).
    pub(super) fn left_the_system(&mut self, ship: Npc, freed: Freed) {
        if freed == Freed::InDispatch {
            let aim = Some(ShipRef::Npc(ship.id));
            for npc in &mut self.npcs {
                if npc.berth < ship.berth && npc.target == aim {
                    npc.target = None;
                }
            }
        }
        self.gone.record(ship, freed);
    }

    /// The ship `id` takes slot `berth`: every ship still holding a record
    /// of that slot ([`GoneShip::held_by`]) finds it active at its next
    /// slot test, so its target is now the new ship, friend or not, and a
    /// fight on the old ship moves to it
    /// ([`Goal::with_quarry`](crate::ai::Goal::with_quarry)). A spawn
    /// rewrites the slot and leaves every other ship's +0x70
    /// (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b), and the engine's
    /// numbers, unlike the slots, are never used again.
    ///
    /// A record freed in the AI loop ([`Freed::InDispatch`]) passes nothing:
    /// every holder's `_HandleShip` runs before the next spawn, and the
    /// record stands only for that frame's dispatch.
    pub(super) fn slot_taken(&mut self, berth: u32, id: NpcId) {
        let new = ShipRef::Npc(id);
        let records = self.gone.ships().iter();
        for gone in records.filter(|gone| gone.ship.berth == berth) {
            if gone.freed == Freed::InDispatch {
                continue;
            }
            let old = Some(ShipRef::Npc(gone.ship.id));
            for npc in self.npcs.iter_mut().filter(|npc| gone.held_by(npc)) {
                npc.target = Some(new);
                if npc.goal.quarry() == old || npc.goal.inspecting() == old {
                    npc.goal = npc.goal.with_quarry(new);
                }
            }
        }
    }

    /// Takes NPC `id`, killed by a deadly stellar, out of the system, if it
    /// is there: its slot freed after the ship loop (@0x39dac), its record
    /// kept for the thinks that still read it ([`Freed::AfterShipLoop`]).
    pub fn remove_killed(&mut self, id: NpcId) {
        if let Some(npc) = self.npcs.iter().find(|npc| npc.id == id) {
            self.gone.record(npc.clone(), Freed::AfterShipLoop);
        }
        self.remove(id);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{ShipId, StellarId};
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;

    fn at_berth(id: u32, berth: u32) -> Npc {
        Npc {
            berth,
            ..npc(id, ShipStats::default())
        }
    }

    fn reader(berth: u32, target: Option<ShipRef>) -> Npc {
        Npc {
            target,
            ..at_berth(9, berth)
        }
    }

    #[test]
    fn a_slot_freed_in_the_ai_loop_is_read_by_later_slots_and_one_freed_in_the_ship_loop_by_earlier()
     {
        let gone = |freed| GoneShip {
            ship: at_berth(2, 5),
            freed,
            let_go: Vec::new(),
        };
        let on_it = Some(ShipRef::Npc(NpcId(2)));
        let dispatch = gone(Freed::InDispatch);
        assert!(dispatch.held_by(&reader(6, on_it)));
        assert!(!dispatch.held_by(&reader(4, on_it)), "its _HandleShip ran");
        let ship_pass = gone(Freed::InShipPass);
        assert!(ship_pass.held_by(&reader(4, on_it)));
        assert!(!ship_pass.held_by(&reader(6, on_it)), "its _HandleShip ran");
        // A ship in the gone one's own slot is the slot's new occupant,
        // neither after nor before it.
        assert!(!dispatch.held_by(&reader(5, on_it)), "its own slot");
        assert!(!ship_pass.held_by(&reader(5, on_it)), "its own slot");
        for other in [None, Some(ShipRef::Player), Some(ShipRef::Npc(NpcId(3)))] {
            assert!(!dispatch.held_by(&reader(6, other)), "{other:?}");
            assert!(!ship_pass.held_by(&reader(4, other)), "{other:?}");
        }
    }

    #[test]
    fn a_slot_freed_after_the_ship_loop_is_read_by_every_other_slot() {
        // No `_HandleShip` runs between a deadly stellar's kill (@0x39dac)
        // and the next frame's thinks.
        let gone = GoneShip {
            ship: at_berth(2, 5),
            freed: Freed::AfterShipLoop,
            let_go: Vec::new(),
        };
        let on_it = Some(ShipRef::Npc(NpcId(2)));
        assert!(gone.held_by(&reader(4, on_it)));
        assert!(gone.held_by(&reader(6, on_it)));
        assert!(!gone.held_by(&reader(5, on_it)), "its own slot");
        assert!(!gone.held_by(&reader(6, None)));
    }

    #[test]
    fn a_ship_that_let_go_of_a_slot_freed_in_the_ai_loop_still_reads_it_from_a_later_slot() {
        // The think that stands for the frame's dispatch comes after the
        // ship's step, which let go (see `GoneShip::let_go`).
        let holder = reader(6, None);
        let gone = |freed| GoneShip {
            ship: at_berth(2, 5),
            freed,
            let_go: vec![holder.id],
        };
        assert!(gone(Freed::InDispatch).held_by(&holder));
        let earlier = reader(4, None);
        assert!(!gone(Freed::InDispatch).held_by(&earlier), "its slot");
        assert!(!gone(Freed::InShipPass).held_by(&holder), "its step ran");
        let other = Npc {
            id: NpcId(10),
            ..reader(6, None)
        };
        assert!(!gone(Freed::InDispatch).held_by(&other), "it never held it");
    }

    #[test]
    fn each_departure_is_freed_where_the_original_frees_it_and_a_landing_is_not() {
        assert_eq!(Freed::by(Outcome::JumpedOut), Some(Freed::InDispatch));
        assert_eq!(
            Freed::by(Outcome::Docked {
                carrier: ShipRef::Player,
                ship: ShipId(128),
            }),
            Some(Freed::InDispatch)
        );
        assert_eq!(Freed::by(Outcome::Freed), Some(Freed::InDispatch));
        assert_eq!(
            Freed::by(Outcome::EnteredGate(StellarId(130))),
            Some(Freed::InShipPass)
        );
        assert_eq!(Freed::by(Outcome::Landed(StellarId(128))), None);
        assert_eq!(Freed::by(Outcome::Flying), None);
    }

    #[test]
    fn a_lone_ship_pass_forgets_a_slot_freed_after_the_ship_loop() {
        let mut gone = GoneShips::default();
        gone.record(at_berth(1, 1), Freed::AfterShipLoop);
        gone.ship_pass();
        assert_eq!(gone.ships(), []);
    }

    #[test]
    fn a_thinks_close_forgets_every_gone_ship_and_a_lone_ship_pass_forgets_those_it_clears() {
        let mut gone = GoneShips::default();
        gone.record(at_berth(1, 1), Freed::InDispatch);
        gone.record(at_berth(2, 2), Freed::InShipPass);
        let mut second = gone.clone();
        second.ship_pass();
        assert_eq!(
            second
                .ships()
                .iter()
                .map(|gone| gone.ship.id)
                .collect::<Vec<_>>(),
            [NpcId(1)]
        );
        assert_eq!(gone.ships().len(), 2);
        gone.clear();
        assert_eq!(gone.ships(), []);
        second.clear();
        assert_eq!(second.ships(), []);
    }

    mod in_traffic {
        use super::*;
        use crate::ai::fixture::{PIRATES, TRADERS, player, ship};
        use crate::ai::{CommCall, Goal, NovaAi};
        use crate::catalog::{DudeId, GovtId, SystemId};
        use crate::chance::NeverFires;
        use crate::combat::hull::Condition;
        use crate::traffic::fixture::calls_on_line;
        use crate::traffic::npc::{AiType, Mode};
        use crate::traffic::table::{ShipKind, SpawnDude, SpawnTable};
        use crate::traffic::{FlyHalf, LeftShip, Traffic, World};
        use nova_sim_base::fixture::{Draws, FAST};
        use std::collections::{BTreeMap, BTreeSet};

        /// A brave trader at the centre, in slot `trader_berth`, provoked
        /// by a warship in slot `attacker_berth` at (`x`, `y`) and fleeing
        /// it, in a traffic deciding every `interval` ticks: the traffic,
        /// the trader and the warship.
        fn fleeing_a_warship(
            (x, y): (f32, f32),
            trader_berth: u32,
            attacker_berth: u32,
            interval: u32,
        ) -> (Traffic, NpcId, NpcId) {
            let mut traffic = Traffic::with_interval(interval);
            let attacker =
                traffic.add_npc_at(ship(0, PIRATES, AiType::Warship, x, y), attacker_berth);
            let mut trader = ship(1, TRADERS, AiType::BraveTrader, 0.0, 0.0);
            trader.provoked = 10.0;
            trader.target = Some(ShipRef::Npc(attacker));
            trader.goal = Goal::Flee(ShipRef::Npc(attacker));
            let trader = traffic.add_npc_at(trader, trader_berth);
            (traffic, trader, attacker)
        }

        fn tick(traffic: &mut Traffic) {
            let world = World {
                player: Some(player(0.0, -5000.0)),
                ..World::new(&[])
            };
            traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::of(&[]));
        }

        fn trader_of(traffic: &Traffic, id: NpcId) -> &Npc {
            traffic
                .npcs()
                .iter()
                .find(|npc| npc.id == id)
                .expect("the trader stays")
        }

        #[test]
        fn a_brave_trader_attacks_where_its_destroyed_attacker_was_within_1250_keeping_its_provocation()
         {
            let (mut traffic, trader, attacker) = fleeing_a_warship((100.0, 0.0), 1, 2, 1);
            traffic.remove_destroyed(attacker);
            tick(&mut traffic);
            let after = trader_of(&traffic, trader).clone();
            assert_eq!(after.goal, Goal::Idle, "@0x8e731");
            assert_eq!(after.target, None, "@0x8d50d");
            assert_eq!(after.provoked, 10.0, "goal 4 with target -1 zeroes nothing");
            let calls: Vec<(NpcId, CommCall)> = calls_on_line(&traffic.take_on_line())
                .into_iter()
                .map(|(id, call, _)| (id, call))
                .collect();
            assert_eq!(
                calls,
                [(trader, CommCall::Distress)],
                "fighting a warship (@0x8b6e3-0x8b71c)"
            );
        }

        #[test]
        fn a_brave_trader_flees_where_its_destroyed_attacker_was_beyond_1250_and_the_flight_ends_its_provocation()
         {
            let (mut traffic, trader, attacker) = fleeing_a_warship((1251.0, 0.0), 1, 2, 1);
            traffic.remove_destroyed(attacker);
            tick(&mut traffic);
            let after = trader_of(&traffic, trader);
            assert_eq!(after.goal, Goal::Idle, "@0x8e228-0x8e23d");
            assert_eq!(after.target, None);
            assert_eq!(after.provoked, 0.0);
            let calls: Vec<CommCall> = calls_on_line(&traffic.take_on_line())
                .into_iter()
                .map(|(_, call, _)| call)
                .collect();
            assert_eq!(calls, [CommCall::Distress, CommCall::Reinforcements]);
        }

        #[test]
        fn a_brave_trader_attacks_where_its_attacker_jumped_out_within_1250_keeping_its_provocation()
         {
            // A wimpy trader provoked by the player far off runs out,
            // begins its warp out on its first tick (see `leave_flight`)
            // and is gone once its warp stage ends (see `warp_out`).
            let mut traffic = Traffic::new();
            let mut runner = ship(0, TRADERS, AiType::WimpyTrader, 0.0, -1001.0);
            runner.goal = Goal::Flee(ShipRef::Player);
            runner.provoked = 10.0;
            runner.target = Some(ShipRef::Player);
            let runner = traffic.add_npc_at(runner, 1);
            let mut trader = ship(1, TRADERS, AiType::BraveTrader, 0.0, -901.0);
            trader.provoked = 10.0;
            trader.target = Some(ShipRef::Npc(runner));
            let trader = traffic.add_npc_at(trader, 2);
            let world = World {
                player: Some(player(251.0, -10.0)),
                ..World::new(&[])
            };
            let warped = (0..600).any(|_| {
                traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::of(&[]));
                !traffic.departed().is_empty()
            });
            assert!(warped, "it warps out");
            assert_eq!(traffic.departed(), [(runner, Outcome::JumpedOut)]);
            assert_eq!(
                trader_of(&traffic, trader).goal,
                Goal::Attack(ShipRef::Npc(runner))
            );
            traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::of(&[]));
            let after = trader_of(&traffic, trader);
            assert_eq!(after.goal, Goal::Idle, "@0x8e731");
            assert_eq!(after.target, None);
            assert_eq!(after.provoked, 10.0);
        }

        /// A ship in slot `berth` of `traffic` aiming at `target`,
        /// provoked, fighting it and with its travel stellar set.
        fn aiming_at(traffic: &mut Traffic, berth: u32, target: ShipRef) -> NpcId {
            let mut holder = ship(1, TRADERS, AiType::BraveTrader, 0.0, 0.0);
            holder.target = Some(target);
            holder.goal = Goal::Attack(target);
            holder.provoked = 10.0;
            holder.travel_set = true;
            traffic.add_npc_at(holder, berth)
        }

        /// A ship in slot `berth` of `traffic`.
        fn in_slot(traffic: &mut Traffic, berth: u32) -> NpcId {
            traffic.add_npc_at(ship(0, PIRATES, AiType::Warship, 100.0, 0.0), berth)
        }

        fn target_of(traffic: &Traffic, id: NpcId) -> Option<ShipRef> {
            trader_of(traffic, id).target
        }

        #[test]
        fn a_ship_lets_go_at_its_step_of_a_target_out_of_the_system_and_keeps_its_goal_and_provocation()
         {
            // `_HandleShip` @0x33865-0x33888 writes +0x70 alone.
            let gone = ShipRef::Npc(NpcId(99));
            let mut traffic = Traffic::new();
            let holder = aiming_at(&mut traffic, 3, gone);
            let before = trader_of(&traffic, holder).clone();
            traffic.let_go_at_step(holder, true);
            assert_eq!(
                trader_of(&traffic, holder),
                &Npc {
                    target: None,
                    ..before
                }
            );
        }

        #[test]
        fn a_ship_lets_go_of_a_target_destroyed_but_not_yet_taken_out() {
            // A final explosion clears +0xb8 at its display (@0x2dee0),
            // where the engine marks the ship and takes it out later.
            let mut traffic = Traffic::new();
            let wreck = in_slot(&mut traffic, 2);
            let holder = aiming_at(&mut traffic, 3, ShipRef::Npc(wreck));
            traffic.npcs[0].condition = Condition::Destroyed;
            traffic.let_go_at_step(holder, true);
            assert_eq!(target_of(&traffic, holder), None);
        }

        #[test]
        fn a_ship_keeps_a_target_breaking_up_disabled_or_landed() {
            // +0xb8 stays set for each.
            let dying = Condition::Dying {
                ticks_left: 5,
                starting: false,
            };
            for (condition, mode) in [
                (dying, Mode::Flying),
                (Condition::Disabled, Mode::Flying),
                (Condition::Intact, Mode::Emerging { ticks_left: 5 }),
            ] {
                let mut traffic = Traffic::new();
                let other = in_slot(&mut traffic, 2);
                let holder = aiming_at(&mut traffic, 3, ShipRef::Npc(other));
                traffic.npcs[0].condition = condition;
                traffic.npcs[0].mode = mode;
                traffic.let_go_at_step(holder, true);
                assert_eq!(
                    target_of(&traffic, holder),
                    Some(ShipRef::Npc(other)),
                    "{condition:?}"
                );
            }
        }

        #[test]
        fn a_ship_lets_go_of_the_player_only_once_its_slot_is_inactive() {
            let mut traffic = Traffic::new();
            let holder = aiming_at(&mut traffic, 3, ShipRef::Player);
            traffic.let_go_at_step(holder, true);
            assert_eq!(target_of(&traffic, holder), Some(ShipRef::Player));
            traffic.let_go_at_step(holder, false);
            assert_eq!(target_of(&traffic, holder), None);
        }

        #[test]
        fn a_ship_with_no_target_or_out_of_the_system_lets_go_of_nothing() {
            let mut traffic = Traffic::new();
            let idle = in_slot(&mut traffic, 2);
            let before = traffic.clone();
            traffic.let_go_at_step(idle, false);
            traffic.let_go_at_step(NpcId(99), false);
            assert_eq!(traffic.npcs(), before.npcs());
        }

        #[test]
        fn a_target_left_behind_is_let_go_by_a_later_slot_and_kept_by_an_earlier() {
            // Its slot is freed at its own display (@0x2b7e0 -> @0x2dee0),
            // so a later slot finds it inactive, an earlier one active.
            let left = NpcId(99);
            let mut traffic = Traffic::new();
            let earlier = aiming_at(&mut traffic, 2, ShipRef::Npc(left));
            let later = aiming_at(&mut traffic, 6, ShipRef::Npc(left));
            let own_slot = aiming_at(&mut traffic, 4, ShipRef::Npc(left));
            traffic.left_behind.insert(
                SystemId(130),
                vec![LeftShip {
                    npc: Npc {
                        id: left,
                        ..at_berth(99, 4)
                    },
                    person: None,
                }],
            );
            traffic.let_go_at_step(earlier, true);
            traffic.let_go_at_step(later, true);
            traffic.let_go_at_step(own_slot, true);
            assert_eq!(target_of(&traffic, earlier), Some(ShipRef::Npc(left)));
            assert_eq!(target_of(&traffic, later), None);
            assert_eq!(target_of(&traffic, own_slot), None, "not below it");
            // The pass's end frees it.
            traffic.left_behind.clear();
            traffic.let_go_at_step(earlier, true);
            assert_eq!(target_of(&traffic, earlier), None);
        }

        /// A ship in slot `berth` of `traffic` on the last pass of its warp
        /// out: its next flight departs it.
        fn warping_out(traffic: &mut Traffic, berth: u32) -> NpcId {
            let mut runner = ship(0, TRADERS, AiType::WimpyTrader, 0.0, -2000.0);
            runner.goal = Goal::JumpOut;
            runner.begin_own_jump();
            runner.warp.stage = 3.0;
            runner.warp.since = 1000;
            runner.warp.aim = 90;
            traffic.add_npc_at(runner, berth)
        }

        /// A fighter in slot `berth` of `traffic`, at its carrier in slot
        /// `carrier_berth`: its next flight docks it.
        fn docking(traffic: &mut Traffic, berth: u32, carrier_berth: u32) -> NpcId {
            let carrier = in_slot(traffic, carrier_berth);
            let mut fighter = ship(2, TRADERS, AiType::WimpyTrader, 100.0, 0.0);
            let bay = ShipRef::Npc(carrier);
            fighter.goal = Goal::Dock(bay);
            fighter.carrier = Some(crate::bay::Carrier {
                ship: bay,
                window: 100.0,
                reach: 30.0,
            });
            traffic.add_npc_at(fighter, berth)
        }

        /// A ship in slot `berth` of `traffic` whose gate wait ends on its
        /// next flight.
        fn entering_a_gate(traffic: &mut Traffic, berth: u32) -> NpcId {
            let mut entering = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
            entering.mode = Mode::EnteringGate {
                gate: StellarId(130),
                ticks_left: 1,
            };
            traffic.add_npc_at(entering, berth)
        }

        /// NPC `id`'s flight, alone, in a fly half of `traffic`.
        fn fly(traffic: &mut Traffic, id: NpcId) {
            let world = World::new(&[]);
            let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
            traffic.fly_npc(&mut pass, id, &world, &mut NeverFires);
        }

        #[test]
        fn a_jump_out_or_dock_lets_go_of_the_earlier_slots_and_leaves_the_later_to_their_step() {
            // The original frees these in the AI loop (@0x859b8, @0x80404),
            // before the `_HandleShip` of every slot.
            for dock in [false, true] {
                let mut traffic = Traffic::new();
                let leaving = if dock {
                    docking(&mut traffic, 3, 8)
                } else {
                    warping_out(&mut traffic, 3)
                };
                let aim = ShipRef::Npc(leaving);
                let earlier = aiming_at(&mut traffic, 2, aim);
                let later = aiming_at(&mut traffic, 6, aim);
                let elsewhere = aiming_at(&mut traffic, 1, ShipRef::Player);
                let own_slot = aiming_at(&mut traffic, 3, aim);
                fly(&mut traffic, leaving);
                assert_eq!(traffic.departed().len(), 1, "{dock}: it left");
                assert_eq!(target_of(&traffic, earlier), None, "{dock}");
                assert_eq!(target_of(&traffic, own_slot), Some(aim), "not below");
                assert_eq!(
                    trader_of(&traffic, earlier).goal,
                    Goal::Attack(aim),
                    "{dock}: +0x70 alone"
                );
                assert_eq!(target_of(&traffic, later), Some(aim), "{dock}");
                assert_eq!(
                    target_of(&traffic, elsewhere),
                    Some(ShipRef::Player),
                    "{dock}"
                );
                traffic.let_go_at_step(later, true);
                assert_eq!(target_of(&traffic, later), None, "{dock}: its step");
            }
        }

        #[test]
        fn a_gate_entry_lets_go_of_no_earlier_slot() {
            // `_HandleShipDisplay` frees it (@0x2c078), after the earlier
            // slots' `_HandleShip`.
            let mut traffic = Traffic::new();
            let leaving = entering_a_gate(&mut traffic, 3);
            let aim = ShipRef::Npc(leaving);
            let earlier = aiming_at(&mut traffic, 2, aim);
            fly(&mut traffic, leaving);
            assert_eq!(traffic.departed().len(), 1, "it left");
            assert_eq!(target_of(&traffic, earlier), Some(aim));
            assert!(traffic.gone.ships()[0].held_by(trader_of(&traffic, earlier)));
        }

        #[test]
        fn a_later_slot_that_let_go_of_a_ship_gone_in_the_ai_loop_still_reads_it_in_the_next_think()
        {
            // The trader lets go at its step, after the flight that stands
            // for the jump out, but the original's trader read the slot in
            // that frame's dispatch, before its step.
            let mut traffic = Traffic::new();
            let runner = warping_out(&mut traffic, 1);
            let mut trader = ship(1, TRADERS, AiType::BraveTrader, 0.0, -1100.0);
            trader.provoked = 10.0;
            trader.target = Some(ShipRef::Npc(runner));
            let trader = traffic.add_npc_at(trader, 2);
            fly(&mut traffic, runner);
            traffic.let_go_at_step(trader, true);
            assert_eq!(target_of(&traffic, trader), None);
            let world = World {
                player: Some(player(251.0, -10.0)),
                ..World::new(&[])
            };
            traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::of(&[]));
            let after = trader_of(&traffic, trader);
            assert_eq!(after.goal, Goal::Idle, "@0x8e731");
            assert_eq!(after.target, None);
            assert_eq!(after.provoked, 10.0, "attacked near: it keeps it");
        }

        #[test]
        fn a_step_notes_only_a_ship_that_held_a_slot_freed_in_the_ai_loop() {
            // Another gone ship's record is no record of what this ship let
            // go of.
            let mut traffic = Traffic::new();
            let runner = warping_out(&mut traffic, 3);
            let holder = aiming_at(&mut traffic, 6, ShipRef::Npc(NpcId(99)));
            fly(&mut traffic, runner);
            traffic.let_go_at_step(holder, true);
            assert_eq!(traffic.gone.ships()[0].let_go, []);
            // A slot freed in the ship loop is read only while the ship
            // holds it: its step is the read's end.
            let mut traffic = Traffic::new();
            let leaving = entering_a_gate(&mut traffic, 3);
            let earlier = aiming_at(&mut traffic, 2, ShipRef::Npc(leaving));
            fly(&mut traffic, leaving);
            traffic.let_go_at_step(earlier, true);
            assert_eq!(traffic.gone.ships()[0].let_go, []);
        }

        #[test]
        fn a_slot_taken_again_passes_its_holders_to_the_new_ship() {
            // A spawn rewrites the slot and leaves every other ship's +0x70
            // (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b), so the holder's
            // slot test passes with the newcomer in it.
            let old = ShipRef::Npc(NpcId(0));
            for goal in [
                Goal::Attack(old),
                Goal::Snipe(old),
                Goal::Flee(old),
                Goal::Inspect(old),
                Goal::Plunder {
                    ship: old,
                    hopeless: false,
                },
            ] {
                let mut traffic = Traffic::new();
                let wreck = in_slot(&mut traffic, 4);
                assert_eq!(ShipRef::Npc(wreck), old);
                let holder = aiming_at(&mut traffic, 2, old);
                traffic.npcs[1].goal = goal;
                traffic.remove_destroyed(wreck);
                let new = in_slot(&mut traffic, 4);
                let after = trader_of(&traffic, holder);
                assert_eq!(after.target, Some(ShipRef::Npc(new)), "{goal:?}");
                assert_eq!(after.goal, goal.with_quarry(ShipRef::Npc(new)));
                assert_eq!(after.provoked, 10.0, "{goal:?}");
            }
        }

        /// A system of `5` ships on average, all of düde 128's ship 200.
        fn arriving() -> SpawnTable {
            SpawnTable {
                avg_ships: 5,
                dudes: vec![(DudeId(128), 100)],
                dude_records: BTreeMap::from([(
                    DudeId(128),
                    SpawnDude {
                        ai_type: 1,
                        govt: Some(GovtId(130)),
                        ships: vec![(ShipId(200), 1)],
                        booty: 0,
                        info_types: 0,
                    },
                )]),
                ships: BTreeMap::from([(
                    ShipId(200),
                    ShipKind {
                        stats: ShipStats::new(FAST, &[]),
                        ..ShipKind::default()
                    },
                )]),
                ..SpawnTable::default()
            }
        }

        #[test]
        fn an_arrival_in_a_freed_slot_takes_the_aim_of_a_ship_holding_it() {
            // `_EnterMoreShips` (@0x44237) spawns into the first free slot
            // before the thinks, leaving every other ship's +0x70
            // (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b).
            let mut traffic = Traffic::new();
            traffic.table = arriving();
            let wreck = in_slot(&mut traffic, 2);
            let holder = aiming_at(&mut traffic, 1, ShipRef::Npc(wreck));
            traffic.remove_destroyed(wreck);
            // The arrival rolled, then düde 128's ship 200.
            traffic.enter_more_ships(World::new(&[]), &mut Draws::of(&[0, 6, 6, 0, 0, 90]));
            let arrival = traffic
                .npcs()
                .iter()
                .find(|npc| npc.berth == 2)
                .expect("it takes the freed slot")
                .id;
            assert_ne!(arrival, wreck);
            let new = ShipRef::Npc(arrival);
            let after = trader_of(&traffic, holder);
            assert_eq!(after.target, Some(new));
            assert_eq!(after.goal, Goal::Attack(new));
        }

        #[test]
        fn a_slot_taken_again_leaves_a_goal_on_another_ship_and_every_other_holder() {
            let mut traffic = Traffic::new();
            let wreck = in_slot(&mut traffic, 4);
            let other = in_slot(&mut traffic, 5);
            let aim = ShipRef::Npc(wreck);
            let own_fight = aiming_at(&mut traffic, 2, aim);
            let at_other = ShipRef::Npc(other);
            traffic.npcs[2].goal = Goal::Attack(at_other);
            let elsewhere = aiming_at(&mut traffic, 3, at_other);
            traffic.remove_destroyed(wreck);
            let new = in_slot(&mut traffic, 4);
            let kept = trader_of(&traffic, own_fight);
            assert_eq!(kept.target, Some(ShipRef::Npc(new)));
            assert_eq!(kept.goal, Goal::Attack(at_other), "its goal's own ship");
            assert_eq!(target_of(&traffic, elsewhere), Some(at_other));
        }

        #[test]
        fn a_slot_taken_again_passes_nothing_to_a_ship_that_let_go() {
            // A later slot's `_HandleShip` cleared it (@0x33865-0x33888).
            let mut traffic = Traffic::new();
            let wreck = in_slot(&mut traffic, 4);
            let aim = ShipRef::Npc(wreck);
            let holder = aiming_at(&mut traffic, 6, aim);
            traffic.remove_destroyed(wreck);
            traffic.let_go_at_step(holder, true);
            in_slot(&mut traffic, 4);
            assert_eq!(target_of(&traffic, holder), None);
            assert_eq!(trader_of(&traffic, holder).goal, Goal::Attack(aim));
        }

        #[test]
        fn a_slot_freed_in_the_ai_loop_passes_nothing_to_its_next_ship() {
            // Every holder's `_HandleShip` ran before the next spawn; the
            // record stands only for the frame's dispatch.
            let mut traffic = Traffic::new();
            let runner = warping_out(&mut traffic, 3);
            let aim = ShipRef::Npc(runner);
            let holder = aiming_at(&mut traffic, 6, aim);
            fly(&mut traffic, runner);
            in_slot(&mut traffic, 3);
            assert_eq!(target_of(&traffic, holder), Some(aim));
        }

        #[test]
        fn a_slot_taken_elsewhere_passes_nothing() {
            let mut traffic = Traffic::new();
            let wreck = in_slot(&mut traffic, 4);
            let aim = ShipRef::Npc(wreck);
            let holder = aiming_at(&mut traffic, 2, aim);
            traffic.remove_destroyed(wreck);
            in_slot(&mut traffic, 5);
            assert_eq!(target_of(&traffic, holder), Some(aim));
        }

        #[test]
        fn a_swarmer_that_let_go_no_longer_suits_a_lead_still_holding_the_slot() {
            // `_AIVerifySwarmLeader` compares the two +0x70 words
            // (@0x7e360): the lead, in the earlier slot, keeps its word
            // where the later swarmer's is -1.
            let aim = ShipRef::Npc(NpcId(99));
            let mut swarmer = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
            swarmer.target = Some(aim);
            swarmer.goal = Goal::Attack(aim);
            swarmer.swarm_leader = Some(1);
            swarmer.hull.swarms = true;
            let mut traffic = Traffic::new();
            traffic.add_npc_at(swarmer.clone(), 1);
            let later = traffic.add_npc_at(swarmer, 2);
            let suits = |traffic: &Traffic| {
                crate::ai::swarm::verify(trader_of(traffic, later), traffic.npcs())
            };
            assert!(suits(&traffic));
            traffic.let_go_at_step(later, true);
            assert!(!suits(&traffic));
        }

        #[test]
        fn a_ship_killed_by_a_stellar_is_read_by_a_later_slot() {
            // `_HandleDeadlyStellars` (@0x4450e) runs after the ship loop.
            for (trader_berth, attacker_berth) in [(3, 2), (1, 2)] {
                let (mut traffic, trader, attacker) =
                    fleeing_a_warship((100.0, 0.0), trader_berth, attacker_berth, 1);
                traffic.remove_killed(attacker);
                assert!(traffic.npcs().iter().all(|npc| npc.id != attacker));
                tick(&mut traffic);
                let after = trader_of(&traffic, trader);
                assert_eq!(after.goal, Goal::Idle, "{trader_berth}: @0x8e731");
                assert_eq!(after.provoked, 10.0, "{trader_berth}: read near");
            }
        }

        #[test]
        fn a_gone_attackers_place_is_forgotten_where_a_slot_test_clears_it() {
            // Read: the attack ends keeping the provocation.
            let read = |traffic: &Traffic, trader| trader_of(traffic, trader).provoked == 10.0;
            let (mut traffic, trader, attacker) = fleeing_a_warship((100.0, 0.0), 1, 2, 1);
            traffic.remove_destroyed(attacker);
            tick(&mut traffic);
            assert!(read(&traffic, trader), "an earlier slot reads it");
            // A later slot's `_HandleShip` ran after the slot was freed
            // (@0x33865-0x33888): its flight from a ship gone ends, zeroed.
            let (mut traffic, trader, attacker) = fleeing_a_warship((100.0, 0.0), 3, 2, 1);
            traffic.remove_destroyed(attacker);
            tick(&mut traffic);
            assert!(!read(&traffic, trader), "a later slot does not");
            // A trader not thinking on the next tick has its target cleared
            // by its `_HandleShip` before it next thinks.
            let (mut traffic, trader, attacker) = fleeing_a_warship((100.0, 0.0), 1, 2, 2);
            traffic.remove_destroyed(attacker);
            tick(&mut traffic);
            tick(&mut traffic);
            assert!(!read(&traffic, trader), "its next think comes too late");
            // Double time's second pass runs `_HandleShip` for every slot.
            let (mut traffic, trader, attacker) = fleeing_a_warship((100.0, 0.0), 1, 2, 1);
            traffic.remove_destroyed(attacker);
            traffic.move_in(World::new(&[]), &mut Draws::of(&[]));
            tick(&mut traffic);
            assert!(!read(&traffic, trader), "the second pass clears it");
        }
    }
}
