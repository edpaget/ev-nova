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
//! [`GoneShips`], one record a slot: each departure and destruction is
//! recorded with its record, which stands, as the original's does, until
//! a ship takes the slot ([`Traffic::slot_taken`]) or the system is left.
//! Every run of every tick's think reads them all. A record is read only
//! where a target (+0x70) or a nav (+0x6c) still names its slot: the
//! think's [`GoneShip::held_by`], whose target `_HandleShip`'s let-go
//! clears on the first pass after the free; a ship whose high level is
//! skipped, noted at its think for its flight (see the traffic's
//! `skipped_high_level`); and a docking's nav, which no slot test clears
//! (see [`repair_dock`](crate::traffic::repair_dock)).
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
//! **A departure in the AI loop** (a jump-out, a fighter's dock) leaves at
//! the ship's own slot of that loop, as the original's low level frees it
//! there (see the traffic's `dispatch_leave`): the later slots' thinks read
//! it on that frame, and every slot's `_HandleShip`, an earlier one's too,
//! lets go of it at its step, before its fire, so no target names it by
//! the next frame.
//!
//! **A slot taken again** ([`Traffic::slot_taken`]) leaves every other
//! ship's +0x70 and +0x6c as they were, so a ship still holding the slot
//! finds it active at its next test and now fights the newcomer, friend
//! or not, a paused fight or guard too, and a docking flies at it. In the
//! ship loop ([`Traffic::set_ship_loop`]) a wreck displayed earlier in the
//! loop holds no berth, and every holder whose step hasn't come passes. It
//! changes what is drawn and breaks no stock mission, so it has no rulebook
//! key.
//!
//! **The other readers of a stale +0x70**, each where the original reads
//! it: the cloak think at the dispatch's head, on goal 4
//! ([`cloak_situation::pressed`](crate::traffic::cloak_situation::pressed));
//! the high level's own slot test before its hold writes goal 2 or 3 (the
//! traffic's `dispatch_leave`); and a think after a dialog's free
//! ([`Freed::InDialog`]). The launch in the AI loop into a slot a dock or
//! jump-out freed that loop is
//! rdm:phase/combat-ai-fidelity-cross-roadmap/phase-12-left-out-of-phase-9's.
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
use crate::combat::hull::Condition;
use crate::player_wrap::PlayerWrap;
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
    /// In a dialog in space: a person swapped for its mission ship on the
    /// hail (`_HandlePlayerCommunication` @0x62520, [`Traffic::vacate`]).
    /// No `_HandleShip` runs between the dialog and the next think, so it
    /// reads as [`Freed::AfterShipLoop`] does. The frees while landed
    /// (@0x403fb, @0x40250, @0x5eda5, @0x99b24) are not recorded: the
    /// take-off wipes every slot first (`_SetupShipsInSystem`'s
    /// `_DeleteAllComputerShips` @0x42b92) and respawns the escorts it
    /// keeps, each spawn writing +0x70 -1 (@0x3c9d9).
    InDialog,
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
    /// (+0x70), no slot test having cleared it by then: its target names
    /// the ship, and its slot comes after the gone one's when that was
    /// freed in the AI loop ([`Freed::InDispatch`]), before it when in the
    /// ship loop ([`Freed::InShipPass`]), and is any other when after it
    /// or in a dialog ([`Freed::AfterShipLoop`], [`Freed::InDialog`]).
    #[must_use]
    pub fn held_by(&self, reader: &Npc) -> bool {
        reader.target == Some(ShipRef::Npc(self.ship.id))
            && match self.freed {
                Freed::InDispatch => reader.berth > self.ship.berth,
                Freed::InShipPass => reader.berth < self.ship.berth,
                Freed::AfterShipLoop | Freed::InDialog => reader.berth != self.ship.berth,
            }
    }
}

/// The records of the freed slots not taken again since the system was
/// entered, one a slot, whose ships a think or a flight may still read
/// (see the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GoneShips(Vec<GoneShip>);

impl GoneShips {
    /// Records `ship`, its slot freed as `freed` says, in place of any
    /// record its slot held, as the slot keeps only its last ship's.
    pub fn record(&mut self, ship: Npc, freed: Freed) {
        self.0.retain(|gone| gone.ship.berth != ship.berth);
        self.0.push(GoneShip { ship, freed });
    }

    /// Drops the record at `berth`, as a spawn into the slot rewrites it
    /// (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b).
    pub fn taken(&mut self, berth: u32) {
        self.0.retain(|gone| gone.ship.berth != berth);
    }

    /// Forgets every ship recorded, as a new system's slots hold none of
    /// them.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Carries each ship recorded within `wrap`'s reach, as the slot still
    /// holds it (see the `wrap_carry` module).
    pub fn carry(&mut self, wrap: &PlayerWrap) {
        for gone in &mut self.0 {
            wrap.carry(&mut gone.ship.state.position);
        }
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
    /// else is inactive. A ship that left at its slot of the AI loop is out
    /// of the system, so every slot, an earlier one too, lets go of it at
    /// its step, before its fire. Nothing happens for an NPC not in the
    /// system.
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

    /// `_HighLevelAIHandler`'s slot test on the NPC at `index`, in the AI
    /// loop (@0x8d4e4-0x8d50d): an NPC target whose slot is inactive is
    /// none. In the AI loop a ship left behind by a move still holds its
    /// slot, as its display comes later; a ship that left at its slot of the
    /// loop, or that is out of the system, does not. A target of the
    /// player is kept: the player's slot is active while the AI loop runs
    /// in its system.
    pub(super) fn slot_test(&mut self, index: usize) {
        let Some(ShipRef::Npc(ship)) = self.npcs[index].target else {
            return;
        };
        let active = match self.npcs.iter().find(|npc| npc.id == ship) {
            Some(npc) => npc.condition.active(),
            None => (self.left_behind.values().flatten()).any(|left| left.npc.id == ship),
        };
        if !active {
            self.npcs[index].target = None;
        }
    }

    /// `ship`, gone out of the system as `freed` says, is recorded.
    pub(super) fn left_the_system(&mut self, ship: Npc, freed: Freed) {
        self.gone.record(ship, freed);
    }

    /// The session's ship loop opens (`running`) or closes: a slot taken
    /// while it runs (an escape ship's `_LaunchFighter` @0x354ea) passes
    /// on as [`Traffic::slot_taken`] says, and a wreck whose display has
    /// run ([`Condition::Destroyed`](crate::combat::hull::Condition::Destroyed))
    /// no longer holds its berth (@0x2dee0), though the take-out comes
    /// after the loop.
    pub fn set_ship_loop(&mut self, running: bool) {
        self.ship_loop = running;
    }

    /// Whether `npc` holds its berth: any ship but a wreck
    /// ([`Condition::Destroyed`](crate::combat::hull::Condition::Destroyed)),
    /// whose display freed its slot (@0x2dee0). A wreck still holds it
    /// outside the ship loop, alone in it, as the engine takes it out only
    /// after the loop; in the loop, or once a launch took the berth
    /// (`_GenericRandomShipSpawn` rewrote the slot, @0x3c8b1-0x3c90b), it
    /// holds none.
    pub(super) fn holds_berth(&self, npc: &Npc) -> bool {
        npc.condition != Condition::Destroyed
            || !(self.ship_loop
                || (self.npcs.iter()).any(|other| other.berth == npc.berth && other.id != npc.id))
    }

    /// The NPC slot `berth` holds ([`Traffic::holds_berth`]): its ship, or
    /// the one a launch put there past a wreck not yet taken out; none for
    /// an empty berth, or one a wreck freed in the ship loop.
    #[must_use]
    pub fn holder_of(&self, berth: u32) -> Option<NpcId> {
        (self.npcs.iter())
            .find(|npc| npc.berth == berth && self.holds_berth(npc))
            .map(|npc| npc.id)
    }

    /// The ship `id` takes slot `berth`: every ship still holding that slot
    /// finds it active at its next slot test, so its target is now the new
    /// ship, friend or not, and a fight on the old ship moves to it
    /// ([`Goal::with_quarry`](crate::ai::Goal::with_quarry)), a fight or
    /// a guard the regroup pause keeps too
    /// ([`Goal::kept_quarry`](crate::ai::Goal::kept_quarry)). A spawn
    /// rewrites the slot and leaves every other ship's +0x70 and +0x6c
    /// (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b), and the engine's
    /// numbers, unlike the slots, are never used again.
    ///
    /// Who still holds the slot turns on where the spawn runs:
    ///
    /// - **Outside the ship loop** (the arrivals pass, a launch in the AI
    ///   loop), each holder of its record ([`GoneShip::held_by`]). A record
    ///   freed in the AI loop ([`Freed::InDispatch`]) passes nothing there:
    ///   the launch into a slot a dock or jump-out freed that loop is
    ///   rdm:phase/combat-ai-fidelity-cross-roadmap/phase-12-left-out-of-phase-9's.
    /// - **In the ship loop** ([`Traffic::set_ship_loop`]), every ship
    ///   whose target names the old ship: those whose `_HandleShip` hasn't
    ///   come hold it still, and those whose has have let go
    ///   ([`Traffic::let_go_at_step`]). The old ship is the record's, or a
    ///   wreck freed at its display this loop and not yet taken out.
    ///
    /// A docking (its nav, +0x6c, which no slot test clears) on the old
    /// ship moves to the new one wherever the spawn runs (see
    /// [`repair_dock`](crate::traffic::repair_dock)).
    pub(super) fn slot_taken(&mut self, berth: u32, id: NpcId) {
        let new = ShipRef::Npc(id);
        let in_loop = self.ship_loop;
        let mut olds: Vec<(NpcId, Option<GoneShip>)> = (self.gone.ships().iter())
            .filter(|gone| gone.ship.berth == berth)
            .map(|gone| (gone.ship.id, Some(gone.clone())))
            .collect();
        if in_loop {
            olds.extend(
                (self.npcs.iter())
                    .filter(|npc| npc.berth == berth && npc.id != id && !self.holds_berth(npc))
                    .map(|npc| (npc.id, None)),
            );
        }
        for (old_id, record) in olds {
            let old = ShipRef::Npc(old_id);
            for npc in &mut self.npcs {
                if let Some(docking) = npc.docking.as_mut().filter(|d| d.ship == old) {
                    docking.ship = new;
                }
            }
            let holds = |npc: &Npc| match (&record, in_loop) {
                (_, true) => npc.target == Some(old),
                (Some(gone), false) => gone.freed != Freed::InDispatch && gone.held_by(npc),
                (None, false) => false,
            };
            for npc in self.npcs.iter_mut().filter(|npc| holds(npc)) {
                npc.target = Some(new);
                let goal = npc.goal;
                if [goal.quarry(), goal.inspecting(), goal.kept_quarry()].contains(&Some(old)) {
                    npc.goal = goal.with_quarry(new);
                }
            }
        }
        self.gone.taken(berth);
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
        };
        let on_it = Some(ShipRef::Npc(NpcId(2)));
        assert!(gone.held_by(&reader(4, on_it)));
        assert!(gone.held_by(&reader(6, on_it)));
        assert!(!gone.held_by(&reader(5, on_it)), "its own slot");
        assert!(!gone.held_by(&reader(6, None)));
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
    fn a_berth_freed_again_keeps_its_latest_record() {
        // A slot holds one record, the last ship's, as `VacatedBerths`
        // keeps one.
        let mut gone = GoneShips::default();
        gone.record(at_berth(1, 5), Freed::InShipPass);
        gone.record(at_berth(2, 6), Freed::InShipPass);
        gone.record(at_berth(3, 5), Freed::AfterShipLoop);
        let kept: Vec<(NpcId, Freed)> = gone
            .ships()
            .iter()
            .map(|gone| (gone.ship.id, gone.freed))
            .collect();
        assert_eq!(
            kept,
            [
                (NpcId(2), Freed::InShipPass),
                (NpcId(3), Freed::AfterShipLoop)
            ]
        );
        gone.clear();
        assert_eq!(gone.ships(), []);
    }

    mod in_traffic {
        use super::*;
        use crate::ai::fixture::{PIRATES, TRADERS, player, ship};
        use crate::ai::{CommCall, Goal, NovaAi};
        use crate::catalog::{DudeId, GovtId, SystemId};
        use crate::chance::NeverFires;
        use crate::combat::hull::Condition;
        use crate::rulebook::RuleSource;
        use crate::traffic::fixture::calls_on_line;
        use crate::traffic::npc::{AiType, MissionMark, Mode};
        use crate::traffic::table::{ShipKind, SpawnDude, SpawnTable};
        use crate::traffic::{FlyHalf, Leave, LeftShip, ShipBehav, Traffic, World};
        use nova_sim_base::fixture::{Draws, FAST};
        use nova_sim_catalog::fixture::planet;
        use std::collections::{BTreeMap, BTreeSet};

        /// The stellar a landing ship lands on.
        const STELLAR: StellarId = StellarId(140);

        /// A mission's special ship's mark.
        const MISSION_SHIP: MissionMark = MissionMark {
            slot: 0,
            behav: ShipBehav::AttackPlayer,
            formation: None,
            order: None,
        };

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
            // Slot 1 left in its own dispatch (@0x859b8), so slot 2's think
            // read its record on that tick: it attacked where it was, and
            // the high level ended the fight.
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
            holder.travel = Some(StellarId(150));
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
        fn a_ship_keeps_a_target_breaking_up_or_disabled() {
            // +0xb8 stays set for each.
            let dying = Condition::Dying {
                ticks_left: 5,
                starting: false,
            };
            for condition in [dying, Condition::Disabled] {
                let mut traffic = Traffic::new();
                let other = in_slot(&mut traffic, 2);
                let holder = aiming_at(&mut traffic, 3, ShipRef::Npc(other));
                traffic.npcs[0].condition = condition;
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
        fn the_high_levels_slot_test_lets_go_of_a_slot_inactive_in_the_ai_loop() {
            // @0x8d4e4-0x8d50d: +0x70 -1 when its slot's +0xb8 is clear. A
            // ship left behind keeps its slot until its display, after the
            // AI loop, whichever slot reads it.
            let mut traffic = Traffic::new();
            let live = in_slot(&mut traffic, 5);
            let wreck = in_slot(&mut traffic, 6);
            traffic.npcs[1].condition = Condition::Destroyed;
            let left = NpcId(99);
            traffic.left_behind.insert(
                SystemId(130),
                vec![LeftShip {
                    npc: Npc {
                        id: left,
                        ..at_berth(99, 1)
                    },
                    person: None,
                }],
            );
            for (aim, kept) in [
                (ShipRef::Npc(live), true),
                (ShipRef::Npc(wreck), false),
                (ShipRef::Npc(left), true),
                (ShipRef::Npc(NpcId(98)), false),
                (ShipRef::Player, true),
            ] {
                let holder = aiming_at(&mut traffic, 3, aim);
                let index = traffic.npcs.iter().position(|npc| npc.id == holder);
                traffic.slot_test(index.expect("there"));
                let after = trader_of(&traffic, holder);
                assert_eq!(after.target, kept.then_some(aim), "{aim:?}");
                assert_eq!(after.goal, Goal::Attack(aim), "+0x70 alone");
                traffic.remove(holder);
            }
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

        /// A ship in slot `berth` of `traffic`, marked as `mission` says, at
        /// rest over planet 140: its next flight lands it there.
        fn landing(traffic: &mut Traffic, berth: u32, mission: Option<MissionMark>) -> NpcId {
            let mut lander = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
            lander.goal = Goal::Land(STELLAR);
            lander.mission = mission;
            traffic.add_npc_at(lander, berth)
        }

        /// NPC `id`'s flight, alone, in a fly half of `traffic` in `world`:
        /// what became of it.
        fn fly_in(traffic: &mut Traffic, id: NpcId, world: &World) -> Option<Outcome> {
            let mut pass = traffic.begin_fly(world, &BTreeSet::new(), FlyHalf::Advance);
            traffic.fly_npc(&mut pass, id, world, &mut NeverFires)
        }

        /// NPC `id`'s flight, alone, in a fly half of `traffic`.
        fn fly(traffic: &mut Traffic, id: NpcId) {
            fly_in(traffic, id, &World::new(&[]));
        }

        /// NPC `id`'s flight, alone, in a fly half of `traffic` over planet
        /// 140, a mission ship that lands leaving as `landed_leaves` says:
        /// what became of it.
        fn land(traffic: &mut Traffic, id: NpcId, landed_leaves: RuleSource) -> Option<Outcome> {
            let sites = [planet(140, 0.0, 0.0)];
            let world = World {
                landed_leaves,
                ..World::new(&sites)
            };
            fly_in(traffic, id, &world)
        }

        /// Every ship keeps its goal and target.
        #[derive(Debug)]
        struct Keeps;

        impl crate::ai::Behaviour for Keeps {
            fn decide(
                &self,
                npc: &Npc,
                _around: &crate::ai::Surroundings,
                _chance: &mut dyn crate::chance::Chance,
            ) -> Goal {
                npc.goal
            }

            fn target(&self, npc: &Npc, _around: &crate::ai::Surroundings) -> Option<ShipRef> {
                npc.target
            }
        }

        #[test]
        fn a_jump_out_or_dock_at_its_slot_is_let_go_by_every_slot_at_its_step() {
            // The original frees these in the AI loop (@0x859b8, @0x80404),
            // before the `_HandleShip` of every slot, which lets go
            // (@0x33865-0x33888), an earlier slot's too. A later slot's
            // high level, after the free, has let go already
            // (@0x8d4e4-0x8d50d).
            for dock in [false, true] {
                let mut traffic = Traffic::with_interval(1);
                let leaving = if dock {
                    docking(&mut traffic, 3, 8)
                } else {
                    warping_out(&mut traffic, 3)
                };
                let aim = ShipRef::Npc(leaving);
                let earlier = aiming_at(&mut traffic, 2, aim);
                let later = aiming_at(&mut traffic, 6, aim);
                let elsewhere = aiming_at(&mut traffic, 1, ShipRef::Player);
                traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
                assert!(
                    traffic.npcs().iter().all(|npc| npc.id != leaving),
                    "{dock}: it left at its slot"
                );
                assert_eq!(target_of(&traffic, earlier), Some(aim), "{dock}");
                assert_eq!(target_of(&traffic, later), None, "{dock}: its high level");
                for holder in [earlier, later] {
                    traffic.let_go_at_step(holder, true);
                    assert_eq!(target_of(&traffic, holder), None, "{dock}: its step");
                    assert_eq!(
                        trader_of(&traffic, holder).goal,
                        Goal::Attack(aim),
                        "{dock}: +0x70 alone"
                    );
                }
                traffic.let_go_at_step(elsewhere, true);
                assert_eq!(
                    target_of(&traffic, elsewhere),
                    Some(ShipRef::Player),
                    "{dock}"
                );
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
        fn a_ship_keeps_a_target_that_landed_and_stays_in_its_slot() {
            // State 1's landing leaves +0xb8 set (@0x8dadc-0x8db5a), so
            // `_HandleShip`'s slot test (@0x33865-0x33888) finds it active
            // from an earlier slot and a later one. `landed_leaves`' other
            // reading moves only a mission ship.
            for (mission, reading) in [
                (None, RuleSource::Engine),
                (None, RuleSource::Bible),
                (Some(MISSION_SHIP), RuleSource::Engine),
            ] {
                let mut traffic = Traffic::new();
                let lander = landing(&mut traffic, 3, mission);
                let aim = ShipRef::Npc(lander);
                let earlier = aiming_at(&mut traffic, 2, aim);
                let later = aiming_at(&mut traffic, 6, aim);
                let case = format!("{mission:?} {reading:?}");
                assert_eq!(
                    land(&mut traffic, lander, reading),
                    Some(Outcome::Landed(STELLAR)),
                    "{case}"
                );
                for holder in [earlier, later] {
                    traffic.let_go_at_step(holder, true);
                    assert_eq!(target_of(&traffic, holder), Some(aim), "{case}");
                }
                assert!(trader_of(&traffic, lander).wait_running(), "{case}");
                assert_eq!(traffic.gone.ships(), [], "{case}");
            }
        }

        #[test]
        fn a_ship_lets_go_at_its_step_of_a_mission_ship_that_left_as_it_landed() {
            // By `landed_leaves`' other reading the ship leaves with no
            // record, so nothing sweeps the earlier slot as a departure in
            // the AI loop would: each holder finds the slot inactive at its
            // own step, writing +0x70 alone.
            let mut traffic = Traffic::new();
            let lander = landing(&mut traffic, 3, Some(MISSION_SHIP));
            let aim = ShipRef::Npc(lander);
            let earlier = aiming_at(&mut traffic, 2, aim);
            let later = aiming_at(&mut traffic, 6, aim);
            assert_eq!(
                land(&mut traffic, lander, RuleSource::Bible),
                Some(Outcome::Landed(STELLAR))
            );
            assert_eq!(traffic.departed(), [(lander, Outcome::Landed(STELLAR))]);
            assert_eq!(traffic.gone.ships(), []);
            for holder in [earlier, later] {
                assert_eq!(target_of(&traffic, holder), Some(aim), "till its step");
                traffic.let_go_at_step(holder, true);
                assert_eq!(target_of(&traffic, holder), None);
                assert_eq!(trader_of(&traffic, holder).goal, Goal::Attack(aim));
            }
        }

        #[test]
        fn a_slot_left_by_a_mission_ship_as_it_landed_passes_nothing_to_its_next_ship() {
            // No record names the slot, so a ship still holding it goes on
            // naming the ship that left until its step lets go.
            let mut traffic = Traffic::new();
            let lander = landing(&mut traffic, 3, Some(MISSION_SHIP));
            let aim = ShipRef::Npc(lander);
            let earlier = aiming_at(&mut traffic, 2, aim);
            land(&mut traffic, lander, RuleSource::Bible);
            in_slot(&mut traffic, 3);
            assert_eq!(target_of(&traffic, earlier), Some(aim));
            traffic.let_go_at_step(earlier, true);
            assert_eq!(target_of(&traffic, earlier), None);
        }

        #[test]
        fn a_freed_slots_record_stands_until_a_ship_takes_its_berth() {
            // Nothing but a spawn into the slot rewrites its record
            // (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b): not a ship pass,
            // nor a later tick.
            let mut traffic = Traffic::new();
            let wreck = in_slot(&mut traffic, 4);
            traffic.remove_destroyed(wreck);
            traffic.move_in(World::new(&[]), &mut Draws::of(&[]));
            tick(&mut traffic);
            let ids = |traffic: &Traffic| -> Vec<NpcId> {
                traffic
                    .gone
                    .ships()
                    .iter()
                    .map(|gone| gone.ship.id)
                    .collect()
            };
            assert_eq!(ids(&traffic), [wreck]);
            in_slot(&mut traffic, 5);
            assert_eq!(ids(&traffic), [wreck], "another berth taken");
            in_slot(&mut traffic, 4);
            assert_eq!(ids(&traffic), [], "its berth taken");
        }

        #[test]
        fn a_new_system_drops_every_record() {
            let mut traffic = Traffic::new();
            let wreck = in_slot(&mut traffic, 4);
            traffic.remove_destroyed(wreck);
            traffic.retarget(SpawnTable::default(), &BTreeSet::new(), Leave::Dropped);
            assert_eq!(traffic.gone.ships(), []);
        }

        #[test]
        fn a_slot_taken_again_moves_a_paused_fight_onto_the_newcomer() {
            // The pause keeps its substate and +0x70 (@0x9002d → @0x901ea),
            // which a spawn leaves (@0x3c8b1-0x3c90b): its low level now
            // reads the newcomer, a kept formation's guard too
            // (@0x876a8-0x876e3).
            use crate::ai::regroup::Kept;
            let old = ShipRef::Npc(NpcId(0));
            for kept in [
                Kept::Attack(old),
                Kept::Snipe(old),
                Kept::Formation { guard: Some(old) },
            ] {
                let mut traffic = Traffic::new();
                let wreck = in_slot(&mut traffic, 4);
                let holder = aiming_at(&mut traffic, 2, old);
                traffic.npcs[1].goal = Goal::Regroup { kept };
                traffic.remove_destroyed(wreck);
                let new = ShipRef::Npc(in_slot(&mut traffic, 4));
                let after = trader_of(&traffic, holder);
                assert_eq!(after.target, Some(new), "{kept:?}");
                assert_eq!(
                    after.goal,
                    Goal::Regroup {
                        kept: kept.with_quarry(new)
                    },
                    "{kept:?}"
                );
            }
        }

        #[test]
        fn a_slot_taken_in_the_ship_loop_passes_to_every_holder_whose_step_has_not_come() {
            // In the ship loop every holder whose `_HandleShip` hasn't run
            // still holds +0x70 (@0x33865-0x33888): those below the freed
            // slot, and those above the launch's step. One between let go
            // at its step. So does a slot freed in the AI loop that frame.
            for freed in [Freed::InShipPass, Freed::AfterShipLoop, Freed::InDispatch] {
                let mut traffic = Traffic::new();
                let gone = in_slot(&mut traffic, 4);
                let aim = ShipRef::Npc(gone);
                let below = aiming_at(&mut traffic, 2, aim);
                let between = aiming_at(&mut traffic, 5, aim);
                let above = aiming_at(&mut traffic, 8, aim);
                let record = traffic.npcs[0].clone();
                traffic.remove(gone);
                traffic.gone.record(record, freed);
                traffic.set_ship_loop(true);
                traffic.let_go_at_step(between, true);
                // A launch at step 6 takes the slot.
                let new = ShipRef::Npc(in_slot(&mut traffic, 4));
                traffic.set_ship_loop(false);
                for (holder, passed) in [(below, true), (between, false), (above, true)] {
                    let after = trader_of(&traffic, holder);
                    let want = if passed { Some(new) } else { None };
                    assert_eq!(after.target, want, "{freed:?} {holder:?}");
                    if passed {
                        assert_eq!(after.goal, Goal::Attack(new), "{freed:?}");
                    }
                }
            }
        }

        #[test]
        fn a_berth_is_held_by_its_newcomer_past_a_wreck_displayed_in_the_loop() {
            // The wreck's display freed the slot (@0x2dee0) and a launch
            // rewrote it (@0x3c8b1-0x3c90b): the slot is the newcomer's,
            // in the loop and after it, until the take-out.
            let mut traffic = Traffic::new();
            let wreck = in_slot(&mut traffic, 1);
            traffic.npcs[0].condition = Condition::Destroyed;
            traffic.set_ship_loop(true);
            let new = in_slot(&mut traffic, 1);
            assert_eq!(traffic.holder_of(1), Some(new), "in the loop");
            traffic.set_ship_loop(false);
            assert_eq!(traffic.holder_of(1), Some(new), "after it");
            assert_ne!(traffic.holder_of(1), Some(wreck));
        }

        #[test]
        fn a_berth_is_held_by_its_lone_ship_and_by_a_lone_wreck_only_outside_the_loop() {
            let mut traffic = Traffic::new();
            let live = in_slot(&mut traffic, 2);
            let wreck = in_slot(&mut traffic, 3);
            traffic.npcs[1].condition = Condition::Destroyed;
            assert_eq!(traffic.holder_of(2), Some(live));
            assert_eq!(traffic.holder_of(3), Some(wreck), "outside the loop");
            assert_eq!(traffic.holder_of(4), None, "empty");
            traffic.set_ship_loop(true);
            assert_eq!(traffic.holder_of(2), Some(live), "live, in the loop");
            assert_eq!(traffic.holder_of(3), None, "displayed: freed");
        }

        #[test]
        fn in_the_ship_loop_a_launch_takes_a_wrecks_berth_freed_at_its_display() {
            // A final explosion frees the slot at its display (@0x2dee0),
            // before the take-out; a launch later in the loop searches the
            // slots from 1 (@0x3c8c6-0x3c8e9). The wreck's record is the
            // newcomer's now, so the take-out records nothing.
            let mut traffic = Traffic::new();
            let wreck = in_slot(&mut traffic, 1);
            let holder = aiming_at(&mut traffic, 3, ShipRef::Npc(wreck));
            traffic.npcs[0].condition = Condition::Destroyed;
            // Another wreck, in another berth, keeps its holder.
            let other = in_slot(&mut traffic, 5);
            let other_holder = aiming_at(&mut traffic, 6, ShipRef::Npc(other));
            let at = traffic.npcs.iter().position(|npc| npc.id == other);
            traffic.npcs[at.expect("there")].condition = Condition::Destroyed;
            assert_eq!(traffic.free_berth(0), Some(2), "outside the loop");
            traffic.set_ship_loop(true);
            assert_eq!(traffic.free_berth(0), Some(1));
            let new = traffic.add_npc_at(ship(0, TRADERS, AiType::Warship, 0.0, 0.0), 1);
            traffic.set_ship_loop(false);
            assert_eq!(target_of(&traffic, holder), Some(ShipRef::Npc(new)));
            assert_eq!(
                target_of(&traffic, other_holder),
                Some(ShipRef::Npc(other)),
                "another berth's wreck"
            );
            traffic.remove_destroyed(wreck);
            assert_eq!(traffic.gone.ships(), [], "rewritten by the newcomer");
            assert!(traffic.npcs().iter().any(|npc| npc.id == new));
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
        fn a_slot_freed_after_the_ship_loop_passes_every_other_slots_holder_to_its_next_ship() {
            // No `_HandleShip` runs between a deadly stellar's kill
            // (@0x39dac) and the next frame's spawn, so a ship in any other
            // slot still holds it.
            let mut traffic = Traffic::new();
            let killed = in_slot(&mut traffic, 4);
            let aim = ShipRef::Npc(killed);
            let earlier = aiming_at(&mut traffic, 2, aim);
            let later = aiming_at(&mut traffic, 6, aim);
            traffic.remove_killed(killed);
            let new = ShipRef::Npc(in_slot(&mut traffic, 4));
            for holder in [earlier, later] {
                let after = trader_of(&traffic, holder);
                assert_eq!(after.target, Some(new));
                assert_eq!(after.goal, Goal::Attack(new));
            }
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
            // by its `_HandleShip` before it next thinks, the session's
            // ship loop's let-go, though the record stands.
            let (mut traffic, trader, attacker) = fleeing_a_warship((100.0, 0.0), 1, 2, 2);
            traffic.remove_destroyed(attacker);
            tick(&mut traffic);
            traffic.let_go_at_step(trader, true);
            tick(&mut traffic);
            assert!(!read(&traffic, trader), "its next think comes too late");
            assert_eq!(traffic.gone.ships().len(), 1, "the record stands");
            // Double time's second pass runs `_HandleShip` for every slot.
            let (mut traffic, trader, attacker) = fleeing_a_warship((100.0, 0.0), 1, 2, 1);
            traffic.remove_destroyed(attacker);
            traffic.move_in(World::new(&[]), &mut Draws::of(&[]));
            traffic.let_go_at_step(trader, true);
            tick(&mut traffic);
            assert!(!read(&traffic, trader), "the second pass clears it");
        }
    }
}
