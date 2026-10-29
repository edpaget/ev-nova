//! Tests of the player's escorts' weapons and rounds as the fleet enters a
//! system (`_RespawnEscort` @0x3d4b8-0x3d62c, its restock argument from
//! `_SetupShipsInSystem` @0x42c79): with no restock, at a jump's arrival
//! and a gate's exit, each keeps its slot's weapons and rounds, a
//! carrier's bay rounds and a weapon's reload included; restocked, at a
//! take-off or a pilot flown, each takes its class's stock, each weapon's
//! reload still the slot's (see `Session::enter_escorts`).
#![allow(clippy::float_cmp)]

use super::Session;
use super::escort_carriers::{aboard, escort_carrying, raided_escort, until_launched};
use super::fighters::tests::{BAY, GUN, carrying, fleet, out, warship};
use crate::catalog::{SystemId, WeaponId};
use crate::chance::NeverFires;
use crate::combat::armament::NEVER_FIRED;
use crate::escort::EscortOrder;
use crate::testkit::{FakePilotCatalog, begin_jump_now, fly_out, land_now};
use crate::traffic::npc::{Npc, NpcId};

/// The warship escort carrying two Vipers, [`escort_carrying`]'s.
fn carrier_escort(catalog: &FakePilotCatalog) -> (Session, NpcId) {
    let session = fleet(catalog, 0, vec![warship(None)]);
    let escort = session.space.escorts.fleet[0].expect("placed");
    assert_eq!(aboard(&session, escort), 2, "its class's stock");
    (session, escort)
}

/// The fleet's first escort's NPC.
fn first(session: &Session) -> &Npc {
    let id = session.space.escorts.fleet[0].expect("placed");
    session.space.npc(id).expect("in the system")
}

/// Its reload of `weapon`.
fn reload(npc: &Npc, weapon: WeaponId) -> f32 {
    npc.armament
        .mounts()
        .iter()
        .find(|mount| mount.spec.id == weapon)
        .map_or(-1.0, |mount| mount.reload)
}

/// Plots a course to 131, flies out and into the jump, runs `then` as the
/// jump ends, and arrives.
fn jump_then(session: &mut Session, catalog: &FakePilotCatalog, then: impl FnOnce(&mut Session)) {
    session.plot_course(SystemId(131)).expect("a route");
    fly_out(session, catalog);
    begin_jump_now(session, catalog).expect("jumps");
    then(session);
    assert_eq!(
        session.arrive(catalog, &mut NeverFires),
        Some(SystemId(131))
    );
}

#[test]
fn an_escort_carrier_that_launched_a_fighter_arrives_from_a_jump_one_short() {
    let catalog = escort_carrying();
    let (mut session, escort, _) = raided_escort(&catalog, EscortOrder::Attack, 0);
    assert!(until_launched(&mut session, &catalog, 5), "it launches");
    assert_eq!(aboard(&session, escort), 1);
    let mut left = None;
    jump_then(&mut session, &catalog, |session| {
        left = Some(aboard(session, escort));
    });
    let left = left.expect("taken as the jump ends");
    assert!(left < 2, "its fighters out: {left} aboard");
    assert_eq!(
        first(&session).rounds.slot(BAY),
        left,
        "no restock at a jump (restock 0 @0x6c1f9)"
    );
}

#[test]
fn an_escort_one_round_short_is_restocked_by_a_take_off() {
    let catalog = escort_carrying();
    let (mut session, escort) = carrier_escort(&catalog);
    session
        .space
        .npc_mut(escort)
        .expect("there")
        .rounds
        .set(BAY, 1);
    land_now(&mut session).expect("lands on 128, under the ship");
    session
        .take_off(&catalog, &mut NeverFires)
        .expect("takes off");
    assert_eq!(first(&session).rounds.slot(BAY), 2, "restock 1 @0x674c2");
}

#[test]
fn a_pilot_flown_flies_its_escorts_with_their_classs_stock() {
    let catalog = escort_carrying();
    let (session, _) = carrier_escort(&catalog);
    let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
    let flown = fleet(&catalog, 0, saved.escorts().to_vec());
    assert_eq!(first(&flown).rounds.slot(BAY), 2);
}

#[test]
fn a_weapon_reloading_as_the_player_jumps_is_still_reloading_on_arrival() {
    let catalog = escort_carrying();
    let (mut session, escort) = carrier_escort(&catalog);
    jump_then(&mut session, &catalog, |session| {
        session
            .space
            .npc_mut(escort)
            .expect("there")
            .armament
            .hold(BAY);
    });
    assert_eq!(reload(first(&session), BAY), 60.0, "+0xf8 never written");
}

#[test]
fn a_take_off_restocks_the_rounds_but_keeps_each_weapons_reload() {
    let catalog = escort_carrying();
    let (mut session, escort) = carrier_escort(&catalog);
    land_now(&mut session).expect("lands");
    let npc = session.space.npc_mut(escort).expect("kept while landed");
    npc.rounds.set(BAY, 0);
    npc.armament.hold(BAY);
    session
        .take_off(&catalog, &mut NeverFires)
        .expect("takes off");
    let npc = first(&session);
    assert_eq!(npc.rounds.slot(BAY), 2, "restocked");
    assert_eq!(reload(npc, BAY), 60.0, "its reload the slot's");
}

#[test]
fn the_players_fighter_keeps_its_guns_reload_through_a_jump() {
    let catalog = carrying();
    let mut session = fleet(&catalog, 0, vec![out()]);
    let viper = session.space.escorts.fleet[0].expect("out");
    jump_then(&mut session, &catalog, |session| {
        session
            .space
            .npc_mut(viper)
            .expect("there")
            .armament
            .hold(GUN);
    });
    let npc = first(&session);
    assert!(npc.carrier.is_some(), "still the player's fighter");
    assert_eq!(reload(npc, GUN), 5.0);
}

#[test]
fn an_escort_that_fired_arrives_timed_as_never_fired() {
    // `_RespawnEscort` zeroes the stamp +0xa8 (@0x3d50e).
    let catalog = escort_carrying();
    let (mut session, escort) = carrier_escort(&catalog);
    jump_then(&mut session, &catalog, |session| {
        session
            .space
            .npc_mut(escort)
            .expect("there")
            .armament
            .stamp_fire();
    });
    assert_eq!(first(&session).armament.fold().since_fire, NEVER_FIRED);
}

#[test]
fn an_escort_keeps_its_rounds_through_a_gate_exit() {
    use crate::gate::GateKind;
    use crate::pilot::Escort;
    use crate::reserves::Reserves;
    use crate::session::gate_follow::tests::{exit_gate_raising, following, gated};
    use crate::session::mission_ships::tests::{accepted, take_off};
    let catalog = gated(following(0, 0), 0);
    let mut session = accepted(&catalog);
    session.pilot.escorts.push(Escort {
        ship: crate::catalog::ShipId(200),
        reserves: Reserves::full(30.0, 45.0, 300.0),
        order: None,
        carried: false,
        wage: None,
        person: None,
        mark: None,
    });
    take_off(&mut session, &catalog);
    let id = session.space.escorts.fleet[0].expect("placed");
    session
        .space
        .npc_mut(id)
        .expect("there")
        .rounds
        .set(WeaponId(140), 3);
    exit_gate_raising(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
    assert_eq!(
        first(&session).rounds.slot(WeaponId(140)),
        3,
        "no restock at a gate's exit (restock 0 @0x672e0)"
    );
}
