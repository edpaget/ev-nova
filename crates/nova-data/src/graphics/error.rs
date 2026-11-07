//! Why a picture or sprite failed to decode.

/// A decoding failure. Offsets are bytes from the start of the resource.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GraphicsError {
    /// The data ended in the middle of a read that began at `offset`.
    #[error("unexpected end of data at byte {offset:#x}")]
    UnexpectedEnd {
        /// Where the failing read began.
        offset: usize,
    },
    /// A width or height is zero or negative.
    #[error("bad image dimensions {width}x{height}")]
    BadDimensions {
        /// The width the header gives.
        width: i32,
        /// The height the header gives.
        height: i32,
    },
    /// The image would decode to more pixels than the budget allows, either
    /// absolutely or for the size of its input.
    #[error("{pixels} pixels is too large to decode from {input_len} bytes")]
    TooLarge {
        /// Pixels the header claims.
        pixels: u64,
        /// Length of the input.
        input_len: usize,
    },
    /// A `rowBytes` field without the `PixMap` flag (bit 15), a plain
    /// `BitMap`, where only a `PixMap` will do: a `PICT`'s direct-pixel
    /// opcodes, a `cicn` or a `ppat`. (The `PICT` opcodes that copy indexed
    /// pixels accept a 1-bit `BitMap`.)
    #[error("expected a PixMap at byte {offset:#x}, found a BitMap")]
    NotAPixMap {
        /// Where the `rowBytes` field starts.
        offset: usize,
    },
    /// A `PICT` of neither version 1 (`0x11 0x01` at offset 10) nor version
    /// 2 (`0x0011 0x02FF`).
    #[error("unsupported PICT version at byte {offset:#x}: only versions 1 and 2 are decoded")]
    UnsupportedVersion {
        /// Where the version opcode starts.
        offset: usize,
    },
    /// A `PICT` opcode this decoder does not implement.
    #[error("unsupported PICT opcode {opcode:#06x} at byte {offset:#x}")]
    UnsupportedOpcode {
        /// The opcode.
        opcode: u16,
        /// Where the opcode starts.
        offset: usize,
    },
    /// A malformed region: its size is too small to hold its bounding
    /// rectangle or runs past the data, or its inversion points are odd in
    /// number on a line, out of order or unterminated.
    #[error("bad region of size {size} at byte {offset:#x}")]
    BadRegion {
        /// The region's size field.
        size: u16,
        /// Where the region starts.
        offset: usize,
    },
    /// A `PixMap` whose pixel format this decoder does not implement.
    #[error(
        "unsupported PixMap: pixelSize {pixel_size}, packType {pack_type}, cmpCount {cmp_count}"
    )]
    UnsupportedPixMap {
        /// `pixelSize`.
        pixel_size: u16,
        /// `packType`.
        pack_type: u16,
        /// `cmpCount`.
        cmp_count: u16,
    },
    /// A `rowBytes` too small to hold one row of pixels.
    #[error("rowBytes {row_bytes} cannot hold a row of {needed} bytes")]
    BadRowBytes {
        /// `rowBytes`, without its flag bits.
        row_bytes: u16,
        /// Bytes one row of pixels needs.
        needed: u32,
    },
    /// A transfer mode other than srcCopy (0) or ditherCopy (64).
    #[error("unsupported transfer mode {mode}")]
    UnsupportedTransferMode {
        /// The mode.
        mode: u16,
    },
    /// A copy whose source leaves the pixel map, whose destination leaves
    /// the picture frame, or which would scale.
    #[error("bad copy rectangles at byte {offset:#x}")]
    BadCopyRect {
        /// Where the source rectangle starts.
        offset: usize,
    },
    /// A packed row that does not unpack to exactly one row.
    #[error("bad packed row {row} at byte {offset:#x}")]
    BadPackedRow {
        /// The row, counting from 0.
        row: u32,
        /// Where the row (its byte count) starts.
        offset: usize,
    },
    /// A `cicn` whose mask bounds differ from its `PixMap` bounds.
    #[error("the icon mask's bounds differ from its PixMap's")]
    MaskMismatch,
    /// A `ppat` other than a full-colour pixel pattern (`patType` 1).
    #[error("unsupported ppat pattern type {pat_type}")]
    UnsupportedPatternType {
        /// `patType`.
        pat_type: u16,
    },
    /// An `rlëD` sheet whose depth the original's draw procs assert on: 7
    /// or less, read as a signed word (`_BlitPixieRLETranslucentDrawProc`
    /// @0xbb46e-0xbb476), so 0x8000 and up as well. The assert quits the
    /// game, so there is nothing to draw.
    #[error("unsupported rlëD depth {depth}")]
    UnsupportedDepth {
        /// The header's depth.
        depth: u16,
    },
    /// An `rlëD` sheet whose header claims no frames, or a `PICT` sprite
    /// grid smaller than one frame.
    #[error("the sprite sheet has no frames")]
    NoFrames,
    /// A `PICT` sprite grid whose width or height is not a whole number of
    /// frames.
    #[error(
        "the {width}x{height} picture is not a whole grid of {frame_width}x{frame_height} frames"
    )]
    PartialGrid {
        /// The picture's width.
        width: u32,
        /// The picture's height.
        height: u32,
        /// The width of each frame.
        frame_width: u32,
        /// The height of each frame.
        frame_height: u32,
    },
    /// An `rlëD` token this decoder does not know.
    #[error("unsupported rlëD token {token} at byte {offset:#x}")]
    UnsupportedToken {
        /// The token's opcode (its top byte).
        token: u8,
        /// Where the token starts.
        offset: usize,
    },
    /// An `rlëD` drawing token before the frame's first line start.
    #[error("rlëD drawing token outside a line at byte {offset:#x}")]
    TokenOutsideLine {
        /// Where the token starts.
        offset: usize,
    },
    /// An `rlëD` token that draws past the end of its line.
    #[error("rlëD frame {frame} line {line} overflows at byte {offset:#x}")]
    LineOverflow {
        /// The frame, counting from 0.
        frame: u32,
        /// The line, counting from 0.
        line: u32,
        /// Where the token starts.
        offset: usize,
    },
    /// An `rlëD` frame with more line starts than the frame height.
    #[error("rlëD frame {frame} has too many lines at byte {offset:#x}")]
    TooManyLines {
        /// The frame, counting from 0.
        frame: u32,
        /// Where the extra line start is.
        offset: usize,
    },
    /// `rlëD` pixel data whose byte count is not a whole number of pixels.
    #[error("odd rlëD pixel byte count at byte {offset:#x}")]
    OddByteCount {
        /// Where the token starts.
        offset: usize,
    },
    /// An `rlëD` pixel token too short for `_BlitPixieRLESwap` to swap: at
    /// 16 bits under 2 bytes, at 32 under 4. Its swap loop counts from 1
    /// to the swap count, so a count of 0 runs it on through memory far
    /// past the sheet (@0xbe460-0xbe46a, @0xbe480-0xbe48c), a crash.
    #[error("rlëD pixel token at byte {offset:#x} runs the byte swap away")]
    RunawaySwap {
        /// Where the token starts.
        offset: usize,
    },
    /// An `rlëD` word where `_SWLockSprite`'s walk wants a line start or a
    /// frame end: it asserts (@0xaaa19-0xaaa2e), which quits the game.
    #[error(
        "rlëD frame {frame} has a word at byte {offset:#x} that is neither a line start nor a frame end"
    )]
    NotALineStart {
        /// The frame, counting from 0.
        frame: u32,
        /// Where the word starts.
        offset: usize,
    },
    /// An `rlëD` line start whose count runs past the end of the sheet:
    /// `_SWLockSprite`'s walk asserts (@0xaaa45-0xaaa5b), which quits the
    /// game.
    #[error("rlëD frame {frame}'s line at byte {offset:#x} runs past the end of the sheet")]
    LinePastEnd {
        /// The frame, counting from 0.
        frame: u32,
        /// Where the line start is.
        offset: usize,
    },
    /// A pixel value has no entry in its colour table.
    #[error("pixel value {index} is not in the colour table")]
    MissingColour {
        /// The pixel value.
        index: u8,
    },
    /// A `PICT` sprite grid holding other than the exact number of frames
    /// its cut needs (see `FrameLimit::Exactly` in the `pict_sheet` module),
    /// or a cut asked for zero or fewer.
    #[error("the picture holds {frames} frames, not {expected}")]
    FrameCountMismatch {
        /// The frames the picture's grid holds.
        frames: u32,
        /// The frames the cut needs.
        expected: i32,
    },
    /// A mask picture whose size differs from the picture it masks.
    #[error("the {mask_width}x{mask_height} mask does not fit the {width}x{height} picture")]
    MaskSizeMismatch {
        /// The picture's width.
        width: u32,
        /// The picture's height.
        height: u32,
        /// The mask's width.
        mask_width: u32,
        /// The mask's height.
        mask_height: u32,
    },
}

impl GraphicsError {
    /// The byte offset in the resource where the failure was found, for the
    /// variants that carry one.
    #[must_use]
    pub fn offset(&self) -> Option<usize> {
        match *self {
            Self::UnexpectedEnd { offset }
            | Self::NotAPixMap { offset }
            | Self::UnsupportedVersion { offset }
            | Self::UnsupportedOpcode { offset, .. }
            | Self::BadRegion { offset, .. }
            | Self::BadCopyRect { offset }
            | Self::BadPackedRow { offset, .. }
            | Self::UnsupportedToken { offset, .. }
            | Self::TokenOutsideLine { offset }
            | Self::LineOverflow { offset, .. }
            | Self::TooManyLines { offset, .. }
            | Self::OddByteCount { offset }
            | Self::RunawaySwap { offset }
            | Self::NotALineStart { offset, .. }
            | Self::LinePastEnd { offset, .. } => Some(offset),
            Self::BadDimensions { .. }
            | Self::TooLarge { .. }
            | Self::UnsupportedPixMap { .. }
            | Self::BadRowBytes { .. }
            | Self::UnsupportedTransferMode { .. }
            | Self::MaskMismatch
            | Self::UnsupportedPatternType { .. }
            | Self::UnsupportedDepth { .. }
            | Self::NoFrames
            | Self::PartialGrid { .. }
            | Self::FrameCountMismatch { .. }
            | Self::MissingColour { .. }
            | Self::MaskSizeMismatch { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::too_many_lines)] // one case per variant
    fn offset_is_the_failing_byte_when_the_variant_has_one() {
        use GraphicsError as E;
        let cases = [
            (E::UnexpectedEnd { offset: 1 }, Some(1)),
            (
                E::BadDimensions {
                    width: 0,
                    height: 0,
                },
                None,
            ),
            (
                E::TooLarge {
                    pixels: 9,
                    input_len: 2,
                },
                None,
            ),
            (E::NotAPixMap { offset: 2 }, Some(2)),
            (E::UnsupportedVersion { offset: 3 }, Some(3)),
            (
                E::UnsupportedOpcode {
                    opcode: 0x9B,
                    offset: 4,
                },
                Some(4),
            ),
            (E::BadRegion { size: 4, offset: 5 }, Some(5)),
            (
                E::UnsupportedPixMap {
                    pixel_size: 1,
                    pack_type: 2,
                    cmp_count: 3,
                },
                None,
            ),
            (
                E::BadRowBytes {
                    row_bytes: 1,
                    needed: 2,
                },
                None,
            ),
            (E::UnsupportedTransferMode { mode: 36 }, None),
            (E::BadCopyRect { offset: 6 }, Some(6)),
            (E::BadPackedRow { row: 1, offset: 7 }, Some(7)),
            (E::MaskMismatch, None),
            (E::UnsupportedPatternType { pat_type: 2 }, None),
            (E::UnsupportedDepth { depth: 1 }, None),
            (E::NoFrames, None),
            (
                E::PartialGrid {
                    width: 5,
                    height: 4,
                    frame_width: 2,
                    frame_height: 2,
                },
                None,
            ),
            (
                E::FrameCountMismatch {
                    frames: 6,
                    expected: 36,
                },
                None,
            ),
            (
                E::UnsupportedToken {
                    token: 5,
                    offset: 8,
                },
                Some(8),
            ),
            (E::TokenOutsideLine { offset: 9 }, Some(9)),
            (
                E::LineOverflow {
                    frame: 1,
                    line: 2,
                    offset: 10,
                },
                Some(10),
            ),
            (
                E::TooManyLines {
                    frame: 1,
                    offset: 11,
                },
                Some(11),
            ),
            (E::OddByteCount { offset: 12 }, Some(12)),
            (E::RunawaySwap { offset: 13 }, Some(13)),
            (
                E::NotALineStart {
                    frame: 1,
                    offset: 14,
                },
                Some(14),
            ),
            (
                E::LinePastEnd {
                    frame: 1,
                    offset: 15,
                },
                Some(15),
            ),
            (E::MissingColour { index: 9 }, None),
            (
                E::MaskSizeMismatch {
                    width: 1,
                    height: 2,
                    mask_width: 3,
                    mask_height: 4,
                },
                None,
            ),
        ];
        for (error, offset) in cases {
            assert_eq!(error.offset(), offset, "{error:?}");
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)] // one case per variant
    fn messages_name_the_problem_and_its_offset() {
        let cases = [
            (
                GraphicsError::UnexpectedEnd { offset: 0x2A },
                "unexpected end of data at byte 0x2a",
            ),
            (
                GraphicsError::BadDimensions {
                    width: 0,
                    height: -3,
                },
                "bad image dimensions 0x-3",
            ),
            (
                GraphicsError::TooLarge {
                    pixels: 4096,
                    input_len: 3,
                },
                "4096 pixels is too large to decode from 3 bytes",
            ),
            (
                GraphicsError::NotAPixMap { offset: 0x30 },
                "expected a PixMap at byte 0x30, found a BitMap",
            ),
            (
                GraphicsError::UnsupportedVersion { offset: 10 },
                "unsupported PICT version at byte 0xa: only versions 1 and 2 are decoded",
            ),
            (
                GraphicsError::UnsupportedOpcode {
                    opcode: 0x9B,
                    offset: 0x100,
                },
                "unsupported PICT opcode 0x009b at byte 0x100",
            ),
            (
                GraphicsError::BadRegion { size: 4, offset: 6 },
                "bad region of size 4 at byte 0x6",
            ),
            (
                GraphicsError::UnsupportedPixMap {
                    pixel_size: 32,
                    pack_type: 2,
                    cmp_count: 4,
                },
                "unsupported PixMap: pixelSize 32, packType 2, cmpCount 4",
            ),
            (
                GraphicsError::BadRowBytes {
                    row_bytes: 8,
                    needed: 12,
                },
                "rowBytes 8 cannot hold a row of 12 bytes",
            ),
            (
                GraphicsError::UnsupportedTransferMode { mode: 36 },
                "unsupported transfer mode 36",
            ),
            (
                GraphicsError::BadCopyRect { offset: 0x50 },
                "bad copy rectangles at byte 0x50",
            ),
            (
                GraphicsError::BadPackedRow {
                    row: 3,
                    offset: 0x60,
                },
                "bad packed row 3 at byte 0x60",
            ),
            (
                GraphicsError::MaskMismatch,
                "the icon mask's bounds differ from its PixMap's",
            ),
            (
                GraphicsError::UnsupportedPatternType { pat_type: 2 },
                "unsupported ppat pattern type 2",
            ),
            (
                GraphicsError::UnsupportedDepth { depth: 1 },
                "unsupported rlëD depth 1",
            ),
            (GraphicsError::NoFrames, "the sprite sheet has no frames"),
            (
                GraphicsError::PartialGrid {
                    width: 5,
                    height: 4,
                    frame_width: 2,
                    frame_height: 2,
                },
                "the 5x4 picture is not a whole grid of 2x2 frames",
            ),
            (
                GraphicsError::FrameCountMismatch {
                    frames: 6,
                    expected: 36,
                },
                "the picture holds 6 frames, not 36",
            ),
            (
                GraphicsError::UnsupportedToken {
                    token: 5,
                    offset: 0x14,
                },
                "unsupported rlëD token 5 at byte 0x14",
            ),
            (
                GraphicsError::TokenOutsideLine { offset: 0x10 },
                "rlëD drawing token outside a line at byte 0x10",
            ),
            (
                GraphicsError::LineOverflow {
                    frame: 2,
                    line: 7,
                    offset: 0x40,
                },
                "rlëD frame 2 line 7 overflows at byte 0x40",
            ),
            (
                GraphicsError::TooManyLines {
                    frame: 1,
                    offset: 0x24,
                },
                "rlëD frame 1 has too many lines at byte 0x24",
            ),
            (
                GraphicsError::OddByteCount { offset: 0x18 },
                "odd rlëD pixel byte count at byte 0x18",
            ),
            (
                GraphicsError::RunawaySwap { offset: 0x14 },
                "rlëD pixel token at byte 0x14 runs the byte swap away",
            ),
            (
                GraphicsError::NotALineStart {
                    frame: 0,
                    offset: 0x1C,
                },
                "rlëD frame 0 has a word at byte 0x1c that is neither a line start nor a frame end",
            ),
            (
                GraphicsError::LinePastEnd {
                    frame: 2,
                    offset: 0x30,
                },
                "rlëD frame 2's line at byte 0x30 runs past the end of the sheet",
            ),
            (
                GraphicsError::MissingColour { index: 9 },
                "pixel value 9 is not in the colour table",
            ),
            (
                GraphicsError::MaskSizeMismatch {
                    width: 13,
                    height: 25,
                    mask_width: 12,
                    mask_height: 25,
                },
                "the 12x25 mask does not fit the 13x25 picture",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
