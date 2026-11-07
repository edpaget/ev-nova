//! A sprite sheet's run-length tokens as the bytes of memory, the form two
//! sprites' pixels touch by (`_SWRLECollision`; see
//! `nova_sim_base::sprite_mask`), which walks them by byte.
//!
//! - **An `rlëD`** ([`rled_words`]). `_SWCreateSpriteFromRLEResource`
//!   (@0xa9990-0xa9be9) keeps the resource, `_BlitPixieRLESwap` swaps it
//!   in place, and `_SWLockSprite` (@0xaa9b6-0xaaa83) walks it from byte
//!   16 to each frame's first token: a frame's tokens run to a 0 word,
//!   and every other word must be a line start (or the game quits), its
//!   count + 4 bytes skipped (`rled_wide`). So the collision reads the
//!   very tokens the blitter draws, as the swap left them in memory, at
//!   every depth: each token and run word read little-endian is the
//!   file's read big-endian, as far as the swap kept in step with the
//!   tokens, and byte-reversed after a token that stopped it. At 16 bits
//!   the swap reverses each pixel's u16 and at 8 leaves the pixel bytes
//!   as they are; at every depth but 8 and 16 it reads the pixel data as
//!   tokens (`rled_wide`), and the sheet must decode as drawn.
//! - **A cut `PICT` sheet** ([`sheet_rle_words`]). `_SWCreateRLEData`
//!   (@0xaec89-0xaefa1) has `_BlitPixieRLEParsePixels`
//!   (@0xc2875-0xc2bef) emit each frame's tokens from its pixels and its
//!   mask: per row a line start with the line's byte count, then skips
//!   (op 3) and pixels (op 2, their data padded to a word), at depth / 8
//!   bytes a pixel; a trailing skip is dropped (@0xc2b14-0xc2b37), and a
//!   0 word ends the frame. It never emits a run (op 4): its state is only
//!   ever 0, 1 or 2 (@0xc2947, @0xc29fc, @0xc2aa1). The pixels it keeps
//!   are those [`cut_pict_sheet`](super::cut_pict_sheet) leaves opaque.

use super::GraphicsError;
use super::reader::Reader;
use super::rled::read_header;
use super::rled_wide::{TOKENS, decode_wide, locked};
use super::rled32::ThirtyTwoBitReading;
use super::sheet::{SheetLayout, SpriteSheet};

/// The op of a line start.
const LINE_START: u32 = 1;
/// The op of drawn pixels.
const PIXELS: u32 = 2;
/// The op of clear pixels.
const SKIP: u32 = 3;

/// A sheet's run-length tokens: its frame size, its depth, its token
/// memory and where each frame starts in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RleWords {
    /// Each frame's width in pixels.
    pub frame_width: u32,
    /// Each frame's height in pixels.
    pub frame_height: u32,
    /// Its depth: any depth [`decode_rled`](super::decode_rled) reads,
    /// 8 to 0x7FFF.
    pub depth: u16,
    /// The tokens after the 16-byte header as bytes, as
    /// `_BlitPixieRLESwap` left them in memory: each word read
    /// little-endian; a part word at the end is kept.
    pub bytes: Vec<u8>,
    /// Each frame's first byte past the header, by `_SWLockSprite`'s walk.
    pub starts: Vec<usize>,
}

/// An `rlëD`'s tokens (see the module docs): its header checked as
/// [`decode_rled`](super::decode_rled) checks it, its swapped memory, and
/// each frame's start found by `_SWLockSprite`'s walk.
///
/// A header of no size is [`GraphicsError::BadDimensions`], of no frames
/// [`GraphicsError::NoFrames`], and of a depth of 7 or less or 0x8000 and
/// up [`GraphicsError::UnsupportedDepth`]. A sheet the swap or the lock
/// walk fails ([`GraphicsError::RunawaySwap`],
/// [`GraphicsError::UnexpectedEnd`], [`GraphicsError::NotALineStart`],
/// [`GraphicsError::LinePastEnd`]), which the original crashes or quits
/// on, fails so. At 16 and 8 bits the tokens are not checked beyond
/// that: the collision reads them as they are. At any other depth a
/// sheet that does not decode ([`decode_rled`](super::decode_rled))
/// fails as that does.
pub fn rled_words(data: &[u8]) -> Result<RleWords, GraphicsError> {
    let mut r = Reader::new(data);
    let header = read_header(&mut r, data.len())?;
    if header.depth != 16 && header.depth != 8 {
        decode_wide(
            data,
            &header,
            SheetLayout::DEFAULT,
            ThirtyTwoBitReading::AsDrawn,
        )?;
    }
    let (mem, starts) = locked(data, &header)?;
    Ok(RleWords {
        frame_width: header.width,
        frame_height: header.height,
        depth: header.depth,
        bytes: mem[TOKENS..].to_vec(),
        starts: starts.iter().map(|start| start - TOKENS).collect(),
    })
}

/// The tokens `_BlitPixieRLEParsePixels` makes of `sheet`'s frames at
/// `depth` bits a pixel (see the module docs): each opaque pixel drawn,
/// each other clear. The pixels' data words are written 0, as the
/// collision skips them.
#[must_use]
pub fn sheet_rle_words(sheet: &SpriteSheet, depth: u16) -> RleWords {
    let bytes = u32::from(depth / 8);
    let mut words = Vec::new();
    let mut starts = Vec::with_capacity(sheet.frames().len());
    for frame in sheet.frames() {
        starts.push(4 * words.len());
        for y in 0..frame.height() {
            let opaque: Vec<bool> = (0..frame.width())
                .map(|x| frame.pixel(x, y).is_some_and(|[.., alpha]| alpha != 0))
                .collect();
            let line = line_tokens(&opaque, bytes);
            words.push(token(LINE_START, 4 * line.len() as u32));
            words.extend(line);
        }
        words.push(0);
    }
    RleWords {
        frame_width: sheet.frame_width(),
        frame_height: sheet.frame_height(),
        depth,
        bytes: words.iter().flat_map(|word| word.to_le_bytes()).collect(),
        starts,
    }
}

/// A token of op `op` and byte count `count`.
fn token(op: u32, count: u32) -> u32 {
    (op << 24) | (count & 0x00FF_FFFF)
}

/// One row's tokens after its line start, `bytes` bytes a pixel: a token
/// for each run of opaque or clear pixels, no trailing skip.
fn line_tokens(opaque: &[bool], bytes: u32) -> Vec<u32> {
    let shown = opaque.iter().rposition(|&o| o).map_or(0, |last| last + 1);
    let mut tokens = Vec::new();
    let mut x = 0;
    while x < shown {
        let drawn = opaque[x];
        let n = opaque[x..shown].iter().take_while(|&&o| o == drawn).count();
        // A row is far shorter than a token's 24-bit count.
        #[allow(clippy::cast_possible_truncation)]
        let count = n as u32 * bytes;
        if drawn {
            tokens.push(token(PIXELS, count));
            tokens.extend(std::iter::repeat_n(0, count.div_ceil(4) as usize));
        } else {
            tokens.push(token(SKIP, count));
        }
        x += n;
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::decode_rled;
    use crate::graphics::fixture::RledBuilder;
    use crate::graphics::image::Image;
    use crate::graphics::sheet::SheetLayout;

    const RED: u16 = 0x7C00;
    const GREEN: u16 = 0x03E0;

    /// The word at byte `at` of `words`' memory, read little-endian as
    /// the collision reads it.
    fn memory_word(words: &RleWords, at: usize) -> u32 {
        u32::from_le_bytes(words.bytes[at..at + 4].try_into().expect("a word"))
    }

    /// The word at byte `at` past the header of the file `data`, read
    /// big-endian.
    fn file_word(data: &[u8], at: usize) -> u32 {
        u32::from_be_bytes(data[16 + at..20 + at].try_into().expect("a word"))
    }

    /// Every word of `words`' memory, read little-endian.
    fn memory_words(words: &RleWords) -> Vec<u32> {
        (0..words.bytes.len() / 4)
            .map(|word| memory_word(words, 4 * word))
            .collect()
    }

    /// A sheet of `width` x `height` frames at `depth`, its lines counted.
    fn sheet(width: u16, height: u16, depth: u16) -> RledBuilder {
        RledBuilder::new(width, height).depth(depth).count_lines()
    }

    #[test]
    fn a_16_bit_sheets_tokens_and_runs_read_from_memory_as_the_file_reads_them() {
        let data = RledBuilder::new(3, 1)
            .frame(|f| f.token(1, 12).pixels(&[RED, GREEN]).skip(1))
            .frame(|f| f.token(1, 8).run(2, 0x7FFF, 0x0001))
            .build();
        let words = rled_words(&data).expect("reads");
        assert_eq!(
            (words.frame_width, words.frame_height, words.depth),
            (3, 1, 16)
        );
        assert_eq!(words.bytes.len(), data.len() - 16, "the whole memory");
        assert_eq!(words.starts, [0, 20], "byte offsets");
        // Every token and run word reads little-endian from the swapped
        // memory as big-endian from the file; the pixels' data, whose
        // u16s the swap reverses, need not.
        for at in [0, 4, 12, 16, 20, 24, 28, 32] {
            assert_eq!(memory_word(&words, at), file_word(&data, at), "{at}");
        }
        assert_eq!(memory_word(&words, 8), 0x03E0_7C00, "pixels swapped");
    }

    #[test]
    fn a_sheet_the_lock_walk_runs_out_on_fails_as_its_decode_does() {
        let mut data = RledBuilder::new(2, 1)
            .frame_count(3)
            .frame(|f| f.token(1, 8).pixels(&[RED, RED]))
            .build();
        data.extend([1, 0]);
        assert!(matches!(
            rled_words(&data),
            Err(GraphicsError::UnexpectedEnd { .. })
        ));
        let open = RledBuilder::new(2, 1)
            .frame_count(2)
            .token(1, 8)
            .token(2, 4)
            .build();
        assert!(matches!(
            rled_words(&open),
            Err(GraphicsError::UnexpectedEnd { .. })
        ));
    }

    #[test]
    fn a_16_bit_sheet_whose_line_chain_lands_off_a_line_start_has_no_words() {
        // Line 0 counts 8 bytes; the word after them is a skip.
        let data = RledBuilder::new(2, 2)
            .frame(|f| f.token(1, 8).pixels(&[RED, RED]).skip(1))
            .build();
        assert_eq!(
            rled_words(&data),
            Err(GraphicsError::NotALineStart {
                frame: 0,
                offset: 28
            })
        );
    }

    #[test]
    fn an_8_bit_sheet_keeps_its_depth_and_a_bad_header_fails() {
        let eight = RledBuilder::new(2, 1)
            .depth(8)
            .frame(|f| f.token(1, 8).pixels8(&[1, 2]))
            .build();
        let words = rled_words(&eight).expect("reads");
        assert_eq!(words.depth, 8);
        // The swap steps over 8-bit pixel data unswapped.
        assert_eq!(
            memory_words(&words),
            [0x0100_0008, 0x0200_0002, 0x0000_0201, 0]
        );
        for depth in [1, 7, 0x8000] {
            let bad = RledBuilder::new(2, 1).depth(depth).frame(|f| f).build();
            assert_eq!(
                rled_words(&bad),
                Err(GraphicsError::UnsupportedDepth { depth })
            );
        }
        let none = RledBuilder::new(2, 1).build();
        assert_eq!(rled_words(&none), Err(GraphicsError::NoFrames));
    }

    #[test]
    fn a_32_bit_sheets_bytes_are_the_swapped_memory() {
        let data = sheet(2, 1, 32)
            .frame(|f| f.line().pixels32(&[0x0012_3456, 0x80AB_CDEF]))
            .frame(|f| f.line().skip32(1).run32(1, 0x7FFF_7FFF))
            .build();
        let words = rled_words(&data).expect("reads");
        assert_eq!(
            (words.frame_width, words.frame_height, words.depth),
            (2, 1, 32)
        );
        // In sync, the swapped memory read natively is the file read
        // big-endian, its 4-byte pixels too.
        let file: Vec<u32> = (0..(data.len() - 16) / 4)
            .map(|word| file_word(&data, 4 * word))
            .collect();
        assert_eq!(memory_words(&words), file);
        assert_eq!(words.starts, [0, 20]);
    }

    #[test]
    fn a_32_bit_sheet_whose_swap_stops_gives_the_bytes_as_the_swap_left_them() {
        // The tail word 07 44 00 00 swaps to op 7, which stops the swap;
        // the word after it, inside the line's count, stays big-endian.
        let data = RledBuilder::new(2, 1)
            .depth(32)
            .frame(|f| {
                f.token(1, 16)
                    .token(2, 6)
                    .raw(&[0x00, 0x11, 0x22, 0x33, 0x07, 0x44, 0x00, 0x00])
                    .raw(&[0x02, 0x00, 0x00, 0x04])
            })
            .token(0, 0)
            .build();
        let words = rled_words(&data).expect("reads");
        assert_eq!(
            memory_words(&words),
            [
                0x0100_0010,
                0x0200_0006,
                0x0011_2233,
                0x0744_0000,
                0x0400_0002,
                0,
                0
            ]
        );
        assert_eq!(words.starts, [0]);
    }

    #[test]
    fn a_32_bit_sheet_that_does_not_decode_has_no_words() {
        let data = sheet(2, 1, 32)
            .frame(|f| f.line().token(2, 2).raw(&[0; 4]))
            .build();
        assert_eq!(
            rled_words(&data),
            Err(GraphicsError::RunawaySwap { offset: 20 })
        );
        let data = RledBuilder::new(2, 1)
            .depth(32)
            .frame(|f| f.line().pixels32(&[1]))
            .build();
        assert!(matches!(
            rled_words(&data),
            Err(GraphicsError::NotALineStart { .. })
        ));
    }

    #[test]
    fn a_24_bit_sheet_reads_the_swapped_memory_and_the_lock_walks_starts() {
        let data = sheet(5, 1, 24)
            .frame(|f| f.line().skip8(3).run8(6, [0xA1, 0xB2, 0xC3, 0xD4]))
            .frame(|f| f.line().skip8(2))
            .build();
        let words = rled_words(&data).expect("reads");
        assert_eq!(
            (words.frame_width, words.frame_height, words.depth),
            (5, 1, 24)
        );
        assert_eq!(words.starts, [0, 20]);
        assert_eq!(
            memory_words(&words),
            [
                0x0100_000C,
                0x0300_0003,
                0x0400_0006,
                0xA1B2_C3D4,
                0,
                0x0100_0004,
                0x0300_0002,
                0
            ]
        );
        // Its pixel data is read, and swapped, as the token it reads as.
        let data = sheet(2, 1, 24)
            .frame(|f| f.line().pixels8(&[0x03, 0x00, 0x00, 0x00]))
            .build();
        let words = rled_words(&data).expect("reads");
        assert_eq!(words.bytes[8..12], [0, 0, 0, 3]);
    }

    #[test]
    fn a_depth_whose_shift_wraps_reads_and_keeps_its_depth() {
        for depth in [0x7FFF, 512, 528] {
            let data = sheet(2, 1, depth).frame(|f| f.line().skip8(2)).build();
            let words = rled_words(&data).expect("reads");
            assert_eq!(words.depth, depth);
            assert_eq!(memory_words(&words), [0x0100_0004, 0x0300_0002, 0]);
            assert_eq!(words.starts, [0]);
        }
    }

    #[test]
    fn another_depth_fails_as_its_decode_does() {
        // The data word 7F 00 00 00 swaps to op 0x7F, which stops the
        // swap; the second line start stays big-endian.
        let data = sheet(2, 2, 24)
            .frame(|f| f.line().pixels8(&[0x7F, 0, 0, 0]).line().skip8(4))
            .build();
        let failed = Err(GraphicsError::NotALineStart {
            frame: 0,
            offset: 28,
        });
        assert_eq!(rled_words(&data), failed);
        assert_eq!(decode_rled(&data, None).err(), failed.err());
    }

    #[test]
    fn a_line_count_short_of_a_word_starts_the_next_frame_at_its_byte() {
        // Line 0 counts 6 bytes, a skip and 2 more: frame 0 ends at byte
        // 10 and frame 1 starts at byte 14.
        for depth in [32, 16] {
            let data = RledBuilder::new(1, 1)
                .depth(depth)
                .frame(|f| f.token(1, 6).token(3, 2).raw(&[0, 0]))
                .frame(|f| f)
                .build();
            let words = rled_words(&data).expect("reads");
            assert_eq!(words.starts, [0, 14], "{depth}");
        }
    }

    /// A frame `width` wide of `rows`, `#` opaque and `.` clear.
    fn image(width: u32, rows: &[&str]) -> Image {
        let pixels = rows
            .iter()
            .flat_map(|row| row.bytes())
            .flat_map(|c| if c == b'#' { [9, 9, 9, 255] } else { [0; 4] })
            .collect();
        Image::from_rgba(width, rows.len() as u32, pixels).expect("whole")
    }

    #[test]
    fn a_cut_sheets_rows_are_skips_and_padded_pixels_with_no_trailing_skip() {
        let frames = vec![image(4, &[".#..", "###."]), image(4, &["....", "####"])];
        let sheet = SpriteSheet::new(4, 2, frames, SheetLayout::DEFAULT);
        let words = sheet_rle_words(&sheet, 16);
        assert_eq!(
            (words.frame_width, words.frame_height, words.depth),
            (4, 2, 16)
        );
        assert_eq!(
            memory_words(&words),
            [
                0x0100_000C,
                0x0300_0002,
                0x0200_0002,
                0,
                0x0100_000C,
                0x0200_0006,
                0,
                0,
                0,
                0x0100_0000,
                0x0100_000C,
                0x0200_0008,
                0,
                0,
                0,
            ]
        );
        assert_eq!(words.starts, [0, 36], "byte offsets");
        // At 8 bits, a byte a pixel.
        let one = SpriteSheet::new(4, 1, vec![image(4, &["##.#"])], SheetLayout::DEFAULT);
        assert_eq!(
            memory_words(&sheet_rle_words(&one, 8)),
            [0x0100_0014, 0x0200_0002, 0, 0x0300_0001, 0x0200_0001, 0, 0]
        );
    }
}
