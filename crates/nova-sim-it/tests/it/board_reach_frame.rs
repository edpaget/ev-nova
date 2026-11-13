//! Wiring: a plug-in ship's board reach reaches `HullSpec` from its loaded
//! base-sprite frame through the `GameData` adapter (rdm combat-and-flight
//! phase 8): an `rlëD`'s header size whatever the `shän` says, and a
//! `PICT` sheet's `shän` sizes.

use nova_data::Record;
use nova_data::graphics::fixture::{RledBuilder, rgb888_pict};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::store::fixture::store_of;
use nova_sim::catalog::ShipId;
use nova_sim::combat::hull::HullSpec;
use nova_sim::{CombatCatalog, Vec2};

/// A `shän` of base image `image`, 24 x 40.
fn shan(image: i16) -> Vec<u8> {
    ShipAnimBuilder::new()
        .base_image_id(image)
        .base_x_size(24)
        .base_y_size(40)
        .build()
}

#[test]
fn a_ship_is_boarded_over_its_loaded_frame() {
    let data = store_of(&[
        (Ship::TYPE, 128, ShipBuilder::new().build()),
        (Ship::TYPE, 129, ShipBuilder::new().build()),
        (ShipAnim::TYPE, 128, shan(1000)),
        (ShipAnim::TYPE, 129, shan(1001)),
        (RLED, 1000, RledBuilder::new(30, 50).frame(|f| f).build()),
        (PICT, 1001, rgb888_pict(1, 1, &[[0; 3]])),
    ]);
    let reach = |id| {
        let hulls = data.hulls();
        let record = hulls.iter().find(|h| h.id == ShipId(id)).expect("a hull");
        HullSpec::new(record).board_reach
    };
    assert_eq!(reach(128), Vec2::new(15.0, 25.0), "the rlëD's header");
    assert_eq!(reach(129), Vec2::new(12.0, 20.0), "a PICT sheet's shän");
}
