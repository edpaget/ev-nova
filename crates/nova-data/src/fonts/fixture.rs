//! Synthetic font builders for tests.
//!
//! Available to this crate's tests and, through the `fixture` feature, to
//! other crates' tests. [`SfntBuilder`] writes a table directory around
//! whatever tables it is given, for the [`check_sfnt`](super::check_sfnt)
//! tests; [`block_font`] builds a small TrueType font that really renders,
//! so render tests never need a real font beyond the bundled one.

pub use super::REQUIRED_TABLES as REQUIRED;

/// Writes an sfnt: the scaler tag, the table directory (sorted by tag, as
/// readers binary-search it) and each table's bytes, 4-byte aligned.
#[derive(Clone, Debug)]
pub struct SfntBuilder {
    scaler: [u8; 4],
    tables: Vec<([u8; 4], Vec<u8>)>,
}

impl SfntBuilder {
    /// An sfnt with scaler tag `scaler` and no tables.
    #[must_use]
    pub fn new(scaler: &[u8; 4]) -> Self {
        Self {
            scaler: *scaler,
            tables: Vec::new(),
        }
    }

    /// A TrueType sfnt (`0x00010000`) with every required table and
    /// `glyf`/`loca` outlines, each holding four placeholder bytes.
    #[must_use]
    pub fn truetype() -> Self {
        REQUIRED
            .into_iter()
            .chain([b"glyf", b"loca"])
            .fold(Self::new(&[0, 1, 0, 0]), |sfnt, tag| {
                sfnt.table(tag, [0; 4])
            })
    }

    /// Adds (or replaces) table `tag` holding `data`.
    #[must_use]
    pub fn table(mut self, tag: &[u8; 4], data: impl Into<Vec<u8>>) -> Self {
        self = self.without(tag);
        self.tables.push((*tag, data.into()));
        self
    }

    /// Removes table `tag`, if present.
    #[must_use]
    pub fn without(mut self, tag: &[u8; 4]) -> Self {
        self.tables.retain(|(t, _)| t != tag);
        self
    }

    /// The font's bytes.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let mut tables = self.tables.clone();
        tables.sort_by_key(|(tag, _)| *tag);
        let count = tables.len() as u16;
        let mut out = self.scaler.to_vec();
        out.extend(count.to_be_bytes());
        // searchRange, entrySelector, rangeShift: hints readers ignore.
        out.extend([0; 6]);
        let mut offset = 12 + 16 * tables.len();
        let mut body = Vec::new();
        for (tag, data) in &tables {
            out.extend(tag);
            out.extend(checksum(data).to_be_bytes());
            out.extend((offset as u32).to_be_bytes());
            out.extend((data.len() as u32).to_be_bytes());
            let padded = data.len().next_multiple_of(4);
            body.extend(data);
            body.resize(body.len() + padded - data.len(), 0);
            offset += padded;
        }
        out.extend(body);
        out
    }
}

/// The sfnt table checksum: the sum of the table's big-endian `u32`s,
/// zero-padded.
fn checksum(data: &[u8]) -> u32 {
    data.chunks(4)
        .map(|chunk| {
            let mut word = [0; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            u32::from_be_bytes(word)
        })
        .fold(0, u32::wrapping_add)
}

/// Units per em in [`block_font`].
pub const BLOCK_UNITS_PER_EM: u16 = 1000;

/// A minimal TrueType font named `family` in which every printable ASCII
/// character (`' '` to `'~'`) is the same glyph: a filled square from the
/// baseline to 700 units up, 800 units wide in a 1000-unit advance. Text in
/// it draws as solid blocks, unlike any real font.
#[must_use]
pub fn block_font(family: &str) -> Vec<u8> {
    let mut glyf = block_glyph();
    glyf.resize(glyf.len().next_multiple_of(4), 0);
    let loca = [0u16, 0, (glyf.len() / 2) as u16];
    SfntBuilder::new(&[0, 1, 0, 0])
        .table(b"cmap", block_cmap())
        .table(b"glyf", glyf)
        .table(b"head", block_head())
        .table(b"hhea", block_hhea())
        .table(b"hmtx", be16(&[1000, 0, 1000, 100]))
        .table(b"loca", be16(&loca))
        .table(b"maxp", block_maxp())
        .table(b"name", block_name(family))
        .table(b"post", block_post())
        .build()
}

/// Big-endian `u16`s.
fn be16(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_be_bytes()).collect()
}

/// `head`: 1000 units per em, short `loca` offsets.
fn block_head() -> Vec<u8> {
    let mut head = Vec::new();
    head.extend(0x0001_0000u32.to_be_bytes()); // version
    head.extend(0x0001_0000u32.to_be_bytes()); // fontRevision
    head.extend(0u32.to_be_bytes()); // checksumAdjustment
    head.extend(0x5F0F_3CF5u32.to_be_bytes()); // magicNumber
    head.extend(be16(&[0, BLOCK_UNITS_PER_EM]));
    head.extend([0; 16]); // created, modified
    head.extend(be16(&[100, 0, 900, 700])); // the bounding box
    head.extend(be16(&[0, 8, 2])); // macStyle, lowestRecPPEM, direction
    head.extend(be16(&[0, 0])); // short loca, glyphDataFormat
    head
}

/// `hhea`: ascender 800, descender -200, two horizontal metrics.
fn block_hhea() -> Vec<u8> {
    let mut hhea = 0x0001_0000u32.to_be_bytes().to_vec();
    hhea.extend(be16(&[
        800,
        (-200i16) as u16,
        0,
        1000,
        0,
        100,
        900,
        1,
        0,
        0,
    ]));
    hhea.extend([0; 8]); // reserved
    hhea.extend(be16(&[0, 2])); // metricDataFormat, numberOfHMetrics
    hhea
}

/// `maxp` version 1.0: two glyphs, at most four points in one contour.
fn block_maxp() -> Vec<u8> {
    let mut maxp = 0x0001_0000u32.to_be_bytes().to_vec();
    maxp.extend(be16(&[2, 4, 1, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0]));
    maxp
}

/// Glyph 1: one clockwise contour round (100, 0), (100, 700), (900, 700),
/// (900, 0), every point on the curve with word-sized deltas.
fn block_glyph() -> Vec<u8> {
    let mut glyph = be16(&[1, 100, 0, 900, 700]); // contours and bounds
    glyph.extend(be16(&[3, 0])); // endPtsOfContours, no instructions
    glyph.extend([1; 4]); // ON_CURVE_POINT
    glyph.extend(be16(&[100, 0, 800, 0])); // x deltas
    glyph.extend(be16(&[0, 700, 0, (-700i16) as u16])); // y deltas
    glyph
}

/// `cmap`: one Windows Unicode BMP subtable (format 4) mapping `' '` to
/// `'~'` to glyph 1, through its glyph ID array.
fn block_cmap() -> Vec<u8> {
    let (first, last) = (u16::from(b' '), u16::from(b'~'));
    let glyphs = vec![1; usize::from(last - first) + 1];
    let length = 32 + 2 * glyphs.len() as u16;
    let mut cmap = be16(&[0, 1, 3, 1, 0, 12]); // header, one encoding record
    cmap.extend(be16(&[4, length, 0])); // format, length, language
    cmap.extend(be16(&[4, 4, 1, 0])); // segCountX2, searchRange, entrySelector, rangeShift
    cmap.extend(be16(&[last, 0xFFFF, 0])); // endCode, reservedPad
    cmap.extend(be16(&[first, 0xFFFF])); // startCode
    cmap.extend(be16(&[0, 1])); // idDelta
    // idRangeOffset: the first segment's glyph IDs start 4 bytes on.
    cmap.extend(be16(&[4, 0]));
    cmap.extend(be16(&glyphs));
    cmap
}

/// `name`: family (1), full name (4) and PostScript name (6), all
/// `family`, as Windows Unicode US English.
fn block_name(family: &str) -> Vec<u8> {
    let utf16: Vec<u8> = family.encode_utf16().flat_map(u16::to_be_bytes).collect();
    let length = utf16.len() as u16;
    let ids = [1, 4, 6];
    let mut name = be16(&[0, ids.len() as u16, 6 + 12 * ids.len() as u16]);
    for id in ids {
        name.extend(be16(&[3, 1, 0x0409, id, length, 0]));
    }
    name.extend(utf16);
    name
}

/// `post` version 3.0: no glyph names.
fn block_post() -> Vec<u8> {
    let mut post = 0x0003_0000u32.to_be_bytes().to_vec();
    post.extend([0; 28]);
    post
}

#[cfg(test)]
mod tests {
    use ttf_parser::{Face, GlyphId, OutlineBuilder, name_id};

    use super::*;

    #[test]
    fn the_directory_lists_each_table_sorted_with_its_checksum_offset_and_length() {
        let bytes = SfntBuilder::new(b"true")
            .table(b"zzzz", [0, 0, 0, 1, 0, 0, 0, 2, 3])
            .table(b"aaaa", [9; 4])
            .build();
        assert_eq!(
            &bytes[..12],
            [b't', b'r', b'u', b'e', 0, 2, 0, 0, 0, 0, 0, 0]
        );
        let record = |at: usize| &bytes[12 + 16 * at..][..16];
        // `aaaa` first: four bytes at the end of the 44-byte header.
        assert_eq!(
            record(0),
            [b"aaaa".as_slice(), &[9; 4], &[0, 0, 0, 44], &[0, 0, 0, 4]].concat()
        );
        // `zzzz` next, padded to 12 bytes; its checksum is 1 + 2 + 0x03000000.
        assert_eq!(
            record(1),
            [
                b"zzzz".as_slice(),
                &[3, 0, 0, 3],
                &[0, 0, 0, 48],
                &[0, 0, 0, 9]
            ]
            .concat()
        );
        assert_eq!(bytes.len(), 60);
        assert_eq!(
            &bytes[44..],
            [9, 9, 9, 9, 0, 0, 0, 1, 0, 0, 0, 2, 3, 0, 0, 0]
        );
    }

    /// Records an outline's path.
    #[derive(Default)]
    struct Path(Vec<String>);

    impl OutlineBuilder for Path {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.push(format!("M {x} {y}"));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.push(format!("L {x} {y}"));
        }
        fn quad_to(&mut self, _: f32, _: f32, x: f32, y: f32) {
            self.0.push(format!("Q {x} {y}"));
        }
        fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, x: f32, y: f32) {
            self.0.push(format!("C {x} {y}"));
        }
        fn close(&mut self) {
            self.0.push("Z".to_owned());
        }
    }

    #[test]
    fn the_block_font_reads_back_as_a_renderer_sees_it() {
        let bytes = block_font("Block Sans");
        let face = Face::parse(&bytes, 0).expect("parses");
        assert_eq!(face.units_per_em(), BLOCK_UNITS_PER_EM);
        assert_eq!((face.ascender(), face.descender()), (800, -200));
        assert_eq!(face.number_of_glyphs(), 2);
        assert!(face.tables().post.is_some(), "post parses");
        for id in [
            name_id::FAMILY,
            name_id::FULL_NAME,
            name_id::POST_SCRIPT_NAME,
        ] {
            let name = face.names().into_iter().find(|n| n.name_id == id);
            assert_eq!(
                name.and_then(|n| n.to_string()).as_deref(),
                Some("Block Sans")
            );
        }
        for c in [' ', 'A', 'g', '~'] {
            assert_eq!(face.glyph_index(c), Some(GlyphId(1)), "{c:?}");
        }
        assert_eq!(face.glyph_index('\u{7f}'), None);
        assert_eq!(face.glyph_index('\u{1f}'), None);
        assert_eq!(face.glyph_hor_advance(GlyphId(1)), Some(1000));
        assert_eq!(face.glyph_hor_side_bearing(GlyphId(1)), Some(100));
        let mut path = Path::default();
        face.outline_glyph(GlyphId(1), &mut path)
            .expect("an outline");
        assert_eq!(
            path.0,
            [
                "M 100 0",
                "L 100 700",
                "L 900 700",
                "L 900 0",
                "L 100 0",
                "Z"
            ]
        );
        assert!(
            face.outline_glyph(GlyphId(0), &mut Path::default())
                .is_none()
        );
    }
}
