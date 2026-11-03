//! Run-length sheets drawn as text, for the tests of the masks
//! ([`sprite_mask`](crate::sprite_mask)) here and in the crates above.
//!
//! Each frame is a list of rows, each row a string of `'#'` (a drawn
//! pixel), `'.'` (a clear one) and `'r'` (a pixel of a solid run). A row
//! becomes a line start, whose count is the byte length of the line's
//! tokens, then one token per maximal run of one character: `'#'` an
//! op-2 pixel token with its data (white, padded to a word), `'.'` an
//! op-3 skip and `'r'` an op-4 run with its fill word. A trailing skip is
//! dropped, as `_BlitPixieRLEParsePixels` drops it, and each frame ends
//! with a 0 word.

use crate::sprite_mask::{FRAME_END, LINE_START, PIXELS, RUN, RleSheet, SKIP};

/// The fill word of stock runs, `0x7FFF7FFF`: white, read as op 0x7F.
pub const STOCK_FILL: u32 = 0x7FFF_7FFF;

/// A 16-bit sheet (shift 1, two bytes a pixel) of `frames`, its runs of
/// [`STOCK_FILL`].
#[must_use]
pub fn sheet(frames: &[&[&str]]) -> RleSheet {
    with_fill(frames, STOCK_FILL)
}

/// A 16-bit sheet of `frames`, as [`sheet`] gives, its runs' fill word
/// `fill`.
#[must_use]
pub fn with_fill(frames: &[&[&str]], fill: u32) -> RleSheet {
    let width = widest(frames);
    build(frames, 2, fill, 1, width)
}

/// An 8-bit sheet (shift 0) of `frames`, each character one byte: its
/// width is half its widest row's bytes, rounded up, as the 16-bit world
/// draws its bytes two to a pixel.
#[must_use]
pub fn sheet8(frames: &[&[&str]]) -> RleSheet {
    let width = widest(frames).div_ceil(2);
    build(frames, 1, STOCK_FILL, 0, width)
}

/// A token: `op` in the top byte, `count` below.
#[must_use]
pub fn token(op: u8, count: u32) -> u32 {
    (u32::from(op) << 24) | (count & 0x00FF_FFFF)
}

/// The widest row of `frames`, in characters.
fn widest(frames: &[&[&str]]) -> u32 {
    let widest = frames
        .iter()
        .flat_map(|rows| rows.iter())
        .map(|row| row.len())
        .max()
        .unwrap_or(0);
    u32::try_from(widest).expect("a small fixture")
}

/// The sheet of `frames`, `unit` bytes a character, runs filled with
/// `fill`, at `shift` and `width`.
fn build(frames: &[&[&str]], unit: u32, fill: u32, shift: u8, width: u32) -> RleSheet {
    let mut words = Vec::new();
    let mut starts = Vec::new();
    for rows in frames {
        starts.push(Some(words.len()));
        for row in *rows {
            let line = line_tokens(row, unit, fill);
            let bytes = u32::try_from(4 * line.len()).expect("a small fixture");
            words.push(token(LINE_START, bytes));
            words.extend(line);
        }
        words.push(token(FRAME_END, 0));
    }
    let height = frames.iter().map(|rows| rows.len()).max().unwrap_or(0);
    RleSheet {
        width,
        height: u32::try_from(height).expect("a small fixture"),
        shift,
        words,
        starts,
    }
}

/// The tokens of one row (see the module docs).
fn line_tokens(row: &str, unit: u32, fill: u32) -> Vec<u32> {
    let row = row.trim_end_matches('.').as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < row.len() {
        let c = row[i];
        let n = row[i..].iter().take_while(|&&next| next == c).count();
        let count = u32::try_from(n).expect("a small fixture") * unit;
        match c {
            b'#' => {
                tokens.push(token(PIXELS, count));
                let data = usize::try_from(count.div_ceil(4)).expect("a small fixture");
                tokens.extend(std::iter::repeat_n(STOCK_FILL, data));
            }
            b'.' => tokens.push(token(SKIP, count)),
            b'r' => {
                tokens.push(token(RUN, count));
                tokens.push(fill);
            }
            other => panic!("no such pixel: {:?}", char::from(other)),
        }
        i += n;
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_is_a_line_start_and_a_token_per_run_with_no_trailing_skip() {
        let sheet = sheet(&[&[".##r.", "#"]]);
        assert_eq!((sheet.width, sheet.height, sheet.shift), (5, 2, 1));
        assert_eq!(sheet.starts, [Some(0)]);
        assert_eq!(
            sheet.words,
            [
                token(1, 20),
                token(3, 2),
                token(2, 4),
                STOCK_FILL,
                token(4, 2),
                STOCK_FILL,
                token(1, 8),
                token(2, 2),
                STOCK_FILL,
                0,
            ]
        );
    }

    #[test]
    fn frames_follow_each_other_and_an_8_bit_sheet_counts_bytes() {
        let two = with_fill(&[&["#"], &["r"]], 0x0102_0304);
        assert_eq!(two.starts, [Some(0), Some(4)]);
        assert_eq!(&two.words[4..], [token(1, 8), token(4, 2), 0x0102_0304, 0]);
        let eight = sheet8(&[&["..###"]]);
        assert_eq!((eight.width, eight.shift), (3, 0));
        assert_eq!(
            eight.words,
            [token(1, 12), token(3, 2), token(2, 3), STOCK_FILL, 0]
        );
    }
}
