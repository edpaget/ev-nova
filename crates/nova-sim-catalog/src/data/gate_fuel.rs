//! Tests of [`ShipFields::gate_fuel`](crate::handling::ShipFields::gate_fuel)
//! over the game data: [`GameData::ship_fields`](crate::catalog::PilotCatalog::ship_fields)
//! and [`GameData::ships`](crate::catalog::PilotCatalog::ships) serve each
//! ship the store's gate fuel (`nova_data::GameData::gate_fuel`), which
//! only a new game's store holds.

use std::collections::BTreeMap;

use nova_data::records::fixture::ShipBuilder;
use nova_data::records::ship::Ship;
use nova_data::{GameData, Record};

use crate::catalog::{PilotCatalog, ShipId};
use crate::test_store::store;

/// Two `shïp`s, 128 and 129.
fn two_ships() -> GameData {
    store(&[
        (Ship::TYPE, 128, ShipBuilder::new().build()),
        (Ship::TYPE, 129, ShipBuilder::new().build()),
    ])
}

/// Each ship and the gate fuel a catalog call serves it.
type Served = Vec<(ShipId, Option<i16>)>;

/// Each ship's gate fuel as `ship_fields` serves it, and as `ships` does.
fn served(data: &GameData) -> (Vec<Option<i16>>, Served) {
    let fields = [128, 129]
        .map(|id| data.ship_fields(ShipId(id)).expect("decodes").gate_fuel)
        .to_vec();
    let records = data
        .ships()
        .iter()
        .map(|ship| (ship.id, ship.fields.gate_fuel))
        .collect();
    (fields, records)
}

#[test]
fn a_ship_s_fields_carry_the_store_s_gate_fuel_for_it() {
    let data = two_ships().with_gate_fuels(BTreeMap::from([(ShipId(128), 150)]));
    assert_eq!(
        served(&data),
        (
            vec![Some(150), None],
            vec![(ShipId(128), Some(150)), (ShipId(129), None)]
        ),
        "the unlisted ship has none"
    );
}

#[test]
fn a_stock_store_s_ships_have_no_gate_fuel() {
    assert_eq!(
        served(&two_ships()),
        (
            vec![None, None],
            vec![(ShipId(128), None), (ShipId(129), None)]
        )
    );
}

#[test]
fn the_gate_fuel_leaves_the_ship_s_other_fields_alone() {
    let stock = two_ships().ship_fields(ShipId(128));
    let mut fared = two_ships()
        .with_gate_fuels(BTreeMap::from([(ShipId(128), 150)]))
        .ship_fields(ShipId(128))
        .expect("decodes");
    fared.gate_fuel = None;
    assert_eq!(Ok(fared), stock);
}
