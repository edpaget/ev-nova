//! Images from the game data: `PICT`s and `rlëD` sprite sheets, decoded on
//! request.

use nova_data::GameData;
use nova_data::graphics::{Image, RLED, decode_pict, decode_rled};
use nova_rsrc::ResType;
use nova_view::ImageKind;

use crate::images::{ImageError, ImageSource};

/// The `PICT` resource type.
const PICT: ResType = ResType::new(*b"PICT");

/// Looks resources up in the store (later files winning) and decodes them
/// on every call; the renderer keeps what it needs in the atlas.
impl ImageSource for GameData {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        let ty = match kind {
            ImageKind::Pict => PICT,
            ImageKind::Rled => RLED,
        };
        let bytes = self
            .resource(ty, id)
            .ok_or(ImageError::Missing)?
            .resource
            .data();
        Ok(match kind {
            ImageKind::Pict => vec![decode_pict(bytes)?],
            ImageKind::Rled => decode_rled(bytes, None)?.into_frames(),
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
    use nova_rsrc::{Fork, ForkReader};

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

    #[test]
    fn pict_is_the_picture_type() {
        assert_eq!(PICT, ResType::new(*b"PICT"));
    }
}
