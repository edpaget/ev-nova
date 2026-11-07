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
//!   frame's +0x5e, depth >> 4 (`_SWCreateFrame` @0xad607-0xad611), with
//!   `shll %cl`, which takes only the count's low 5 bits: any shift
//!   `0..=31`, `(depth >> 4) & 31`. So x counts `2^shift` bytes of the RLE
//!   lines a pixel: 1 for a 16-bit sheet, whose shift cancels its 2-byte
//!   pixels, 0 for an 8-bit one, whose byte counts are read as pixels,
//!   twice the width it is drawn at (`EightBitReading::AsDrawn` in
//!   `nova_data`), cut to its rect, and 2 for a 32-bit one, whose shift
//!   cancels its 4-byte pixels, so it touches by its own pixels at its
//!   box's size though it is drawn twice as wide. At another depth the
//!   shift need not match its pixels' bytes: a 24-bit sheet (shift 1)
//!   touches by each line's first 2W bytes, a 48-bit one (shift 3) reads
//!   past its line's bytes to the next line start, and a depth of 512
//!   (shift 0) reads its bytes as pixels. A 16-bit sprite against an 8-bit
//!   one compares the two in mixed units, bounded by the first's shifted
//!   width. Then `_BlitPixieRLECollision` (@0xc0c8d-0xc0ebe) walks both.
//!   The original stores each shifted left and right as a 16-bit word
//!   (@0xbbabc, @0xbbaca, @0xbbadb, @0xbbae9), so they wrap at a shift of
//!   16 or more; that integer width is not replicated: they are kept in
//!   `i64`.
//! - **The tokens.** Each is a word as the swap left it in memory, read
//!   little-endian, an op in its top byte and a byte count below. They
//!   are the very tokens the blitter draws: an `rlëD`'s, from
//!   `_SWLockSprite`'s walk (@0xaa9b6-0xaaa83), or those
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
//! The walk keeps bytes, as the original's does: a line start steps
//! `4 + count` bytes, so a count short of a word puts the next line, and
//! any frame after it, at an unaligned byte; pixels step past their data
//! to the next multiple of 4 (the handle is aligned, so an offset aligns
//! as its address does); any other token steps 4. A walk past a frame's
//! last token reads on into the next frame's, as the original's does, and
//! a word running past the sheet's end reads as a frame end, where the
//! original would read past its handle. None of these routines plays a
//! sound.

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
    /// Its frames' +0x5e as `_BlitPixieRLECollisionProc` shifts by it,
    /// `(depth >> 4) & 31`: 1 at 16 bits, 0 at 8, 2 at 32, and at other
    /// depths 1 at 24, 3 at 48, 31 at 0x7FFF.
    pub shift: u8,
    /// The tokens after the 16-byte header as bytes, as the swap left
    /// them in memory: each word read little-endian.
    pub bytes: Vec<u8>,
    /// Each frame's first byte, as `_SWLockSprite` walks to it.
    pub starts: Vec<usize>,
}

impl RleSheet {
    /// Frame `n`, modulo the frames, as `_SWSetCurrentFrameIndex` takes it
    /// (@0xa90d4-0xa90e9); `None` when the sheet has no frames.
    #[must_use]
    pub fn frame(&self, n: u16) -> Option<RleFrame<'_>> {
        let count = self.starts.len();
        if count == 0 {
            return None;
        }
        let start = self.starts[usize::from(n) % count];
        Some(RleFrame { sheet: self, start })
    }

    /// The little-endian word at byte `at`: a frame end where it would run
    /// past the sheet's end.
    fn word(&self, at: usize) -> u32 {
        match self.bytes.get(at..) {
            Some(&[a, b, c, d, ..]) => u32::from_le_bytes([a, b, c, d]),
            _ => 0,
        }
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

    /// Its width, right less left, in whole pixels.
    #[must_use]
    pub fn width(self) -> i64 {
        i64::from(self.right) - i64::from(self.left)
    }

    /// Whether it overlaps `other`, both ways strictly, as
    /// `_SWCollideSpriteLayer` tests two sprites' rects before their proc
    /// (@0xacc30-0xacc81) and `_SWRLECollision` again (@0xac020-0xac088):
    /// rects that share only an edge do not.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self.overlap(other).is_some()
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

/// Past the token at byte `at` and the `count` bytes after it: where a
/// line start's next line starts (@0xc0d7a-0xc0d84).
fn past(at: usize, count: i64) -> usize {
    at + 4 + usize::try_from(count).unwrap_or(0)
}

/// Past the pixels token at byte `at` and its `count` bytes of data, to
/// the next multiple of 4: the original aligns the address
/// (@0xc0df3-0xc0e04), and the handle is aligned, so the offset aligns as
/// the address does.
fn past_pixels(at: usize, count: i64) -> usize {
    past(at, count).next_multiple_of(4)
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
        at = past(at, count);
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
                self.at = past_pixels(self.at, count);
            }
            SKIP => {
                self.x += count;
                self.drawing = false;
                self.at += 4;
            }
            _ => self.at += 4,
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
        let mut side_a = Side::new(a.sheet, line_a + 4, -la.left);
        let mut side_b = Side::new(b.sheet, line_b + 4, -lb.left);
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
        line_a = past(line_a, count_a);
        line_b = past(line_b, count_b);
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
                at += 4;
            }
            PIXELS => {
                draw(x);
                x += pixels;
                at = past_pixels(at, count);
            }
            SKIP => {
                x += pixels;
                at += 4;
            }
            RUN => {
                draw(x);
                x += pixels;
                at += 8;
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
    use crate::fixture::rle::{le_bytes, of_words, sheet, sheet8, token, with_fill, words_of};

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
    fn rects_overlap_only_strictly_on_both_axes() {
        let a = MaskRect::placed((0, 0), (4, 4));
        assert!(a.overlaps(MaskRect::placed((3, 3), (4, 4))));
        assert!(!a.overlaps(MaskRect::placed((4, 0), (4, 4))), "an edge");
        assert!(!a.overlaps(MaskRect::placed((0, 4), (4, 4))), "an edge");
        assert!(!a.overlaps(MaskRect::placed((0, -4), (4, 4))), "above");
        assert!(!a.overlaps(MaskRect::placed((-4, 0), (4, 4))), "left");
        assert!(a.overlaps(MaskRect::placed((-3, -3), (4, 4))));
        assert!(!a.overlaps(MaskRect::placed((1, 1), (0, 2))), "no width");
        assert_eq!(MaskRect::placed((3, 1), (9, 2)).width(), 9);
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
        // Nor the mirror at the left edge, the second's pixel outside it.
        assert!(!touch(&left, (1, 0), &left, (0, 0), MaskWalk::Original));
        // The second read from its own left, left of the overlap: its pixel
        // outside the overlap is two away from the first's.
        let far = sheet(&[&["#..."]]);
        for walk in BOTH {
            assert!(
                !touch(&sheet(&[&[".#."]]), (1, 0), &far, (0, 0), walk),
                "{walk:?}"
            );
        }
        // Not at its right edge, the first's rect reaching left of it.
        let end = sheet(&[&["..#"]]);
        assert!(!touch(&end, (0, 0), &end, (1, 0), MaskWalk::Original));
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
        assert_eq!(words_of(&cut)[second_line], token(LINE_START, 8));
        cut.bytes.truncate(4 * second_line);
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
    fn by_the_original_a_16_bit_and_an_8_bit_sheet_each_shift_by_their_own_depth_and_stop_at_the_firsts_width()
     {
        use MaskWalk::Original;
        // Pixels 0 and 2 of 3: 16-bit (shift 1), and 8-bit (bytes 0 and 3
        // of 6, shift 0).
        let wide = sheet(&[&["#.#"]]);
        let bytes = sheet8(&[&["#..#.."]]);
        assert_eq!(wide.shift, 1);
        assert_eq!((bytes.width, bytes.shift), (3, 0));
        // 16-bit A: its width doubles to 2, and B's local rect keeps its
        // own shift of 0, so B's byte 3 meets A's bytes 0 and 1.
        assert!(touch(&wide, (0, 0), &bytes, (-2, 0), Original));
        // 8-bit A: B's local rect still doubles by B's own shift of 1, so
        // B's pixel 2 (bytes 4 and 5) meets A's byte 0.
        assert!(touch(&bytes, (0, 0), &wide, (-2, 0), Original));
        // The walk stops at A's shifted width of 2, before A's byte 3 is
        // compared with B's pixel 2.
        assert!(!touch(&bytes, (0, 0), &wide, (-1, 0), Original));
    }

    #[test]
    fn a_lower_row_is_reached_past_the_lines_above_it() {
        let both = sheet(&[&["##", "##"]]);
        for walk in BOTH {
            assert!(
                touch(&both, (0, 0), &sheet(&[&["..", "##"]]), (0, 0), walk),
                "{walk:?}"
            );
            // Each frame's lines above the overlap skipped by their counts.
            assert!(touch(&both, (0, -1), &sheet(&[&["##"]]), (0, 0), walk));
            assert!(touch(&sheet(&[&["##"]]), (0, 0), &both, (0, -1), walk));
        }
    }

    /// One 6-pixel row: pixels 0 and 1, whose data word reads as a skip of
    /// 40 bytes, a skip of 2, then pixels 4 and 5.
    fn data_like_a_skip() -> RleSheet {
        let words = [
            token(LINE_START, 24),
            token(PIXELS, 4),
            token(SKIP, 40),
            token(SKIP, 4),
            token(PIXELS, 4),
            0x7FFF_7FFF,
            0,
        ];
        of_words(6, 1, 1, &words, &[0])
    }

    #[test]
    fn a_pixels_data_is_skipped_not_read_as_tokens() {
        let tricky = data_like_a_skip();
        let last = sheet(&[&["....##"]]);
        for walk in BOTH {
            assert!(touch(&tricky, (0, 0), &last, (0, 0), walk), "{walk:?}");
            assert!(touch(&last, (0, 0), &tricky, (0, 0), walk), "{walk:?}");
        }
        // A pixel after a skip on its line, as drawn.
        let gap = sheet(&[&["#.#"]]);
        assert!(touch(
            &gap,
            (0, 0),
            &sheet(&[&["..#"]]),
            (0, 0),
            MaskWalk::Drawn
        ));
        assert!(!touch(
            &gap,
            (0, 0),
            &sheet(&[&[".#."]]),
            (0, 0),
            MaskWalk::Drawn
        ));
    }

    #[test]
    fn a_frame_end_at_a_rows_line_start_touches_nothing_whatever_follows() {
        // One line, then a frame end, then pixels no line starts.
        let words = [
            token(LINE_START, 8),
            token(PIXELS, 4),
            0x7FFF_7FFF,
            0,
            token(PIXELS, 4),
            0x7FFF_7FFF,
        ];
        let ended = of_words(2, 2, 1, &words, &[0]);
        let rows = sheet(&[&["..", "##"]]);
        let rect = MaskRect::placed((0, 0), (2, 2));
        for walk in BOTH {
            let (a, b) = (
                ended.frame(0).expect("a frame"),
                rows.frame(0).expect("a frame"),
            );
            assert!(!rle_collision(a, rect, b, rect, walk), "{walk:?}");
            assert!(!rle_collision(b, rect, a, rect, walk), "{walk:?}");
        }
    }

    #[test]
    fn rows_below_the_overlap_are_not_compared_though_the_frames_hold_them() {
        let two = sheet(&[&["..", "##"]]);
        let three = sheet(&[&["..", "..", "##"]]);
        let (a, b) = (
            two.frame(0).expect("a frame"),
            three.frame(0).expect("a frame"),
        );
        let short = MaskRect::placed((0, 1), (2, 1));
        let tall = MaskRect::placed((0, 0), (2, 3));
        for walk in BOTH {
            assert!(!rle_collision(a, short, b, tall, walk), "{walk:?}");
        }
    }

    #[test]
    fn each_frame_is_read_from_its_own_left_as_drawn() {
        let drawn = MaskWalk::Drawn;
        assert!(touch(
            &sheet(&[&["#."]]),
            (1, 0),
            &sheet(&[&[".#"]]),
            (0, 0),
            drawn
        ));
        assert!(touch(
            &sheet(&[&["#."]]),
            (1, 0),
            &sheet(&[&["##"]]),
            (0, 0),
            drawn
        ));
        assert!(!touch(
            &sheet(&[&["##."]]),
            (0, 0),
            &sheet(&[&[".#"]]),
            (1, 0),
            drawn
        ));
        assert!(!touch(
            &sheet(&[&[".#"]]),
            (1, 0),
            &sheet(&[&["##."]]),
            (0, 0),
            drawn
        ));
    }

    /// A sheet of one frame at byte 0 of `bytes`, `width` x `height` at
    /// `shift`.
    fn of_bytes(width: u32, height: u32, shift: u8, bytes: Vec<u8>, start: usize) -> RleSheet {
        RleSheet {
            width,
            height,
            shift,
            bytes,
            starts: vec![start],
        }
    }

    #[test]
    fn a_line_count_short_of_a_word_puts_the_next_line_at_its_byte() {
        // Each line a pixel token of 2 bytes, counted 6: line 1 starts at
        // byte 10, its pixel token at 14.
        let line = |count| {
            [
                le_bytes(&[token(LINE_START, count), token(PIXELS, 2)]),
                vec![0xFF, 0x7F],
            ]
            .concat()
        };
        let lines = |rows: &[u32]| {
            let mut bytes: Vec<u8> = rows.iter().flat_map(|&count| line(count)).collect();
            bytes.extend([0; 6]);
            of_bytes(1, 2, 1, bytes, 0)
        };
        let unaligned = lines(&[6, 6]);
        let dot = sheet(&[&["#"]]);
        assert!(touch(&unaligned, (0, 0), &dot, (0, 1), MaskWalk::Original));
        assert!(touch(&dot, (0, 1), &unaligned, (0, 0), MaskWalk::Original));
        // Counted 8, line 1 would start at byte 12, where line 0's last
        // bytes and line 1's start read as a frame end.
        let mut aligned = unaligned.clone();
        aligned.bytes[0] = 8;
        assert!(!touch(&aligned, (0, 0), &dot, (0, 1), MaskWalk::Original));
    }

    #[test]
    fn a_frame_starting_at_an_unaligned_byte_is_read_from_there() {
        // Two filler bytes, then line 0 (a skip) at byte 2 and line 1 (a
        // pixel) at byte 10, its data padded to byte 20.
        let mut bytes = vec![0xAA, 0xAA];
        bytes.extend(le_bytes(&[
            token(LINE_START, 4),
            token(SKIP, 2),
            token(LINE_START, 6),
            token(PIXELS, 2),
        ]));
        bytes.extend([0xFF, 0x7F]);
        bytes.extend([0; 4]);
        let late = of_bytes(1, 2, 1, bytes, 2);
        let dot = sheet(&[&["#"]]);
        for walk in BOTH {
            assert!(touch(&late, (0, 0), &dot, (0, 1), walk), "{walk:?}");
            assert!(touch(&dot, (0, 1), &late, (0, 0), walk), "{walk:?}");
            assert!(
                !touch(&late, (0, 0), &dot, (0, 0), walk),
                "{walk:?}: row 0 clear"
            );
        }
    }

    #[test]
    fn pixels_at_an_unaligned_byte_pad_their_data_to_the_next_multiple_of_4() {
        // At byte 2: a line of 4 bytes of pixels (token at 6, data 10 to
        // 14), padding to 16, a skip of 2 at 16 and pixels at 20.
        let mut bytes = vec![0, 0];
        bytes.extend(le_bytes(&[token(LINE_START, 20), token(PIXELS, 4)]));
        bytes.extend([0xFF; 4]);
        bytes.extend([0; 2]);
        bytes.extend(le_bytes(&[token(SKIP, 2), token(PIXELS, 2)]));
        bytes.extend([0xFF, 0xFF, 0, 0, 0, 0, 0, 0]);
        let gap = of_bytes(4, 1, 1, bytes, 2);
        let last = sheet(&[&["...#"]]);
        for walk in BOTH {
            assert!(touch(&gap, (0, 0), &last, (0, 0), walk), "{walk:?}");
        }
        let third = sheet(&[&["..#"]]);
        assert!(!touch(&gap, (0, 0), &third, (0, 0), MaskWalk::Drawn));
    }

    /// A 2-pixel, 3-bytes-a-pixel line (a 24-bit sheet, shift 1) of
    /// `tokens`.
    fn sheet24(tokens: &[u32]) -> RleSheet {
        let count = u32::try_from(4 * tokens.len()).expect("small");
        let words = [&[token(LINE_START, count)], tokens, &[0]].concat();
        of_words(2, 1, 1, &words, &[0])
    }

    #[test]
    fn a_shift_short_of_a_pixels_bytes_touches_by_a_lines_first_bytes() {
        use MaskWalk::Original;
        // Shift 1 over 3-byte pixels: the 2 pixels' overlap is bytes 0 to 4.
        let solid = sheet(&[&["##"]]);
        let byte_3 = sheet24(&[token(SKIP, 3), token(PIXELS, 1), 0]);
        let byte_4 = sheet24(&[token(SKIP, 4), token(PIXELS, 2), 0]);
        assert!(touch(&solid, (0, 0), &byte_3, (0, 0), Original));
        assert!(
            !touch(&solid, (0, 0), &byte_4, (0, 0), Original),
            "pixel 1's 2nd byte"
        );
        // A local left of 1 starts the walk at byte 2, not 3.
        let byte_2 = sheet24(&[token(SKIP, 2), token(PIXELS, 1), 0, token(SKIP, 3)]);
        let dot = sheet(&[&["#"]]);
        assert!(touch(&dot, (1, 0), &byte_2, (0, 0), Original));
        assert!(!touch(&dot, (1, 0), &byte_4, (0, 0), Original));
    }

    #[test]
    fn a_shift_past_a_lines_bytes_reads_the_next_line_start_and_ends_the_row() {
        use MaskWalk::Original;
        // Shift 3 over 6-byte pixels: a pixel's overlap is 8 bytes, past
        // the line's 6.
        let words = [
            token(LINE_START, 4),
            token(SKIP, 6),
            token(LINE_START, 12),
            token(PIXELS, 6),
            0,
            0,
            0,
        ];
        let clear_then_drawn = of_words(1, 2, 3, &words, &[0]);
        let wide = sheet(&[&["####"]]);
        assert!(
            !touch(&clear_then_drawn, (0, 0), &wide, (0, 0), Original),
            "row 0 ends at line 1's start"
        );
        assert!(touch(&clear_then_drawn, (0, 0), &wide, (0, 1), Original));
        // On the last line it reads the frame end: nothing touches.
        let last = of_words(1, 1, 3, &[token(LINE_START, 4), token(SKIP, 6), 0], &[0]);
        assert!(!touch(&last, (0, 0), &wide, (0, 0), Original));
        let drawn = of_words(1, 1, 3, &words[2..], &[0]);
        assert!(touch(&drawn, (0, 0), &wide, (0, 0), Original));
    }

    #[test]
    fn a_word_running_past_the_sheets_end_reads_as_a_frame_end() {
        let solid = sheet(&[&["#"]]);
        let mut short = solid.clone();
        // Its pixel token's last byte cut: three bytes left of it.
        short.bytes.truncate(7);
        for walk in BOTH {
            assert!(!touch(&short, (0, 0), &solid, (0, 0), walk), "{walk:?}");
            assert!(touch(&solid, (0, 0), &solid, (0, 0), walk), "{walk:?}");
        }
    }

    #[test]
    fn a_frame_wraps_by_the_frame_count_and_the_walk_follows_its_source() {
        let mut two = sheet(&[&["#"], &["##"]]);
        assert_eq!(two.frame(3).expect("frame 1").start, 16, "its byte");
        assert_eq!(two.frame(2).expect("frame 0").start, 0);
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
        gone.starts.clear();
        let unread = PlacedMask {
            sheet: Rc::new(gone),
            frame: 1,
            rect: MaskRect::placed((0, 0), (2, 1)),
        };
        assert_eq!(unread.touches(&at(0), MaskWalk::Drawn), None);
        assert_eq!(at(0).touches(&unread, MaskWalk::Drawn), None);
    }
}
