//! A ship of an NPC's fleet whose lead or carrier is lost leaves the
//! fleet ([`Space::lose_leads`]), after each traffic tick's motion and
//! each fight (`Session::after_traffic_motion`, `fight_tail`).
//!
//! By the engine's reading of the rulebook's
//! [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi), every ship with
//! an NPC parent (a fleet's escort, a ship a pirate took, an NPC
//! carrier's fighter) whose lead is gone from the system or not intact,
//! or whose carrier is, loses its lead and its carrier, idles (goal 0)
//! and takes its class's `InherentAI` as its AI type, as `_EscortAI`'s
//! lost-parent branch has it (@0x83917-0x8397b, the AI type written
//! @0x83c4a from class +0x12). Its target is kept, as the branch writes
//! none. A fighter that leaves holds its class's shield and armour
//! capacities again, keeping what it holds ([`Npc::leave_carrier`](crate::traffic::npc::Npc::leave_carrier)),
//! as its AI type is no longer 5. For an NPC parent the original reaches that branch only with
//! the parent already -1: `_AICheckParentExists` runs first on every frame
//! (`_DoPlayGameWork` @0x440c6-0x44101) and handles a parent inactive or
//! disabled. This pass stands in for it until
//! rdm:phase/escort-fleet-fidelity/phase-7-leader-succession replaces its
//! NPC-parent half with `_AICheckParentExists`'s succession and goal 0x13.
//! Its +0x50 write (-1.0) is
//! rdm:phase/escort-fleet-fidelity/phase-8-jump-with-leader's.
//!
//! By the other reading, the port's earlier pass: only a fighter whose
//! carrier is gone or not intact leaves, keeping its AI type, and a fleet
//! escort whose lead dies keeps its lead.
//!
//! The original leaves the fleet in the escort's own think or the frame's
//! parent check; the engine settles it after the motion and the fight,
//! before the frame is drawn: a lag of under a tick, which crosses no
//! boundary the player sees.

use super::Space;
use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::rulebook::RuleSource;
use crate::traffic::npc::{AiType, NpcId};

impl Space {
    /// Every ship whose lead or carrier is lost leaves its fleet, as
    /// `reading` reads [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi)
    /// (see the module docs).
    pub(super) fn lose_leads(&mut self, reading: RuleSource) {
        let intact: Vec<NpcId> = self
            .traffic
            .npcs()
            .iter()
            .filter(|npc| npc.condition == Condition::Intact)
            .map(|npc| npc.id)
            .collect();
        let lost = |id: NpcId| !intact.contains(&id);
        for npc in self.traffic.npcs_mut() {
            let carrier_lost = matches!(
                npc.carrier,
                Some(crate::bay::Carrier { ship: ShipRef::Npc(carrier), .. }) if lost(carrier)
            );
            let lead_lost = npc.leader.is_some_and(lost);
            match reading {
                RuleSource::Engine if carrier_lost || lead_lost => {
                    npc.ai_type = AiType::from_raw(npc.hull.inherent_ai);
                }
                RuleSource::Bible if carrier_lost => {}
                _ => continue,
            }
            npc.leave_carrier();
            npc.leader = None;
            npc.goal = Goal::Idle;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::bay::Carrier;
    use crate::chance::{Chance, NeverFires};
    use crate::combat::hull::Condition;
    use crate::combat::{Rules, ShipRef};
    use crate::pilot::Pilot;
    use crate::rulebook::{RuleKey, Rulebook};
    use crate::session::Session;
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, npc};
    use crate::traffic::npc::{AiType, Npc, NpcId};

    /// A session flying in an empty system.
    fn session() -> Session {
        let catalog = catalog();
        Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies")
    }

    /// A pirate warship, its class's `InherentAI` a warship's (3).
    fn pirate() -> Npc {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.ai_type = AiType::Warship;
        ship.hull.inherent_ai = 3;
        ship
    }

    /// A ship of the fleet of `lead`, flying as a trader though its class
    /// is a warship's, as a captured ship keeps its own type, following
    /// its lead and targeting the player.
    fn escort_of(lead: NpcId) -> Npc {
        let mut ship = pirate();
        ship.ai_type = AiType::WimpyTrader;
        ship.leader = Some(lead);
        ship.goal = Goal::Follow { lead, guard: None };
        ship.target = Some(ShipRef::Player);
        ship
    }

    /// [`escort_of`] `lead` as a fighter of `carrier`.
    fn fighter_of(lead: NpcId, carrier: NpcId) -> Npc {
        Npc {
            carrier: Some(Carrier {
                ship: ShipRef::Npc(carrier),
                window: 100.0,
                reach: 40.0,
            }),
            ..escort_of(lead)
        }
    }

    /// `session`'s NPC `id`.
    fn ship(session: &Session, id: NpcId) -> &Npc {
        session.space.npc(id).expect("in the system")
    }

    /// Whether `ship` has left its fleet: no lead, no carrier, idle, and
    /// flying by `ai_type`, its target kept.
    fn left(ship: &Npc, ai_type: AiType) -> bool {
        ship.leader.is_none()
            && ship.carrier.is_none()
            && ship.goal == Goal::Idle
            && ship.ai_type == ai_type
            && ship.target == Some(ShipRef::Player)
    }

    #[test]
    fn a_pirate_escort_whose_lead_dies_leaves_the_fleet_with_its_classs_inherent_ai() {
        // `_EscortAI`'s lost parent (@0x83917-0x8397b): parent -1, AI type
        // the class's `InherentAI` (+0x12, @0x83c4a), goal 0.
        for condition in [
            Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
            Condition::Disabled,
            Condition::Destroyed,
        ] {
            let mut session = session();
            let lead = session.space.traffic.add_npc(pirate());
            let escort = session.space.traffic.add_npc(escort_of(lead));
            session.space.npc_mut(lead).expect("there").condition = condition;
            session.space.lose_leads(RuleSource::Engine);
            assert!(
                left(ship(&session, escort), AiType::Warship),
                "{condition:?}: {:?}",
                ship(&session, escort)
            );
        }
    }

    #[test]
    fn a_fleet_escort_whose_lead_has_left_the_system_leaves_the_fleet() {
        let mut session = session();
        let escort = session.space.traffic.add_npc(escort_of(NpcId(99)));
        session.space.lose_leads(RuleSource::Engine);
        assert!(left(ship(&session, escort), AiType::Warship));
    }

    #[test]
    fn a_fleet_escort_whose_lead_is_intact_keeps_it() {
        for reading in RuleSource::ALL {
            let mut session = session();
            let lead = session.space.traffic.add_npc(pirate());
            let escort = session.space.traffic.add_npc(escort_of(lead));
            let before = ship(&session, escort).clone();
            session.space.lose_leads(reading);
            assert_eq!(ship(&session, escort), &before, "{reading:?}");
        }
    }

    #[test]
    fn by_the_other_reading_a_pirate_escort_whose_lead_dies_keeps_it_as_before() {
        let mut session = session();
        let lead = session.space.traffic.add_npc(pirate());
        let escort = session.space.traffic.add_npc(escort_of(lead));
        let gone = session.space.traffic.add_npc(escort_of(NpcId(99)));
        session.space.npc_mut(lead).expect("there").condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let before = (ship(&session, escort).clone(), ship(&session, gone).clone());
        session.space.lose_leads(RuleSource::Bible);
        assert_eq!(
            (ship(&session, escort), ship(&session, gone)),
            (&before.0, &before.1)
        );
    }

    #[test]
    fn a_fighter_whose_carrier_is_lost_leaves_by_either_reading() {
        // The port's earlier orphaning keeps the fighter's AI type; the
        // original's gives it its class's.
        for (reading, ai_type) in [
            (RuleSource::Engine, AiType::Warship),
            (RuleSource::Bible, AiType::WimpyTrader),
        ] {
            let mut session = session();
            let lead = session.space.traffic.add_npc(pirate());
            let carrier = session.space.traffic.add_npc(escort_of(lead));
            let fighter = session.space.traffic.add_npc(fighter_of(lead, carrier));
            let gone = session.space.traffic.add_npc(fighter_of(lead, NpcId(99)));
            session.space.npc_mut(carrier).expect("there").condition = Condition::Disabled;
            session.space.lose_leads(reading);
            for id in [fighter, gone] {
                assert!(
                    left(ship(&session, id), ai_type),
                    "{reading:?}: {:?}",
                    ship(&session, id)
                );
            }
            assert_eq!(ship(&session, carrier).leader, Some(lead), "{reading:?}");
        }
    }

    #[test]
    fn a_fighter_whose_carrier_is_lost_holds_its_classs_capacities_again_keeping_what_it_has() {
        // Its AI type no longer 5, `_ShipShieldCapacity` and
        // `_ShipArmorCapacity` read its class's (@0x2a91, @0x2cb9); the
        // original keeps +0x54 and +0x58 as they are.
        for reading in RuleSource::ALL {
            let mut session = session();
            let lead = session.space.traffic.add_npc(pirate());
            let carrier = session.space.traffic.add_npc(escort_of(lead));
            let mut scaled = fighter_of(lead, carrier);
            scaled.hold_capacities(RuleSource::Engine);
            scaled.reserves.shield.now = 35.0;
            scaled.reserves.armor.now = 50.0;
            let fighter = session.space.traffic.add_npc(scaled);
            session.space.npc_mut(carrier).expect("there").condition = Condition::Destroyed;
            session.space.lose_leads(reading);
            let reserves = ship(&session, fighter).reserves;
            assert_eq!(
                (reserves.shield.now, reserves.shield.max),
                (35.0, 30.0),
                "{reading:?}"
            );
            assert_eq!(
                (reserves.armor.now, reserves.armor.max),
                (50.0, 45.0),
                "{reading:?}"
            );
        }
    }

    #[test]
    fn a_fighter_whose_carrier_is_intact_but_lead_is_lost_leaves_by_the_engine_alone() {
        let lost = |reading: RuleSource| {
            let mut session = session();
            let lead = session.space.traffic.add_npc(pirate());
            let carrier = session.space.traffic.add_npc(pirate());
            let fighter = session.space.traffic.add_npc(fighter_of(lead, carrier));
            session.space.npc_mut(lead).expect("there").condition = Condition::Disabled;
            session.space.lose_leads(reading);
            ship(&session, fighter).leader.is_none()
        };
        assert!(lost(RuleSource::Engine));
        assert!(!lost(RuleSource::Bible));
    }

    // Through the session's ticks.

    /// Thinks nothing: each ship keeps its goal, so only the session's
    /// own work moves a ship out of its fleet.
    #[derive(Debug)]
    struct Keeping;

    impl Behaviour for Keeping {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }
    }

    /// A tick of the session, reading the catalog given.
    type Tick = fn(&mut Session, &FakePilotCatalog);

    /// Each tick that takes a ship out of its fleet, by name.
    const TICKS: [(&str, Tick); 2] = [("traffic", traffic_tick), ("fight", fight_tick)];

    /// A traffic tick, its work after the motion
    /// (`Session::after_traffic_motion`) among it.
    fn traffic_tick(session: &mut Session, catalog: &FakePilotCatalog) {
        session.tick_traffic(catalog, &Keeping, &mut NeverFires);
    }

    /// A fight's tick, its tail (`Session::fight_tail`) among it.
    fn fight_tick(session: &mut Session, _catalog: &FakePilotCatalog) {
        session.tick_combat(Rules::default(), &mut NeverFires);
    }

    /// A pirate lead disabled and its escort as the escort is after a
    /// `tick` of a session by `rules`, its first traffic tick done before
    /// the two arrive.
    fn escort_after(rules: Rulebook, tick: Tick) -> (NpcId, Npc) {
        let catalog = catalog();
        let mut session = Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts"))
            .expect("flies")
            .with_rules(rules);
        traffic_tick(&mut session, &catalog);
        let lead = session.space.traffic.add_npc(pirate());
        let escort = session.space.traffic.add_npc(escort_of(lead));
        // Its armour down to 1 of 45 as well, as the fight reads its
        // condition afresh from its armour.
        let disabled = session.space.npc_mut(lead).expect("there");
        disabled.condition = Condition::Disabled;
        disabled.reserves.armor.now = 1.0;
        tick(&mut session, &catalog);
        (lead, ship(&session, escort).clone())
    }

    #[test]
    fn by_the_default_rules_either_tick_takes_an_escort_whose_lead_is_disabled_out_of_its_fleet() {
        for (name, tick) in TICKS {
            let (_, escort) = escort_after(Rulebook::default(), tick);
            // The traffic tick lets go of the player as a target by either
            // reading, so only the fleet's fields are read here.
            assert_eq!(
                (escort.leader, escort.carrier, escort.goal, escort.ai_type),
                (None, None, Goal::Idle, AiType::Warship),
                "{name}"
            );
        }
    }

    #[test]
    fn by_the_other_reading_neither_tick_takes_an_escort_whose_lead_is_disabled_out() {
        let rules = Rulebook::default().with_override(RuleKey::NpcEscortAi, RuleSource::Bible);
        for (name, tick) in TICKS {
            let (lead, escort) = escort_after(rules, tick);
            assert_eq!(escort.leader, Some(lead), "{name}: {escort:?}");
            assert_eq!(escort.ai_type, AiType::WimpyTrader, "{name}");
            assert!(matches!(escort.goal, Goal::Follow { .. }), "{name}");
        }
    }

    #[test]
    fn the_players_ships_and_a_ship_with_no_lead_are_left_as_they_are() {
        let mut session = session();
        let mut players = pirate();
        players.carrier = Some(Carrier {
            ship: ShipRef::Player,
            window: 100.0,
            reach: 40.0,
        });
        players.ai_type = AiType::WimpyTrader;
        let fighter = session.space.traffic.add_npc(players);
        let mut alone = pirate();
        alone.ai_type = AiType::Interceptor;
        let alone = session.space.traffic.add_npc(alone);
        let before = (
            ship(&session, fighter).clone(),
            ship(&session, alone).clone(),
        );
        session.space.lose_leads(RuleSource::Engine);
        assert_eq!(
            (ship(&session, fighter), ship(&session, alone)),
            (&before.0, &before.1)
        );
    }
}
