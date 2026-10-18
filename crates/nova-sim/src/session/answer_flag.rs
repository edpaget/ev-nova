//! The session's part in `_DamageShip`'s answer flag (see
//! [`answer_flag`](crate::ai::answer_flag)): it weighs the fight's
//! strikes against the traffic as they settle, before anything answers
//! them, and eases the NPCs' stray provocation once a fight's tick.

use super::Session;
use crate::chance::Chance;
use crate::combat::Strike;

impl Session {
    /// Weighs the fight's `strikes` against the traffic in flight, rolling
    /// on `chance` ([`Traffic::weigh_strikes`](crate::traffic::Traffic::weigh_strikes)),
    /// then eases every NPC's stray provocation
    /// ([`Traffic::ease_stray`](crate::traffic::Traffic::ease_stray)), as
    /// `_HandleShip` (@0x44483) follows the collision passes and the shots
    /// (@0x43e70-0x44015, @0x44385) in `_DoPlayGameWork`. Both passes of
    /// double time do so, as each runs `_HandleShip`.
    pub(super) fn weigh_strikes(&mut self, strikes: &mut [Strike], chance: &mut dyn Chance) {
        if !strikes.is_empty() {
            self.step_traffic(|traffic, world| traffic.weigh_strikes(&world, strikes, chance));
        }
        self.traffic.ease_stray();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ai::Goal;
    use crate::ai::NovaAi;
    use crate::catalog::{EscortClass, GovtId, GovtRecord, ShipId, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::armament::Trigger;
    use crate::combat::{Rules, ShipRef};
    use crate::handling::ShipFields;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::tests::{FIRE, armed, facing_an_npc};
    use crate::traffic::npc::{AiType, NpcId};
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::FakePilotCatalog;
    use nova_sim_catalog::fixture::{govt, ship};

    /// [`armed`], the traders (140, the NPC's, `CrimeTol` 5) governing the
    /// player's system, with police (141) allied with them.
    fn lawful() -> FakePilotCatalog {
        let mut catalog = armed();
        catalog.govts = vec![
            GovtRecord {
                crime_tol: 5,
                classes: [1, -1, -1, -1],
                ..govt(140)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                classes: [2, -1, -1, -1],
                ..govt(141)
            },
        ];
        catalog.star_map[0].govt = Some(GovtId(140));
        catalog
    }

    /// Sets the player's record in its system to `record`.
    fn on_record(session: &mut Session, record: i16) {
        session.pilot.set_system_record(SystemId(130), record);
        assert_eq!(session.system_record(), record);
    }

    /// Adds a police warship far off, idle: a Good Samaritan.
    fn samaritan(session: &mut Session) -> NpcId {
        let mut police = session.npcs()[0].clone();
        police.govt = Some(GovtId(141));
        police.ai_type = AiType::Warship;
        police.state.position.x += 3000.0;
        police.goal = Goal::Idle;
        session.traffic.add_npc(police)
    }

    /// Fires one shot at NPC 0, the player's target, then lets the target
    /// go and ticks the fight until the shot lands; then the NPCs answer.
    fn shot_after_letting_go(session: &mut Session) {
        session.target = Some(NpcId(0));
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut NeverFires);
        session.hold_trigger(Trigger::default());
        session.target = None;
        for _ in 0..60 {
            if session.npcs()[0].reserves.shield.now < 30.0 {
                break;
            }
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert!(session.npcs()[0].reserves.shield.now < 30.0, "never hit");
        session.answer_strikes(&NovaAi::default());
    }

    #[test]
    fn a_well_regarded_players_shot_at_a_lawful_ship_it_does_not_target_is_let_pass() {
        let catalog = lawful();
        for (record, answers) in [(10, false), (9, true)] {
            let mut session = facing_an_npc(&catalog, 180);
            on_record(&mut session, record);
            let police = samaritan(&mut session);
            shot_after_letting_go(&mut session);
            let npc = &session.npcs()[0];
            assert_eq!(
                npc.target == Some(ShipRef::Player),
                answers,
                "{record}: the ship hit"
            );
            let police = session
                .npcs()
                .iter()
                .find(|npc| npc.id == police)
                .expect("the police");
            assert_eq!(
                police.goal == Goal::Attack(ShipRef::Player),
                answers,
                "{record}: the Samaritan"
            );
        }
    }

    #[test]
    fn a_players_stray_hits_build_stray_provocation_that_eases_each_tick() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let mut ticks = 0;
        while session.npcs()[0].reserves.shield.now >= 30.0 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            session.hold_trigger(Trigger::default());
            ticks += 1;
            assert!(ticks < 60, "never hit");
        }
        // 1.75 x the blaster's `Reload` of 2, less the tick's ease.
        assert_eq!(session.npcs()[0].stray, 3.0);
        session.answer_strikes(&NovaAi::default());
        assert_eq!(session.npcs()[0].target, None, "forgiven: below 50");
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.npcs()[0].stray, 2.5);
    }

    #[test]
    fn the_traffic_weighs_by_the_sessions_rules() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let mut rules = None;
        session.step_traffic(|_, world| rules = Some(world.flag_rules()));
        assert_eq!(rules, Some(crate::ai::answer_flag::FlagRules::default()));
        assert_eq!(session.world().flag_rules(), rules.expect("stepped"));
        for key in [RuleKey::StrayShootPenalty, RuleKey::IndependentKin] {
            let session = facing_an_npc(&catalog, 180)
                .with_rules(Rulebook::default().with_override(key, RuleSource::Bible));
            let mut session = session;
            let mut stepped = None;
            session.step_traffic(|_, world| stepped = Some(world.flag_rules()));
            let read = |rules: crate::ai::answer_flag::FlagRules| match key {
                RuleKey::StrayShootPenalty => (rules.shoot_penalty, rules.independent_kin),
                _ => (rules.independent_kin, rules.shoot_penalty),
            };
            assert_eq!(
                read(stepped.expect("stepped")),
                (RuleSource::Bible, RuleSource::Engine),
                "{key:?}"
            );
            assert_eq!(
                read(session.world().flag_rules()),
                (RuleSource::Bible, RuleSource::Engine),
                "{key:?}: the world"
            );
        }
    }

    #[test]
    fn the_player_side_carries_the_players_target_and_class() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let seen = session.player_side();
        assert_eq!((seen.target, seen.class), (None, EscortClass::Freighter));
        session.target = Some(NpcId(0));
        assert_eq!(session.player_side().target, Some(ShipRef::Npc(NpcId(0))));
        let class = |session: &mut Session, escort_type: i16, inherent_ai: i16, mass: i16| {
            let mut record = ship(session.pilot.ship.0, ShipFields { mass, ..FAST });
            record.escort_type = escort_type;
            record.inherent_ai = inherent_ai;
            session.ships = vec![record];
            session.player_side().class
        };
        assert_eq!(class(&mut session, 0, 1, 500), EscortClass::Fighter);
        assert_eq!(class(&mut session, -1, 3, 49), EscortClass::Fighter);
        assert_eq!(class(&mut session, 2, 3, 49), EscortClass::Warship);
        session.ships = vec![ship(129, FAST)];
        assert_eq!(session.player_side().class, EscortClass::Freighter);
        assert_eq!(session.pilot.ship, ShipId(128));
    }

    #[test]
    fn the_player_side_carries_its_ship_types_crew_for_the_pirates() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        for crew in [-1, 0, 7] {
            let mut record = ship(session.pilot.ship.0, FAST);
            record.crew = crew;
            session.ships = vec![record];
            assert_eq!(session.player_side().hull.crew, crew);
        }
        session.ships = vec![ship(129, FAST)];
        assert_eq!(session.player_side().hull.crew, 0, "no record of its own");
    }
}
