//! Resolving a `spïn` to its sprite sheet: the link stellars, shots and
//! explosions share.

use super::{GameData, SourceFile};
use crate::error::DecodeError;
use crate::graphics::{GraphicsError, RLED, SpriteSheet, decode_rled};
use crate::records::spin::Spin;

/// A `spïn`'s sprite sheet and the files each step came from.
#[derive(Debug)]
pub struct SpinSheet<'a> {
    /// The `spïn`'s ID.
    pub spin_id: i16,
    /// The sheet's `rlëD` ID (the `spïn`'s `SpritesID`).
    pub image_id: i16,
    /// The decoded sheet, laid out with the `spïn`'s `xTiles` as its
    /// columns.
    pub sheet: SpriteSheet,
    /// The file the `spïn` came from.
    pub spin: &'a SourceFile,
    /// The file the `rlëD` came from.
    pub sheet_source: &'a SourceFile,
}

impl SpinSheet<'_> {
    /// How many frames the sheet holds.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.sheet.frames().len()
    }
}

/// Why a `spïn`'s sheet could not be resolved: one variant per missing or
/// bad link from the `spïn` to its `rlëD`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SpinSpriteError {
    /// There is no `spïn` with this ID.
    #[error("no spïn {0}")]
    NoSpin(i16),
    /// The `spïn` failed to decode.
    #[error(transparent)]
    Decode(DecodeError),
    /// The `spïn`'s sprites have no `rlëD` (a `PICT` sprite grid is not
    /// supported by this lookup).
    #[error("spïn {spin}: no rlëD {image_id} for its sprites")]
    NoSheet {
        /// The `spïn`'s ID.
        spin: i16,
        /// Its `SpritesID`.
        image_id: i16,
    },
    /// The `rlëD` failed to decode.
    #[error("rlëD {image_id}: {source}")]
    Graphics {
        /// The `rlëD`'s ID.
        image_id: i16,
        /// What went wrong.
        source: GraphicsError,
    },
}

impl GameData {
    /// Resolves `spïn` `spin_id` to its sprite sheet: the `spïn` must exist
    /// and decode; the `rlëD` is its `SpritesID`, decoded with its `xTiles`
    /// as the columns.
    ///
    /// The `spïn`'s `xSize` and `ySize` are not checked against the sheet:
    /// the decoded frames are authoritative. The sheet is decoded on every
    /// call and not cached.
    pub fn spin_sheet(&self, spin_id: i16) -> Result<SpinSheet<'_>, SpinSpriteError> {
        let spin = self
            .get::<Spin>(spin_id)
            .ok_or(SpinSpriteError::NoSpin(spin_id))?
            .map_err(|e| SpinSpriteError::Decode(e.clone()))?;
        let image_id = spin.record.sprites_id;
        let found = self
            .resource(RLED, image_id)
            .ok_or(SpinSpriteError::NoSheet {
                spin: spin_id,
                image_id,
            })?;
        let sheet = decode_rled(found.resource.data(), spin.record.sheet_layout())
            .map_err(|source| SpinSpriteError::Graphics { image_id, source })?;
        Ok(SpinSheet {
            spin_id,
            image_id,
            sheet,
            spin: spin.source,
            sheet_source: found.source,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_rsrc::ResType;
    use nova_rsrc::fixture::ForkBuilder;

    use super::*;
    use crate::decode::Record;
    use crate::graphics::PICT;
    use crate::graphics::fixture::RledBuilder;
    use crate::store::fake::{FakeForks, FakeTree};
    use crate::store::fs::EntryKind::File;
    use crate::testutil::buf;

    /// A `spïn` naming sprites `image`, `width` x `height` each, in a grid
    /// `x_tiles` across and `y_tiles` down.
    fn spin(image: i16, width: i16, height: i16, x_tiles: i16, y_tiles: i16) -> Vec<u8> {
        buf::<Spin>()
            .i16(0x00, image)
            .i16(0x02, -1)
            .i16(0x04, width)
            .i16(0x06, height)
            .i16(0x08, x_tiles)
            .i16(0x0A, y_tiles)
            .0
    }

    /// An `rlëD` of `frames` 2x1 frames whose pixels are `color`.
    fn sheet(frames: u16, color: u16) -> Vec<u8> {
        let mut builder = RledBuilder::new(2, 1);
        for _ in 0..frames {
            builder = builder.frame(|f| f.line().pixels(&[color, color]));
        }
        builder.build()
    }

    fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> Vec<u8> {
        resources
            .iter()
            .fold(ForkBuilder::new(), |b, (ty, id, data)| {
                b.resource(*ty, *id, None, data)
            })
            .build()
            .bytes
    }

    /// `spïn` 3000 (sprites 2000, a 6 x 6 grid of 2x1 frames) and its
    /// 36-frame `rlëD` in two files, plus `extra` plug-ins.
    fn store(extra: &[(&str, Vec<u8>)]) -> GameData {
        let plugins: Vec<(&str, _)> = extra.iter().map(|(name, _)| (*name, File)).collect();
        let tree = FakeTree::new()
            .dir("/d", &[("spins", File), ("sprites", File)])
            .dir("/p", &plugins);
        let mut forks = FakeForks::new()
            .file(
                "/d/spins",
                fork(&[(Spin::TYPE, 3000, spin(2000, 2, 1, 6, 6))]),
            )
            .file("/d/sprites", fork(&[(RLED, 2000, sheet(36, 0x7C00))]));
        for (name, bytes) in extra {
            forks = forks.file(&format!("/p/{name}"), bytes.clone());
        }
        GameData::load(&tree, &forks, Path::new("/d"), Some(Path::new("/p"))).expect("opens")
    }

    #[test]
    fn a_spin_resolves_to_its_sheet() {
        let data = store(&[]);
        let sheet = data.spin_sheet(3000).expect("resolves");
        assert_eq!((sheet.spin_id, sheet.image_id), (3000, 2000));
        assert_eq!(sheet.frame_count(), 36);
        assert_eq!(sheet.sheet.layout().columns(), 6);
        assert_eq!(
            (sheet.sheet.frame_width(), sheet.sheet.frame_height()),
            (2, 1)
        );
        assert_eq!(sheet.sheet.frames()[0].pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(sheet.spin.path, Path::new("/d/spins"));
        assert_eq!(sheet.sheet_source.path, Path::new("/d/sprites"));
    }

    #[test]
    fn a_missing_spin_is_no_spin() {
        let err = store(&[]).spin_sheet(3001).expect_err("absent");
        assert_eq!(err, SpinSpriteError::NoSpin(3001));
        assert_eq!(err.to_string(), "no spïn 3001");
    }

    #[test]
    fn a_spin_that_fails_to_decode_is_a_decode_error() {
        let short = fork(&[(Spin::TYPE, 3000, spin(2000, 2, 1, 6, 6)[1..].to_vec())]);
        let data = store(&[("Broken", short)]);
        let err = data.spin_sheet(3000).expect_err("bad spïn");
        let SpinSpriteError::Decode(inner) = &err else {
            panic!("a decode error: {err:?}")
        };
        assert_eq!((inner.res_type, inner.id), (Spin::TYPE, 3000));
        assert_eq!(err.to_string(), inner.to_string());
    }

    #[test]
    fn sprites_with_no_sheet_are_no_sheet() {
        let pict_grid = fork(&[
            (Spin::TYPE, 3000, spin(2001, 2, 1, 6, 6)),
            (PICT, 2001, vec![0; 16]),
        ]);
        let err = store(&[("Grid", pict_grid)])
            .spin_sheet(3000)
            .expect_err("no rlëD 2001");
        assert_eq!(
            err,
            SpinSpriteError::NoSheet {
                spin: 3000,
                image_id: 2001
            }
        );
        assert_eq!(err.to_string(), "spïn 3000: no rlëD 2001 for its sprites");
    }

    #[test]
    fn a_corrupt_sheet_is_a_graphics_error() {
        let eight_bit = RledBuilder::new(2, 1).depth(8).build();
        let data = store(&[("Bad Paint", fork(&[(RLED, 2000, eight_bit)]))]);
        let err = data.spin_sheet(3000).expect_err("8-bit");
        assert_eq!(
            err,
            SpinSpriteError::Graphics {
                image_id: 2000,
                source: GraphicsError::UnsupportedDepth { depth: 8 },
            }
        );
        assert_eq!(err.to_string(), "rlëD 2000: unsupported rlëD depth 8");
    }
}
