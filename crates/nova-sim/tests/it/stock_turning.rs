//! Turning over the stock data: the Pegasus (`shïp` 132, `Maneuver` 25)
//! turns 2° a tick flown by the player, its rate's whole degrees, as
//! `_HandlePlayer`'s turn block truncates it, and 2.5° as an NPC, which
//! turns the full rate. Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_data::records::ship::Ship;
use nova_sim::flight::{FlightRules, Flyer, step};
use nova_sim::{Controls, Pilot, PilotCatalog, Session, ShipId, ShipState, ShipStats, Turn};

const PEGASUS: ShipId = ShipId(132);

#[test]
fn the_stock_pegasus_turns_2_a_tick_as_the_player_and_2_5_as_an_npc() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let record = data
        .get::<Ship>(PEGASUS.0)
        .expect("present")
        .expect("decodes");
    assert_eq!(record.name, Some("Pegasus"));
    assert_eq!(record.record.maneuver, 25);
    let right = Controls {
        turn: Turn::Right,
        ..Controls::default()
    };

    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["ship"] = serde_json::json!(PEGASUS.0);
    save["stellar"] = serde_json::Value::Null;
    let flown = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(&data, flown).expect("flies");
    assert_eq!(session.ship(), PEGASUS);
    assert!((session.handling().turn_rate - 2.5).abs() < 1e-6);
    let before = session.player().heading;
    session.tick(right);
    let turned = session.player().heading - before;
    assert!((turned - 2.0).abs() < 1e-6, "whole degrees: {turned}");

    let fields = data.ship_fields(PEGASUS).expect("decodes");
    let npc = ShipStats::new(fields, &[]).handling;
    let mut state = ShipState::default();
    step(&mut state, &npc, right, FlightRules::default(), Flyer::Npc);
    assert!(
        (state.heading - 2.5).abs() < 1e-6,
        "the full rate: {state:?}"
    );
}
