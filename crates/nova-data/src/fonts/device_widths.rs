//! A face's device widths: the whole-pixel advance of each glyph at the
//! sizes its `hdmx` (horizontal device metrics) table records.
//!
//! With fractional widths off, as EV Nova leaves them, QuickDraw advances
//! each glyph by the TrueType scaler's hinted device width, which `hdmx`
//! caches per ppem. [`DeviceWidths`] reads that table so the renderer can
//! advance glyphs by the same widths. A face with no `hdmx`, or one that
//! does not parse, has none.

use ttf_parser::{RawFace, Tag};

/// The `hdmx` header: version, record count and record size.
const HEADER: usize = 8;

/// The per-ppem device widths of one face, read from its `hdmx` table.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeviceWidths {
    /// Each record's ppem and its widths, one byte per glyph, in table
    /// order.
    records: Vec<(u8, Vec<u8>)>,
}

impl DeviceWidths {
    /// The device widths in the font `bytes`: none when it has no `hdmx`,
    /// or when that table (or the `maxp` glyph count it is read by) is
    /// cut short or does not parse.
    #[must_use]
    pub fn read(bytes: &[u8]) -> Self {
        Self::parse(bytes).unwrap_or_default()
    }

    /// The table's records, or `None` when anything is missing or out of
    /// bounds.
    fn parse(bytes: &[u8]) -> Option<Self> {
        let face = RawFace::parse(bytes, 0).ok()?;
        let maxp = face.table(Tag::from_bytes(b"maxp"))?;
        let glyphs = usize::from(u16::from_be_bytes(maxp.get(4..6)?.try_into().ok()?));
        let hdmx = face.table(Tag::from_bytes(b"hdmx"))?;
        let header = hdmx.get(..HEADER)?;
        let count = usize::try_from(i16::from_be_bytes([header[2], header[3]])).ok()?;
        let size = usize::try_from(u32::from_be_bytes(header[4..8].try_into().ok()?)).ok()?;
        if size < 2 + glyphs {
            return None;
        }
        let body = hdmx.get(HEADER..HEADER.checked_add(count.checked_mul(size)?)?)?;
        let records = body
            .chunks_exact(size)
            .map(|record| (record[0], record[2..2 + glyphs].to_vec()))
            .collect();
        Some(Self { records })
    }

    /// Glyph `glyph`'s device width at `ppem`, if the table has a record
    /// for that size and the glyph is in it.
    #[must_use]
    pub fn width(&self, ppem: u8, glyph: u16) -> Option<u8> {
        let (_, widths) = self.records.iter().find(|(size, _)| *size == ppem)?;
        widths.get(usize::from(glyph)).copied()
    }

    /// The ppems the table has records for, in table order.
    pub fn ppems(&self) -> impl Iterator<Item = u8> + '_ {
        self.records.iter().map(|(ppem, _)| *ppem)
    }

    /// Whether the table recorded no size at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixture::{block_sfnt, hdmx};
    use super::*;

    /// The block font (two glyphs) with `table` as its `hdmx`.
    fn with_hdmx(table: Vec<u8>) -> Vec<u8> {
        block_sfnt("Charcoal").table(b"hdmx", table).build()
    }

    #[test]
    fn a_records_widths_read_back_by_ppem_and_glyph() {
        let widths = DeviceWidths::read(&with_hdmx(hdmx(2, &[(10, &[0, 11]), (12, &[3, 13])])));
        assert_eq!(widths.width(10, 1), Some(11));
        assert_eq!(widths.width(10, 0), Some(0));
        assert_eq!(widths.width(12, 0), Some(3));
        assert_eq!(widths.width(12, 1), Some(13));
        assert_eq!(widths.ppems().collect::<Vec<_>>(), [10, 12]);
        assert!(!widths.is_empty());
    }

    #[test]
    fn a_ppem_or_glyph_with_no_record_has_no_width() {
        let widths = DeviceWidths::read(&with_hdmx(hdmx(2, &[(10, &[0, 11])])));
        assert_eq!(widths.width(9, 1), None);
        assert_eq!(widths.width(11, 1), None);
        assert_eq!(widths.width(10, 2), None);
    }

    #[test]
    fn the_first_record_for_a_ppem_wins() {
        let widths = DeviceWidths::read(&with_hdmx(hdmx(2, &[(10, &[0, 11]), (10, &[0, 7])])));
        assert_eq!(widths.width(10, 1), Some(11));
    }

    #[test]
    fn a_font_with_no_hdmx_has_none() {
        let widths = DeviceWidths::read(&block_sfnt("Charcoal").build());
        assert_eq!(widths, DeviceWidths::default());
        assert!(widths.is_empty());
        assert_eq!(widths.width(10, 1), None);
    }

    #[test]
    fn bytes_that_are_not_a_font_have_none() {
        assert!(DeviceWidths::read(b"not a font").is_empty());
        assert!(DeviceWidths::read(&[]).is_empty());
    }

    #[test]
    fn a_truncated_table_has_none() {
        let table = hdmx(2, &[(10, &[0, 11]), (12, &[3, 13])]);
        for len in [0, 4, 7, 8, 11, table.len() - 1] {
            let widths = DeviceWidths::read(&with_hdmx(table[..len].to_vec()));
            assert!(widths.is_empty(), "{len} bytes: {widths:?}");
        }
    }

    /// `table` with its record size set to `size`.
    fn sized(mut table: Vec<u8>, size: u32) -> Vec<u8> {
        table[4..8].copy_from_slice(&size.to_be_bytes());
        table
    }

    #[test]
    fn a_record_size_past_the_end_has_none() {
        let table = hdmx(2, &[(10, &[0, 11])]);
        for size in [5, 8, 64, u32::MAX] {
            let widths = DeviceWidths::read(&with_hdmx(sized(table.clone(), size)));
            assert!(widths.is_empty(), "size {size}: {widths:?}");
        }
    }

    #[test]
    fn a_record_too_small_for_every_glyph_has_none() {
        // Two glyphs need 2 + 2 bytes a record; 3 leaves the second out.
        let table = sized(hdmx(2, &[(10, &[0, 11])]), 3);
        assert!(DeviceWidths::read(&with_hdmx(table)).is_empty());
    }

    #[test]
    fn a_negative_record_count_has_none() {
        let mut table = hdmx(2, &[(10, &[0, 11])]);
        table[2..4].copy_from_slice(&(-1i16).to_be_bytes());
        assert!(DeviceWidths::read(&with_hdmx(table)).is_empty());
    }

    #[test]
    fn a_maxp_too_short_for_its_glyph_count_has_none() {
        let bytes = block_sfnt("Charcoal")
            .table(b"hdmx", hdmx(2, &[(10, &[0, 11])]))
            .table(b"maxp", [0, 1, 0, 0, 0])
            .build();
        assert!(DeviceWidths::read(&bytes).is_empty());
    }

    #[test]
    fn a_record_holds_one_width_per_glyph_the_maxp_counts() {
        // Five glyphs (a `maxp` version 0.5): records of 2 + 5 bytes,
        // padded to 8.
        let bytes = block_sfnt("Charcoal")
            .table(b"maxp", [0, 0, 0x50, 0, 0, 5])
            .table(
                b"hdmx",
                hdmx(5, &[(10, &[1, 2, 3, 4, 5]), (12, &[6, 7, 8, 9, 10])]),
            )
            .build();
        let widths = DeviceWidths::read(&bytes);
        assert_eq!(widths.width(10, 4), Some(5));
        assert_eq!(widths.width(12, 4), Some(10));
        assert_eq!(widths.width(12, 5), None, "past the glyphs");
    }

    #[test]
    fn records_are_read_by_the_record_size_not_the_glyph_count() {
        // A wider record than the glyphs need: the padding is skipped.
        let mut table = hdmx(2, &[]);
        table[2..4].copy_from_slice(&2u16.to_be_bytes());
        table = sized(table, 8);
        table.extend([10, 11, 0, 11, 9, 9, 9, 9]);
        table.extend([12, 13, 3, 13, 9, 9, 9, 9]);
        let widths = DeviceWidths::read(&with_hdmx(table));
        assert_eq!(widths.width(12, 0), Some(3));
        assert_eq!(widths.ppems().collect::<Vec<_>>(), [10, 12]);
    }
}
