//! Tests of the order the fight's phases run in through a pass
//! (`Session::pass_after_move`), where the fight's own tests, which run
//! one phase at a time, can't see it: the player's point defence comes
//! before its trigger (`_HandlePlayer` @0x68c74, @0x6ab55), and both
//! before the collision passes (@0x43e70-0x44015); every NPC's point
//! defence comes after them (`_AIDispatch` @0x4429d) and before the ship
//! loop's picks (`_HandleShip` @0x44483), so a missile an NPC fires is
//! met first in the next pass, and one the player fires in the same pass;
//! and the collision passes come before `_HandleShot` (@0x44385), which
//! flies what they released.

use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::{WeaponId, WeaponRecord};
use crate::chance::{Chance, NeverFires};
use crate::combat::armament::{Armament, Trigger};
use crate::combat::hull::Condition;
use crate::combat::weapon::WeaponSpec;
use crate::combat::{CombatEvent, Rules, ShipRef};
use crate::flight::Controls;
use crate::rulebook::Rulebook;
use crate::session::Session;
use crate::session::fighters::tests::{armed_npc, carrying, flying_by, game_pass, pilot};
use crate::session::tests::{
    Attacking, arming, blaster, facing_an_npc, homing_missile, with_an_npc_at,
};
use crate::testkit::{FakePilotCatalog, weapon};
use crate::traffic::npc::{Npc, NpcId};

/// The Quad Light Blaster Turret's point defence.
const QUAD: WeaponId = WeaponId(133);
/// A homing missile.
const MISSILE: WeaponId = WeaponId(134);

/// The Quad Light Blaster Turret: point defence firing every 5 ticks, 20
/// pixels a tick for 12, doing 1 mass and 4 energy damage.
fn quad() -> WeaponRecord {
    WeaponRecord {
        id: QUAD,
        guidance: 9,
        reload: 5,
        count: 12,
        speed: 2000,
        mass_dmg: 1,
        energy_dmg: 4,
        ..blaster()
    }
}

/// A homing missile 5 pixels a tick for 200, fired once.
fn slow_missile() -> WeaponRecord {
    WeaponRecord {
        speed: 500,
        count: 200,
        ..homing_missile(MISSILE.0, 1000)
    }
}

/// Every NPC idles, its point defence on or off.
#[derive(Debug)]
struct Defending(bool);

impl Behaviour for Defending {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }

    fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
        Trigger {
            point_defence: self.0,
            ..Trigger::default()
        }
    }
}

/// An advance pass of `session` in flight, by Nova's rules, its NPCs
/// deciding as `behaviour` says: the fight's events.
fn pass(
    session: &mut Session,
    catalog: &FakePilotCatalog,
    behaviour: &impl Behaviour,
) -> Vec<CombatEvent> {
    let rescued = session.game_work(
        Controls::default(),
        catalog,
        behaviour,
        Rules::default(),
        &mut NeverFires,
    );
    assert!(!rescued);
    session.take_combat_events()
}

/// Whether `ship` fired `weapon` among `events`.
fn fired(events: &[CombatEvent], ship: ShipRef, weapon: WeaponId) -> bool {
    events.iter().any(|event| {
        matches!(event, CombatEvent::Fired { ship: s, weapon: w, .. } if *s == ship && *w == weapon)
    })
}

#[test]
fn the_players_point_defence_first_meets_an_npcs_missile_on_the_pass_after_it_is_fired() {
    // The NPC's pick in the ship loop comes after the player's point
    // defence.
    let catalog = arming(quad(), slow_missile(), 0);
    let mut session = with_an_npc_at(&catalog, 750, 550, 180);
    let events = pass(&mut session, &catalog, &Attacking);
    assert!(
        fired(&events, ShipRef::Npc(NpcId(0)), MISSILE),
        "{events:?}"
    );
    assert!(!fired(&events, ShipRef::Player, QUAD), "not yet");
    let events = pass(&mut session, &catalog, &Attacking);
    assert!(fired(&events, ShipRef::Player, QUAD), "{events:?}");
}

#[test]
fn the_players_point_defence_fires_before_its_trigger_and_launches_before_its_recoil() {
    // Its point defence reads the ship before `_FirePlayerWeapon`'s
    // recoil (@0x62ea2-0x62f35) pushes it, and a point-defence shot
    // carries its firer's velocity (`_SpawnShot`).
    let kicking = WeaponRecord {
        recoil: 100,
        reload: 10,
        ..blaster()
    };
    let defended = |held: bool| {
        let catalog = arming(quad(), slow_missile(), 0);
        let mut session = with_an_npc_at(&catalog, 750, 550, 180);
        // The player's class, of no mass in the fixture, given one.
        session.ship.fit.hull.mass = 40.0;
        session.ship.fit.armament = Armament::new([
            (WeaponSpec::new(&quad()), 1),
            (WeaponSpec::new(&kicking), 1),
        ]);
        pass(&mut session, &catalog, &Attacking);
        session.hold_fire(held, false);
        let events = pass(&mut session, &catalog, &Attacking);
        let mine: Vec<WeaponId> = events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Fired {
                    ship: ShipRef::Player,
                    weapon,
                    ..
                } => Some(weapon),
                _ => None,
            })
            .collect();
        let launched = session
            .shots()
            .iter()
            .find(|shot| shot.weapon.id == QUAD)
            .expect("point defence flies")
            .velocity;
        (mine, launched, session.player().velocity)
    };
    let (mine, launched, pushed) = defended(true);
    assert_eq!(mine, [QUAD, kicking.id], "point defence first");
    let (_, unpushed, still) = defended(false);
    assert_ne!(pushed, still, "the recoil pushed it");
    assert_eq!(launched, unpushed, "point defence launched before it");
}

/// Every NPC idles; `defender` runs its point defence, and `picker`
/// fires its missile at `defender`.
#[derive(Debug)]
struct Picking {
    defender: NpcId,
    picker: NpcId,
}

impl Behaviour for Picking {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
        Trigger {
            point_defence: npc.id == self.defender,
            only: (npc.id == self.picker).then_some(MISSILE),
            ..Trigger::default()
        }
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        (npc.id == self.picker).then_some(ShipRef::Npc(self.defender))
    }
}

#[test]
fn an_npcs_point_defence_first_meets_an_npcs_missile_on_the_pass_after_it_is_fired() {
    // `_AIDispatch` runs every NPC's point defence before `_HandleShip`
    // fires any NPC's pick, whichever slot is lower.
    let missile = WeaponRecord {
        guidance: 1,
        speed: 500,
        count: 100,
        guided_turn: 70,
        durability: 4,
        mass_dmg: 20,
        energy_dmg: 10,
        ..weapon(MISSILE.0)
    };
    for picker_slot in [1, 3] {
        let catalog = carrying();
        let mut session = flying_by(
            &catalog,
            pilot(&catalog, 0, Vec::new()),
            Rulebook::default(),
        );
        let picker = session
            .space
            .traffic
            .add_npc_at(armed_npc(1000.0, 850.0, missile), picker_slot);
        let defender = session
            .space
            .traffic
            .add_npc_at(armed_npc(1000.0, 1000.0, quad()), 2);
        let script = Picking { defender, picker };
        session.take_combat_events();
        game_pass(&mut session, &catalog, &script);
        let events = session.take_combat_events();
        assert!(fired(&events, ShipRef::Npc(picker), MISSILE), "{events:?}");
        assert!(
            !fired(&events, ShipRef::Npc(defender), QUAD),
            "{picker_slot}: not yet"
        );
        game_pass(&mut session, &catalog, &script);
        let events = session.take_combat_events();
        assert!(
            fired(&events, ShipRef::Npc(defender), QUAD),
            "{picker_slot}: {events:?}"
        );
        let in_flight = session
            .shots()
            .iter()
            .filter(|shot| shot.firer == ShipRef::Npc(defender) && shot.weapon.id == QUAD)
            .count();
        assert_eq!(in_flight, 1, "{picker_slot}");
    }
}

#[test]
fn an_npcs_point_defence_meets_the_players_missile_on_the_pass_it_is_fired() {
    // The player's trigger comes before `_AIDispatch`.
    let catalog = arming(slow_missile(), quad(), 0);
    let mut session = with_an_npc_at(&catalog, 750, 550, 180);
    session.chart.target = Some(NpcId(0));
    session.hold_fire(true, false);
    let events = pass(&mut session, &catalog, &Defending(true));
    assert!(fired(&events, ShipRef::Player, MISSILE), "{events:?}");
    assert!(fired(&events, ShipRef::Npc(NpcId(0)), QUAD), "{events:?}");
}

#[test]
fn the_player_fires_in_the_pass_a_shot_downs_it() {
    // Its trigger comes before the collision passes that down it.
    let harmless = WeaponRecord {
        reload: 0,
        mass_dmg: 0,
        energy_dmg: 0,
        ..WeaponRecord {
            id: WeaponId(140),
            ..blaster()
        }
    };
    let catalog = arming(harmless, blaster(), 0);
    let mut session = facing_an_npc(&catalog, 180);
    // Intact, a blaster's hit from breaking up.
    session.pilot.reserves.shield.now = 0.0;
    session.pilot.reserves.armor.max = 3.0;
    session.pilot.reserves.armor.now = 3.0;
    session.hold_fire(true, false);
    for _ in 0..20 {
        let events = pass(&mut session, &catalog, &Attacking);
        if !matches!(session.player_condition(), Condition::Dying { .. }) {
            continue;
        }
        let fire = events.iter().position(|event| {
            matches!(
                event,
                CombatEvent::Fired {
                    ship: ShipRef::Player,
                    ..
                }
            )
        });
        let hit = events.iter().position(|event| {
            matches!(
                event,
                CombatEvent::Hit {
                    ship: ShipRef::Player,
                    ..
                }
            )
        });
        assert!(hit.is_some(), "downed by a shot: {events:?}");
        assert!(fire.is_some() && fire < hit, "fired first: {events:?}");
        return;
    }
    panic!("never downed");
}

#[test]
fn a_sub_munition_a_proximity_hit_releases_first_flies_in_the_pass_it_is_released() {
    // A shell 10 pixels a tick, fused 20 pixels out, releasing 2 of weapon
    // 148 only on a hit (`Flags2` 0x0020).
    let shell = WeaponRecord {
        reload: 1000,
        count: 30,
        speed: 1000,
        prox_radius: 20,
        energy_dmg: 1,
        sub_count: 2,
        sub_type: Some(WeaponId(148)),
        flags2: 0x0020,
        ..weapon(182)
    };
    let sub = WeaponRecord {
        count: 50,
        speed: 500,
        ..weapon(148)
    };
    let catalog = arming(shell, sub, 0);
    let mut session = facing_an_npc(&catalog, 180);
    session.hold_fire(true, false);
    pass(&mut session, &catalog, &Defending(false));
    session.hold_fire(false, false);
    for _ in 0..20 {
        pass(&mut session, &catalog, &Defending(false));
        let subs: Vec<_> = session
            .shots()
            .iter()
            .filter(|shot| shot.weapon.id == WeaponId(148))
            .collect();
        if subs.is_empty() {
            continue;
        }
        assert_eq!(subs.len(), 2);
        for sub in subs {
            assert_eq!(sub.age, 1, "flown once: {sub:?}");
            assert!(sub.moved_from.is_some(), "{sub:?}");
        }
        return;
    }
    panic!("never released");
}
