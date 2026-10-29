//! A mission's special ships of `ShipBehav` 2 attack the stellars of their
//! enemies (stellar-fidelity phase 2, merging mission-ships-fidelity phase
//! 29): `_AIDispatch` sends them to `_DeathStarAI` (@0x8ff3a-0x8ff43; see
//! [`death_star`](crate::ai::death_star)), which flies them to the first
//! enemy stellar shots can shoot down and fires their planet gun at it
//! (see [`stellar_attack`](crate::traffic::stellar_attack)), and their
//! shots shoot it down as the player's do (see the `stellars` module).

#[cfg(test)]
mod tests {
    use crate::ai::{Goal, NovaAi};
    use crate::catalog::{
        GovtId, GovtRecord, HullRecord, MissionShip, StellarId, StellarLife, StockWeapon, WeaponId,
        WeaponRecord,
    };
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::combat::weapon::PLANET_TYPE;
    use crate::control::{Bit, Script};
    use crate::flight::Controls;
    use crate::govt::XENOPHOBIC;
    use crate::mission::FailedBy;
    use crate::session::mission_ships::tests::{in_flight, shipping, ships_catalog};
    use crate::session::{Session, SessionEvent};
    use crate::testkit::{FakePilotCatalog, govt, hull, life, weapon, work_with};
    use crate::traffic::npc::NpcId;

    /// A gun of `flags2`, `wëap` 130: 10 pixels a tick for 40 ticks, every
    /// 10 ticks, of 30 mass and 20 energy damage.
    fn gun(flags2: u16) -> WeaponRecord {
        WeaponRecord {
            guidance: -1,
            count: 40,
            speed: 1000,
            reload: 10,
            mass_dmg: 30,
            energy_dmg: 20,
            flags2,
            ..weapon(130)
        }
    }

    /// Mission 140's two ships of `ShipBehav` 2 (warships of govt 131,
    /// ship 200, carrying `gun` in stock) in system 130, whose planet 129,
    /// at (2000, 0), belongs to the xenophobic govt 132, of `Strength` 100,
    /// running `^b5` as it is shot down.
    fn attacking(gun: WeaponRecord) -> FakePilotCatalog {
        let mut catalog = ships_catalog(MissionShip {
            behav: 2,
            ..shipping(-1, 0)
        });
        catalog.govts = vec![GovtRecord {
            flags: XENOPHOBIC,
            ..govt(132)
        }];
        catalog.sites[0].1[1].govt = Some(GovtId(132));
        catalog.lives = vec![StellarLife {
            on_destroy: Script::parse("^b5"),
            ..life(129, 2)
        }];
        catalog.weapons = vec![gun];
        catalog.hulls = vec![HullRecord {
            weapons: vec![StockWeapon {
                weapon: WeaponId(130),
                count: 1,
                ammo: 0,
            }],
            ..hull(200)
        }];
        catalog
    }

    /// One whole pass ([`Session::game_work`]), the traffic by Nova's AI.
    /// The session events it raised.
    fn full_tick(session: &mut Session, catalog: &FakePilotCatalog) -> Vec<SessionEvent> {
        work_with(
            session,
            catalog,
            Controls::default(),
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        session.take_events()
    }

    /// Whether 129 is destroyed.
    fn destroyed(session: &Session) -> bool {
        session
            .galaxy()
            .overlay()
            .stellar(StellarId(129))
            .destroyed
            .is_some()
    }

    /// The marked NPCs' goals.
    fn goals(session: &Session) -> Vec<Goal> {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.mission.is_some())
            .map(|npc| npc.goal)
            .collect()
    }

    /// The player's legal records.
    fn records(session: &Session) -> Vec<i16> {
        [131, 132]
            .map(|govt| session.pilot().legal_record(GovtId(govt)))
            .into()
    }

    #[test]
    fn a_ship_behav_2_ship_flies_to_an_enemy_stellar_and_shoots_it_down_once() {
        let catalog = attacking(gun(PLANET_TYPE));
        let mut session = in_flight(&catalog);
        let before = records(&session);
        let mut events = Vec::new();
        let mut ticks = 0;
        while !destroyed(&session) && ticks < 3000 {
            events.extend(full_tick(&mut session, &catalog));
            ticks += 1;
            if ticks == 1 {
                assert_eq!(goals(&session), [Goal::AttackStellar(StellarId(129)); 2]);
            }
        }
        assert!(destroyed(&session), "shot down within {ticks} ticks");
        assert!(events.contains(&SessionEvent::GalaxyChanged), "{events:?}");
        assert!(
            session.control_bit(Bit::new(5).expect("a bit")),
            "its OnDestroy"
        );
        for _ in 0..60 {
            full_tick(&mut session, &catalog);
        }
        assert!(
            session.control_bit(Bit::new(5).expect("a bit")),
            "once: a toggle run again clears it"
        );
        assert_eq!(records(&session), before, "no one is convicted");
    }

    #[test]
    fn a_ship_behav_2_ship_with_no_planet_gun_never_fires_at_the_stellar() {
        let catalog = attacking(gun(0));
        let mut session = in_flight(&catalog);
        for _ in 0..600 {
            full_tick(&mut session, &catalog);
            assert!(
                goals(&session)
                    .iter()
                    .all(|goal| !matches!(goal, Goal::AttackStellar(_))),
                "{:?}",
                goals(&session)
            );
        }
        assert!(!destroyed(&session));
        assert_eq!(session.space.strength.get(&StellarId(129)), None, "whole");
    }

    #[test]
    fn a_captured_ship_behav_2_mission_escort_attacks_the_stellar_too() {
        let catalog = attacking(gun(PLANET_TYPE));
        let mut session = in_flight(&catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.failed = Some(FailedBy::Quick);
        }
        let id: NpcId = session
            .npcs()
            .iter()
            .find(|npc| npc.mission.is_some())
            .expect("a mission ship")
            .id;
        session.join_fleet(id, &mut NeverFires);
        assert!(
            session
                .space
                .npc(id)
                .is_some_and(crate::traffic::npc::Npc::mission_escort)
        );
        full_tick(&mut session, &catalog);
        assert_eq!(
            session.space.npc(id).map(|npc| npc.goal),
            Some(Goal::AttackStellar(StellarId(129)))
        );
    }
}
