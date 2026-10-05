//! Resolving a stellar object to its sprite sheet.

use super::spin_sprite::SpinSpriteError;
use super::{GameData, SourceFile};
use crate::error::DecodeError;
use crate::graphics::{GraphicsError, SpriteSheet};
use crate::records::stellar::Stellar;
use crate::wire::id::StellarId;

/// The `spïn` of a stellar's first graphic type.
const FIRST_STELLAR_SPIN: i16 = 1000;
/// The last graphic type the Bible allows.
const LAST_GRAPHIC_TYPE: i16 = 255;

/// The `spïn` ID for a stellar's graphic type (`spöb` `Type`): the Bible
/// gives `spïn` 1000 to 1255 to stellar objects, so `1000 + Type`. `None`
/// for a type outside 0 to 255.
#[must_use]
pub const fn stellar_spin_id(graphic_type: i16) -> Option<i16> {
    match graphic_type {
        t @ 0..=LAST_GRAPHIC_TYPE => Some(FIRST_STELLAR_SPIN + t),
        _ => None,
    }
}

/// A stellar's sprite sheet and the files each step came from.
#[derive(Debug)]
pub struct StellarSprite<'a> {
    /// The `spïn`'s ID (1000 + the `spöb`'s graphic type).
    pub spin_id: i16,
    /// The sheet's `rlëD` ID (the `spïn`'s `SpritesID`).
    pub image_id: i16,
    /// The decoded sheet, laid out with the `spïn`'s `xTiles` as its
    /// columns.
    pub sheet: SpriteSheet,
    /// The file the `spöb` came from.
    pub stellar: &'a SourceFile,
    /// The file the `spïn` came from.
    pub spin: &'a SourceFile,
    /// The file the `rlëD` came from.
    pub sheet_source: &'a SourceFile,
}

/// Why a stellar's sprite could not be resolved: one variant per missing
/// or bad link from the `spöb` to its `rlëD`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StellarSpriteError {
    /// There is no `spöb` with this ID.
    #[error("no spöb {0}")]
    NoStellar(i16),
    /// The `spöb` or its `spïn` failed to decode.
    #[error(transparent)]
    Decode(DecodeError),
    /// The `spöb`'s graphic type has no `spïn` by the Bible's numbering.
    #[error("spöb {stellar}: graphic type {graphic_type} is outside 0 to 255")]
    BadType {
        /// The stellar's ID.
        stellar: i16,
        /// Its graphic type.
        graphic_type: i16,
    },
    /// There is no `spïn` for the `spöb`'s graphic type.
    #[error("spöb {stellar}: no spïn {spin} for its graphic type {graphic_type}")]
    NoSpin {
        /// The stellar's ID.
        stellar: i16,
        /// Its graphic type.
        graphic_type: i16,
        /// The `spïn` ID that type names.
        spin: i16,
    },
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
    /// Resolves `stellar` to its sprite sheet: the `spöb` must exist and
    /// decode; its graphic type names `spïn` 1000 + type
    /// ([`stellar_spin_id`]), which must exist and decode; the `rlëD` is the
    /// `spïn`'s `SpritesID`, decoded with the `spïn`'s `xTiles` as its
    /// columns ([`GameData::spin_sheet`]).
    ///
    /// The `spïn`'s `xSize` and `ySize` are not checked against the sheet:
    /// the decoded frames are authoritative (stock `spöb` 472's sheet has
    /// larger frames than its `spïn` says). The sheet is decoded on every
    /// call and not cached.
    pub fn stellar_sprite(
        &self,
        stellar: StellarId,
    ) -> Result<StellarSprite<'_>, StellarSpriteError> {
        let id = stellar.0;
        let entry = self
            .get::<Stellar>(id)
            .ok_or(StellarSpriteError::NoStellar(id))?
            .map_err(|e| StellarSpriteError::Decode(e.clone()))?;
        let graphic_type = entry.record.graphic_type;
        let spin_id = stellar_spin_id(graphic_type).ok_or(StellarSpriteError::BadType {
            stellar: id,
            graphic_type,
        })?;
        let spin = self.spin_sheet(spin_id).map_err(|err| match err {
            SpinSpriteError::NoSpin(spin) => StellarSpriteError::NoSpin {
                stellar: id,
                graphic_type,
                spin,
            },
            SpinSpriteError::Decode(err) => StellarSpriteError::Decode(err),
            SpinSpriteError::NoSheet { spin, image_id } => {
                StellarSpriteError::NoSheet { spin, image_id }
            }
            SpinSpriteError::Graphics { image_id, source } => {
                StellarSpriteError::Graphics { image_id, source }
            }
        })?;
        Ok(StellarSprite {
            spin_id,
            image_id: spin.image_id,
            sheet: spin.sheet,
            stellar: entry.source,
            spin: spin.spin,
            sheet_source: spin.sheet_source,
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
    use crate::graphics::fixture::RledBuilder;
    use crate::graphics::{PICT, RLED};
    use crate::records::spin::Spin;
    use crate::store::fake::{FakeForks, FakeTree};
    use crate::store::fs::EntryKind::File;
    use crate::testutil::buf;

    /// A `spöb` of graphic type `graphic_type`.
    fn stellar(graphic_type: i16) -> Vec<u8> {
        buf::<Stellar>().i16(0x04, graphic_type).0
    }

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

    /// Stellar 128 of type 3, with its `spöb`, `spïn` 1003 (sprites 2000,
    /// a 4 x 2 grid of 2x1 frames) and its 8-frame `rlëD` in three files,
    /// plus `extra` plug-ins.
    fn store(extra: &[(&str, Vec<u8>)]) -> GameData {
        let plugins: Vec<(&str, _)> = extra.iter().map(|(name, _)| (*name, File)).collect();
        let tree = FakeTree::new()
            .dir(
                "/d",
                &[("stellars", File), ("spins", File), ("sprites", File)],
            )
            .dir("/p", &plugins);
        let mut forks = FakeForks::new()
            .file("/d/stellars", fork(&[(Stellar::TYPE, 128, stellar(3))]))
            .file(
                "/d/spins",
                fork(&[(Spin::TYPE, 1003, spin(2000, 2, 1, 4, 2))]),
            )
            .file("/d/sprites", fork(&[(RLED, 2000, sheet(8, 0x7C00))]));
        for (name, bytes) in extra {
            forks = forks.file(&format!("/p/{name}"), bytes.clone());
        }
        GameData::load(&tree, &forks, Path::new("/d"), Some(Path::new("/p"))).expect("opens")
    }

    #[test]
    fn graphic_types_map_onto_spin_1000_onwards() {
        assert_eq!(stellar_spin_id(0), Some(1000));
        assert_eq!(stellar_spin_id(65), Some(1065));
        assert_eq!(stellar_spin_id(255), Some(1255));
    }

    #[test]
    fn types_outside_0_to_255_have_no_spin() {
        for graphic_type in [-1, 256, i16::MIN, i16::MAX] {
            assert_eq!(stellar_spin_id(graphic_type), None, "{graphic_type}");
        }
    }

    #[test]
    fn a_stellar_resolves_through_its_spin_to_its_sheet() {
        let data = store(&[]);
        let sprite = data.stellar_sprite(StellarId(128)).expect("resolves");
        assert_eq!((sprite.spin_id, sprite.image_id), (1003, 2000));
        assert_eq!(sprite.sheet.frames().len(), 8);
        assert_eq!(sprite.sheet.layout().columns(), 4);
        assert_eq!(sprite.sheet.frame_width(), 2);
        assert_eq!(sprite.sheet.frame_height(), 1);
        assert_eq!(sprite.sheet.frames()[0].pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(sprite.stellar.path, Path::new("/d/stellars"));
        assert_eq!(sprite.spin.path, Path::new("/d/spins"));
        assert_eq!(sprite.sheet_source.path, Path::new("/d/sprites"));
    }

    #[test]
    fn a_spin_with_no_columns_gives_the_default_layout() {
        for x_tiles in [0, -1] {
            let no_columns = fork(&[(Spin::TYPE, 1003, spin(2000, 2, 1, x_tiles, 2))]);
            let data = store(&[("Grid", no_columns)]);
            let sprite = data.stellar_sprite(StellarId(128)).expect("resolves");
            assert_eq!(sprite.sheet.layout(), crate::graphics::SheetLayout::DEFAULT);
            assert_eq!(sprite.spin.path, Path::new("/p/Grid"));
        }
    }

    #[test]
    fn a_spin_size_that_differs_from_the_sheet_still_resolves_at_the_sheets_size() {
        let wrong_size = fork(&[(Spin::TYPE, 1003, spin(2000, 140, 120, 4, 2))]);
        let data = store(&[("Sizes", wrong_size)]);
        let sprite = data.stellar_sprite(StellarId(128)).expect("resolves");
        assert_eq!(
            (sprite.sheet.frame_width(), sprite.sheet.frame_height()),
            (2, 1)
        );
    }

    #[test]
    fn a_plugin_overriding_only_the_sheet_changes_its_source() {
        let data = store(&[("New Paint", fork(&[(RLED, 2000, sheet(8, 0x001F))]))]);
        let sprite = data.stellar_sprite(StellarId(128)).expect("resolves");
        assert_eq!(sprite.sheet_source.path, Path::new("/p/New Paint"));
        assert_eq!(sprite.stellar.path, Path::new("/d/stellars"));
        assert_eq!(sprite.spin.path, Path::new("/d/spins"));
        assert_eq!(sprite.sheet.frames()[0].pixel(0, 0), Some([0, 0, 255, 255]));
    }

    #[test]
    fn a_missing_stellar_is_no_stellar() {
        let err = store(&[])
            .stellar_sprite(StellarId(129))
            .expect_err("absent");
        assert_eq!(err, StellarSpriteError::NoStellar(129));
        assert_eq!(err.to_string(), "no spöb 129");
    }

    #[test]
    fn a_stellar_or_spin_that_fails_to_decode_is_a_decode_error() {
        let short_stellar = fork(&[(Stellar::TYPE, 128, stellar(3)[1..].to_vec())]);
        let data = store(&[("Broken", short_stellar)]);
        let err = data.stellar_sprite(StellarId(128)).expect_err("bad spöb");
        let StellarSpriteError::Decode(inner) = &err else {
            panic!("a decode error: {err:?}")
        };
        assert_eq!((inner.res_type, inner.id), (Stellar::TYPE, 128));
        assert_eq!(err.to_string(), inner.to_string());

        let short_spin = fork(&[(Spin::TYPE, 1003, spin(2000, 2, 1, 4, 2)[1..].to_vec())]);
        let data = store(&[("Broken", short_spin)]);
        let err = data.stellar_sprite(StellarId(128)).expect_err("bad spïn");
        let StellarSpriteError::Decode(inner) = &err else {
            panic!("a decode error: {err:?}")
        };
        assert_eq!((inner.res_type, inner.id), (Spin::TYPE, 1003));
        assert_eq!(err.to_string(), inner.to_string());
    }

    #[test]
    fn a_type_outside_0_to_255_is_a_bad_type() {
        let below = fork(&[(Stellar::TYPE, 128, stellar(-1))]);
        let err = store(&[("Below", below)])
            .stellar_sprite(StellarId(128))
            .expect_err("type -1");
        assert_eq!(
            err,
            StellarSpriteError::BadType {
                stellar: 128,
                graphic_type: -1
            }
        );
        assert_eq!(
            err.to_string(),
            "spöb 128: graphic type -1 is outside 0 to 255"
        );

        let above = fork(&[(Stellar::TYPE, 128, stellar(256))]);
        let err = store(&[("Above", above)])
            .stellar_sprite(StellarId(128))
            .expect_err("type 256");
        assert_eq!(
            err.to_string(),
            "spöb 128: graphic type 256 is outside 0 to 255"
        );
    }

    #[test]
    fn a_type_with_no_spin_is_no_spin() {
        let other_type = fork(&[(Stellar::TYPE, 128, stellar(4))]);
        let err = store(&[("Other", other_type)])
            .stellar_sprite(StellarId(128))
            .expect_err("no spïn 1004");
        assert_eq!(
            err,
            StellarSpriteError::NoSpin {
                stellar: 128,
                graphic_type: 4,
                spin: 1004
            }
        );
        assert_eq!(
            err.to_string(),
            "spöb 128: no spïn 1004 for its graphic type 4"
        );
    }

    #[test]
    fn sprites_with_no_sheet_are_no_sheet() {
        // The spïn names a PICT grid, which this lookup does not support.
        let pict_grid = fork(&[
            (Spin::TYPE, 1003, spin(2001, 2, 1, 4, 2)),
            (PICT, 2001, vec![0; 16]),
        ]);
        let err = store(&[("Grid", pict_grid)])
            .stellar_sprite(StellarId(128))
            .expect_err("no rlëD 2001");
        assert_eq!(
            err,
            StellarSpriteError::NoSheet {
                spin: 1003,
                image_id: 2001
            }
        );
        assert_eq!(err.to_string(), "spïn 1003: no rlëD 2001 for its sprites");
    }

    #[test]
    fn a_corrupt_sheet_is_a_graphics_error() {
        let eight_bit = RledBuilder::new(2, 1).depth(8).build();
        let data = store(&[("Bad Paint", fork(&[(RLED, 2000, eight_bit)]))]);
        let err = data.stellar_sprite(StellarId(128)).expect_err("8-bit");
        assert_eq!(
            err,
            StellarSpriteError::Graphics {
                image_id: 2000,
                source: GraphicsError::UnsupportedDepth { depth: 8 },
            }
        );
        assert_eq!(err.to_string(), "rlëD 2000: unsupported rlëD depth 8");
    }
}
