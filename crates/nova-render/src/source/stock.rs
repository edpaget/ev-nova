//! A new-game hull's borrowed look, decoded from the player's own stock
//! art ([`ImageKind::StockShip`](nova_view::ImageKind::StockShip),
//! `nova_data::stock_look`).
//!
//! A stock ship's image is read from the stock art alone, as the game
//! data's ship sheets are read ([`GameImages::ship_frames`]'s way): its
//! `rlëD` under the images' readings, or its `PICT` pair cut. Each frame
//! is then scaled to the hull by the key's [`Scale`], to
//! [`Scale::apply`]'s size of each side, by the crate's box filter
//! ([`resized`]), which averages premultiplied colour by the area each
//! source pixel covers. Scaling as it decodes keeps every layer, glow, cut,
//! colorize and 8-bit path as it is, and the layers stay centred and
//! aligned, as they share the ratio.
//!
//! It doesn't use QuickDraw's stretch (`crate::quickdraw_stretch`): that
//! averages straight RGBA channel by channel, alpha included, as the
//! original's picture draws do, which stretch the mask apart. A decoded
//! sprite's transparent pixels are clear (`[0, 0, 0, 0]`) and sprites are
//! drawn with straight alpha, so each shrunk edge would mix in that black
//! and be faded by alpha again when drawn: a dark fringe round every hull.
//! The box filter keeps an edge's colour and lowers only its alpha. The
//! borrowed look is the new game's own, so there is no draw of the
//! original's to match.
//!
//! Without stock art, or without that ship in it, the key is
//! [`ImageError::Missing`]. It never reads the game data, and no game-data
//! kind ever reads the stock art, so the same ID in both never crosses.
//! Nothing decoded is kept but in the renderer's atlas for the run.

use std::borrow::Borrow;
use std::rc::Rc;

use nova_data::graphics::{Image, decode_rled_with};
use nova_data::stock_look::Scale;
use nova_data::{GameData, SheetKind, ShipId, ShipImage, StockArt};

use super::GameImages;
use crate::box_filter::resized;
use crate::images::ImageError;

impl<D: Borrow<GameData>> GameImages<D> {
    /// These images, with a borrowed look's frames
    /// ([`ImageKind::StockShip`](nova_view::ImageKind::StockShip)) decoded
    /// from `art` (see the module docs).
    #[must_use]
    pub fn with_stock_art(self, art: Rc<StockArt>) -> Self {
        Self {
            stock_art: Some(art),
            ..self
        }
    }

    /// The frames stock `shïp` `ship`'s `image` resolves to in the stock
    /// art, each scaled by `scale` (see the module docs).
    pub(super) fn stock_frames(
        &self,
        ship: i16,
        image: ShipImage,
        scale: Scale,
    ) -> Result<Vec<Image>, ImageError> {
        let art = self.stock_art.as_ref().ok_or(ImageError::Missing)?;
        let frames = match art.ship_image(ShipId(ship), image) {
            Ok(sheet) if sheet.kind == SheetKind::Rled => {
                let bytes = art.rled(sheet.image_id).ok_or(ImageError::Missing)?;
                decode_rled_with(bytes, None, self.readings)?.into_frames()
            }
            Ok(sheet) => sheet.sheet.into_frames(),
            Err(err) => {
                return Err(err.graphics().map_or(ImageError::Missing, |source| {
                    ImageError::Decode(source.clone())
                }));
            }
        };
        Ok(frames
            .into_iter()
            .map(|frame| scaled(frame, scale))
            .collect())
    }
}

/// `frame` resized to `scale` of each side by the box filter, or as it is
/// at 1:1.
fn scaled(frame: Image, scale: Scale) -> Image {
    if scale == Scale::ONE {
        return frame;
    }
    let (width, height) = (scale.apply(frame.width()), scale.apply(frame.height()));
    resized(&frame, width, height)
}

#[cfg(test)]
mod tests {
    use nova_data::Record;
    use nova_data::graphics::fixture::rgb888_pict;
    use nova_data::graphics::{EightBitReading, PICT};
    use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
    use nova_data::records::ship::Ship;
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::stock_look::fixture::{ROTATIONS, base_rled, glow_rled, stock_art, stock_store};
    use nova_data::store::fixture::store_of;
    use nova_view::ImageKind;

    use super::*;
    use crate::images::ImageSource;

    fn stock(image: ShipImage, scale: Scale) -> ImageKind {
        ImageKind::StockShip { image, scale }
    }

    /// The game data's images, with the stock art of ship 300, 40 px.
    fn images(game: &GameData) -> GameImages<&GameData> {
        GameImages::new(game, EightBitReading::AsDrawn)
            .with_stock_art(Rc::new(stock_art(&[(300, 40)])))
    }

    #[test]
    fn a_borrowed_rled_base_and_its_layer_decode_at_the_scaled_size() {
        let game = GameData::default();
        let images = images(&game);
        let scale = Scale::of(30, 40);
        let base = images.frames(stock(ShipImage::Base, scale), 300).unwrap();
        assert_eq!(base.len(), usize::from(ROTATIONS));
        let side = scale.apply(40);
        assert!(
            base.iter()
                .all(|frame| (frame.width(), frame.height()) == (side, side)),
            "{:?}",
            base.iter()
                .map(|f| (f.width(), f.height()))
                .collect::<Vec<_>>()
        );
        // A solid red frame stays solid red, every pixel opaque.
        assert_eq!(base[0].pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(base[0].pixel(side - 1, side - 1), Some([255, 0, 0, 255]));
        let glow = images.frames(stock(ShipImage::Glow, scale), 300).unwrap();
        assert_eq!(
            (glow[0].width(), glow[0].height()),
            (scale.apply(42), scale.apply(42))
        );
        let deep = images
            .deep_frames(stock(ShipImage::Base, scale), 300)
            .unwrap();
        assert_eq!(deep.len(), base.len());
    }

    const RED: [u8; 4] = [255, 0, 0, 255];
    const CLEAR: [u8; 4] = [0; 4];

    fn row(pixels: &[[u8; 4]]) -> Image {
        Image::from_rgba(pixels.len() as u32, 1, pixels.concat()).unwrap()
    }

    #[test]
    fn a_shrunk_edge_keeps_its_colour_and_loses_only_alpha() {
        // Sprites are drawn with straight alpha, so an edge pixel half
        // over clear stays red at half its alpha, never darkened.
        let shrunk = scaled(row(&[RED, RED, RED, CLEAR, CLEAR, CLEAR]), Scale::of(1, 2));
        assert_eq!((shrunk.width(), shrunk.height()), (3, 1));
        assert_eq!(shrunk.pixel(0, 0), Some(RED));
        assert_eq!(shrunk.pixel(1, 0), Some([255, 0, 0, 128]));
        assert_eq!(shrunk.pixel(2, 0), Some(CLEAR), "all clear stays clear");
    }

    #[test]
    fn an_enlarged_edge_keeps_its_colour_and_loses_only_alpha() {
        // The middle of three covers half of each of the two (and the one
        // row rounds to two).
        let enlarged = scaled(row(&[RED, CLEAR]), Scale::of(3, 2));
        assert_eq!((enlarged.width(), enlarged.height()), (3, 2));
        assert_eq!(enlarged.pixel(0, 0), Some(RED));
        assert_eq!(enlarged.pixel(1, 0), Some([255, 0, 0, 128]));
        assert_eq!(enlarged.pixel(2, 0), Some(CLEAR));
    }

    #[test]
    fn a_shrink_averages_by_the_area_each_pixel_covers() {
        // Three columns to two: each takes one whole and half of the
        // middle, both axes alike.
        let blue = [0, 0, 255, 255];
        let image =
            Image::from_rgba(3, 3, [[RED; 3], [RED; 3], [blue; 3]].concat().concat()).unwrap();
        let shrunk = scaled(image, Scale::of(2, 3));
        assert_eq!((shrunk.width(), shrunk.height()), (2, 2));
        assert_eq!(shrunk.pixel(0, 0), Some(RED));
        assert_eq!(shrunk.pixel(1, 1), Some([85, 0, 170, 255]));
    }

    #[test]
    fn a_borrowed_look_at_one_to_one_is_the_stock_frames_as_they_are() {
        let game = GameData::default();
        let stock = stock_store(&[(300, 40)]);
        let own = GameImages::new(&stock, EightBitReading::AsDrawn)
            .frames(ImageKind::Rled, base_rled(300))
            .unwrap();
        let borrowed = images(&game).frames(stock_kind_one(), 300).unwrap();
        assert_eq!(borrowed, own);
    }

    fn stock_kind_one() -> ImageKind {
        stock(ShipImage::Base, Scale::ONE)
    }

    #[test]
    fn a_borrowed_rled_decodes_under_the_images_readings() {
        use nova_data::graphics::fixture::RledBuilder;
        use nova_data::graphics::{RLED, ThirtyTwoBitReading};
        // Stock ship 320's base: a 32-bit rlëD, whose colours the reading
        // decides.
        let thirty_two = RledBuilder::new(2, 1)
            .depth(32)
            .count_lines()
            .frame(|f| f.line().pixels32(&[0x0000_7C00, 0x0000_03E0]))
            .build();
        let anim = ShipAnimBuilder::new()
            .frames_per(1)
            .base_set_count(1)
            .base_image_id(1500)
            .build();
        let resources = [
            (Ship::TYPE, 320, ShipBuilder::new().build()),
            (ShipAnim::TYPE, 320, anim),
            (RLED, 1500, thirty_two),
        ];
        let stock = store_of(&resources);
        let reading = |data| {
            GameImages::new(data, EightBitReading::AsDrawn)
                .with_thirty_two_bit(ThirtyTwoBitReading::TrueColour)
        };
        let own = reading(&stock).frames(ImageKind::Rled, 1500).unwrap();
        let game = GameData::default();
        let borrowed = reading(&game)
            .with_stock_art(Rc::new(StockArt::new(store_of(&resources))))
            .frames(stock_kind_one(), 320)
            .unwrap();
        assert_eq!(borrowed, own);
        let as_drawn = GameImages::new(&game, EightBitReading::AsDrawn)
            .with_stock_art(Rc::new(StockArt::new(store_of(&resources))))
            .frames(stock_kind_one(), 320)
            .unwrap();
        assert_ne!(borrowed, as_drawn, "the reading shows");
    }

    #[test]
    fn a_borrowed_pict_pair_base_decodes_cut_and_scaled() {
        // Stock ship 310: two 2 x 1 frames cut from PICT 2000, masked by
        // 2100.
        let anim = ShipAnimBuilder::new()
            .frames_per(2)
            .base_set_count(1)
            .base_image_id(2000)
            .base_mask_id(2100)
            .base_x_size(2)
            .base_y_size(1)
            .build();
        let art = StockArt::new(store_of(&[
            (Ship::TYPE, 310, ShipBuilder::new().build()),
            (ShipAnim::TYPE, 310, anim),
            (PICT, 2000, rgb888_pict(4, 1, &[[0, 255, 0]; 4])),
            (PICT, 2100, rgb888_pict(4, 1, &[[255; 3]; 4])),
        ]));
        let game = GameData::default();
        let images = GameImages::new(&game, EightBitReading::AsDrawn).with_stock_art(Rc::new(art));
        let scale = Scale::of(2, 1);
        let frames = images.frames(stock(ShipImage::Base, scale), 310).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(
            (frames[0].width(), frames[0].height()),
            (scale.apply(2), scale.apply(1))
        );
        assert_eq!(frames[0].pixel(3, 1), Some([0, 255, 0, 255]));
    }

    #[test]
    fn without_stock_art_or_its_ship_a_borrowed_look_is_missing() {
        let game = stock_store(&[(300, 40)]);
        let bare = GameImages::new(&game, EightBitReading::AsDrawn);
        assert_eq!(
            bare.frames(stock_kind_one(), 300),
            Err(ImageError::Missing),
            "the game data never answers for the stock art"
        );
        let other = GameData::default();
        assert_eq!(
            images(&other).frames(stock_kind_one(), 301),
            Err(ImageError::Missing),
            "not in the stock art"
        );
    }

    #[test]
    fn a_game_data_kind_never_reads_the_stock_art() {
        let game = GameData::default();
        let images = images(&game);
        for (kind, id) in [
            (ImageKind::Rled, base_rled(300)),
            (ImageKind::Rled, glow_rled(300)),
            (
                ImageKind::ShipSheet {
                    image: ShipImage::Base,
                },
                300,
            ),
        ] {
            assert_eq!(
                images.frames(kind, id),
                Err(ImageError::Missing),
                "{kind:?}"
            );
        }
        assert!(images.frames(stock_kind_one(), 300).is_ok());
    }
}
