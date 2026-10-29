//! Colour conversion: 16-bit direct pixels, colour tables and indexed rows.
//!
//! Source: *Inside Macintosh: Imaging With QuickDraw* (1994) ch. 4, "Color
//! QuickDraw": the 16-bit direct pixel (`xRRRRRGGGGGBBBBB`), the
//! `ColorTable` record (`ctSeed`, `ctFlags`, `ctSize` = entries minus one,
//! then `ColorSpec` entries of `value`, red, green, blue as 16-bit
//! components) and indexed pixels packed leftmost-first into bytes.

use super::GraphicsError;
use super::reader::Reader;

/// `ctFlags` bit 15: a device colour table, indexed by entry position.
const DEVICE_FLAG: u16 = 0x8000;

/// An opaque RGBA8 pixel from a 16-bit `xRRRRRGGGGGBBBBB` value; the high
/// bit is ignored and each 5-bit component is widened by bit replication.
pub(crate) fn rgb555(value: u16) -> [u8; 4] {
    let widen = |shift: u16| {
        let five = ((value >> shift) & 0x1F) as u8;
        // The two halves share no bits, so `+` is the same as `|`.
        (five << 3) + (five >> 2)
    };
    [widen(10), widen(5), widen(0), 255]
}

/// The 16-bit `xRRRRRGGGGGBBBBB` value a decoded pixel lands as in a
/// 16-bit QuickDraw world: each channel's top five bits. A clear pixel
/// (alpha 0), which no picture opcode covered, is the white the world was
/// erased to.
pub(crate) fn to_rgb555(rgba: [u8; 4]) -> u16 {
    if rgba[3] == 0 {
        return 0x7FFF;
    }
    let [r, g, b, _] = rgba.map(|c| u16::from(c >> 3));
    // The fields share no bits, so `+` is the same as `|`.
    (r << 10) + (g << 5) + b
}

/// A colour table, indexed by the pixel values it maps.
#[derive(Clone, Debug)]
pub(crate) struct ColorTable {
    colours: [Option<[u8; 3]>; 256],
}

impl ColorTable {
    /// Reads a `ColorTable` record.
    pub(crate) fn read(r: &mut Reader<'_>) -> Result<Self, GraphicsError> {
        r.skip(4)?; // ctSeed
        let device = r.u16()? & DEVICE_FLAG != 0;
        let count = usize::from(r.u16()?) + 1;
        let entries = r.bytes(count * 8)?;
        let mut colours = [None; 256];
        for (position, entry) in entries.as_chunks::<8>().0.iter().enumerate() {
            let word = |i: usize| u16::from_be_bytes([entry[i], entry[i + 1]]);
            let key = if device {
                position
            } else {
                usize::from(word(0))
            };
            if let Some(slot @ None) = colours.get_mut(key) {
                *slot = Some([word(2), word(4), word(6)].map(|c| (c >> 8) as u8));
            }
        }
        Ok(Self { colours })
    }

    /// The colours of a 1-bit `BitMap` copied in srcCopy with picture
    /// playback's default pen: a clear bit (0) is the white background, a
    /// set bit (1) the black foreground.
    pub(crate) fn black_on_white() -> Self {
        let mut colours = [None; 256];
        colours[0] = Some([255; 3]);
        colours[1] = Some([0; 3]);
        Self { colours }
    }

    /// The opaque colour for pixel value `index`.
    pub(crate) fn rgba(&self, index: u8) -> Result<[u8; 4], GraphicsError> {
        let [r, g, b] =
            self.colours[usize::from(index)].ok_or(GraphicsError::MissingColour { index })?;
        Ok([r, g, b, 255])
    }
}

/// The first `width` pixel values of an indexed row at `bits` (1, 2, 4 or
/// 8) per pixel, or `None` if the row is too short.
pub(crate) fn indices(row: &[u8], bits: u16, width: usize) -> Option<Vec<u8>> {
    let bits = usize::from(bits);
    let mask = (1u16 << bits) - 1;
    (0..width)
        .map(|x| {
            let bit = x * bits;
            let byte = u16::from(*row.get(bit / 8)?);
            let shift = 8 - bits - bit % 8;
            Some(((byte >> shift) & mask) as u8)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::fixture::Ctab;

    #[test]
    fn rgb555_replicates_bits_and_ignores_the_high_bit() {
        assert_eq!(rgb555(0x0000), [0, 0, 0, 255]);
        assert_eq!(rgb555(0x8000), [0, 0, 0, 255]);
        assert_eq!(rgb555(0x7FFF), [255, 255, 255, 255]);
        assert_eq!(rgb555(0xFFFF), [255, 255, 255, 255]);
        assert_eq!(rgb555(0x7C00), [255, 0, 0, 255]);
        assert_eq!(rgb555(0x03E0), [0, 255, 0, 255]);
        assert_eq!(rgb555(0x001F), [0, 0, 255, 255]);
        // 16 widens to 0b1000_0100; 1, 2, 3 to 8, 16, 24.
        assert_eq!(rgb555(0x4210), [132, 132, 132, 255]);
        assert_eq!(rgb555((1 << 10) | (2 << 5) | 0b11), [8, 16, 24, 255]);
        assert_eq!(rgb555((5 << 10) | (9 << 5) | 0b1_1110), [41, 74, 247, 255]);
    }

    #[test]
    fn to_rgb555_keeps_each_channels_top_five_bits() {
        assert_eq!(to_rgb555([0, 0, 0, 255]), 0x0000);
        assert_eq!(to_rgb555([0xFF, 0xFF, 0xFF, 255]), 0x7FFF);
        assert_eq!(to_rgb555([0xF8, 0xF8, 0xF8, 255]), 0x7FFF);
        assert_eq!(to_rgb555([0xF7, 0xFF, 0xFF, 255]), 0x7BFF);
        assert_eq!(to_rgb555([0xFF, 0, 0, 255]), 0x7C00);
        assert_eq!(to_rgb555([0, 0xFF, 0, 255]), 0x03E0);
        assert_eq!(to_rgb555([0, 0, 0xFF, 255]), 0x001F);
        assert_eq!(
            to_rgb555([0x12, 0x34, 0x56, 255]),
            (0b10 << 10) | (0b110 << 5) | 0b1010
        );
        // Partly clear pixels keep their colour; fully clear ones are white.
        assert_eq!(
            to_rgb555([0x08, 0x10, 0x18, 1]),
            (0b1 << 10) | (0b10 << 5) | 0b11
        );
        assert_eq!(to_rgb555([0, 0, 0, 0]), 0x7FFF);
        // Round trips through rgb555.
        for value in [0x0000, 0x7FFF, 0x1234, 0x7C1F] {
            assert_eq!(to_rgb555(rgb555(value)), value);
        }
    }

    fn table(bytes: &[u8]) -> Result<ColorTable, GraphicsError> {
        ColorTable::read(&mut Reader::new(bytes))
    }

    /// Two sparse entries, written out by hand: value 7 is dark red, value 2
    /// is mid blue.
    const SPARSE: [u8; 24] = [
        0, 0, 0, 0, // ctSeed
        0, 0, // ctFlags
        0, 1, // ctSize: two entries
        0, 7, 0x80, 0x00, 0x00, 0xFF, 0x00, 0x00, // value 7
        0, 2, 0x00, 0x00, 0x12, 0x34, 0xAB, 0xCD, // value 2
    ];

    #[test]
    fn colours_are_looked_up_by_value_using_the_high_byte() {
        let ctab = table(&SPARSE).unwrap();
        assert_eq!(ctab.rgba(7), Ok([0x80, 0x00, 0x00, 255]));
        assert_eq!(ctab.rgba(2), Ok([0x00, 0x12, 0xAB, 255]));
        assert_eq!(ctab.rgba(0), Err(GraphicsError::MissingColour { index: 0 }));
        assert_eq!(ctab.rgba(1), Err(GraphicsError::MissingColour { index: 1 }));
    }

    #[test]
    fn a_bitmap_draws_clear_bits_white_and_set_bits_black() {
        let ctab = ColorTable::black_on_white();
        assert_eq!(ctab.rgba(0), Ok([255, 255, 255, 255]));
        assert_eq!(ctab.rgba(1), Ok([0, 0, 0, 255]));
        assert_eq!(ctab.rgba(2), Err(GraphicsError::MissingColour { index: 2 }));
    }

    #[test]
    fn read_consumes_exactly_the_table() {
        let mut bytes = SPARSE.to_vec();
        bytes.push(0xEE);
        let mut r = Reader::new(&bytes);
        ColorTable::read(&mut r).unwrap();
        assert_eq!(r.pos(), 24);
    }

    #[test]
    fn device_tables_are_looked_up_by_position() {
        let mut bytes = SPARSE;
        bytes[4] = 0x80;
        let ctab = table(&bytes).unwrap();
        assert_eq!(ctab.rgba(0), Ok([0x80, 0x00, 0x00, 255]));
        assert_eq!(ctab.rgba(1), Ok([0x00, 0x12, 0xAB, 255]));
        assert_eq!(ctab.rgba(7), Err(GraphicsError::MissingColour { index: 7 }));
        // Other flag bits do not make a device table.
        bytes[4] = 0x40;
        bytes[5] = 0xFF;
        assert_eq!(table(&bytes).unwrap().rgba(7), Ok([0x80, 0, 0, 255]));
    }

    #[test]
    fn the_first_entry_for_a_value_wins_and_values_past_255_are_unused() {
        let ctab = Ctab {
            device: false,
            entries: vec![
                (3, [0x1100, 0x2200, 0x3300]),
                (3, [0xFF00, 0xFF00, 0xFF00]),
                (256 + 4, [0x4400, 0x4400, 0x4400]),
                (255, [0x0100, 0x0200, 0x0300]),
            ],
        };
        let parsed = table(&ctab.bytes()).unwrap();
        assert_eq!(parsed.rgba(3), Ok([0x11, 0x22, 0x33, 255]));
        assert_eq!(
            parsed.rgba(4),
            Err(GraphicsError::MissingColour { index: 4 })
        );
        assert_eq!(parsed.rgba(255), Ok([1, 2, 3, 255]));
    }

    #[test]
    fn a_truncated_table_is_an_unexpected_end() {
        for len in 0..SPARSE.len() {
            assert!(
                matches!(
                    table(&SPARSE[..len]),
                    Err(GraphicsError::UnexpectedEnd { .. })
                ),
                "{len}"
            );
        }
        // The entries are read in one piece, starting after the header.
        assert_eq!(
            table(&SPARSE[..20]).unwrap_err(),
            GraphicsError::UnexpectedEnd { offset: 8 }
        );
        // ctSize 0xFFFF claims 65536 entries.
        let mut huge = SPARSE;
        huge[6..8].copy_from_slice(&[0xFF, 0xFF]);
        assert_eq!(
            table(&huge).unwrap_err(),
            GraphicsError::UnexpectedEnd { offset: 8 }
        );
    }

    #[test]
    fn the_fixture_writes_the_table_layout() {
        let ctab = Ctab {
            device: false,
            entries: vec![(7, [0x8000, 0x00FF, 0]), (2, [0, 0x1234, 0xABCD])],
        };
        assert_eq!(ctab.bytes(), SPARSE);
        let device = Ctab {
            device: true,
            ..ctab
        };
        assert_eq!(&device.bytes()[4..6], [0x80, 0]);
    }

    #[test]
    fn indexed_rows_unpack_leftmost_pixel_first() {
        assert_eq!(
            indices(&[0b1011_0001], 1, 8),
            Some(vec![1, 0, 1, 1, 0, 0, 0, 1])
        );
        assert_eq!(
            indices(&[0b1011_0001, 0x80], 1, 9),
            Some(vec![1, 0, 1, 1, 0, 0, 0, 1, 1])
        );
        assert_eq!(
            indices(&[0b1110_0100, 0b0100_0000], 2, 5),
            Some(vec![3, 2, 1, 0, 1])
        );
        assert_eq!(indices(&[0xA5, 0x3C], 4, 3), Some(vec![0xA, 0x5, 0x3]));
        assert_eq!(indices(&[9, 200, 255], 8, 3), Some(vec![9, 200, 255]));
        assert_eq!(indices(&[9, 200, 255], 8, 2), Some(vec![9, 200]));
        assert_eq!(indices(&[], 8, 0), Some(Vec::new()));
    }

    #[test]
    fn a_short_indexed_row_is_none() {
        assert_eq!(indices(&[0xFF], 1, 9), None);
        assert_eq!(indices(&[0xFF], 4, 3), None);
        assert_eq!(indices(&[1, 2], 8, 3), None);
    }
}
