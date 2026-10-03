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
/// `'~'` to glyph 1.
fn block_cmap() -> Vec<u8> {
    let first = u16::from(b' ');
    let mut cmap = be16(&[0, 1, 3, 1, 0, 12]); // header, one encoding record
    cmap.extend(be16(&[4, 32, 0])); // format, length, language
    cmap.extend(be16(&[4, 4, 1, 0])); // segCountX2, searchRange, entrySelector, rangeShift
    cmap.extend(be16(&[u16::from(b'~'), 0xFFFF, 0])); // endCode, reservedPad
    cmap.extend(be16(&[first, 0xFFFF])); // startCode
    cmap.extend(be16(&[1u16.wrapping_sub(first), 1])); // idDelta
    cmap.extend(be16(&[0, 0])); // idRangeOffset
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
