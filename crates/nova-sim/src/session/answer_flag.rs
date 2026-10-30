//! The session's part in `_DamageShip`'s answer flag (see
//! [`answer_flag`](crate::ai::answer_flag)): as the fight's strikes
//! settle, the traffic weighs and answers each in turn, its cloak on the
//! hit between ([`Session::answer_landed`]), so each flag reads the
//! targets and goals the reactions before it left. While the traffic
//! doesn't answer (landed, or the system not yet set up) they are only
//! weighed. The NPCs' stray provocation eases once a pass, at
//! `_HandleShip`
//! ([`Traffic::ease_stray`](crate::traffic::Traffic::ease_stray), see the
//! `game_work` module).

use super::{Session, cloaks};
use crate::ai::Behaviour;
use crate::chance::Chance;
use crate::combat::Strike;
use crate::rulebook::RuleKey;

impl Session {
    /// Whether the traffic answers strikes now: not while landed, nor
    /// before the system is set up. The world runs through a jump, so a
    /// hit in it is answered.
    pub(super) fn answers_now(&self) -> bool {
        self.mode.landed().is_none() && !self.space.traffic_due
    }

    /// The fight's `strikes` as they settle, rolling on `chance`: while
    /// the traffic answers ([`Session::answers_now`]), it weighs each in
    /// turn, lets the ship it struck take it on its cloak
    /// (`cloaks::npc_takes_hit`, @0x3bb95-0x3bbdd and @0x3bf1f) and has
    /// the NPCs answer it as `behaviour` says, before the next
    /// ([`Traffic::answer_in_turn`](crate::traffic::Traffic::answer_in_turn)),
    /// as `_DamageShip` runs hit by hit; the persons' grudges are held
    /// after (`Session::step_traffic`). Otherwise it only weighs them
    /// ([`Session::weigh_strikes`]) and lets each ship's cloak take its
    /// hit. Whether it answered them.
    pub(super) fn answer_landed(
        &mut self,
        behaviour: &(impl Behaviour + ?Sized),
        strikes: &mut [Strike],
        chance: &mut dyn Chance,
    ) -> bool {
        if strikes.is_empty() {
            return true;
        }
        let reserves = self.setup.rules.source_for(RuleKey::CloakReserves);
        if !self.answers_now() {
            self.weigh_strikes(strikes, chance);
            self.space.cloak_hits(&self.statics, reserves, strikes);
            return false;
        }
        self.step_traffic(|traffic, world| {
            let arsenal = world.weapons;
            traffic.answer_in_turn(behaviour, &world, strikes, chance, &mut |npc, strike| {
                cloaks::npc_takes_hit(npc, arsenal, reserves, strike);
            });
        });
        true
    }

    /// Weighs the fight's `strikes` against the traffic in flight, rolling
    /// on `chance` ([`Traffic::weigh_strikes`](crate::traffic::Traffic::weigh_strikes)),
    /// for strikes the traffic doesn't answer now: no reaction runs
    /// between them.
    pub(super) fn weigh_strikes(&mut self, strikes: &mut [Strike], chance: &mut dyn Chance) {
        if !strikes.is_empty() {
            self.step_traffic(|traffic, world| traffic.weigh_strikes(&world, strikes, chance));
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ai::Goal;
    use crate::ai::NovaAi;
    use crate::catalog::{EscortClass, GovtId, GovtRecord, ShipId, StellarId, SystemId};

    use crate::cloak::{CLOAK_WHEN_HIT, CloakGear};
    use crate::combat::armament::Trigger;
    use crate::combat::fixture::strike;
    use crate::combat::{ShipRef, Strike};
    use crate::flight::Controls;
    use crate::handling::ShipFields;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::tests::{CAN_CLOAK, CLOAKED, FIRE, armed, facing_an_npc};
    use crate::session::{Mode, Session};
    use crate::testkit::work;
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
        assert_eq!(
            session.chart.system_record(&session.pilot, &session.setup),
            record
        );
    }

    /// Adds a police warship far off, idle: a Good Samaritan.
    fn samaritan(session: &mut Session) -> NpcId {
        let mut police = session.npcs()[0].clone();
        police.govt = Some(GovtId(141));
        police.ai_type = AiType::Warship;
        police.state.position.x += 3000.0;
        police.goal = Goal::Idle;
        session.space.traffic.add_npc(police)
    }

    /// A pass of `session`, its NPCs deciding and answering hits as
    /// Nova's do, in the pass a hit lands in.
    fn nova_pass(session: &mut Session) {
        crate::testkit::work_with(
            session,
            &crate::testkit::catalog(),
            Controls::default(),
            &NovaAi::default(),
            crate::combat::Rules::default(),
            &mut crate::chance::NeverFires,
        );
    }

    /// Fires one shot at NPC 0, the player's target, then lets the target
    /// go and works passes until the shot lands, the NPCs answering it in
    /// that pass.
    fn shot_after_letting_go(session: &mut Session) {
        session.chart.target = Some(NpcId(0));
        session.hold_trigger(FIRE);
        nova_pass(session);
        session.hold_trigger(Trigger::default());
        session.chart.target = None;
        for _ in 0..60 {
            if session.npcs()[0].reserves.shield.now < 30.0 {
                break;
            }
            nova_pass(session);
        }
        assert!(session.npcs()[0].reserves.shield.now < 30.0, "never hit");
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

    /// Makes NPC 0 the escort of a new idle lead of its government, of
    /// `ai_type`, 3000 off; then fires at NPC 0, the player's target, until
    /// the shot lands, the NPCs answering it in that pass. Gives the lead
    /// as it is then.
    fn lead_of_the_escort_shot(ai_type: AiType) -> crate::traffic::npc::Npc {
        let catalog = lawful();
        let mut session = facing_an_npc(&catalog, 180);
        let mut lead = session.npcs()[0].clone();
        lead.govt = Some(GovtId(140));
        lead.ai_type = ai_type;
        lead.goal = Goal::Idle;
        lead.state.position.x += 3000.0;
        let lead = session.space.traffic.add_npc(lead);
        session.space.traffic.npcs_mut()[0].leader = Some(lead);
        session.chart.target = Some(NpcId(0));
        session.hold_trigger(FIRE);
        for _ in 0..60 {
            nova_pass(&mut session);
            session.hold_trigger(Trigger::default());
            if session.npcs()[0].reserves.shield.now < 30.0 {
                break;
            }
        }
        assert!(session.npcs()[0].reserves.shield.now < 30.0, "never hit");
        session
            .npcs()
            .iter()
            .find(|npc| npc.id == lead)
            .expect("the lead")
            .clone()
    }

    #[test]
    fn the_warship_lead_of_an_escort_the_player_shoots_answers_as_a_good_samaritan() {
        // `_DamageShip`'s lead part (@0x3bd1b-0x3bd79) then the call
        // `_DoGoodSamaritan(victim, player)` (@0x3bed2-0x3bef4).
        let lead = lead_of_the_escort_shot(AiType::Warship);
        assert_eq!(lead.target, Some(ShipRef::Player));
        assert_eq!(lead.goal, Goal::Attack(ShipRef::Player));
        let trader = lead_of_the_escort_shot(AiType::WimpyTrader);
        assert_eq!(trader.target, Some(ShipRef::Player), "a trader lead");
        assert_ne!(
            trader.goal,
            Goal::Attack(ShipRef::Player),
            "a trader is no Samaritan"
        );
    }

    #[test]
    fn a_players_stray_hits_build_stray_provocation_that_eases_each_tick() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let mut ticks = 0;
        while session.npcs()[0].reserves.shield.now >= 30.0 {
            work(&mut session, &catalog, Controls::default());
            session.hold_trigger(Trigger::default());
            ticks += 1;
            assert!(ticks < 60, "never hit");
        }
        // 1.75 x the blaster's `Reload` of 2, less the tick's ease.
        assert_eq!(session.npcs()[0].stray, 3.0);
        // Each hit was weighed, and answered or not, as it settled.
        assert_eq!(session.npcs()[0].target, None, "forgiven: below 50");
        assert!(session.space.strikes.is_empty(), "none left to answer");
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.npcs()[0].stray, 2.5);
    }

    /// Settles the fight's `strikes` on `session` as a phase that landed
    /// them does, against no NPC's earlier state, by the default law.
    fn settle(session: &mut Session, strikes: Vec<Strike>) {
        let law = crate::combat::Rules::default().law;
        session.settle_strikes(
            &NovaAi::default(),
            (strikes, Vec::new()),
            &[],
            law,
            &mut crate::chance::NeverFires,
        );
    }

    /// The player's direct hit on NPC 0, unweighed, as the fight makes it.
    fn players_hit() -> Strike {
        Strike {
            answered: false,
            ..strike(ShipRef::Npc(NpcId(0)), ShipRef::Player)
        }
    }

    #[test]
    fn the_fights_strikes_are_answered_as_they_settle() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        settle(&mut session, vec![players_hit()]);
        assert_eq!(session.npcs()[0].target, Some(ShipRef::Player));
        assert_eq!(session.npcs()[0].goal, Goal::Attack(ShipRef::Player));
        assert!(session.space.strikes.is_empty(), "none kept");
    }

    #[test]
    fn landed_or_not_set_up_the_strikes_are_weighed_and_kept() {
        let catalog = armed();
        for landed in [true, false] {
            let mut session = facing_an_npc(&catalog, 180);
            if landed {
                session.mode = Mode::Landed(StellarId(128));
            } else {
                session.space.traffic_due = true;
            }
            let npc = session.npcs()[0].clone();
            settle(&mut session, vec![players_hit()]);
            assert_eq!(session.npcs()[0], npc, "landed {landed}: unanswered");
            let kept: Vec<bool> = session.space.strikes.iter().map(|s| s.answered).collect();
            assert_eq!(kept, [true], "landed {landed}: weighed and kept");
        }
    }

    #[test]
    fn an_empty_settle_answers_and_keeps_nothing() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let npcs = session.npcs().to_vec();
        settle(&mut session, Vec::new());
        assert_eq!(session.npcs(), npcs.as_slice());
        assert!(session.space.strikes.is_empty());
    }

    /// [`armed`] whose blaster (128, NPC 0's class's) bursts 15 shots and
    /// reloads 40 after.
    fn bursting() -> FakePilotCatalog {
        let mut catalog = armed();
        catalog.weapons[0].burst_count = 15;
        catalog.weapons[0].burst_reload = 40;
        catalog
    }

    #[test]
    fn a_hit_answered_as_it_settles_still_cloaks_a_cloak_when_hit_ship_and_starts_its_burst_reload()
    {
        // `_DamageShip` cloaks the ship (@0x3bb95-0x3bbdd) between its
        // flag and its reaction: the ship, idle as the hit lands, is not
        // yet attacking the player, so it cloaks.
        let catalog = bursting();
        let mut session = facing_an_npc(&catalog, 180);
        session.space.traffic.npcs_mut()[0].cloak_gear = CloakGear {
            flags2: CLOAK_WHEN_HIT,
            ..CAN_CLOAK
        };
        settle(&mut session, vec![players_hit()]);
        let npc = &session.npcs()[0];
        assert_eq!(npc.goal, Goal::Attack(ShipRef::Player), "answered");
        assert_eq!(npc.cloak.direction, 1, "cloaking");
        assert_eq!(npc.armament.mounts()[0].reload, 40.0);
    }

    #[test]
    fn a_blast_answered_as_it_settles_leaves_a_decloak_when_hit_ships_cloak_alone() {
        // A blast is no direct hit: `_DamageShip` skips its decloak
        // (@0x3bf1f) as it skips its flag (@0x3b44d).
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let npc = &mut session.space.traffic.npcs_mut()[0];
        npc.cloak_gear = CloakGear {
            decloaks_when_hit: true,
            ..CAN_CLOAK
        };
        npc.cloak = CLOAKED;
        let blast = Strike {
            direct: false,
            ..players_hit()
        };
        settle(&mut session, vec![blast]);
        assert_eq!(session.npcs()[0].cloak, CLOAKED, "the blast");
        settle(&mut session, vec![players_hit()]);
        assert_eq!(session.npcs()[0].cloak.direction, -1, "a direct hit");
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
        session.chart.target = Some(NpcId(0));
        assert_eq!(session.player_side().target, Some(ShipRef::Npc(NpcId(0))));
        let class = |session: &mut Session, escort_type: i16, inherent_ai: i16, mass: i16| {
            let mut record = ship(session.pilot.ship.0, ShipFields { mass, ..FAST });
            record.escort_type = escort_type;
            record.inherent_ai = inherent_ai;
            session.statics_mut().ships = vec![record];
            session.player_side().class
        };
        assert_eq!(class(&mut session, 0, 1, 500), EscortClass::Fighter);
        assert_eq!(class(&mut session, -1, 3, 49), EscortClass::Fighter);
        assert_eq!(class(&mut session, 2, 3, 49), EscortClass::Warship);
        session.statics_mut().ships = vec![ship(129, FAST)];
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
            session.statics_mut().ships = vec![record];
            assert_eq!(session.player_side().hull.crew, crew);
        }
        session.statics_mut().ships = vec![ship(129, FAST)];
        assert_eq!(session.player_side().hull.crew, 0, "no record of its own");
    }
}
