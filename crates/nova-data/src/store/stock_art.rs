//! The stock art: the player's own stock `Nova Files`, opened beside the
//! new game's data for the opt-in stock look ([`crate::stock_look`]).
//!
//! The stock EV Nova art is copyrighted, so it is read only here, only to
//! draw, and nothing it reads is ever written anywhere: [`StockArt`] wraps
//! the stock store read-only. It isn't `Clone` (nothing copies it out) and
//! isn't `Serialize` (nothing saves it), and it has no `&mut self` or
//! by-value accessor. It answers only the reads drawing a ship takes, each
//! by stock ID: a `shïp`'s sheet and layers, its `shän`, an image of it,
//! and the raw `rlëD` those name. The app builds one at startup, shares it
//! in an `Rc` between the view's and the renderer's adapters, and drops it
//! when the game ends; nothing is cached to disk.
//!
//! ```compile_fail
//! fn copies<T: Clone>() {}
//! copies::<nova_data::StockArt>();
//! ```
//!
//! ```compile_fail
//! fn saves<T: serde::Serialize>() {}
//! saves::<nova_data::StockArt>();
//! ```
//!
//! It never answers for the new game's store, nor that store for it: the
//! two are keyed apart (the renderer's `ImageKind::StockShip` resolves only
//! here, `nova_render::GameImages::with_stock_art`).

use super::{
    GameData, LayerSprite, ShipImage, ShipImageError, ShipLayerInfos, ShipSpriteInfo, SpriteError,
};
use crate::graphics::RLED;
use crate::records::ship_anim::ShipAnim;
use crate::wire::id::ShipId;

/// The player's stock data, read-only, for the borrowed look (see the
/// module docs).
#[derive(Debug)]
pub struct StockArt {
    data: GameData,
}

impl StockArt {
    /// The stock art of `data`, the stock `Nova Files` the player owns.
    #[must_use]
    pub fn new(data: GameData) -> Self {
        Self { data }
    }

    /// Stock `shïp` `ship`'s base sheet, scanned
    /// ([`GameData::ship_sprite_info`]).
    ///
    /// # Errors
    ///
    /// Why the sheet can't be read, as the store says.
    pub fn ship_sprite_info(&self, ship: ShipId) -> Result<ShipSpriteInfo<'_>, SpriteError> {
        self.data.ship_sprite_info(ship)
    }

    /// Stock `shïp` `ship`'s layers, scanned
    /// ([`GameData::ship_layer_infos`]).
    ///
    /// # Errors
    ///
    /// Why its `shïp` or `shän` can't be read, as the store says.
    pub fn ship_layer_infos(&self, ship: ShipId) -> Result<ShipLayerInfos<'_>, SpriteError> {
        self.data.ship_layer_infos(ship)
    }

    /// Stock `shïp` `ship`'s `shän`, when it has one that decodes.
    #[must_use]
    pub fn ship_anim(&self, ship: ShipId) -> Option<&ShipAnim> {
        Some(self.data.get::<ShipAnim>(ship.0)?.ok()?.record)
    }

    /// Stock `shïp` `ship`'s `image`, decoded ([`GameData::ship_image`]).
    ///
    /// # Errors
    ///
    /// Why the image can't be read, as the store says.
    pub fn ship_image(
        &self,
        ship: ShipId,
        image: ShipImage,
    ) -> Result<LayerSprite<'_>, ShipImageError> {
        self.data.ship_image(ship, image)
    }

    /// The bytes of stock `rlëD` `id`, which a sheet's image names.
    #[must_use]
    pub fn rled(&self, id: i16) -> Option<&[u8]> {
        Some(self.data.resource(RLED, id)?.resource.data())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Record;
    use crate::graphics::fixture::RledBuilder;
    use crate::records::fixture::{ShipAnimBuilder, ShipBuilder};
    use crate::records::ship::Ship;
    use crate::store::fixture::store_of;

    /// Stock `shïp` 140, its `shän` naming `rlëD` 1100 of two 3 x 2
    /// frames, and a glow `rlëD` 1101.
    fn art() -> StockArt {
        let rled = || {
            RledBuilder::new(3, 2)
                .frame(|f| f.line().pixels(&[0x7C00; 3]).line().pixels(&[0x7C00; 3]))
                .frame(|f| f.line().pixels(&[0x001F; 3]).line().pixels(&[0x001F; 3]))
                .build()
        };
        let anim = ShipAnimBuilder::new()
            .base_image_id(1100)
            .base_set_count(1)
            .frames_per(2)
            .glow_image_id(1101)
            .build();
        StockArt::new(store_of(&[
            (Ship::TYPE, 140, ShipBuilder::new().build()),
            (ShipAnim::TYPE, 140, anim),
            (RLED, 1100, rled()),
            (RLED, 1101, rled()),
        ]))
    }

    #[test]
    fn it_answers_a_stock_ship_s_sheet_layers_shan_and_images_by_its_id() {
        let art = art();
        let sheet = art.ship_sprite_info(ShipId(140)).expect("a sheet");
        assert_eq!(sheet.image_id, 1100);
        assert_eq!(
            (sheet.sheet.info.frame_width, sheet.sheet.info.frame_height),
            (3, 2)
        );
        let layers = art.ship_layer_infos(ShipId(140)).expect("layers");
        assert_eq!(layers.glow.expect("a glow").expect("reads").image_id, 1101);
        assert_eq!(
            art.ship_anim(ShipId(140)).map(|anim| anim.frames_per),
            Some(2)
        );
        let base = art.ship_image(ShipId(140), ShipImage::Base).expect("reads");
        assert_eq!(base.image_id, 1100);
        assert!(art.rled(1100).is_some_and(|bytes| !bytes.is_empty()));
    }

    #[test]
    fn a_stock_id_it_does_not_hold_answers_nothing() {
        let art = art();
        assert!(art.ship_sprite_info(ShipId(141)).is_err());
        assert!(art.ship_layer_infos(ShipId(141)).is_err());
        assert_eq!(art.ship_anim(ShipId(141)), None);
        assert!(art.ship_image(ShipId(141), ShipImage::Base).is_err());
        assert_eq!(art.rled(1102), None);
    }
}
