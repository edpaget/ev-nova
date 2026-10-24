//! `rlëD`: EV Nova's run-length encoded 16-bit sprite sheets.
//!
//! Sources: the EV Nova Bible for meaning (16-bit run-length sprite sheets,
//! referenced by `spïn` and `shän`), and the open-source ResForge editor's
//! (MIT) `rlëD` decoder, consulted as a second reading for the byte layout
//! and token semantics. No code was copied.

use nova_rsrc::ResType;

use super::GraphicsError;
use super::budget::check_budget;
use super::color::rgb555;
use super::image::Image;
use super::reader::Reader;
use super::sheet::{SheetLayout, SpriteSheet};

/// The `rlëD` resource type: `rl`, Mac Roman `ë` (`0x91`), `D`.
pub const RLED: ResType = ResType::new([b'r', b'l', 0x91, b'D']);

/// The only depth stock data (and this decoder) uses.
const DEPTH: u16 = 16;

/// Token opcodes: the top byte of each big-endian `u32` token.
const FRAME_END: u8 = 0;
const LINE_START: u8 = 1;
const PIXELS: u8 = 2;
const SKIP: u8 = 3;
const RUN: u8 = 4;

/// The low 24 bits of a token: its byte count.
const COUNT_MASK: u32 = 0x00FF_FFFF;

/// What [`scan_rled`] reads from an `rlëD` without decoding its pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RledInfo {
    /// Each frame's width in pixels.
    pub frame_width: u32,
    /// Each frame's height in pixels.
    pub frame_height: u32,
    /// How many frames the sheet holds: at least one.
    pub frames: u16,
    /// Whether any frame has a run, so the decoded sheet would keep
    /// [`SpriteSheet::solid_frames`].
    pub has_runs: bool,
}

/// Decodes an `rlëD` to its frames, arranged by `layout` (or
/// [`SheetLayout::DEFAULT`]) when composed.
///
/// The 16-byte header is width, height, depth (16), palette (unused),
/// frame count and three reserved words. Tokens follow, each a big-endian
/// `u32` with an opcode in the top byte and a byte count below:
///
/// - `0` ends the frame. Lines not drawn stay transparent (every stock
///   frame has exactly one line start per row).
/// - `1` starts the next line at its left edge. Its count is the byte
///   length of the line's tokens in stock data; it is not needed.
/// - `2` draws `count / 2` opaque 16-bit pixels from the data that follows,
///   padded to a 4-byte boundary.
/// - `3` skips `count / 2` transparent pixels.
/// - `4` draws `count / 2` pixels of one colour: the high half of the `u32`
///   that follows. The low half is ignored, as ResForge does; stock data
///   (always `0x7FFF7FFF`) cannot tell this from alternating halves.
///
/// A sheet with any run also keeps its runs alone, as a second set of
/// frames ([`SpriteSheet::solid_frames`]): each run's texels in its
/// colour, opaque, every other texel clear. The original's 16-bit RLE
/// blits fill a run raw (`_BlitPixieMemSet`), with no colorize,
/// translucency or blend, so a view draws these again over the layer.
/// A sheet with none keeps nothing more.
///
/// Tokens after the last frame are ignored.
pub fn decode_rled(data: &[u8], layout: Option<SheetLayout>) -> Result<SpriteSheet, GraphicsError> {
    let mut r = Reader::new(data);
    let header = read_header(&mut r, data.len())?;
    let (width, height) = (header.width, header.height);
    let mut painter = Painter {
        width,
        height,
        frames: Vec::with_capacity(usize::from(header.count)),
        frame: Image::transparent(width, height),
        solids: Vec::with_capacity(usize::from(header.count)),
        solid: None,
    };
    walk(&mut r, &header, data.len(), &mut painter)?;
    let layout = layout.unwrap_or(SheetLayout::DEFAULT);
    let sheet = SpriteSheet::new(width, height, painter.frames, layout);
    Ok(sheet.with_solid(solid_frames(painter.solids, width, height)))
}

/// Reads an `rlëD`'s frame size, frame count and whether it has runs,
/// without decoding a pixel.
///
/// It walks the same tokens as [`decode_rled`], so it fails exactly where
/// and as that does, and succeeds with what the decoded sheet would hold.
pub fn scan_rled(data: &[u8]) -> Result<RledInfo, GraphicsError> {
    let mut r = Reader::new(data);
    let header = read_header(&mut r, data.len())?;
    let mut scanner = Scanner { has_runs: false };
    walk(&mut r, &header, data.len(), &mut scanner)?;
    Ok(RledInfo {
        frame_width: header.width,
        frame_height: header.height,
        frames: header.count,
        has_runs: scanner.has_runs,
    })
}

/// An `rlëD`'s checked header.
struct Header {
    width: u32,
    height: u32,
    /// At least one.
    count: u16,
}

/// Reads and checks the 16-byte header of an `input_len`-byte sheet.
fn read_header(r: &mut Reader<'_>, input_len: usize) -> Result<Header, GraphicsError> {
    let width = r.u16()?;
    let height = r.u16()?;
    let depth = r.u16()?;
    r.skip(2)?; // palette
    let count = r.u16()?;
    r.skip(6)?; // reserved
    if depth != DEPTH {
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
        input_len,
    )?;
    Ok(Header {
        width,
        height,
        count,
    })
}

/// What a token walk does with the tokens it has checked.
trait Sink {
    /// `pixels` holds big-endian 16-bit pixels, drawn from `(x, y)` right.
    fn pixels(&mut self, x: u32, y: u32, pixels: &[u8]);
    /// `len` pixels of `colour`, from `(x, y)` right.
    fn run(&mut self, x: u32, y: u32, len: u32, colour: u16);
    /// The current frame is finished.
    fn frame_end(&mut self);
}

/// Walks the tokens after the header of an `input_len`-byte sheet,
/// checking each, until `header.count` frames have ended.
fn walk(
    r: &mut Reader<'_>,
    header: &Header,
    input_len: usize,
    sink: &mut impl Sink,
) -> Result<(), GraphicsError> {
    let (width, height) = (header.width, header.height);
    let mut ended: u32 = 0;
    let mut lines = 0;
    let mut x = 0;
    // Every token is four bytes, so the data runs out first.
    for _ in 0..input_len {
        let offset = r.pos();
        let token = r.u32()?;
        let op = (token >> 24) as u8;
        let len = (token & COUNT_MASK) / 2;
        match op {
            FRAME_END => {
                sink.frame_end();
                ended += 1;
                if ended == u32::from(header.count) {
                    return Ok(());
                }
                lines = 0;
            }
            LINE_START => {
                if lines == height {
                    return Err(GraphicsError::TooManyLines {
                        frame: ended,
                        offset,
                    });
                }
                lines += 1;
                x = 0;
            }
            PIXELS | SKIP | RUN => {
                let Some(y) = lines.checked_sub(1) else {
                    return Err(GraphicsError::TokenOutsideLine { offset });
                };
                if op == PIXELS && token & 1 == 1 {
                    return Err(GraphicsError::OddByteCount { offset });
                }
                if x + len > width {
                    return Err(GraphicsError::LineOverflow {
                        frame: ended,
                        line: y,
                        offset,
                    });
                }
                match op {
                    PIXELS => {
                        let bytes = r.bytes(2 * len as usize)?;
                        r.skip(bytes.len() % 4)?;
                        sink.pixels(x, y, bytes);
                    }
                    RUN => {
                        let colour = (r.u32()? >> 16) as u16;
                        sink.run(x, y, len, colour);
                    }
                    _ => {}
                }
                x += len;
            }
            token => return Err(GraphicsError::UnsupportedToken { token, offset }),
        }
    }
    Err(GraphicsError::UnexpectedEnd { offset: r.pos() })
}

/// A sink that paints each frame, and each frame's runs alone.
struct Painter {
    width: u32,
    height: u32,
    frames: Vec<Image>,
    frame: Image,
    /// Each finished frame's runs, if it had any.
    solids: Vec<Option<Image>>,
    /// The current frame's runs, if it has any.
    solid: Option<Image>,
}

impl Sink for Painter {
    fn pixels(&mut self, x: u32, y: u32, pixels: &[u8]) {
        for (i, &word) in (0..).zip(pixels.as_chunks::<2>().0) {
            self.frame.put(x + i, y, rgb555(u16::from_be_bytes(word)));
        }
    }

    fn run(&mut self, x: u32, y: u32, len: u32, colour: u16) {
        let colour = rgb555(colour);
        let (width, height) = (self.width, self.height);
        let solid = self
            .solid
            .get_or_insert_with(|| Image::transparent(width, height));
        for i in 0..len {
            self.frame.put(x + i, y, colour);
            solid.put(x + i, y, colour);
        }
    }

    fn frame_end(&mut self) {
        let next = Image::transparent(self.width, self.height);
        self.frames.push(std::mem::replace(&mut self.frame, next));
        self.solids.push(self.solid.take());
    }
}

/// A sink that only notes whether any run appeared.
struct Scanner {
    has_runs: bool,
}

impl Sink for Scanner {
    fn pixels(&mut self, _: u32, _: u32, _: &[u8]) {}

    fn run(&mut self, _: u32, _: u32, _: u32, _: u16) {
        self.has_runs = true;
    }

    fn frame_end(&mut self) {}
}

/// Each frame's solid runs, a frame without any clear, when any frame
/// has one; else none.
fn solid_frames(solids: Vec<Option<Image>>, width: u32, height: u32) -> Option<Vec<Image>> {
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
            &[0x7F, 0xFF, 0x7C, 0x00], // colours: white, red
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
                vec![white, white, white, CLEAR, CLEAR, CLEAR],
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

    /// ResForge reads op 4 as `count / 2` copies of the high 16 bits of the
    /// colour word; the low half is ignored. Stock data cannot tell this
    /// from alternating colours (both halves are always `0x7FFF`).
    #[test]
    fn a_run_repeats_the_first_colour_of_its_pair() {
        let bytes = RledBuilder::new(5, 1)
            .frame(|f| f.line().skip(1).run(3, RED, BLUE))
            .build();
        let red = rgb555(RED);
        assert_eq!(
            frames(&decode(&bytes).unwrap()),
            [vec![CLEAR, red, red, red, CLEAR]]
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
            one(RledBuilder::new(2, 2).depth(8)),
            Err(GraphicsError::UnsupportedDepth { depth: 8 })
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

    /// What `scan_rled` reads, taken from a decoded sheet.
    fn info_of(sheet: &SpriteSheet) -> RledInfo {
        RledInfo {
            frame_width: sheet.frame_width(),
            frame_height: sheet.frame_height(),
            frames: sheet.frames().len() as u16,
            has_runs: sheet.solid_frames().is_some(),
        }
    }

    #[test]
    fn a_scan_reads_the_size_frame_count_and_runs() {
        let bytes = RledBuilder::new(3, 2)
            .frame(|f| f.line().pixels(&[RED, GREEN]))
            .frame(|f| f.line().run(3, WHITE, RED))
            .build();
        let expected = RledInfo {
            frame_width: 3,
            frame_height: 2,
            frames: 2,
            has_runs: true,
        };
        assert_eq!(scan_rled(&bytes), Ok(expected));
        assert_eq!(decode(&bytes).map(|sheet| info_of(&sheet)), Ok(expected));
    }

    #[test]
    fn a_scan_of_a_sheet_without_runs_says_so() {
        let bytes = RledBuilder::new(4, 1)
            .frame(|f| f.line().pixels(&[RED]).skip(1).pixels(&[BLUE]))
            .frame(|f| f.line().skip(4))
            .frame(|f| f)
            .build();
        assert_eq!(
            scan_rled(&bytes),
            Ok(RledInfo {
                frame_width: 4,
                frame_height: 1,
                frames: 3,
                has_runs: false,
            })
        );
    }

    /// Every way a sheet can be broken fails the scan as it fails the
    /// decode, offsets and all.
    #[test]
    fn a_scan_fails_exactly_as_the_decode_does() {
        let truncated = RledBuilder::new(3, 2)
            .frame(|f| f.line().pixels(&[RED, GREEN, BLUE]).line().run(2, RED, RED))
            .frame(|f| f.line().skip(1))
            .build();
        let cases: Vec<(&str, Vec<u8>)> = vec![
            (
                "bad depth",
                RledBuilder::new(2, 2).depth(8).frame(|f| f).build(),
            ),
            ("zero width", RledBuilder::new(0, 2).frame(|f| f).build()),
            ("zero height", RledBuilder::new(2, 0).frame(|f| f).build()),
            ("no frames", RledBuilder::new(2, 2).frame_count(0).build()),
            (
                "over budget",
                RledBuilder::new(65535, 65535).frame_count(65535).build(),
            ),
            (
                "too many lines",
                RledBuilder::new(1, 2)
                    .frame(|f| f.line().line())
                    .frame(|f| f.line().line().line())
                    .build(),
            ),
            (
                "token outside a line",
                RledBuilder::new(2, 2)
                    .frame(RledFrame::line)
                    .frame(|f| f.run(1, RED, RED))
                    .build(),
            ),
            (
                "odd byte count",
                RledBuilder::new(3, 1)
                    .frame(|f| f.line().token(2, 3).raw(&[0; 4]))
                    .build(),
            ),
            (
                "line overflow",
                RledBuilder::new(3, 2)
                    .frame(RledFrame::line)
                    .frame(|f| f.line().line().skip(2).run(2, RED, RED))
                    .build(),
            ),
            (
                "unknown token",
                RledBuilder::new(2, 2)
                    .frame(|f| f.line().token(5, 0))
                    .build(),
            ),
            ("truncated pixels", truncated[..28].to_vec()),
            ("truncated padding", truncated[..31].to_vec()),
            ("truncated run", truncated[..truncated.len() - 16].to_vec()),
            (
                "no last frame end",
                truncated[..truncated.len() - 4].to_vec(),
            ),
        ];
        for (name, bytes) in cases {
            let decoded = decode(&bytes).map(|sheet| info_of(&sheet));
            assert!(decoded.is_err(), "{name}: {decoded:?}");
            assert_eq!(scan_rled(&bytes), decoded, "{name}");
        }
        for len in 0..truncated.len() {
            let bytes = &truncated[..len];
            assert_eq!(
                scan_rled(bytes),
                decode(bytes).map(|sheet| info_of(&sheet)),
                "{len}"
            );
        }
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
