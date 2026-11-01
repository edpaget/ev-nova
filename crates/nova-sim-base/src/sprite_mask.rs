//! Whether two placed sprites touch by their pixels, as the original's
//! `_SWRLECollision` (@0xabfbd-0xac1a6) tests them (`EV Nova.app`, i386):
//! over the run-length tokens each sheet is drawn by ([`RleSheet`]), not a
//! bitmask of its pixels. The walk is the original's, quirks included
//! ([`rle_collision`]).
//!
//! - **The rects** ([`MaskRect`]). A sprite's rect is a Mac `Rect` at
//!   +0x1a: top, left, bottom, right. `_SWSetSpriteLocation` (@0xab640)
//!   places it at the frame's hot spot, keeping its size. Two sprites
//!   touch nothing unless their rects overlap, max left < min right and
//!   max top < min bottom, both strict (@0xac020-0xac088). Each sprite's
//!   local rect is the overlap less its own top left (@0xac0a0-0xac0ef).
//! - **The shift** ([`RleSheet::shift`]). `_BlitPixieRLECollisionProc`
//!   (@0xbbab2-0xbbae9) shifts each local rect's left and right by its
//!   frame's +0x5e, depth >> 4 (`_SWCreateFrame` @0xad5f3), so x counts
//!   the bytes of the RLE lines: 1 for a 16-bit sheet, whose shift cancels
//!   its 2-byte pixels, and 0 for an 8-bit one, whose byte counts are read
//!   as pixels, twice the width it is drawn at (`EightBitReading::AsDrawn`
//!   in `nova_data`), cut to its rect. A 16-bit sprite against an 8-bit one
//!   compares the two in mixed units, bounded by the first's shifted
//!   width. Then `_BlitPixieRLECollision` (@0xc0c8d-0xc0ebe) walks both.
//! - **The tokens.** Each is a big-endian word, an op in its top byte and
//!   a byte count below. They are the very tokens the blitter draws: an
//!   `rlëD`'s, from `_SWLockSprite`'s walk (@0xaa9b6-0xaaa83), or those
//!   `_BlitPixieRLEParsePixels` (@0xc2875-0xc2bef) emits for a `PICT` and
//!   its mask, which hold no runs.
//! - **The rows.** The walk skips each frame's first local-top lines, a
//!   line start each (a frame end there touches nothing), then for each of
//!   the first's rows reads a line start from each, a frame end touching
//!   nothing (@0xc0c98-0xc0d71).
//! - **The merge.** Each side starts at x = −its shifted local left, not
//!   drawing. While the lesser x is short of the first's shifted width,
//!   the side behind reads its next token, **the second when they are
//!   equal** (@0xc0db6). A line start sets its x to the width and ends the
//!   row; a frame end touches nothing; pixels (op 2) add their count, draw
//!   and skip their data padded to a word; a skip (op 3) adds its count
//!   and stops drawing; any other op, a run (op 4) among them, changes
//!   nothing and moves on one word. They touch as soon as both are drawing
//!   (@0xc0e6a-0xc0e77).
//!
//! That walk has two quirks, which [`MaskWalk::Original`] keeps:
//!
//! 1. **Runs.** A solid run is passed with no width, and its fill word is
//!    read as a token: stock's `0x7FFF7FFF` is op 0x7F, ignored too. So a
//!    run never touches, and each later token on its line is read the
//!    run's length to the left of where it is drawn. A fill whose top byte
//!    is 0 to 3 reads as a frame end, a line start, pixels or a skip.
//! 2. **The equal step.** As the second steps on a tie, the first's drawn
//!    token ending at x meets the second's starting at x: the first's last
//!    pixel at x − 1 touches the second's first at x, in the same row,
//!    just left of the overlap too, but not the other way about.
//!
//! [`MaskWalk::Drawn`] reads the tokens as the 16-bit blitter draws them,
//! runs included and every count halved to pixels: two sprites touch only
//! where both draw one pixel inside the overlap.
//!
//! The walk keeps whole words: pixels round their data up to a word and a
//! line start skips count / 4 words, as the original does; an unaligned
//! count (not in stock) is not modelled. A walk past a frame's last token
//! reads on into the next frame's, as the original's does, and past the
//! sheet's end reads a frame end, where the original would read past its
//! handle. None of these routines plays a sound.

use std::rc::Rc;

#[cfg(doc)]
use crate::rulebook::RuleKey;
use crate::rulebook::RuleSource;

/// The op that ends a frame.
pub const FRAME_END: u8 = 0;
/// The op that starts a line, its count the line's bytes after it.
pub const LINE_START: u8 = 1;
/// The op of drawn pixels, its data after it padded to a word.
pub const PIXELS: u8 = 2;
/// The op of clear pixels.
pub const SKIP: u8 = 3;
/// The op of a solid run, its fill word after it.
pub const RUN: u8 = 4;

/// The low 24 bits of a token: its byte count.
const COUNT_MASK: u32 = 0x00FF_FFFF;

/// A sheet's run-length tokens as the collision reads them (see the
/// module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RleSheet {
    /// Each frame's width in pixels.
    pub width: u32,
    /// Each frame's height in pixels.
    pub height: u32,
    /// Its frames' +0x5e, depth >> 4: 1 at 16 bits, 0 at 8.
    pub shift: u8,
    /// The tokens after the 16-byte header, each word read big-endian.
    pub words: Vec<u32>,
    /// Each frame's first word, as `_SWLockSprite` walks to it; `None`
    /// for a frame the walk ran out before.
    pub starts: Vec<Option<usize>>,
}

impl RleSheet {
    /// Frame `n`, modulo the frames, as `_SWSetCurrentFrameIndex` takes it
    /// (@0xa90d4-0xa90e9); `None` when its start wasn't found.
    #[must_use]
    pub fn frame(&self, n: u16) -> Option<RleFrame<'_>> {
        let count = self.starts.len();
        if count == 0 {
            return None;
        }
        let start = self.starts[usize::from(n) % count]?;
        Some(RleFrame { sheet: self, start })
    }

    /// The word at `at`: a frame end past the sheet's end.
    fn word(&self, at: usize) -> u32 {
        self.words.get(at).copied().unwrap_or(0)
    }
}

/// One frame of an [`RleSheet`]: where its tokens start.
#[derive(Clone, Copy, Debug)]
pub struct RleFrame<'a> {
    sheet: &'a RleSheet,
    start: usize,
}

/// A placed sprite's rect, a Mac `Rect`, in screen pixels relative to the
/// player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MaskRect {
    /// Its top.
    pub top: i32,
    /// Its left.
    pub left: i32,
    /// Its bottom, past its last row.
    pub bottom: i32,
    /// Its right, past its last column.
    pub right: i32,
}

impl MaskRect {
    /// The rect of `size` (width, height) with its top left at `top_left`
    /// (x, y).
    #[must_use]
    pub fn placed(top_left: (i32, i32), size: (u32, u32)) -> Self {
        let whole = |length: u32| i32::try_from(length).unwrap_or(i32::MAX);
        Self {
            top: top_left.1,
            left: top_left.0,
            bottom: top_left.1.saturating_add(whole(size.1)),
            right: top_left.0.saturating_add(whole(size.0)),
        }
    }

    /// Where it overlaps `other`, if they overlap, both ways strictly
    /// (@0xac020-0xac088).
    fn overlap(self, other: Self) -> Option<Self> {
        let overlap = Self {
            top: self.top.max(other.top),
            left: self.left.max(other.left),
            bottom: self.bottom.min(other.bottom),
            right: self.right.min(other.right),
        };
        (overlap.left < overlap.right && overlap.top < overlap.bottom).then_some(overlap)
    }
}

/// How two sprites' tokens are walked, by [`RuleKey::RleCollisionWalk`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MaskWalk {
    /// The original's walk, its quirks kept (see the module docs).
    #[default]
    Original,
    /// The tokens as the blitter draws them: two sprites touch only where
    /// both draw a pixel.
    Drawn,
}

impl From<RuleSource> for MaskWalk {
    /// The engine's reading is the original's walk; the other reads the
    /// tokens as drawn.
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Original,
            RuleSource::Bible => Self::Drawn,
        }
    }
}

/// A sprite's sheet, the frame it shows and where it is placed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedMask {
    /// The sheet it is drawn from.
    pub sheet: Rc<RleSheet>,
    /// The frame it shows.
    pub frame: u16,
    /// Where it is placed.
    pub rect: MaskRect,
}

impl PlacedMask {
    /// Whether it touches `other` by `walk` ([`rle_collision`], it first);
    /// `None` when either's frame can't be read.
    #[must_use]
    pub fn touches(&self, other: &Self, walk: MaskWalk) -> Option<bool> {
        let a = self.sheet.frame(self.frame)?;
        let b = other.sheet.frame(other.frame)?;
        Some(rle_collision(a, self.rect, b, other.rect, walk))
    }
}

/// Whether frame `a` placed at `a_rect` touches frame `b` placed at
/// `b_rect`, walked by `walk` (see the module docs): `_SWRLECollision`,
/// `_BlitPixieRLECollisionProc` and `_BlitPixieRLECollision` for
/// [`MaskWalk::Original`]. The order matters to the original's walk: `a`
/// is the one whose rows and width bound it, and the one whose tie goes
/// second.
#[must_use]
pub fn rle_collision(
    a: RleFrame<'_>,
    a_rect: MaskRect,
    b: RleFrame<'_>,
    b_rect: MaskRect,
    walk: MaskWalk,
) -> bool {
    let Some(overlap) = a_rect.overlap(b_rect) else {
        return false;
    };
    match walk {
        MaskWalk::Original => original_walk(
            a,
            Local::of(overlap, a_rect, a.sheet.shift),
            b,
            Local::of(overlap, b_rect, b.sheet.shift),
        ),
        MaskWalk::Drawn => drawn_walk(a, a_rect, b, b_rect, overlap),
    }
}

/// The overlap in one frame's own coordinates, its left and right shifted
/// to bytes.
#[derive(Clone, Copy, Debug)]
struct Local {
    top: i64,
    left: i64,
    bottom: i64,
    right: i64,
}

impl Local {
    /// `overlap` less `rect`'s top left, its left and right shifted by
    /// `shift`.
    fn of(overlap: MaskRect, rect: MaskRect, shift: u8) -> Self {
        let x = |x: i32| (i64::from(x) - i64::from(rect.left)) << shift;
        let y = |y: i32| i64::from(y) - i64::from(rect.top);
        Self {
            top: y(overlap.top),
            left: x(overlap.left),
            bottom: y(overlap.bottom),
            right: x(overlap.right),
        }
    }
}

/// A token's op and byte count.
fn split(word: u32) -> (u8, i64) {
    // The top byte, by the shift.
    #[allow(clippy::cast_possible_truncation)]
    let op = (word >> 24) as u8;
    (op, i64::from(word & COUNT_MASK))
}

/// The whole words in a count of `bytes`: a line's count is a whole
/// number of words in every sheet modelled.
fn words(bytes: i64) -> usize {
    usize::try_from(bytes.div_euclid(4)).unwrap_or(0)
}

/// Where `frame`'s line `lines` lines down starts, skipping a line start
/// each; `None` at a frame end.
fn skip_lines(frame: RleFrame<'_>, lines: i64) -> Option<usize> {
    let mut at = frame.start;
    for _ in 0..lines {
        let (op, count) = split(frame.sheet.word(at));
        if op == FRAME_END {
            return None;
        }
        at += 1 + words(count);
    }
    Some(at)
}

/// What one side of the merge read.
enum Read {
    /// It read a frame end: they touch nothing.
    FrameEnd,
    /// It read some other token.
    Token,
}

/// One side of the original's merge along a row.
struct Side<'a> {
    sheet: &'a RleSheet,
    at: usize,
    x: i64,
    drawing: bool,
    line_ended: bool,
}

impl<'a> Side<'a> {
    fn new(sheet: &'a RleSheet, at: usize, x: i64) -> Self {
        Self {
            sheet,
            at,
            x,
            drawing: false,
            line_ended: false,
        }
    }

    /// Reads its next token (see the module docs) in a row `width` wide.
    fn read(&mut self, width: i64) -> Read {
        let (op, count) = split(self.sheet.word(self.at));
        match op {
            FRAME_END => return Read::FrameEnd,
            LINE_START => {
                self.x = width;
                self.drawing = false;
                self.line_ended = true;
            }
            PIXELS => {
                self.x += count;
                self.drawing = true;
                self.at += 1 + words(count + 3);
            }
            SKIP => {
                self.x += count;
                self.drawing = false;
                self.at += 1;
            }
            _ => self.at += 1,
        }
        Read::Token
    }
}

/// `_BlitPixieRLECollision` over `a` and `b`, their overlaps `la` and
/// `lb` (see the module docs).
fn original_walk(a: RleFrame<'_>, la: Local, b: RleFrame<'_>, lb: Local) -> bool {
    let (Some(mut line_a), Some(mut line_b)) = (skip_lines(a, la.top), skip_lines(b, lb.top))
    else {
        return false;
    };
    let width = la.right - la.left;
    for _ in la.top..la.bottom {
        let (op_a, count_a) = split(a.sheet.word(line_a));
        let (op_b, count_b) = split(b.sheet.word(line_b));
        if op_a == FRAME_END || op_b == FRAME_END {
            return false;
        }
        let mut side_a = Side::new(a.sheet, line_a + 1, -la.left);
        let mut side_b = Side::new(b.sheet, line_b + 1, -lb.left);
        while side_a.x.min(side_b.x) < width {
            let behind = if side_a.x < side_b.x {
                &mut side_a
            } else {
                &mut side_b
            };
            if let Read::FrameEnd = behind.read(width) {
                return false;
            }
            if side_a.drawing && side_b.drawing {
                return true;
            }
            if side_a.line_ended || side_b.line_ended {
                break;
            }
        }
        line_a += 1 + words(count_a);
        line_b += 1 + words(count_b);
    }
    false
}

/// The pixels `frame` draws on each of its `rows` lines from `first`, as
/// the 16-bit blitter draws its tokens: spans from x to x + width, each
/// count halved to pixels, runs drawn, the frame's tokens read in order
/// to its end.
fn drawn_rows(frame: RleFrame<'_>, first: i64, rows: i64) -> Vec<Vec<(i64, i64)>> {
    let mut drawn = vec![Vec::new(); usize::try_from(rows).unwrap_or(0)];
    let mut at = frame.start;
    let mut line: i64 = -1;
    let mut x = 0;
    loop {
        let (op, count) = split(frame.sheet.word(at));
        let pixels = count / 2;
        let mut draw = |x: i64| {
            if let Some(spans) = usize::try_from(line - first)
                .ok()
                .and_then(|row| drawn.get_mut(row))
            {
                spans.push((x, x + pixels));
            }
        };
        match op {
            LINE_START => {
                line += 1;
                x = 0;
                if line >= first + rows {
                    break;
                }
                at += 1;
            }
            PIXELS => {
                draw(x);
                x += pixels;
                at += 1 + words(count + 3);
            }
            SKIP => {
                x += pixels;
                at += 1;
            }
            RUN => {
                draw(x);
                x += pixels;
                at += 2;
            }
            _ => break,
        }
    }
    drawn
}

/// [`MaskWalk::Drawn`]: whether `a` at `a_rect` and `b` at `b_rect` draw a
/// pixel in the same place inside `overlap`.
fn drawn_walk(
    a: RleFrame<'_>,
    a_rect: MaskRect,
    b: RleFrame<'_>,
    b_rect: MaskRect,
    overlap: MaskRect,
) -> bool {
    let rows = i64::from(overlap.bottom) - i64::from(overlap.top);
    let first = |rect: MaskRect| i64::from(overlap.top) - i64::from(rect.top);
    let rows_a = drawn_rows(a, first(a_rect), rows);
    let rows_b = drawn_rows(b, first(b_rect), rows);
    let (left, right) = (i64::from(overlap.left), i64::from(overlap.right));
    rows_a.iter().zip(&rows_b).any(|(spans_a, spans_b)| {
        spans_a.iter().any(|&(a0, a1)| {
            let (a0, a1) = (a0 + i64::from(a_rect.left), a1 + i64::from(a_rect.left));
            spans_b.iter().any(|&(b0, b1)| {
                let (b0, b1) = (b0 + i64::from(b_rect.left), b1 + i64::from(b_rect.left));
                a0.max(b0).max(left) < a1.min(b1).min(right)
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::rle::{sheet, sheet8, token, with_fill};

    const BOTH: [MaskWalk; 2] = [MaskWalk::Original, MaskWalk::Drawn];

    /// Whether frame 0 of `a`, its top left at `a_at`, touches frame 0 of
    /// `b` at `b_at` by `walk`, each placed at its sheet's size.
    fn touch(
        a: &RleSheet,
        a_at: (i32, i32),
        b: &RleSheet,
        b_at: (i32, i32),
        walk: MaskWalk,
    ) -> bool {
        let rect = |sheet: &RleSheet, at| MaskRect::placed(at, (sheet.width, sheet.height));
        rle_collision(
            a.frame(0).expect("a frame"),
            rect(a, a_at),
            b.frame(0).expect("a frame"),
            rect(b, b_at),
            walk,
        )
    }

    #[test]
    fn a_rect_is_placed_by_its_top_left_and_size() {
        assert_eq!(
            MaskRect::placed((-5, 3), (10, 4)),
            MaskRect {
                top: 3,
                left: -5,
                bottom: 7,
                right: 5,
            }
        );
    }

    #[test]
    fn rects_that_share_only_an_edge_or_are_apart_touch_nothing() {
        let solid = sheet(&[&["##"]]);
        for walk in BOTH {
            assert!(touch(&solid, (0, 0), &solid, (1, 0), walk), "{walk:?}");
            assert!(!touch(&solid, (0, 0), &solid, (2, 0), walk), "an edge");
            assert!(!touch(&solid, (0, 0), &solid, (-2, 0), walk), "an edge");
            assert!(!touch(&solid, (0, 0), &solid, (0, 1), walk), "below");
            assert!(!touch(&solid, (0, 0), &solid, (0, -1), walk), "above");
            assert!(!touch(&solid, (0, 0), &solid, (5, 5), walk), "apart");
        }
    }

    #[test]
    fn overlapping_rects_touch_only_where_both_draw() {
        for walk in BOTH {
            assert!(
                !touch(
                    &sheet(&[&["#.."]]),
                    (0, 0),
                    &sheet(&[&["..#"]]),
                    (0, 0),
                    walk
                ),
                "{walk:?}: the overlap clear"
            );
            assert!(touch(
                &sheet(&[&["##."]]),
                (0, 0),
                &sheet(&[&[".#."]]),
                (0, 0),
                walk
            ));
            assert!(touch(
                &sheet(&[&[".#."]]),
                (0, 0),
                &sheet(&[&["##."]]),
                (0, 0),
                walk
            ));
        }
    }

    #[test]
    fn only_the_overlap_is_compared_each_frame_read_from_its_own_top() {
        for walk in BOTH {
            assert!(
                !touch(
                    &sheet(&[&["#.."]]),
                    (0, 0),
                    &sheet(&[&["..#"]]),
                    (1, 0),
                    walk
                ),
                "{walk:?}: both drawn outside"
            );
            let low = sheet(&[&["..", "..", "##"]]);
            assert!(touch(&low, (0, 0), &sheet(&[&["..", "##"]]), (0, 1), walk));
            assert!(!touch(&low, (0, 0), &sheet(&[&["##", ".."]]), (0, 1), walk));
            assert!(touch(&sheet(&[&["##", ".."]]), (0, 1), &low, (0, -1), walk));
        }
    }

    #[test]
    fn by_the_original_a_run_never_touches_and_shifts_the_rest_of_its_line_left() {
        let run = sheet(&[&["r"]]);
        let drawn = sheet(&[&["#"]]);
        assert!(!touch(&run, (0, 0), &drawn, (0, 0), MaskWalk::Original));
        assert!(!touch(&drawn, (0, 0), &run, (0, 0), MaskWalk::Original));
        assert!(touch(&run, (0, 0), &drawn, (0, 0), MaskWalk::Drawn));
        assert!(touch(&drawn, (0, 0), &run, (0, 0), MaskWalk::Drawn));
        // Its `#` drawn at 2 is read at 1, the run's length to its left.
        let after = sheet(&[&["r.#"]]);
        let middle = sheet(&[&[".#."]]);
        assert!(touch(&after, (0, 0), &middle, (0, 0), MaskWalk::Original));
        assert!(!touch(&after, (0, 0), &middle, (0, 0), MaskWalk::Drawn));
        // A pixel drawn after a run of 2 misses the other's at its place.
        let late = sheet(&[&["rr.#"]]);
        let last = sheet(&[&["...#"]]);
        assert!(!touch(&late, (0, 0), &last, (0, 0), MaskWalk::Original));
        assert!(touch(&late, (0, 0), &last, (0, 0), MaskWalk::Drawn));
    }

    #[test]
    fn by_the_original_a_fill_word_read_as_a_frame_end_touches_nothing() {
        let solid = sheet(&[&["##"]]);
        let stock = sheet(&[&["r#"]]);
        let ending = with_fill(&[&["r#"]], 0x0000_7FFF);
        assert!(touch(&stock, (0, 0), &solid, (0, 0), MaskWalk::Original));
        assert!(!touch(&ending, (0, 0), &solid, (0, 0), MaskWalk::Original));
        assert!(touch(&ending, (0, 0), &solid, (0, 0), MaskWalk::Drawn));
    }

    #[test]
    fn by_the_original_the_firsts_pixel_touches_the_seconds_just_right_of_it() {
        let left = sheet(&[&["#."]]);
        let right = sheet(&[&[".#"]]);
        assert!(touch(&left, (0, 0), &right, (0, 0), MaskWalk::Original));
        assert!(
            !touch(&right, (0, 0), &left, (0, 0), MaskWalk::Original),
            "not the mirror"
        );
        assert!(!touch(&left, (0, 0), &right, (0, 0), MaskWalk::Drawn));
        // At the overlap's left edge too, the first's pixel outside it.
        assert!(touch(&left, (0, 0), &left, (1, 0), MaskWalk::Original));
        assert!(!touch(&left, (0, 0), &left, (1, 0), MaskWalk::Drawn));
        // Not at its right edge, the second's pixel outside it.
        assert!(!touch(
            &right,
            (0, 0),
            &sheet(&[&["..#"]]),
            (0, 0),
            MaskWalk::Original
        ));
        // Not a row apart.
        assert!(!touch(
            &sheet(&[&["#.", ".."]]),
            (0, 0),
            &sheet(&[&["..", ".#"]]),
            (0, 0),
            MaskWalk::Original
        ));
    }

    #[test]
    fn a_frame_short_of_lines_or_a_walk_past_the_sheet_touches_nothing_below() {
        let lower = sheet(&[&["..", "##"]]);
        let one_line = sheet(&[&["##"]]);
        let tall = MaskRect::placed((0, 0), (2, 2));
        let lower_rect = MaskRect::placed((0, 0), (2, 2));
        let mut cut = sheet(&[&["##", "##"]]);
        let second_line = 3;
        assert_eq!(cut.words[second_line], token(LINE_START, 8));
        cut.words.truncate(second_line);
        for walk in BOTH {
            for short in [&one_line, &cut] {
                let frame = short.frame(0).expect("a frame");
                assert!(
                    !rle_collision(
                        frame,
                        tall,
                        lower.frame(0).expect("a frame"),
                        lower_rect,
                        walk
                    ),
                    "{walk:?}"
                );
                assert!(rle_collision(
                    frame,
                    tall,
                    one_line.frame(0).expect("a frame"),
                    lower_rect,
                    walk
                ));
            }
        }
    }

    #[test]
    fn an_8_bit_sheets_byte_counts_are_read_as_pixels() {
        // Drawn at pixel 1; its bytes 2 and 3 read as pixels 2 and 3, past
        // its 2-pixel rect.
        let late = sheet8(&[&["..##"]]);
        assert_eq!((late.width, late.shift), (2, 0));
        assert!(!touch(&late, (0, 0), &late, (0, 0), MaskWalk::Original));
        assert!(touch(&late, (0, 0), &late, (0, 0), MaskWalk::Drawn));
        // One byte drawn, no pixel as drawn, a pixel as read.
        let byte = sheet8(&[&[".#.."]]);
        let first = sheet8(&[&["##.."]]);
        assert!(touch(&first, (0, 0), &byte, (0, 0), MaskWalk::Original));
        assert!(!touch(&first, (0, 0), &byte, (0, 0), MaskWalk::Drawn));
    }

    #[test]
    fn a_frame_wraps_by_the_frame_count_and_the_walk_follows_its_source() {
        let mut two = sheet(&[&["#"], &["##"]]);
        assert_eq!(two.frame(3).expect("frame 1").start, 4);
        assert_eq!(two.frame(2).expect("frame 0").start, 0);
        two.starts[1] = None;
        assert!(two.frame(1).is_none());
        two.starts.clear();
        assert!(two.frame(0).is_none());
        assert_eq!(MaskWalk::from(RuleSource::Engine), MaskWalk::Original);
        assert_eq!(MaskWalk::from(RuleSource::Bible), MaskWalk::Drawn);
        assert_eq!(MaskWalk::default(), MaskWalk::Original);
    }

    #[test]
    fn a_placed_mask_touches_by_its_frame_and_none_without_one() {
        let shared = Rc::new(sheet(&[&["#."], &[".#"]]));
        let at = |frame| PlacedMask {
            sheet: Rc::clone(&shared),
            frame,
            rect: MaskRect::placed((0, 0), (2, 1)),
        };
        assert_eq!(at(0).touches(&at(0), MaskWalk::Drawn), Some(true));
        assert_eq!(at(0).touches(&at(1), MaskWalk::Drawn), Some(false));
        let mut gone = sheet(&[&["#."], &[".#"]]);
        gone.starts[1] = None;
        let unread = PlacedMask {
            sheet: Rc::new(gone),
            frame: 1,
            rect: MaskRect::placed((0, 0), (2, 1)),
        };
        assert_eq!(unread.touches(&at(0), MaskWalk::Drawn), None);
        assert_eq!(at(0).touches(&unread, MaskWalk::Drawn), None);
    }
}
