//! Synthetic stock art for tests: stock ships with plain sheets built in
//! code, so no test needs the player's data, which is copyrighted and
//! never committed. Behind the `fixture` feature, for other crates' tests.
//!
//! Each ship `ship` of side `side` is a `shïp`, a `shän` with one set of
//! [`ROTATIONS`] rotations naming base `rlëD` [`base_rled`]`(ship)` and
//! glow `rlëD` [`glow_rled`]`(ship)`, a base sheet of `side` x `side`
//! frames, and a glow sheet of frames two pixels wider and taller, as a
//! stock glow sits about its hull (an `rlëD`'s own frame size is what is
//! read, so the `shän` names no glow size).

use nova_rsrc::ResType;

use crate::Record;
use crate::graphics::RLED;
use crate::graphics::fixture::RledBuilder;
use crate::records::fixture::{ShipAnimBuilder, ShipBuilder};
use crate::records::ship::Ship;
use crate::records::ship_anim::ShipAnim;
use crate::store::fixture::store_of;
use crate::store::{GameData, StockArt};

/// The rotations each fixture ship turns through.
pub const ROTATIONS: u16 = 2;

/// The base `rlëD` of fixture ship `ship`.
#[must_use]
pub const fn base_rled(ship: i16) -> i16 {
    ship + 1000
}

/// The glow `rlëD` of fixture ship `ship`.
#[must_use]
pub const fn glow_rled(ship: i16) -> i16 {
    ship + 2000
}

/// An `rlëD` of [`ROTATIONS`] solid `side` x `side` frames, red then blue.
#[must_use]
pub fn sheet(side: u16) -> Vec<u8> {
    [0x7C00, 0x001F]
        .into_iter()
        .take(usize::from(ROTATIONS))
        .fold(RledBuilder::new(side, side), |sheet, colour| {
            sheet.frame(|frame| {
                (0..side).fold(frame, |frame, _| {
                    frame.line().pixels(&vec![colour; usize::from(side)])
                })
            })
        })
        .build()
}

/// The resources of stock ship `ship` of side `side` (see the module
/// docs).
#[must_use]
pub fn ship_resources(ship: i16, side: u16) -> Vec<(ResType, i16, Vec<u8>)> {
    let px = i16::try_from(side).expect("a side within i16");
    let anim = ShipAnimBuilder::new()
        .base_image_id(base_rled(ship))
        .base_set_count(1)
        .base_x_size(px)
        .base_y_size(px)
        .frames_per(i16::try_from(ROTATIONS).expect("a few"))
        .glow_image_id(glow_rled(ship))
        .build();
    vec![
        (Ship::TYPE, ship, ShipBuilder::new().build()),
        (ShipAnim::TYPE, ship, anim),
        (RLED, base_rled(ship), sheet(side)),
        (RLED, glow_rled(ship), sheet(side + 2)),
    ]
}

/// The stock store of `ships`, each a (stock `shïp` ID, side).
#[must_use]
pub fn stock_store(ships: &[(i16, u16)]) -> GameData {
    let resources: Vec<_> = ships
        .iter()
        .flat_map(|&(ship, side)| ship_resources(ship, side))
        .collect();
    store_of(&resources)
}

/// The stock art of `ships` ([`stock_store`]).
#[must_use]
pub fn stock_art(ships: &[(i16, u16)]) -> StockArt {
    StockArt::new(stock_store(ships))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::id::ShipId;

    #[test]
    fn each_ship_s_sheets_are_its_own_rleds_with_the_glow_two_pixels_wider() {
        assert_eq!((base_rled(300), glow_rled(300)), (1300, 2300));
        let art = stock_art(&[(300, 40), (301, 32)]);
        for (ship, side) in [(300, 40), (301, 32)] {
            let base = art.ship_sprite_info(ShipId(ship)).expect("a sheet");
            assert_eq!(base.image_id, base_rled(ship));
            assert_eq!(
                (base.sheet.info.frame_width, base.sheet.info.frame_height),
                (side, side)
            );
            let layers = art.ship_layer_infos(ShipId(ship)).expect("layers");
            let glow = layers.glow.expect("a glow").expect("reads");
            assert_eq!(glow.image_id, glow_rled(ship));
            assert_eq!(glow.sheet.info.frame_width, side + 2);
        }
    }
}
