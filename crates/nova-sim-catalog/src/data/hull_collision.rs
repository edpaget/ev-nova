//! Tests of [`HullRecord::collision_radius`](crate::catalog::HullRecord::collision_radius)
//! over the game data: [`GameData::hulls`](crate::catalog::CombatCatalog::hulls)
//! serves each hull the store's collision radius for its ship
//! (`nova_data::GameData::collision_radius`), which only a new game's
//! store holds.

use std::collections::BTreeMap;

use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::{GameData, Record};

use crate::catalog::{CombatCatalog, ShipId};
use crate::test_store::store;

/// Two `shïp`s, 128 and 129, each with a 26 x 40 `shän`.
fn two_ships() -> GameData {
    let anim = || {
        ShipAnimBuilder::new()
            .base_x_size(26)
            .base_y_size(40)
            .build()
    };
    store(&[
        (Ship::TYPE, 128, ShipBuilder::new().build()),
        (Ship::TYPE, 129, ShipBuilder::new().build()),
        (ShipAnim::TYPE, 128, anim()),
        (ShipAnim::TYPE, 129, anim()),
    ])
}

fn radii(data: &GameData) -> Vec<(ShipId, Option<i16>)> {
    data.hulls()
        .iter()
        .map(|hull| (hull.id, hull.collision_radius))
        .collect()
}

#[test]
fn a_hull_carries_the_store_s_collision_radius_for_its_ship() {
    let data = two_ships().with_collision_radii(BTreeMap::from([(ShipId(128), 20)]));
    assert_eq!(
        radii(&data),
        [(ShipId(128), Some(20)), (ShipId(129), None)],
        "the unlisted ship has none"
    );
}

#[test]
fn a_stock_store_s_hulls_have_no_collision_radius() {
    assert_eq!(
        radii(&two_ships()),
        [(ShipId(128), None), (ShipId(129), None)]
    );
}

#[test]
fn the_radius_leaves_the_hull_s_sprite_sizes_alone() {
    let stock = two_ships().hulls();
    let mut sized = two_ships()
        .with_collision_radii(BTreeMap::from([(ShipId(128), 20)]))
        .hulls();
    sized[0].collision_radius = None;
    assert_eq!(sized, stock);
}
