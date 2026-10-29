//! Images from the game data: `PICT`s, masked `PICT`s, `rlëD` sprite
//! sheets, the sheets `spïn`s resolve to (a `PICT` pair cut into frames),
//! `ppat` pixel patterns and `cicn` colour icons, decoded on request.

use std::borrow::Borrow;

use nova_data::graphics::{
    CICN, EightBitReading, Image, PICT, PPAT, RLED, apply_mask, decode_cicn, decode_pict,
    decode_ppat, decode_rled_as,
};
use nova_data::{GameData, SheetKind, SpinSpriteError};
use nova_view::ImageKind;

use crate::images::{ImageError, ImageSource};

/// The game data as an image source, decoding an 8-bit `rlëD` by
/// `eight_bit` (the rulebook's `eight_bit_sheet` reading).
///
/// It looks resources up in the store (later files winning) and decodes
/// them on every call; the renderer keeps what it needs in the atlas. A
/// masked picture is its picture with the mask picture applied
/// ([`apply_mask`]); either one missing is [`ImageError::Missing`].
#[derive(Clone, Debug)]
pub struct GameImages<D> {
    data: D,
    eight_bit: EightBitReading,
}

impl<D: Borrow<GameData>> GameImages<D> {
    /// `data`'s images, with 8-bit sheets read as `eight_bit` says.
    pub fn new(data: D, eight_bit: EightBitReading) -> Self {
        Self { data, eight_bit }
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
}

impl<D: Borrow<GameData>> ImageSource for GameImages<D> {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        let bytes = |ty, id| {
            self.data
                .borrow()
                .resource(ty, id)
                .map(|found| found.resource.data())
                .ok_or(ImageError::Missing)
        };
        let rled = |id| -> Result<_, ImageError> {
            Ok(decode_rled_as(bytes(RLED, id)?, None, self.eight_bit)?)
        };
        Ok(match kind {
            ImageKind::Pict => vec![decode_pict(bytes(PICT, id)?)?],
            ImageKind::Rled => rled(id)?.into_frames(),
            ImageKind::RledSolid => rled(id)?.into_solid_frames().ok_or(ImageError::Missing)?,
            ImageKind::MaskedPict { mask } => {
                let picture = decode_pict(bytes(PICT, id)?)?;
                let mask = decode_pict(bytes(PICT, mask)?)?;
                vec![apply_mask(&picture, &mask)?]
            }
            ImageKind::Pattern => vec![decode_ppat(bytes(PPAT, id)?)?],
            ImageKind::Icon => vec![decode_cicn(bytes(CICN, id)?)?],
            ImageKind::SpinSheet => self.spin_frames(id)?,
        })
    }
}

/// The game data's images as the original draws them: [`GameImages`] with
/// 8-bit sheets [`EightBitReading::AsDrawn`].
impl ImageSource for GameData {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        GameImages::new(self, EightBitReading::AsDrawn).frames(kind, id)
    }
}

#[cfg(test)]
mod tests {

    use nova_data::Record;
    use nova_data::graphics::GraphicsError;
    use nova_data::graphics::fixture::{
        Cicn, Ctab, DirectBits, PictBuilder, Ppat, RledBuilder, rgb888_pict,
    };
    use nova_data::records::spin::Spin;
    use nova_data::store::fixture::store_of;

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
}
