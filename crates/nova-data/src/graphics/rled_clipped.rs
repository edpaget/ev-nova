//! An `rlëD` frame at the world's edge, as `_BlitPixieRLEClipped` draws it.
//!
//! The draw proc (`_BlitPixieRLETranslucentDrawProc` @0xbb3fc) takes the
//! plain blit only for a frame whose box lies wholly inside the world.
//! Otherwise it hands `_BlitPixieRLEClipped` (@0xbde9c-0xbe0b8) the swapped
//! tokens, the box's own top-left as the destination, and a clip rect in
//! the box's terms (@0xbb4af-0xbb5a0): top and bottom in rows, the bottom
//! the box's height, and left and right in bytes, shifted by the 16-bit
//! world's shift, the right the box's width times 2 and exclusive. The
//! translucent twin a murked stellar takes, `_BlitPixieRLETranslucentClipped`
//! (@0xc18c0-0xc1bca), does the same with every token, so one clipped decode
//! serves both. Every depth takes it, 8 as drawn included (@0xbb895-0xbb8b1
//! → @0xbb751).
//!
//! # What it draws
//!
//! - **Rows.** A line start above the clip's top jumps the stream by its
//!   count, unaligned, without walking the row's tokens (@0xbe080-0xbe0a5):
//!   the arithmetic of `_SWLockSprite`'s walk, so the skips land where that
//!   walk landed, on a line start or a frame end, which ends the frame. A
//!   line start at or below the clip's bottom ends the frame (@0xbe0aa-0xbe0b3).
//!   A sheet whose line counts disagree with its tokens draws desynced when
//!   cut at the top, and whole when not: the plain blit ignores the counts.
//! - **Runs** (@0xbdefe-0xbdf6b). With `last = x + count - 1`, a run is
//!   skipped whole when `x >= right` or `last <= left`. Otherwise it is
//!   trimmed to the left, and to the right when `last > right`, and
//!   `_BlitPixieMemSet` (@0xc0452-0xc04a0) fills it from the cut with the
//!   run's word as it was: the fill restarts at the cut. A 16-bit run cut by
//!   an odd number of pixels swaps its word's halves from there.
//! - **Pixels** (@0xbdfa0-0xbe06a) are skipped and trimmed alike, and
//!   copied from the cut (`_BlitPixieMemCopy`): what lies inside the clip
//!   is what the plain blit draws there. The stream steps over every byte
//!   and aligns to 4 either way.
//! - **Skips** step on (@0xbe070-0xbe074), and a frame end or an op above 4
//!   returns (@0xbdedd-0xbdee4, @0xbdef9-0xbdefc). Nothing plays a sound.
//!
//! The `last <= left` test skips a token whose last byte is the clip's left
//! byte, though that byte is inside the clip, even at a left of 0
//! (@0xbdf16-0xbdf1c, @0xbdfe4-0xbdfea): a one-byte token at a row's start
//! is dropped. The left is even, so only a token ending on an even byte is,
//! which a 16 or 32-bit sheet's whole pixels never do, but an 8-bit one's
//! bytes can. The world pixel whose byte goes unwritten keeps the
//! background's byte in the original; here it is clear, as everywhere a
//! pixel is half written.
//!
//! The decoded frames are the box: header width x height, the rows above
//! the cut and the pixels left of it clear. Only the top and left cuts
//! change what a frame draws; a cut at the right or the bottom is a crop
//! of this, which the view's clip makes.
//!
//! # Left out
//!
//! - **The extra byte past the right.** The right trim leaves a token
//!   ending at byte `right`, one past the exclusive right (runs
//!   @0xbdf3d-0xbdf47, pixels @0xbe013-0xbe01d; the translucent twin
//!   @0xc1adf-0xc1ae8, @0xc1b74-0xc1b7b). That byte is the low byte of the
//!   first world pixel past the box, outside the world's rect, and never
//!   seen. The draw stops at the box's right; it only counts that byte
//!   when it works out where a trimmed run's fill ends.
//! - **Reads of unknown memory.** After a desynced skip, the tokens can be
//!   read from inside pixel data, and run on past the resource, where the
//!   original reads whatever memory follows. The draw stops the frame at
//!   the first read past the resource, keeping what it drew.
//!
//! # Which sheets
//!
//! A sheet read otherwise than the original draws it (an 8-bit sheet
//! through the system palette, a 32-bit one in true colour) has no blit to
//! port: its clipped draw is its plain decode cut to the box. So is a 16
//! or 8-bit sheet whose line counts fail the lock walk, which today's token
//! walk loads though the original would not. [`SpriteSheet::clip_quirks`]
//! says whether a sheet's clipped draw can differ from that crop at all.

use super::GraphicsError;
use super::color::rgb555;
use super::image::Image;
use super::reader::Reader;
use super::rled::{Header, PIXELS, RUN, RledReadings, SKIP, decode_rled_with, read_header};
use super::rled_wide::{COUNT_MASK, align4, locked, word_at};
use super::rled8::{EightBitReading, mem_set_byte};
use super::rled32::ThirtyTwoBitReading;
use super::sheet::{SheetLayout, SpriteSheet};

/// The op of a frame end.
const FRAME_END: u8 = 0;
/// The op of a line start.
const LINE_START: u8 = 1;

/// How much of a frame's box lies outside the world, at its top and its
/// left: the rows above the world's top edge and the pixels left of its
/// left edge, counted from the box's top-left.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ClipCut {
    /// The box's rows above the world's top edge.
    pub top: u16,
    /// The box's pixels left of the world's left edge.
    pub left: u16,
}

/// Decodes an `rlëD` as `_BlitPixieRLEClipped` draws each frame whose box
/// the world cuts by `cut` (see the module docs): a sheet of box-sized
/// frames, with the solid runs of that draw when the plain decode has
/// any, reading an 8 or 32-bit sheet as `readings` say.
///
/// It fails where [`decode_rled_with`] fails, as it does. A cut past the
/// box clears it all.
pub fn decode_rled_clipped(
    data: &[u8],
    layout: Option<SheetLayout>,
    readings: RledReadings,
    cut: ClipCut,
) -> Result<SpriteSheet, GraphicsError> {
    let plain = decode_rled_with(data, layout, readings)?;
    let header = read_header(&mut Reader::new(data), data.len())?;
    let port = takes_the_blit(&header, readings)
        .then(|| locked(data, &header).ok())
        .flatten();
    let sheet = match port {
        Some((mem, starts)) => drawn(&mem, &starts, &header, cut, &plain),
        None => cropped(&plain, cut),
    };
    Ok(sheet
        .with_depth(plain.depth())
        .with_clip_quirks(plain.clip_quirks()))
}

/// Whether `header`'s sheet, read as `readings` say, is drawn as the
/// original's blit draws it: every depth but an 8-bit sheet through the
/// palette and a 32-bit one in true colour.
pub(super) fn takes_the_blit(header: &Header, readings: RledReadings) -> bool {
    match header.depth {
        8 => readings.eight_bit == EightBitReading::AsDrawn,
        32 => readings.thirty_two_bit == ThirtyTwoBitReading::AsDrawn,
        _ => true,
    }
}

/// Whether the `rlëD` `data`, whose checked header is `header`, can draw
/// otherwise than a crop when clipped ([`SpriteSheet::clip_quirks`]):
/// false when it fails the lock walk, else whether any frame, walked by
/// its tokens from where the walk found it, has a line start whose count
/// doesn't land on the next line start or frame end, a run whose fill
/// shows a restart at a cut, or a token ending on an even byte.
pub(super) fn clip_quirks(data: &[u8], header: &Header) -> bool {
    let Ok((mem, starts)) = locked(data, header) else {
        return false;
    };
    starts.into_iter().any(|start| frame_quirks(&mem, start))
}

/// Whether the frame at `start` of the swapped `mem` has a clip quirk
/// (see [`clip_quirks`]). A token walk that runs out or meets an op above
/// 4 counts as one.
fn frame_quirks(mem: &[u8], start: usize) -> bool {
    let (mut pos, mut x) = (start, 0u64);
    // Where the last line start's count lands.
    let mut lands = None;
    // Each token moves on at least 4 bytes, so the data runs out first.
    loop {
        let Ok(token) = word_at(mem, pos) else {
            return true;
        };
        let n = u64::from(token & COUNT_MASK);
        let data = pos + 4;
        let op = (token >> 24) as u8;
        if (op == FRAME_END || op == LINE_START) && lands.take().is_some_and(|at| at != pos) {
            return true;
        }
        match op {
            FRAME_END => return false,
            LINE_START => {
                lands = Some(data + n as usize);
                (pos, x) = (data, 0);
            }
            PIXELS => {
                if ends_on_an_even_byte(x, n) {
                    return true;
                }
                pos = align4(data + n as usize);
                x += n;
            }
            SKIP => {
                pos = data;
                x += n;
            }
            RUN => {
                let Ok(word) = word_at(mem, data) else {
                    return true;
                };
                if ends_on_an_even_byte(x, n) || restart_shows(word, x) {
                    return true;
                }
                pos = data + 4;
                x += n;
            }
            _ => return true,
        }
    }
}

/// Whether an `n`-byte token from byte `x` ends on an even byte, which the
/// clip's even left can skip it at.
fn ends_on_an_even_byte(x: u64, n: u64) -> bool {
    n > 0 && !(x + n).is_multiple_of(2)
}

/// Whether a run of `word` (as read big-endian from the file) from byte
/// `x` fills otherwise when restarted at a cut: the clip's left is even,
/// so a run from an even byte is cut by an even number of bytes, which
/// shows only when its halves differ, and one from an odd byte by an odd
/// number, which shows unless its bytes are all alike.
fn restart_shows(word: u32, x: u64) -> bool {
    let bytes = word.to_le_bytes();
    if x.is_multiple_of(2) {
        bytes[..2] != bytes[2..]
    } else {
        bytes.iter().any(|&byte| byte != bytes[0])
    }
}

/// `_BlitPixieRLEClipped`'s clip rect for a box cut by a [`ClipCut`]:
/// top and bottom in rows, left and right in bytes, all from the box's
/// top-left; the bottom and right exclusive.
#[derive(Clone, Copy, Debug)]
struct Clip {
    top: u64,
    left: u64,
    bottom: u64,
    right: u64,
}

impl Clip {
    /// The clip of `header`'s box cut by `cut`, each cut at most the box.
    fn of(header: &Header, cut: ClipCut) -> Self {
        let (width, height) = (u64::from(header.width), u64::from(header.height));
        Self {
            top: u64::from(cut.top).min(height),
            left: 2 * u64::from(cut.left).min(width),
            bottom: height,
            right: 2 * width,
        }
    }

    /// What of an `n`-byte token from byte `x` the blit draws: none when it
    /// is skipped whole; else the byte it starts from, the count it hands
    /// on (which runs one byte past the right when trimmed there), and how
    /// many of those bytes lie inside the box.
    fn cut(&self, x: u64, n: u64) -> Option<(u64, u64, u64)> {
        // `last <= left`, with `last = x + n - 1`.
        if x >= self.right || x + n <= self.left + 1 {
            return None;
        }
        let from = x.max(self.left);
        let last = x + n - 1;
        let count = last.min(self.right) + 1 - from;
        Some((from, count, count.min(self.right - from)))
    }
}

/// One frame's drawn bytes within the box, row by row: each byte's value
/// and whether a run wrote it.
struct Bytes {
    row: usize,
    cells: Vec<Option<(u8, bool)>>,
}

impl Bytes {
    fn new(clip: &Clip) -> Self {
        // The header passed the decoding budget, so these fit memory.
        let row = clip.right as usize;
        Self {
            row,
            cells: vec![None; row * clip.bottom as usize],
        }
    }

    /// Writes `value` at byte `x` of row `y`, both inside the box.
    fn put(&mut self, x: u64, y: u64, value: u8, run: bool) {
        self.cells[y as usize * self.row + x as usize] = Some((value, run));
    }

    /// The frame's `width` x `height` pixels, each two bytes a
    /// little-endian RGB555 pixel, clear unless both were written, and by
    /// runs alone when `runs` says so.
    fn image(&self, width: u32, height: u32, runs: bool) -> Image {
        let mut image = Image::transparent(width, height);
        for (y, row) in (0..height).zip(self.cells.chunks(self.row)) {
            for (x, pair) in (0..width).zip(row.chunks(2)) {
                if let [Some((low, low_run)), Some((high, high_run))] = *pair
                    && (!runs || (low_run && high_run))
                {
                    image.put(x, y, rgb555(u16::from_le_bytes([low, high])));
                }
            }
        }
        image
    }
}

/// `_BlitPixieRLEClipped` (@0xbde9c-0xbe0b8; see the module docs) of the
/// frame whose tokens start at `start` of the swapped `mem`, into `frame`.
fn draw_frame(mem: &[u8], start: usize, clip: &Clip, frame: &mut Bytes) {
    let (mut pos, mut x, mut y, mut row) = (start, 0u64, 0u64, 0u64);
    // Each token moves on at least 4 bytes, so the data runs out first.
    while let Ok(token) = word_at(mem, pos) {
        let n = u64::from(token & COUNT_MASK);
        let data = pos + 4;
        match (token >> 24) as u8 {
            LINE_START => {
                pos = if row < clip.top {
                    data + n as usize
                } else if row >= clip.bottom {
                    return;
                } else {
                    data
                };
                (x, y) = (0, row);
                row += 1;
            }
            PIXELS => {
                if let Some((from, _, inside)) = clip.cut(x, n) {
                    let src = data + (from - x) as usize;
                    let Some(bytes) = mem.get(src..src + inside as usize) else {
                        return;
                    };
                    for (at, &byte) in (from..).zip(bytes) {
                        frame.put(at, y, byte, false);
                    }
                }
                pos = align4(data + n as usize);
                x += n;
            }
            SKIP => {
                pos = data;
                x += n;
            }
            RUN => {
                let Ok(word) = word_at(mem, data) else {
                    return;
                };
                if let Some((from, count, inside)) = clip.cut(x, n) {
                    // A count is at most 24 bits.
                    let count = count as u32;
                    for i in 0..inside {
                        frame.put(from + i, y, mem_set_byte(word, count, i as u32), true);
                    }
                }
                pos = data + 4;
                x += n;
            }
            _ => return,
        }
    }
}

/// The frames at `starts` of the swapped `mem`, drawn as `header`'s box
/// cut by `cut`; with solid runs when `plain`, the sheet's plain decode,
/// has them.
fn drawn(
    mem: &[u8],
    starts: &[usize],
    header: &Header,
    cut: ClipCut,
    plain: &SpriteSheet,
) -> SpriteSheet {
    let clip = Clip::of(header, cut);
    let (width, height) = (header.width, header.height);
    let (mut frames, mut solids) = (Vec::new(), Vec::new());
    for &start in starts {
        let mut bytes = Bytes::new(&clip);
        draw_frame(mem, start, &clip, &mut bytes);
        frames.push(bytes.image(width, height, false));
        solids.push(bytes.image(width, height, true));
    }
    let solid = plain.solid_frames().is_some().then_some(solids);
    SpriteSheet::new(width, height, frames, plain.layout()).with_solid(solid)
}

/// `plain`'s frames cut to their box, the `cut` cleared.
fn cropped(plain: &SpriteSheet, cut: ClipCut) -> SpriteSheet {
    let (width, height) = (plain.frame_width(), plain.frame_height());
    let (origin_x, origin_y) = plain.box_origin();
    let (left, top) = (
        u32::from(cut.left).min(width),
        u32::from(cut.top).min(height),
    );
    let crop = |image: &Image| {
        let mut out = Image::transparent(width, height);
        for y in top..height {
            for x in left..width {
                if let Some(pixel) = image.pixel(origin_x + x, origin_y + y) {
                    out.put(x, y, pixel);
                }
            }
        }
        out
    };
    let frames = plain.frames().iter().map(crop).collect();
    let solid = plain
        .solid_frames()
        .map(|solid| solid.iter().map(crop).collect());
    SpriteSheet::new(width, height, frames, plain.layout()).with_solid(solid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::color::rgb555;
    use crate::graphics::fixture::{RledBuilder, RledFrame};
    use crate::graphics::rled8::EightBitReading;
    use crate::graphics::rled32::ThirtyTwoBitReading;

    const CLEAR: [u8; 4] = [0; 4];
    const RED: u16 = 0x7C00;
    const GREEN: u16 = 0x03E0;
    const BLUE: u16 = 0x001F;
    const WHITE: u16 = 0x7FFF;

    fn rgba(image: &Image) -> Vec<[u8; 4]> {
        image
            .pixels()
            .chunks(4)
            .map(|p| p.try_into().unwrap())
            .collect()
    }

    fn frames(sheet: &SpriteSheet) -> Vec<Vec<[u8; 4]>> {
        sheet.frames().iter().map(rgba).collect()
    }

    fn solids(sheet: &SpriteSheet) -> Option<Vec<Vec<[u8; 4]>>> {
        sheet
            .solid_frames()
            .map(|solid| solid.iter().map(rgba).collect())
    }

    /// A sheet of `width` x `height` frames at `depth`, its lines counted.
    fn sheet(width: u16, height: u16, depth: u16) -> RledBuilder {
        RledBuilder::new(width, height).depth(depth).count_lines()
    }

    fn cut(top: u16, left: u16) -> ClipCut {
        ClipCut { top, left }
    }

    fn clipped(data: &[u8], at: ClipCut) -> SpriteSheet {
        decode_rled_clipped(data, None, RledReadings::default(), at).unwrap()
    }

    fn plain(data: &[u8]) -> SpriteSheet {
        decode_rled_with(data, None, RledReadings::default()).unwrap()
    }

    /// Each of `sheet`'s images within its box, row by row.
    fn boxes(sheet: &SpriteSheet) -> Vec<Vec<[u8; 4]>> {
        let (left, top) = sheet.box_origin();
        sheet
            .frames()
            .iter()
            .map(|image| {
                let mut inside = Vec::new();
                for y in top..top + sheet.frame_height() {
                    for x in left..left + sheet.frame_width() {
                        inside.push(image.pixel(x, y).unwrap());
                    }
                }
                inside
            })
            .collect()
    }

    /// A 16-bit 4x1 run whose word's halves differ: `A` high, `B` low.
    fn two_colour_run_16() -> Vec<u8> {
        sheet(4, 1, 16)
            .frame(|f| f.line().run(4, RED, BLUE))
            .build()
    }

    #[test]
    fn a_16_bit_run_cut_at_the_left_restarts_its_fill_at_the_cut() {
        let bytes = two_colour_run_16();
        let (low, high) = (rgb555(BLUE), rgb555(RED));
        assert_eq!(frames(&plain(&bytes)), [vec![low, high, low, high]]);
        // Cut by one pixel (two bytes): the fill restarts from the word's
        // low half at pixel 1, where the plain blit drew its high half.
        let one = clipped(&bytes, cut(0, 1));
        assert_eq!(frames(&one), [vec![CLEAR, low, high, low]]);
        assert_eq!(solids(&one), Some(vec![vec![CLEAR, low, high, low]]));
        // Cut by two, the restart lands where the plain fill was.
        assert_eq!(
            frames(&clipped(&bytes, cut(0, 2))),
            [vec![CLEAR, CLEAR, low, high]]
        );
    }

    #[test]
    fn a_32_bit_run_cut_by_an_odd_number_of_drawn_pixels_swaps_its_halves() {
        // A 4-pixel box: the run's two 32-bit pixels are its 8 bytes.
        let bytes = sheet(4, 1, 32)
            .frame(|f| f.line().run32(2, 0xA1B2_C3D4))
            .build();
        let (low, high) = (rgb555(0xC3D4), rgb555(0xA1B2));
        assert_eq!(boxes(&plain(&bytes)), [vec![low, high, low, high]]);
        assert_eq!(
            frames(&clipped(&bytes, cut(0, 1))),
            [vec![CLEAR, low, high, low]]
        );
        assert_eq!(
            frames(&clipped(&bytes, cut(0, 3))),
            [vec![CLEAR, CLEAR, CLEAR, low]]
        );
        assert_eq!(
            frames(&clipped(&bytes, cut(0, 2))),
            [vec![CLEAR, CLEAR, low, high]]
        );
    }

    /// A 2x3 sheet at `depth` whose rows are drawn by `row(i)`; row 0's
    /// line start counts row 0's tokens, row 1's line start and row 1's
    /// tokens, so it lands on row 2's line start.
    fn row_0_counts_over_row_1(depth: u16, row: impl Fn(RledFrame, usize) -> RledFrame) -> Vec<u8> {
        let row_bytes = |i| row(RledFrame::default(), i).byte_len() as u32;
        let first = row_bytes(0) + 4 + row_bytes(1);
        RledBuilder::new(2, 3)
            .depth(depth)
            .count_lines()
            .frame(|f| {
                let f = row(f.token(1, first), 0).line();
                let f = row(f, 1).line();
                row(f, 2)
            })
            .build()
    }

    #[test]
    fn rows_above_the_top_are_skipped_by_their_line_counts_at_16_bits() {
        let colours = [RED, GREEN, BLUE];
        let bytes = row_0_counts_over_row_1(16, |f, i| f.pixels(&[colours[i]; 2]));
        let [red, green, blue] = colours.map(rgb555);
        // The plain blit ignores the counts.
        let whole = vec![red, red, green, green, blue, blue];
        assert_eq!(frames(&plain(&bytes)), std::slice::from_ref(&whole));
        assert_eq!(frames(&clipped(&bytes, cut(0, 0))), [whole]);
        // Cut at the top by a row, the skip lands on row 2's line start,
        // which draws on row 1.
        assert_eq!(
            frames(&clipped(&bytes, cut(1, 0))),
            [vec![CLEAR, CLEAR, blue, blue, CLEAR, CLEAR]]
        );
    }

    #[test]
    fn rows_above_the_top_are_skipped_by_their_line_counts_at_32_bits() {
        let colours = [0x7C00_u32, 0x03E0, 0x001F];
        let bytes = row_0_counts_over_row_1(32, |f, i| f.pixels32(&[colours[i]]));
        let pixel = |c: u32| [rgb555(c as u16), rgb555(0)];
        let whole: Vec<_> = colours.iter().flat_map(|&c| pixel(c)).collect();
        assert_eq!(boxes(&plain(&bytes)), std::slice::from_ref(&whole));
        assert_eq!(frames(&clipped(&bytes, cut(0, 0))), [whole]);
        let mut desynced = vec![CLEAR; 2];
        desynced.extend(pixel(colours[2]));
        desynced.extend([CLEAR; 2]);
        assert_eq!(frames(&clipped(&bytes, cut(1, 0))), [desynced]);
    }

    #[test]
    fn a_skip_that_reaches_the_frame_end_draws_nothing_more() {
        // Row 0's count runs to the frame's end.
        let bytes = RledBuilder::new(2, 2)
            .frame(|f| {
                f.token(1, 20)
                    .pixels(&[RED, RED])
                    .token(1, 8)
                    .pixels(&[GREEN, GREEN])
            })
            .frame(|f| {
                f.token(1, 8)
                    .pixels(&[BLUE, BLUE])
                    .token(1, 8)
                    .pixels(&[GREEN, GREEN])
            })
            .build();
        let green = rgb555(GREEN);
        assert_eq!(
            frames(&plain(&bytes))[0],
            [rgb555(RED), rgb555(RED), green, green]
        );
        assert_eq!(
            frames(&clipped(&bytes, cut(1, 0))),
            [vec![CLEAR; 4], vec![CLEAR, CLEAR, green, green]]
        );
    }

    #[test]
    fn a_token_whose_last_byte_is_the_clips_left_byte_is_skipped_whole() {
        // An 8-bit sheet as drawn: a 3-byte token, then one writing byte 3.
        let bytes = sheet(2, 1, 8)
            .frame(|f| f.line().pixels8(&[0x11, 0x22, 0x33]).pixels8(&[0x44]))
            .build();
        assert_eq!(
            frames(&plain(&bytes)),
            [vec![rgb555(0x2211), rgb555(0x4433)]]
        );
        // Cut by one pixel, the first token ends on the clip's left byte,
        // 2: world pixel 1 keeps only its high byte, so it is clear.
        assert_eq!(frames(&clipped(&bytes, cut(0, 1))), [vec![CLEAR, CLEAR]]);
        // Even uncut, a one-byte token at a row's first byte is dropped.
        let bytes = sheet(2, 1, 8)
            .frame(|f| f.line().pixels8(&[0x11]).pixels8(&[0x22, 0x33, 0x44]))
            .build();
        assert_eq!(
            frames(&plain(&bytes)),
            [vec![rgb555(0x2211), rgb555(0x4433)]]
        );
        assert_eq!(
            frames(&clipped(&bytes, cut(0, 0))),
            [vec![CLEAR, rgb555(0x4433)]]
        );
    }

    #[test]
    fn a_token_past_the_boxs_right_draws_only_inside_the_box() {
        // A 2-pixel box: 4 bytes of each 8-byte token.
        let bytes = sheet(2, 2, 32)
            .frame(|f| {
                f.line()
                    .pixels32(&[0x0012_3456, 0x0078_9ABC])
                    .line()
                    .run32(2, 0xA1B2_C3D4)
            })
            .build();
        let (low, high) = (rgb555(0xC3D4), rgb555(0xA1B2));
        let sheet = clipped(&bytes, cut(0, 0));
        assert_eq!((sheet.image_width(), sheet.image_height()), (2, 2));
        assert_eq!(sheet.box_origin(), (0, 0));
        assert_eq!(
            frames(&sheet),
            [vec![rgb555(0x3456), rgb555(0x0012), low, high]]
        );
        // Cut by a pixel too, the run restarts at the cut.
        assert_eq!(
            frames(&clipped(&bytes, cut(0, 1))),
            [vec![CLEAR, rgb555(0x0012), CLEAR, low]]
        );
    }

    #[test]
    fn a_run_trimmed_at_the_right_counts_the_extra_byte_for_where_its_fill_ends() {
        // A 32-bit sheet with a 2-pixel box (bytes 0-3), its tokens counted
        // in bytes as `skip8` and `run8` write them: the plain decode
        // reads a 1-byte skip as none and a 5-byte run as one pixel, but
        // the blit trims a 5-byte run from byte 1 to bytes 1-3 and hands on
        // a count of 4 (one byte past the right). So byte 3 is the fill's
        // byte 2, not the one-byte tail's byte 0 that a count of 3 draws.
        let bytes = sheet(2, 1, 32)
            .frame(|f| f.line().skip8(1).run8(5, [0x11, 0x22, 0x33, 0x44]))
            .build();
        assert_eq!(
            frames(&clipped(&bytes, cut(0, 0))),
            [vec![CLEAR, rgb555(0x2233)]]
        );
    }

    /// Cuts that reach the row skip, the left trim, a run's restart and a
    /// token ending at the clip's left, and ones past the box.
    const CUTS: [(u16, u16); 10] = [
        (0, 0),
        (0, 1),
        (0, 2),
        (1, 0),
        (1, 1),
        (1, 2),
        (2, 3),
        (2, 0),
        (0, 9),
        (9, 0),
    ];

    /// The cuts at which `bytes`' clipped draw is not its plain decode
    /// cropped.
    fn cuts_drawn_otherwise(bytes: &[u8]) -> Vec<(u16, u16)> {
        let plain = plain(bytes);
        CUTS.into_iter()
            .filter(|&(top, left)| {
                let at = cut(top, left);
                let (drawn, crop) = (clipped(bytes, at), cropped(&plain, at));
                frames(&drawn) != frames(&crop) || solids(&drawn) != solids(&crop)
            })
            .collect()
    }

    /// Unflagged sheets: a 16, an 8 and a 32-bit one, and a stock-shaped one.
    fn sheets_without_quirks() -> Vec<Vec<u8>> {
        let sixteen = sheet(3, 2, 16)
            .frame(|f| {
                f.line()
                    .pixels(&[RED, GREEN])
                    .skip(1)
                    .line()
                    .run(3, WHITE, WHITE)
            })
            .frame(|f| f.line().skip(1).pixels(&[BLUE]))
            .build();
        let eight = sheet(2, 2, 8)
            .frame(|f| {
                f.line()
                    .pixels8(&[1, 2, 3, 4])
                    .line()
                    .skip8(2)
                    .run8(2, [9, 9, 9, 9])
            })
            .build();
        let thirty_two = sheet(2, 2, 32)
            .frame(|f| {
                f.line()
                    .pixels32(&[0x0012_3456, 0x0078_9ABC])
                    .line()
                    .run32(1, 0x7FFF_7FFF)
            })
            .build();
        vec![sixteen, eight, thirty_two, stock_shaped()]
    }

    fn stock_shaped() -> Vec<u8> {
        sheet(3, 2, 16)
            .frame(|f| {
                f.line()
                    .pixels(&[RED])
                    .run(2, WHITE, WHITE)
                    .line()
                    .skip(1)
                    .pixels(&[BLUE, GREEN])
            })
            .build()
    }

    /// Flagged sheets, one per kind of quirk, each drawn otherwise than a
    /// crop at some cut.
    fn sheets_with_quirks() -> Vec<Vec<u8>> {
        vec![
            row_0_counts_over_row_1(16, |f, i| f.pixels(&[[RED, GREEN, BLUE][i]; 2])),
            two_colour_run_16(),
            sheet(4, 1, 8)
                .frame(|f| f.line().skip8(1).run8(7, [1, 2, 3, 4]))
                .build(),
            sheet(2, 1, 8)
                .frame(|f| f.line().pixels8(&[1, 2, 3]).pixels8(&[4]))
                .build(),
            sheet(2, 1, 8)
                .frame(|f| f.line().run8(1, [5; 4]).pixels8(&[6, 7, 8]))
                .build(),
        ]
    }

    #[test]
    fn uncut_a_sheet_without_quirks_draws_its_box_as_the_plain_decode() {
        for bytes in sheets_without_quirks() {
            let plain = plain(&bytes);
            assert!(!plain.clip_quirks());
            let clipped = clipped(&bytes, cut(0, 0));
            assert_eq!(frames(&clipped), boxes(&plain));
            assert_eq!(solids(&clipped).is_some(), plain.solid_frames().is_some());
            assert_eq!(clipped.depth(), plain.depth());
        }
    }

    #[test]
    fn a_sheet_without_quirks_draws_as_its_plain_decode_cropped_at_every_cut() {
        for bytes in sheets_without_quirks() {
            assert!(!plain(&bytes).clip_quirks());
            assert_eq!(cuts_drawn_otherwise(&bytes), []);
        }
    }

    #[test]
    fn every_sheet_drawn_otherwise_than_a_crop_at_some_cut_is_flagged() {
        // A flag that under-flags any of these leaves a sheet drawn as a
        // crop though its clipped draw differs.
        for (i, bytes) in sheets_with_quirks().iter().enumerate() {
            let otherwise = cuts_drawn_otherwise(bytes);
            assert!(!otherwise.is_empty(), "sheet {i} draws as a crop");
            assert!(
                plain(bytes).clip_quirks(),
                "sheet {i} differs at {otherwise:?} but is not flagged"
            );
        }
    }

    #[test]
    fn a_stock_shaped_sheet_has_no_clip_quirks() {
        assert!(!plain(&stock_shaped()).clip_quirks());
    }

    #[test]
    fn lying_line_counts_two_colour_runs_and_tokens_ending_on_even_bytes_are_clip_quirks() {
        let lying = row_0_counts_over_row_1(16, |f, _| f.pixels(&[RED; 2]));
        assert!(plain(&lying).clip_quirks(), "a line count lies");
        assert!(
            plain(&two_colour_run_16()).clip_quirks(),
            "a run's halves differ"
        );
        let odd_run = sheet(2, 1, 8)
            .frame(|f| f.line().skip8(1).run8(3, [1, 1, 2, 2]))
            .build();
        assert!(plain(&odd_run).clip_quirks(), "an odd run's bytes differ");
        let odd_uniform = sheet(2, 1, 8)
            .frame(|f| f.line().skip8(1).run8(3, [3; 4]))
            .build();
        assert!(!plain(&odd_uniform).clip_quirks(), "an odd run of one byte");
        let even_end = sheet(2, 1, 8)
            .frame(|f| f.line().pixels8(&[1, 2, 3]))
            .build();
        assert!(
            plain(&even_end).clip_quirks(),
            "a token ends on an even byte"
        );
        let even_run = sheet(2, 1, 8).frame(|f| f.line().run8(1, [5; 4])).build();
        assert!(plain(&even_run).clip_quirks(), "a run ends on an even byte");
    }

    #[test]
    fn a_sheet_read_otherwise_or_failing_the_lock_walk_has_no_clip_quirks() {
        let eight = sheet(3, 1, 8)
            .frame(|f| f.line().pixels8(&[1, 2, 3]))
            .build();
        assert!(plain(&eight).clip_quirks());
        let through_palette = RledReadings {
            eight_bit: EightBitReading::SystemPalette,
            ..RledReadings::default()
        };
        let decoded = decode_rled_with(&eight, None, through_palette).unwrap();
        assert!(!decoded.clip_quirks());
        let thirty_two = sheet(4, 1, 32)
            .frame(|f| f.line().run32(2, 0xA1B2_C3D4))
            .build();
        assert!(plain(&thirty_two).clip_quirks());
        let true_colour = RledReadings {
            thirty_two_bit: ThirtyTwoBitReading::TrueColour,
            ..RledReadings::default()
        };
        let decoded = decode_rled_with(&thirty_two, None, true_colour).unwrap();
        assert!(!decoded.clip_quirks());
        // Its line counts left 0, a 16-bit sheet fails the lock walk.
        let uncounted = RledBuilder::new(4, 1)
            .frame(|f| f.line().run(4, RED, BLUE))
            .build();
        assert!(!plain(&uncounted).clip_quirks());
        // Which leaves the clipped draw a crop.
        assert_eq!(frames(&clipped(&uncounted, cut(0, 1)))[0][1], rgb555(RED));
    }

    #[test]
    fn read_otherwise_the_clipped_draw_is_the_plain_decode_cut() {
        let thirty_two = sheet(2, 2, 32)
            .frame(|f| {
                f.line()
                    .pixels32(&[0x0012_3456, 0x0078_9ABC])
                    .line()
                    .run32(2, 0x00AB_CDEF)
            })
            .build();
        let true_colour = RledReadings {
            thirty_two_bit: ThirtyTwoBitReading::TrueColour,
            ..RledReadings::default()
        };
        let whole = decode_rled_with(&thirty_two, None, true_colour).unwrap();
        let sheet = decode_rled_clipped(&thirty_two, None, true_colour, cut(1, 1)).unwrap();
        let mut expected = boxes(&whole);
        for frame in &mut expected {
            frame[0] = CLEAR;
            frame[1] = CLEAR;
            frame[2] = CLEAR;
        }
        assert_eq!(frames(&sheet), expected);
        assert_eq!(sheet.depth(), 32);
        let solid = solids(&sheet).unwrap();
        assert_eq!(solid, [vec![CLEAR, CLEAR, CLEAR, [0xAB, 0xCD, 0xEF, 255]]]);
        let eight = sheet_8_through_palette();
        let palette = RledReadings {
            eight_bit: EightBitReading::SystemPalette,
            ..RledReadings::default()
        };
        let whole = decode_rled_with(&eight, None, palette).unwrap();
        let sheet = decode_rled_clipped(&eight, None, palette, cut(0, 1)).unwrap();
        let mut expected = frames(&whole);
        expected[0][0] = CLEAR;
        assert_eq!(frames(&sheet), expected);
        assert_eq!(sheet.depth(), 8);
    }

    fn sheet_8_through_palette() -> Vec<u8> {
        sheet(2, 1, 8).frame(|f| f.line().pixels8(&[1, 2])).build()
    }

    #[test]
    fn cuts_past_the_box_clear_it() {
        let bytes = two_colour_run_16();
        assert_eq!(frames(&clipped(&bytes, cut(0, 9))), [vec![CLEAR; 4]]);
        assert_eq!(frames(&clipped(&bytes, cut(9, 0))), [vec![CLEAR; 4]]);
    }

    #[test]
    fn a_desynced_skip_that_reads_past_the_resource_stops_drawing() {
        // An 8-bit sheet: row 0's count lands inside its own pixel data,
        // on bytes that read as a line start whose count lands on the
        // frame's end. Below the top, the blit reads the word after that
        // as a pixel token of 0xFFFF bytes, past the resource.
        let bytes = RledBuilder::new(4, 2)
            .depth(8)
            .frame(|f| f.token(1, 4).pixels8(&[4, 0, 0, 1, 0xFF, 0xFF, 0x00, 0x02]))
            .build();
        assert!(decode_rled_with(&bytes, None, RledReadings::default()).is_ok());
        let sheet = decode_rled_clipped(&bytes, None, RledReadings::default(), cut(1, 0));
        assert_eq!(sheet.map(|sheet| frames(&sheet)), Ok(vec![vec![CLEAR; 8]]));
    }

    fn clipped_at_1_1(data: &[u8]) -> Result<SpriteSheet, GraphicsError> {
        decode_rled_clipped(data, None, RledReadings::default(), cut(1, 1))
    }

    fn clipped_at_0_0(data: &[u8]) -> Result<SpriteSheet, GraphicsError> {
        decode_rled_clipped(data, None, RledReadings::default(), cut(0, 0))
    }

    fn clipped_at_2_3(data: &[u8]) -> Result<SpriteSheet, GraphicsError> {
        decode_rled_clipped(data, None, RledReadings::default(), cut(2, 3))
    }

    #[test]
    fn corrupt_sheets_never_panic_when_clipped() {
        let wide = sheet(2, 3, 32)
            .frame(|f| {
                f.line()
                    .pixels32(&[0x0012_3456, 0x0078_9ABC])
                    .line()
                    .skip32(1)
                    .run32(1, 0x7FFF_7FFF)
                    .line()
            })
            .frame(|f| f.line().pixels32(&[1]).line())
            .build();
        let sixteen = row_0_counts_over_row_1(16, |f, i| f.skip(1).run(1, RED, i as u16));
        let eight = sheet(3, 2, 8)
            .frame(|f| f.line().pixels8(&[1, 2, 3]).run8(2, [1, 2, 3, 4]).line())
            .build();
        for bytes in [wide, sixteen, eight] {
            assert!(bytes.len() < 1024);
            for decode in [clipped_at_0_0, clipped_at_1_1, clipped_at_2_3] {
                crate::sweep::assert_never_panics(&bytes, decode);
            }
        }
    }

    #[test]
    fn the_flag_follows_each_tokens_place_along_its_row() {
        // After a pixel token from byte 1 to 3, a one-byte run lands on
        // byte 4, an even end.
        let after_pixels = sheet(3, 1, 8)
            .frame(|f| f.line().skip8(1).pixels8(&[1, 2, 3]).run8(1, [5; 4]))
            .build();
        assert!(plain(&after_pixels).clip_quirks());
        let after_run = sheet(3, 1, 8)
            .frame(|f| f.line().skip8(1).run8(3, [5; 4]).run8(1, [6; 4]))
            .build();
        assert!(plain(&after_run).clip_quirks());
        // A run of no bytes ends on no byte.
        let empty_run = sheet(2, 1, 8)
            .frame(|f| f.line().skip8(1).run8(0, [3; 4]).skip8(1))
            .build();
        assert!(!plain(&empty_run).clip_quirks());
    }

    #[test]
    fn a_token_draws_only_its_own_bytes() {
        // Byte 3, the pixel token's pad, stays unwritten.
        let bytes = sheet(2, 1, 8)
            .frame(|f| f.line().pixels8(&[1, 2, 3]))
            .build();
        assert_eq!(
            frames(&clipped(&bytes, cut(0, 0))),
            [vec![rgb555(0x0201), CLEAR]]
        );
    }

    #[test]
    fn a_pixel_half_written_by_a_run_is_not_solid() {
        let bytes = sheet(2, 1, 8)
            .frame(|f| f.line().pixels8(&[1, 2, 3]).run8(1, [9; 4]))
            .build();
        let sheet = clipped(&bytes, cut(0, 0));
        assert_eq!(frames(&sheet), [vec![rgb555(0x0201), rgb555(0x0903)]]);
        assert_eq!(solids(&sheet), Some(vec![vec![CLEAR, CLEAR]]));
    }

    #[test]
    fn tokens_after_a_run_draw_where_it_ends() {
        let bytes = sheet(4, 1, 16)
            .frame(|f| f.line().skip(1).run(2, RED, BLUE).pixels(&[GREEN]))
            .build();
        assert_eq!(
            frames(&clipped(&bytes, cut(0, 0))),
            [vec![CLEAR, rgb555(BLUE), rgb555(RED), rgb555(GREEN)]]
        );
    }
}
