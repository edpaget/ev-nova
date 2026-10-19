//! The tractor hold over the stock data: `wëap` 171, the Flower of
//! Spring (`oütf` 221), is the one stock beam of negative `Impact` (−5),
//! and fired by the stock first ship it holds the stock ships of up to
//! 4/3 its mass. A
//! held stock player is refused its jump with the refusal beep, is pulled
//! at a quarter of its thrust, and its lock lapses 15 ticks after its
//! touch. Skips, passing, when `NOVA_DATA` is unset.

#![allow(clippy::float_cmp)]

use crate::common;

use nova_data::GameData;
use nova_sim::combat::ShipRef;
use nova_sim::combat::armament::MOD_WEAPON;
use nova_sim::combat::impact::{Body, HOLD_SHARE, Tractor, TractorLock, tractor};
use nova_sim::combat::weapon::WeaponSpec;
use nova_sim::hyperspace::JumpRefusal;
use nova_sim::speed_cap::SpeedCap;
use nova_sim::{CombatCatalog, Controls, NpcId, PilotCatalog, Session, SimSound, SystemId, Vec2};

const FLOWER_OF_SPRING: i16 = 171;

fn data() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// The stock first `chär`'s session, its first tick flown.
fn flying(data: &GameData) -> Session {
    let mut session = Session::start(data).expect("the stock first chär starts");
    session.tick(Controls::default());
    session
}

/// A ship of `mass` at the centre, at rest.
fn body(mass: i16) -> Body {
    Body {
        mass: f32::from(mass),
        class_speed: 3.0,
        top_speed: 3.0,
        ..Body::default()
    }
}

#[test]
fn the_flower_of_spring_is_the_one_stock_tractor_and_holds_a_ship_three_quarters_its_firers_mass() {
    let Some(data) = data() else {
        return;
    };
    let tractors: Vec<i16> = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .filter(|spec| spec.beam_length > 0.0 && spec.impact < 0.0)
        .map(|spec| spec.id.0)
        .collect();
    assert_eq!(tractors, [FLOWER_OF_SPRING]);
    let spec = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .find(|spec| spec.id.0 == FLOWER_OF_SPRING)
        .expect("wëap 171");
    assert_eq!(spec.impact, -5.0);
    // `oütf` 221 mounts it; no stock ship carries one by default.
    let mounts: Vec<_> = data
        .outfits()
        .iter()
        .filter(|outfit| outfit.mods.contains(&(MOD_WEAPON, FLOWER_OF_SPRING)))
        .map(|outfit| (outfit.id.0, outfit.name.clone()))
        .collect();
    assert_eq!(mounts, [(221, "Flower Of Spring".to_owned())]);
    // The stock first ship firing it: it holds every stock ship of no
    // more than 4/3 its mass, and pulls itself toward any heavier one.
    let session = flying(&data);
    let ships = data.ships();
    let first = ships
        .iter()
        .find(|ship| ship.id == session.ship())
        .expect("the first ship");
    let firer = body(first.fields.mass);
    let (mut holds, mut pulls) = (0, 0);
    for ship in ships.iter().filter(|ship| ship.fields.mass > 0) {
        let touched = tractor(
            &body(ship.fields.mass),
            &firer,
            Vec2::new(0.0, -100.0),
            spec.impact,
            SpeedCap::PerAxis,
        );
        if f64::from(first.fields.mass) >= HOLD_SHARE * f64::from(ship.fields.mass) {
            assert_eq!(touched, Tractor::Holds, "shïp {}", ship.id.0);
            holds += 1;
        } else {
            assert!(matches!(touched, Tractor::Pulls(_)), "shïp {}", ship.id.0);
            pulls += 1;
        }
    }
    assert!(
        holds > 0 && pulls > 0,
        "{holds} held, {pulls} pulled toward"
    );
}

/// The stock first ship's session on course for the first system linked
/// to its own, held by `holder`.
fn held(data: &GameData, holder: ShipRef) -> Session {
    use nova_data::records::system::System;
    let mut session = flying(data);
    let system = data
        .get::<System>(session.system().0)
        .expect("present")
        .expect("decodes")
        .record;
    let next: SystemId = system.con.into_iter().flatten().next().expect("a link");
    session.plot_course(next).expect("a route");
    nova_sim::fixture::set_player_tractor(&mut session, Some(TractorLock::by(holder)));
    session.take_sounds();
    session
}

#[test]
fn a_held_stock_player_is_refused_its_jump_with_the_refusal_beep() {
    let Some(data) = data() else {
        return;
    };
    let mut session = held(&data, ShipRef::Npc(NpcId(u32::MAX)));
    assert_eq!(session.begin_jump(), Err(JumpRefusal::Tractored));
    assert_eq!(session.take_sounds(), [SimSound::Refused]);
    assert_eq!(session.preparing_jump(), None);
}

#[test]
fn a_stock_player_holding_itself_brakes_by_a_quarter_of_its_thrust_and_lapses_after_15_ticks() {
    let Some(data) = data() else {
        return;
    };
    let mut session = held(&data, ShipRef::Player);
    let thrust = Controls {
        thrust: true,
        ..Controls::default()
    };
    for _ in 0..3 {
        session.tick(thrust);
    }
    let lock = session.tractor().expect("held");
    assert_eq!(lock.ticks_since_touch, 3);
    let before = session.player().velocity;
    assert!(before.length() > 0.0, "it gained speed against the brake");
    for _ in 4..15 {
        session.tick(Controls::default());
    }
    assert!(session.tractor().is_some(), "held for 14 ticks");
    session.tick(Controls::default());
    assert_eq!(session.tractor(), None, "lapsed on the 15th");
    assert!(session.player().velocity.length() < before.length());
}
