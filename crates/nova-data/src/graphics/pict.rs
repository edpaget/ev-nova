//! `PICT`: QuickDraw version 2 pictures.
//!
//! Source: *Inside Macintosh: Imaging With QuickDraw* (1994) appendix A,
//! "Picture Opcodes": the version 2 header, the opcode table and data
//! lengths, and word alignment of opcodes.

use super::GraphicsError;
use super::budget::check_budget;
use super::image::Image;
use super::pixmap::QdRect;
use super::reader::Reader;

/// Offset of the version opcode, after `picSize` and `picFrame`.
const VERSION_OFFSET: usize = 10;
/// The version 2 opcode (`0x0011`) and its version word (`0x02FF`).
const VERSION_2: [u16; 2] = [0x0011, 0x02FF];

/// Decodes a version 2 `PICT` to an image the size of its `picFrame`.
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
    let canvas = Image::transparent(width, height);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::fixture::PictBuilder;

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
}
