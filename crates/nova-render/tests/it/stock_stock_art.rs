//! A borrowed stock look decoded from the player's own stock data: a role
//! default's base frames decode at the scaled size.
//!
//! The stock data is copyrighted and never committed, so this test gets
//! its location from `common` and skips, passing, without `NOVA_DATA`. It
//! writes nothing and asserts in memory only.

use std::rc::Rc;

use nova_data::graphics::EightBitReading;
use nova_data::stock_look::{ROLE_DEFAULTS, Scale};
use nova_data::store::GameData;
use nova_data::{ShipId, ShipImage, StockArt};
use nova_render::{GameImages, ImageSource};
use nova_view::ImageKind;

use crate::common::nova_data;

#[test]
fn a_role_default_s_base_frames_decode_at_the_scaled_size() {
    let Some(dir) = nova_data() else { return };
    let art = StockArt::new(GameData::open(&dir, None).expect("the stock data opens"));
    let (_, stock) = ROLE_DEFAULTS[0];
    let sprite = art
        .ship_sprite_info(ShipId(stock))
        .expect("its sheet reads");
    let (width, height) = (
        sprite.sheet.info.image_width,
        sprite.sheet.info.image_height,
    );
    let frames_in_sheet = usize::from(sprite.sheet.info.frames);
    let scale = Scale::of(30, width.max(height));
    let game = GameData::default();
    let images = GameImages::new(&game, EightBitReading::AsDrawn).with_stock_art(Rc::new(art));
    let frames = images
        .frames(
            ImageKind::StockShip {
                image: ShipImage::Base,
                scale,
            },
            stock,
        )
        .expect("decodes");
    assert_eq!(frames.len(), frames_in_sheet);
    for frame in &frames {
        assert_eq!(
            (frame.width(), frame.height()),
            (scale.apply(width), scale.apply(height))
        );
    }
}
