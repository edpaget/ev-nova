//! `PICT`: QuickDraw version 2 pictures.
//!
//! Source: *Inside Macintosh: Imaging With QuickDraw* (1994) appendix A,
//! "Picture Opcodes": the version 2 header, the opcode table and data
//! lengths, and word alignment of opcodes.

use std::borrow::Cow;

use super::GraphicsError;
use super::budget::check_budget;
use super::color::{ColorTable, indices, rgb555};
use super::image::Image;
use super::packbits::unpack;
use super::pixmap::{PixMap, QdRect};
use super::reader::Reader;

/// Offset of the version opcode, after `picSize` and `picFrame`.
const VERSION_OFFSET: usize = 10;
/// The version 2 opcode (`0x0011`) and its version word (`0x02FF`).
const VERSION_2: [u16; 2] = [0x0011, 0x02FF];

/// Decodes a version 2 `PICT` to an image the size of its `picFrame`.
///
/// The `picSize` word and the HeaderOp's rectangle are ignored. Opcodes
/// start at even offsets; bytes after EndPic are ignored.
///
/// # Example
///
/// ```
/// use nova_data::graphics::{GraphicsError, decode_pict};
///
/// // A 1x1 picture: one unpacked 16-bit red pixel, copied unchanged.
/// let pict: Vec<u8> = [
///     &[0, 0, 0, 0, 0, 0, 0, 1, 0, 1][..],   // picSize, picFrame
///     &[0x00, 0x11, 0x02, 0xFF],             // version 2
///     &[0x00, 0x9A, 0, 0, 0, 0xFF],          // DirectBitsRect, baseAddr
///     &[0x80, 0x02, 0, 0, 0, 0, 0, 1, 0, 1], // rowBytes 2, bounds
///     &[0, 0, 0, 3, 0, 0, 0, 0],             // pmVersion, packType 3, packSize
///     &[0; 8],                               // hRes, vRes
///     &[0, 16, 0, 16, 0, 3, 0, 5],           // pixelType, pixelSize 16, 3x5 bits
///     &[0; 12],                              // planeBytes, pmTable, pmReserved
///     &[0, 0, 0, 0, 0, 1, 0, 1],             // srcRect
///     &[0, 0, 0, 0, 0, 1, 0, 1],             // dstRect
///     &[0, 0],                               // mode: srcCopy
///     &[0x7C, 0x00],                         // the pixel (rowBytes < 8: unpacked)
///     &[0x00, 0xFF],                         // EndPic
/// ]
/// .concat();
/// let image = decode_pict(&pict)?;
/// assert_eq!(image.pixel(0, 0), Some([255, 0, 0, 255]));
///
/// // QuickTime-compressed pictures are not decoded; the error says so.
/// let mut quicktime = pict.clone();
/// quicktime[14..16].copy_from_slice(&[0x82, 0x00]);
/// assert_eq!(
///     decode_pict(&quicktime).unwrap_err().to_string(),
///     "unsupported PICT opcode 0x8200 at byte 0xe"
/// );
/// # Ok::<(), GraphicsError>(())
/// ```
pub fn decode_pict(data: &[u8]) -> Result<Image, GraphicsError> {
    let mut r = Reader::new(data);
    r.skip(2)?; // picSize: meaningless for pictures over 32 KB
    let frame = QdRect::read(&mut r)?;
    if [r.u16()?, r.u16()?] != VERSION_2 {
        return Err(GraphicsError::UnsupportedVersion {
            offset: VERSION_OFFSET,
        });
    }
    let (width, height) = frame.size()?;
    check_budget(u64::from(width) * u64::from(height), data.len())?;
    let mut canvas = Image::transparent(width, height);

    // Every opcode is at least two bytes, so the data runs out first.
    for _ in 0..data.len() {
        r.align2()?;
        let offset = r.pos();
        match r.u16()? {
            NOP | DEF_HILITE => {}
            SHORT_COMMENT => r.skip(2)?,
            LONG_COMMENT => {
                r.skip(2)?; // kind
                let len = r.u16()?;
                r.skip(usize::from(len))?;
            }
            HEADER_OP => r.skip(24)?,
            CLIP => skip_region(&mut r)?,
            DIRECT_BITS_RECT => direct_bits(&mut r, frame, &mut canvas)?,
            PACK_BITS_RECT => packbits_rect(&mut r, frame, &mut canvas)?,
            END_PIC => return Ok(canvas),
            opcode => return Err(GraphicsError::UnsupportedOpcode { opcode, offset }),
        }
    }
    Err(GraphicsError::UnexpectedEnd { offset: r.pos() })
}

const NOP: u16 = 0x0000;
const CLIP: u16 = 0x0001;
const DEF_HILITE: u16 = 0x001E;
const SHORT_COMMENT: u16 = 0x00A0;
const LONG_COMMENT: u16 = 0x00A1;
const END_PIC: u16 = 0x00FF;
const HEADER_OP: u16 = 0x0C00;
const PACK_BITS_RECT: u16 = 0x0098;
const DIRECT_BITS_RECT: u16 = 0x009A;

/// The smallest region: its size word and bounding rectangle.
const MIN_REGION: u16 = 10;

/// Skips a region: a size word that counts itself, then the rest.
fn skip_region(r: &mut Reader<'_>) -> Result<(), GraphicsError> {
    let offset = r.pos();
    let size = r.u16()?;
    if size < MIN_REGION {
        return Err(GraphicsError::BadRegion { size, offset });
    }
    r.skip(usize::from(size - 2))
}

/// `009A` DirectBitsRect: a 16-bit or 32-bit `PixMap` (after a 4-byte
/// `baseAddr`), the copy rectangles and mode, then the pixel rows.
fn direct_bits(r: &mut Reader<'_>, frame: QdRect, canvas: &mut Image) -> Result<(), GraphicsError> {
    r.skip(4)?; // baseAddr
    let pm = PixMap::read(r)?;
    let format = match (pm.pixel_size, pm.cmp_count, pm.pack_type) {
        (16, _, 0 | 3) => RowFormat::Rgb555,
        (32, 3, 0 | 4) if pm.row_bytes < MIN_PACKED_ROW_BYTES => RowFormat::ChunkyXrgb,
        (32, 3, 0 | 4) => RowFormat::PlanarRgb,
        _ => {
            return Err(GraphicsError::UnsupportedPixMap {
                pixel_size: pm.pixel_size,
                pack_type: pm.pack_type,
                cmp_count: pm.cmp_count,
            });
        }
    };
    copy_rows(r, &pm, &format, frame, canvas)
}

/// `0098` PackBitsRect: an indexed `PixMap` (with no `baseAddr`), its
/// colour table, the copy rectangles and mode, then the pixel rows.
fn packbits_rect(
    r: &mut Reader<'_>,
    frame: QdRect,
    canvas: &mut Image,
) -> Result<(), GraphicsError> {
    let pm = PixMap::read(r)?;
    if !matches!(pm.pixel_size, 1 | 2 | 4 | 8) {
        return Err(GraphicsError::UnsupportedPixMap {
            pixel_size: pm.pixel_size,
            pack_type: pm.pack_type,
            cmp_count: pm.cmp_count,
        });
    }
    let ctab = ColorTable::read(r)?;
    let format = RowFormat::Indexed {
        bits: pm.pixel_size,
        ctab: &ctab,
    };
    copy_rows(r, &pm, &format, frame, canvas)
}

/// srcCopy and ditherCopy, which both copy pixels unchanged.
const SRC_COPY: u16 = 0;
const DITHER_COPY: u16 = 64;

/// Below this `rowBytes`, rows are stored unpacked with no byte count.
const MIN_PACKED_ROW_BYTES: u16 = 8;
/// Above this `rowBytes`, row byte counts are `u16`s rather than `u8`s.
const MAX_BYTE_COUNT_ROW_BYTES: u16 = 250;

/// How a pixel map's rows are stored and turned into pixels.
enum RowFormat<'a> {
    /// 16-bit `xRRRRRGGGGGBBBBB`, packed by 16-bit words.
    Rgb555,
    /// 32-bit pixels packed by component: each row unpacks to a plane of
    /// red bytes, then green, then blue.
    PlanarRgb,
    /// 32-bit `xRGB` pixels, in rows too short to be packed.
    ChunkyXrgb,
    /// 1, 2, 4 or 8-bit pixel values looked up in a colour table.
    Indexed { bits: u16, ctab: &'a ColorTable },
}

impl RowFormat<'_> {
    /// Bytes one row of `width` pixels needs within `rowBytes`.
    fn needed(&self, width: u32) -> u32 {
        match self {
            Self::Rgb555 => 2 * width,
            Self::PlanarRgb | Self::ChunkyXrgb => 4 * width,
            Self::Indexed { bits, .. } => (width * u32::from(*bits)).div_ceil(8),
        }
    }

    /// The PackBits unit, in bytes.
    fn unit(&self) -> usize {
        match self {
            Self::Rgb555 => 2,
            Self::PlanarRgb | Self::ChunkyXrgb | Self::Indexed { .. } => 1,
        }
    }

    /// Bytes one packed row unpacks to: three planes for component
    /// packing, otherwise all of `rowBytes`.
    fn expected(&self, row_bytes: u16, width: u32) -> usize {
        match self {
            Self::PlanarRgb => 3 * width as usize,
            Self::Rgb555 | Self::ChunkyXrgb | Self::Indexed { .. } => usize::from(row_bytes),
        }
    }

    /// The pixels of one unpacked row.
    fn pixels(&self, row: &[u8], width: u32) -> Result<Vec<[u8; 4]>, GraphicsError> {
        Ok(match self {
            Self::Rgb555 => row
                .as_chunks::<2>()
                .0
                .iter()
                .take(width as usize)
                .map(|&word| rgb555(u16::from_be_bytes(word)))
                .collect(),
            Self::PlanarRgb => {
                let width = width as usize;
                let plane = |c: usize, x: usize| row.get(c * width + x).copied().unwrap_or(0);
                (0..width)
                    .map(|x| [plane(0, x), plane(1, x), plane(2, x), 255])
                    .collect()
            }
            Self::ChunkyXrgb => row
                .as_chunks::<4>()
                .0
                .iter()
                .take(width as usize)
                .map(|&[_, r, g, b]| [r, g, b, 255])
                .collect(),
            Self::Indexed { bits, ctab } => indices(row, *bits, width as usize)
                .unwrap_or_default()
                .into_iter()
                .map(|index| ctab.rgba(index))
                .collect::<Result<_, _>>()?,
        })
    }
}

/// Reads the copy rectangles, mode and rows of a pixel opcode whose
/// `PixMap` is `pm`, and copies its source rectangle onto `canvas`.
fn copy_rows(
    r: &mut Reader<'_>,
    pm: &PixMap,
    format: &RowFormat<'_>,
    frame: QdRect,
    canvas: &mut Image,
) -> Result<(), GraphicsError> {
    let src_offset = r.pos();
    let src = QdRect::read(r)?;
    let dst = QdRect::read(r)?;
    let mode = r.u16()?;
    if mode != SRC_COPY && mode != DITHER_COPY {
        return Err(GraphicsError::UnsupportedTransferMode { mode });
    }
    let (width, height) = pm.bounds.size()?;
    let scaled = src.width() != dst.width() || src.height() != dst.height();
    if scaled || !pm.bounds.contains(src) || !frame.contains(dst) {
        return Err(GraphicsError::BadCopyRect { offset: src_offset });
    }
    let needed = format.needed(width);
    if u32::from(pm.row_bytes) < needed {
        return Err(GraphicsError::BadRowBytes {
            row_bytes: pm.row_bytes,
            needed,
        });
    }
    let expected = format.expected(pm.row_bytes, width);
    for row in 0..height {
        let bytes = read_row(r, pm.row_bytes, format.unit(), expected, row)?;
        let y = i32::from(pm.bounds.top) + row as i32;
        if y < i32::from(src.top) || y >= i32::from(src.bottom) {
            continue;
        }
        let pixels = format.pixels(&bytes, width)?;
        let canvas_y = y - i32::from(src.top) + i32::from(dst.top) - i32::from(frame.top);
        for x in i32::from(src.left)..i32::from(src.right) {
            let canvas_x = x - i32::from(src.left) + i32::from(dst.left) - i32::from(frame.left);
            if let Some(&pixel) = pixels.get((x - i32::from(pm.bounds.left)) as usize) {
                canvas.put(canvas_x as u32, canvas_y as u32, pixel);
            }
        }
    }
    Ok(())
}

/// One row, unpacked to `expected` bytes. Rows are stored raw when
/// `rowBytes` < 8, otherwise as a byte count (a `u16` when `rowBytes` > 250,
/// else a `u8`) and that many bytes of PackBits data.
fn read_row<'a>(
    r: &mut Reader<'a>,
    row_bytes: u16,
    unit: usize,
    expected: usize,
    row: u32,
) -> Result<Cow<'a, [u8]>, GraphicsError> {
    if row_bytes < MIN_PACKED_ROW_BYTES {
        return Ok(Cow::Borrowed(r.bytes(usize::from(row_bytes))?));
    }
    let offset = r.pos();
    let count = if row_bytes > MAX_BYTE_COUNT_ROW_BYTES {
        r.u16()?
    } else {
        u16::from(r.u8()?)
    };
    let packed = r.bytes(usize::from(count))?;
    unpack(packed, unit, expected)
        .map(Cow::Owned)
        .ok_or(GraphicsError::BadPackedRow { row, offset })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::color::rgb555;
    use crate::graphics::fixture::{Ctab, DirectBits, IndexedBits, PictBuilder};

    const FRAME: [i16; 4] = [0, 0, 3, 5];

    fn transparent(width: u32, height: u32) -> Image {
        Image::from_rgba(width, height, vec![0; (width * height * 4) as usize]).unwrap()
    }

    /// The smallest valid picture, written out by hand.
    #[test]
    fn decodes_a_hand_written_empty_picture() {
        let bytes = [
            0x00, 0x00, // picSize (ignored)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x03, // picFrame 3x2
            0x00, 0x11, 0x02, 0xFF, // version 2
            0x00, 0xFF, // EndPic
        ];
        assert_eq!(decode_pict(&bytes), Ok(transparent(3, 2)));
    }

    #[test]
    fn the_image_is_the_size_of_the_frame() {
        let pict = PictBuilder::new([10, 20, 13, 25]).end().build();
        assert_eq!(decode_pict(&pict), Ok(transparent(5, 3)));
    }

    #[test]
    fn empty_or_inverted_frames_have_bad_dimensions() {
        let pict = PictBuilder::new([0, 0, 0, 5]).end().build();
        assert_eq!(
            decode_pict(&pict),
            Err(GraphicsError::BadDimensions {
                width: 5,
                height: 0
            })
        );
        let pict = PictBuilder::new([0, 9, 4, 5]).end().build();
        assert_eq!(
            decode_pict(&pict),
            Err(GraphicsError::BadDimensions {
                width: -4,
                height: 4
            })
        );
    }

    #[test]
    fn only_version_2_is_supported() {
        let mut v1 = PictBuilder::new(FRAME).end().build();
        v1[10..12].copy_from_slice(&[0x11, 0x01]);
        assert_eq!(
            decode_pict(&v1),
            Err(GraphicsError::UnsupportedVersion { offset: 10 })
        );
        let mut garbage = PictBuilder::new(FRAME).end().build();
        garbage[12] = 0x01;
        assert_eq!(
            decode_pict(&garbage),
            Err(GraphicsError::UnsupportedVersion { offset: 10 })
        );
    }

    #[test]
    fn a_truncated_header_is_an_unexpected_end() {
        let pict = PictBuilder::new(FRAME).build();
        for len in 0..pict.len() {
            assert!(
                matches!(
                    decode_pict(&pict[..len]),
                    Err(GraphicsError::UnexpectedEnd { .. })
                ),
                "{len}"
            );
        }
        assert_eq!(
            decode_pict(&pict[..13]),
            Err(GraphicsError::UnexpectedEnd { offset: 12 })
        );
    }

    #[test]
    fn missing_end_pic_is_an_unexpected_end() {
        let pict = PictBuilder::new(FRAME).header_op().build();
        assert_eq!(
            decode_pict(&pict),
            Err(GraphicsError::UnexpectedEnd { offset: 0x28 })
        );
    }

    #[test]
    fn skippable_opcodes_are_skipped() {
        let pict = PictBuilder::new(FRAME)
            .header_op()
            .nop()
            .def_hilite()
            .clip_rect(FRAME)
            .short_comment(100)
            .long_comment(498, &[1, 2, 3])
            .nop()
            .long_comment(7, &[])
            .end()
            .build();
        assert_eq!(decode_pict(&pict), Ok(transparent(5, 3)));
    }

    #[test]
    fn opcodes_after_an_odd_length_comment_start_after_a_pad_byte() {
        let pict = PictBuilder::new(FRAME)
            .long_comment(1, &[0xAA])
            .end()
            .build();
        // The comment ends at 21; the pad byte is 21 and EndPic is at 22.
        assert_eq!(pict.len(), 24);
        assert_eq!(pict[21], 0);
        assert_eq!(decode_pict(&pict), Ok(transparent(5, 3)));
        // A missing pad byte is an unexpected end.
        assert_eq!(
            decode_pict(&pict[..21]),
            Err(GraphicsError::UnexpectedEnd { offset: 21 })
        );
    }

    #[test]
    fn comment_lengths_are_skipped_exactly() {
        // If the comment's data were not skipped, its bytes would be read
        // as opcodes: 0x8200 is unsupported.
        let pict = PictBuilder::new(FRAME)
            .long_comment(1, &[0x82, 0x00])
            .short_comment(0x8200)
            .end()
            .build();
        assert_eq!(decode_pict(&pict), Ok(transparent(5, 3)));
    }

    #[test]
    fn clip_regions_are_skipped_by_their_size() {
        let mut region = vec![0, 12];
        region.extend([0, 0, 0, 3, 0, 5, 0x82, 0x00, 0x82, 0x00]);
        let pict = PictBuilder::new(FRAME).raw(0x0001, &region).end().build();
        assert_eq!(decode_pict(&pict), Ok(transparent(5, 3)));
    }

    #[test]
    fn a_clip_region_smaller_than_a_rectangle_is_rejected() {
        let pict = PictBuilder::new(FRAME)
            .raw(0x0001, &[0, 9, 0, 0, 0, 0, 0, 0, 0])
            .end()
            .build();
        assert_eq!(
            decode_pict(&pict),
            Err(GraphicsError::BadRegion {
                size: 9,
                offset: 16
            })
        );
        let pict = PictBuilder::new(FRAME)
            .raw(0x0001, &[0, 10, 0, 0, 0, 0, 0, 0, 0, 0])
            .end()
            .build();
        assert!(decode_pict(&pict).is_ok());
    }

    #[test]
    fn bytes_after_end_pic_are_ignored() {
        let mut pict = PictBuilder::new(FRAME).end().build();
        pict.extend([0x82, 0x00, 0xFF]);
        assert_eq!(decode_pict(&pict), Ok(transparent(5, 3)));
    }

    #[test]
    fn an_unsupported_opcode_is_named_with_its_offset() {
        let pict = PictBuilder::new(FRAME)
            .header_op()
            .raw(0x8200, &[0; 16])
            .end()
            .build();
        let err = decode_pict(&pict).unwrap_err();
        assert_eq!(
            err,
            GraphicsError::UnsupportedOpcode {
                opcode: 0x8200,
                offset: 0x28
            }
        );
        assert_eq!(
            err.to_string(),
            "unsupported PICT opcode 0x8200 at byte 0x28"
        );
    }

    #[test]
    fn bitmap_and_region_pixel_opcodes_are_unsupported() {
        for opcode in [0x0090, 0x0091, 0x0099, 0x009B, 0x0011, 0x00FE] {
            let pict = PictBuilder::new(FRAME)
                .nop()
                .raw(opcode, &[0; 64])
                .end()
                .build();
            assert_eq!(
                decode_pict(&pict),
                Err(GraphicsError::UnsupportedOpcode { opcode, offset: 16 }),
                "{opcode:#06x}"
            );
        }
    }

    #[test]
    fn a_huge_frame_in_a_tiny_picture_is_too_large() {
        let pict = PictBuilder::new([0, 0, 32767, 32767]).end().build();
        assert_eq!(
            decode_pict(&pict),
            Err(GraphicsError::TooLarge {
                pixels: 32767 * 32767,
                input_len: 16
            })
        );
        let pict = PictBuilder::new([-32768, -32768, 32767, 32767])
            .end()
            .build();
        assert_eq!(
            decode_pict(&pict),
            Err(GraphicsError::TooLarge {
                pixels: 65535 * 65535,
                input_len: 16
            })
        );
    }

    const RED: u16 = 0x7C00;
    const GREEN: u16 = 0x03E0;
    const BLUE: u16 = 0x001F;
    const WHITE: u16 = 0x7FFF;
    const BLACK: u16 = 0x0000;

    /// The pixels of an image, as RGBA.
    fn rgba(image: &Image) -> Vec<[u8; 4]> {
        image
            .pixels()
            .chunks(4)
            .map(|p| p.try_into().unwrap())
            .collect()
    }

    fn opaque(pixels: &[u16]) -> Vec<[u8; 4]> {
        pixels.iter().map(|&p| rgb555(p)).collect()
    }

    /// A 4x2 16-bit picture with PackBits rows, written out by hand.
    #[test]
    fn decodes_a_hand_written_16_bit_picture() {
        let bytes = [
            &[0x00, 0x00][..],                                 // picSize
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x04], // picFrame 4x2
            &[0x00, 0x11, 0x02, 0xFF],                         // version 2
            &[0x00, 0x9A],                                     // DirectBitsRect
            &[0x00, 0x00, 0x00, 0xFF],                         // baseAddr
            &[0x80, 0x08],                                     // rowBytes 8
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x04], // bounds
            &[0x00, 0x00, 0x00, 0x03],                         // pmVersion, packType 3
            &[0x00, 0x00, 0x00, 0x00],                         // packSize
            &[0x00, 0x48, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00], // hRes, vRes
            &[0x00, 0x10, 0x00, 0x10],                         // pixelType, pixelSize 16
            &[0x00, 0x03, 0x00, 0x05],                         // cmpCount 3, cmpSize 5
            &[0x00; 12],                                       // planeBytes, pmTable, pmReserved
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x04], // srcRect
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x04], // dstRect
            &[0x00, 0x40],                                     // mode: ditherCopy
            // Row 0: red three times, then one literal blue.
            &[0x06, 0xFE, 0x7C, 0x00, 0x00, 0x00, 0x1F],
            // Row 1: literal green and white, then black twice.
            &[0x08, 0x01, 0x03, 0xE0, 0x7F, 0xFF, 0xFF, 0x00, 0x00],
            &[0x00, 0xFF], // EndPic
        ]
        .concat();
        let image = decode_pict(&bytes).unwrap();
        assert_eq!((image.width(), image.height()), (4, 2));
        let red = [255, 0, 0, 255];
        let green = [0, 255, 0, 255];
        let blue = [0, 0, 255, 255];
        let white = [255, 255, 255, 255];
        let black = [0, 0, 0, 255];
        assert_eq!(
            rgba(&image),
            [red, red, red, blue, green, white, black, black]
        );
    }

    /// Six distinct colours.
    const SIX: [u16; 6] = [RED, GREEN, BLUE, WHITE, BLACK, 0x1234];

    #[test]
    fn packed_16_bit_rows_decode_opaque() {
        let pict = PictBuilder::new([0, 0, 2, 3])
            .header_op()
            .def_hilite()
            .clip_rect([0, 0, 2, 3])
            .direct_bits(&DirectBits::rgb555([0, 0, 2, 3], &SIX))
            .end()
            .build();
        assert_eq!(rgba(&decode_pict(&pict).unwrap()), opaque(&SIX));
    }

    #[test]
    fn narrow_16_bit_rows_are_stored_unpacked() {
        let pixels = [RED, GREEN, BLUE, WHITE, BLACK, RED];
        let bits = DirectBits::rgb555([0, 0, 3, 2], &pixels);
        assert_eq!(bits.row_bytes, 4);
        let pict = PictBuilder::new([0, 0, 3, 2])
            .direct_bits(&bits)
            .end()
            .build();
        // Header, opcode, baseAddr, PixMap, rectangles, mode, then 3 raw rows.
        assert_eq!(pict.len(), 14 + 2 + 4 + 46 + 18 + 3 * 4 + 2);
        assert_eq!(rgba(&decode_pict(&pict).unwrap()), opaque(&pixels));
    }

    #[test]
    fn pack_type_0_is_the_default_word_packing() {
        let mut bits = DirectBits::rgb555([0, 0, 2, 3], &SIX);
        bits.pack_type = 0;
        let pict = PictBuilder::new([0, 0, 2, 3])
            .direct_bits(&bits)
            .end()
            .build();
        assert_eq!(rgba(&decode_pict(&pict).unwrap()), opaque(&SIX));
    }

    /// Where the first row starts when the picture is just the header and
    /// one `009A`.
    const FIRST_ROW: usize = 14 + 2 + 4 + 46 + 18;

    #[test]
    fn row_counts_are_bytes_up_to_250_row_bytes_and_words_above() {
        let narrow = DirectBits::rgb555([0, 0, 2, 125], &[GREEN; 250]);
        assert_eq!(narrow.row_bytes, 250);
        let pict = PictBuilder::new([0, 0, 2, 125])
            .direct_bits(&narrow)
            .end()
            .build();
        // A 1-byte count of 3: a repeat flag (257 - 125) and one word.
        assert_eq!(pict[FIRST_ROW..FIRST_ROW + 2], [3, 132]);
        assert_eq!(rgba(&decode_pict(&pict).unwrap()), opaque(&[GREEN; 250]));

        let wide = DirectBits::rgb555([0, 0, 2, 126], &[GREEN; 252]);
        assert_eq!(wide.row_bytes, 252);
        let pict = PictBuilder::new([0, 0, 2, 126])
            .direct_bits(&wide)
            .end()
            .build();
        // A 2-byte count, then the repeat flag 257 - 126.
        assert_eq!(pict[FIRST_ROW..FIRST_ROW + 3], [0, 3, 131]);
        assert_eq!(rgba(&decode_pict(&pict).unwrap()), opaque(&[GREEN; 252]));
    }

    #[test]
    fn padded_rows_are_cut_to_the_image_width() {
        let mut bits = DirectBits::rgb555([0, 0, 2, 3], &SIX);
        bits.row_bytes = 10;
        let pict = PictBuilder::new([0, 0, 2, 3])
            .direct_bits(&bits)
            .end()
            .build();
        assert_eq!(rgba(&decode_pict(&pict).unwrap()), opaque(&SIX));
    }

    #[test]
    fn copy_and_dither_modes_both_copy_pixels() {
        for mode in [0, 64] {
            let mut bits = DirectBits::rgb555([0, 0, 2, 3], &SIX);
            bits.mode = mode;
            let pict = PictBuilder::new([0, 0, 2, 3])
                .direct_bits(&bits)
                .end()
                .build();
            assert_eq!(rgba(&decode_pict(&pict).unwrap()), opaque(&SIX), "{mode}");
        }
    }

    #[test]
    fn other_transfer_modes_are_unsupported() {
        for mode in [1, 2, 8, 32, 36, 63, 65] {
            let mut bits = DirectBits::rgb555([0, 0, 2, 3], &SIX);
            bits.mode = mode;
            let pict = PictBuilder::new([0, 0, 2, 3])
                .direct_bits(&bits)
                .end()
                .build();
            assert_eq!(
                decode_pict(&pict),
                Err(GraphicsError::UnsupportedTransferMode { mode }),
                "{mode}"
            );
        }
    }

    #[test]
    fn the_source_rectangle_is_copied_to_the_destination_in_the_frame() {
        // A 4x3 pixmap at (100, 200); its middle 2x2 goes to (11, 22) in a
        // 6x4 frame at (10, 20).
        let pixels: Vec<u16> = (0..12).map(|i| 0x0400 * (i + 1)).collect();
        let mut bits = DirectBits::rgb555([100, 200, 103, 204], &pixels);
        bits.src = [101, 201, 103, 203];
        bits.dst = [11, 22, 13, 24];
        let pict = PictBuilder::new([10, 20, 14, 26])
            .direct_bits(&bits)
            .end()
            .build();
        let image = decode_pict(&pict).unwrap();
        let mut expected = vec![[0; 4]; 24];
        for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            expected[(1 + y) * 6 + 2 + x] = rgb555(pixels[(1 + y) * 4 + 1 + x]);
        }
        assert_eq!(rgba(&image), expected);
    }

    #[test]
    fn bands_compose_one_image_and_uncovered_pixels_stay_transparent() {
        let top = DirectBits::rgb555([0, 0, 1, 3], &[RED, GREEN, BLUE]);
        let bottom = DirectBits::rgb555([2, 0, 3, 3], &[WHITE, BLACK, RED]);
        let pict = PictBuilder::new([0, 0, 3, 3])
            .direct_bits(&top)
            .direct_bits(&bottom)
            .end()
            .build();
        let mut expected = opaque(&[RED, GREEN, BLUE]);
        expected.extend([[0; 4]; 3]);
        expected.extend(opaque(&[WHITE, BLACK, RED]));
        assert_eq!(rgba(&decode_pict(&pict).unwrap()), expected);
    }

    #[test]
    fn copies_must_stay_inside_bounds_and_frame_without_scaling() {
        let frame = [0, 0, 4, 4];
        let bounds = [0, 0, 4, 4];
        let src_offset = FIRST_ROW - 18;
        let cases: [([i16; 4], [i16; 4]); 10] = [
            // Source outside the bounds, on each side.
            ([-1, 0, 3, 4], [0, 0, 4, 4]),
            ([0, -1, 4, 3], [0, 0, 4, 4]),
            ([1, 0, 5, 4], [0, 0, 4, 4]),
            ([0, 1, 4, 5], [0, 0, 4, 4]),
            // Destination outside the frame, on each side.
            ([0, 0, 2, 2], [-1, 0, 1, 2]),
            ([0, 0, 2, 2], [0, -1, 2, 1]),
            ([0, 0, 2, 2], [3, 0, 5, 2]),
            ([0, 0, 2, 2], [0, 3, 2, 5]),
            // Scaled in either direction.
            ([0, 0, 2, 2], [0, 0, 3, 2]),
            ([0, 0, 2, 2], [0, 0, 2, 3]),
        ];
        for (src, dst) in cases {
            let mut bits = DirectBits::rgb555(bounds, &[RED; 16]);
            bits.src = src;
            bits.dst = dst;
            let pict = PictBuilder::new(frame).direct_bits(&bits).end().build();
            assert_eq!(
                decode_pict(&pict),
                Err(GraphicsError::BadCopyRect { offset: src_offset }),
                "{src:?} -> {dst:?}"
            );
        }
    }

    #[test]
    fn a_translated_copy_is_not_scaled() {
        let mut bits = DirectBits::rgb555([0, 0, 2, 2], &[RED, GREEN, BLUE, WHITE]);
        bits.dst = [2, 1, 4, 3];
        let pict = PictBuilder::new([0, 0, 4, 3])
            .direct_bits(&bits)
            .end()
            .build();
        let image = decode_pict(&pict).unwrap();
        assert_eq!(image.pixel(1, 2), Some(rgb555(RED)));
        assert_eq!(image.pixel(2, 3), Some(rgb555(WHITE)));
        assert_eq!(image.pixel(0, 2), Some([0; 4]));
    }

    #[test]
    fn rows_narrower_than_the_pixels_are_rejected() {
        let mut bits = DirectBits::rgb555([0, 0, 2, 5], &[RED; 10]);
        bits.row_bytes = 9;
        let pict = PictBuilder::new([0, 0, 2, 5])
            .direct_bits(&bits)
            .end()
            .build();
        assert_eq!(
            decode_pict(&pict),
            Err(GraphicsError::BadRowBytes {
                row_bytes: 9,
                needed: 10
            })
        );
    }

    #[test]
    fn an_empty_pixmap_has_bad_dimensions() {
        let mut bits = DirectBits::rgb555([0, 0, 2, 2], &[RED; 4]);
        bits.bounds = [0, 0, 0, 2];
        bits.src = [0, 0, 0, 2];
        bits.dst = [0, 0, 0, 2];
        let pict = PictBuilder::new([0, 0, 2, 2])
            .direct_bits(&bits)
            .end()
            .build();
        assert_eq!(
            decode_pict(&pict),
            Err(GraphicsError::BadDimensions {
                width: 2,
                height: 0
            })
        );
    }

    #[test]
    fn a_row_that_does_not_unpack_to_row_bytes_is_rejected() {
        let bits = DirectBits::rgb555(
            [0, 0, 2, 4],
            &[RED, RED, RED, BLUE, GREEN, WHITE, BLACK, BLACK],
        );
        let pict = PictBuilder::new([0, 0, 2, 4])
            .direct_bits(&bits)
            .end()
            .build();
        let first_count = usize::from(pict[FIRST_ROW]);
        // A count one byte too long swallows the next row's count.
        let mut long = pict.clone();
        long[FIRST_ROW] += 1;
        assert_eq!(
            decode_pict(&long),
            Err(GraphicsError::BadPackedRow {
                row: 0,
                offset: FIRST_ROW
            })
        );
        // Corrupt the second row's repeat flag so it unpacks too far.
        let mut bad = pict.clone();
        let second = FIRST_ROW + 1 + first_count;
        let flag = second + 1 + usize::from(pict[second]) - 3;
        assert_eq!(pict[flag], 0xFF);
        bad[flag] = 0xFE;
        assert_eq!(
            decode_pict(&bad),
            Err(GraphicsError::BadPackedRow {
                row: 1,
                offset: second
            })
        );
    }

    #[test]
    fn truncated_pixel_data_is_an_unexpected_end() {
        let pict = PictBuilder::new([0, 0, 2, 3])
            .direct_bits(&DirectBits::rgb555([0, 0, 2, 3], &SIX))
            .end()
            .build();
        for len in 16..pict.len() - 2 {
            assert!(
                matches!(
                    decode_pict(&pict[..len]),
                    Err(GraphicsError::UnexpectedEnd { .. })
                ),
                "{len}"
            );
        }
    }

    #[test]
    fn unsupported_direct_pixmaps_are_named() {
        let unsupported = |change: fn(&mut DirectBits)| {
            let mut bits = DirectBits::rgb555([0, 0, 2, 3], &SIX);
            change(&mut bits);
            let pict = PictBuilder::new([0, 0, 2, 3])
                .direct_bits(&bits)
                .end()
                .build();
            decode_pict(&pict)
        };
        let error = |pixel_size, pack_type, cmp_count| {
            Err(GraphicsError::UnsupportedPixMap {
                pixel_size,
                pack_type,
                cmp_count,
            })
        };
        assert_eq!(unsupported(|b| b.pack_type = 1), error(16, 1, 3));
        assert_eq!(unsupported(|b| b.pack_type = 2), error(16, 2, 3));
        assert_eq!(unsupported(|b| b.pack_type = 4), error(16, 4, 3));
        assert_eq!(unsupported(|b| b.pixel_size = 8), error(8, 3, 3));
        assert_eq!(unsupported(|b| b.pixel_size = 24), error(24, 3, 3));
    }

    #[test]
    fn a_direct_bitmap_is_not_a_pixmap() {
        let mut pict = PictBuilder::new([0, 0, 2, 3])
            .direct_bits(&DirectBits::rgb555([0, 0, 2, 3], &SIX))
            .end()
            .build();
        pict[20] &= 0x7F;
        assert_eq!(
            decode_pict(&pict),
            Err(GraphicsError::NotAPixMap { offset: 20 })
        );
    }

    fn opaque888(pixels: &[[u8; 3]]) -> Vec<[u8; 4]> {
        pixels.iter().map(|&[r, g, b]| [r, g, b, 255]).collect()
    }

    /// Six distinct 24-bit colours.
    const SIX_888: [[u8; 3]; 6] = [
        [1, 2, 3],
        [250, 0, 7],
        [9, 200, 11],
        [12, 13, 255],
        [0, 0, 0],
        [255, 255, 255],
    ];

    #[test]
    fn rows_of_32_bit_pixels_are_packed_as_red_green_and_blue_planes() {
        let bits = DirectBits::rgb888([0, 0, 1, 2], &[[1, 2, 3], [4, 5, 6]]);
        assert_eq!(bits.row_bytes, 8);
        let pict = PictBuilder::new([0, 0, 1, 2])
            .direct_bits(&bits)
            .end()
            .build();
        // One count byte, then a literal run of six bytes: R R G G B B.
        assert_eq!(pict[FIRST_ROW..FIRST_ROW + 8], [7, 5, 1, 4, 2, 5, 3, 6]);
        assert_eq!(
            rgba(&decode_pict(&pict).unwrap()),
            opaque888(&[[1, 2, 3], [4, 5, 6]])
        );
    }

    #[test]
    fn planar_32_bit_rows_decode_opaque() {
        let pict = PictBuilder::new([0, 0, 2, 3])
            .direct_bits(&DirectBits::rgb888([0, 0, 2, 3], &SIX_888))
            .end()
            .build();
        assert_eq!(rgba(&decode_pict(&pict).unwrap()), opaque888(&SIX_888));
    }

    #[test]
    fn pack_type_0_is_treated_as_component_packing() {
        let mut bits = DirectBits::rgb888([0, 0, 2, 3], &SIX_888);
        bits.pack_type = 0;
        let pict = PictBuilder::new([0, 0, 2, 3])
            .direct_bits(&bits)
            .end()
            .build();
        assert_eq!(rgba(&decode_pict(&pict).unwrap()), opaque888(&SIX_888));
    }

    #[test]
    fn narrow_32_bit_rows_are_stored_unpacked_as_xrgb() {
        let pixels = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];
        let bits = DirectBits::rgb888([0, 0, 3, 1], &pixels);
        assert_eq!(bits.row_bytes, 4);
        let pict = PictBuilder::new([0, 0, 3, 1])
            .direct_bits(&bits)
            .end()
            .build();
        assert_eq!(pict[FIRST_ROW..FIRST_ROW + 4], [0, 1, 2, 3]);
        assert_eq!(rgba(&decode_pict(&pict).unwrap()), opaque888(&pixels));
    }

    #[test]
    fn row_counts_switch_to_words_above_250_row_bytes_for_32_bit_rows() {
        let pixels: Vec<[u8; 3]> = (0..124).map(|i| [i as u8, 0, 0]).collect();
        for (row_bytes, count) in [(250, &[65][..]), (251, &[0, 65][..])] {
            let mut bits = DirectBits::rgb888([0, 0, 2, 62], &pixels);
            bits.row_bytes = row_bytes;
            let pict = PictBuilder::new([0, 0, 2, 62])
                .direct_bits(&bits)
                .end()
                .build();
            // A literal run of 62 reds (63 bytes) and one repeat run of 124
            // zero greens and blues (2 bytes).
            assert_eq!(
                &pict[FIRST_ROW..FIRST_ROW + count.len()],
                count,
                "{row_bytes}"
            );
            assert_eq!(
                rgba(&decode_pict(&pict).unwrap()),
                opaque888(&pixels),
                "{row_bytes}"
            );
        }
    }

    #[test]
    fn rows_narrower_than_32_bit_pixels_are_rejected() {
        let mut bits = DirectBits::rgb888([0, 0, 2, 3], &SIX_888);
        bits.row_bytes = 11;
        let pict = PictBuilder::new([0, 0, 2, 3])
            .direct_bits(&bits)
            .end()
            .build();
        assert_eq!(
            decode_pict(&pict),
            Err(GraphicsError::BadRowBytes {
                row_bytes: 11,
                needed: 12
            })
        );
    }

    #[test]
    fn unsupported_32_bit_pixmaps_are_named() {
        let unsupported = |change: fn(&mut DirectBits)| {
            let mut bits = DirectBits::rgb888([0, 0, 2, 3], &SIX_888);
            change(&mut bits);
            let pict = PictBuilder::new([0, 0, 2, 3])
                .direct_bits(&bits)
                .end()
                .build();
            decode_pict(&pict)
        };
        let error = |pixel_size, pack_type, cmp_count| {
            Err(GraphicsError::UnsupportedPixMap {
                pixel_size,
                pack_type,
                cmp_count,
            })
        };
        assert_eq!(unsupported(|b| b.cmp_count = 4), error(32, 4, 4));
        assert_eq!(unsupported(|b| b.cmp_count = 1), error(32, 4, 1));
        assert_eq!(unsupported(|b| b.pack_type = 1), error(32, 1, 3));
        assert_eq!(unsupported(|b| b.pack_type = 2), error(32, 2, 3));
        assert_eq!(unsupported(|b| b.pack_type = 3), error(32, 3, 3));
    }

    /// A sparse table: values 0, 1, 3 and 255 (not 2) map to colours whose
    /// components are distinct.
    fn sparse_ctab() -> Ctab {
        Ctab {
            device: false,
            entries: vec![
                (255, [0x1100, 0x2200, 0x3300]),
                (3, [0x4400, 0x5500, 0x6600]),
                (0, [0x7700, 0x8800, 0x9900]),
                (1, [0xAA00, 0xBB00, 0xCC00]),
            ],
        }
    }

    fn colour(index: u8) -> [u8; 4] {
        match index {
            255 => [0x11, 0x22, 0x33, 255],
            3 => [0x44, 0x55, 0x66, 255],
            0 => [0x77, 0x88, 0x99, 255],
            1 => [0xAA, 0xBB, 0xCC, 255],
            _ => unreachable!(),
        }
    }

    fn indexed_pict(bits: &IndexedBits) -> Vec<u8> {
        let [top, left, bottom, right] = bits.bounds;
        PictBuilder::new([top, left, bottom, right])
            .header_op()
            .packbits_rect(bits)
            .end()
            .build()
    }

    #[test]
    fn indexed_pixels_are_looked_up_by_value_at_every_depth() {
        let cases: [(u16, &[u8]); 4] = [
            (1, &[1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 0, 0]),
            (2, &[3, 0, 1, 3, 3, 1, 0, 0, 1, 3, 1, 0]),
            (4, &[3, 0, 1, 3, 3, 1, 0, 0, 1, 3, 1, 0]),
            (8, &[255, 0, 1, 3, 3, 1, 255, 0, 1, 3, 1, 0]),
        ];
        for (depth, indices) in cases {
            // 3 pixels wide: never a whole number of bytes below 8 bits.
            for bounds in [[0, 0, 4, 3], [0, 0, 2, 6]] {
                let bits = IndexedBits::new(bounds, depth, &sparse_ctab(), indices);
                let image = decode_pict(&indexed_pict(&bits)).unwrap();
                let expected: Vec<[u8; 4]> = indices.iter().map(|&i| colour(i)).collect();
                assert_eq!(rgba(&image), expected, "{depth}-bit {bounds:?}");
            }
        }
    }

    #[test]
    fn wide_indexed_rows_are_packed() {
        let indices: Vec<u8> = (0..40)
            .map(|i| [0, 1, 3, 255][i % 4 / 2 * 2 + i / 20])
            .collect();
        let bits = IndexedBits::new([0, 0, 2, 20], 8, &sparse_ctab(), &indices);
        assert_eq!(bits.row_bytes, 20);
        let image = decode_pict(&indexed_pict(&bits)).unwrap();
        let expected: Vec<[u8; 4]> = indices.iter().map(|&i| colour(i)).collect();
        assert_eq!(rgba(&image), expected);
    }

    /// A 2x2 4-bit picture, written out by hand.
    #[test]
    fn decodes_a_hand_written_indexed_picture() {
        let bytes = [
            &[0x00, 0x00][..],                                 // picSize
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02], // picFrame 2x2
            &[0x00, 0x11, 0x02, 0xFF],                         // version 2
            &[0x00, 0x98],                                     // PackBitsRect
            &[0x80, 0x01],                                     // rowBytes 1
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02], // bounds
            &[0x00; 4],                                        // pmVersion, packType
            &[0x00; 12],                                       // packSize, hRes, vRes
            &[0x00, 0x00, 0x00, 0x04, 0x00, 0x01, 0x00, 0x04], // pixelType, size 4
            &[0x00; 12],                                       // planeBytes, pmTable, reserved
            &[0x00; 6],                                        // ctSeed, ctFlags
            &[0x00, 0x01],                                     // ctSize: two entries
            &[0x00, 0x0F, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00], // 15: red
            &[0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF], // 2: blue
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02], // srcRect
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02], // dstRect
            &[0x00, 0x00],                                     // mode: srcCopy
            &[0xF2, 0x2F],                                     // rows, unpacked
            &[0x00, 0xFF],                                     // EndPic
        ]
        .concat();
        let (red, blue) = ([255, 0, 0, 255], [0, 0, 255, 255]);
        assert_eq!(rgba(&decode_pict(&bytes).unwrap()), [red, blue, blue, red]);
    }

    #[test]
    fn a_pixel_missing_from_the_table_is_named() {
        let bits = IndexedBits::new([0, 0, 1, 3], 4, &sparse_ctab(), &[0, 2, 1]);
        assert_eq!(
            decode_pict(&indexed_pict(&bits)),
            Err(GraphicsError::MissingColour { index: 2 })
        );
    }

    #[test]
    fn an_indexed_bitmap_is_not_a_pixmap() {
        let bits = IndexedBits::new([0, 0, 1, 3], 4, &sparse_ctab(), &[0, 3, 1]);
        let mut pict = PictBuilder::new([0, 0, 1, 3])
            .packbits_rect(&bits)
            .end()
            .build();
        pict[16] &= 0x7F;
        assert_eq!(
            decode_pict(&pict),
            Err(GraphicsError::NotAPixMap { offset: 16 })
        );
    }

    #[test]
    fn indexed_depths_other_than_1_2_4_and_8_are_unsupported() {
        for depth in [0, 3, 16, 32] {
            let mut bits = IndexedBits::new([0, 0, 1, 3], 8, &sparse_ctab(), &[0, 3, 1]);
            bits.bits = depth;
            assert_eq!(
                decode_pict(&indexed_pict(&bits)),
                Err(GraphicsError::UnsupportedPixMap {
                    pixel_size: depth,
                    pack_type: 0,
                    cmp_count: 1
                }),
                "{depth}"
            );
        }
    }

    #[test]
    fn indexed_rows_must_hold_every_pixel() {
        let mut bits = IndexedBits::new([0, 0, 1, 17], 4, &sparse_ctab(), &[0; 17]);
        assert_eq!(bits.row_bytes, 9);
        bits.row_bytes = 8;
        assert_eq!(
            decode_pict(&indexed_pict(&bits)),
            Err(GraphicsError::BadRowBytes {
                row_bytes: 8,
                needed: 9
            })
        );
    }

    #[test]
    fn indexed_copies_follow_the_copy_rules() {
        let mut bits = IndexedBits::new([0, 0, 2, 2], 8, &sparse_ctab(), &[0, 1, 3, 255]);
        bits.mode = 1;
        assert_eq!(
            decode_pict(&indexed_pict(&bits)),
            Err(GraphicsError::UnsupportedTransferMode { mode: 1 })
        );
        bits.mode = 0;
        bits.src = [0, 0, 1, 1];
        bits.dst = [1, 1, 2, 2];
        let image = decode_pict(&indexed_pict(&bits)).unwrap();
        assert_eq!(rgba(&image), [[0; 4], [0; 4], [0; 4], colour(0)]);
    }

    #[test]
    fn corrupt_pictures_never_panic() {
        let pict = PictBuilder::new([0, 0, 3, 3])
            .header_op()
            .def_hilite()
            .clip_rect([0, 0, 3, 3])
            .long_comment(498, &[1, 2, 3])
            .short_comment(7)
            .direct_bits(&DirectBits::rgb555([0, 0, 1, 3], &[RED, GREEN, BLUE]))
            .direct_bits(&DirectBits::rgb888([1, 0, 2, 3], &SIX_888[..3]))
            .packbits_rect(&IndexedBits::new(
                [2, 0, 3, 3],
                2,
                &sparse_ctab(),
                &[0, 1, 3],
            ))
            .end()
            .build();
        assert!(pict.len() < 1024);
        crate::sweep::assert_never_panics(&pict, decode_pict);
    }
}
