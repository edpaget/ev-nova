//! An `rlëD` whose header says 8 bits, as the original draws it.
//!
//! The original runs a 16-bit world on every machine it supports, and it
//! never turns an 8-bit sheet's bytes into colours: it copies them.
//!
//! - **Loading.** `_SWCreateSpriteFromRLEResource` (@0xa9990) reads only
//!   the header and makes each frame at the header's depth with no pixel
//!   buffer (`_SWCreateFrame` @0xad556, @0xad62c → @0xad6f2). No `clut` or
//!   palette is read anywhere on the path.
//! - **Byte order.** `_BlitPixieRLESwap` (@0xbe3a8) byte-swaps each token
//!   and run word to little-endian (@0xbe440), and leaves 8-bit pixel
//!   bytes as they are (@0xbe3f0-0xbe3f7).
//! - **Drawing.** A stellar's draw proc, `_BlitPixieRLETranslucentDrawProc`
//!   (@0xbb3fc), calls plain `_BlitPixieRLE` (@0xbddd0) for a depth-8 frame
//!   drawn opaque (@0xbb792 → @0xbb8b6 → @0xbb9fb → @0xbb9c3), addressing
//!   the destination with the 16-bit world's shift (@0xbb53c-0xbb555).
//!   `_BlitPixieRLE` works in bytes: a line start moves down a row, op 2
//!   copies *count* bytes and pads the stream to 4, op 3 skips *count*
//!   bytes, and op 4 stores its swapped run word with `_BlitPixieMemSet`
//!   (@0xc0452-0xc049a): 4-byte chunks from the run's first byte, then a
//!   2-byte tail of the word's low half, then a 1-byte tail of its low byte.
//!
//! So row *y*'s bytes land from the frame's left edge in a little-endian
//! 16-bit row: pixel *k* is `byte[2k] | byte[2k + 1] << 8`, read as
//! RGB555 with its top bit ignored. The picture fills the left half of its
//! frame box, and each pair of colour indexes becomes one meaningless
//! colour. [`EightBitReading::AsDrawn`] decodes that, and is the default.
//!
//! A 16-bit pixel only one of whose bytes is written keeps the background's
//! other byte in the original. A straight-alpha texture cannot replicate
//! that, so such a pixel is drawn clear.
//!
//! [`EightBitReading::SystemPalette`] reads what an 8-bit sheet means
//! instead: each byte is one pixel, a colour of the Mac's standard
//! 256-colour table, at full width. A run fills its bytes with its word's
//! first byte in the file.
//!
//! The rulebook's `eight_bit_sheet` key chooses between them. The full
//! address record, with what was left out (an 8-bit sheet drawn
//! translucent, the legacy 8-bit world), is in `nova_sim_story`'s
//! "Checked for stellar-fidelity phase 4".

use super::color::rgb555;
use super::image::Image;
use super::rled::{Sink, solid_frames};
use super::sheet::{SheetLayout, SpriteSheet};
use super::system_clut::system_colour;

/// How an `rlëD` whose header says 8 bits is turned into pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EightBitReading {
    /// As the original's 16-bit world draws it: each pair of bytes is one
    /// little-endian RGB555 pixel, filling the left half of the frame.
    #[default]
    AsDrawn,
    /// Each byte is one pixel, a colour of the Mac's standard 256-colour
    /// table (`clut` 8), at full width.
    SystemPalette,
}

/// One byte of a row: its value, and whether a run wrote it.
#[derive(Clone, Copy, Debug)]
struct Cell {
    value: u8,
    run: bool,
}

/// An 8-bit sheet's frames, painted byte by byte and turned into pixels at
/// each frame's end.
pub(super) struct Rled8 {
    reading: EightBitReading,
    width: u32,
    height: u32,
    /// Bytes a row holds: two per pixel as drawn, one through the palette.
    row_len: u32,
    /// The frame's bytes, `row_len` per row.
    cells: Vec<Option<Cell>>,
    /// Whether the frame has a run.
    has_run: bool,
    frames: Vec<Image>,
    solids: Vec<Option<Image>>,
}

impl Rled8 {
    /// An empty sheet of `count` frames, `width` x `height` each.
    pub(super) fn new(reading: EightBitReading, width: u32, height: u32, count: u16) -> Self {
        let row_len = match reading {
            EightBitReading::AsDrawn => 2 * width,
            EightBitReading::SystemPalette => width,
        };
        Self {
            reading,
            width,
            height,
            row_len,
            cells: vec![None; row_len as usize * height as usize],
            has_run: false,
            frames: Vec::with_capacity(usize::from(count)),
            solids: Vec::with_capacity(usize::from(count)),
        }
    }

    /// The finished sheet.
    pub(super) fn finish(self, layout: SheetLayout) -> SpriteSheet {
        let solid = solid_frames(self.solids, self.width, self.height);
        SpriteSheet::new(self.width, self.height, self.frames, layout).with_solid(solid)
    }

    fn put(&mut self, x: u32, y: u32, cell: Cell) {
        let index = y as usize * self.row_len as usize + x as usize;
        if let Some(slot) = self.cells.get_mut(index) {
            *slot = Some(cell);
        }
    }

    /// The pixel at (`x`, `y`) and whether runs wrote all of it, or `None`
    /// where it is clear.
    fn pixel(&self, x: u32, y: u32) -> Option<([u8; 4], bool)> {
        let row = y as usize * self.row_len as usize;
        let cell = |i: u32| self.cells.get(row + i as usize).copied().flatten();
        match self.reading {
            EightBitReading::AsDrawn => {
                let (low, high) = (cell(2 * x)?, cell(2 * x + 1)?);
                let value = u16::from_le_bytes([low.value, high.value]);
                Some((rgb555(value), low.run && high.run))
            }
            EightBitReading::SystemPalette => {
                let cell = cell(x)?;
                Some((system_colour(cell.value), cell.run))
            }
        }
    }
}

/// The byte `_BlitPixieMemSet` stores at `i` of an `n`-byte run of `word`
/// (as read big-endian from the file): the little-endian word repeated in
/// 4-byte chunks, then a 2-byte tail of its low half and a 1-byte tail of
/// its low byte.
///
/// So byte `i` is the word's byte `i % 4`, but for the 1-byte tail after a
/// 2-byte one: the run's last byte when `i % 4` is 2, which is the low
/// byte again. (Tails start on a multiple of 4, so every other tail byte
/// already has `i % 4` equal to `i % 2`.)
fn mem_set_byte(word: u32, n: u32, i: u32) -> u8 {
    let le = word.to_le_bytes();
    let one_byte_tail = i % 4 == 2 && i + 1 == n;
    let k = if one_byte_tail { 0 } else { i % 4 };
    le[k as usize]
}

impl Sink for Rled8 {
    fn pixels(&mut self, x: u32, y: u32, pixels: &[u8]) {
        for (i, &value) in (0..).zip(pixels) {
            self.put(x + i, y, Cell { value, run: false });
        }
    }

    fn run(&mut self, x: u32, y: u32, n: u32, word: u32) {
        self.has_run = true;
        for i in 0..n {
            let value = match self.reading {
                EightBitReading::AsDrawn => mem_set_byte(word, n, i),
                EightBitReading::SystemPalette => (word >> 24) as u8,
            };
            self.put(x + i, y, Cell { value, run: true });
        }
    }

    fn frame_end(&mut self) {
        let mut frame = Image::transparent(self.width, self.height);
        let (width, height) = (self.width, self.height);
        // The next frame starts with no run.
        let mut solid =
            std::mem::take(&mut self.has_run).then(|| Image::transparent(width, height));
        for y in 0..self.height {
            for x in 0..self.width {
                let Some((colour, run)) = self.pixel(x, y) else {
                    continue;
                };
                frame.put(x, y, colour);
                if let (Some(solid), true) = (solid.as_mut(), run) {
                    solid.put(x, y, colour);
                }
            }
        }
        self.frames.push(frame);
        self.solids.push(solid);
        self.cells.fill(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::GraphicsError;
    use crate::graphics::fixture::{RledBuilder, RledFrame};
    use crate::graphics::rled::decode_rled_as;

    const CLEAR: [u8; 4] = [0; 4];

    fn rgba(image: &Image) -> Vec<[u8; 4]> {
        image
            .pixels()
            .chunks(4)
            .map(|p| p.try_into().unwrap())
            .collect()
    }

    /// A one-frame 8-bit sheet of `width` x 1 built by `f`.
    fn one_frame(width: u16, f: impl FnOnce(RledFrame) -> RledFrame) -> Vec<u8> {
        RledBuilder::new(width, 1)
            .depth(8)
            .frame(|frame| f(frame.line()))
            .build()
    }

    fn as_drawn(data: &[u8]) -> Result<SpriteSheet, GraphicsError> {
        decode_rled_as(data, None, EightBitReading::AsDrawn)
    }

    fn palette(data: &[u8]) -> Result<SpriteSheet, GraphicsError> {
        decode_rled_as(data, None, EightBitReading::SystemPalette)
    }

    /// The first frame's pixels.
    fn first(sheet: &SpriteSheet) -> Vec<[u8; 4]> {
        rgba(&sheet.frames()[0])
    }

    #[test]
    fn the_original_reading_is_the_default() {
        assert_eq!(EightBitReading::default(), EightBitReading::AsDrawn);
    }

    #[test]
    fn an_8_bit_sheet_decodes_as_the_16_bit_world_draws_it() {
        let sheet = as_drawn(&one_frame(4, |f| f.pixels8(&[0x12, 0x34, 0x56]))).unwrap();
        assert_eq!((sheet.frame_width(), sheet.frame_height()), (4, 1));
        assert_eq!(sheet.frames().len(), 1);
        // Byte 2 alone writes half of pixel 1, which stays clear.
        assert_eq!(first(&sheet), [rgb555(0x3412), CLEAR, CLEAR, CLEAR]);
        assert_eq!(sheet.solid_frames(), None);
    }

    #[test]
    fn an_8_bit_run_lays_its_word_bytes_down_reversed() {
        let bytes = one_frame(2, |f| f.run8(4, [0xA1, 0xB2, 0xC3, 0xD4]));
        let sheet = as_drawn(&bytes).unwrap();
        let pixels = [rgb555(0xC3D4), rgb555(0xA1B2)];
        assert_eq!(first(&sheet), pixels);
        assert_eq!(rgba(&sheet.solid_frames().unwrap()[0]), pixels);
    }

    #[test]
    fn an_8_bit_runs_tails_repeat_the_low_half_then_the_low_byte() {
        // 7 bytes: D4 C3 B2 A1, then the 2-byte tail D4 C3 and the 1-byte
        // tail D4; one copied byte completes pixel 3.
        let bytes = one_frame(4, |f| f.run8(7, [0xA1, 0xB2, 0xC3, 0xD4]).pixels8(&[0x7F]));
        let sheet = as_drawn(&bytes).unwrap();
        let (low, high) = (rgb555(0xC3D4), rgb555(0xA1B2));
        assert_eq!(first(&sheet), [low, high, low, rgb555(0x7FD4)]);
        // Pixel 3 was half copied, so it is not a solid run pixel.
        assert_eq!(
            rgba(&sheet.solid_frames().unwrap()[0]),
            [low, high, low, CLEAR]
        );
        // 3 bytes: the 2-byte tail, then the 1-byte tail.
        let bytes = one_frame(2, |f| f.run8(3, [0xA1, 0xB2, 0xC3, 0xD4]).pixels8(&[0x01]));
        assert_eq!(
            first(&as_drawn(&bytes).unwrap()),
            [rgb555(0xC3D4), rgb555(0x01D4)]
        );
        // 5 bytes: one chunk, then the 1-byte tail.
        let bytes = one_frame(3, |f| f.run8(5, [0xA1, 0xB2, 0xC3, 0xD4]).pixels8(&[0x02]));
        assert_eq!(
            first(&as_drawn(&bytes).unwrap()),
            [rgb555(0xC3D4), rgb555(0xA1B2), rgb555(0x02D4)]
        );
    }

    #[test]
    fn an_8_bit_skip_shifts_the_bytes_that_follow() {
        // Skip 1, then 2 bytes: each pixel gets one byte and stays clear.
        let bytes = one_frame(2, |f| f.skip8(1).pixels8(&[0x11, 0x22]));
        assert_eq!(first(&as_drawn(&bytes).unwrap()), [CLEAR, CLEAR]);
        // Skip 1, then 3 bytes: pixel 1 is whole.
        let bytes = one_frame(2, |f| f.skip8(1).pixels8(&[0x11, 0x22, 0x33]));
        assert_eq!(first(&as_drawn(&bytes).unwrap()), [CLEAR, rgb555(0x3322)]);
    }

    #[test]
    fn an_8_bit_pixel_run_is_padded_to_four_bytes() {
        for n in 1..=5u8 {
            let data: Vec<u8> = (1..=n).collect();
            // The bytes then 0xCD, which lands on byte `n`.
            let bytes = one_frame(4, |f| f.pixels8(&data).pixels8(&[0xCD]));
            let mut cells: Vec<u8> = data.clone();
            cells.push(0xCD);
            let mut expected: Vec<[u8; 4]> = cells
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&pair| rgb555(u16::from_le_bytes(pair)))
                .collect();
            expected.resize(4, CLEAR);
            assert_eq!(first(&as_drawn(&bytes).unwrap()), expected, "{n}");
        }
    }

    #[test]
    fn an_8_bit_row_past_twice_its_width_is_a_line_overflow() {
        let overflow = Err(GraphicsError::LineOverflow {
            frame: 0,
            line: 0,
            offset: 20,
        });
        assert_eq!(as_drawn(&one_frame(2, |f| f.pixels8(&[1; 5]))), overflow);
        assert_eq!(as_drawn(&one_frame(2, |f| f.run8(5, [1; 4]))), overflow);
        assert_eq!(as_drawn(&one_frame(2, |f| f.skip8(5))), overflow);
        assert!(as_drawn(&one_frame(2, |f| f.pixels8(&[1; 4]))).is_ok());
        // The second token overflows.
        let bytes = one_frame(2, |f| f.skip8(3).run8(2, [1; 4]));
        assert_eq!(
            as_drawn(&bytes),
            Err(GraphicsError::LineOverflow {
                frame: 0,
                line: 0,
                offset: 24,
            })
        );
        // Through the palette a row holds one byte per pixel.
        assert_eq!(palette(&one_frame(2, |f| f.pixels8(&[1; 3]))), overflow);
        assert!(palette(&one_frame(2, |f| f.pixels8(&[1; 2]))).is_ok());
    }

    #[test]
    fn through_the_system_palette_each_byte_is_a_colour_at_full_width() {
        let bytes = one_frame(4, |f| f.pixels8(&[0, 35, 215, 255]));
        let sheet = palette(&bytes).unwrap();
        assert_eq!(
            first(&sheet),
            [
                [0xFF, 0xFF, 0xFF, 255],
                [0xFF, 0, 0, 255],
                [0xEE, 0, 0, 255],
                [0, 0, 0, 255],
            ]
        );
        assert_eq!(sheet.solid_frames(), None);
        // A run fills its bytes with the word's first file byte; a skip
        // skips pixels.
        let bytes = one_frame(4, |f| f.skip8(1).run8(2, [35, 0, 215, 255]));
        let sheet = palette(&bytes).unwrap();
        let red = [0xFF, 0, 0, 255];
        assert_eq!(first(&sheet), [CLEAR, red, red, CLEAR]);
        assert_eq!(
            rgba(&sheet.solid_frames().unwrap()[0]),
            [CLEAR, red, red, CLEAR]
        );
    }

    #[test]
    fn each_frame_and_line_of_an_8_bit_sheet_starts_clear() {
        let bytes = RledBuilder::new(1, 2)
            .depth(8)
            .frame(|f| f.line().pixels8(&[0x00, 0x7C]).line().run8(2, [0; 4]))
            .frame(|f| f.line().line().pixels8(&[0xE0, 0x03]))
            .build();
        let sheet = as_drawn(&bytes).unwrap();
        let frames: Vec<_> = sheet.frames().iter().map(rgba).collect();
        assert_eq!(
            frames,
            [vec![rgb555(0x7C00), rgb555(0)], vec![CLEAR, rgb555(0x03E0)],]
        );
        let solid: Vec<_> = sheet.solid_frames().unwrap().iter().map(rgba).collect();
        assert_eq!(solid, [vec![CLEAR, rgb555(0)], vec![CLEAR, CLEAR]]);
        // Through the palette, too.
        let bytes = RledBuilder::new(1, 2)
            .depth(8)
            .frame(|f| f.line().pixels8(&[35]).line().run8(1, [215, 0, 0, 0]))
            .frame(|f| f.line().line().pixels8(&[0xE0]))
            .build();
        let sheet = palette(&bytes).unwrap();
        let frames: Vec<_> = sheet.frames().iter().map(rgba).collect();
        let (red, ramp) = (system_colour(35), system_colour(215));
        assert_eq!(frames, [vec![red, ramp], vec![CLEAR, system_colour(0xE0)]]);
        let solid: Vec<_> = sheet.solid_frames().unwrap().iter().map(rgba).collect();
        assert_eq!(solid, [vec![CLEAR, ramp], vec![CLEAR, CLEAR]]);
    }

    #[test]
    fn corrupt_8_bit_sheets_never_panic() {
        let bytes = RledBuilder::new(4, 3)
            .depth(8)
            .frame(|f| {
                f.line()
                    .pixels8(&[1, 2, 3])
                    .line()
                    .skip8(1)
                    .run8(3, [9, 8, 7, 6])
                    .line()
            })
            .frame(|f| f.line().pixels8(&[4, 5]).line())
            .build();
        assert!(bytes.len() < 1024);
        crate::sweep::assert_never_panics(&bytes, as_drawn);
        crate::sweep::assert_never_panics(&bytes, palette);
    }
}
