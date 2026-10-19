//! Stock ships leaving by hyperspace (`_ShipCanExpiditeJumps` @0x7872 and
//! `_AIShipHasFuelForJump` @0x7f48f; see `nova_sim::ai::leave`). Skips,
//! passing, when `NOVA_DATA` is unset.
//!
//! - The classes of `shïp` `Flags2` 0x0020 expedite: the Wraith (Adult),
//!   the Vell-os Dart, Arrow and Javelin, and the cloaking and fast-jump
//!   variants 266-273, 326-330 and 406-411.
//! - The Hyperioid (171), whose class `Fuel` is 0, can't jump; a Shuttle
//!   (128) can, with an empty tank.

use crate::common;

use nova_data::GameData;
use nova_sim::ai::leave::{can_jump, expedites};
use nova_sim::combat::armament::Arsenal;
use nova_sim::rulebook::RuleSource;
use nova_sim::traffic::npc::Npc;
use nova_sim::traffic::table::kind;
use nova_sim::{PilotCatalog, ShipId};

fn opened() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// An NPC of stock ship type `ship`, as the traffic builds its kind.
fn stock_npc(data: &GameData, arsenal: &Arsenal, ship: ShipId) -> Npc {
    let record = data
        .ships()
        .into_iter()
        .find(|record| record.id == ship)
        .expect("the ship type reads");
    let kind = kind(
        &record,
        &data.outfits(),
        arsenal,
        nova_sim::RuleSource::Engine,
    );
    let mut npc = nova_sim::traffic::fixture::npc(1, kind.stats);
    npc.ship = ship;
    npc.class_fuel = kind.class_fuel;
    npc
}

#[test]
fn the_stock_classes_of_flags2_0x0020_expedite_their_jumps() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let expediting: Vec<i16> = data
        .ships()
        .iter()
        .map(|record| stock_npc(&data, &arsenal, record.id))
        .filter(expedites)
        .map(|npc| npc.ship.0)
        .collect();
    let mut flagged: Vec<i16> = vec![168, 173, 174, 175, 185];
    flagged.extend(266..=273);
    flagged.extend(326..=330);
    flagged.extend([381, 382, 383]);
    flagged.extend(406..=411);
    assert_eq!(expediting, flagged);
}

#[test]
fn a_stock_class_with_no_fuel_cant_jump_and_a_shuttle_can_with_an_empty_tank() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let hyperioid = stock_npc(&data, &arsenal, ShipId(171));
    assert_eq!(hyperioid.class_fuel, 0);
    assert!(!can_jump(&hyperioid, RuleSource::Engine));
    let mut shuttle = stock_npc(&data, &arsenal, ShipId(128));
    assert_eq!(shuttle.class_fuel, 300);
    shuttle.reserves.fuel.now = 0.0;
    assert!(can_jump(&shuttle, RuleSource::Engine));
    assert!(!can_jump(&shuttle, RuleSource::Bible));
}
