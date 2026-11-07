//! A `shän`'s six images, and reading one from its `PICT` pair when it has
//! no `rlëD`, as `_LoadExtendedShipSprites` (@0x226af-0x23e3e) does.
//!
//! - **Per image, with no fallback between images.** The base, alt, glow,
//!   lights, weapon and shield ([`ShipImage`]) each test for an `rlëD` of
//!   their image ID (base @0x22856-0x2287d, alt @0x22bb8-0x22be2, glow
//!   @0x22eab-0x22ed5, lights @0x2317a-0x231a4, weapon @0x234d1-0x234fb,
//!   shield @0x23791-0x237bb). An `rlëD` that exists is loaded, and one
//!   that fails is not followed by the `PICT`. Otherwise the image is cut
//!   from its own `PICT` pair by `_SWCreateSpriteFromSinglePictXY` (base
//!   @0x2298b, alt @0x22d09, glow @0x22fe3, lights @0x232b2, weapon
//!   @0x23609, shield @0x238b7) and converted by `_ConvertSpriteToRLE`.
//!   One image's path never decides another's.
//! - **The fields** ([`ShipImage::fields`]): each image's sprites, mask,
//!   `XSize` and `YSize` `shän` fields, at 0x00/0x02/0x06/0x08 for the
//!   base, 0x0c/0x0e/0x12/0x14 for the alt, 0x16/0x18/0x1a/0x1c for the
//!   glow, 0x1e/0x20/0x22/0x24 for the lights, 0x26/0x28/0x2a/0x2c for the
//!   weapon and 0x40/0x42/0x44/0x46 for the shield.
//! - **Which are loaded** ([`ShipImage::defined`]): the base always, with
//!   no ID check; the alt with `AltImageID` and `AltSetCount` above 0
//!   (@0x22ae8-0x22afe); the others with their image ID above 0. The glow,
//!   lights and weapon are also gated on preferences, which the sim
//!   models; that does not change here.
//! - **The frame count** ([`ShipImage::frame_limit`]): the cut's
//!   `maxFrames`, `FramesPer` × `BaseSetCount` for the base, glow, lights
//!   and weapon (@0x22934-0x2295b, @0x22f8b-0x22fb3, @0x23258-0x23282,
//!   @0x235b1-0x235d9), `FramesPer` × `AltSetCount` for the alt
//!   (@0x22cac-0x22cd1), and the whole grid for the shield
//!   (@0x23858-0x238b7). `FramesPer` 0 is read as 36 (@0x22731-0x2274c)
//!   and `BaseSetCount` 0 or less as 1 (@0x2275b-0x2276f). A pair then
//!   loads only when its picture is a grid of exactly that many frames
//!   ([`FrameLimit`]). With `ShipAnimations` off the original cuts the alt
//!   by `FramesPer` × 1, which fails a multi-set alt's conversion and
//!   asserts on its first draw; the engine always cuts it by `AltSetCount`,
//!   as intended. The original multiplies in 16 bits (`imulw` @0x22941);
//!   the engine multiplies wider.
//! - **Failure.** A failed cut or conversion stops the game with alert
//!   `ALRT` 131 (`_ReportSpriteLoadError`), then crashes on the null or
//!   empty sprite. The engine reads each as that image's error instead, as
//!   for a `spïn`'s pair. No sound is played.
//!
//! Not built: a class whose image ID matches an earlier class's for the
//! same image takes that class's sprite (`_SWCloneSprite`, base
//! @0x227d6-0x22851, matched by the table at `_gPause`+0xa), ignoring its
//! own mask, size and count. The engine resolves each `shän` on its own
//! (rdm `rendering-2`).

use std::fmt;

use super::pict_pair::{PairCut, PairError, SheetKind};
use super::sprite::{LayerError, LayerSprite, ReadSheet, ShipLayer, SpriteError};
use super::{GameData, SourceFile};
use crate::graphics::{FrameLimit, GraphicsError, PICT, RLED};
use crate::records::ship_anim::ShipAnim;
use crate::wire::id::ShipId;

/// One of a `shän`'s six images (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShipImage {
    /// The hull (`BaseImageID`).
    Base,
    /// The alt layer (`AltImageID`).
    Alt,
    /// The engine glow (`GlowImageID`).
    Glow,
    /// The running lights (`LightImageID`).
    Lights,
    /// The weapon glow (`WeapImageID`).
    Weapon,
    /// The shield (`ShieldImageID`).
    Shield,
}

/// The `shän` fields one image is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipImageFields {
    /// The image's `rlëD`, or its sprites `PICT`.
    pub image_id: i16,
    /// Its mask `PICT`: the sprites themselves when it is `image_id`.
    pub mask_id: i16,
    /// Each frame's width in a `PICT` pair.
    pub x_size: i16,
    /// Each frame's height in a `PICT` pair.
    pub y_size: i16,
}

/// The frames per rotation `_LoadExtendedShipSprites` reads `FramesPer` 0
/// as (@0x22731-0x2274c).
const DEFAULT_FRAMES_PER: i32 = 36;

impl ShipImage {
    /// Every image, in the order `_LoadExtendedShipSprites` loads them.
    pub const ALL: [Self; 6] = [
        Self::Base,
        Self::Alt,
        Self::Glow,
        Self::Lights,
        Self::Weapon,
        Self::Shield,
    ];

    /// The layer it is drawn as over the hull: `None` for the hull itself.
    #[must_use]
    pub const fn layer(self) -> Option<ShipLayer> {
        match self {
            Self::Base => None,
            Self::Alt => Some(ShipLayer::Alt),
            Self::Glow => Some(ShipLayer::Glow),
            Self::Lights => Some(ShipLayer::Lights),
            Self::Weapon => Some(ShipLayer::Weapon),
            Self::Shield => Some(ShipLayer::Shield),
        }
    }

    /// The `shän` fields it is read from.
    #[must_use]
    pub const fn fields(self, anim: &ShipAnim) -> ShipImageFields {
        let [image_id, mask_id, x_size, y_size] = match self {
            Self::Base => [
                anim.base_image_id,
                anim.base_mask_id,
                anim.base_x_size,
                anim.base_y_size,
            ],
            Self::Alt => [
                anim.alt_image_id,
                anim.alt_mask_id,
                anim.alt_x_size,
                anim.alt_y_size,
            ],
            Self::Glow => [
                anim.glow_image_id,
                anim.glow_mask_id,
                anim.glow_x_size,
                anim.glow_y_size,
            ],
            Self::Lights => [
                anim.light_image_id,
                anim.light_mask_id,
                anim.light_x_size,
                anim.light_y_size,
            ],
            Self::Weapon => [
                anim.weap_image_id,
                anim.weap_mask_id,
                anim.weap_x_size,
                anim.weap_y_size,
            ],
            Self::Shield => [
                anim.shield_image_id,
                anim.shield_mask_id,
                anim.shield_x_size,
                anim.shield_y_size,
            ],
        };
        ShipImageFields {
            image_id,
            mask_id,
            x_size,
            y_size,
        }
    }

    /// Whether `anim` loads it at all.
    #[must_use]
    pub const fn defined(self, anim: &ShipAnim) -> bool {
        match self {
            Self::Base => true,
            Self::Alt => anim.alt_image_id > 0 && anim.alt_set_count > 0,
            Self::Glow | Self::Lights | Self::Weapon | Self::Shield => {
                self.fields(anim).image_id > 0
            }
        }
    }

    /// How many frames its `PICT` pair must hold.
    #[must_use]
    pub fn frame_limit(self, anim: &ShipAnim) -> FrameLimit {
        let frames_per = match anim.frames_per {
            0 => DEFAULT_FRAMES_PER,
            frames_per => i32::from(frames_per),
        };
        let sets = match self {
            Self::Shield => return FrameLimit::WholeGrid,
            Self::Alt => i32::from(anim.alt_set_count),
            Self::Base | Self::Glow | Self::Lights | Self::Weapon => {
                i32::from(anim.base_set_count.max(1))
            }
        };
        FrameLimit::Exactly(frames_per * sets)
    }
}

/// One image read as a sheet `S`, and where from.
pub(super) struct ImageRead<'a, S> {
    /// What it was read from.
    pub kind: SheetKind,
    /// The sheet.
    pub sheet: S,
    /// The file its `rlëD`, or its sprites `PICT`, came from.
    pub source: &'a SourceFile,
}

/// Why one image could not be read; the caller names the ship and image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ImageFault {
    /// Neither an `rlëD` nor a sprites `PICT` of its ID.
    NoSheet,
    /// No mask `PICT` of its mask ID.
    NoMask {
        /// Its mask ID.
        mask_id: i16,
    },
    /// Its `rlëD` failed to read.
    Graphics(GraphicsError),
    /// A `PICT` failed to decode, or the pair to cut.
    Picture {
        /// The `PICT`'s ID.
        pict_id: i16,
        /// What went wrong.
        source: GraphicsError,
    },
}

/// Why [`GameData::ship_image`] could not resolve one image.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ShipImageError {
    /// The `shïp` or `shän` is missing or broken, or the base image is.
    #[error(transparent)]
    Sprite(SpriteError),
    /// The layer is.
    #[error(transparent)]
    Layer(LayerError),
    /// The `shän` does not load this image ([`ShipImage::defined`]).
    #[error("shïp {ship}: no {image} image")]
    Undefined {
        /// The ship's ID.
        ship: i16,
        /// Which image.
        image: ShipImage,
    },
}

impl ShipImageError {
    /// What went wrong reading the image, when its resources are there but
    /// do not read: an `rlëD` that does not decode, or a `PICT` pair that
    /// does not decode or cut. `None` when a link is missing.
    #[must_use]
    pub fn graphics(&self) -> Option<&GraphicsError> {
        match self {
            Self::Sprite(
                SpriteError::Graphics { source, .. } | SpriteError::Picture { source, .. },
            )
            | Self::Layer(
                LayerError::Graphics { source, .. } | LayerError::Picture { source, .. },
            ) => Some(source),
            Self::Sprite(
                SpriteError::NoShip(_)
                | SpriteError::NoShipAnim(_)
                | SpriteError::Decode(_)
                | SpriteError::NoSheet { .. }
                | SpriteError::NoMask { .. },
            )
            | Self::Layer(LayerError::NoSheet { .. } | LayerError::NoMask { .. })
            | Self::Undefined { .. } => None,
        }
    }
}

impl GameData {
    /// Resolves one of `ship`'s images, decoded, as
    /// [`GameData::ship_sprite`] resolves the base and
    /// [`GameData::ship_layers`] each layer: what a renderer asks of one
    /// image key. An image the `shän` does not load is
    /// [`ShipImageError::Undefined`]. The sheet is decoded on every call
    /// and not cached.
    pub fn ship_image(
        &self,
        ship: ShipId,
        image: ShipImage,
    ) -> Result<LayerSprite<'_>, ShipImageError> {
        let Some(layer) = image.layer() else {
            let sprite = self.ship_sprite(ship).map_err(ShipImageError::Sprite)?;
            return Ok(LayerSprite {
                image_id: sprite.image_id,
                kind: sprite.kind,
                sheet: sprite.sheet,
                source: sprite.sheet_source,
            });
        };
        let (_, anim) = self.ship_and_anim(ship).map_err(ShipImageError::Sprite)?;
        if !image.defined(anim.record) {
            return Err(ShipImageError::Undefined {
                ship: ship.0,
                image,
            });
        }
        self.layer_sprite(ship.0, layer, anim.record)
            .map_err(ShipImageError::Layer)
    }

    /// Reads `anim`'s `image` as `S` (see the module docs): its `rlëD`
    /// with the `shän`'s layout, or with none its `PICT` pair.
    pub(super) fn read_ship_image<S: ReadSheet>(
        &self,
        anim: &ShipAnim,
        image: ShipImage,
    ) -> Result<ImageRead<'_, S>, ImageFault> {
        let fields = image.fields(anim);
        if let Some(found) = self.resource(RLED, fields.image_id) {
            let sheet = S::from_rled(found.resource.data(), anim.sheet_layout())
                .map_err(ImageFault::Graphics)?;
            return Ok(ImageRead {
                kind: SheetKind::Rled,
                sheet,
                source: found.source,
            });
        }
        let found = self
            .resource(PICT, fields.image_id)
            .ok_or(ImageFault::NoSheet)?;
        let cut = PairCut {
            x_size: fields.x_size,
            y_size: fields.y_size,
            limit: image.frame_limit(anim),
            layout: anim.sheet_layout(),
        };
        let (mask_id, sheet) = self
            .cut_pict_pair(
                fields.image_id,
                found.resource.data(),
                Some(fields.mask_id),
                cut,
            )
            .map_err(|error| match error {
                PairError::NoMask => ImageFault::NoMask {
                    mask_id: fields.mask_id,
                },
                PairError::Picture { pict_id, source } => ImageFault::Picture { pict_id, source },
            })?;
        Ok(ImageRead {
            kind: SheetKind::PictPair { mask_id },
            sheet: S::from_sheet(sheet),
            source: found.source,
        })
    }
}

impl From<ShipLayer> for ShipImage {
    fn from(layer: ShipLayer) -> Self {
        match layer {
            ShipLayer::Glow => Self::Glow,
            ShipLayer::Lights => Self::Lights,
            ShipLayer::Weapon => Self::Weapon,
            ShipLayer::Alt => Self::Alt,
            ShipLayer::Shield => Self::Shield,
        }
    }
}

impl fmt::Display for ShipImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.layer() {
            Some(layer) => layer.fmt(f),
            None => f.write_str("base"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_bytes;
    use crate::records::fixture::ShipAnimBuilder;

    fn decoded(builder: ShipAnimBuilder) -> ShipAnim {
        decode_bytes::<ShipAnim>(&builder.build())
            .expect("decodes")
            .record
    }

    /// A `shän` whose every image field holds its own value: image `n`'s
    /// fields are 10n + 1 to 10n + 4.
    fn every_field() -> ShipAnim {
        decoded(
            ShipAnimBuilder::new()
                .base_image_id(1)
                .base_mask_id(2)
                .base_x_size(3)
                .base_y_size(4)
                .alt_image_id(11)
                .alt_mask_id(12)
                .alt_x_size(13)
                .alt_y_size(14)
                .glow_image_id(21)
                .glow_mask_id(22)
                .glow_x_size(23)
                .glow_y_size(24)
                .light_image_id(31)
                .light_mask_id(32)
                .light_x_size(33)
                .light_y_size(34)
                .weap_image_id(41)
                .weap_mask_id(42)
                .weap_x_size(43)
                .weap_y_size(44)
                .shield_image_id(51)
                .shield_mask_id(52)
                .shield_x_size(53)
                .shield_y_size(54),
        )
    }

    #[test]
    fn each_image_reads_its_own_fields() {
        let anim = every_field();
        for (n, image) in (0..).zip(ShipImage::ALL) {
            assert_eq!(
                image.fields(&anim),
                ShipImageFields {
                    image_id: 10 * n + 1,
                    mask_id: 10 * n + 2,
                    x_size: 10 * n + 3,
                    y_size: 10 * n + 4,
                },
                "{image}"
            );
        }
    }

    /// A `shän` of `frames_per` frames per rotation, `sets` base sets and
    /// `alt_sets` alt sets.
    fn counts(frames_per: i16, sets: i16, alt_sets: i16) -> ShipAnim {
        decoded(
            ShipAnimBuilder::new()
                .frames_per(frames_per)
                .base_set_count(sets)
                .alt_set_count(alt_sets),
        )
    }

    #[test]
    fn the_base_glow_lights_and_weapon_hold_frames_per_by_base_sets() {
        let anim = counts(8, 3, 5);
        for image in [
            ShipImage::Base,
            ShipImage::Glow,
            ShipImage::Lights,
            ShipImage::Weapon,
        ] {
            assert_eq!(image.frame_limit(&anim), FrameLimit::Exactly(24), "{image}");
        }
    }

    #[test]
    fn the_alt_holds_frames_per_by_alt_sets_and_the_shield_any_grid() {
        let anim = counts(8, 3, 5);
        assert_eq!(ShipImage::Alt.frame_limit(&anim), FrameLimit::Exactly(40));
        assert_eq!(ShipImage::Shield.frame_limit(&anim), FrameLimit::WholeGrid);
    }

    #[test]
    fn frames_per_0_is_36_and_base_sets_of_0_or_less_are_1() {
        for sets in [0, -1] {
            let anim = counts(0, sets, 2);
            assert_eq!(
                ShipImage::Base.frame_limit(&anim),
                FrameLimit::Exactly(36),
                "{sets}"
            );
            assert_eq!(ShipImage::Alt.frame_limit(&anim), FrameLimit::Exactly(72));
        }
        // Only 0 is replaced.
        let anim = counts(-2, 1, 1);
        assert_eq!(ShipImage::Base.frame_limit(&anim), FrameLimit::Exactly(-2));
    }

    #[test]
    fn the_counts_multiply_wider_than_16_bits() {
        let anim = counts(300, 300, 300);
        assert_eq!(
            ShipImage::Weapon.frame_limit(&anim),
            FrameLimit::Exactly(90_000)
        );
        assert_eq!(
            ShipImage::Alt.frame_limit(&anim),
            FrameLimit::Exactly(90_000)
        );
    }

    #[test]
    fn the_base_is_always_loaded_and_the_others_by_their_ids() {
        let none = decoded(
            ShipAnimBuilder::new()
                .base_image_id(-1)
                .alt_image_id(-1)
                .glow_image_id(0)
                .light_image_id(-1)
                .weap_image_id(0)
                .shield_image_id(-1)
                .alt_set_count(2),
        );
        assert!(ShipImage::Base.defined(&none));
        for image in &ShipImage::ALL[1..] {
            assert!(!image.defined(&none), "{image}");
        }
        let all = decoded(
            ShipAnimBuilder::new()
                .alt_set_count(1)
                .base_image_id(1)
                .alt_image_id(1)
                .glow_image_id(1)
                .light_image_id(1)
                .weap_image_id(1)
                .shield_image_id(1),
        );
        for image in ShipImage::ALL {
            assert!(image.defined(&all), "{image}");
        }
    }

    #[test]
    fn the_alt_needs_alt_sets_too() {
        for sets in [0, -1] {
            let anim = decoded(ShipAnimBuilder::new().alt_image_id(5).alt_set_count(sets));
            assert!(!ShipImage::Alt.defined(&anim), "{sets}");
        }
    }

    #[test]
    fn each_layer_is_its_image_and_the_base_none() {
        assert_eq!(ShipImage::Base.layer(), None);
        for layer in [
            ShipLayer::Glow,
            ShipLayer::Lights,
            ShipLayer::Weapon,
            ShipLayer::Alt,
            ShipLayer::Shield,
        ] {
            let image = ShipImage::from(layer);
            assert_eq!(image.layer(), Some(layer));
            assert_eq!(image.to_string(), layer.to_string());
        }
        assert_eq!(ShipImage::Base.to_string(), "base");
        let images: Vec<_> = ShipImage::ALL.iter().filter_map(|i| i.layer()).collect();
        assert_eq!(images.len(), 5);
    }

    mod resolving {
        use nova_rsrc::ResType;

        use super::super::*;
        use crate::decode::Record;
        use crate::graphics::fixture::{RledBuilder, rgb888_pict};
        use crate::graphics::{PICT, RleWords, SpriteSheet, sheet_rle_words};
        use crate::records::fixture::{ShipAnimBuilder, ShipBuilder};
        use crate::records::ship::Ship;
        use crate::store::fixture::store_of;
        use crate::store::{LayerError, SheetScan, SpriteError};
        use crate::wire::id::ShipId;

        const SHIP: ShipId = ShipId(128);
        const CLEAR: [u8; 4] = [0; 4];

        /// Pixel `i` of a [`distinct`] picture, distinct at five bits.
        fn nth(i: usize) -> [u8; 3] {
            [(i * 8) as u8, 0x40, 0x80]
        }

        /// The colour [`nth`]`(i)` draws as once cut.
        fn drawn(i: usize) -> [u8; 4] {
            [((i << 3) | (i >> 2)) as u8, 66, 132, 255]
        }

        /// A `width` x `height` `PICT` whose every pixel is distinct.
        fn distinct(width: i16, height: i16) -> Vec<u8> {
            let pixels: Vec<[u8; 3]> = (0..(width * height) as usize).map(nth).collect();
            rgb888_pict(width, height, &pixels)
        }

        /// A `width` x `height` white mask `PICT`, black at `black`.
        fn mask(width: i16, height: i16, black: &[(usize, usize)]) -> Vec<u8> {
            let mut pixels = vec![[0xFF; 3]; (width * height) as usize];
            for &(x, y) in black {
                pixels[y * width as usize + x] = [0; 3];
            }
            rgb888_pict(width, height, &pixels)
        }

        /// A `shän` of 2 frames per rotation and 2 base sets, so 4 frames
        /// to an image, and 1 alt set.
        fn anim() -> ShipAnimBuilder {
            ShipAnimBuilder::new()
                .frames_per(2)
                .base_set_count(2)
                .alt_set_count(1)
        }

        /// Ship 128 with `anim` as its `shän`, and `resources`.
        fn ship_with(anim: ShipAnimBuilder, resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
            let mut all = vec![
                (Ship::TYPE, 128, ShipBuilder::new().build()),
                (ShipAnim::TYPE, 128, anim.build()),
            ];
            all.extend_from_slice(resources);
            store_of(&all)
        }

        /// A base of sprites `PICT` 2000 and mask 2100, 2x1 frames.
        fn base(anim: ShipAnimBuilder) -> ShipAnimBuilder {
            anim.base_image_id(2000)
                .base_mask_id(2100)
                .base_x_size(2)
                .base_y_size(1)
        }

        #[test]
        fn a_base_with_no_rled_resolves_to_its_pict_pair() {
            let data = ship_with(
                base(anim()),
                &[
                    (PICT, 2000, distinct(4, 2)),
                    (PICT, 2100, mask(4, 2, &[(1, 0)])),
                ],
            );
            let sprite = data.ship_sprite(SHIP).expect("resolves");
            assert_eq!(sprite.image_id, 2000);
            assert_eq!(sprite.kind, SheetKind::PictPair { mask_id: 2100 });
            let sheet = &sprite.sheet;
            assert_eq!(sheet.frames().len(), 4);
            assert_eq!((sheet.frame_width(), sheet.frame_height()), (2, 1));
            assert_eq!(sheet.layout().columns(), 2);
            assert_eq!(sheet.depth(), 16);
            // Black in the mask hides the sprite; white shows its colour.
            assert_eq!(sheet.frames()[0].pixel(0, 0), Some(drawn(0)));
            assert_eq!(sheet.frames()[0].pixel(1, 0), Some(CLEAR));
            // Frame 3 is the picture's cell at (2, 1).
            assert_eq!(sheet.frames()[3].pixel(0, 0), Some(drawn(6)));
            assert_eq!(sheet.frames()[3].pixel(1, 0), Some(drawn(7)));
        }

        #[test]
        fn a_base_masked_by_its_own_sprites_hides_their_white() {
            let mut pixels: Vec<[u8; 3]> = (0..8).map(nth).collect();
            pixels[1] = [0xFF; 3];
            let data = ship_with(
                base(anim()).base_mask_id(2000),
                &[(PICT, 2000, rgb888_pict(4, 2, &pixels))],
            );
            let sprite = data.ship_sprite(SHIP).expect("resolves");
            assert_eq!(sprite.kind, SheetKind::PictPair { mask_id: 2000 });
            assert_eq!(sprite.sheet.frames()[0].pixel(0, 0), Some(drawn(0)));
            assert_eq!(sprite.sheet.frames()[0].pixel(1, 0), Some(CLEAR));
        }

        /// One image's pair: (image, sprites ID, mask ID, frame size,
        /// picture size, frames).
        type Pair = (ShipImage, i16, i16, (i16, i16), (i16, i16), usize);

        /// Every image a pair of its own.
        const PAIRS: [Pair; 6] = [
            (ShipImage::Base, 2000, 2100, (2, 1), (4, 2), 4),
            (ShipImage::Alt, 2001, 2101, (1, 2), (2, 2), 2),
            (ShipImage::Glow, 2002, 2002, (3, 1), (6, 2), 4),
            (ShipImage::Lights, 2003, 2103, (1, 1), (2, 2), 4),
            (ShipImage::Weapon, 2004, 2104, (2, 2), (4, 4), 4),
            (ShipImage::Shield, 2005, 2105, (1, 3), (3, 3), 3),
        ];

        /// A `shän` naming every image of [`PAIRS`], and their `PICT`s.
        fn every_pair() -> (ShipAnimBuilder, Vec<(ResType, i16, Vec<u8>)>) {
            let anim = base(anim())
                .alt_image_id(2001)
                .alt_mask_id(2101)
                .alt_x_size(1)
                .alt_y_size(2)
                .glow_image_id(2002)
                .glow_mask_id(2002)
                .glow_x_size(3)
                .glow_y_size(1)
                .light_image_id(2003)
                .light_mask_id(2103)
                .light_x_size(1)
                .light_y_size(1)
                .weap_image_id(2004)
                .weap_mask_id(2104)
                .weap_x_size(2)
                .weap_y_size(2)
                .shield_image_id(2005)
                .shield_mask_id(2105)
                .shield_x_size(1)
                .shield_y_size(3);
            let mut picts = Vec::new();
            for (_, image, mask_id, _, (w, h), _) in PAIRS {
                picts.push((PICT, image, distinct(w, h)));
                if mask_id != image {
                    picts.push((PICT, mask_id, mask(w, h, &[])));
                }
            }
            (anim, picts)
        }

        #[test]
        fn each_image_with_no_rled_resolves_to_its_own_pair() {
            let (anim, picts) = every_pair();
            let data = ship_with(anim, &picts);
            for (image, image_id, mask_id, (w, h), _, frames) in PAIRS {
                let sprite = data.ship_image(SHIP, image).expect("resolves");
                assert_eq!(sprite.image_id, image_id, "{image}");
                assert_eq!(sprite.kind, SheetKind::PictPair { mask_id }, "{image}");
                assert_eq!(sprite.sheet.frames().len(), frames, "{image}");
                let size = (sprite.sheet.frame_width(), sprite.sheet.frame_height());
                assert_eq!(size, (w as u32, h as u32), "{image}");
                assert_eq!(sprite.sheet.layout().columns(), 2, "{image}");
            }
            let layers = data.ship_layers(SHIP).expect("resolves");
            for (layer, frames) in [
                (layers.alt, 2),
                (layers.glow, 4),
                (layers.lights, 4),
                (layers.weapon, 4),
                (layers.shield, 3),
            ] {
                let layer = layer.expect("defined").expect("resolves");
                assert!(matches!(layer.kind, SheetKind::PictPair { .. }));
                assert_eq!(layer.sheet.frames().len(), frames);
            }
        }

        /// An `rlëD` of `frames` 2x1 frames of `color`.
        fn rled(frames: u16, color: u16) -> Vec<u8> {
            let mut builder = RledBuilder::new(2, 1).count_lines();
            for _ in 0..frames {
                builder = builder.frame(|f| f.line().pixels(&[color, color]));
            }
            builder.build()
        }

        #[test]
        fn an_rled_wins_over_a_pair_and_a_broken_one_does_not_fall_back() {
            let pair = [(PICT, 2000, distinct(4, 2)), (PICT, 2100, mask(4, 2, &[]))];
            let mut with_rled = pair.to_vec();
            with_rled.push((crate::graphics::RLED, 2000, rled(4, 0x7C00)));
            let data = ship_with(base(anim()), &with_rled);
            let sprite = data.ship_sprite(SHIP).expect("resolves");
            assert_eq!(sprite.kind, SheetKind::Rled);
            assert_eq!(sprite.sheet.frames()[0].pixel(0, 0), Some([255, 0, 0, 255]));

            let mut broken = pair.to_vec();
            broken.push((
                crate::graphics::RLED,
                2000,
                RledBuilder::new(2, 1).depth(1).build(),
            ));
            let data = ship_with(base(anim()), &broken);
            assert_eq!(
                data.ship_sprite(SHIP).expect_err("broken"),
                SpriteError::Graphics {
                    image_id: 2000,
                    source: GraphicsError::UnsupportedDepth { depth: 1 },
                }
            );
        }

        #[test]
        fn a_pair_layer_and_an_rled_layer_resolve_on_their_own() {
            // The glow is an `rlëD`, the lights a pair, the weapon a broken
            // pair (no mask).
            let anim = base(anim())
                .glow_image_id(1100)
                .light_image_id(2003)
                .light_mask_id(2103)
                .light_x_size(1)
                .light_y_size(1)
                .weap_image_id(2004)
                .weap_mask_id(2104)
                .weap_x_size(1)
                .weap_y_size(1);
            let data = ship_with(
                anim,
                &[
                    (crate::graphics::RLED, 1000, rled(4, 0x7C00)),
                    (crate::graphics::RLED, 1100, rled(4, 0x7C00)),
                    (PICT, 2000, distinct(4, 2)),
                    (PICT, 2100, mask(4, 2, &[])),
                    (PICT, 2003, distinct(2, 2)),
                    (PICT, 2103, mask(2, 2, &[])),
                    (PICT, 2004, distinct(2, 2)),
                ],
            );
            let layers = data.ship_layers(SHIP).expect("resolves");
            let glow = layers.glow.expect("defined").expect("resolves");
            assert_eq!(glow.kind, SheetKind::Rled);
            let lights = layers.lights.expect("defined").expect("resolves");
            assert_eq!(lights.kind, SheetKind::PictPair { mask_id: 2103 });
            assert_eq!(
                layers.weapon.expect("defined").expect_err("no mask"),
                LayerError::NoMask {
                    ship: 128,
                    layer: ShipLayer::Weapon,
                    mask_id: 2104,
                }
            );
            assert!(data.ship_sprite(SHIP).is_ok(), "the base still resolves");
        }

        #[test]
        fn a_pair_with_no_mask_or_no_picture_is_its_error() {
            let data = ship_with(base(anim()), &[(PICT, 2000, distinct(4, 2))]);
            let err = data.ship_sprite(SHIP).expect_err("no mask");
            assert_eq!(
                err,
                SpriteError::NoMask {
                    ship: 128,
                    mask_id: 2100
                }
            );
            assert_eq!(
                err.to_string(),
                "shïp 128: no mask PICT 2100 for its base image"
            );
            let err = ship_with(base(anim()), &[])
                .ship_sprite(SHIP)
                .expect_err("none");
            assert_eq!(
                err,
                SpriteError::NoSheet {
                    ship: 128,
                    image_id: 2000
                }
            );
            let data = ship_with(
                base(anim()).glow_image_id(2002).glow_mask_id(2102),
                &[(PICT, 2002, distinct(4, 2))],
            );
            let glow = data.ship_layers(SHIP).expect("resolves").glow;
            let err = glow.expect("defined").expect_err("no mask");
            assert_eq!(
                err.to_string(),
                "shïp 128: no mask PICT 2102 for its glow image"
            );
        }

        #[test]
        fn a_pair_of_the_wrong_count_or_a_partial_grid_is_a_picture_error() {
            // 6 frames, not 4.
            let data = ship_with(
                base(anim()),
                &[(PICT, 2000, distinct(6, 2)), (PICT, 2100, mask(6, 2, &[]))],
            );
            let err = data.ship_sprite(SHIP).expect_err("6 frames");
            assert_eq!(
                err,
                SpriteError::Picture {
                    pict_id: 2000,
                    source: GraphicsError::FrameCountMismatch {
                        frames: 6,
                        expected: 4
                    },
                }
            );
            assert_eq!(
                err.to_string(),
                "PICT 2000: the picture holds 6 frames, not 4"
            );
            // A partial grid, on a layer.
            let data = ship_with(
                base(anim())
                    .light_image_id(2003)
                    .light_mask_id(2003)
                    .light_x_size(2)
                    .light_y_size(1),
                &[(PICT, 2003, distinct(5, 2))],
            );
            let lights = data.ship_layers(SHIP).expect("resolves").lights;
            let err = lights.expect("defined").expect_err("partial");
            assert!(
                matches!(
                    err,
                    LayerError::Picture {
                        layer: ShipLayer::Lights,
                        pict_id: 2003,
                        source: GraphicsError::PartialGrid { .. },
                    }
                ),
                "{err:?}"
            );
            assert!(err.to_string().starts_with("lights PICT 2003: "), "{err}");
            // A mask that does not decode is reported against itself.
            let data = ship_with(
                base(anim()),
                &[(PICT, 2000, distinct(4, 2)), (PICT, 2100, vec![0; 4])],
            );
            assert!(matches!(
                data.ship_sprite(SHIP),
                Err(SpriteError::Picture { pict_id: 2100, .. })
            ));
        }

        #[test]
        fn a_pairs_scan_and_words_agree_with_its_cut_sheet() {
            let (anim, picts) = every_pair();
            let data = ship_with(anim, &picts);
            let sprite = data.ship_sprite(SHIP).expect("resolves");
            let info = data.ship_sprite_info(SHIP).expect("resolves");
            assert_eq!(info.sheet, SheetScan::of(&sprite.sheet));
            assert_eq!(info.kind, sprite.kind);
            assert_eq!(info.sheet.info.frames, 4);
            assert_eq!(info.sheet.info.depth, 16);
            assert!(!info.sheet.info.has_runs && !info.sheet.info.clip_quirks);
            assert_eq!(
                (info.sheet.info.image_width, info.sheet.info.image_height),
                (2, 1)
            );
            let words = data.ship_sprite_words(SHIP).expect("resolves");
            let expected: RleWords = sheet_rle_words(&sprite.sheet, 16);
            assert_eq!(words.sheet, expected);
            assert_eq!(words.image_id, 2000);
            let layers = data.ship_layers(SHIP).expect("resolves");
            let infos = data.ship_layer_infos(SHIP).expect("resolves");
            let scan = |l: Option<Result<crate::store::LayerSprite<'_>, LayerError>>| {
                SheetScan::of(&l.expect("defined").expect("resolves").sheet)
            };
            let info = |l: Option<Result<crate::store::LayerSprite<'_, SheetScan>, LayerError>>| {
                l.expect("defined").expect("resolves").sheet
            };
            assert_eq!(info(infos.shield), scan(layers.shield));
            assert_eq!(info(infos.alt), scan(layers.alt));
        }

        #[test]
        fn a_single_image_is_what_the_sprite_and_layers_resolve() {
            let (anim, picts) = every_pair();
            let data = ship_with(anim, &picts);
            let frames = |sheet: &SpriteSheet| sheet.frames().to_vec();
            let base = data.ship_image(SHIP, ShipImage::Base).expect("resolves");
            let sprite = data.ship_sprite(SHIP).expect("resolves");
            assert_eq!(
                (base.image_id, base.kind, frames(&base.sheet)),
                (sprite.image_id, sprite.kind, frames(&sprite.sheet))
            );
            assert_eq!(base.source.path, sprite.sheet_source.path);
            let layers = data.ship_layers(SHIP).expect("resolves");
            for (image, layer) in [
                (ShipImage::Alt, layers.alt),
                (ShipImage::Glow, layers.glow),
                (ShipImage::Lights, layers.lights),
                (ShipImage::Weapon, layers.weapon),
                (ShipImage::Shield, layers.shield),
            ] {
                let one = data.ship_image(SHIP, image).expect("resolves");
                let layer = layer.expect("defined").expect("resolves");
                assert_eq!(
                    (one.image_id, one.kind, frames(&one.sheet)),
                    (layer.image_id, layer.kind, frames(&layer.sheet)),
                    "{image}"
                );
            }
        }

        #[test]
        fn a_single_images_errors_say_what_is_missing_or_broken() {
            let data = ship_with(base(anim()), &[(PICT, 2000, distinct(6, 2))]);
            let undefined = data.ship_image(SHIP, ShipImage::Glow).expect_err("none");
            assert_eq!(
                undefined,
                ShipImageError::Undefined {
                    ship: 128,
                    image: ShipImage::Glow
                }
            );
            assert_eq!(undefined.to_string(), "shïp 128: no glow image");
            assert_eq!(undefined.graphics(), None);
            let no_ship = data
                .ship_image(ShipId(129), ShipImage::Glow)
                .expect_err("none");
            assert_eq!(no_ship, ShipImageError::Sprite(SpriteError::NoShip(129)));
            assert_eq!(no_ship.to_string(), "no shïp 129");
            assert_eq!(no_ship.graphics(), None);
            let no_mask = data.ship_image(SHIP, ShipImage::Base).expect_err("no mask");
            assert_eq!(no_mask.graphics(), None);
            let data = ship_with(
                base(anim()).base_mask_id(2000).shield_image_id(1500),
                &[
                    (PICT, 2000, distinct(6, 2)),
                    (
                        crate::graphics::RLED,
                        1500,
                        RledBuilder::new(2, 1).depth(1).build(),
                    ),
                ],
            );
            let count = data.ship_image(SHIP, ShipImage::Base).expect_err("count");
            assert_eq!(
                count.graphics(),
                Some(&GraphicsError::FrameCountMismatch {
                    frames: 6,
                    expected: 4
                })
            );
            let shield = data.ship_image(SHIP, ShipImage::Shield).expect_err("depth");
            assert_eq!(
                shield.graphics(),
                Some(&GraphicsError::UnsupportedDepth { depth: 1 })
            );
            assert_eq!(
                shield.to_string(),
                "shield rlëD 1500: unsupported rlëD depth 1"
            );
        }
    }
}
