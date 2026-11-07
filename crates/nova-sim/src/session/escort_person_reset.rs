//! A person escort as the fleet respawns, by
//! [`RuleKey::PersonEscortRespawn`].
//!
//! The original's `_RespawnEscort` (@0x3d4b8-0x3d62c), which
//! `_SetupShipsInSystem` calls for each escort (@0x42c79) at a take-off,
//! a pilot loaded, a pod rescue, a gate's exit and a jump's arrival, is a
//! general slot reset: among about twenty per-spawn fields it writes -1 to
//! the escort's person (+0xc8d0 @0x3d531), and nothing writes it back. So
//! by the engine's reading, opt-in, each respawn forgets the escort's
//! person ([`forget_escort_persons`], run first in
//! `Session::populate_as`, before the system's people are drawn as
//! `_SpawnPerson` draws them after the reset, @0x431fd, @0x433af). The
//! escort then flies on as its class, which the rest of the entry already
//! does for an escort of no person:
//!
//! - named by its class (`_DrawStatusTarg` @0x4b6ed), unpainted
//!   (`_GetShipPaintColor` @0x7d2f), with its class's stats;
//! - its shield and armour capped by its class
//!   (`_ShipShieldCapacity`/`_ShipArmorCapacity` @0x2a55, @0x2c80) but
//!   kept as they were at a jump or gate, even above those caps, as
//!   `_HandleShip` recharges only below the cap (@0x345a6-0x34672), and
//!   refilled to the class's at a take-off or load;
//! - its weapons kept through a jump or gate and restocked to its class's
//!   stock at a take-off or load (@0x3d5a7-0x3d600);
//! - the person no longer in the fleet, so it may be drawn again, in this
//!   very system as a second copy (the original keeps no person on an
//!   escort's record or in its save; only përs +0x620 stops a person
//!   appearing, @0x4099d, @0x43340, and no join or respawn clears it);
//! - its death leaves the person alive (`_HandleShipDisplay`
//!   @0x2dc4a-0x2dca1) and the player's hits give it no grudge
//!   (`_DamageShip` @0x3be92).
//!
//! By the other reading, the default, a person escort flies as itself in
//! every system (see the `escorts` module).

use super::{Setup, Statics};
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource};

/// Under [`RuleKey::PersonEscortRespawn`]'s engine reading, each escort
/// record forgets its person as the fleet respawns (`_RespawnEscort`
/// +0xc8d0 = -1 @0x3d531), its shield and armour capped by its class's,
/// what it holds kept (`_HandleShip` recharges only below the cap,
/// @0x345a6-0x34672). A record whose ship has no record only forgets its
/// person. By the other reading nothing changes.
pub(super) fn forget_escort_persons(pilot: &mut Pilot, statics: &Statics, setup: &Setup) {
    if setup.rules.source_for(RuleKey::PersonEscortRespawn) != RuleSource::Engine {
        return;
    }
    for escort in pilot
        .escorts
        .iter_mut()
        .filter(|escort| escort.person.is_some())
    {
        escort.person = None;
        let Some(record) = statics.ship_record(escort.ship) else {
            continue;
        };
        let full = setup.ship_kind(statics, record).stats.full();
        escort.reserves.shield.max = full.shield.max;
        escort.reserves.armor.max = full.armor.max;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::rc::Rc;

    use super::super::Session;
    use super::super::escorts::tests::{
        TURRET, WARSHIP, escort_npcs, flying_ace, tick, with_ace, with_ace_escort,
    };
    use super::super::gate_follow::tests::{exit_gate_raising, with_gates};
    use super::forget_escort_persons;
    use crate::catalog::{PersonId, ShipId, SystemId, SystemTraffic};
    use crate::chance::{Chance, NeverFires};
    use crate::flight::Controls;
    use crate::gate::GateKind;
    use crate::person::{GRUDGE, PersonRoll, PersonRules};
    use crate::pilot::Escort;
    use crate::reserves::{Gauge, Reserves};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, FakePilotCatalog, jump, land_now, work};
    use crate::traffic::npc::Npc;

    /// The rules with the original's respawn reset.
    fn opt_in() -> Rulebook {
        Rulebook::default().with_override(RuleKey::PersonEscortRespawn, RuleSource::Engine)
    }

    /// [`with_ace`], 129 out of the way, with the gates of
    /// [`with_gates`], Ace painted and holding grudges (`Flags` 0x0001).
    fn catalog() -> FakePilotCatalog {
        let mut catalog = with_gates(crate::testkit::far_129_reachable(with_ace()));
        catalog.persons[0].color = 0x00F8_1008;
        catalog.persons[0].flags |= GRUDGE;
        catalog
    }

    /// Ace flying as the player's escort, a person escort by the default
    /// reading, then under `rules`.
    fn flown(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        flying_ace(catalog).with_rules(rules)
    }

    /// The ways the fleet respawns that keep the session in flight.
    #[derive(Clone, Copy, Debug)]
    enum Respawn {
        Jump,
        Gate,
        TakeOff,
    }

    fn respawn(session: &mut Session, catalog: &FakePilotCatalog, way: Respawn) {
        match way {
            Respawn::Jump => {
                assert_eq!(jump(session, catalog, 131), Some(SystemId(131)));
            }
            Respawn::Gate => {
                exit_gate_raising(session, catalog, GateKind::Hypergate, &mut NeverFires);
            }
            Respawn::TakeOff => {
                land_now(session).expect("lands on 128, under the ship");
                session
                    .take_off(catalog, &mut NeverFires)
                    .expect("takes off");
                tick(session, catalog);
            }
        }
    }

    fn ace(session: &Session) -> &Npc {
        escort_npcs(session)[0]
    }

    fn mounts(npc: &Npc) -> Vec<(crate::catalog::WeaponId, u32)> {
        npc.armament
            .mounts()
            .iter()
            .map(|mount| (mount.spec.id, mount.count))
            .collect()
    }

    /// Sets Ace's shield to 50, on its record too.
    fn shield_at_50(session: &mut Session) {
        let id = session.space.escorts.fleet[0].expect("placed");
        session
            .space
            .npc_mut(id)
            .expect("there")
            .reserves
            .shield
            .now = 50.0;
        session.space.sync_fleet(&mut session.pilot);
    }

    #[test]
    fn by_default_a_person_escort_keeps_its_person_through_a_jump_a_gate_and_a_take_off() {
        let catalog = catalog();
        for way in [Respawn::Jump, Respawn::Gate, Respawn::TakeOff] {
            let mut session = flown(&catalog, Rulebook::default());
            respawn(&mut session, &catalog, way);
            let ace = ace(&session);
            assert_eq!(
                ace.person.map(|person| person.id),
                Some(PersonId(700)),
                "{way:?}"
            );
            assert_eq!(session.npc_name(ace), Some("Ace"), "{way:?}");
            assert_eq!(ace.stats.shield, 2.0 * f32::from(FAST.shield), "{way:?}");
            assert_eq!(ace.stats.armor, 2.0 * f32::from(FAST.armor), "{way:?}");
            assert_eq!(mounts(ace), [(TURRET, 3)], "{way:?}");
            assert!(
                session.space.npc_paint(ace, &session.statics).is_some(),
                "{way:?}: painted"
            );
            assert_eq!(
                session.pilot().escorts()[0].person,
                Some(PersonId(700)),
                "{way:?}"
            );
        }
    }

    /// The checks a jump's arrival and a gate's exit share by the
    /// original: Ace forgotten, flying as its class, holding what it held.
    fn forgotten_holding_its_own(session: &Session, way: Respawn) {
        let ace = ace(session);
        assert_eq!(ace.person, None, "{way:?}");
        assert_eq!(session.pilot().escorts()[0].person, None, "{way:?}");
        assert_eq!(session.npc_name(ace), Some("Ship 150"), "{way:?}");
        assert_eq!(ace.stats, ShipStats::new(FAST, &[]), "{way:?}");
        assert_eq!(
            ace.reserves.shield,
            Gauge {
                now: 50.0,
                max: 30.0
            },
            "{way:?}: kept above its class's cap"
        );
        assert_eq!(ace.reserves.armor.max, 45.0, "{way:?}");
        assert_eq!(mounts(ace), [(TURRET, 3)], "{way:?}: its weapons kept");
        assert_eq!(
            session.space.npc_paint(ace, &session.statics),
            None,
            "{way:?}"
        );
    }

    #[test]
    fn by_the_original_a_person_escort_loses_its_person_at_a_jumps_arrival() {
        let catalog = catalog();
        let mut session = flown(&catalog, opt_in());
        shield_at_50(&mut session);
        respawn(&mut session, &catalog, Respawn::Jump);
        forgotten_holding_its_own(&session, Respawn::Jump);
    }

    #[test]
    fn by_the_original_a_person_escort_loses_its_person_at_a_gate_exit() {
        let catalog = catalog();
        let mut session = flown(&catalog, opt_in());
        shield_at_50(&mut session);
        respawn(&mut session, &catalog, Respawn::Gate);
        forgotten_holding_its_own(&session, Respawn::Gate);
    }

    #[test]
    fn by_the_original_a_person_escort_loses_its_person_at_a_take_off() {
        let catalog = catalog();
        let mut session = flown(&catalog, opt_in());
        shield_at_50(&mut session);
        respawn(&mut session, &catalog, Respawn::TakeOff);
        let ace = ace(&session);
        assert_eq!(ace.person, None);
        assert_eq!(session.pilot().escorts()[0].person, None);
        assert_eq!(session.npc_name(ace), Some("Ship 150"));
        assert_eq!(ace.stats, ShipStats::new(FAST, &[]));
        assert_eq!(
            ace.reserves,
            Reserves {
                fuel: ace.reserves.fuel,
                ..Reserves::full(30.0, 45.0, 300.0)
            },
            "full to its class"
        );
        assert_eq!(ace.reserves.fuel.max, 300.0);
        assert_eq!(mounts(ace), [(TURRET, 1)], "its class's stock");
    }

    #[test]
    fn by_the_original_a_pilot_flown_with_a_person_escort_flies_it_as_its_class() {
        let catalog = catalog();
        let mut session = Session::fly(&catalog, with_ace_escort(&catalog))
            .expect("flies")
            .with_rules(opt_in());
        tick(&mut session, &catalog);
        let ace = ace(&session);
        assert_eq!(ace.person, None);
        assert_eq!(ace.stats, ShipStats::new(FAST, &[]));
        assert_eq!(
            ace.reserves,
            Reserves {
                fuel: super::super::escorts::tests::worn().fuel,
                ..Reserves::full(30.0, 45.0, 300.0)
            },
            "full to its class, the fuel as recorded"
        );
        assert_eq!(mounts(ace), [(TURRET, 1)]);
        assert_eq!(session.pilot().escorts()[0].person, None);
    }

    #[test]
    fn by_the_original_a_reset_person_escort_destroyed_leaves_its_person_alive() {
        let catalog = catalog();
        let mut session = flown(&catalog, opt_in());
        respawn(&mut session, &catalog, Respawn::Jump);
        let id = session.space.escorts.fleet[0].expect("placed");
        session.space.npc_mut(id).expect("there").reserves.armor.now = 0.0;
        for _ in 0..4 {
            work(&mut session, &catalog, Controls::default());
        }
        assert_eq!(session.pilot().escorts(), []);
        assert!(!session.pilot().gone(PersonId(700)));
    }

    #[test]
    fn by_the_original_a_reset_person_escort_hit_holds_no_grudge() {
        let catalog = catalog();
        for (rules, grudges) in [
            (opt_in(), vec![]),
            (Rulebook::default(), vec![PersonId(700)]),
        ] {
            let mut session = flown(&catalog, rules);
            respawn(&mut session, &catalog, Respawn::Jump);
            let id = session.space.escorts.fleet[0].expect("placed");
            session.space.hold_grudges(&mut session.pilot, &[id]);
            assert_eq!(
                session.pilot.grudges.iter().copied().collect::<Vec<_>>(),
                grudges
            );
        }
    }

    /// Brings the first person who may appear, and no slot's.
    #[derive(Debug)]
    struct FirstComes;

    impl PersonRules for FirstComes {
        fn roll(&self, eligible: &[PersonId], _chance: &mut dyn Chance) -> PersonRoll {
            eligible
                .first()
                .map_or(PersonRoll::Empty, |&id| PersonRoll::Person(id))
        }

        fn listed(&self, _prob: i16, _chance: &mut dyn Chance) -> bool {
            true
        }

        fn link_slip(&self) -> RuleSource {
            RuleSource::Engine
        }

        fn shield_mod(&self) -> RuleSource {
            RuleSource::Engine
        }

        fn derelict_setup_speed(&self) -> RuleSource {
            RuleSource::Engine
        }
    }

    #[test]
    fn by_the_original_a_reset_person_may_appear_again_beside_its_former_ship() {
        // Ace in 131's first person slot, always listed.
        let mut catalog = catalog();
        catalog.persons[0].link_syst = 0;
        let mut persons = [(None, 0); 8];
        persons[0] = (Some(PersonId(700)), 100);
        catalog
            .traffic
            .retain(|(system, _)| *system != SystemId(131));
        catalog.traffic.push((
            SystemId(131),
            SystemTraffic {
                dude_types: [(-1, 0); 8],
                avg_ships: 0,
                persons,
            },
        ));
        for (rules, others) in [(opt_in(), 1), (Rulebook::default(), 0)] {
            let mut session = flown(&catalog, rules).with_person_rules(Rc::new(FirstComes));
            respawn(&mut session, &catalog, Respawn::Jump);
            let escort = session.space.escorts.fleet[0].expect("placed");
            let aces = session
                .npcs()
                .iter()
                .filter(|npc| npc.id != escort)
                .filter(|npc| npc.person.is_some_and(|person| person.id == PersonId(700)))
                .count();
            assert_eq!(
                aces,
                others,
                "{:?}",
                session.rules().source_for(RuleKey::PersonEscortRespawn)
            );
        }
    }

    #[test]
    fn forgetting_leaves_an_escort_of_no_person_and_by_default_every_escort_alone() {
        let catalog = catalog();
        let mut session = flying_ace(&catalog);
        let hired = Escort {
            person: None,
            ..session.pilot.escorts[0]
        };
        let classless = Escort {
            ship: ShipId(999),
            ..session.pilot.escorts[0]
        };
        session.pilot.escorts = vec![session.pilot.escorts[0], hired, classless];
        let before = session.pilot.escorts.clone();
        forget_escort_persons(&mut session.pilot, &session.statics, &session.setup);
        assert_eq!(session.pilot.escorts, before, "by default");
        let mut session = session.with_rules(opt_in());
        forget_escort_persons(&mut session.pilot, &session.statics, &session.setup);
        let after = &session.pilot.escorts;
        assert_eq!(after[0].person, None);
        assert_eq!(after[0].ship, WARSHIP);
        assert_eq!(
            after[0].reserves.shield,
            Gauge {
                now: before[0].reserves.shield.now,
                max: 30.0
            }
        );
        assert_eq!(after[0].reserves.armor.max, 45.0);
        assert_eq!(after[0].reserves.fuel, before[0].reserves.fuel);
        assert_eq!(after[1], before[1], "no person, untouched");
        assert_eq!(
            after[2],
            Escort {
                person: None,
                ..before[2]
            },
            "no class: its person forgotten alone"
        );
    }
}
