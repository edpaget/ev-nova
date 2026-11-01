//! A docking ship's low level at its own slot of `_AIDispatch`'s loop:
//! sub 0xf's whole body, and subs 9, 0xb and 1 beside it
//! (`_LowLevelAIHandler` @0x86a08-0x8709a, @0x8769a-0x878f4,
//! @0x87337-0x87692 in the `EV Nova` executable; see
//! [`plunder_dock`](super::plunder_dock)).
//!
//! The low level is the last thing in a ship's `_AIDispatch` (@0x901f4),
//! so everything it writes is written at the ship's slot, before the
//! higher slots' dispatches:
//!
//! - the ship's own velocity, speed (+0x48) and place: sub 0xf's match
//!   (@0x86c08-0x86cb2) and an inertialess match's speed
//!   (@0x86aee-0x86b33), the aligned velocity (@0x86cd5-0x86ce4), the
//!   creep (@0x8700f-0x87095), the start's velocity (@0x86ff1-0x87007),
//!   and sub 0xb's swarm branch (@0x87364-0x873a3);
//! - the victim's boarded flag (+0xb9): set by the start (@0x86fc7), the
//!   player's by the session as the pirate's line settles at its slot
//!   ([`PirateEvent::PlayerDockedWith`]), cleared by the repair
//!   (@0x86d92);
//! - the repair's armour (@0x86d9c-0x86dcf) and, for the player alone, its
//!   line and `snd ` 151 (@0x86dd4-0x86ef4): the session's, from
//!   [`PirateEvent::PlayerRepaired`] or [`PirateEvent::NpcRepaired`],
//!   queued on the line at the repairer's berth, which the session's
//!   dispatch settles as the slot's run ends
//!   (`nova_sim::Session::dispatch_npcs`);
//! - the repairer's drift (goal 0xe, its wait 100, its target, nav and
//!   substate cleared, @0x86f09-0x86f31, [`repair_dock::complete`]).
//!
//! So a higher slot's think reads them that tick, and a lower slot's as
//! the tick began. Which think of another slot reads them: any that reads
//! a ship's place or velocity (a target's distance, the pursuit, the
//! odds), the pirates' and the repairers' reads of the victim's boarded
//! flag ([`pirate`](crate::ai::pirate)), and the reads of the patient's
//! condition and armour.
//!
//! The victim is read live, at the slot: an NPC as the lower slots left
//! it, the player as the tick began (its flight is the ship loop's). A
//! ship gone is read by its slot's record, which the nav (+0x6c) still
//! names whatever tick freed it, and which stands until a ship takes the
//! berth (see [`repair_dock`]).
//!
//! The flight the low level leaves `_HandleShip`, its heading wanted
//! (+0x68) and its thrust word (+0x30), is kept for the fly half
//! ([`DockFlight`]), which flies it ([`plunder_dock::handle_ship`]) and
//! decides nothing. Double time's second pass runs no `_AIDispatch`
//! (@0x44350), so it matches and creeps nothing: it flies the same flight
//! again, as `_HandleShip` reads the words the low level left.
//!
//! The plunder's completion ([`DockOutcome::Completed`], @0x86f3c-0x86f93)
//! still does nothing: no pirate reaches it ([`plunder_dock`]'s docs).
//! Nothing here plays a sound but the session's `snd ` 151.

use super::plunder::{self, PirateEvent};
use super::plunder_dock::DockOutcome;
#[cfg(doc)]
use super::plunder_dock::{self, DockFlight};
use super::{Traffic, World, repair_dock};
use crate::ai::{Surroundings, swarm};
use crate::chance::Chance;
use crate::combat::ShipRef;

impl Traffic {
    /// The low level of the docking of the NPC at `index` in `world`,
    /// rolling a dock's wait on `chance` (see the module docs): none for a
    /// ship with no docking, or one not flying intact. The flight it
    /// leaves is kept for the fly half; one whose low level flew no
    /// docking keeps none.
    pub(super) fn dock_in_dispatch(
        &mut self,
        index: usize,
        world: &World,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let npc = &self.npcs[index];
        let id = npc.id;
        self.dock_flights.remove(&id);
        let Some(docking) = npc.docking else {
            return;
        };
        let victim = match docking.ship {
            ShipRef::Player => world.player.map(|player| player.state),
            ShipRef::Npc(ship) => (self.npcs.iter())
                .chain(self.gone.ships().iter().map(|gone| &gone.ship))
                .find(|npc| npc.id == ship)
                .map(|npc| npc.state),
        };
        // Sub 0xb's swarm branch reads its swarm leader's slot live, a freed
        // one by its record (see `swarm::lead`).
        let around = Surroundings {
            gone: self.gone.ships(),
            ..world.around(&self.npcs)
        };
        let swarm = swarm::lead(npc, &around);
        let own = npc.top_speed(world.led_speed);
        let npc = &mut self.npcs[index];
        let flight = world.flight;
        let Some((flown, outcome)) =
            plunder::dock_low_level(npc, victim.as_ref(), own, flight, swarm, chance)
        else {
            return;
        };
        self.dock_flights.insert(id, flown);
        match outcome {
            DockOutcome::Started => match docking.ship {
                ShipRef::Player => self.queue_pirate(PirateEvent::PlayerDockedWith { pirate: id }),
                ShipRef::Npc(ship) => {
                    if let Some(npc) = self.npcs.iter_mut().find(|npc| npc.id == ship) {
                        npc.boarded = true;
                    }
                }
            },
            DockOutcome::Repaired => {
                // Its nav, the ship it repaired, before the completion
                // clears it (@0x86d84, @0x86f31).
                let npc = &mut self.npcs[index];
                let helper = (npc.id, npc.ship);
                repair_dock::complete(npc);
                self.queue_pirate(match docking.ship {
                    ShipRef::Player => PirateEvent::PlayerRepaired {
                        helper: helper.0,
                        ship: helper.1,
                    },
                    ShipRef::Npc(ship) => PirateEvent::NpcRepaired {
                        helper: helper.0,
                        ship,
                    },
                });
            }
            DockOutcome::Flying | DockOutcome::Completed => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use crate::ai::fixture::{PIRATES, TRADERS, ship};
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::chance::{Chance, NeverFires};
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::hail::Help;
    use crate::traffic::NoBay;
    use crate::traffic::npc::{AiType, Npc, NpcId};
    use crate::traffic::on_line::OnLine;
    use crate::traffic::plunder::PirateEvent;
    use crate::traffic::plunder_dock::{DockSub, Docking};
    use crate::traffic::{Traffic, World};
    use nova_sim_base::fixture::Draws;

    /// Every ship keeps its goal; each think records where it reads
    /// `watched`.
    #[derive(Debug)]
    struct Reads {
        watched: NpcId,
        seen: RefCell<BTreeMap<NpcId, Option<ShipState>>>,
    }

    impl Reads {
        fn of(watched: NpcId) -> Self {
            Self {
                watched,
                seen: RefCell::default(),
            }
        }

        fn seen(&self, id: NpcId) -> Option<ShipState> {
            self.seen.borrow()[&id]
        }
    }

    impl Behaviour for Reads {
        fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            let watched = around.npc(self.watched).map(|ship| ship.state);
            self.seen.borrow_mut().insert(npc.id, watched);
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

    /// A disabled victim at (100, 0) at rest in slot 5, and a pirate in
    /// slot 3 docking with it (sub 0xf) at `state`: the pirate and the
    /// victim, with a watcher of the pirate in slots 1 and 6.
    fn docking(traffic: &mut Traffic, state: ShipState) -> [NpcId; 4] {
        let below = traffic.add_npc_at(ship(0, TRADERS, AiType::Warship, -500.0, 0.0), 1);
        let mut victim = ship(0, TRADERS, AiType::WimpyTrader, 100.0, 0.0);
        victim.condition = Condition::Disabled;
        let victim = traffic.add_npc_at(victim, 5);
        let mut pirate = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
        pirate.state = state;
        pirate.goal = Goal::Plunder {
            ship: ShipRef::Npc(victim),
            hopeless: false,
        };
        pirate.target = Some(ShipRef::Npc(victim));
        pirate.hull.class_turn = 3.0;
        pirate.docking = Some(Docking {
            ship: ShipRef::Npc(victim),
            sub: DockSub::Dock,
        });
        let pirate = traffic.add_npc_at(pirate, 3);
        let above = traffic.add_npc_at(ship(0, TRADERS, AiType::Warship, -500.0, 0.0), 6);
        [below, pirate, victim, above]
    }

    fn at(x: f32, y: f32, vx: f32, vy: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading: 0.0,
        }
    }

    #[test]
    fn a_higher_slots_think_reads_a_docking_ships_matched_velocity_and_creep_that_tick() {
        // Sub 0xf's match (@0x86c08-0x86cb2) and creep (@0x8700f-0x87095)
        // are the pirate's own low level's, at its slot.
        let matching = at(100.0, 50.0, 0.0, 3.0);
        let creeping = at(100.0, 10.0, 0.0, 0.0);
        for (start, after) in [
            (matching, at(100.0, 50.0, 0.0, 2.85)),
            (creeping, at(100.0, 9.0, 0.0, 0.0)),
        ] {
            let mut traffic = Traffic::with_interval(1);
            let [below, pirate, _, above] = docking(&mut traffic, start);
            let reads = Reads::of(pirate);
            traffic.think_holding(&reads, World::new(&[]), &[], &mut NeverFires);
            let state = |id| reads.seen(id);
            assert_eq!(state(below), Some(start), "as the tick began");
            assert_eq!(state(above), Some(after), "after its slot");
        }
    }

    /// A disabled patient at (100, 0) at rest in slot 5, with 10 armour,
    /// and a repairer in slot 3 docked with it, its wait `wait`: the
    /// repairer and the patient, with a watcher of the patient in slots 1
    /// and 6.
    fn repairing(traffic: &mut Traffic, wait: u32) -> [NpcId; 4] {
        let watcher = |traffic: &mut Traffic, berth| {
            traffic.add_npc_at(ship(0, TRADERS, AiType::Warship, -500.0, 0.0), berth)
        };
        let below = watcher(traffic, 1);
        let mut patient = ship(0, TRADERS, AiType::WimpyTrader, 100.0, 0.0);
        patient.condition = Condition::Disabled;
        patient.boarded = true;
        let patient = traffic.add_npc_at(patient, 5);
        let mut helper = ship(0, TRADERS, AiType::WimpyTrader, 101.0, 1.0);
        helper.goal = Goal::Assist(Help::Repair);
        helper.target = Some(ShipRef::Npc(patient));
        helper.hull.class_turn = 3.0;
        helper.ai_wait = wait;
        helper.docking = Some(Docking {
            ship: ShipRef::Npc(patient),
            sub: DockSub::Dock,
        });
        let helper = traffic.add_npc_at(helper, 3);
        let above = watcher(traffic, 6);
        [below, helper, patient, above]
    }

    #[test]
    fn a_repair_completed_at_its_slot_is_queued_for_that_slots_line_settle() {
        // The completion is the repairer's own low level's (@0x86d54-0x86f37):
        // its line settles before slot 4's think, and it drifts at once
        // (@0x86f09-0x86f31).
        let mut traffic = Traffic::with_interval(1);
        let [_, helper, patient, _] = repairing(&mut traffic, 50);
        let reads = Reads::of(patient);
        traffic.open_think(&reads, World::new(&[]), &[], &mut NeverFires);
        for slot in [1, 3] {
            let picks = |npc: &Npc| npc.berth == slot;
            traffic.think_run(
                &reads,
                World::new(&[]),
                picks,
                &NoBay::new(),
                &mut NeverFires,
            );
        }
        let repaired = PirateEvent::NpcRepaired {
            helper,
            ship: patient,
        };
        assert_eq!(
            traffic.take_on_line_through(3),
            [OnLine::Pirate(repaired)],
            "settled as slot 3's run ends"
        );
        let npc = of(&traffic, helper);
        assert_eq!(
            (npc.goal, npc.docking, npc.target),
            (Goal::Drift, None, None)
        );
    }

    #[test]
    fn a_dock_started_at_its_slot_completes_nothing_that_tick() {
        // The completion reads the wait the start had not yet set: one
        // pass, two branches (@0x86d54-0x86d6d, @0x86f98-0x86f9e). With
        // `Rand(80)` of 0 the wait is 100, a completion's next tick.
        let mut traffic = Traffic::with_interval(1);
        let [_, helper, patient, _] = repairing(&mut traffic, 0);
        let npc = traffic.npcs_mut().iter_mut().find(|npc| npc.id == patient);
        npc.expect("there").boarded = false;
        let reads = Reads::of(patient);
        let draws = &mut Draws::of(&[0; 8]);
        traffic.tick_in(&reads, World::new(&[]), &[], draws);
        assert!(of(&traffic, patient).boarded, "the start boarded it");
        assert_eq!(traffic.take_on_line(), []);
        assert_eq!(of(&traffic, helper).goal, Goal::Assist(Help::Repair));
        traffic.tick_in(&reads, World::new(&[]), &[], draws);
        assert_eq!(traffic.take_on_line().len(), 1, "the next tick");
        assert_eq!(of(&traffic, helper).goal, Goal::Drift);
    }

    #[test]
    fn on_double_times_second_pass_a_docking_ship_matches_and_creeps_nothing() {
        // `_DoPlayGameWork(0)` runs `_HandleShip` alone (@0x44350): it flies
        // the heading and thrust the advance pass's low level left, and
        // moves the ship on.
        let creeping = at(100.0, 10.0, 0.0, 0.0);
        let mut traffic = Traffic::with_interval(1);
        let [_, pirate, ..] = docking(&mut traffic, creeping);
        let reads = Reads::of(pirate);
        traffic.tick_in(&reads, World::new(&[]), &[], &mut NeverFires);
        assert_eq!(of(&traffic, pirate).state.position, Vec2::new(100.0, 9.0));
        traffic.move_in(World::new(&[]), &mut NeverFires);
        assert_eq!(
            of(&traffic, pirate).state.position,
            Vec2::new(100.0, 9.0),
            "no creep, at rest"
        );
        let mut matching = Traffic::with_interval(1);
        let [_, pirate, ..] = docking(&mut matching, at(100.0, 50.0, 0.0, 3.0));
        matching.tick_in(&reads, World::new(&[]), &[], &mut NeverFires);
        let once = of(&matching, pirate).state.velocity.y;
        matching.move_in(World::new(&[]), &mut NeverFires);
        let twice = of(&matching, pirate).state.velocity.y;
        // Damped from 3 to 2.85, then a thrust against it.
        let thrust = 2.85 - once;
        assert!(thrust > 0.0, "{once}");
        assert!(
            (twice - (once - thrust)).abs() < 1e-5,
            "thrust alone, undamped: {twice}"
        );
    }
}
