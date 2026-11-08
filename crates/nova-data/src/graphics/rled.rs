//! `rlëD`: EV Nova's run-length encoded sprite sheets, 16-bit, 8-bit, and
//! of any other depth the original draws.
//!
//! Sources: the EV Nova Bible for meaning (16-bit run-length sprite sheets,
//! referenced by `spïn` and `shän`), and the open-source ResForge editor's
//! (MIT) `rlëD` decoder, consulted as a second reading for the byte layout
//! and token semantics. No code was copied. How the original fills a run,
//! and how it draws an 8-bit sheet ([`super::rled8`]) or a sheet of any
//! other depth ([`super::rled_wide`]), come from its own blitter.

use nova_rsrc::ResType;

use super::GraphicsError;
use super::budget::check_budget;
use super::color::rgb555;
use super::image::Image;
use super::reader::Reader;
use super::rled_clipped::{clip_quirks, takes_the_blit};
use super::rled_wide::decode_wide;
use super::rled8::{EightBitReading, Rled8};
use super::rled32::ThirtyTwoBitReading;
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
    /// The header's depth, as the decoded sheet's [`SpriteSheet::depth`]:
    /// 16, 8, 32 or another the original draws.
    pub depth: u16,
    /// Each frame's image's width in pixels, as the decoded sheet's
    /// [`SpriteSheet::image_width`]: the frame's, unless the sheet draws
    /// past its box, as a 32-bit sheet does.
    pub image_width: u32,
    /// Each frame's image's height in pixels
    /// ([`SpriteSheet::image_height`]).
    pub image_height: u32,
    /// Whether a frame the world's edge cuts can draw otherwise than its
    /// image cut to its box, as the decoded sheet's
    /// [`SpriteSheet::clip_quirks`].
    pub clip_quirks: bool,
}

/// Decodes an `rlëD` to its frames, arranged by `layout` (or
/// [`SheetLayout::DEFAULT`]) when composed. An 8-bit sheet decodes as the
/// original's 16-bit world draws it ([`EightBitReading::AsDrawn`]); see
/// [`decode_rled_as`] for the other reading.
///
/// The 16-byte header is width, height, depth, palette (unused), frame
/// count and three reserved words. Tokens follow, each a big-endian
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
/// Tokens after the last frame are ignored. A pixel token of no bytes is
/// [`GraphicsError::RunawaySwap`]: the original's byte swap runs away on
/// it (@0xbe460-0xbe46a).
///
/// A sheet of any other depth whose signed value is above 7, 32 among
/// them, decodes byte for byte as the original's 16-bit world draws it,
/// its images padded about its box where they overhang it (see
/// [`super::rled_wide`]). A depth of 7 or less, or of 0x8000 and up, is
/// [`GraphicsError::UnsupportedDepth`]: the original's draw procs assert
/// on it, which quits the game.
pub fn decode_rled(data: &[u8], layout: Option<SheetLayout>) -> Result<SpriteSheet, GraphicsError> {
    decode_rled_with(data, layout, RledReadings::default())
}

/// Decodes an `rlëD` as [`decode_rled`] does, reading an 8-bit sheet as
/// `reading` says, and a 32-bit one as the original draws it. A sheet of
/// another depth decodes the same under either reading.
pub fn decode_rled_as(
    data: &[u8],
    layout: Option<SheetLayout>,
    reading: EightBitReading,
) -> Result<SpriteSheet, GraphicsError> {
    let readings = RledReadings {
        eight_bit: reading,
        ..RledReadings::default()
    };
    decode_rled_with(data, layout, readings)
}

/// How the depths that have another reading are read: 8 bits and 32. Each
/// defaults to the original's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RledReadings {
    /// How an 8-bit sheet is read.
    pub eight_bit: EightBitReading,
    /// How a 32-bit sheet is read.
    pub thirty_two_bit: ThirtyTwoBitReading,
}

/// Decodes an `rlëD` as [`decode_rled`] does, reading an 8-bit or 32-bit
/// sheet as `readings` say. A sheet of any other depth decodes the same
/// under every reading.
pub fn decode_rled_with(
    data: &[u8],
    layout: Option<SheetLayout>,
    readings: RledReadings,
) -> Result<SpriteSheet, GraphicsError> {
    let mut r = Reader::new(data);
    let header = read_header(&mut r, data.len())?;
    let layout = layout.unwrap_or(SheetLayout::DEFAULT);
    let sheet = decode_plain(data, r.pos(), &header, layout, readings)?;
    let quirks = takes_the_blit(&header, readings) && clip_quirks(data, &header);
    Ok(sheet.with_clip_quirks(quirks))
}

/// The sheet [`decode_rled_with`] decodes, before its
/// [`SpriteSheet::clip_quirks`]: `data`'s tokens from `start`, under the
/// checked `header`.
fn decode_plain(
    data: &[u8],
    start: usize,
    header: &Header,
    layout: SheetLayout,
    readings: RledReadings,
) -> Result<SpriteSheet, GraphicsError> {
    let (width, height) = (header.width, header.height);
    if header.depth != DEPTH && header.depth != DEPTH_8 {
        return decode_wide(data, header, layout, readings.thirty_two_bit)
            .map(|sheet| sheet.with_depth(header.depth));
    }
    let reading = readings.eight_bit;
    let units = Units::of(header, reading);
    if header.depth == DEPTH_8 {
        let mut painter = Rled8::new(reading, width, height, header.count);
        walk(data, start, header, units, &mut painter)?;
        Ok(painter.finish(layout).with_depth(DEPTH_8))
    } else {
        let mut painter = Painter {
            width,
            height,
            frames: Vec::with_capacity(usize::from(header.count)),
            frame: Image::transparent(width, height),
            solids: Vec::with_capacity(usize::from(header.count)),
            solid: None,
        };
        walk(data, start, header, units, &mut painter)?;
        let sheet = SpriteSheet::new(width, height, painter.frames, layout);
        Ok(sheet.with_solid(solid_frames(painter.solids, width, height)))
    }
}

/// Reads an `rlëD`'s frame size, frame count, whether it has runs, its
/// image size and its clip quirks, without decoding a pixel of a 16 or
/// 8-bit sheet.
///
/// It walks the same tokens as [`decode_rled`], so it fails exactly where
/// and as that does, and succeeds with what the decoded sheet would hold.
pub fn scan_rled(data: &[u8]) -> Result<RledInfo, GraphicsError> {
    let mut r = Reader::new(data);
    let header = read_header(&mut r, data.len())?;
    if header.depth != DEPTH && header.depth != DEPTH_8 {
        // Either reading loads, fails and sizes a sheet alike.
        let sheet = decode_wide(
            data,
            &header,
            SheetLayout::DEFAULT,
            ThirtyTwoBitReading::AsDrawn,
        )?;
        return Ok(RledInfo {
            frame_width: header.width,
            frame_height: header.height,
            frames: header.count,
            has_runs: sheet.solid_frames().is_some(),
            depth: header.depth,
            image_width: sheet.image_width(),
            image_height: sheet.image_height(),
            clip_quirks: scanned_quirks(data, &header),
        });
    }
    let units = Units::of(&header, EightBitReading::AsDrawn);
    let mut scanner = Scanner { has_runs: false };
    walk(data, r.pos(), &header, units, &mut scanner)?;
    Ok(RledInfo {
        frame_width: header.width,
        frame_height: header.height,
        frames: header.count,
        has_runs: scanner.has_runs,
        depth: header.depth,
        image_width: header.width,
        image_height: header.height,
        clip_quirks: scanned_quirks(data, &header),
    })
}

/// What [`rled_frame`] reads from an `rlëD`'s header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RledFrameSize {
    /// Each frame's width in pixels: at least one.
    pub width: u32,
    /// Each frame's height in pixels: at least one.
    pub height: u32,
}

/// Reads an `rlëD`'s frame size from its header alone, as the original's
/// `_SWCreateSpriteFromRLEResource` (@0xa9990) makes a sprite from one:
/// it reads the header (width, height, depth, count) and decodes no token,
/// so a sheet whose tokens are corrupt still has its header's frame.
///
/// The header is checked as [`decode_rled`] checks it, so a bad depth, a
/// zero size, no frames or a sheet over the decode budget fails with the
/// same [`GraphicsError`].
pub fn rled_frame(data: &[u8]) -> Result<RledFrameSize, GraphicsError> {
    let header = read_header(&mut Reader::new(data), data.len())?;
    Ok(RledFrameSize {
        width: header.width,
        height: header.height,
    })
}

/// The [`SpriteSheet::clip_quirks`] of the sheet `data`, whose checked
/// header is `header`, as [`decode_rled`] reads it: the walk
/// [`decode_rled_with`] takes, under the default readings.
fn scanned_quirks(data: &[u8], header: &Header) -> bool {
    takes_the_blit(header, RledReadings::default()) && clip_quirks(data, header)
}

/// An `rlëD`'s checked header.
pub(super) struct Header {
    pub(super) width: u32,
    pub(super) height: u32,
    /// Above 7 as a signed word: [`DEPTH`], [`DEPTH_8`], or another the
    /// original draws as [`super::rled_wide`] says.
    pub(super) depth: u16,
    /// At least one.
    pub(super) count: u16,
}

/// Reads and checks the 16-byte header of an `input_len`-byte sheet.
pub(super) fn read_header(r: &mut Reader<'_>, input_len: usize) -> Result<Header, GraphicsError> {
    let width = r.u16()?;
    let height = r.u16()?;
    let depth = r.u16()?;
    r.skip(2)?; // palette
    let count = r.u16()?;
    r.skip(6)?; // reserved
    // The draw procs compare the depth signed (`cmpw $0x7; jle`,
    // @0xbb46e-0xbb476).
    #[allow(clippy::cast_possible_wrap)]
    if depth as i16 <= 7 {
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
        depth,
        count,
    })
}

/// How a walk measures a token's count: in 16-bit pixels (two bytes
/// each), or byte by byte for an 8-bit sheet.
#[derive(Clone, Copy, Debug)]
struct Units {
    /// Bytes per unit: 2 for a 16-bit pixel, 1 for an 8-bit byte.
    size: u32,
    /// Units a line holds.
    row_len: u32,
}

impl Units {
    /// The units of `header`'s sheet, an 8-bit one read as `reading` says:
    /// as drawn, a row holds two bytes per pixel; through the palette, one.
    fn of(header: &Header, reading: EightBitReading) -> Self {
        if header.depth == DEPTH_8 {
            let row_len = match reading {
                EightBitReading::AsDrawn => 2 * header.width,
                EightBitReading::SystemPalette => header.width,
            };
            Self { size: 1, row_len }
        } else {
            Self {
                size: 2,
                row_len: header.width,
            }
        }
    }
}

/// What a token walk does with the tokens it has checked. Positions and
/// lengths are in the walk's [`Units`].
pub(super) trait Sink {
    /// A `PIXELS` token's data, drawn from `(x, y)` right: big-endian
    /// 16-bit pixels at 16 bits, one byte per unit at 8.
    fn pixels(&mut self, x: u32, y: u32, pixels: &[u8]);
    /// A `RUN` of `len` units of `word` (as read big-endian from the
    /// file), from `(x, y)` right.
    fn run(&mut self, x: u32, y: u32, len: u32, word: u32);
    /// The current frame is finished.
    fn frame_end(&mut self);
}

/// The padding after `n` bytes of pixel data, up to a 4-byte boundary.
fn pad4(n: usize) -> usize {
    (4 - n % 4) % 4
}

/// Walks the tokens of `data` from `start` (just past the header),
/// checking each, until `header.count` frames have ended.
///
/// It reads the bytes directly rather than through a [`Reader`], as this
/// loop runs once per token of every sheet the flight reads; each failed
/// read is an [`GraphicsError::UnexpectedEnd`] at the read's start, as a
/// [`Reader`]'s is.
fn walk(
    data: &[u8],
    start: usize,
    header: &Header,
    units: Units,
    sink: &mut impl Sink,
) -> Result<(), GraphicsError> {
    let height = header.height;
    let mut pos = start;
    let mut ended: u32 = 0;
    let mut lines = 0;
    let mut x = 0;
    // Every token is four bytes, so the data runs out first.
    for _ in 0..data.len() {
        let offset = pos;
        let Some(&[op, count_high, count_mid, count_low]) = data.get(pos..pos + 4) else {
            return Err(GraphicsError::UnexpectedEnd { offset });
        };
        pos += 4;
        let token = u32::from_be_bytes([op, count_high, count_mid, count_low]);
        let len = (token & COUNT_MASK) / units.size;
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
                if op == PIXELS && !(token & COUNT_MASK).is_multiple_of(units.size) {
                    return Err(GraphicsError::OddByteCount { offset });
                }
                if units.size > 1 && op == PIXELS && len == 0 {
                    return Err(GraphicsError::RunawaySwap { offset });
                }
                if x + len > units.row_len {
                    return Err(GraphicsError::LineOverflow {
                        frame: ended,
                        line: y,
                        offset,
                    });
                }
                match op {
                    PIXELS => {
                        // `len` is within the row, so this cannot
                        // overflow.
                        let count = (units.size * len) as usize;
                        let Some(pixels) = data.get(pos..pos + count) else {
                            return Err(GraphicsError::UnexpectedEnd { offset: pos });
                        };
                        pos += count;
                        // Padded to four bytes.
                        let padded = pos + pad4(count);
                        if padded > data.len() {
                            return Err(GraphicsError::UnexpectedEnd { offset: pos });
                        }
                        pos = padded;
                        sink.pixels(x, y, pixels);
                    }
                    RUN => {
                        let Some(&[b0, b1, b2, b3]) = data.get(pos..pos + 4) else {
                            return Err(GraphicsError::UnexpectedEnd { offset: pos });
                        };
                        pos += 4;
                        sink.run(x, y, len, u32::from_be_bytes([b0, b1, b2, b3]));
                    }
                    _ => {}
                }
                x += len;
            }
            token => return Err(GraphicsError::UnsupportedToken { token, offset }),
        }
    }
    Err(GraphicsError::UnexpectedEnd { offset: pos })
}

/// A sink that paints each frame of a 16-bit sheet, and each frame's runs
/// alone.
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

    fn run(&mut self, x: u32, y: u32, len: u32, word: u32) {
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

    fn run(&mut self, _: u32, _: u32, _: u32, _: u32) {
        self.has_runs = true;
    }

    fn frame_end(&mut self) {}
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
    use crate::graphics::rled32::ThirtyTwoBitReading;

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
            one(RledBuilder::new(2, 2).depth(0x8000)),
            Err(GraphicsError::UnsupportedDepth { depth: 0x8000 })
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
    fn a_32_bit_sheet_decodes_as_drawn_unless_asked_otherwise() {
        let bytes = RledBuilder::new(1, 1)
            .depth(32)
            .count_lines()
            .frame(|f| f.line().pixels32(&[0x0000_7C00]))
            .build();
        let as_drawn = RledReadings {
            thirty_two_bit: ThirtyTwoBitReading::AsDrawn,
            ..RledReadings::default()
        };
        let drawn = decode_rled_with(&bytes, None, as_drawn).unwrap();
        assert_eq!(decode(&bytes), Ok(drawn.clone()));
        assert_eq!(
            decode_rled_as(&bytes, None, EightBitReading::SystemPalette),
            Ok(drawn.clone())
        );
        assert_eq!(drawn.image_width(), 3);
        let true_colour = RledReadings {
            thirty_two_bit: ThirtyTwoBitReading::TrueColour,
            ..RledReadings::default()
        };
        let three = SheetLayout::new(3).unwrap();
        let sheet = decode_rled_with(&bytes, Some(three), true_colour).unwrap();
        // In images the size of the drawn sheet's.
        assert_eq!((sheet.image_width(), sheet.layout()), (3, three));
        assert_ne!(sheet.frames(), drawn.frames());
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
        // A depth of 7 or less is unsupported.
        let bytes = RledBuilder::new(2, 1).depth(1).frame(|f| f).build();
        assert_eq!(
            decode_rled_as(&bytes, None, EightBitReading::SystemPalette),
            Err(GraphicsError::UnsupportedDepth { depth: 1 })
        );
    }

    /// One-pixel sheets of depth 16, 8 and 32.
    fn sheets_of_each_depth() -> [(u16, Vec<u8>); 3] {
        [
            (
                16,
                RledBuilder::new(1, 1)
                    .frame(|f| f.line().pixels(&[RED]))
                    .build(),
            ),
            (
                8,
                RledBuilder::new(2, 1)
                    .depth(8)
                    .frame(|f| f.line().pixels8(&[0x00, 0x7C]))
                    .build(),
            ),
            (
                32,
                RledBuilder::new(1, 1)
                    .depth(32)
                    .count_lines()
                    .frame(|f| f.line().pixels32(&[0x0000_7C00]))
                    .build(),
            ),
        ]
    }

    #[test]
    fn a_sheet_reports_its_header_depth_under_either_reading() {
        for (depth, bytes) in sheets_of_each_depth() {
            for reading in [EightBitReading::AsDrawn, EightBitReading::SystemPalette] {
                let sheet = decode_rled_as(&bytes, None, reading).unwrap();
                assert_eq!(sheet.depth(), depth, "{depth} {reading:?}");
            }
            assert_eq!(decode(&bytes).unwrap().depth(), depth);
        }
    }

    #[test]
    fn a_scan_reports_the_header_depth() {
        for (depth, bytes) in sheets_of_each_depth() {
            assert_eq!(scan_rled(&bytes).map(|info| info.depth), Ok(depth));
        }
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
        // Cut just past the padding: the next token is what is missing.
        assert_eq!(
            decode(&bytes[..32]),
            Err(GraphicsError::UnexpectedEnd { offset: 32 })
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

    /// `_BlitPixieRLESwap` swaps `count / 2` words from 1 up, so a pixel
    /// token of no bytes runs it away (@0xbe460-0xbe46a).
    #[test]
    fn a_16_bit_pixel_token_of_no_bytes_is_an_error() {
        let bytes = RledBuilder::new(2, 1)
            .frame(|f| f.line().token(2, 0).pixels(&[RED]))
            .build();
        assert_eq!(
            decode(&bytes),
            Err(GraphicsError::RunawaySwap { offset: 20 })
        );
        assert_eq!(
            scan_rled(&bytes),
            Err(GraphicsError::RunawaySwap { offset: 20 })
        );
        // An 8-bit sheet steps over its bytes instead: none is nothing.
        let bytes = RledBuilder::new(2, 1)
            .depth(8)
            .frame(|f| f.line().token(2, 0))
            .build();
        assert!(decode(&bytes).is_ok());
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
            depth: sheet.depth(),
            image_width: sheet.image_width(),
            image_height: sheet.image_height(),
            clip_quirks: sheet.clip_quirks(),
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
            depth: 16,
            image_width: 3,
            image_height: 2,
            clip_quirks: false,
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
                depth: 16,
                image_width: 4,
                image_height: 1,
                clip_quirks: false,
            })
        );
    }

    /// A 32-bit sheet whose image overhangs its box, a 16-bit sheet whose
    /// two-colour run is a clip quirk, and a plain 16-bit sheet.
    fn sheets_of_each_edge() -> [(&'static str, Vec<u8>); 3] {
        [
            (
                "overhanging",
                RledBuilder::new(1, 1)
                    .depth(32)
                    .count_lines()
                    .frame(|f| f.line().pixels32(&[0x0000_7C00]))
                    .build(),
            ),
            (
                "quirky",
                RledBuilder::new(4, 1)
                    .count_lines()
                    .frame(|f| f.line().run(4, RED, BLUE))
                    .build(),
            ),
            (
                "plain",
                RledBuilder::new(2, 1)
                    .count_lines()
                    .frame(|f| f.line().pixels(&[RED, GREEN]))
                    .build(),
            ),
        ]
    }

    #[test]
    fn a_scan_reads_the_image_size_and_clip_quirks_as_the_decode_has_them() {
        let [overhanging, quirky, plain] = sheets_of_each_edge();
        for (name, bytes) in [&overhanging, &quirky, &plain] {
            assert_eq!(
                scan_rled(bytes),
                decode(bytes).map(|sheet| info_of(&sheet)),
                "{name}"
            );
        }
        let overhanging = scan_rled(&overhanging.1).unwrap();
        assert_ne!(
            (overhanging.image_width, overhanging.image_height),
            (overhanging.frame_width, overhanging.frame_height),
            "the 32-bit sheet overhangs"
        );
        assert!(!overhanging.clip_quirks);
        assert!(
            scan_rled(&quirky.1).unwrap().clip_quirks,
            "a two-colour run"
        );
        let plain = scan_rled(&plain.1).unwrap();
        assert_eq!((plain.image_width, plain.image_height), (2, 1));
        assert!(!plain.clip_quirks);
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
                RledBuilder::new(2, 2).depth(7).frame(|f| f).build(),
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
                "runaway swap",
                RledBuilder::new(3, 1)
                    .frame(|f| f.line().token(2, 0))
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

    /// `_SWCreateSpriteFromRLEResource` (@0xa9990) reads only the header,
    /// so a sheet whose tokens are corrupt still has its header's frame.
    #[test]
    fn the_frame_is_the_headers_size_without_reading_a_token() {
        let bytes = RledBuilder::new(30, 50)
            .frame(|f| f.line().token(5, 0))
            .build();
        assert!(scan_rled(&bytes).is_err());
        assert_eq!(
            rled_frame(&bytes),
            Ok(RledFrameSize {
                width: 30,
                height: 50
            })
        );
    }

    #[test]
    fn the_frame_checks_the_header_as_decode_does() {
        let cases: Vec<(&str, Vec<u8>)> = vec![
            (
                "bad depth",
                RledBuilder::new(2, 2).depth(7).frame(|f| f).build(),
            ),
            ("zero width", RledBuilder::new(0, 2).frame(|f| f).build()),
            ("zero height", RledBuilder::new(2, 0).frame(|f| f).build()),
            ("no frames", RledBuilder::new(2, 2).frame_count(0).build()),
            (
                "over budget",
                RledBuilder::new(65535, 65535).frame_count(65535).build(),
            ),
            (
                "truncated header",
                RledBuilder::new(2, 2).build()[..15].to_vec(),
            ),
        ];
        for (name, bytes) in cases {
            let decoded = decode(&bytes).map(|_| ()).unwrap_err();
            assert_eq!(rled_frame(&bytes), Err(decoded), "{name}");
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
