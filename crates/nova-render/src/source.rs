//! Images from the game data: `PICT`s, masked `PICT`s, `rlëD` sprite
//! sheets (whole, their solid runs, or as the clipped blit draws them at
//! the world's edge, [`ImageKind::RledClipped`], and their solid runs so,
//! [`ImageKind::RledClippedSolid`]), the sheets `spïn`s and a `shän`'s
//! images resolve to (a `PICT` pair cut into frames),
//! `ppat` pixel patterns and `cicn` colour icons, decoded on request; and,
//! when it is given one, from the interface file (`Nova-DF.rsrc` or
//! `Nova.rez`): its `PICT`s, as [`ImageKind::InterfacePict`].
//!
//! The two files never answer for each other. An interface picture is
//! looked up only in the interface file, so a game-data `PICT` with the
//! same ID never stands in for it, and [`ImageKind::Pict`] never reads the
//! interface file.

use std::borrow::Borrow;
use std::rc::Rc;

use nova_data::graphics::{
    CICN, ClipCut, DeepImage, EightBitReading, Image, PICT, PPAT, PixelDepth, RLED, RledReadings,
    ThirtyTwoBitReading, apply_mask, decode_cicn, decode_pict, decode_pict_deep, decode_ppat,
    decode_rled_clipped, decode_rled_with,
};
use nova_data::{GameData, InterfaceData, PictId, SheetKind, ShipId, ShipImage, SpinSpriteError};
use nova_view::ImageKind;

use crate::images::{ImageError, ImageSource};

/// The game data as an image source, decoding an 8-bit `rlëD` by
/// `eight_bit` (the rulebook's `eight_bit_sheet` reading) and a 32-bit one
/// by the reading given to
/// [`with_thirty_two_bit`](Self::with_thirty_two_bit) (the
/// `thirty_two_bit_sheet` reading), by default as the original draws it.
/// Either 32-bit reading draws a sheet the game data resolves
/// ([`GameData::spin_sheet`]) at the size it resolves, and fails one it
/// fails: the reading changes only the colours.
///
/// It looks resources up in the store (later files winning) and decodes
/// them on every call; the renderer keeps what it needs in the atlas. A
/// masked picture is its picture with the mask picture applied
/// ([`apply_mask`]); either one missing is [`ImageError::Missing`]. An
/// interface picture comes from the interface file given to
/// [`with_interface`](Self::with_interface); without one, or without that
/// `PICT` in it, it is [`ImageError::Missing`].
#[derive(Clone, Debug)]
pub struct GameImages<D> {
    data: D,
    readings: RledReadings,
    interface: Option<Rc<InterfaceData>>,
}

impl<D: Borrow<GameData>> GameImages<D> {
    /// `data`'s images, with 8-bit sheets read as `eight_bit` says, 32-bit
    /// sheets as the original draws them, and no interface file.
    pub fn new(data: D, eight_bit: EightBitReading) -> Self {
        Self {
            data,
            readings: RledReadings {
                eight_bit,
                ..RledReadings::default()
            },
            interface: None,
        }
    }

    /// These images, with 32-bit sheets read as `thirty_two_bit` says.
    #[must_use]
    pub fn with_thirty_two_bit(self, thirty_two_bit: ThirtyTwoBitReading) -> Self {
        Self {
            readings: RledReadings {
                thirty_two_bit,
                ..self.readings
            },
            ..self
        }
    }

    /// These images, with the interface file's pictures resolved from
    /// `interface`.
    #[must_use]
    pub fn with_interface(self, interface: Rc<InterfaceData>) -> Self {
        Self {
            interface: Some(interface),
            ..self
        }
    }

    /// The interface file's `PICT` `id`, decoded with its depth.
    fn interface_picture(&self, id: i16) -> Result<DeepImage, ImageError> {
        let interface = self.interface.as_ref().ok_or(ImageError::Missing)?;
        Ok(interface
            .picture_deep(PictId(id))
            .ok_or(ImageError::Missing)??)
    }

    /// The bytes of the store's `PICT` `id`.
    fn pict_bytes(&self, id: i16) -> Result<&[u8], ImageError> {
        self.data
            .borrow()
            .resource(PICT, id)
            .map(|found| found.resource.data())
            .ok_or(ImageError::Missing)
    }

    /// A picture's one frame, with its depth, for the kinds that decode a
    /// `PICT`: `None` for the others.
    fn deep_picture(&self, kind: ImageKind, id: i16) -> Option<Result<DeepImage, ImageError>> {
        let deep =
            |id| -> Result<DeepImage, ImageError> { Ok(decode_pict_deep(self.pict_bytes(id)?)?) };
        Some(match kind {
            ImageKind::Pict => deep(id),
            ImageKind::InterfacePict => self.interface_picture(id),
            ImageKind::MaskedPict { mask } => deep(id).and_then(|picture| {
                let mask = decode_pict(self.pict_bytes(mask)?)?;
                let masked = apply_mask(picture.image(), &mask)?;
                Ok(picture
                    .with_image(masked)
                    .expect("a mask keeps the picture's size"))
            }),
            ImageKind::Rled
            | ImageKind::RledSolid
            | ImageKind::RledClipped { .. }
            | ImageKind::RledClippedSolid { .. }
            | ImageKind::Pattern
            | ImageKind::Icon
            | ImageKind::SpinSheet
            | ImageKind::ShipSheet { .. } => return None,
        })
    }

    /// The frames `spïn` `id` resolves to ([`GameData::spin_sheet`]): an
    /// `rlëD`'s as [`ImageKind::Rled`] decodes them, or its `PICT` pair
    /// cut. A link that is not there is [`ImageError::Missing`], a sheet
    /// that does not decode or cut is [`ImageError::Decode`].
    fn spin_frames(&self, id: i16) -> Result<Vec<Image>, ImageError> {
        match self.data.borrow().spin_sheet(id) {
            Ok(spin) if spin.kind == SheetKind::Rled => self.frames(ImageKind::Rled, spin.image_id),
            Ok(spin) => Ok(spin.sheet.into_frames()),
            Err(
                SpinSpriteError::Graphics { source, .. } | SpinSpriteError::Picture { source, .. },
            ) => Err(ImageError::Decode(source)),
            Err(
                SpinSpriteError::NoSpin(_)
                | SpinSpriteError::Decode(_)
                | SpinSpriteError::NoSheet { .. }
                | SpinSpriteError::NoMask { .. },
            ) => Err(ImageError::Missing),
        }
    }

    /// The frames `shïp` `ship`'s `image` resolves to
    /// ([`GameData::ship_image`]): an `rlëD`'s as [`ImageKind::Rled`]
    /// decodes them, or its `PICT` pair cut. A link that is not there is
    /// [`ImageError::Missing`], a sheet that does not decode or cut is
    /// [`ImageError::Decode`].
    fn ship_frames(&self, ship: i16, image: ShipImage) -> Result<Vec<Image>, ImageError> {
        match self.data.borrow().ship_image(ShipId(ship), image) {
            Ok(sheet) if sheet.kind == SheetKind::Rled => {
                self.frames(ImageKind::Rled, sheet.image_id)
            }
            Ok(sheet) => Ok(sheet.sheet.into_frames()),
            Err(err) => Err(err.graphics().map_or(ImageError::Missing, |source| {
                ImageError::Decode(source.clone())
            })),
        }
    }
}

/// A picture (`Pict`, `MaskedPict`, `InterfacePict`) has its depth: the
/// one it was stored at ([`decode_pict_deep`]), the mask setting only its
/// alpha. Every other kind (`rlëD`s, `ppat`s, `cicn`s, and `spïn` and
/// ship sheets)
/// gives its frames at [`PixelDepth::Direct32`]: none goes through
/// `_DrawPicHandle`, and no stock site stretches one.
impl<D: Borrow<GameData>> ImageSource for GameImages<D> {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        if let Some(picture) = self.deep_picture(kind, id) {
            return Ok(vec![picture?.into_image()]);
        }
        let bytes = |ty, id| {
            self.data
                .borrow()
                .resource(ty, id)
                .map(|found| found.resource.data())
                .ok_or(ImageError::Missing)
        };
        let rled = |id| -> Result<_, ImageError> {
            Ok(decode_rled_with(bytes(RLED, id)?, None, self.readings)?)
        };
        Ok(match kind {
            ImageKind::Rled => rled(id)?.into_frames(),
            ImageKind::RledSolid => rled(id)?.into_solid_frames().ok_or(ImageError::Missing)?,
            ImageKind::RledClipped { top, left } => {
                let cut = ClipCut { top, left };
                decode_rled_clipped(bytes(RLED, id)?, None, self.readings, cut)?.into_frames()
            }
            ImageKind::RledClippedSolid { top, left } => {
                let cut = ClipCut { top, left };
                decode_rled_clipped(bytes(RLED, id)?, None, self.readings, cut)?
                    .into_solid_frames()
                    .ok_or(ImageError::Missing)?
            }
            ImageKind::Pattern => vec![decode_ppat(bytes(PPAT, id)?)?],
            ImageKind::Icon => vec![decode_cicn(bytes(CICN, id)?)?],
            ImageKind::SpinSheet => self.spin_frames(id)?,
            ImageKind::ShipSheet { image } => self.ship_frames(id, image)?,
            ImageKind::Pict | ImageKind::InterfacePict | ImageKind::MaskedPict { .. } => {
                unreachable!("a picture is decoded deep")
            }
        })
    }

    fn deep_frames(&self, kind: ImageKind, id: i16) -> Result<Vec<DeepImage>, ImageError> {
        match self.deep_picture(kind, id) {
            Some(picture) => Ok(vec![picture?]),
            None => Ok(self
                .frames(kind, id)?
                .into_iter()
                .filter_map(|frame| DeepImage::new(frame, PixelDepth::Direct32))
                .collect()),
        }
    }
}

/// The game data's images as the original draws them: [`GameImages`] with
/// 8-bit sheets [`EightBitReading::AsDrawn`], 32-bit sheets
/// [`ThirtyTwoBitReading::AsDrawn`] and no interface file, so an interface
/// picture is [`ImageError::Missing`].
impl ImageSource for GameData {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        GameImages::new(self, EightBitReading::AsDrawn).frames(kind, id)
    }

    fn deep_frames(&self, kind: ImageKind, id: i16) -> Result<Vec<DeepImage>, ImageError> {
        GameImages::new(self, EightBitReading::AsDrawn).deep_frames(kind, id)
    }
}

#[cfg(test)]
mod tests {

    use nova_data::Record;
    use nova_data::graphics::GraphicsError;
    use std::rc::Rc;

    use nova_data::ShipImage;
    use nova_data::graphics::PixelDepth;
    use nova_data::graphics::fixture::{
        BitMap, Cicn, Ctab, DirectBits, IndexedBits, PictBuilder, Ppat, RledBuilder, rgb888_pict,
    };
    use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
    use nova_data::records::ship::Ship;
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::records::spin::Spin;
    use nova_data::store::fixture::{interface_of, store_of};

    use nova_rsrc::ResType;

    use super::*;

    fn store(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
        store_of(resources)
    }

    /// A 3x2 picture: red, green, blue over white, black, grey.
    fn picture() -> Vec<u8> {
        let pixels = [
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [255, 255, 255],
            [0, 0, 0],
            [128, 128, 128],
        ];
        PictBuilder::new([0, 0, 2, 3])
            .direct_bits(&DirectBits::rgb888([0, 0, 2, 3], &pixels))
            .end()
            .build()
    }

    /// Two 1x1 frames: red, then blue. The depth is set because the pixels
    /// are 16-bit words, rather than left to the builder's default.
    fn sheet() -> Vec<u8> {
        RledBuilder::new(1, 1)
            .depth(16)
            .frame(|f| f.line().pixels(&[0x7C00]))
            .frame(|f| f.line().pixels(&[0x001F]))
            .build()
    }

    #[test]
    fn a_picture_is_one_frame_with_its_pixels() {
        let data = store(&[(PICT, 128, picture())]);
        let frames = data.frames(ImageKind::Pict, 128).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!((frames[0].width(), frames[0].height()), (3, 2));
        assert_eq!(frames[0].pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(frames[0].pixel(2, 1), Some([128, 128, 128, 255]));
    }

    #[test]
    fn a_sprite_sheet_is_all_its_frames_in_order() {
        let data = store(&[(RLED, 200, sheet())]);
        let frames = data.frames(ImageKind::Rled, 200).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(frames[1].pixel(0, 0), Some([0, 0, 255, 255]));
    }

    #[test]
    fn a_sprite_sheets_solid_runs_are_its_run_frames_or_missing() {
        let runs = RledBuilder::new(2, 1)
            .depth(16)
            .frame(|f| f.line().pixels(&[0x7C00]).run(1, 0x7FFF, 0x7FFF))
            .frame(|f| f.line().pixels(&[0x001F]))
            .build();
        let data = store(&[(RLED, 200, runs), (RLED, 201, sheet())]);
        let frames = data.frames(ImageKind::RledSolid, 200).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].pixel(0, 0), Some([0, 0, 0, 0]));
        assert_eq!(frames[0].pixel(1, 0), Some([255, 255, 255, 255]));
        assert_eq!(frames[1].pixel(0, 0), Some([0, 0, 0, 0]));
        assert_eq!(
            data.frames(ImageKind::RledSolid, 201),
            Err(ImageError::Missing),
            "no runs"
        );
        assert_eq!(
            data.frames(ImageKind::RledSolid, 202),
            Err(ImageError::Missing),
            "no sheet"
        );
    }

    /// A 2x2 pattern: red, green over blue, red.
    fn pattern() -> Vec<u8> {
        let ctab = Ctab {
            device: false,
            entries: vec![
                (0, [0xFFFF, 0, 0]),
                (1, [0, 0xFFFF, 0]),
                (2, [0, 0, 0xFFFF]),
            ],
        };
        Ppat::new(2, 2, 8, &ctab, &[0, 1, 2, 0]).bytes()
    }

    #[test]
    fn a_pattern_is_one_frame_with_its_pixels() {
        let data = store(&[(PPAT, 128, pattern())]);
        let frames = data.frames(ImageKind::Pattern, 128).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!((frames[0].width(), frames[0].height()), (2, 2));
        assert_eq!(frames[0].pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(frames[0].pixel(1, 0), Some([0, 255, 0, 255]));
        assert_eq!(frames[0].pixel(0, 1), Some([0, 0, 255, 255]));
        assert_eq!(
            data.frames(ImageKind::Pattern, 129),
            Err(ImageError::Missing)
        );
        assert_eq!(data.frames(ImageKind::Pict, 128), Err(ImageError::Missing));
    }

    /// A 2x1 icon: red, then green masked out.
    fn icon() -> Vec<u8> {
        let ctab = Ctab {
            device: false,
            entries: vec![(0, [0xFFFF, 0, 0]), (1, [0, 0xFFFF, 0])],
        };
        Cicn::new(2, 1, 8, &ctab, &[0, 1], &[true, false]).bytes()
    }

    #[test]
    fn an_icon_is_one_frame_with_its_mask_as_alpha() {
        let data = store(&[(CICN, 18000, icon())]);
        let frames = data.frames(ImageKind::Icon, 18000).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!((frames[0].width(), frames[0].height()), (2, 1));
        assert_eq!(frames[0].pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(frames[0].pixel(1, 0), Some([0, 0, 0, 0]));
        assert_eq!(
            data.frames(ImageKind::Icon, 18001),
            Err(ImageError::Missing)
        );
        assert_eq!(
            data.frames(ImageKind::Pict, 18000),
            Err(ImageError::Missing)
        );
    }

    #[test]
    fn a_missing_id_or_kind_is_missing() {
        let data = store(&[(PICT, 128, picture()), (RLED, 200, sheet())]);
        assert_eq!(data.frames(ImageKind::Pict, 129), Err(ImageError::Missing));
        assert_eq!(data.frames(ImageKind::Rled, 128), Err(ImageError::Missing));
        assert_eq!(data.frames(ImageKind::Pict, 200), Err(ImageError::Missing));
    }

    /// A 3x2 mask: black, white, black over white, black, white.
    fn mask() -> Vec<u8> {
        let (black, white) = ([0, 0, 0], [255, 255, 255]);
        let pixels = [black, white, black, white, black, white];
        PictBuilder::new([0, 0, 2, 3])
            .direct_bits(&DirectBits::rgb888([0, 0, 2, 3], &pixels))
            .end()
            .build()
    }

    #[test]
    fn a_masked_picture_is_clear_where_its_mask_is_white() {
        let data = store(&[(PICT, 128, picture()), (PICT, 228, mask())]);
        let frames = data
            .frames(ImageKind::MaskedPict { mask: 228 }, 128)
            .unwrap();
        assert_eq!(frames.len(), 1);
        let masked = &frames[0];
        assert_eq!((masked.width(), masked.height()), (3, 2));
        assert_eq!(masked.pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(masked.pixel(1, 0), Some([0, 0, 0, 0]));
        assert_eq!(masked.pixel(2, 0), Some([0, 0, 255, 255]));
        assert_eq!(masked.pixel(0, 1), Some([0, 0, 0, 0]));
        assert_eq!(masked.pixel(1, 1), Some([0, 0, 0, 255]));
        assert_eq!(masked.pixel(2, 1), Some([0, 0, 0, 0]));
    }

    #[test]
    fn the_plain_picture_of_a_masked_one_still_draws_unmasked() {
        let data = store(&[(PICT, 128, picture()), (PICT, 228, mask())]);
        let plain = data.frames(ImageKind::Pict, 128).unwrap();
        assert_eq!(plain[0].pixel(1, 0), Some([0, 255, 0, 255]));
    }

    #[test]
    fn a_missing_picture_or_mask_is_missing() {
        let data = store(&[(PICT, 128, picture()), (PICT, 228, mask())]);
        assert_eq!(
            data.frames(ImageKind::MaskedPict { mask: 229 }, 128),
            Err(ImageError::Missing)
        );
        assert_eq!(
            data.frames(ImageKind::MaskedPict { mask: 228 }, 129),
            Err(ImageError::Missing)
        );
        let sprite_only = store(&[(RLED, 128, sheet()), (RLED, 228, sheet())]);
        assert_eq!(
            sprite_only.frames(ImageKind::MaskedPict { mask: 228 }, 128),
            Err(ImageError::Missing)
        );
    }

    #[test]
    fn a_bad_picture_or_mask_or_a_mismatched_mask_is_a_decode_error() {
        let small = PictBuilder::new([0, 0, 1, 1])
            .direct_bits(&DirectBits::rgb888([0, 0, 1, 1], &[[0, 0, 0]]))
            .end()
            .build();
        let data = store(&[
            (PICT, 128, picture()),
            (PICT, 228, mask()),
            (PICT, 1, vec![0; 3]),
            (PICT, 2, small),
        ]);
        let end = ImageError::Decode(GraphicsError::UnexpectedEnd { offset: 2 });
        assert_eq!(
            data.frames(ImageKind::MaskedPict { mask: 228 }, 1),
            Err(end.clone())
        );
        assert_eq!(
            data.frames(ImageKind::MaskedPict { mask: 1 }, 128),
            Err(end)
        );
        assert_eq!(
            data.frames(ImageKind::MaskedPict { mask: 2 }, 128),
            Err(ImageError::Decode(GraphicsError::MaskSizeMismatch {
                width: 3,
                height: 2,
                mask_width: 1,
                mask_height: 1,
            }))
        );
    }

    /// A `spïn` naming sprites `sprites` and masks `masks`, 1x1 frames.
    fn spin(sprites: i16, masks: i16) -> Vec<u8> {
        [sprites, masks, 1, 1, 3, 1]
            .iter()
            .flat_map(|v| v.to_be_bytes())
            .collect()
    }

    #[test]
    fn a_spin_sheet_is_the_frames_its_pict_pair_cuts() {
        let data = store(&[
            (Spin::TYPE, 3000, spin(2000, 2100)),
            (PICT, 2000, picture()),
            (PICT, 2100, rgb888_pict(3, 2, &[[255; 3]; 6])),
            (Spin::TYPE, 3001, spin(2001, 2100)),
            (Spin::TYPE, 3002, spin(2000, 2101)),
            (Spin::TYPE, 3003, spin(2003, 2003)),
            (PICT, 2003, rgb888_pict(2, 1, &[[0; 3]; 2])),
            (Spin::TYPE, 3004, spin(2000, -1)),
            (Spin::TYPE, 3005, vec![0; 3]),
        ]);
        let frames = data.frames(ImageKind::SpinSheet, 3000).unwrap();
        assert_eq!(frames.len(), 6, "3x2 1x1 frames");
        assert_eq!(frames[0].pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(frames[5].pixel(0, 0), Some([132, 132, 132, 255]));
        // The same spïn read as an rlëD is missing.
        assert_eq!(data.frames(ImageKind::Rled, 3000), Err(ImageError::Missing));
        for (spin, why) in [
            (3001, "no sheet"),
            (3002, "no mask"),
            (3004, "a mask of -1"),
            (3005, "a spïn that does not decode"),
            (3006, "no spïn"),
        ] {
            assert_eq!(
                data.frames(ImageKind::SpinSheet, spin),
                Err(ImageError::Missing),
                "{why}"
            );
        }
        // A sheet that cuts to no whole grid of 1x1 frames cannot happen;
        // 2x1 frames of a 3-wide picture can.
        let partial = store(&[
            (Spin::TYPE, 3000, {
                let mut bytes = spin(2000, 2000);
                bytes[4..6].copy_from_slice(&2i16.to_be_bytes());
                bytes
            }),
            (PICT, 2000, picture()),
        ]);
        assert_eq!(
            partial.frames(ImageKind::SpinSheet, 3000),
            Err(ImageError::Decode(GraphicsError::PartialGrid {
                width: 3,
                height: 2,
                frame_width: 2,
                frame_height: 1,
            }))
        );
        assert_eq!(
            data.frames(ImageKind::SpinSheet, 3003).map(|f| f.len()),
            Ok(2)
        );
    }

    /// Ship 128 of a `shän` of one set of 3 frames per rotation, base
    /// sprites `PICT` 2000 masked by 2100 in 1x1 frames, and glow `glow`:
    /// sprites `PICT` 2002 masked by itself, in 2x1 frames.
    fn ship_store(glow: i16, extra: &[(ResType, i16, Vec<u8>)]) -> GameData {
        let anim = ShipAnimBuilder::new()
            .frames_per(3)
            .base_set_count(1)
            .base_image_id(2000)
            .base_mask_id(2100)
            .base_x_size(1)
            .base_y_size(1)
            .glow_image_id(glow)
            .glow_mask_id(2002)
            .glow_x_size(2)
            .glow_y_size(1)
            .build();
        let mut resources = vec![
            (Ship::TYPE, 128, ShipBuilder::new().build()),
            (ShipAnim::TYPE, 128, anim),
        ];
        resources.extend_from_slice(extra);
        store(&resources)
    }

    /// A ship image's key kind.
    fn ship_sheet(image: ShipImage) -> ImageKind {
        ImageKind::ShipSheet { image }
    }

    #[test]
    fn a_ship_sheet_is_the_frames_its_pict_pair_cuts() {
        let data = ship_store(
            2002,
            &[
                (
                    PICT,
                    2000,
                    rgb888_pict(3, 1, &[[255, 0, 0], [0, 255, 0], [0, 0, 255]]),
                ),
                (PICT, 2100, rgb888_pict(3, 1, &[[255; 3]; 3])),
                (PICT, 2002, rgb888_pict(6, 1, &[[0; 3]; 6])),
            ],
        );
        let frames = data.frames(ship_sheet(ShipImage::Base), 128).unwrap();
        assert_eq!(frames.len(), 3, "3 frames of 1x1");
        assert_eq!(frames[0].pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(frames[2].pixel(0, 0), Some([0, 0, 255, 255]));
        // The glow is its own pair, of its own size.
        let glow = data.frames(ship_sheet(ShipImage::Glow), 128).unwrap();
        assert_eq!(glow.len(), 3);
        assert_eq!((glow[0].width(), glow[0].height()), (2, 1));
        // The base read as an `rlëD` is missing.
        assert_eq!(data.frames(ImageKind::Rled, 2000), Err(ImageError::Missing));
    }

    #[test]
    fn a_ship_sheet_of_an_rled_is_its_rled() {
        let data = ship_store(200, &[(RLED, 200, sheet())]);
        assert_eq!(
            data.frames(ship_sheet(ShipImage::Glow), 128),
            data.frames(ImageKind::Rled, 200)
        );
    }

    #[test]
    fn a_ship_sheet_with_a_missing_link_is_missing_and_a_bad_cut_a_decode_error() {
        let pair = [(PICT, 2000, rgb888_pict(3, 1, &[[0; 3]; 3]))];
        let data = ship_store(-1, &pair);
        for (image, ship, why) in [
            (ShipImage::Base, 128, "no mask"),
            (ShipImage::Glow, 128, "no glow"),
            (ShipImage::Base, 129, "no shïp"),
        ] {
            assert_eq!(
                data.frames(ship_sheet(image), ship),
                Err(ImageError::Missing),
                "{why}"
            );
        }
        // 4 frames, not 3.
        let data = ship_store(
            -1,
            &[
                (PICT, 2000, rgb888_pict(4, 1, &[[0; 3]; 4])),
                (PICT, 2100, rgb888_pict(4, 1, &[[255; 3]; 4])),
            ],
        );
        assert_eq!(
            data.frames(ship_sheet(ShipImage::Base), 128),
            Err(ImageError::Decode(GraphicsError::FrameCountMismatch {
                frames: 4,
                expected: 3
            }))
        );
    }

    #[test]
    fn a_spin_sheet_of_an_rled_is_its_rled() {
        let data = store(&[(Spin::TYPE, 3000, spin(200, -1)), (RLED, 200, sheet())]);
        assert_eq!(
            data.frames(ImageKind::SpinSheet, 3000),
            data.frames(ImageKind::Rled, 200)
        );
        let bad = store(&[(Spin::TYPE, 3000, spin(200, -1)), (RLED, 200, vec![0; 3])]);
        assert!(matches!(
            bad.frames(ImageKind::SpinSheet, 3000),
            Err(ImageError::Decode(_))
        ));
    }

    #[test]
    fn a_32_bit_sheet_is_drawn_as_the_reading_says() {
        // One 2x1 frame: a run of the file pixel 00 00 7C 00, then the
        // pixel 00 00 03 E0.
        let thirty_two = RledBuilder::new(2, 1)
            .depth(32)
            .count_lines()
            .frame(|f| f.line().run32(1, 0x0000_7C00).pixels32(&[0x0000_03E0]))
            .build();
        let data = store(&[(RLED, 200, thirty_two), (Spin::TYPE, 3000, spin(200, -1))]);
        let rgba = |frames: Vec<Image>| -> Vec<(u32, Vec<u8>)> {
            frames
                .into_iter()
                .map(|frame| (frame.width(), frame.into_pixels()))
                .collect()
        };
        let (red, green, black, clear) =
            ([255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 0, 255], [0; 4]);
        // As drawn, each pixel is two RGB555 pixels, little-endian, from
        // the box's left edge; the image is padded on the left to keep the
        // box at its centre.
        let as_drawn = vec![(6, [clear, clear, red, black, green, black].concat())];
        let as_drawn_solid = vec![(6, [clear, clear, red, black, clear, clear].concat())];
        // In true colour, each pixel is one, in the box of an image the
        // drawn one's size.
        let (true_red, true_green) = ([0, 0x7C, 0, 255], [0, 0x03, 0xE0, 255]);
        let true_colour = vec![(
            6,
            [clear, clear, true_red, true_green, clear, clear].concat(),
        )];
        let true_colour_solid = vec![(6, [clear, clear, true_red, clear, clear, clear].concat())];
        let drawn = GameImages::new(&data, EightBitReading::AsDrawn);
        let true_colour_images = drawn
            .clone()
            .with_thirty_two_bit(ThirtyTwoBitReading::TrueColour);
        for (images, frames, solid) in [
            (&drawn, &as_drawn, &as_drawn_solid),
            (&true_colour_images, &true_colour, &true_colour_solid),
        ] {
            assert_eq!(rgba(images.frames(ImageKind::Rled, 200).unwrap()), *frames);
            assert_eq!(
                rgba(images.frames(ImageKind::SpinSheet, 3000).unwrap()),
                *frames
            );
            assert_eq!(
                rgba(images.frames(ImageKind::RledSolid, 200).unwrap()),
                *solid
            );
        }
        assert_eq!(
            rgba(data.frames(ImageKind::Rled, 200).unwrap()),
            as_drawn,
            "the game data alone draws it as the original does"
        );
    }

    /// A clipped key ([`ImageKind::RledClipped`]) gives the sheet's frames
    /// as the clipped blit draws them, each its box: under the true-colour
    /// reading, its frames cut to the box.
    #[test]
    fn a_clipped_sprite_is_the_sheet_as_the_clipped_blit_draws_it() {
        let two_colour = RledBuilder::new(4, 1)
            .count_lines()
            .frame(|f| f.line().run(4, 0x7C00, 0x001F))
            .build();
        let thirty_two = RledBuilder::new(2, 1)
            .depth(32)
            .count_lines()
            .frame(|f| f.line().run32(1, 0x0000_7C00).pixels32(&[0x0000_03E0]))
            .build();
        let data = store(&[(RLED, 200, two_colour), (RLED, 201, thirty_two)]);
        let clipped = |top, left| ImageKind::RledClipped { top, left };
        let rgba = |frames: Vec<Image>| -> Vec<(u32, Vec<u8>)> {
            frames
                .into_iter()
                .map(|frame| (frame.width(), frame.into_pixels()))
                .collect()
        };
        let (red, blue, black, clear) =
            ([255, 0, 0, 255], [0, 0, 255, 255], [0, 0, 0, 255], [0; 4]);
        // The run restarts at the cut: blue (its low half) at pixel 1.
        assert_eq!(
            rgba(data.frames(clipped(0, 1), 200).unwrap()),
            [(4, [clear, blue, red, blue].concat())]
        );
        assert_eq!(
            rgba(data.frames(clipped(0, 0), 201).unwrap()),
            [(2, [red, black].concat())]
        );
        let true_colour = GameImages::new(&data, EightBitReading::AsDrawn)
            .with_thirty_two_bit(ThirtyTwoBitReading::TrueColour);
        assert_eq!(
            rgba(true_colour.frames(clipped(0, 1), 201).unwrap()),
            [(2, [clear, [0, 0x03, 0xE0, 255]].concat())]
        );
        assert_eq!(data.frames(clipped(0, 1), 202), Err(ImageError::Missing));
    }

    /// A clipped solid key ([`ImageKind::RledClippedSolid`]) gives the
    /// clipped draw's solid runs alone, and is missing for a sheet with
    /// none.
    #[test]
    fn a_clipped_sprites_solid_runs_are_the_clipped_draws_runs() {
        let two_colour = RledBuilder::new(4, 1)
            .count_lines()
            .frame(|f| f.line().run(4, 0x7C00, 0x001F))
            .build();
        let no_runs = RledBuilder::new(2, 1)
            .count_lines()
            .frame(|f| f.line().pixels(&[0x7C00, 0x001F]))
            .build();
        let data = store(&[(RLED, 200, two_colour), (RLED, 201, no_runs)]);
        let solid = |top, left| ImageKind::RledClippedSolid { top, left };
        let rgba = |frames: Vec<Image>| -> Vec<(u32, Vec<u8>)> {
            frames
                .into_iter()
                .map(|frame| (frame.width(), frame.into_pixels()))
                .collect()
        };
        let (red, blue, clear) = ([255, 0, 0, 255], [0, 0, 255, 255], [0; 4]);
        // The run restarts at the cut, as the clipped draw fills it.
        assert_eq!(
            rgba(data.frames(solid(0, 1), 200).unwrap()),
            [(4, [clear, blue, red, blue].concat())]
        );
        assert_eq!(data.frames(solid(0, 1), 201), Err(ImageError::Missing));
        assert_eq!(data.frames(solid(0, 1), 202), Err(ImageError::Missing));
    }

    /// The game data resolves a sheet by the original's reading, which
    /// decides graphic type 0's fallback and the sizes a view places it
    /// by. Under either reading, the images draw a 32-bit sheet the store
    /// resolves at the size it resolves, and fail one it fails.
    #[test]
    fn either_reading_draws_a_32_bit_sheet_as_the_store_resolves_it() {
        let wide = RledBuilder::new(1, 1)
            .depth(32)
            .count_lines()
            .frame(|f| f.line().pixels32(&[1, 2]))
            .build();
        let uncounted = RledBuilder::new(2, 1)
            .depth(32)
            .frame(|f| f.line().pixels32(&[1, 2]))
            .build();
        for (why, bytes, resolves) in [
            ("a row wider than its box", wide, true),
            ("line starts without their byte counts", uncounted, false),
        ] {
            let data = store(&[(RLED, 200, bytes), (Spin::TYPE, 3000, spin(200, -1))]);
            let resolved = match data.spin_sheet(3000) {
                Ok(spin) => Ok((spin.sheet.image_width(), spin.sheet.image_height())),
                Err(SpinSpriteError::Graphics { source, .. }) => Err(ImageError::Decode(source)),
                Err(other) => panic!("{why}: {other}"),
            };
            assert_eq!(resolved.is_ok(), resolves, "{why}");
            for reading in [
                ThirtyTwoBitReading::AsDrawn,
                ThirtyTwoBitReading::TrueColour,
            ] {
                let images =
                    GameImages::new(&data, EightBitReading::AsDrawn).with_thirty_two_bit(reading);
                let drawn = images
                    .frames(ImageKind::Rled, 200)
                    .map(|frames| (frames[0].width(), frames[0].height()));
                assert_eq!(drawn, resolved, "{why}, {reading:?}");
            }
        }
    }

    #[test]
    fn an_8_bit_sheet_is_drawn_as_the_reading_says() {
        // One 2x1 frame: a 1-byte run of 35 (pure red in the palette),
        // then the byte 0x7C.
        let eight_bit = RledBuilder::new(2, 1)
            .depth(8)
            .frame(|f| f.line().run8(1, [35, 0, 0, 0]).pixels8(&[0x7C]))
            .build();
        let data = store(&[(RLED, 200, eight_bit), (Spin::TYPE, 3000, spin(200, -1))]);
        let (red, clear) = ([255, 0, 0, 255], [0; 4]);
        let pixels = |frames: Vec<Image>| -> Vec<_> {
            frames
                .iter()
                .flat_map(|frame| [frame.pixel(0, 0), frame.pixel(1, 0)])
                .collect()
        };
        // As drawn, the run's byte 0 and the byte 0x7C make pixel 0, red at
        // RGB555 0x7C00; through the palette each byte is a pixel.
        let as_drawn = vec![Some(red), Some(clear)];
        let palette = vec![Some(red), Some([0x66, 0x99, 0x33, 255])];
        let reading = |eight_bit| GameImages::new(&data, eight_bit);
        for (images, expected) in [
            (reading(EightBitReading::AsDrawn), &as_drawn),
            (reading(EightBitReading::SystemPalette), &palette),
        ] {
            assert_eq!(
                pixels(images.frames(ImageKind::Rled, 200).unwrap()),
                *expected
            );
            // A spïn that names an rlëD reads it the same way.
            assert_eq!(
                pixels(images.frames(ImageKind::SpinSheet, 3000).unwrap()),
                *expected
            );
        }
        assert_eq!(
            pixels(data.frames(ImageKind::Rled, 200).unwrap()),
            as_drawn,
            "the game data alone draws it as the original does"
        );
        // The solid runs follow the reading too: as drawn, the run wrote
        // only half of pixel 0.
        let solid = |eight_bit| {
            pixels(
                reading(eight_bit)
                    .frames(ImageKind::RledSolid, 200)
                    .unwrap(),
            )
        };
        assert_eq!(solid(EightBitReading::AsDrawn), [Some(clear); 2]);
        assert_eq!(
            solid(EightBitReading::SystemPalette),
            [Some(red), Some(clear)]
        );
        // A shared store is a store.
        let shared = std::rc::Rc::new(data);
        assert_eq!(
            GameImages::new(std::rc::Rc::clone(&shared), EightBitReading::SystemPalette)
                .frames(ImageKind::Rled, 200)
                .map(pixels),
            Ok(palette)
        );
    }

    /// A version 1 picture of a 2x1 `BitMap`: black, then white.
    fn interface_picture() -> Vec<u8> {
        PictBuilder::v1([0, 0, 1, 2])
            .bits_rect(&BitMap::new([0, 0, 1, 2], &[true, false]))
            .end()
            .build()
    }

    #[test]
    fn an_interface_picture_comes_from_the_interface_file_alone() {
        let data = store(&[(PICT, 128, picture()), (PICT, 131, picture())]);
        let interface = Rc::new(interface_of(&[
            (PICT, 128, interface_picture()),
            (PICT, 129, interface_picture()),
        ]));
        let images =
            GameImages::new(&data, EightBitReading::AsDrawn).with_interface(Rc::clone(&interface));
        let frames = images.frames(ImageKind::InterfacePict, 128).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!((frames[0].width(), frames[0].height()), (2, 1));
        assert_eq!(frames[0].pixel(0, 0), Some([0, 0, 0, 255]));
        assert_eq!(frames[0].pixel(1, 0), Some([255, 255, 255, 255]));
        // The game data's PICT 128 is still the game data's.
        let game = images.frames(ImageKind::Pict, 128).unwrap();
        assert_eq!((game[0].width(), game[0].height()), (3, 2));
        // Neither file answers for the other.
        assert_eq!(
            images.frames(ImageKind::Pict, 129),
            Err(ImageError::Missing)
        );
        assert_eq!(
            images.frames(ImageKind::InterfacePict, 131),
            Err(ImageError::Missing)
        );
        assert_eq!(
            images.frames(ImageKind::InterfacePict, 130),
            Err(ImageError::Missing)
        );
    }

    #[test]
    fn without_an_interface_file_an_interface_picture_is_missing() {
        let data = store(&[(PICT, 128, picture())]);
        let images = GameImages::new(&data, EightBitReading::AsDrawn);
        assert_eq!(
            images.frames(ImageKind::InterfacePict, 128),
            Err(ImageError::Missing)
        );
        assert_eq!(
            data.frames(ImageKind::InterfacePict, 128),
            Err(ImageError::Missing)
        );
    }

    #[test]
    fn an_interface_picture_that_does_not_decode_is_a_decode_error() {
        let data = store(&[]);
        let interface = Rc::new(interface_of(&[(PICT, 128, vec![0; 3])]));
        let images = GameImages::new(&data, EightBitReading::AsDrawn).with_interface(interface);
        assert_eq!(
            images.frames(ImageKind::InterfacePict, 128),
            Err(ImageError::Decode(GraphicsError::UnexpectedEnd {
                offset: 2
            }))
        );
    }

    #[test]
    fn corrupt_bytes_are_a_decode_error() {
        let data = store(&[(PICT, 128, vec![0; 3]), (RLED, 200, vec![0; 3])]);
        assert_eq!(
            data.frames(ImageKind::Pict, 128),
            Err(ImageError::Decode(GraphicsError::UnexpectedEnd {
                offset: 2
            }))
        );
        assert!(matches!(
            data.frames(ImageKind::Rled, 200),
            Err(ImageError::Decode(_))
        ));
    }

    /// A 2x1 16-bit picture: red, then blue.
    fn picture_16() -> Vec<u8> {
        PictBuilder::new([0, 0, 1, 2])
            .direct_bits(&DirectBits::rgb555([0, 0, 1, 2], &[0x7C00, 0x001F]))
            .end()
            .build()
    }

    /// A 3x2 8-bit indexed picture of indices 0 to 5.
    fn picture_indexed() -> Vec<u8> {
        let entries = (0..6).map(|i| (i, [i * 0x2000; 3])).collect();
        let ctab = Ctab {
            device: false,
            entries,
        };
        PictBuilder::new([0, 0, 2, 3])
            .packbits_rect(&IndexedBits::new(
                [0, 0, 2, 3],
                8,
                &ctab,
                &[0, 1, 2, 3, 4, 5],
            ))
            .end()
            .build()
    }

    fn indexed(indices: &[u8]) -> PixelDepth {
        PixelDepth::Indexed {
            indices: indices.to_vec(),
        }
    }

    /// The depth and colours of (`kind`, `id`)'s one deep frame, after
    /// checking its colours are `frames`'s.
    fn deep_one(source: &impl ImageSource, kind: ImageKind, id: i16) -> PixelDepth {
        let deep = source.deep_frames(kind, id).expect("decodes");
        let frames = source.frames(kind, id).expect("decodes");
        assert_eq!(deep.len(), 1);
        assert_eq!(deep[0].image(), &frames[0], "the colours are frames'");
        deep[0].depth().clone()
    }

    #[test]
    fn a_pictures_deep_frame_has_the_depth_it_was_stored_at() {
        let data = store(&[
            (PICT, 128, picture()),
            (PICT, 129, picture_16()),
            (PICT, 130, picture_indexed()),
        ]);
        let images = GameImages::new(&data, EightBitReading::AsDrawn);
        assert_eq!(
            deep_one(&images, ImageKind::Pict, 128),
            PixelDepth::Direct32
        );
        assert_eq!(
            deep_one(&images, ImageKind::Pict, 129),
            PixelDepth::Direct16
        );
        assert_eq!(
            deep_one(&images, ImageKind::Pict, 130),
            indexed(&[0, 1, 2, 3, 4, 5])
        );
        // The game data itself answers as its images do.
        assert_eq!(deep_one(&data, ImageKind::Pict, 129), PixelDepth::Direct16);
    }

    #[test]
    fn a_masked_picture_keeps_its_pictures_depth_with_the_masks_alpha() {
        let data = store(&[(PICT, 130, picture_indexed()), (PICT, 228, mask())]);
        let masked = ImageKind::MaskedPict { mask: 228 };
        assert_eq!(deep_one(&data, masked, 130), indexed(&[0, 1, 2, 3, 4, 5]));
        let deep = data.deep_frames(masked, 130).expect("decodes");
        assert_eq!(deep[0].image().pixel(1, 0).map(|p| p[3]), Some(0));
    }

    #[test]
    fn an_interface_pictures_deep_frame_has_its_depth() {
        let data = store(&[]);
        let interface = Rc::new(interface_of(&[
            (PICT, 128, interface_picture()),
            (PICT, 129, picture_16()),
        ]));
        let images = GameImages::new(&data, EightBitReading::AsDrawn).with_interface(interface);
        assert_eq!(
            deep_one(&images, ImageKind::InterfacePict, 128),
            indexed(&[1, 0])
        );
        assert_eq!(
            deep_one(&images, ImageKind::InterfacePict, 129),
            PixelDepth::Direct16
        );
    }

    #[test]
    fn other_kinds_deep_frames_are_their_frames_at_32_bits() {
        let data = store(&[(RLED, 200, sheet())]);
        let deep = data.deep_frames(ImageKind::Rled, 200).expect("decodes");
        let frames = data.frames(ImageKind::Rled, 200).expect("decodes");
        assert_eq!(deep.len(), 2);
        for (deep, frame) in deep.iter().zip(&frames) {
            assert_eq!(deep.depth(), &PixelDepth::Direct32);
            assert_eq!(deep.image(), frame);
        }
    }

    #[test]
    fn a_deep_frame_fails_as_its_frame_does() {
        let data = store(&[
            (PICT, 128, picture()),
            (PICT, 228, mask()),
            (PICT, 1, vec![0; 3]),
            (PICT, 2, picture_16()),
        ]);
        let interface = Rc::new(interface_of(&[(PICT, 1, vec![0; 3])]));
        let images = GameImages::new(&data, EightBitReading::AsDrawn).with_interface(interface);
        let bare = GameImages::new(&data, EightBitReading::AsDrawn);
        for (source, kind, id) in [
            (&images, ImageKind::Pict, 1),
            (&images, ImageKind::Pict, 3),
            (&images, ImageKind::MaskedPict { mask: 228 }, 1),
            (&images, ImageKind::MaskedPict { mask: 1 }, 128),
            (&images, ImageKind::MaskedPict { mask: 229 }, 128),
            (&images, ImageKind::MaskedPict { mask: 228 }, 129),
            (&images, ImageKind::MaskedPict { mask: 2 }, 128),
            (&images, ImageKind::InterfacePict, 1),
            (&images, ImageKind::InterfacePict, 2),
            (&bare, ImageKind::InterfacePict, 1),
        ] {
            let frames = source.frames(kind, id).map(|_| ());
            assert!(frames.is_err(), "{kind:?} {id}");
            assert_eq!(
                source.deep_frames(kind, id).map(|_| ()),
                frames,
                "{kind:?} {id}"
            );
        }
    }
}
