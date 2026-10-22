//! The port's earlier lost-lead pass ([`Space::lose_leads`]), after each
//! traffic tick's motion and each fight (`Session::after_traffic_motion`,
//! `fight_tail`), by the other reading of the rulebook's
//! [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi): only a fighter
//! whose carrier is gone or not intact leaves its fleet, keeping its AI
//! type, its class's shield and armour capacities held again
//! ([`Npc::leave_carrier`](crate::traffic::npc::Npc::leave_carrier)), and
//! idles; a fleet escort whose lead dies keeps its lead.
//!
//! By the engine's reading it does nothing. A ship whose lead is lost is
//! the traffic's think half's, as the original's: `_AICheckParentExists`
//! gives every ship of a lost lead a successor to follow, or makes it lead,
//! or leaves it alone in the regroup pause (`nova_sim_ai::traffic::succession`),
//! before the formation pass; `_AIDispatch`'s disabled reset drops a
//! disabled ship's leader (`nova_sim_ai::traffic::disabled_reset`); and
//! `_EscortAI`'s lost parent takes a ship with no leader, whole again,
//! out of its fleet with its class's `InherentAI`
//! (`nova_sim_ai::ai::fleet_escort`). A lead lost in a fight is so
//! succeeded at the next think, where the original's marking pass runs
//! (@0x4405e-0x441f7) before the frame's dispatches.

use super::Space;
use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::rulebook::RuleSource;
use crate::traffic::npc::NpcId;

impl Space {
    /// Every fighter whose carrier is lost leaves its fleet, as the other
    /// reading of [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi)
    /// has it; nothing by the engine's `reading` (see the module docs).
    pub(super) fn lose_leads(&mut self, reading: RuleSource) {
        if reading == RuleSource::Engine {
            return;
        }
        let intact: Vec<NpcId> = self
            .traffic
            .npcs()
            .iter()
            .filter(|npc| npc.condition == Condition::Intact)
            .map(|npc| npc.id)
            .collect();
        for npc in self.traffic.npcs_mut() {
            if matches!(
                npc.carrier,
                Some(crate::bay::Carrier { ship: ShipRef::Npc(carrier), .. }) if !intact.contains(&carrier)
            ) {
                npc.leave_carrier();
                npc.leader = None;
                npc.goal = Goal::Idle;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::regroup::Kept;
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

    #[test]
    fn by_the_other_reading_a_fighter_whose_carrier_is_lost_leaves_keeping_its_type() {
        let mut session = session();
        let lead = session.space.traffic.add_npc(pirate());
        let carrier = session.space.traffic.add_npc(escort_of(lead));
        let fighter = session.space.traffic.add_npc(fighter_of(carrier, carrier));
        let gone = session
            .space
            .traffic
            .add_npc(fighter_of(NpcId(99), NpcId(99)));
        session.space.npc_mut(carrier).expect("there").condition = Condition::Disabled;
        session.space.lose_leads(RuleSource::Bible);
        for id in [fighter, gone] {
            let left = ship(&session, id);
            assert_eq!(
                (
                    left.leader,
                    left.carrier,
                    left.goal,
                    left.ai_type,
                    left.target
                ),
                (
                    None,
                    None,
                    Goal::Idle,
                    AiType::WimpyTrader,
                    Some(ShipRef::Player)
                ),
                "{id:?}"
            );
        }
        assert_eq!(ship(&session, carrier).leader, Some(lead), "its lead kept");
    }

    #[test]
    fn by_the_other_reading_a_fighter_whose_carrier_is_lost_holds_its_classs_capacities() {
        // The original keeps +0x54 and +0x58 as they are.
        let mut session = session();
        let carrier = session.space.traffic.add_npc(pirate());
        let mut scaled = fighter_of(carrier, carrier);
        scaled.hold_capacities(RuleSource::Engine);
        scaled.reserves.shield.now = 35.0;
        scaled.reserves.armor.now = 50.0;
        let fighter = session.space.traffic.add_npc(scaled);
        session.space.npc_mut(carrier).expect("there").condition = Condition::Destroyed;
        session.space.lose_leads(RuleSource::Bible);
        let reserves = ship(&session, fighter).reserves;
        assert_eq!((reserves.shield.now, reserves.shield.max), (35.0, 30.0));
        assert_eq!((reserves.armor.now, reserves.armor.max), (50.0, 45.0));
    }

    #[test]
    fn the_pass_leaves_a_ship_whose_lead_alone_is_lost_and_does_nothing_by_the_engine() {
        // By the engine a lost lead or carrier is the traffic's think's.
        for (reading, fighters_kept) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
            let mut session = session();
            let lead = session.space.traffic.add_npc(pirate());
            let carrier = session.space.traffic.add_npc(pirate());
            let escort = session.space.traffic.add_npc(escort_of(lead));
            let fighter = session.space.traffic.add_npc(fighter_of(carrier, carrier));
            session.space.npc_mut(lead).expect("there").condition = Condition::Disabled;
            session.space.npc_mut(carrier).expect("there").condition = Condition::Destroyed;
            let before = (
                ship(&session, escort).clone(),
                ship(&session, fighter).clone(),
            );
            session.space.lose_leads(reading);
            assert_eq!(ship(&session, escort), &before.0, "{reading:?}");
            assert_eq!(
                ship(&session, fighter) == &before.1,
                fighters_kept,
                "{reading:?}"
            );
        }
    }

    #[test]
    fn the_players_ships_and_a_ship_with_no_lead_are_left_as_they_are() {
        for reading in RuleSource::ALL {
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
            session.space.lose_leads(reading);
            assert_eq!(
                (ship(&session, fighter), ship(&session, alone)),
                (&before.0, &before.1),
                "{reading:?}"
            );
        }
    }

    // Through the session's ticks.

    /// Thinks nothing: each ship keeps its goal, so only the session's
    /// own work and the traffic's succession move a ship out of its
    /// fleet.
    #[derive(Debug)]
    struct Keeping;

    impl Behaviour for Keeping {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }
    }

    /// A tick of the session, reading the catalog given.
    type Tick = fn(&mut Session, &FakePilotCatalog);

    /// A traffic tick, its think half and its work after the motion
    /// (`Session::after_traffic_motion`) among it.
    fn traffic_tick(session: &mut Session, catalog: &FakePilotCatalog) {
        session.tick_traffic(catalog, &Keeping, &mut NeverFires);
    }

    /// An advance pass, its AI and its fight's tail among it.
    fn advance_pass(session: &mut Session, catalog: &FakePilotCatalog) {
        session.game_work(
            crate::flight::Controls::default(),
            catalog,
            &Keeping,
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// A fight's tick alone, its tail (`Session::fight_tail`) among it.
    fn fight_tick(session: &mut Session, _catalog: &FakePilotCatalog) {
        session.tick_combat(Rules::default(), &mut NeverFires);
    }

    /// A pirate lead disabled, a heavy escort and a light one, as the
    /// escorts are after a `tick` of a session by `rules`, its first
    /// traffic tick done before they arrive: (lead, heavy, light).
    fn fleet_after(rules: Rulebook, tick: Tick) -> (NpcId, Npc, Npc) {
        let catalog = catalog();
        let mut session = Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts"))
            .expect("flies")
            .with_rules(rules);
        traffic_tick(&mut session, &catalog);
        let lead = session.space.traffic.add_npc(pirate());
        let mut heavy = escort_of(lead);
        heavy.hull.signed_mass = 300;
        let heavy = session.space.traffic.add_npc(heavy);
        let mut light = escort_of(lead);
        light.hull.signed_mass = 100;
        let light = session.space.traffic.add_npc(light);
        // Its armour down to 1 of 45 as well, as the fight reads its
        // condition afresh from its armour.
        let disabled = session.space.npc_mut(lead).expect("there");
        disabled.condition = Condition::Disabled;
        disabled.reserves.armor.now = 1.0;
        tick(&mut session, &catalog);
        (
            lead,
            ship(&session, heavy).clone(),
            ship(&session, light).clone(),
        )
    }

    #[test]
    fn by_the_default_rules_a_think_makes_the_heaviest_escort_lead_and_the_other_pause() {
        for (name, tick) in [
            ("traffic", traffic_tick as Tick),
            ("advance", advance_pass as Tick),
        ] {
            let (_, heavy, light) = fleet_after(Rulebook::default(), tick);
            assert_eq!(
                (heavy.leader, heavy.ai_type),
                (None, AiType::Warship),
                "{name}"
            );
            assert_eq!(light.leader, Some(heavy.id), "{name}");
            assert_eq!(
                light.goal,
                Goal::Regroup {
                    kept: Kept::Formation { guard: None }
                },
                "{name}"
            );
        }
    }

    #[test]
    fn a_fight_alone_changes_nothing_until_the_next_think() {
        let (lead, heavy, light) = fleet_after(Rulebook::default(), fight_tick);
        for escort in [heavy, light] {
            assert_eq!(escort.leader, Some(lead));
            assert!(matches!(escort.goal, Goal::Follow { .. }));
            assert_eq!(escort.ai_type, AiType::WimpyTrader);
        }
    }

    #[test]
    fn by_the_other_reading_no_tick_takes_an_escort_whose_lead_is_disabled_out() {
        let rules = Rulebook::default().with_override(RuleKey::NpcEscortAi, RuleSource::Bible);
        for (name, tick) in [
            ("traffic", traffic_tick as Tick),
            ("advance", advance_pass as Tick),
            ("fight", fight_tick as Tick),
        ] {
            let (lead, heavy, light) = fleet_after(rules, tick);
            for escort in [heavy, light] {
                assert_eq!(escort.leader, Some(lead), "{name}: {escort:?}");
                assert_eq!(escort.ai_type, AiType::WimpyTrader, "{name}");
                assert!(matches!(escort.goal, Goal::Follow { .. }), "{name}");
            }
        }
    }
}
