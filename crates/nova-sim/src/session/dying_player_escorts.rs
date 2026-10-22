//! The player's death frees its escorts
//! ([`Session::free_dying_players_escorts`]).
//!
//! `_EscortAI`'s lost parent (@0x83917-0x8397b) takes a ship whose parent
//! is `_IsDying` (@0x253d: its death timer set, or its armour at or below
//! none) out of its fleet: parent -1, goal 0, sub 0, its class's
//! `InherentAI` (@0x83c4a), its target kept. For the player (parent 0,
//! whose slot is never inactive) that frees, at each one's think, every
//! ship it leads: its hired and captured escorts and its launched
//! fighters, which drop AI type 5 and so their 1.333 capacities. No other
//! routine frees them at its death.
//!
//! Here the session frees them before the traffic's AI
//! (`Session::before_traffic_ai`, after the mission escorts join), so the
//! advance pass's think half sees it after the collision passes, as the
//! original's AI loop (@0x44270) follows them (@0x43e70-0x44015): a hit
//! there frees the escorts before the next frame's eject (`_HandlePlayer`
//! @0x6f22c), and a player who then ejects into a pod has lost them. Each
//! leaves the fleet as the pod releases it ([`Session::release_led`]). A
//! mission escort stays its mission's: the dispatch leads it again each
//! frame (@0x8fbcd-0x8fc22), so it idles under its own think
//! ([`EscortAi`](crate::ai::EscortAi)); its +0x9a of -1 between its think
//! and the next dispatch is
//! rdm:phase/escort-fleet-fidelity-followups/phase-9-left-out-of-leader-succession's.
//! An escort's own fighters are led by the escort, not the player, and
//! stay. No rulebook key: no reading disagrees with the original's.
//!
//! Nothing here plays a sound or draws a random number.

use super::Session;
use crate::ai::Goal;
use crate::combat::hull::Condition;
use crate::traffic::npc::NpcId;

impl Session {
    /// While the player breaks up or is destroyed, every ship it leads
    /// that is its own (hired or captured escorts, launched fighters)
    /// leaves its fleet (see the module docs).
    pub(super) fn free_dying_players_escorts(&mut self) {
        let condition = self.ship.condition;
        if condition != Condition::Destroyed && !condition.is_dying(self.pilot.reserves.armor) {
            return;
        }
        let led: Vec<NpcId> = self
            .space
            .traffic
            .npcs()
            .iter()
            .filter(|npc| npc.is_players())
            .map(|npc| npc.id)
            .collect();
        for id in led {
            self.release_led(id);
        }
    }

    /// NPC `id`, a ship the player leads, is released (+0x9a = -1): out of
    /// the fleet or the formation ([`Session::unfleet`]), out of its fleet
    /// with its class's `InherentAI` and no carrier
    /// ([`Npc::leave_fleet`](crate::traffic::npc::Npc::leave_fleet)), and
    /// deciding afresh (goal 0); the rest re-form. The pod's release and
    /// the player's death share it.
    pub(super) fn release_led(&mut self, id: NpcId) {
        self.unfleet(id);
        if let Some(npc) = self.space.npc_mut(id) {
            npc.leave_fleet();
            npc.goal = Goal::Idle;
        }
        self.reform();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::{Behaviour, Surroundings};
    use crate::bay::Carrier;
    use crate::chance::{Chance, NeverFires};
    use crate::combat::{Rules, ShipRef};
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::mission::ships::ShipBehav;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::rulebook::RuleSource;
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, npc};
    use crate::traffic::npc::{AiType, MissionMark, Npc};

    /// Every ship keeps its goal and target.
    #[derive(Debug)]
    struct Keeping;

    impl Behaviour for Keeping {
        fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    /// A ship of a warship's class (`InherentAI` 3) flying as a trader,
    /// as a captured ship keeps its own type, targeting the player.
    fn ship() -> Npc {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.ai_type = AiType::WimpyTrader;
        ship.hull.inherent_ai = 3;
        ship.target = Some(ShipRef::Player);
        ship
    }

    /// The player's escort record, `carried` for a fighter.
    fn record(carried: bool) -> Escort {
        Escort {
            ship: crate::catalog::ShipId(129),
            reserves: Reserves::full(1.0, 1.0, 1.0),
            order: None,
            carried,
            wage: None,
            person: None,
        }
    }

    /// The ships of the player's fleet: a hired escort, a fighter it
    /// launched, a mission escort, and the escort's own fighter.
    struct Fleet {
        session: Session,
        escort: NpcId,
        fighter: NpcId,
        mission: NpcId,
        escorts_fighter: NpcId,
    }

    fn fleet() -> (Fleet, FakePilotCatalog) {
        let catalog = catalog();
        let mut session =
            Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies");
        // Its first traffic tick enters the system, before the fleet
        // arrives.
        traffic_tick(&mut session, &catalog);
        let duty = EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        };
        let escort = session.space.traffic.add_npc(Npc {
            escort: Some(duty),
            goal: Goal::Formation { guard: None },
            ..ship()
        });
        session.pilot.escorts.push(record(false));
        session.space.escorts.fleet.push(Some(escort));
        let mut fighter = Npc {
            escort: Some(duty),
            goal: Goal::Formation { guard: None },
            ..ship()
        };
        fighter.take_carrier(session.player_carrier(30), RuleSource::Engine);
        let fighter = session.space.traffic.add_npc(fighter);
        session.pilot.escorts.push(record(true));
        session.space.escorts.fleet.push(Some(fighter));
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 4));
        let mission = session.space.traffic.add_npc(Npc {
            mission: Some(MissionMark {
                slot: 0,
                behav: ShipBehav::EscortPlayer,
                formation: Some(FormationSlot::default()),
                order: None,
            }),
            goal: Goal::Formation { guard: None },
            ..ship()
        });
        let mut escorts_fighter = Npc {
            leader: Some(escort),
            goal: Goal::Formation { guard: None },
            ..ship()
        };
        escorts_fighter.take_carrier(
            Carrier {
                ship: ShipRef::Npc(escort),
                window: 100.0,
                reach: 40.0,
            },
            RuleSource::Engine,
        );
        let escorts_fighter = session.space.traffic.add_npc(escorts_fighter);
        session.reform();
        (
            Fleet {
                session,
                escort,
                fighter,
                mission,
                escorts_fighter,
            },
            catalog,
        )
    }

    /// A traffic tick, its work before the AI among it.
    fn traffic_tick(session: &mut Session, catalog: &FakePilotCatalog) {
        session.tick_traffic(catalog, &Keeping, &mut NeverFires);
    }

    /// An advance pass.
    fn advance_pass(session: &mut Session, catalog: &FakePilotCatalog) {
        session.game_work(
            crate::flight::Controls::default(),
            catalog,
            &Keeping,
            Rules::default(),
            &mut NeverFires,
        );
    }

    type Tick = fn(&mut Session, &FakePilotCatalog);

    const TICKS: [(&str, Tick); 2] = [("traffic", traffic_tick), ("advance", advance_pass)];

    /// A traffic tick by Nova's AI.
    fn nova_traffic_tick(session: &mut Session, catalog: &FakePilotCatalog) {
        session.tick_traffic(catalog, &crate::ai::NovaAi::default(), &mut NeverFires);
    }

    /// An advance pass by Nova's AI.
    fn nova_advance_pass(session: &mut Session, catalog: &FakePilotCatalog) {
        session.game_work(
            crate::flight::Controls::default(),
            catalog,
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// The player breaking up, as `_IsDying` reads it each way: its death
    /// timer set, or its armour gone.
    fn dying(session: &mut Session, timer: bool) {
        if timer {
            session.ship.condition = Condition::Dying {
                ticks_left: 30,
                starting: false,
            };
        } else {
            session.pilot.reserves.armor.now = 0.0;
        }
    }

    #[test]
    fn while_the_player_breaks_up_its_escorts_and_fighters_leave_its_fleet() {
        for (name, tick) in TICKS {
            for timer in [true, false] {
                let (mut fleet, catalog) = fleet();
                dying(&mut fleet.session, timer);
                tick(&mut fleet.session, &catalog);
                let session = &fleet.session;
                for id in [fleet.escort, fleet.fighter] {
                    let freed = session.space.npc(id).expect("here");
                    assert!(!freed.escorts_player(), "{name} {timer}: {id:?}");
                    assert_eq!(freed.ai_type, AiType::Warship, "{name} {timer}");
                    assert_eq!(freed.goal, Goal::Idle, "{name} {timer}");
                    assert_eq!(freed.carrier, None, "{name} {timer}");
                    assert_eq!(freed.target, Some(ShipRef::Player), "{name} {timer}");
                }
                let fighter = session.space.npc(fleet.fighter).expect("here");
                assert_eq!(
                    (fighter.reserves.shield.max, fighter.reserves.armor.max),
                    (30.0, 45.0),
                    "{name} {timer}: its class's capacities"
                );
                assert_eq!(session.pilot.escorts, [], "{name} {timer}");
                assert_eq!(session.space.escorts.fleet, [], "{name} {timer}");
                let own = session.space.npc(fleet.escorts_fighter).expect("here");
                assert_eq!(
                    (own.leader, own.carrier.map(|bay| bay.ship), own.ai_type),
                    (
                        Some(fleet.escort),
                        Some(ShipRef::Npc(fleet.escort)),
                        AiType::WimpyTrader
                    ),
                    "{name} {timer}: the escort's own fighter stays its"
                );
            }
        }
    }

    #[test]
    fn a_mission_escort_idles_keeping_its_target_and_stays_its_missions() {
        let ticks: [(&str, Tick); 2] = [
            ("traffic", nova_traffic_tick),
            ("advance", nova_advance_pass),
        ];
        for (name, tick) in ticks {
            let (mut fleet, catalog) = fleet();
            dying(&mut fleet.session, true);
            tick(&mut fleet.session, &catalog);
            let mission = fleet.session.space.npc(fleet.mission).expect("here");
            assert!(mission.mission_escort(), "{name}");
            assert_eq!(mission.goal, Goal::Idle, "{name}");
            assert_eq!(mission.target, Some(ShipRef::Player), "{name}");
            assert_eq!(mission.ai_type, AiType::WimpyTrader, "{name}");
        }
    }

    #[test]
    fn an_intact_or_disabled_player_frees_no_one() {
        for (name, tick) in TICKS {
            for condition in [Condition::Intact, Condition::Disabled] {
                let (mut fleet, catalog) = fleet();
                fleet.session.ship.condition = condition;
                fleet.session.pilot.reserves.armor.now = 1.0;
                tick(&mut fleet.session, &catalog);
                for id in [fleet.escort, fleet.fighter] {
                    let kept = fleet.session.space.npc(id).expect("here");
                    assert!(kept.escorts_player(), "{name} {condition:?}");
                }
                assert_eq!(fleet.session.pilot.escorts.len(), 2, "{name} {condition:?}");
            }
        }
    }
}
