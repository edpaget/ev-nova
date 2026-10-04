//! Images from the game data: `PICT`s, masked `PICT`s and `rlëD` sprite
//! sheets, decoded on request.

use nova_data::GameData;
use nova_data::graphics::{Image, PICT, RLED, apply_mask, decode_pict, decode_rled};
use nova_view::ImageKind;

use crate::images::{ImageError, ImageSource};

/// Looks resources up in the store (later files winning) and decodes them
/// on every call; the renderer keeps what it needs in the atlas. A masked
/// picture is its picture with the mask picture applied
/// ([`apply_mask`]); either one missing is [`ImageError::Missing`].
impl ImageSource for GameData {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        let bytes = |ty, id| {
            self.resource(ty, id)
                .map(|found| found.resource.data())
                .ok_or(ImageError::Missing)
        };
        Ok(match kind {
            ImageKind::Pict => vec![decode_pict(bytes(PICT, id)?)?],
            ImageKind::Rled => decode_rled(bytes(RLED, id)?, None)?.into_frames(),
            ImageKind::MaskedPict { mask } => {
                let picture = decode_pict(bytes(PICT, id)?)?;
                let mask = decode_pict(bytes(PICT, mask)?)?;
                vec![apply_mask(&picture, &mask)?]
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;

    use nova_data::graphics::GraphicsError;
    use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};

    use super::*;

    /// One data file, `/data/Nova Data`, holding a fork of `resources`.
    struct OneFile(Vec<u8>);

    impl DirLister for OneFile {
        fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
            Ok(vec![Listing {
                name: "Nova Data".into(),
                kind: EntryKind::File,
            }])
        }
    }

    impl ForkReader for OneFile {
        fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            Ok((fork == Fork::Data).then(|| self.0.clone()))
        }
    }

    fn store(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
        let fork = resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
                fork.resource(*ty, *id, None, data)
            })
            .build()
            .bytes;
        let file = OneFile(fork);
        GameData::load(&file, &file, Path::new("/data"), None).unwrap()
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

    /// Two 1x1 frames: red, then blue.
    fn sheet() -> Vec<u8> {
        RledBuilder::new(1, 1)
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
