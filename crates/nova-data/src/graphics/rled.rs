//! `rlëD`: EV Nova's run-length encoded sprite sheets, 16-bit and 8-bit.
//!
//! Sources: the EV Nova Bible for meaning (16-bit run-length sprite sheets,
//! referenced by `spïn` and `shän`), and the open-source ResForge editor's
//! (MIT) `rlëD` decoder, consulted as a second reading for the byte layout
//! and token semantics. No code was copied. How the original fills a run,
//! and how it draws an 8-bit sheet ([`super::rled8`]), come from its own
//! blitter.

use nova_rsrc::ResType;

use super::GraphicsError;
use super::budget::check_budget;
use super::color::rgb555;
use super::image::Image;
use super::reader::Reader;
use super::rled8::{EightBitReading, Rled8};
use super::sheet::{SheetLayout, SpriteSheet};

/// The `rlëD` resource type: `rl`, Mac Roman `ë` (`0x91`), `D`.
pub const RLED: ResType = ResType::new([b'r', b'l', 0x91, b'D']);

/// The depth of every stock sheet: one 16-bit pixel per two bytes.
const DEPTH: u16 = 16;

/// The other depth the original draws: one byte per token count, drawn as
/// [`EightBitReading`] says.
const DEPTH_8: u16 = 8;

/// Token opcodes: the top byte of each big-endian `u32` token.
const FRAME_END: u8 = 0;
const LINE_START: u8 = 1;
pub(super) const PIXELS: u8 = 2;
pub(super) const SKIP: u8 = 3;
pub(super) const RUN: u8 = 4;

/// The low 24 bits of a token: its byte count.
const COUNT_MASK: u32 = 0x00FF_FFFF;

/// Decodes an `rlëD` to its frames, arranged by `layout` (or
/// [`SheetLayout::DEFAULT`]) when composed. An 8-bit sheet decodes as the
/// original's 16-bit world draws it ([`EightBitReading::AsDrawn`]); see
/// [`decode_rled_as`] for the other reading.
///
/// The 16-byte header is width, height, depth (16 or 8), palette (unused),
/// frame count and three reserved words. Tokens follow, each a big-endian
/// `u32` with an opcode in the top byte and a byte count below. At 16 bits:
///
/// - `0` ends the frame. Lines not drawn stay transparent (every stock
///   frame has exactly one line start per row).
/// - `1` starts the next line at its left edge. Its count is the byte
///   length of the line's tokens in stock data; it is not needed.
/// - `2` draws `count / 2` opaque 16-bit pixels from the data that follows,
///   padded to a 4-byte boundary.
/// - `3` skips `count / 2` transparent pixels.
/// - `4` draws `count / 2` pixels from the `u32` that follows, as the
///   original fills it: the word's **low** half on the run's even pixels
///   (counting from the run's start) and its **high** half on the odd
///   ones. `_BlitPixieRLESwap` byte-swaps the word to native little-endian
///   (@0xbe440), and `_BlitPixieRLE`'s op 4 (@0xbde2a-0xbde45) hands it to
///   `_BlitPixieMemSet` (@0xc0452-0xc049a), which stores it from the run's
///   first byte in 4-byte chunks, then a 2-byte and a 1-byte tail. Stock
///   runs are all `0x7FFF7FFF`, so stock cannot tell the halves apart.
///
/// A sheet with any run also keeps its runs alone, as a second set of
/// frames ([`SpriteSheet::solid_frames`]): each run's texels in its
/// colour, opaque, every other texel clear. The original's 16-bit RLE
/// blits fill a run raw (`_BlitPixieMemSet`), with no colorize,
/// translucency or blend, so a view draws these again over the layer.
/// A sheet with none keeps nothing more.
///
/// Tokens after the last frame are ignored. Any other depth is
/// [`GraphicsError::UnsupportedDepth`].
pub fn decode_rled(data: &[u8], layout: Option<SheetLayout>) -> Result<SpriteSheet, GraphicsError> {
    decode_rled_as(data, layout, EightBitReading::AsDrawn)
}

/// Decodes an `rlëD` as [`decode_rled`] does, reading an 8-bit sheet as
/// `reading` says. A 16-bit sheet decodes the same under either reading.
pub fn decode_rled_as(
    data: &[u8],
    layout: Option<SheetLayout>,
    reading: EightBitReading,
) -> Result<SpriteSheet, GraphicsError> {
    let mut r = Reader::new(data);
    let width = r.u16()?;
    let height = r.u16()?;
    let depth = r.u16()?;
    r.skip(2)?; // palette
    let count = r.u16()?;
    r.skip(6)?; // reserved
    if depth != DEPTH && depth != DEPTH_8 {
        return Err(GraphicsError::UnsupportedDepth { depth });
    }
    if width == 0 || height == 0 {
        return Err(GraphicsError::BadDimensions {
            width: i32::from(width),
            height: i32::from(height),
        });
    }
    if count == 0 {
        return Err(GraphicsError::NoFrames);
    }
    let (width, height) = (u32::from(width), u32::from(height));
    check_budget(
        u64::from(width) * u64::from(height) * u64::from(count),
        data.len(),
    )?;
    let layout = layout.unwrap_or(SheetLayout::DEFAULT);
    if depth == DEPTH_8 {
        let mut painter = Rled8::new(reading, width, height, count);
        walk(&mut r, data.len(), height, count, &mut painter)?;
        Ok(painter.finish(layout))
    } else {
        let mut painter = Rled16::new(width, height, count);
        walk(&mut r, data.len(), height, count, &mut painter)?;
        Ok(painter.finish(layout))
    }
}

/// One drawing token (`PIXELS`, `SKIP` or `RUN`) and where it is.
#[derive(Clone, Copy, Debug)]
pub(super) struct Token {
    /// The opcode.
    pub(super) op: u8,
    /// Its byte count.
    pub(super) count: u32,
    /// The frame it draws in, counting from 0.
    pub(super) frame: u32,
    /// The line it draws on, counting from 0.
    pub(super) line: u32,
    /// Where it starts.
    pub(super) offset: usize,
}

impl Token {
    /// The error for a token that draws past the end of its line.
    pub(super) fn overflow(&self) -> GraphicsError {
        GraphicsError::LineOverflow {
            frame: self.frame,
            line: self.line,
            offset: self.offset,
        }
    }
}

/// What one depth does with a sheet's tokens. The walk ([`walk`]) checks
/// the frame and line structure; a painter draws.
pub(super) trait Painter {
    /// A line starts: drawing returns to its left edge.
    fn line_start(&mut self);
    /// Draws `token`, reading any data it carries from `r`.
    fn draw(&mut self, token: Token, r: &mut Reader<'_>) -> Result<(), GraphicsError>;
    /// The frame ends.
    fn frame_end(&mut self);
}

/// Walks the tokens after the header until `count` frames have ended,
/// handing each to `painter`. `steps` bounds the walk: every token is four
/// bytes, so the data runs out first.
fn walk(
    r: &mut Reader<'_>,
    steps: usize,
    height: u32,
    count: u16,
    painter: &mut impl Painter,
) -> Result<(), GraphicsError> {
    let mut frames = 0;
    let mut lines = 0;
    for _ in 0..steps {
        let offset = r.pos();
        let token = r.u32()?;
        let op = (token >> 24) as u8;
        match op {
            FRAME_END => {
                painter.frame_end();
                frames += 1;
                if frames == u32::from(count) {
                    return Ok(());
                }
                lines = 0;
            }
            LINE_START => {
                if lines == height {
                    return Err(GraphicsError::TooManyLines {
                        frame: frames,
                        offset,
                    });
                }
                lines += 1;
                painter.line_start();
            }
            PIXELS | SKIP | RUN => {
                let Some(line) = lines.checked_sub(1) else {
                    return Err(GraphicsError::TokenOutsideLine { offset });
                };
                let token = Token {
                    op,
                    count: token & COUNT_MASK,
                    frame: frames,
                    line,
                    offset,
                };
                painter.draw(token, r)?;
            }
            token => return Err(GraphicsError::UnsupportedToken { token, offset }),
        }
    }
    Err(GraphicsError::UnexpectedEnd { offset: r.pos() })
}

/// The padding after `n` bytes of pixel data, up to a 4-byte boundary.
pub(super) fn pad4(n: usize) -> usize {
    (4 - n % 4) % 4
}

/// A 16-bit sheet's frames as they are drawn.
struct Rled16 {
    width: u32,
    height: u32,
    frames: Vec<Image>,
    /// Each finished frame's runs, if it had any.
    solids: Vec<Option<Image>>,
    frame: Image,
    /// The frame's own runs, once it has one.
    solid: Option<Image>,
    x: u32,
}

impl Rled16 {
    fn new(width: u32, height: u32, count: u16) -> Self {
        Self {
            width,
            height,
            frames: Vec::with_capacity(usize::from(count)),
            solids: Vec::with_capacity(usize::from(count)),
            frame: Image::transparent(width, height),
            solid: None,
            x: 0,
        }
    }

    fn finish(self, layout: SheetLayout) -> SpriteSheet {
        let solid = solid_frames(self.solids, self.width, self.height);
        SpriteSheet::new(self.width, self.height, self.frames, layout).with_solid(solid)
    }
}

impl Painter for Rled16 {
    fn line_start(&mut self) {
        self.x = 0;
    }

    fn draw(&mut self, token: Token, r: &mut Reader<'_>) -> Result<(), GraphicsError> {
        if token.op == PIXELS && token.count & 1 == 1 {
            return Err(GraphicsError::OddByteCount {
                offset: token.offset,
            });
        }
        let (x, y, len) = (self.x, token.line, token.count / 2);
        if x + len > self.width {
            return Err(token.overflow());
        }
        match token.op {
            PIXELS => {
                let bytes = r.bytes(2 * len as usize)?;
                for (i, &word) in (0..).zip(bytes.as_chunks::<2>().0) {
                    self.frame.put(x + i, y, rgb555(u16::from_be_bytes(word)));
                }
                r.skip(pad4(bytes.len()))?;
            }
            RUN => {
                let word = r.u32()?;
                // Low half on even pixels, high half on odd ones.
                let colours = [rgb555(word as u16), rgb555((word >> 16) as u16)];
                let (width, height) = (self.width, self.height);
                let solid = self
                    .solid
                    .get_or_insert_with(|| Image::transparent(width, height));
                for i in 0..len {
                    let colour = colours[(i % 2) as usize];
                    self.frame.put(x + i, y, colour);
                    solid.put(x + i, y, colour);
                }
            }
            _ => {}
        }
        self.x += len;
        Ok(())
    }

    fn frame_end(&mut self) {
        let frame = std::mem::replace(&mut self.frame, Image::transparent(self.width, self.height));
        self.frames.push(frame);
        self.solids.push(self.solid.take());
    }
}

/// Each frame's solid runs, a frame without any clear, when any frame
/// has one; else none.
pub(super) fn solid_frames(
    solids: Vec<Option<Image>>,
    width: u32,
    height: u32,
) -> Option<Vec<Image>> {
    solids.iter().any(Option::is_some).then(|| {
        solids
            .into_iter()
            .map(|solid| solid.unwrap_or_else(|| Image::transparent(width, height)))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::color::rgb555;
    use crate::graphics::fixture::{RledBuilder, RledFrame};
    use crate::graphics::image::Image;

    #[test]
    fn the_resource_type_is_rled_in_mac_roman() {
        let mac_roman = nova_rsrc::ResType::from_mac_roman("rlëD").expect("Mac Roman type");
        assert_eq!(RLED, mac_roman);
    }

    const RED: u16 = 0x7C00;
    const GREEN: u16 = 0x03E0;
    const BLUE: u16 = 0x001F;
    const WHITE: u16 = 0x7FFF;
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

    fn decode(data: &[u8]) -> Result<SpriteSheet, GraphicsError> {
        decode_rled(data, None)
    }

    /// Two 3x2 frames, written out by hand with stock-style line counts.
    #[test]
    fn decodes_a_hand_written_two_frame_sheet() {
        let bytes = [
            &[0x00, 0x03, 0x00, 0x02][..], // 3x2 frames
            &[0x00, 0x10, 0x00, 0x00],     // depth 16, palette 0
            &[0x00, 0x02, 0x00, 0x00],     // two frames, reserved
            &[0x00, 0x00, 0x00, 0x00],     // reserved
            // Frame 0.
            &[0x01, 0x00, 0x00, 0x08], // line: 8 bytes follow
            &[0x02, 0x00, 0x00, 0x04], // 4 bytes of pixels
            &[0x7C, 0x00, 0x03, 0xE0], // red, green
            &[0x01, 0x00, 0x00, 0x0C], // line: 12 bytes follow
            &[0x03, 0x00, 0x00, 0x02], // skip one pixel
            &[0x02, 0x00, 0x00, 0x02], // 2 bytes of pixels
            &[0x00, 0x1F, 0x00, 0x00], // blue, then 2 pad bytes
            &[0x00, 0x00, 0x00, 0x00], // frame end
            // Frame 1.
            &[0x01, 0x00, 0x00, 0x08], // line: 8 bytes follow
            &[0x04, 0x00, 0x00, 0x06], // run of three pixels
            &[0x7F, 0xFF, 0x7C, 0x00], // colours: red (low half), white
            &[0x01, 0x00, 0x00, 0x00], // empty line
            &[0x00, 0x00, 0x00, 0x00], // frame end
        ]
        .concat();
        let sheet = decode(&bytes).unwrap();
        let (red, green, blue, white) = (
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 255, 255],
        );
        assert_eq!((sheet.frame_width(), sheet.frame_height()), (3, 2));
        assert_eq!(
            frames(&sheet),
            [
                vec![red, green, CLEAR, CLEAR, blue, CLEAR],
                vec![red, white, red, CLEAR, CLEAR, CLEAR],
            ]
        );
    }

    #[test]
    fn every_frame_has_the_header_size_and_count() {
        let bytes = RledBuilder::new(4, 3)
            .frame(|f| f.line().pixels(&[RED]))
            .frame(|f| f)
            .frame(|f| f.line().line().line().skip(4))
            .build();
        let sheet = decode(&bytes).unwrap();
        assert_eq!(sheet.frames().len(), 3);
        for frame in sheet.frames() {
            assert_eq!((frame.width(), frame.height()), (4, 3));
        }
        assert_eq!(sheet.frames()[0].pixel(0, 0), Some(rgb555(RED)));
        assert_eq!(rgba(&sheet.frames()[1]), [CLEAR; 12]);
        assert_eq!(rgba(&sheet.frames()[2]), [CLEAR; 12]);
    }

    #[test]
    fn pixel_data_is_padded_to_four_bytes() {
        for pixels in [&[RED][..], &[RED, GREEN], &[RED, GREEN, BLUE], &[BLUE; 4]] {
            let bytes = RledBuilder::new(6, 1)
                .frame(|f| f.line().pixels(pixels).skip(1).pixels(&[WHITE]))
                .build();
            let mut expected: Vec<[u8; 4]> = pixels.iter().map(|&p| rgb555(p)).collect();
            expected.push(CLEAR);
            expected.push(rgb555(WHITE));
            expected.resize(6, CLEAR);
            assert_eq!(frames(&decode(&bytes).unwrap()), [expected], "{pixels:?}");
        }
    }

    #[test]
    fn lines_fill_top_down_and_each_starts_at_the_left() {
        let bytes = RledBuilder::new(2, 3)
            .frame(|f| {
                f.line()
                    .skip(1)
                    .pixels(&[RED])
                    .line()
                    .line()
                    .pixels(&[GREEN, BLUE])
            })
            .build();
        let red = rgb555(RED);
        assert_eq!(
            frames(&decode(&bytes).unwrap()),
            [vec![CLEAR, red, CLEAR, CLEAR, rgb555(GREEN), rgb555(BLUE)]]
        );
    }

    /// `_BlitPixieMemSet` stores the byte-swapped run word from the run's
    /// first pixel, so in the little-endian 16-bit world the run starts with
    /// the word's low half: file word `0x7C00_03E0` draws green, red, green.
    #[test]
    fn a_run_alternates_the_low_and_high_halves_of_its_word() {
        let bytes = RledBuilder::new(5, 1)
            .frame(|f| f.line().skip(1).run(3, RED, GREEN))
            .build();
        let (red, green) = (rgb555(RED), rgb555(GREEN));
        let sheet = decode(&bytes).unwrap();
        assert_eq!(frames(&sheet), [vec![CLEAR, green, red, green, CLEAR]]);
        let solid = sheet.solid_frames().expect("it has a run");
        assert_eq!(
            solid.iter().map(rgba).collect::<Vec<_>>(),
            [vec![CLEAR, green, red, green, CLEAR]]
        );
    }

    #[test]
    fn a_sheets_runs_are_kept_as_solid_frames_of_their_texels_alone() {
        let bytes = RledBuilder::new(5, 2)
            .frame(|f| {
                f.line()
                    .pixels(&[GREEN])
                    .run(2, WHITE, WHITE)
                    .skip(1)
                    .pixels(&[BLUE])
                    .line()
                    .skip(3)
                    .run(1, RED, RED)
            })
            .build();
        let sheet = decode(&bytes).unwrap();
        let solid = sheet.solid_frames().expect("it has runs");
        let (white, red) = (rgb555(WHITE), rgb555(RED));
        assert_eq!(
            solid.iter().map(rgba).collect::<Vec<_>>(),
            [vec![
                CLEAR, white, white, CLEAR, CLEAR, //
                CLEAR, CLEAR, CLEAR, red, CLEAR,
            ]]
        );
        // The frames themselves still hold the runs.
        assert_eq!(sheet.frames()[0].pixel(1, 0), Some(white));
        assert_eq!(sheet.into_solid_frames().map(|f| f.len()), Some(1));
    }

    #[test]
    fn a_sheet_without_runs_has_no_solid_frames() {
        let bytes = RledBuilder::new(3, 1)
            .frame(|f| f.line().pixels(&[RED]).skip(1).pixels(&[BLUE]))
            .frame(|f| f.line().skip(3))
            .build();
        let sheet = decode(&bytes).unwrap();
        assert_eq!(sheet.solid_frames(), None);
        assert_eq!(sheet.into_solid_frames(), None);
    }

    #[test]
    fn each_frames_runs_land_on_its_own_solid_frame() {
        let bytes = RledBuilder::new(2, 1)
            .frame(|f| f.line().pixels(&[RED, RED]))
            .frame(|f| f.line().run(1, WHITE, WHITE))
            .frame(|f| f.line().pixels(&[RED]))
            .frame(|f| f.line().skip(1).run(1, GREEN, GREEN))
            .build();
        let sheet = decode(&bytes).unwrap();
        let solid: Vec<_> = sheet
            .solid_frames()
            .expect("runs")
            .iter()
            .map(rgba)
            .collect();
        assert_eq!(
            solid,
            [
                vec![CLEAR, CLEAR],
                vec![rgb555(WHITE), CLEAR],
                vec![CLEAR, CLEAR],
                vec![CLEAR, rgb555(GREEN)],
            ]
        );
    }

    #[test]
    fn tokens_after_the_last_frame_are_ignored() {
        let bytes = RledBuilder::new(1, 1)
            .frame(|f| f.line().pixels(&[RED]))
            .token(0x77, 0)
            .frame(|f| f.line().pixels(&[RED, RED, RED]))
            .frame_count(1)
            .build();
        assert_eq!(frames(&decode(&bytes).unwrap()), [vec![rgb555(RED)]]);
    }

    #[test]
    fn the_layout_is_kept_or_defaulted() {
        let bytes = RledBuilder::new(1, 1).frame(|f| f).build();
        let three = SheetLayout::new(3).unwrap();
        assert_eq!(decode_rled(&bytes, Some(three)).unwrap().layout(), three);
        assert_eq!(decode(&bytes).unwrap().layout(), SheetLayout::DEFAULT);
    }

    #[test]
    fn header_fields_are_checked() {
        let one = |b: RledBuilder| decode(&b.frame(|f| f).build());
        assert_eq!(
            one(RledBuilder::new(2, 2).depth(1)),
            Err(GraphicsError::UnsupportedDepth { depth: 1 })
        );
        assert_eq!(
            one(RledBuilder::new(2, 2).depth(32)),
            Err(GraphicsError::UnsupportedDepth { depth: 32 })
        );
        assert_eq!(
            one(RledBuilder::new(0, 2)),
            Err(GraphicsError::BadDimensions {
                width: 0,
                height: 2
            })
        );
        assert_eq!(
            one(RledBuilder::new(2, 0)),
            Err(GraphicsError::BadDimensions {
                width: 2,
                height: 0
            })
        );
        assert_eq!(
            one(RledBuilder::new(2, 2).frame_count(0)),
            Err(GraphicsError::NoFrames)
        );
    }

    #[test]
    fn an_8_bit_sheet_decodes_as_drawn_unless_asked_otherwise() {
        let bytes = RledBuilder::new(2, 1)
            .depth(8)
            .frame(|f| f.line().pixels8(&[0x00, 0x7C]))
            .build();
        let as_drawn = decode_rled_as(&bytes, None, EightBitReading::AsDrawn).unwrap();
        assert_eq!(decode(&bytes), Ok(as_drawn.clone()));
        assert_eq!(frames(&as_drawn), [vec![rgb555(0x7C00), CLEAR]]);
        let palette = decode_rled_as(&bytes, None, EightBitReading::SystemPalette).unwrap();
        assert_ne!(palette, as_drawn);
        // The layout is kept at 8 bits, too.
        let three = SheetLayout::new(3).unwrap();
        assert_eq!(decode_rled(&bytes, Some(three)).unwrap().layout(), three);
        // A 16-bit sheet is the same under either reading.
        let bytes = RledBuilder::new(1, 1)
            .frame(|f| f.line().run(1, RED, GREEN))
            .build();
        assert_eq!(
            decode_rled_as(&bytes, None, EightBitReading::SystemPalette),
            decode(&bytes)
        );
        // Any other depth is unsupported.
        let bytes = RledBuilder::new(2, 1).depth(32).frame(|f| f).build();
        assert_eq!(
            decode_rled_as(&bytes, None, EightBitReading::SystemPalette),
            Err(GraphicsError::UnsupportedDepth { depth: 32 })
        );
    }

    #[test]
    fn a_truncated_sheet_is_an_unexpected_end() {
        let bytes = RledBuilder::new(3, 2)
            .frame(|f| f.line().pixels(&[RED, GREEN, BLUE]).line().run(2, RED, RED))
            .frame(|f| f.line().skip(1))
            .build();
        for len in 0..bytes.len() {
            assert!(
                matches!(
                    decode(&bytes[..len]),
                    Err(GraphicsError::UnexpectedEnd { .. })
                ),
                "{len}"
            );
        }
        assert_eq!(
            decode(&bytes[..15]),
            Err(GraphicsError::UnexpectedEnd { offset: 10 })
        );
        // Missing the last frame end.
        assert_eq!(
            decode(&bytes[..bytes.len() - 4]),
            Err(GraphicsError::UnexpectedEnd {
                offset: bytes.len() - 4
            })
        );
        // Pixel data cut short, and its padding missing.
        assert_eq!(
            decode(&bytes[..28]),
            Err(GraphicsError::UnexpectedEnd { offset: 24 })
        );
        assert_eq!(
            decode(&bytes[..31]),
            Err(GraphicsError::UnexpectedEnd { offset: 30 })
        );
    }

    #[test]
    fn unknown_tokens_are_named() {
        for op in [5, 6, 0x80, 0xFF] {
            let bytes = RledBuilder::new(2, 2)
                .frame(|f| f.line().token(op, 0))
                .build();
            assert_eq!(
                decode(&bytes),
                Err(GraphicsError::UnsupportedToken {
                    token: op,
                    offset: 20
                }),
                "{op}"
            );
        }
    }

    #[test]
    fn drawing_before_a_line_starts_is_rejected() {
        let starts = [
            RledBuilder::new(2, 2).frame(|f| f.pixels(&[RED])),
            RledBuilder::new(2, 2).frame(|f| f.skip(1)),
            RledBuilder::new(2, 2).frame(|f| f.run(1, RED, RED)),
        ];
        for builder in starts {
            assert_eq!(
                decode(&builder.build()),
                Err(GraphicsError::TokenOutsideLine { offset: 16 })
            );
        }
        // A new frame starts with no line.
        let bytes = RledBuilder::new(2, 2)
            .frame(RledFrame::line)
            .frame(|f| f.pixels(&[RED]))
            .build();
        assert_eq!(
            decode(&bytes),
            Err(GraphicsError::TokenOutsideLine { offset: 24 })
        );
    }

    #[test]
    fn drawing_past_the_end_of_a_line_is_rejected() {
        let overflow = |frame: u32, line: u32, offset: usize| {
            Err(GraphicsError::LineOverflow {
                frame,
                line,
                offset,
            })
        };
        let bytes = RledBuilder::new(3, 2)
            .frame(|f| f.line().pixels(&[RED, RED]).pixels(&[RED, RED]))
            .build();
        assert_eq!(decode(&bytes), overflow(0, 0, 28));
        let bytes = RledBuilder::new(3, 2)
            .frame(RledFrame::line)
            .frame(|f| f.line().line().skip(2).run(2, RED, RED))
            .build();
        assert_eq!(decode(&bytes), overflow(1, 1, 36));
        // Exactly full is fine.
        let bytes = RledBuilder::new(3, 1)
            .frame(|f| f.line().skip(1).run(1, RED, RED).pixels(&[RED]))
            .build();
        assert!(decode(&bytes).is_ok());
        let bytes = RledBuilder::new(3, 1).frame(|f| f.line().skip(4)).build();
        assert_eq!(decode(&bytes), overflow(0, 0, 20));
    }

    #[test]
    fn a_maximal_count_overflows_without_reading_data() {
        for op in [2, 3, 4] {
            let bytes = RledBuilder::new(3, 1)
                .frame(|f| f.line().token(op, 0xFF_FFFE))
                .build();
            assert_eq!(
                decode(&bytes),
                Err(GraphicsError::LineOverflow {
                    frame: 0,
                    line: 0,
                    offset: 20
                }),
                "{op}"
            );
        }
    }

    #[test]
    fn more_lines_than_the_height_are_rejected() {
        let bytes = RledBuilder::new(1, 2)
            .frame(|f| f.line().line().line())
            .build();
        assert_eq!(
            decode(&bytes),
            Err(GraphicsError::TooManyLines {
                frame: 0,
                offset: 24
            })
        );
        let bytes = RledBuilder::new(1, 2)
            .frame(|f| f.line().line())
            .frame(|f| f.line().line().line())
            .build();
        assert_eq!(
            decode(&bytes),
            Err(GraphicsError::TooManyLines {
                frame: 1,
                offset: 36
            })
        );
    }

    #[test]
    fn pixel_data_must_be_whole_pixels() {
        let bytes = RledBuilder::new(3, 1)
            .frame(|f| f.line().token(2, 3).raw(&[0; 4]))
            .build();
        assert_eq!(
            decode(&bytes),
            Err(GraphicsError::OddByteCount { offset: 20 })
        );
    }

    #[test]
    fn a_huge_header_in_a_tiny_sheet_is_too_large() {
        let bytes = RledBuilder::new(65535, 65535).frame_count(65535).build();
        assert_eq!(
            decode(&bytes),
            Err(GraphicsError::TooLarge {
                pixels: 65535 * 65535 * 65535,
                input_len: 16
            })
        );
    }

    #[test]
    fn corrupt_sheets_never_panic() {
        let bytes = RledBuilder::new(4, 3)
            .frame(|f| {
                f.line()
                    .pixels(&[RED, GREEN, BLUE])
                    .line()
                    .skip(1)
                    .run(2, WHITE, RED)
                    .line()
            })
            .frame(|f| f.line().pixels(&[RED, RED]).line())
            .build();
        assert!(bytes.len() < 1024);
        crate::sweep::assert_never_panics(&bytes, decode);
    }
}
