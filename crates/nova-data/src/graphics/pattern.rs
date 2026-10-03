//! `ppat`: pixel patterns.
//!
//! Source: *Inside Macintosh: Imaging With QuickDraw* (1994) ch. 4, the
//! `PixPat` record and "The Pixel Pattern Resource": `patType`, then the
//! offsets of the `PixMap` (`patMap`) and its pixel data (`patData`); the
//! colour table sits at the `PixMap`'s `pmTable` offset. Only full-colour
//! patterns (`patType` 1) with indexed, unpacked pixels are decoded.

use super::GraphicsError;
use super::color::ColorTable;
use super::icon::{checked_indexed_size, indexed_image};
use super::image::Image;
use super::pixmap::PixMap;
use super::reader::Reader;

/// `patType` of a full-colour pixel pattern.
const FULL_COLOUR: u16 = 1;

/// Decodes a `ppat` to an opaque image the size of its `PixMap` bounds.
pub fn decode_ppat(data: &[u8]) -> Result<Image, GraphicsError> {
    let mut r = Reader::new(data);
    let pat_type = r.u16()?;
    let pat_map = r.u32()?;
    let pat_data = r.u32()?;
    r.skip(4)?; // patXData
    r.skip(2)?; // patXValid
    r.skip(4)?; // patXMap
    r.skip(8)?; // pat1Data: the 1-bit fallback pattern
    if pat_type != FULL_COLOUR {
        return Err(GraphicsError::UnsupportedPatternType { pat_type });
    }

    r.seek_to(pat_map as usize)?;
    r.skip(4)?; // baseAddr
    let pm = PixMap::read(&mut r)?;
    let (width, height) = checked_indexed_size(&pm, data.len())?;
    r.seek_to(pm.pm_table as usize)?;
    let ctab = ColorTable::read(&mut r)?;
    r.seek_to(pat_data as usize)?;
    let pixels = r.bytes(usize::from(pm.row_bytes) * height as usize)?;
    indexed_image(pixels, &pm, &ctab, width, height, std::iter::empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::fixture::{Ctab, Ppat};

    fn rgba(image: &Image) -> Vec<[u8; 4]> {
        image
            .pixels()
            .chunks(4)
            .map(|p| p.try_into().unwrap())
            .collect()
    }

    /// A 2x2 8-bit pattern, written out by hand, with its colour table
    /// before its pixels (the reverse of the stock layout).
    #[test]
    fn decodes_a_hand_written_pattern_by_its_offsets() {
        let bytes = [
            &[0x00, 0x01][..],                                 // patType 1
            &[0x00, 0x00, 0x00, 0x1C],                         // patMap 28
            &[0x00, 0x00, 0x00, 0x66],                         // patData 102
            &[0x00; 4],                                        // patXData
            &[0xFF, 0xFF],                                     // patXValid
            &[0x00; 4],                                        // patXMap
            &[0xAA; 8],                                        // pat1Data
            &[0x00; 4],                                        // 28: baseAddr
            &[0x80, 0x02],                                     // rowBytes 2
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02], // bounds 2x2
            &[0x00; 4],                                        // pmVersion, packType
            &[0x00; 12],                                       // packSize, hRes, vRes
            &[0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00, 0x08], // pixelType, size 8
            &[0x00; 4],                                        // planeBytes
            &[0x00, 0x00, 0x00, 0x4E],                         // pmTable 78
            &[0x00; 4],                                        // pmReserved
            &[0x00; 6],                                        // 78: ctSeed, ctFlags
            &[0x00, 0x01],                                     // ctSize: two entries
            &[0x00, 0x09, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00], // 9: red
            &[0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF], // 4: blue
            &[0x09, 0x04, 0x04, 0x09],                         // 102: pixels
        ]
        .concat();
        let (red, blue) = ([255, 0, 0, 255], [0, 0, 255, 255]);
        let image = decode_ppat(&bytes).unwrap();
        assert_eq!((image.width(), image.height()), (2, 2));
        assert_eq!(rgba(&image), [red, blue, blue, red]);
    }

    fn ctab() -> Ctab {
        Ctab {
            device: false,
            entries: vec![
                (3, [0x1000, 0x2000, 0x3000]),
                (0, [0x4000, 0x5000, 0x6000]),
                (1, [0x7000, 0x8000, 0x9000]),
                (77, [0xA000, 0xB000, 0xC000]),
            ],
        }
    }

    fn colour(index: u8) -> [u8; 4] {
        let [r, g, b] = ctab()
            .entries
            .iter()
            .find(|e| e.0 == u16::from(index))
            .unwrap()
            .1
            .map(|c| (c >> 8) as u8);
        [r, g, b, 255]
    }

    #[test]
    fn every_indexed_depth_decodes_opaque() {
        for bits in [1, 2, 4, 8] {
            let indices: Vec<u8> = if bits == 8 {
                vec![77, 0, 1, 3, 3, 77, 0, 1, 1]
            } else {
                vec![1, 0, 1, 1, 0, 0, 0, 1, 1]
                    .into_iter()
                    .map(|i| if bits > 1 && i == 0 { 3 } else { i })
                    .collect()
            };
            let ppat = Ppat::new(3, 3, bits, &ctab(), &indices);
            let image = decode_ppat(&ppat.bytes()).unwrap();
            let expected: Vec<[u8; 4]> = indices.iter().map(|&i| colour(i)).collect();
            assert_eq!(rgba(&image), expected, "{bits}-bit");
        }
    }

    #[test]
    fn padded_rows_are_skipped_by_their_row_bytes() {
        let mut ppat = Ppat::new(3, 2, 4, &ctab(), &[0, 1, 3, 3, 1, 0]);
        ppat.row_bytes = 5;
        let image = decode_ppat(&ppat.bytes()).unwrap();
        let expected: Vec<[u8; 4]> = [0, 1, 3, 3, 1, 0].iter().map(|&i| colour(i)).collect();
        assert_eq!(rgba(&image), expected);
    }

    #[test]
    fn only_full_colour_patterns_are_supported() {
        for pat_type in [0, 2, 3] {
            let mut ppat = Ppat::new(2, 1, 8, &ctab(), &[0, 1]);
            ppat.pat_type = pat_type;
            assert_eq!(
                decode_ppat(&ppat.bytes()),
                Err(GraphicsError::UnsupportedPatternType { pat_type }),
                "{pat_type}"
            );
        }
    }

    #[test]
    fn offsets_past_the_end_are_unexpected_ends_at_that_offset() {
        let bytes = Ppat::new(2, 1, 8, &ctab(), &[0, 1]).bytes();
        // patMap, patData and pmTable.
        for at in [2, 6, 28 + 4 + 38] {
            for offset in [0xFFFF_FFFF_u32, bytes.len() as u32 + 1] {
                let mut hostile = bytes.clone();
                hostile[at..at + 4].copy_from_slice(&offset.to_be_bytes());
                assert_eq!(
                    decode_ppat(&hostile),
                    Err(GraphicsError::UnexpectedEnd {
                        offset: offset as usize
                    }),
                    "{at}"
                );
            }
        }
    }

    #[test]
    fn a_truncated_pattern_is_an_unexpected_end() {
        let bytes = Ppat::new(2, 1, 8, &ctab(), &[0, 1]).bytes();
        for len in 0..bytes.len() {
            assert!(
                matches!(
                    decode_ppat(&bytes[..len]),
                    Err(GraphicsError::UnexpectedEnd { .. })
                ),
                "{len}"
            );
        }
        // The header's unused fields must be present too.
        assert_eq!(
            decode_ppat(&bytes[..27]),
            Err(GraphicsError::UnexpectedEnd { offset: 20 })
        );
    }

    #[test]
    fn pixel_data_must_fit_before_the_end() {
        let bytes = Ppat::new(2, 2, 8, &ctab(), &[0, 1, 3, 77]).bytes();
        let mut moved = bytes.clone();
        // Point patData at the last byte: two rows of 2 bytes do not fit.
        let last = bytes.len() as u32 - 1;
        moved[6..10].copy_from_slice(&last.to_be_bytes());
        assert_eq!(
            decode_ppat(&moved),
            Err(GraphicsError::UnexpectedEnd {
                offset: last as usize
            })
        );
    }

    #[test]
    fn bad_pixmaps_are_rejected() {
        let mut ppat = Ppat::new(2, 1, 8, &ctab(), &[0, 1]);
        ppat.bits = 16;
        assert_eq!(
            decode_ppat(&ppat.bytes()),
            Err(GraphicsError::UnsupportedPixMap {
                pixel_size: 16,
                pack_type: 0,
                cmp_count: 1
            })
        );
        let mut ppat = Ppat::new(2, 1, 8, &ctab(), &[0, 1]);
        ppat.row_bytes = 1;
        assert_eq!(
            decode_ppat(&ppat.bytes()),
            Err(GraphicsError::BadRowBytes {
                row_bytes: 1,
                needed: 2
            })
        );
        let mut bytes = Ppat::new(2, 1, 8, &ctab(), &[0, 2]).bytes();
        assert_eq!(
            decode_ppat(&bytes),
            Err(GraphicsError::MissingColour { index: 2 })
        );
        bytes[32] &= 0x7F;
        assert_eq!(
            decode_ppat(&bytes),
            Err(GraphicsError::NotAPixMap { offset: 32 })
        );
    }

    #[test]
    fn empty_and_huge_patterns_are_rejected_before_reading_pixels() {
        let mut ppat = Ppat::new(2, 1, 8, &ctab(), &[0, 1]);
        ppat.width = 0;
        assert_eq!(
            decode_ppat(&ppat.bytes()),
            Err(GraphicsError::BadDimensions {
                width: 0,
                height: 1
            })
        );
        ppat.width = 32767;
        ppat.height = 32767;
        assert_eq!(
            decode_ppat(&ppat.bytes()),
            Err(GraphicsError::TooLarge {
                pixels: 32767 * 32767,
                input_len: ppat.bytes().len()
            })
        );
    }

    #[test]
    fn corrupt_patterns_never_panic() {
        let ppat = Ppat::new(3, 2, 2, &ctab(), &[0, 1, 3, 3, 1, 0]);
        crate::sweep::assert_never_panics(&ppat.bytes(), decode_ppat);
    }
}
