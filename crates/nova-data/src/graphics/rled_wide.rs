//! An `rlëD` of depth 32, or of any depth but 8 and 16, as the original
//! draws it.
//!
//! The original loads an `rlëD` of any depth, and draws every depth but 8
//! with its 16-bit routines into its 16-bit, little-endian world. What a
//! sheet of another depth looks like is what its bytes do on the way
//! there. The steps below are ported byte for byte, on a copy of the
//! resource.
//!
//! # Loading and the swap (G1)
//!
//! `_FlipRle8` (@0xa5cd6) swaps the header's eight words, then calls
//! `_BlitPixieRLESwap(header, data + 16)` (@0xa5ce1-0xa5d1b) once, in
//! place; everything later reads the swapped memory.
//! `_SWCreateSpriteFromRLEResource` makes every frame at the header's depth
//! (@0xa99f3, @0xa9ac7) with no pixel buffer (`_SWCreateFrame` @0xad6f2),
//! and nothing on the way tests the depth.
//!
//! `_BlitPixieRLESwap` (@0xbe3a8-0xbe48e) byte-swaps each token in place
//! *before* testing its op (@0xbe410-0xbe41c), then dispatches on it
//! (jump table @0xdde90):
//!
//! - op > 4 stops the swap (@0xbe423-0xbe426): every later word stays
//!   big-endian;
//! - op 0 counts a frame down from the header's count, and the swap returns
//!   at 0 (@0xbe434-0xbe435);
//! - ops 1 and 3 step on 4 bytes (@0xbe430-0xbe432);
//! - op 4 swaps its run word too and steps on 8 (@0xbe440-0xbe44b);
//! - op 2 depends on the depth: at 16 it swaps `count / 2` u16s
//!   (@0xbe44d-0xbe46c), at 32 `count / 4` u32s (@0xbe46e-0xbe48e), leaving
//!   a part word at the end (`count % 4` bytes) to be read as the next
//!   token; at 8 it steps over `count` bytes unswapped (@0xbe3f0-0xbe3f7);
//!   at any other depth it does not step over the data at all
//!   (@0xbe3f0-0xbe401), so the pixel data is read, and swapped, as tokens.
//!   Each path then aligns the stream to 4 (@0xbe3fa-0xbe401).
//!
//! A swap count of 0 runs its loop about 2^32 times, a crash; that is
//! [`GraphicsError::RunawaySwap`]. A swap that reads past the sheet is
//! [`GraphicsError::UnexpectedEnd`].
//!
//! # Finding the frames (G2)
//!
//! `_SWLockSprite`'s walk (@0xaa9b6-0xaaa83) starts at `(data + 16) & ~3`
//! and walks the swapped words. A word of 0 ends a frame; any other must be
//! a line start (op 1), or it asserts (@0xaaa19-0xaaa2e,
//! [`GraphicsError::NotALineStart`]), and the walk skips its count + 4
//! bytes, which must stay within the sheet (@0xaaa45-0xaaa5b,
//! [`GraphicsError::LinePastEnd`]). The assert quits the game
//! (`_SWAssertFail` @0xa7a50). Each frame is drawn from where the walk
//! found it, so line starts must carry their lines' byte counts. A swap
//! that stopped early leaves a big-endian line start behind, whose op read
//! little-endian is its count's low byte, never 1: the walk asserts there.
//!
//! # Drawing (G3, G4)
//!
//! `_BlitPixieRLETranslucentDrawProc` (@0xbb3fc) asserts on a depth of 7 or
//! less, compared signed (@0xbb46e-0xbb476 → @0xbb612), so those depths are
//! [`GraphicsError::UnsupportedDepth`]. Every depth but 8 then takes the
//! 16-bit routines, the plain one `_BlitPixieRLE` (@0xbddd0-0xbde97) when
//! the frame's box lies wholly inside the world. It works in bytes and
//! checks no bounds: op 1 moves to the next row's start (@0xbde90-0xbde97;
//! tokens before the first line start draw on row 0), op 2 copies `count`
//! bytes and pads the stream to 4 (@0xbde6c-0xbde8e), op 3 steps `count`
//! bytes (@0xbde50-0xbde52), op 4 stores its run word with
//! `_BlitPixieMemSet` (@0xbde2a-0xbde45), and op 0 or op > 4 returns
//! (@0xbde0a).
//!
//! So a 32-bit file pixel `AA RR GG BB` lies in memory as `BB GG RR AA`
//! after the swap, and lands in the 16-bit world as two RGB555 pixels,
//! `0xGGBB` then `0xAARR` (top bit ignored): a row of W pixels is 2W world
//! pixels wide, running from the box's left edge W pixels past its right.
//! A run word `b0 b1 b2 b3` fills as at 16 bits, its low half `b2b3` on
//! even drawn pixels and its high half `b0b1` on odd ones. Line starts
//! beyond the header's height draw below the box.
//!
//! # The decoded sheet
//!
//! Each frame's bytes are drawn into little-endian 16-bit rows from the
//! box's top-left. A world pixel only one of whose bytes is written keeps
//! the background's other byte in the original; a straight-alpha texture
//! cannot replicate that, so it is drawn clear.
//!
//! The sheet keeps the header's width x height as its frames' box (what
//! is clicked, and what touches are measured by), and its images grow to
//! hold what is drawn: X world pixels wide, the written bytes halved and
//! rounded up, and R rows deep, each at least the box's. Every draw site
//! places a frame by its box's centre, while the original blits from the
//! box's top-left, so each image is padded to `2X - W` by `2R - H`, its
//! first `X - W` columns and `R - H` rows clear, putting the box at its
//! centre ([`SpriteSheet::box_origin`]). Drawn centred, it lands where the
//! original draws it. Its size is checked against the decoding budget as
//! it grows.
//!
//! A frame whose box touches the world's edge takes the clipped blit,
//! which cuts what is drawn to the box (G5, `_BlitPixieRLEClipped`
//! @0xbde9c-0xbe0b3); that is the view's to do, as it knows the world.

use super::GraphicsError;
use super::budget::check_budget;
use super::color::rgb555;
use super::image::Image;
use super::rled::{Header, PIXELS, RUN, SKIP, solid_frames};
use super::rled8::mem_set_byte;
use super::rled32::{self, ThirtyTwoBitReading};
use super::sheet::{SheetLayout, SpriteSheet};

/// Where the tokens start: just past the 16-byte header.
pub(super) const TOKENS: usize = 16;

/// The low 24 bits of a token: its byte count.
const COUNT_MASK: u32 = 0x00FF_FFFF;

/// The op of a frame end.
const FRAME_END: u32 = 0;
/// The op of a line start.
const LINE_START: u32 = 1;

/// The native (little-endian) word at `pos` of `mem`, or an
/// [`GraphicsError::UnexpectedEnd`] there.
fn word_at(mem: &[u8], pos: usize) -> Result<u32, GraphicsError> {
    match mem.get(pos..pos + 4) {
        Some(&[a, b, c, d]) => Ok(u32::from_le_bytes([a, b, c, d])),
        _ => Err(GraphicsError::UnexpectedEnd { offset: pos }),
    }
}

/// `pos` rounded up to a multiple of 4: the handle is aligned to 4, so an
/// offset into it aligns as its address does.
fn align4(pos: usize) -> usize {
    pos.next_multiple_of(4)
}

/// Reverses each `unit`-byte chunk of the `count` units at `pos` of `mem`
/// in place, or fails as a read past the sheet there.
fn swap_units(mem: &mut [u8], pos: usize, unit: usize, count: usize) -> Result<(), GraphicsError> {
    let Some(units) = mem.get_mut(pos..pos + unit * count) else {
        return Err(GraphicsError::UnexpectedEnd { offset: pos });
    };
    for chunk in units.chunks_exact_mut(unit) {
        chunk.reverse();
    }
    Ok(())
}

/// `_BlitPixieRLESwap` (@0xbe3a8-0xbe48e; see the module docs) over the
/// tokens of the whole resource `mem`, a sheet of `depth` and `count`
/// frames, in place.
pub(super) fn swap(mem: &mut [u8], depth: u16, count: u16) -> Result<(), GraphicsError> {
    let mut frames = count;
    let mut pos = TOKENS;
    // Each step moves on at least 4 bytes, so the data runs out first.
    loop {
        swap_units(mem, pos, 4, 1)?;
        let token = word_at(mem, pos)?;
        let n = (token & COUNT_MASK) as usize;
        let next = pos + 4;
        pos = match token >> 24 {
            FRAME_END => {
                frames = frames.saturating_sub(1);
                if frames == 0 {
                    return Ok(());
                }
                next
            }
            LINE_START | 3 => next,
            4 => {
                swap_units(mem, next, 4, 1)?;
                next + 4
            }
            2 => {
                let end = match depth {
                    16 | 32 => {
                        let unit = usize::from(depth / 8);
                        let units = n / unit;
                        if units == 0 {
                            return Err(GraphicsError::RunawaySwap { offset: pos });
                        }
                        swap_units(mem, next, unit, units)?;
                        next + unit * units
                    }
                    8 => next + n,
                    _ => next,
                };
                align4(end)
            }
            _ => return Ok(()),
        };
    }
}

/// `_SWLockSprite`'s walk (@0xaa9b6-0xaaa83; see the module docs) over the
/// swapped resource `mem`: where each of its `count` frames starts.
pub(super) fn frame_starts(mem: &[u8], count: u16) -> Result<Vec<usize>, GraphicsError> {
    let mut starts = Vec::with_capacity(usize::from(count));
    let mut pos = TOKENS;
    for frame in 0..u32::from(count) {
        starts.push(pos);
        // Each word moves on at least 4 bytes, so the data runs out first.
        loop {
            let word = word_at(mem, pos)?;
            if word == 0 {
                pos += 4;
                break;
            }
            if word >> 24 != LINE_START {
                return Err(GraphicsError::NotALineStart { frame, offset: pos });
            }
            let next = pos + 4 + (word & COUNT_MASK) as usize;
            if next > mem.len() {
                return Err(GraphicsError::LinePastEnd { frame, offset: pos });
            }
            pos = next;
        }
    }
    Ok(starts)
}

/// One byte drawn: its value, and whether a run wrote it.
#[derive(Clone, Copy, Debug)]
struct Cell {
    value: u8,
    run: bool,
}

/// A frame's drawn bytes, row by row from its box's top-left.
#[derive(Debug, Default)]
struct Drawn {
    rows: Vec<Vec<Option<Cell>>>,
    has_run: bool,
}

/// What a sheet's frames have drawn so far, and the budget it must keep.
struct Canvas<'h> {
    header: &'h Header,
    input_len: usize,
    /// The bytes the widest row reaches.
    bytes: u64,
    /// The rows drawn on.
    rows: u64,
}

impl Canvas<'_> {
    /// The images' size: X world pixels by R rows, each at least the box's.
    fn reach(&self) -> (u64, u64) {
        (
            self.bytes.div_ceil(2).max(u64::from(self.header.width)),
            self.rows.max(u64::from(self.header.height)),
        )
    }

    /// The padded images' width and height (see the module docs).
    fn image_size(&self) -> (u64, u64) {
        let (x, r) = self.reach();
        (
            2 * x - u64::from(self.header.width),
            2 * r - u64::from(self.header.height),
        )
    }

    /// Grows the canvas to hold `n` bytes from `x` on row `y`, checking
    /// the padded sheet against the budget.
    fn reach_to(&mut self, x: u64, y: u64, n: u64) -> Result<(), GraphicsError> {
        let (bytes, rows) = (self.bytes.max(x + n), self.rows.max(y + 1));
        if (bytes, rows) == (self.bytes, self.rows) {
            return Ok(());
        }
        (self.bytes, self.rows) = (bytes, rows);
        let (width, height) = self.image_size();
        let pixels = width
            .saturating_mul(height)
            .saturating_mul(u64::from(self.header.count));
        check_budget(pixels, self.input_len)
    }

    /// Writes `n` bytes, the `i`th `byte(i)`, from `x` on row `y` of
    /// `frame`.
    fn write(
        &mut self,
        frame: &mut Drawn,
        (x, y): (u64, u64),
        n: u32,
        run: bool,
        byte: impl Fn(u32) -> u8,
    ) -> Result<(), GraphicsError> {
        if n == 0 {
            return Ok(());
        }
        self.reach_to(x, y, u64::from(n))?;
        // Within the budget, so these fit memory.
        let (x, y) = (x as usize, y as usize);
        if frame.rows.len() <= y {
            frame.rows.resize_with(y + 1, Vec::new);
        }
        let row = &mut frame.rows[y];
        let end = x + n as usize;
        if row.len() < end {
            row.resize(end, None);
        }
        for (i, slot) in (0..n).zip(&mut row[x..end]) {
            *slot = Some(Cell {
                value: byte(i),
                run,
            });
        }
        Ok(())
    }

    /// `_BlitPixieRLE` (@0xbddd0-0xbde97; see the module docs) of the
    /// frame whose tokens start at `start` of the swapped resource `mem`.
    fn draw(&mut self, mem: &[u8], start: usize) -> Result<Drawn, GraphicsError> {
        let mut frame = Drawn::default();
        let (mut x, mut y, mut next_row) = (0u64, 0u64, 0u64);
        let mut pos = start;
        // Each token moves on at least 4 bytes, so the data runs out first.
        loop {
            let token = word_at(mem, pos)?;
            let n = token & COUNT_MASK;
            let data = pos + 4;
            match (token >> 24) as u8 {
                1 => {
                    (x, y) = (0, next_row);
                    next_row += 1;
                    pos = data;
                }
                PIXELS => {
                    let Some(bytes) = mem.get(data..data + n as usize) else {
                        return Err(GraphicsError::UnexpectedEnd { offset: data });
                    };
                    self.write(&mut frame, (x, y), n, false, |i| bytes[i as usize])?;
                    pos = align4(data + n as usize);
                    x += u64::from(n);
                }
                SKIP => {
                    pos = data;
                    x += u64::from(n);
                }
                RUN => {
                    let word = word_at(mem, data)?;
                    frame.has_run = true;
                    self.write(&mut frame, (x, y), n, true, |i| mem_set_byte(word, n, i))?;
                    pos = data + 4;
                    x += u64::from(n);
                }
                _ => return Ok(frame),
            }
        }
    }

    /// The pixels of `frame` from its box's top-left, as `true_colour`
    /// says: each with its place, its colour, and whether runs alone
    /// wrote it.
    ///
    /// As drawn, each two bytes are a little-endian RGB555 pixel; in true
    /// colour ([`super::rled32`]), each four within the box are one pixel,
    /// and what lies past the box is dropped. A pixel with a byte unwritten
    /// is clear either way.
    fn pixels(&self, frame: &Drawn, true_colour: bool) -> Vec<((u32, u32), [u8; 4], bool)> {
        let (unit, columns, rows) = if true_colour {
            (4, self.header.width as usize, self.header.height as usize)
        } else {
            (2, usize::MAX, usize::MAX)
        };
        let mut pixels = Vec::new();
        for (y, row) in (0..).zip(frame.rows.iter().take(rows)) {
            for (x, cells) in (0..).zip(row.chunks(unit).take(columns)) {
                let (colour, run) = match *cells {
                    [Some(b0), Some(b1), Some(b2), Some(b3)] if true_colour => (
                        rled32::true_colour([b0.value, b1.value, b2.value, b3.value]),
                        b0.run && b1.run && b2.run && b3.run,
                    ),
                    [Some(low), Some(high)] if !true_colour => (
                        rgb555(u16::from_le_bytes([low.value, high.value])),
                        low.run && high.run,
                    ),
                    // A byte unwritten, or a part pixel at the row's end.
                    _ => continue,
                };
                pixels.push(((x, y), colour, run));
            }
        }
        pixels
    }

    /// The frames drawn, as a sheet of images padded about the box, read
    /// in `true_colour` or not as [`Self::pixels`] says.
    fn finish(&self, drawn: Vec<Drawn>, layout: SheetLayout, true_colour: bool) -> SpriteSheet {
        let (width, height) = (self.header.width, self.header.height);
        let (reach_x, reach_y) = self.reach();
        // Within the budget, so these fit a u32.
        let (image_width, image_height) = {
            let (w, h) = self.image_size();
            (w as u32, h as u32)
        };
        let origin = (
            (reach_x - u64::from(width)) as u32,
            (reach_y - u64::from(height)) as u32,
        );
        let mut frames = Vec::with_capacity(drawn.len());
        let mut solids = Vec::with_capacity(drawn.len());
        for frame in drawn {
            let mut image = Image::transparent(image_width, image_height);
            let mut solid = frame
                .has_run
                .then(|| Image::transparent(image_width, image_height));
            for ((x, y), colour, run) in self.pixels(&frame, true_colour) {
                let at = (origin.0 + x, origin.1 + y);
                image.put(at.0, at.1, colour);
                if let (Some(solid), true) = (solid.as_mut(), run) {
                    solid.put(at.0, at.1, colour);
                }
            }
            frames.push(image);
            solids.push(solid);
        }
        let solid = solid_frames(solids, image_width, image_height);
        SpriteSheet::overhanging(
            (width, height),
            (image_width, image_height),
            origin,
            frames,
            layout,
        )
        .with_solid(solid)
    }
}

/// The `rlëD` `data`, whose checked header is `header`, as the original
/// holds it once loaded and locked: swapped in memory, and where each
/// frame starts in it.
pub(super) fn locked(data: &[u8], header: &Header) -> Result<(Vec<u8>, Vec<usize>), GraphicsError> {
    let mut mem = data.to_vec();
    swap(&mut mem, header.depth, header.count)?;
    let starts = frame_starts(&mem, header.count)?;
    Ok((mem, starts))
}

/// Decodes the `rlëD` `data`, whose checked header is `header` and whose
/// depth is neither 8 nor 16, as the original draws it (see the module
/// docs), or, when it is 32-bit and `reading` says so, its drawn bytes
/// read back in true colour ([`super::rled32`]): a sheet loads or fails,
/// and keeps its sizes, the same under either reading.
pub(super) fn decode_wide(
    data: &[u8],
    header: &Header,
    layout: SheetLayout,
    reading: ThirtyTwoBitReading,
) -> Result<SpriteSheet, GraphicsError> {
    let (mem, starts) = locked(data, header)?;
    let mut canvas = Canvas {
        header,
        input_len: data.len(),
        bytes: 0,
        rows: 0,
    };
    let drawn = starts
        .into_iter()
        .map(|start| canvas.draw(&mem, start))
        .collect::<Result<Vec<_>, _>>()?;
    let true_colour = header.depth == 32 && reading == ThirtyTwoBitReading::TrueColour;
    Ok(canvas.finish(drawn, layout, true_colour))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::fixture::{RledBuilder, RledFrame};
    use crate::graphics::rled::{decode_rled, scan_rled};

    const CLEAR: [u8; 4] = [0; 4];

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

    fn decode(data: &[u8]) -> Result<SpriteSheet, GraphicsError> {
        decode_rled(data, None)
    }

    #[test]
    fn a_32_bit_sheet_decodes_as_the_16_bit_world_draws_it() {
        // File pixels 00 12 34 56 and 80 AB CD EF.
        let bytes = sheet(2, 1, 32)
            .frame(|f| f.line().pixels32(&[0x0012_3456, 0x80AB_CDEF]))
            .build();
        let sheet = decode(&bytes).unwrap();
        assert_eq!((sheet.frame_width(), sheet.frame_height()), (2, 1));
        assert_eq!((sheet.image_width(), sheet.image_height()), (6, 1));
        assert_eq!(sheet.box_origin(), (2, 0));
        assert_eq!(
            frames(&sheet),
            [vec![
                CLEAR,
                CLEAR,
                rgb555(0x3456),
                rgb555(0x0012),
                rgb555(0xCDEF),
                rgb555(0x80AB),
            ]]
        );
        assert_eq!(sheet.solid_frames(), None);
    }

    #[test]
    fn a_32_bit_run_fills_low_half_then_high_half() {
        let bytes = sheet(2, 1, 32)
            .frame(|f| f.line().run32(2, 0xA1B2_C3D4))
            .build();
        let sheet = decode(&bytes).unwrap();
        let (low, high) = (rgb555(0xC3D4), rgb555(0xA1B2));
        let drawn = vec![CLEAR, CLEAR, low, high, low, high];
        assert_eq!(solids(&sheet), Some(vec![drawn.clone()]));
        assert_eq!(frames(&sheet), [drawn]);
    }

    #[test]
    fn a_32_bit_rows_tokens_draw_one_after_another() {
        // A skip of two pixels (8 bytes), then two pixel tokens.
        let bytes = sheet(4, 1, 32)
            .frame(|f| {
                f.line()
                    .skip32(2)
                    .pixels32(&[0x0000_7C00])
                    .pixels32(&[0x0000_001F])
            })
            .build();
        let sheet = decode(&bytes).unwrap();
        assert_eq!((sheet.image_width(), sheet.box_origin()), (12, (4, 0)));
        let (red, blue, black) = (rgb555(0x7C00), rgb555(0x001F), rgb555(0));
        let mut drawn = vec![CLEAR; 8];
        drawn.extend([red, black, blue, black]);
        assert_eq!(frames(&sheet), [drawn]);
    }

    #[test]
    fn a_world_pixel_is_solid_only_when_runs_wrote_both_its_bytes() {
        // Bytes 0..3 from a run, 3..7 (66 55 44 00 once swapped) from a
        // pixel token: the world pixel of bytes 2 and 3 is half a run's.
        let bytes = sheet(1, 1, 32)
            .frame(|f| f.line().run8(3, [0x21; 4]).pixels32(&[0x0044_5566]))
            .build();
        let sheet = decode(&bytes).unwrap();
        assert_eq!((sheet.image_width(), sheet.box_origin()), (7, (3, 0)));
        let run = rgb555(0x2121);
        let mut drawn = vec![CLEAR; 3];
        drawn.extend([run, rgb555(0x6621), rgb555(0x4455), CLEAR]);
        let mut solid = vec![CLEAR; 3];
        solid.extend([run, CLEAR, CLEAR, CLEAR]);
        assert_eq!(solids(&sheet), Some(vec![solid]));
        assert_eq!(frames(&sheet), [drawn]);
    }

    #[test]
    fn a_32_bit_pixel_tail_draws_as_the_swap_left_it() {
        // The swap swaps one word, then reads the tail word 03 44 00 00
        // as a skip and swaps it to 00 00 44 03.
        let bytes = sheet(2, 1, 32)
            .frame(|f| {
                f.line()
                    .token(2, 6)
                    .raw(&[0x00, 0x11, 0x22, 0x33, 0x03, 0x44, 0x00, 0x00])
            })
            .build();
        let sheet = decode(&bytes).unwrap();
        assert_eq!((sheet.image_width(), sheet.box_origin()), (4, (1, 0)));
        assert_eq!(
            frames(&sheet),
            [vec![CLEAR, rgb555(0x2233), rgb555(0x0011), rgb555(0x0000)]]
        );
        assert_eq!(frames(&sheet)[0][3], [0, 0, 0, 255]);
    }

    #[test]
    fn a_32_bit_pixel_token_under_four_bytes_is_an_error() {
        for count in 0..4 {
            let bytes = sheet(2, 1, 32)
                .frame(|f| f.line().token(2, count).raw(&[1, 2, 3, 0]))
                .build();
            assert_eq!(
                decode(&bytes),
                Err(GraphicsError::RunawaySwap { offset: 20 }),
                "{count}"
            );
        }
    }

    #[test]
    fn line_starts_past_the_height_draw_below_the_box() {
        let bytes = sheet(1, 1, 32)
            .frame(|f| {
                f.line()
                    .pixels32(&[0x0000_7C00])
                    .line()
                    .pixels32(&[0x0000_001F])
            })
            .build();
        let sheet = decode(&bytes).unwrap();
        assert_eq!((sheet.frame_width(), sheet.frame_height()), (1, 1));
        assert_eq!((sheet.image_width(), sheet.image_height()), (3, 3));
        assert_eq!(sheet.box_origin(), (1, 1));
        let (red, blue, black) = (rgb555(0x7C00), rgb555(0x001F), rgb555(0));
        assert_eq!(
            frames(&sheet),
            [vec![
                CLEAR, CLEAR, CLEAR, //
                CLEAR, red, black, //
                CLEAR, blue, black,
            ]]
        );
    }

    #[test]
    fn each_frame_draws_from_where_the_lock_walk_finds_it() {
        // Frame 0's line count runs over its first frame end, so the walk
        // starts frame 1 after the second; the swap counts both ends.
        let bytes = RledBuilder::new(1, 1)
            .depth(32)
            .frame(|f| f.token(1, 12).pixels32(&[0x0000_7C00]).token(0, 0))
            .frame(|f| f.token(1, 8).pixels32(&[0x0000_001F]))
            .frame(|f| f)
            .build();
        let sheet = decode(&bytes).unwrap();
        assert_eq!(
            frames(&sheet),
            [
                vec![CLEAR, rgb555(0x7C00), rgb555(0)],
                vec![CLEAR, rgb555(0x001F), rgb555(0)],
                vec![CLEAR; 3],
            ]
        );
    }

    #[test]
    fn every_frame_shares_the_widest_frames_image() {
        let bytes = sheet(1, 1, 32)
            .frame(|f| f.line().pixels32(&[0x0000_7C00]))
            .frame(RledFrame::line)
            .build();
        let sheet = decode(&bytes).unwrap();
        for frame in sheet.frames() {
            assert_eq!((frame.width(), frame.height()), (3, 1));
        }
        assert_eq!(rgba(&sheet.frames()[1]), [CLEAR; 3]);
    }

    #[test]
    fn a_sheet_that_draws_within_its_box_is_not_padded() {
        let bytes = sheet(4, 1, 32)
            .frame(|f| f.line().pixels32(&[0x0000_7C00]))
            .build();
        let sheet = decode(&bytes).unwrap();
        assert_eq!((sheet.image_width(), sheet.image_height()), (4, 1));
        assert_eq!(sheet.box_origin(), (0, 0));
        assert_eq!(
            frames(&sheet),
            [vec![rgb555(0x7C00), rgb555(0), CLEAR, CLEAR]]
        );
    }

    #[test]
    fn another_depth_without_pixel_tokens_draws_in_sync() {
        // Bytes 3..9 of the row: D4 C3 B2 A1, then the 2-byte tail D4 C3.
        let bytes = sheet(5, 1, 24)
            .frame(|f| f.line().skip8(3).run8(6, [0xA1, 0xB2, 0xC3, 0xD4]))
            .build();
        let sheet = decode(&bytes).unwrap();
        assert_eq!((sheet.image_width(), sheet.box_origin()), (5, (0, 0)));
        let drawn = vec![CLEAR, CLEAR, rgb555(0xB2C3), rgb555(0xD4A1), CLEAR];
        assert_eq!(solids(&sheet), Some(vec![drawn.clone()]));
        assert_eq!(frames(&sheet), [drawn]);
    }

    #[test]
    fn another_depths_pixel_data_is_swapped_as_the_token_it_reads_as() {
        // The swap reads 03 00 00 00 as a skip and swaps it in place.
        let bytes = sheet(2, 1, 24)
            .frame(|f| f.line().pixels8(&[0x03, 0x00, 0x00, 0x00]))
            .build();
        assert_eq!(
            frames(&decode(&bytes).unwrap()),
            [vec![rgb555(0x0000), rgb555(0x0300)]]
        );
    }

    #[test]
    fn another_depth_whose_swap_stops_fails_the_lock_walk() {
        // The data word 7F 00 00 00 swaps to op 0x7F, which stops the
        // swap; the second line start stays big-endian.
        let bytes = sheet(2, 2, 24)
            .frame(|f| f.line().pixels8(&[0x7F, 0, 0, 0]).line().skip8(4))
            .build();
        assert_eq!(
            decode(&bytes),
            Err(GraphicsError::NotALineStart {
                frame: 0,
                offset: 28
            })
        );
    }

    #[test]
    fn a_line_start_past_the_end_fails_the_lock_walk() {
        let bytes = RledBuilder::new(1, 1)
            .depth(32)
            .frame(|f| f.token(1, 16).pixels32(&[0]))
            .build();
        assert_eq!(
            decode(&bytes),
            Err(GraphicsError::LinePastEnd {
                frame: 0,
                offset: 16
            })
        );
        // Reaching the end exactly passes the check, but the frame end is
        // then missing.
        let bytes = RledBuilder::new(1, 1)
            .depth(32)
            .frame(|f| f.token(1, 12).pixels32(&[0]))
            .build();
        assert_eq!(
            decode(&bytes),
            Err(GraphicsError::UnexpectedEnd { offset: 32 })
        );
    }

    #[test]
    fn a_depth_of_7_or_less_or_0x8000_and_up_is_unsupported() {
        for depth in [0, 1, 7, 0x8000, 0xFFFF] {
            let bytes = sheet(2, 1, depth).frame(RledFrame::line).build();
            assert_eq!(
                decode(&bytes),
                Err(GraphicsError::UnsupportedDepth { depth }),
                "{depth}"
            );
            assert_eq!(
                scan_rled(&bytes),
                Err(GraphicsError::UnsupportedDepth { depth }),
                "{depth}"
            );
        }
        for depth in [9, 15, 17, 24, 31, 33, 0x7FFF] {
            let bytes = sheet(2, 1, depth).frame(RledFrame::line).build();
            assert!(decode(&bytes).is_ok(), "{depth}");
        }
    }

    #[test]
    fn a_runaway_count_is_too_large() {
        let bytes = sheet(1, 1, 32)
            .frame(|f| f.line().token(4, 0xFF_FFFF).raw(&[0; 4]))
            .build();
        assert!(
            matches!(decode(&bytes), Err(GraphicsError::TooLarge { .. })),
            "{:?}",
            decode(&bytes)
        );
        // A skip that far draws nothing, and costs nothing.
        let bytes = sheet(1, 1, 32)
            .frame(|f| f.line().token(3, 0xFF_FFFF))
            .build();
        assert_eq!(decode(&bytes).map(|s| s.image_width()), Ok(1));
    }

    #[test]
    fn a_scan_reads_what_the_decode_would_hold() {
        let bytes = sheet(2, 1, 32)
            .frame(|f| f.line().run32(1, 0x7FFF_7FFF))
            .frame(RledFrame::line)
            .build();
        let info = scan_rled(&bytes).unwrap();
        assert_eq!(
            (
                info.frame_width,
                info.frame_height,
                info.frames,
                info.has_runs
            ),
            (2, 1, 2, true)
        );
        let bytes = sheet(2, 1, 24).frame(|f| f.line().skip8(2)).build();
        assert!(!scan_rled(&bytes).unwrap().has_runs);
        let broken = sheet(2, 1, 32)
            .frame(|f| f.line().token(2, 2).raw(&[0; 4]))
            .build();
        assert_eq!(scan_rled(&broken).map(|_| ()), decode(&broken).map(|_| ()));
    }

    #[test]
    fn the_swap_and_the_walk_name_where_they_run_out() {
        let mut mem = sheet(1, 1, 32).frame(|f| f.line().run32(1, 0)).build();
        mem.truncate(mem.len() - 8);
        assert_eq!(
            swap(&mut mem, 32, 1),
            Err(GraphicsError::UnexpectedEnd { offset: 24 })
        );
        let mut mem = sheet(1, 1, 32)
            .frame(|f| f.line().pixels32(&[1, 2]))
            .build();
        mem.truncate(28);
        assert_eq!(
            swap(&mut mem, 32, 1),
            Err(GraphicsError::UnexpectedEnd { offset: 24 })
        );
    }

    /// At depth 16 the swap's op 2 swaps `count / 2` u16s (@0xbe44d-0xbe46c),
    /// steps on to the next 4-byte boundary, and swaps the token there.
    #[test]
    fn the_swap_at_depth_16_swaps_u16s_then_realigns() {
        let mut mem = sheet(2, 1, 16)
            .frame(|f| {
                f.line()
                    .token(2, 4)
                    .raw(&[0x12, 0x34, 0x56, 0x78])
                    .skip(1)
                    .token(2, 3)
                    .raw(&[0x9A, 0xBC, 0xDE, 0xF0])
                    .skip(1)
            })
            .build();
        assert_eq!(swap(&mut mem, 16, 1), Ok(()));
        assert_eq!(
            mem[TOKENS..],
            [
                24, 0, 0, 1, // the line start
                4, 0, 0, 2, 0x34, 0x12, 0x78, 0x56, // two u16s
                2, 0, 0, 3, // the skip after them
                3, 0, 0, 2, 0xBC, 0x9A, 0xDE, 0xF0, // one u16; the odd byte as it was
                2, 0, 0, 3, // the skip after the boundary
                0, 0, 0, 0, // the frame's end
            ]
        );
    }

    /// At depth 16 a pixel token of no bytes, or of one, swaps no u16s,
    /// which the original's loop counts down from 0 (@0xbe44d-0xbe46c): a
    /// runaway.
    #[test]
    fn the_swap_at_depth_16_runs_away_on_a_pixel_token_under_2_bytes() {
        for count in [0, 1] {
            let mut mem = sheet(2, 1, 16)
                .frame(|f| f.line().token(2, count).raw(&[0; 4]))
                .build();
            assert_eq!(
                swap(&mut mem, 16, 1),
                Err(GraphicsError::RunawaySwap { offset: 20 }),
                "{count} bytes"
            );
        }
    }

    /// At depth 8 the swap's op 2 steps over `count` bytes unswapped
    /// (@0xbe3f0-0xbe3f7), then on to the next 4-byte boundary, and swaps
    /// the token there.
    #[test]
    fn the_swap_at_depth_8_steps_over_the_bytes_unswapped_then_realigns() {
        let mut mem = sheet(3, 1, 8)
            .frame(|f| f.line().token(2, 3).raw(&[0x12, 0x34, 0x56, 0x78]).skip8(2))
            .build();
        assert_eq!(swap(&mut mem, 8, 1), Ok(()));
        assert_eq!(
            mem[TOKENS..],
            [
                12, 0, 0, 1, // the line start
                3, 0, 0, 2, 0x12, 0x34, 0x56, 0x78, // the bytes as they were
                2, 0, 0, 3, // the skip after the boundary
                0, 0, 0, 0, // the frame's end
            ]
        );
    }

    #[test]
    fn corrupt_wide_sheets_never_panic() {
        let bytes = sheet(2, 3, 32)
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
        assert!(bytes.len() < 1024);
        crate::sweep::assert_never_panics(&bytes, decode);
        let bytes = sheet(2, 2, 24)
            .frame(|f| f.line().skip8(3).run8(2, [1, 2, 3, 4]).line())
            .build();
        crate::sweep::assert_never_panics(&bytes, decode);
    }
}
