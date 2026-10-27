//! The session's tests of the order in which the player's point defence
//! and trigger take their turret exits: `_HandlePlayer` runs
//! `_HandleShipPointDefense` (@0x68c74) before `_FirePlayerWeapon`
//! (@0x6ab55), and both step the one turret exits' cursor
//! (`_ModifyShotStartPosition`), so point defence takes its exits first.

use super::tests::{Attacking, arming, homing_missile, with_an_npc_at};
use super::*;
use crate::catalog::{ShipExits, StockWeapon, WeaponRecord};
use crate::chance::NeverFires;
use crate::combat::Rules;
use crate::combat::armament::MOD_WEAPON;
use crate::flight::Controls;
use crate::rulebook::{RuleSource, Rulebook};
use crate::testkit::FakePilotCatalog;
use crate::testkit::work_with;
use nova_sim_catalog::fixture::{outfit, weapon};

/// A point-defence turret firing from its ship's turret exits.
fn exit_defence() -> WeaponRecord {
    WeaponRecord {
        id: WeaponId(133),
        guidance: 9,
        reload: 5,
        count: 12,
        speed: 2000,
        mass_dmg: 1,
        energy_dmg: 4,
        exit_type: 1,
        ..weapon(133)
    }
}

/// A gun of no reload firing from its ship's turret exits too.
fn turret_gun() -> WeaponRecord {
    WeaponRecord {
        reload: 0,
        count: 30,
        speed: 1000,
        exit_type: 1,
        ..weapon(128)
    }
}

/// The player carrying [`exit_defence`] and [`turret_gun`], its turret
/// exits 10, 20, 30 and 40 pixels to its right, and its traffic a slow
/// homing missile.
fn defended_player() -> FakePilotCatalog {
    let missile = WeaponRecord {
        speed: 500,
        count: 200,
        ..homing_missile(134, 1000)
    };
    let mut catalog = arming(turret_gun(), missile, 0);
    catalog.weapons.push(exit_defence());
    catalog.outfits.push(outfit(251, &[(MOD_WEAPON, 133)]));
    let player = &mut catalog.hulls[0];
    player.weapons.push(StockWeapon {
        weapon: WeaponId(133),
        count: 1,
        ammo: 0,
    });
    let mut exits = ShipExits {
        up_compress: (100, 100),
        down_compress: (100, 100),
        frames: 36,
        ..ShipExits::default()
    };
    exits.points[1] = [(10, 0), (20, 0), (30, 0), (40, 0)];
    player.exits = Some(exits);
    catalog
}

/// Which of [`defended_player`]'s turret exits, 0 to 3, the newest shot
/// of `weapon` the player fired left from, the player facing up at
/// `centre`.
fn exit_of(session: &Session, weapon: WeaponId, centre: Vec2) -> usize {
    let shot = session
        .shots()
        .iter()
        .rfind(|shot| shot.firer == ShipRef::Player && shot.weapon.id == weapon)
        .expect("a shot");
    let start = shot.moved_from.unwrap_or(shot.position) - centre;
    assert!(start.y.abs() < 1e-3, "beside it: {start:?}");
    (start.x / 10.0).round() as usize - 1
}

#[test]
fn the_players_point_defence_steps_the_turret_exits_before_its_trigger() {
    for (reading, defence_exit, gun_exit) in [(RuleSource::Engine, 1, 2), (RuleSource::Bible, 0, 1)]
    {
        let catalog = defended_player();
        let rules = Rulebook::default().with_override(RuleKey::PdExitStep, reading);
        let mut session = with_an_npc_at(&catalog, 750, 550, 180).with_rules(rules);
        let pass = |session: &mut Session| {
            work_with(
                session,
                &catalog,
                Controls::default(),
                &Attacking,
                Rules::default(),
                &mut NeverFires,
            );
        };
        // The NPC's missile in flight first, the player's trigger let go.
        pass(&mut session);
        let missile = |session: &Session| {
            session
                .shots()
                .iter()
                .any(|shot| shot.weapon.id == WeaponId(134))
        };
        assert!(missile(&session), "{reading:?}: the missile");
        assert!(
            session
                .shots()
                .iter()
                .all(|shot| shot.firer != ShipRef::Player),
            "{reading:?}: nothing of the player's yet"
        );
        // Then a pass in which its point defence and its trigger both fire.
        session.hold_fire(true, false);
        let centre = session.player().position;
        pass(&mut session);
        assert_eq!(
            exit_of(&session, WeaponId(133), centre),
            defence_exit,
            "{reading:?}: point defence"
        );
        assert_eq!(
            exit_of(&session, WeaponId(128), centre),
            gun_exit,
            "{reading:?}: the gun, from the exit after"
        );
    }
}
