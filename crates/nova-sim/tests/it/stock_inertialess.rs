//! Inertialess flight over the stock data: 26 stock hulls carry `shïp`
//! `Flags2` 0x0040, the Raven (`shïp` 164) among them, and no stock
//! `oütf` has `ModType` 38. A Raven flown by the player, or as an NPC,
//! turns a quarter and flies on along its new heading with no drift, as
//! `_ShipIsInertialess` and `_AdjustInertialessShipVelocity` fly it.
//! Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_data::records::ship::Ship;
use nova_sim::flight::{FlightRules, facing, inertialess_step, shortest_turn};
use nova_sim::speed_cap::whole_degree;
use nova_sim::stats::{INERTIAL_DAMPENER, INERTIALESS_HULL};
use nova_sim::{Controls, Pilot, PilotCatalog, Session, ShipId, ShipState, ShipStats, Turn, Vec2};

const RAVEN: ShipId = ShipId(164);

fn near(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < 1e-3
}

#[test]
fn twenty_six_stock_hulls_are_inertialess_and_no_stock_outfit_is_a_dampener() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let inertialess: Vec<ShipId> = data
        .ships()
        .into_iter()
        .filter(|record| record.fields.flags2 & INERTIALESS_HULL != 0)
        .map(|record| record.id)
        .collect();
    assert_eq!(inertialess.len(), 26, "{inertialess:?}");
    assert!(inertialess.contains(&RAVEN), "{inertialess:?}");
    let dampeners: Vec<_> = data
        .outfits()
        .into_iter()
        .filter(|outfit| {
            outfit
                .mods
                .iter()
                .any(|&(mod_type, _)| mod_type == INERTIAL_DAMPENER)
        })
        .map(|outfit| outfit.id)
        .collect();
    assert!(dampeners.is_empty(), "{dampeners:?}");
}

#[test]
fn the_stock_raven_flown_by_the_player_turns_a_quarter_and_flies_along_its_heading() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let record = data
        .get::<Ship>(RAVEN.0)
        .expect("present")
        .expect("decodes");
    assert_eq!(record.name, Some("Raven"));
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["ship"] = serde_json::json!(RAVEN.0);
    save["stellar"] = serde_json::Value::Null;
    let flown = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(&data, flown).expect("flies");
    assert_eq!(session.ship(), RAVEN);

    let thrust = Controls {
        thrust: true,
        ..Controls::default()
    };
    for _ in 0..300 {
        session.tick(thrust);
    }
    let top = session.top_speed();
    assert!((session.player_speed() - top).abs() < 1e-3, "at full speed");
    let from = session.player().heading;
    let left = Controls {
        turn: Turn::Left,
        ..Controls::default()
    };
    while shortest_turn(session.player().heading, from) < 90.0 {
        session.tick(left);
    }
    for _ in 0..100 {
        session.tick(Controls::default());
    }
    let ship = *session.player();
    let along = facing(whole_degree(ship.heading)) * session.player_speed();
    assert!(near(ship.velocity, along), "{ship:?}");
    session.tick(Controls::default());
    assert!(
        near(session.player().position - ship.position, along),
        "no drift: {:?}",
        session.player()
    );
}

#[test]
fn a_stock_raven_npc_turned_a_quarter_flies_along_its_heading() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let fields = data.ship_fields(RAVEN).expect("decodes");
    let raven = ShipStats::new(fields, &[]);
    assert!(raven.inertialess_hull);
    let handling = raven.handling;
    let top = handling.max_speed;
    // East at full speed, facing north.
    let mut state = ShipState {
        velocity: Vec2::new(top, 0.0),
        ..ShipState::default()
    };
    let mut speed = top;
    for _ in 0..200 {
        inertialess_step(
            &mut state,
            &mut speed,
            &handling,
            Controls::default(),
            FlightRules::default(),
        );
    }
    assert!((speed - top).abs() < f32::EPSILON, "{speed}");
    assert!(near(state.velocity, Vec2::new(0.0, -top)), "{state:?}");
}
