//! Resolving a stellar object to its sprite sheet.

use super::spin_sprite::{SheetKind, SpinSpriteError, mask_label};
use super::{GameData, SourceFile, StoreEntry};
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
    /// The `spïn`'s `SpritesID`: the sheet's `rlëD`, or its sprites `PICT`
    /// ([`SheetKind`] says which).
    pub image_id: i16,
    /// What the sheet was read from.
    pub kind: SheetKind,
    /// The decoded sheet, laid out with the `spïn`'s `xTiles` as its
    /// columns.
    pub sheet: SpriteSheet,
    /// The file the `spöb` came from.
    pub stellar: &'a SourceFile,
    /// The file the `spïn` came from.
    pub spin: &'a SourceFile,
    /// The file the `rlëD`, or the sprites `PICT`, came from.
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
    /// The `spïn`'s sprites have neither an `rlëD` nor a `PICT`.
    #[error("spïn {spin}: no rlëD or PICT {image_id} for its sprites")]
    NoSheet {
        /// The `spïn`'s ID.
        spin: i16,
        /// Its `SpritesID`.
        image_id: i16,
    },
    /// The `spïn`'s sprites are a `PICT` with no mask `PICT`: its
    /// `MasksID` is -1 or names no `PICT`.
    #[error("spïn {spin}: no mask PICT {} for its sprites", mask_label(*mask_id))]
    NoMask {
        /// The `spïn`'s ID.
        spin: i16,
        /// Its `MasksID`, `None` for -1.
        mask_id: Option<i16>,
    },
    /// The `rlëD` failed to decode.
    #[error("rlëD {image_id}: {source}")]
    Graphics {
        /// The `rlëD`'s ID.
        image_id: i16,
        /// What went wrong.
        source: GraphicsError,
    },
    /// The sprites or mask `PICT` failed to decode, or the pair could not
    /// be cut into frames.
    #[error("PICT {pict_id}: {source}")]
    Picture {
        /// The `PICT`'s ID.
        pict_id: i16,
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
        let entry = self.stellar_entry(stellar)?;
        let graphic_type = entry.record.graphic_type;
        let spin_id = stellar_spin_id(graphic_type).ok_or(StellarSpriteError::BadType {
            stellar: stellar.0,
            graphic_type,
        })?;
        self.graphic_type_sprite(stellar, entry.source, graphic_type, spin_id)
    }

    /// Resolves `stellar`'s `DeadType` to the sprite sheet it draws while
    /// destroyed, as [`stellar_sprite`](Self::stellar_sprite) resolves its
    /// `Type`. `Ok(None)` when the `DeadType` is outside 0 to 255: the
    /// Bible's -1 is "don't display different graphic type when
    /// destroyed", and the engine reads any other such value the same way
    /// (story-runtime phase 7). Otherwise the errors are
    /// `stellar_sprite`'s, never [`StellarSpriteError::BadType`].
    pub fn stellar_dead_sprite(
        &self,
        stellar: StellarId,
    ) -> Result<Option<StellarSprite<'_>>, StellarSpriteError> {
        let entry = self.stellar_entry(stellar)?;
        let dead_type = entry.record.dead_type;
        stellar_spin_id(dead_type)
            .map(|spin_id| self.graphic_type_sprite(stellar, entry.source, dead_type, spin_id))
            .transpose()
    }

    /// `stellar`'s `spöb`, which must exist and decode.
    fn stellar_entry(
        &self,
        stellar: StellarId,
    ) -> Result<StoreEntry<'_, Stellar>, StellarSpriteError> {
        self.get::<Stellar>(stellar.0)
            .ok_or(StellarSpriteError::NoStellar(stellar.0))?
            .map_err(|e| StellarSpriteError::Decode(e.clone()))
    }

    /// The sheet of `stellar`'s graphic type `graphic_type`, `spïn`
    /// `spin_id`, whose `spöb` came from `source`.
    fn graphic_type_sprite<'a>(
        &'a self,
        stellar: StellarId,
        source: &'a SourceFile,
        graphic_type: i16,
        spin_id: i16,
    ) -> Result<StellarSprite<'a>, StellarSpriteError> {
        let id = stellar.0;
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
            SpinSpriteError::NoMask { spin, mask_id } => {
                StellarSpriteError::NoMask { spin, mask_id }
            }
            SpinSpriteError::Graphics { image_id, source } => {
                StellarSpriteError::Graphics { image_id, source }
            }
            SpinSpriteError::Picture { pict_id, source } => {
                StellarSpriteError::Picture { pict_id, source }
            }
        })?;
        Ok(StellarSprite {
            spin_id,
            image_id: spin.image_id,
            kind: spin.kind,
            sheet: spin.sheet,
            stellar: source,
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
    use crate::graphics::fixture::{RledBuilder, rgb888_pict};
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
        let nothing = fork(&[(Spin::TYPE, 1003, spin(2001, 2, 1, 4, 2))]);
        let err = store(&[("Grid", nothing)])
            .stellar_sprite(StellarId(128))
            .expect_err("no rlëD or PICT 2001");
        assert_eq!(
            err,
            StellarSpriteError::NoSheet {
                spin: 1003,
                image_id: 2001
            }
        );
        assert_eq!(
            err.to_string(),
            "spïn 1003: no rlëD or PICT 2001 for its sprites"
        );
    }

    /// A `spïn` naming sprites `image` and masks `mask`, `width` x
    /// `height` each, in a 1 x 1 grid.
    fn masked_spin(image: i16, mask: i16, width: i16, height: i16) -> Vec<u8> {
        buf::<Spin>()
            .i16(0x00, image)
            .i16(0x02, mask)
            .i16(0x04, width)
            .i16(0x06, height)
            .i16(0x08, 1)
            .i16(0x0A, 1)
            .0
    }

    #[test]
    fn a_dead_type_pict_pair_resolves_to_its_sheet() {
        let red = [0xFF, 0, 0];
        let data = store(&[(
            "Dead",
            fork(&[
                (Stellar::TYPE, 128, dead(5)),
                (Spin::TYPE, 1005, masked_spin(2005, 2105, 2, 1)),
                (PICT, 2005, rgb888_pict(4, 1, &[red; 4])),
                (PICT, 2105, rgb888_pict(4, 1, &[[0xFF; 3]; 4])),
            ]),
        )]);
        let sprite = data
            .stellar_dead_sprite(StellarId(128))
            .expect("resolves")
            .expect("a dead type");
        assert_eq!((sprite.spin_id, sprite.image_id), (1005, 2005));
        assert_eq!(sprite.kind, SheetKind::PictPair { mask_id: 2105 });
        assert_eq!(sprite.sheet.frames().len(), 2);
        assert_eq!(sprite.sheet.frames()[1].pixel(1, 0), Some([255, 0, 0, 255]));
        assert_eq!(sprite.sheet_source.path, Path::new("/p/Dead"));
        let alive = data.stellar_sprite(StellarId(128)).expect("type 3");
        assert_eq!(alive.kind, SheetKind::Rled);
    }

    #[test]
    fn a_pict_pairs_mask_and_picture_errors_are_kept() {
        let no_mask = fork(&[
            (Spin::TYPE, 1003, masked_spin(2001, 2101, 2, 1)),
            (PICT, 2001, rgb888_pict(2, 1, &[[0; 3]; 2])),
        ]);
        let err = store(&[("Grid", no_mask)])
            .stellar_sprite(StellarId(128))
            .expect_err("no mask");
        assert_eq!(
            err,
            StellarSpriteError::NoMask {
                spin: 1003,
                mask_id: Some(2101)
            }
        );
        assert_eq!(
            err.to_string(),
            "spïn 1003: no mask PICT 2101 for its sprites"
        );
        let none = fork(&[
            (Spin::TYPE, 1003, masked_spin(2001, -1, 2, 1)),
            (PICT, 2001, rgb888_pict(2, 1, &[[0; 3]; 2])),
        ]);
        let err = store(&[("Grid", none)])
            .stellar_sprite(StellarId(128))
            .expect_err("no mask");
        assert_eq!(
            err.to_string(),
            "spïn 1003: no mask PICT none for its sprites"
        );
        let partial = fork(&[
            (Spin::TYPE, 1003, masked_spin(2001, 2001, 2, 1)),
            (PICT, 2001, rgb888_pict(3, 1, &[[0; 3]; 3])),
        ]);
        let err = store(&[("Grid", partial)])
            .stellar_sprite(StellarId(128))
            .expect_err("a partial grid");
        let source = GraphicsError::PartialGrid {
            width: 3,
            height: 1,
            frame_width: 2,
            frame_height: 1,
        };
        assert_eq!(err.to_string(), format!("PICT 2001: {source}"));
        assert_eq!(
            err,
            StellarSpriteError::Picture {
                pict_id: 2001,
                source
            }
        );
    }

    /// A `spöb` of graphic type 3 whose `DeadType` is `dead_type`.
    fn dead(dead_type: i16) -> Vec<u8> {
        buf::<Stellar>().i16(0x04, 3).i16(0x240, dead_type).0
    }

    #[test]
    fn a_dead_type_outside_0_to_255_is_no_different_graphic() {
        for dead_type in [-1, -2, 256, i16::MIN] {
            let data = store(&[("Dead", fork(&[(Stellar::TYPE, 128, dead(dead_type))]))]);
            let sprite = data.stellar_dead_sprite(StellarId(128)).expect("no error");
            assert!(sprite.is_none(), "{dead_type}");
        }
    }

    #[test]
    fn a_dead_type_resolves_through_its_spin_to_its_sheet() {
        let data = store(&[(
            "Dead",
            fork(&[
                (Stellar::TYPE, 128, dead(5)),
                (Spin::TYPE, 1005, spin(2005, 2, 1, 3, 1)),
                (RLED, 2005, sheet(3, 0x001F)),
            ]),
        )]);
        let sprite = data
            .stellar_dead_sprite(StellarId(128))
            .expect("resolves")
            .expect("a dead type");
        assert_eq!((sprite.spin_id, sprite.image_id), (1005, 2005));
        assert_eq!(sprite.sheet.frames().len(), 3);
        assert_eq!(sprite.sheet.layout().columns(), 3);
        assert_eq!(sprite.sheet.frames()[0].pixel(0, 0), Some([0, 0, 255, 255]));
        assert_eq!(sprite.stellar.path, Path::new("/p/Dead"));
        let alive = data.stellar_sprite(StellarId(128)).expect("type 3");
        assert_eq!(alive.spin_id, 1003, "the Type sheet is unchanged");
    }

    #[test]
    fn a_dead_type_of_0_is_spin_1000() {
        let data = store(&[(
            "Dead",
            fork(&[
                (Stellar::TYPE, 128, dead(0)),
                (Spin::TYPE, 1000, spin(2010, 2, 1, 1, 1)),
                (RLED, 2010, sheet(1, 0x001F)),
            ]),
        )]);
        let sprite = data.stellar_dead_sprite(StellarId(128)).expect("resolves");
        assert_eq!(sprite.map(|s| s.spin_id), Some(1000));
    }

    #[test]
    fn a_dead_type_with_no_spin_is_no_spin() {
        let data = store(&[("Dead", fork(&[(Stellar::TYPE, 128, dead(7))]))]);
        let err = data
            .stellar_dead_sprite(StellarId(128))
            .expect_err("no spïn 1007");
        assert_eq!(
            err,
            StellarSpriteError::NoSpin {
                stellar: 128,
                graphic_type: 7,
                spin: 1007
            }
        );
    }

    #[test]
    fn a_missing_stellar_has_no_dead_sprite_either() {
        let err = store(&[])
            .stellar_dead_sprite(StellarId(129))
            .expect_err("absent");
        assert_eq!(err, StellarSpriteError::NoStellar(129));
    }

    #[test]
    fn a_corrupt_sheet_is_a_graphics_error() {
        let deep = RledBuilder::new(2, 1).depth(32).build();
        let data = store(&[("Bad Paint", fork(&[(RLED, 2000, deep)]))]);
        let err = data.stellar_sprite(StellarId(128)).expect_err("32-bit");
        assert_eq!(
            err,
            StellarSpriteError::Graphics {
                image_id: 2000,
                source: GraphicsError::UnsupportedDepth { depth: 32 },
            }
        );
        assert_eq!(err.to_string(), "rlëD 2000: unsupported rlëD depth 32");
    }
}
