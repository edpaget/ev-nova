//! Resolving a `spïn` to its sprite sheet: the link stellars, shots and
//! explosions share.

use super::{GameData, SourceFile};
use crate::error::DecodeError;
use crate::graphics::{
    GraphicsError, Image, PICT, RLED, SheetMask, SpriteSheet, cut_pict_sheet, decode_pict,
    decode_rled,
};
use crate::records::spin::Spin;
use crate::wire::id::NovaId;

/// A `spïn`'s sprite sheet and the files each step came from.
#[derive(Debug)]
pub struct SpinSheet<'a> {
    /// The `spïn`'s ID.
    pub spin_id: i16,
    /// The `spïn`'s `SpritesID`: the sheet's `rlëD`, or its sprites `PICT`
    /// ([`SheetKind`] says which).
    pub image_id: i16,
    /// What the sheet was read from.
    pub kind: SheetKind,
    /// The decoded sheet, laid out with the `spïn`'s `xTiles` as its
    /// columns.
    pub sheet: SpriteSheet,
    /// The file the `spïn` came from.
    pub spin: &'a SourceFile,
    /// The file the `rlëD`, or the sprites `PICT`, came from.
    pub sheet_source: &'a SourceFile,
}

/// What a `spïn`'s sheet was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SheetKind {
    /// The `rlëD` named by its `SpritesID`.
    Rled,
    /// The sprites `PICT` named by its `SpritesID` and the mask `PICT`
    /// `mask_id` (its `MasksID`, which may be the sprites themselves), cut
    /// into frames as the original does with no `rlëD`
    /// ([`cut_pict_sheet`]).
    PictPair {
        /// The mask `PICT`'s ID.
        mask_id: i16,
    },
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
    /// be cut into frames (reported against the sprites).
    #[error("PICT {pict_id}: {source}")]
    Picture {
        /// The `PICT`'s ID.
        pict_id: i16,
        /// What went wrong.
        source: GraphicsError,
    },
}

/// A `MasksID` for an error message: the ID, or "none".
pub(super) fn mask_label(mask_id: Option<i16>) -> String {
    mask_id.map_or_else(|| "none".to_owned(), |id| id.to_string())
}

impl GameData {
    /// Resolves `spïn` `spin_id` to its sprite sheet, as the original's
    /// `_LoadMasterStellarSprite` (@0x1eb72) and `_LoadSprites` load one.
    /// The `spïn` must exist and decode. Its `SpritesID` names the sheet:
    ///
    /// - An `rlëD` of that ID, when there is one (`_ResourceExists`
    ///   @0x1ebee), decoded with the `spïn`'s `xTiles` as the columns. An
    ///   `rlëD` that fails is an error, with no fall back to a `PICT`.
    /// - Otherwise the sprites `PICT` of that ID and its mask, cut into
    ///   frames of the `spïn`'s `xSize` x `ySize` ([`cut_pict_sheet`];
    ///   `_SWCreateSpriteFromSinglePictXY` @0x1ece3). The mask is the
    ///   sprites themselves when `MasksID` is `SpritesID`, else the `PICT`
    ///   `MasksID`.
    ///
    /// An `rlëD`'s `xSize` and `ySize` are not checked against it: its
    /// decoded frames are authoritative. The sheet is decoded on every call
    /// and not cached.
    pub fn spin_sheet(&self, spin_id: i16) -> Result<SpinSheet<'_>, SpinSpriteError> {
        let spin = self
            .get::<Spin>(spin_id)
            .ok_or(SpinSpriteError::NoSpin(spin_id))?
            .map_err(|e| SpinSpriteError::Decode(e.clone()))?;
        let record = spin.record;
        let image_id = record.sprites_id;
        let (kind, sheet, found) = if let Some(found) = self.resource(RLED, image_id) {
            let sheet = decode_rled(found.resource.data(), record.sheet_layout())
                .map_err(|source| SpinSpriteError::Graphics { image_id, source })?;
            (SheetKind::Rled, sheet, found)
        } else {
            let found = self
                .resource(PICT, image_id)
                .ok_or(SpinSpriteError::NoSheet {
                    spin: spin_id,
                    image_id,
                })?;
            let (mask_id, sheet) = self.pict_pair(spin_id, record, found.resource.data())?;
            (SheetKind::PictPair { mask_id }, sheet, found)
        };
        Ok(SpinSheet {
            spin_id,
            image_id,
            kind,
            sheet,
            spin: spin.source,
            sheet_source: found.source,
        })
    }

    /// Cuts `spïn` `spin_id`'s sprites `PICT` (`sprites`, its bytes) and
    /// its mask into a sheet, with the mask's ID.
    fn pict_pair(
        &self,
        spin_id: i16,
        record: &Spin,
        sprites: &[u8],
    ) -> Result<(i16, SpriteSheet), SpinSpriteError> {
        let image_id = record.sprites_id;
        let decode = |pict_id: i16, data: &[u8]| {
            decode_pict(data).map_err(|source| SpinSpriteError::Picture { pict_id, source })
        };
        let sprites = decode(image_id, sprites)?;
        let no_mask = SpinSpriteError::NoMask {
            spin: spin_id,
            mask_id: record.masks_id.map(NovaId::raw),
        };
        let mask_id = record.masks_id.ok_or(no_mask.clone())?.raw();
        let mask: Option<Image> = if mask_id == image_id {
            None
        } else {
            let found = self.resource(PICT, mask_id).ok_or(no_mask)?;
            Some(decode(mask_id, found.resource.data())?)
        };
        let mask = mask.as_ref().map_or(SheetMask::Itself, SheetMask::Picture);
        let sheet = cut_pict_sheet(
            &sprites,
            mask,
            record.x_size,
            record.y_size,
            record.sheet_layout(),
        )
        .map_err(|source| SpinSpriteError::Picture {
            pict_id: image_id,
            source,
        })?;
        Ok((mask_id, sheet))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_rsrc::ResType;
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::fixture::forks::MemForks;

    use super::*;
    use crate::decode::Record;
    use crate::graphics::fixture::{RledBuilder, rgb888_pict};
    use crate::graphics::{PICT, decode_pict};
    use crate::store::fixture::FakeTree;
    use crate::store::fs::EntryKind::File;
    use crate::testutil::buf;

    /// A `spïn` naming sprites `image`, `width` x `height` each, in a grid
    /// `x_tiles` across and `y_tiles` down.
    fn spin(image: i16, width: i16, height: i16, x_tiles: i16, y_tiles: i16) -> Vec<u8> {
        masked_spin(image, -1, width, height, x_tiles, y_tiles)
    }

    /// A `spïn` as [`spin`] gives, with masks `mask`.
    fn masked_spin(
        image: i16,
        mask: i16,
        width: i16,
        height: i16,
        x_tiles: i16,
        y_tiles: i16,
    ) -> Vec<u8> {
        buf::<Spin>()
            .i16(0x00, image)
            .i16(0x02, mask)
            .i16(0x04, width)
            .i16(0x06, height)
            .i16(0x08, x_tiles)
            .i16(0x0A, y_tiles)
            .0
    }

    /// Pixel `i` of a [`distinct`] picture, distinct at five bits.
    fn nth(i: usize) -> [u8; 3] {
        [(i * 8) as u8, 0x40, 0x80]
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

    /// The colour picture pixel `i` draws as: each channel's top five
    /// bits, widened again by bit replication (0x40 is 8, which widens to
    /// 66; 0x80 is 16, which widens to 132).
    fn drawn(i: usize) -> [u8; 4] {
        [((i << 3) | (i >> 2)) as u8, 66, 132, 255]
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
        let mut forks = MemForks::new()
            .file(
                "/d/spins",
                fork(&[(Spin::TYPE, 3000, spin(2000, 2, 1, 6, 6))]),
            )
            .file("/d/sprites", fork(&[(RLED, 2000, sheet(36, 0x7C00))]));
        for (name, bytes) in extra {
            forks = forks.file(format!("/p/{name}"), bytes.clone());
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
    fn a_spin_with_neither_sheet_is_no_sheet() {
        let nothing = fork(&[(Spin::TYPE, 3000, masked_spin(2001, 2101, 2, 1, 6, 6))]);
        let err = store(&[("Grid", nothing)])
            .spin_sheet(3000)
            .expect_err("no rlëD or PICT 2001");
        assert_eq!(
            err,
            SpinSpriteError::NoSheet {
                spin: 3000,
                image_id: 2001
            }
        );
        assert_eq!(
            err.to_string(),
            "spïn 3000: no rlëD or PICT 2001 for its sprites"
        );
    }

    /// A plug-in `Grid` giving `spïn` 3000 sprites `PICT` 2001 (`sprites`),
    /// masks `masks`, 2x1 frames in a 2 x 2 grid, plus `extra` resources.
    fn pict_grid(masks: i16, sprites: Vec<u8>, extra: &[(ResType, i16, Vec<u8>)]) -> GameData {
        let mut resources = vec![
            (Spin::TYPE, 3000, masked_spin(2001, masks, 2, 1, 2, 2)),
            (PICT, 2001, sprites),
        ];
        resources.extend_from_slice(extra);
        store(&[("Grid", fork(&resources))])
    }

    #[test]
    fn a_spin_with_no_rled_resolves_to_the_frames_its_pict_pair_cuts() {
        let data = pict_grid(2101, distinct(6, 2), &[(PICT, 2101, mask(6, 2, &[(3, 1)]))]);
        let sheet = data.spin_sheet(3000).expect("resolves");
        assert_eq!((sheet.spin_id, sheet.image_id), (3000, 2001));
        assert_eq!(sheet.kind, SheetKind::PictPair { mask_id: 2101 });
        // Three columns from the picture, not the spïn's two tiles.
        assert_eq!(sheet.frame_count(), 6);
        assert_eq!(
            (sheet.sheet.frame_width(), sheet.sheet.frame_height()),
            (2, 1)
        );
        assert_eq!(sheet.sheet.layout().columns(), 2);
        // Frame 4 is the cell at (2, 1); the mask hides its right pixel.
        let frame = &sheet.sheet.frames()[4];
        assert_eq!(frame.pixel(0, 0), Some(drawn(8)));
        assert_eq!(frame.pixel(1, 0), Some([0; 4]));
        assert_eq!(sheet.sheet.frames()[1].pixel(1, 0), Some(drawn(3)));
        assert_eq!(sheet.sheet.solid_frames(), None);
        assert_eq!(sheet.spin.path, Path::new("/p/Grid"));
        assert_eq!(sheet.sheet_source.path, Path::new("/p/Grid"));
    }

    #[test]
    fn an_rled_wins_over_a_pict_of_the_same_id() {
        let data = store(&[(
            "Picture",
            fork(&[(PICT, 2000, distinct(2, 1)), (PICT, 2001, mask(2, 1, &[]))]),
        )]);
        let sheet = data.spin_sheet(3000).expect("resolves");
        assert_eq!(sheet.kind, SheetKind::Rled);
        assert_eq!(sheet.frame_count(), 36);
        assert_eq!(sheet.sheet_source.path, Path::new("/d/sprites"));
    }

    #[test]
    fn a_spin_masked_by_its_own_sprites_hides_their_white() {
        let pixels = [[0xFF; 3], [0x80, 0x40, 0x20], [0xF8, 0xFF, 0xFC], [0; 3]];
        let data = pict_grid(2001, rgb888_pict(4, 1, &pixels), &[]);
        let sheet = data.spin_sheet(3000).expect("resolves");
        assert_eq!(sheet.kind, SheetKind::PictPair { mask_id: 2001 });
        let frames = sheet.sheet.frames();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].pixel(0, 0), Some([0; 4]));
        assert_eq!(frames[0].pixel(1, 0), Some([132, 66, 33, 255]));
        assert_eq!(frames[1].pixel(0, 0), Some([0; 4]));
        assert_eq!(frames[1].pixel(1, 0), Some([0, 0, 0, 255]));
    }

    #[test]
    fn a_pict_pair_with_no_mask_is_no_mask() {
        let err = pict_grid(-1, distinct(6, 2), &[])
            .spin_sheet(3000)
            .expect_err("no mask");
        assert_eq!(
            err,
            SpinSpriteError::NoMask {
                spin: 3000,
                mask_id: None
            }
        );
        assert_eq!(
            err.to_string(),
            "spïn 3000: no mask PICT none for its sprites"
        );
        let err = pict_grid(2101, distinct(6, 2), &[])
            .spin_sheet(3000)
            .expect_err("no mask");
        assert_eq!(
            err,
            SpinSpriteError::NoMask {
                spin: 3000,
                mask_id: Some(2101)
            }
        );
        assert_eq!(
            err.to_string(),
            "spïn 3000: no mask PICT 2101 for its sprites"
        );
    }

    #[test]
    fn a_pict_that_does_not_decode_or_cut_is_a_picture_error() {
        let junk = vec![0; 4];
        let short = decode_pict(&junk).expect_err("short");
        // The sprites.
        let data = pict_grid(2101, junk.clone(), &[(PICT, 2101, mask(6, 2, &[]))]);
        let err = data.spin_sheet(3000).expect_err("bad sprites");
        assert_eq!(
            err,
            SpinSpriteError::Picture {
                pict_id: 2001,
                source: short.clone()
            }
        );
        assert_eq!(err.to_string(), format!("PICT 2001: {short}"));
        // The mask.
        let data = pict_grid(2101, distinct(6, 2), &[(PICT, 2101, junk)]);
        let err = data.spin_sheet(3000).expect_err("bad mask");
        assert_eq!(
            err,
            SpinSpriteError::Picture {
                pict_id: 2101,
                source: short
            }
        );
        // The cut.
        let data = pict_grid(2101, distinct(5, 2), &[(PICT, 2101, mask(5, 2, &[]))]);
        let err = data.spin_sheet(3000).expect_err("a partial grid");
        assert_eq!(
            err,
            SpinSpriteError::Picture {
                pict_id: 2001,
                source: GraphicsError::PartialGrid {
                    width: 5,
                    height: 2,
                    frame_width: 2,
                    frame_height: 1,
                },
            }
        );
        assert_eq!(
            err.to_string(),
            "PICT 2001: the 5x2 picture is not a whole grid of 2x1 frames"
        );
        let data = pict_grid(2101, distinct(6, 2), &[(PICT, 2101, mask(6, 1, &[]))]);
        assert!(matches!(
            data.spin_sheet(3000),
            Err(SpinSpriteError::Picture {
                pict_id: 2001,
                source: GraphicsError::MaskSizeMismatch { .. },
            })
        ));
    }

    #[test]
    fn an_8_bit_sheet_resolves() {
        let eight_bit = RledBuilder::new(2, 1)
            .depth(8)
            .frame(|f| f.line().pixels8(&[0x00, 0x7C]))
            .build();
        let data = store(&[("Old Paint", fork(&[(RLED, 2000, eight_bit)]))]);
        let sheet = data.spin_sheet(3000).expect("resolves");
        assert_eq!(sheet.kind, SheetKind::Rled);
        assert_eq!(sheet.frame_count(), 1);
        let frame = &sheet.sheet.frames()[0];
        assert_eq!(frame.pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(frame.pixel(1, 0), Some([0; 4]));
    }

    #[test]
    fn a_corrupt_sheet_is_a_graphics_error() {
        let deep = RledBuilder::new(2, 1).depth(32).build();
        let data = store(&[("Bad Paint", fork(&[(RLED, 2000, deep)]))]);
        let err = data.spin_sheet(3000).expect_err("32-bit");
        assert_eq!(
            err,
            SpinSpriteError::Graphics {
                image_id: 2000,
                source: GraphicsError::UnsupportedDepth { depth: 32 },
            }
        );
        assert_eq!(err.to_string(), "rlëD 2000: unsupported rlëD depth 32");
    }

    #[test]
    fn a_corrupt_rled_does_not_fall_back_to_its_pict_pair() {
        // `spïn` 3000 masked by `PICT` 2100, with a good `PICT` pair at
        // its `SpritesID` and `MasksID`.
        let pair = [
            (Spin::TYPE, 3000, masked_spin(2000, 2100, 2, 1, 2, 2)),
            (PICT, 2000, distinct(6, 2)),
            (PICT, 2100, mask(6, 2, &[])),
        ];
        // On its own, with no `rlëD` 2000, the pair resolves.
        let no_rled = FakeTree::new().dir("/d", &[("Pair", File)]);
        let no_rled = GameData::load(
            &no_rled,
            &MemForks::new().file("/d/Pair", fork(&pair)),
            Path::new("/d"),
            None,
        )
        .expect("opens");
        let sheet = no_rled.spin_sheet(3000).expect("the pair resolves");
        assert_eq!(sheet.kind, SheetKind::PictPair { mask_id: 2100 });
        // With a corrupt `rlëD` 2000 beside it, the `rlëD` is still the
        // sheet (`_ResourceExists` @0x1ebee), and its failure the error.
        let mut resources = pair.to_vec();
        resources.push((RLED, 2000, RledBuilder::new(2, 1).depth(32).build()));
        let err = store(&[("Bad Paint", fork(&resources))])
            .spin_sheet(3000)
            .expect_err("the rlëD fails, not the pair resolves");
        assert_eq!(
            err,
            SpinSpriteError::Graphics {
                image_id: 2000,
                source: GraphicsError::UnsupportedDepth { depth: 32 },
            }
        );
    }
}
