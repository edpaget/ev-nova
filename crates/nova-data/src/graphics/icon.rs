//! `cicn`: colour icons with a 1-bit mask.
//!
//! Source: *Inside Macintosh: Imaging With QuickDraw* (1994) ch. 4, the
//! `CIcon` record and "The Color Icon Resource": a `PixMap` (with
//! `baseAddr`), a mask `BitMap`, a 1-bit icon `BitMap`, an `iconData`
//! handle, then the mask bits, the icon bits, the colour table and the
//! unpacked pixel rows.

use super::GraphicsError;
use super::budget::check_budget;
use super::color::{ColorTable, indices};
use super::image::Image;
use super::pixmap::{PixMap, QdRect};
use super::reader::Reader;

/// Decodes a `cicn` to an image the size of its `PixMap` bounds, with the
/// mask as alpha: masked-out pixels are fully transparent black.
pub fn decode_cicn(data: &[u8]) -> Result<Image, GraphicsError> {
    let mut r = Reader::new(data);
    r.skip(4)?; // baseAddr
    let pm = PixMap::read(&mut r)?;
    let (mask_row_bytes, mask_bounds) = read_bitmap(&mut r)?;
    let (icon_row_bytes, icon_bounds) = read_bitmap(&mut r)?;
    r.skip(4)?; // iconData
    let (width, height) = checked_indexed_size(&pm, data.len())?;
    if mask_bounds != pm.bounds {
        return Err(GraphicsError::MaskMismatch);
    }
    check_row_bytes(mask_row_bytes, width.div_ceil(8))?;

    let mask = r.bytes(usize::from(mask_row_bytes) * height as usize)?;
    let icon_rows = usize::try_from(icon_bounds.height()).unwrap_or(0);
    r.skip(usize::from(icon_row_bytes) * icon_rows)?;
    let ctab = ColorTable::read(&mut r)?;
    let pixels = r.bytes(usize::from(pm.row_bytes) * height as usize)?;

    let mask_rows = mask.chunks(usize::from(mask_row_bytes));
    indexed_image(pixels, &pm, &ctab, width, height, mask_rows)
}

/// A `BitMap` after its `baseAddr`: `rowBytes` and bounds.
fn read_bitmap(r: &mut Reader<'_>) -> Result<(u16, QdRect), GraphicsError> {
    r.skip(4)?; // baseAddr
    Ok((r.u16()?, QdRect::read(r)?))
}

/// The size of an indexed (1, 2, 4 or 8-bit) `PixMap`, checked against the
/// budget for `input_len` bytes and against its `rowBytes`.
pub(super) fn checked_indexed_size(
    pm: &PixMap,
    input_len: usize,
) -> Result<(u32, u32), GraphicsError> {
    let (width, height) = pm.bounds.size()?;
    check_budget(u64::from(width) * u64::from(height), input_len)?;
    if !matches!(pm.pixel_size, 1 | 2 | 4 | 8) {
        return Err(GraphicsError::UnsupportedPixMap {
            pixel_size: pm.pixel_size,
            pack_type: pm.pack_type,
            cmp_count: pm.cmp_count,
        });
    }
    check_row_bytes(pm.row_bytes, (width * u32::from(pm.pixel_size)).div_ceil(8))?;
    Ok((width, height))
}

fn check_row_bytes(row_bytes: u16, needed: u32) -> Result<(), GraphicsError> {
    if u32::from(row_bytes) < needed {
        return Err(GraphicsError::BadRowBytes { row_bytes, needed });
    }
    Ok(())
}

/// Colours unpacked indexed `pixels` rows (`rowBytes` each) through `ctab`.
/// Each row of `mask` (one bit per pixel, 1 opaque) masks a pixel row;
/// pixels without a mask row are opaque.
pub(super) fn indexed_image<'m>(
    pixels: &[u8],
    pm: &PixMap,
    ctab: &ColorTable,
    width: u32,
    height: u32,
    mut mask: impl Iterator<Item = &'m [u8]>,
) -> Result<Image, GraphicsError> {
    let mut image = Image::transparent(width, height);
    let rows = pixels.chunks(usize::from(pm.row_bytes));
    for (y, row) in (0..height).zip(rows) {
        let values = indices(row, pm.pixel_size, width as usize).unwrap_or_default();
        let opaque = match mask.next() {
            Some(bits) => indices(bits, 1, width as usize).unwrap_or_default(),
            None => vec![1; width as usize],
        };
        for ((x, value), opaque) in (0..width).zip(values).zip(opaque) {
            if opaque == 1 {
                image.put(x, y, ctab.rgba(value)?);
            }
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::fixture::{Cicn, Ctab};

    fn rgba(image: &Image) -> Vec<[u8; 4]> {
        image
            .pixels()
            .chunks(4)
            .map(|p| p.try_into().unwrap())
            .collect()
    }

    const CLEAR: [u8; 4] = [0; 4];

    /// A 2x2 2-bit icon, written out by hand.
    fn hand_written() -> Vec<u8> {
        [
            &[0x00; 4][..],                                    // baseAddr
            &[0x80, 0x01],                                     // rowBytes 1, PixMap
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02], // bounds 2x2
            &[0x00; 4],                                        // pmVersion, packType
            &[0x00; 4],                                        // packSize
            &[0x00, 0x48, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00], // hRes, vRes
            &[0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0x02], // pixelType, size 2
            &[0x00; 12],                                       // planeBytes .. pmReserved
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x01],             // mask: baseAddr, rowBytes 1
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02], // mask bounds
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x01],             // icon: baseAddr, rowBytes 1
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02], // icon bounds
            &[0x00; 4],                                        // iconData
            &[0x80, 0xC0],                                     // mask rows
            &[0x40, 0x00],                                     // icon rows
            &[0x00; 6],                                        // ctSeed, ctFlags
            &[0x00, 0x01],                                     // ctSize: two entries
            &[0x00, 0x03, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00], // 3: green
            &[0x00, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // 1: white
            &[0xD0, 0x70],                                     // pixels 3 1 / 1 3
        ]
        .concat()
    }

    #[test]
    fn decodes_a_hand_written_icon() {
        let (green, white) = ([0, 255, 0, 255], [255, 255, 255, 255]);
        let image = decode_cicn(&hand_written()).unwrap();
        assert_eq!((image.width(), image.height()), (2, 2));
        assert_eq!(rgba(&image), [green, CLEAR, white, green]);
    }

    /// The fixture writes the same bytes, including the icon rows (the
    /// inverse of the mask), which the decoder skips.
    #[test]
    fn the_fixture_writes_the_hand_written_layout() {
        let ctab = Ctab {
            device: false,
            entries: vec![(3, [0, 0xFFFF, 0]), (1, [0xFFFF; 3])],
        };
        let cicn = Cicn::new(2, 2, 2, &ctab, &[3, 1, 1, 3], &[true, false, true, true]);
        assert_eq!(cicn.bytes(), hand_written());
    }

    fn ctab() -> Ctab {
        Ctab {
            device: false,
            entries: vec![
                (200, [0x0100, 0x0200, 0x0300]),
                (1, [0x1100, 0x1200, 0x1300]),
                (0, [0x2100, 0x2200, 0x2300]),
                (3, [0x3100, 0x3200, 0x3300]),
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

    /// A 3x2 icon whose mask clears two pixels.
    fn icon(bits: u16) -> (Cicn, Vec<[u8; 4]>) {
        let indices = if bits == 8 {
            vec![200, 0, 1, 3, 3, 200]
        } else {
            vec![1, 0, 1, 3 % (1 << bits), 0, 1]
        };
        let mask = [true, false, true, true, true, false];
        let expected = indices
            .iter()
            .zip(mask)
            .map(|(&i, m)| if m { colour(i) } else { CLEAR })
            .collect();
        (Cicn::new(3, 2, bits, &ctab(), &indices, &mask), expected)
    }

    #[test]
    fn every_indexed_depth_decodes_with_the_mask_as_alpha() {
        for bits in [1, 2, 4, 8] {
            let (cicn, expected) = icon(bits);
            let image = decode_cicn(&cicn.bytes()).unwrap();
            assert_eq!((image.width(), image.height()), (3, 2));
            assert_eq!(rgba(&image), expected, "{bits}-bit");
        }
    }

    #[test]
    fn masked_out_pixels_are_transparent_even_with_a_colour() {
        let mask = [false; 4];
        let cicn = Cicn::new(2, 2, 8, &ctab(), &[200, 1, 0, 3], &mask);
        assert_eq!(rgba(&decode_cicn(&cicn.bytes()).unwrap()), [CLEAR; 4]);
    }

    #[test]
    fn padded_rows_are_skipped_by_their_row_bytes() {
        let (mut cicn, expected) = icon(4);
        cicn.row_bytes = 6;
        cicn.mask_row_bytes = 3;
        cicn.icon_row_bytes = 5;
        assert_eq!(rgba(&decode_cicn(&cicn.bytes()).unwrap()), expected);
    }

    #[test]
    fn trailing_bytes_are_ignored() {
        let (cicn, expected) = icon(2);
        let mut bytes = cicn.bytes();
        bytes.extend([0xFF; 5]);
        assert_eq!(rgba(&decode_cicn(&bytes).unwrap()), expected);
    }

    #[test]
    fn each_truncated_section_is_an_unexpected_end() {
        let (cicn, _) = icon(8);
        let bytes = cicn.bytes();
        let end = |len: usize| decode_cicn(&bytes[..len]).unwrap_err();
        // Mask rows at 82, icon rows at 84, colour table at 86, pixels at
        // 86 + 8 + 4 * 8 = 126.
        assert_eq!(bytes.len(), 132);
        assert_eq!(end(81), GraphicsError::UnexpectedEnd { offset: 78 });
        assert_eq!(end(83), GraphicsError::UnexpectedEnd { offset: 82 });
        assert_eq!(end(85), GraphicsError::UnexpectedEnd { offset: 84 });
        assert_eq!(end(125), GraphicsError::UnexpectedEnd { offset: 94 });
        assert_eq!(end(131), GraphicsError::UnexpectedEnd { offset: 126 });
        for len in 0..bytes.len() {
            assert!(
                matches!(
                    decode_cicn(&bytes[..len]),
                    Err(GraphicsError::UnexpectedEnd { .. })
                ),
                "{len}"
            );
        }
    }

    #[test]
    fn a_mask_with_different_bounds_is_rejected() {
        let (cicn, _) = icon(4);
        for at in [56, 58, 60, 62] {
            let mut bytes = cicn.bytes();
            bytes[at + 1] += 1;
            assert_eq!(
                decode_cicn(&bytes),
                Err(GraphicsError::MaskMismatch),
                "{at}"
            );
        }
    }

    #[test]
    fn rows_too_narrow_for_their_pixels_are_rejected() {
        let (mut cicn, _) = icon(8);
        cicn.row_bytes = 2;
        assert_eq!(
            decode_cicn(&cicn.bytes()),
            Err(GraphicsError::BadRowBytes {
                row_bytes: 2,
                needed: 3
            })
        );
        let mut cicn = Cicn::new(9, 1, 8, &ctab(), &[0; 9], &[true; 9]);
        cicn.mask_row_bytes = 1;
        assert_eq!(
            decode_cicn(&cicn.bytes()),
            Err(GraphicsError::BadRowBytes {
                row_bytes: 1,
                needed: 2
            })
        );
    }

    #[test]
    fn depths_other_than_1_2_4_and_8_are_unsupported() {
        for bits in [0, 3, 16, 32] {
            let mut cicn = Cicn::new(2, 1, 8, &ctab(), &[0, 1], &[true; 2]);
            cicn.bits = bits;
            assert_eq!(
                decode_cicn(&cicn.bytes()),
                Err(GraphicsError::UnsupportedPixMap {
                    pixel_size: bits,
                    pack_type: 0,
                    cmp_count: 1
                }),
                "{bits}"
            );
        }
    }

    #[test]
    fn an_icon_bitmap_is_not_a_pixmap() {
        let (cicn, _) = icon(4);
        let mut bytes = cicn.bytes();
        bytes[4] &= 0x7F;
        assert_eq!(
            decode_cicn(&bytes),
            Err(GraphicsError::NotAPixMap { offset: 4 })
        );
    }

    #[test]
    fn a_pixel_missing_from_the_table_is_named() {
        let cicn = Cicn::new(2, 1, 8, &ctab(), &[0, 2], &[true; 2]);
        assert_eq!(
            decode_cicn(&cicn.bytes()),
            Err(GraphicsError::MissingColour { index: 2 })
        );
    }

    #[test]
    fn empty_and_huge_icons_are_rejected_before_reading_pixels() {
        let mut cicn = Cicn::new(2, 1, 8, &ctab(), &[0, 1], &[true; 2]);
        cicn.height = 0;
        assert_eq!(
            decode_cicn(&cicn.bytes()),
            Err(GraphicsError::BadDimensions {
                width: 2,
                height: 0
            })
        );
        cicn.height = 32767;
        cicn.width = 32767;
        let bytes = cicn.bytes();
        assert_eq!(
            decode_cicn(&bytes[..120]),
            Err(GraphicsError::TooLarge {
                pixels: 32767 * 32767,
                input_len: 120
            })
        );
    }

    #[test]
    fn corrupt_icons_never_panic() {
        let (cicn, _) = icon(4);
        crate::sweep::assert_never_panics(&cicn.bytes(), decode_cicn);
    }
}
