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
    /// A `rowBytes` field without the `PixMap` flag (bit 15): a plain
    /// `BitMap`, which carries no colour.
    #[error("expected a PixMap at byte {offset:#x}, found a BitMap")]
    NotAPixMap {
        /// Where the `rowBytes` field starts.
        offset: usize,
    },
    /// A `PICT` that is not version 2 (offset 10 is not `0x0011 0x02FF`).
    #[error("unsupported PICT version at byte {offset:#x}: only version 2 is decoded")]
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
    /// A region whose size is too small to hold its bounding rectangle.
    #[error("bad region size {size} at byte {offset:#x}")]
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
    /// A pixel value has no entry in its colour table.
    #[error("pixel value {index} is not in the colour table")]
    MissingColour {
        /// The pixel value.
        index: u8,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
                "unsupported PICT version at byte 0xa: only version 2 is decoded",
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
                "bad region size 4 at byte 0x6",
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
                GraphicsError::MissingColour { index: 9 },
                "pixel value 9 is not in the colour table",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
