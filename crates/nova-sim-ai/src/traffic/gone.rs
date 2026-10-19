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
//! record; each decision pass takes them all, as its thinks are the last
//! that can read them; a ship pass with no decisions before it, double
//! time's second ([`Traffic::move_in`](crate::traffic::Traffic::move_in)),
//! forgets those freed in a ship pass, as its `_HandleShip` clears every
//! target that names them. A departure in the decision pass's own flight
//! is read one tick late: the original frees a jump-out in the low level,
//! in the same frame's AI loop, so a later slot reads it on that frame,
//! where the engine flies after the decisions and reads it on the next
//! tick. The lag is one tick of the reader's motion, which no player sees,
//! and the answer settles before any dialog, landing, jump or save.
//!
//! A ship that lands is not recorded: the original keeps a landed ship in
//! its slot, waiting at the stellar (`_HighLevelAIHandler`
//! @0x8dadc-0x8db5a), which rdm:phase/stellar-fidelity/phase-5-npc-landing-reach
//! builds. An escape pod freed (`_AIDispatch` @0x8fbb3) fires nothing, so
//! no target names it.

use crate::combat::ShipRef;
use crate::traffic::autopilot::Outcome;
use crate::traffic::npc::Npc;

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
}

impl GoneShip {
    /// Whether `reader`'s think still finds this slot in its target
    /// (+0x70), no slot test having cleared it: its target names the ship,
    /// and its slot comes after the gone one's when that was freed in the
    /// AI loop ([`Freed::InDispatch`]), or before it when in the ship loop
    /// ([`Freed::InShipPass`]).
    #[must_use]
    pub fn held_by(&self, reader: &Npc) -> bool {
        reader.target == Some(ShipRef::Npc(self.ship.id))
            && match self.freed {
                Freed::InDispatch => reader.berth > self.ship.berth,
                Freed::InShipPass => reader.berth < self.ship.berth,
            }
    }
}

/// The ships gone since the last decision pass, whose slots a think may
/// still read (see the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GoneShips(Vec<GoneShip>);

impl GoneShips {
    /// Records `ship`, its slot freed as `freed` says.
    pub fn record(&mut self, ship: Npc, freed: Freed) {
        self.0.push(GoneShip { ship, freed });
    }

    /// Every ship recorded, taken for a decision pass, whose thinks are
    /// the last to read them.
    pub fn take(&mut self) -> Vec<GoneShip> {
        std::mem::take(&mut self.0)
    }

    /// A ship pass with no decisions before it runs `_HandleShip` for
    /// every slot, clearing each target that names a slot freed before it
    /// in a ship pass. One freed in the AI loop stands for a read the
    /// original made on that frame, before the pass, so it is kept for the
    /// next decision pass (see the module docs).
    pub fn ship_pass(&mut self) {
        self.0.retain(|gone| gone.freed == Freed::InDispatch);
    }

    /// Forgets every ship recorded, as a new system's slots hold none of
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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{ShipId, StellarId};
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::NpcId;

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
    fn a_decision_pass_takes_every_gone_ship_and_a_lone_ship_pass_forgets_those_it_clears() {
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
        let taken = gone.take();
        assert_eq!(taken.len(), 2);
        assert_eq!(gone.ships(), []);
        second.clear();
        assert_eq!(second.ships(), []);
    }

    mod in_traffic {
        use super::*;
        use crate::ai::fixture::{PIRATES, TRADERS, player, ship};
        use crate::ai::{CommCall, Goal, NovaAi};
        use crate::traffic::fixture::calls_on_line;
        use crate::traffic::npc::AiType;
        use crate::traffic::{Traffic, World};
        use nova_sim_base::fixture::Draws;

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
