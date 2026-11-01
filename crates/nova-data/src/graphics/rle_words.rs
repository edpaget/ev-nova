//! A sprite sheet's run-length tokens as words, the form two sprites'
//! pixels touch by (`_SWRLECollision`; see `nova_sim_base::sprite_mask`).
//!
//! - **An `rlëD`** ([`rled_words`]). `_SWCreateSpriteFromRLEResource`
//!   (@0xa9990-0xa9be9) keeps the resource, and `_SWLockSprite`
//!   (@0xaa9b6-0xaaa83) walks it from byte 16 to each frame's first
//!   token: a frame's tokens run to a 0 word, and every other word is
//!   taken for a line start (asserted only), its count + 4 bytes skipped.
//!   So the collision reads the very tokens the blitter draws.
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
use super::sheet::SpriteSheet;

/// The op of a line start.
const LINE_START: u32 = 1;
/// The op of drawn pixels.
const PIXELS: u32 = 2;
/// The op of clear pixels.
const SKIP: u32 = 3;

/// A sheet's run-length tokens: its frame size, its depth, its token
/// words and where each frame's start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RleWords {
    /// Each frame's width in pixels.
    pub frame_width: u32,
    /// Each frame's height in pixels.
    pub frame_height: u32,
    /// Its depth: 16 or 8.
    pub depth: u16,
    /// The tokens after the 16-byte header, each read as a big-endian
    /// word; a part word at the end is dropped.
    pub words: Vec<u32>,
    /// Each frame's first word, by `_SWLockSprite`'s walk; `None` for each
    /// frame the walk ran out before.
    pub starts: Vec<Option<usize>>,
}

/// An `rlëD`'s tokens (see the module docs): its header checked as
/// [`decode_rled`](super::decode_rled) checks it, its words, and each
/// frame's start found by `_SWLockSprite`'s walk.
///
/// A header of another depth is [`GraphicsError::UnsupportedDepth`], of
/// no size [`GraphicsError::BadDimensions`] and of no frames
/// [`GraphicsError::NoFrames`]. The tokens themselves are not checked:
/// the collision reads them as they are.
pub fn rled_words(data: &[u8]) -> Result<RleWords, GraphicsError> {
    let mut r = Reader::new(data);
    let header = read_header(&mut r, data.len())?;
    let words: Vec<u32> = data[r.pos()..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| u32::from_be_bytes(*word))
        .collect();
    let mut starts = Vec::with_capacity(usize::from(header.count));
    let mut at = 0;
    for _ in 0..header.count {
        if at >= words.len() {
            starts.push(None);
            continue;
        }
        starts.push(Some(at));
        while let Some(&word) = words.get(at) {
            at += 1;
            if word == 0 {
                break;
            }
            at += ((word & 0x00FF_FFFF) / 4) as usize;
        }
    }
    Ok(RleWords {
        frame_width: header.width,
        frame_height: header.height,
        depth: header.depth,
        words,
        starts,
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
        starts.push(Some(words.len()));
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
        words,
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
    use crate::graphics::fixture::RledBuilder;
    use crate::graphics::image::Image;
    use crate::graphics::sheet::SheetLayout;

    const RED: u16 = 0x7C00;
    const GREEN: u16 = 0x03E0;

    #[test]
    fn an_rleds_words_follow_its_header_and_each_frame_starts_after_the_last_ends() {
        let data = RledBuilder::new(3, 1)
            .frame(|f| f.token(1, 12).pixels(&[RED, GREEN]).skip(1))
            .frame(|f| f.token(1, 8).run(2, 0x7FFF, 0x7FFF))
            .build();
        let words = rled_words(&data).expect("reads");
        assert_eq!(
            (words.frame_width, words.frame_height, words.depth),
            (3, 1, 16)
        );
        assert_eq!(
            words.words,
            [
                0x0100_000C,
                0x0200_0004,
                0x7C00_03E0,
                0x0300_0002,
                0,
                0x0100_0008,
                0x0400_0004,
                0x7FFF_7FFF,
                0,
            ]
        );
        assert_eq!(words.starts, [Some(0), Some(5)]);
    }

    #[test]
    fn a_frame_the_walk_runs_out_before_has_no_start_and_a_part_word_is_dropped() {
        let mut data = RledBuilder::new(2, 1)
            .frame_count(3)
            .frame(|f| f.token(1, 8).pixels(&[RED, RED]))
            .build();
        data.extend([1, 0]);
        let words = rled_words(&data).expect("reads");
        assert_eq!(words.words.len(), 4);
        assert_eq!(words.starts, [Some(0), None, None]);
        let open = RledBuilder::new(2, 1)
            .frame_count(2)
            .token(1, 8)
            .token(2, 4)
            .build();
        assert_eq!(rled_words(&open).expect("reads").starts, [Some(0), None]);
    }

    #[test]
    fn an_8_bit_sheet_keeps_its_depth_and_a_bad_header_fails() {
        let eight = RledBuilder::new(2, 1)
            .depth(8)
            .frame(|f| f.token(1, 8).pixels8(&[1, 2]))
            .build();
        assert_eq!(rled_words(&eight).expect("reads").depth, 8);
        let deep = RledBuilder::new(2, 1).depth(32).build();
        assert_eq!(
            rled_words(&deep),
            Err(GraphicsError::UnsupportedDepth { depth: 32 })
        );
        let none = RledBuilder::new(2, 1).build();
        assert_eq!(rled_words(&none), Err(GraphicsError::NoFrames));
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
            words.words,
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
        assert_eq!(words.starts, [Some(0), Some(9)]);
        // At 8 bits, a byte a pixel.
        let one = SpriteSheet::new(4, 1, vec![image(4, &["##.#"])], SheetLayout::DEFAULT);
        assert_eq!(
            sheet_rle_words(&one, 8).words,
            [0x0100_0014, 0x0200_0002, 0, 0x0300_0001, 0x0200_0001, 0, 0]
        );
    }
}
