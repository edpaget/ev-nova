//! Synthetic picture and sprite builders for tests.
//!
//! Available to this crate's tests and, through the `fixture` feature, to
//! other crates' tests. The builders write well-formed resources from plain
//! pixel values, so tests never need game data.

/// PackBits-encodes `data` as runs of `unit`-byte units (1 or 2): repeats of
/// two or more equal units become repeat runs, everything else literal runs,
/// each at most 128 units long. `data.len()` must be a multiple of `unit`.
#[must_use]
pub fn pack_bits(data: &[u8], unit: usize) -> Vec<u8> {
    let units: Vec<&[u8]> = data.chunks(unit).collect();
    let n = units.len();
    let mut out = Vec::new();
    let mut i = 0;
    // Each pass emits at least one unit, so `n` passes always suffice.
    for _ in 0..n {
        if i >= n {
            break;
        }
        let run = units[i..]
            .iter()
            .take(128)
            .take_while(|&&u| u == units[i])
            .count();
        if run >= 2 {
            out.push((257 - run) as u8);
            out.extend_from_slice(units[i]);
            i += run;
        } else {
            let literal = (i..n)
                .take(128)
                .take_while(|&j| units.get(j + 1) != Some(&units[j]))
                .count();
            out.push((literal - 1) as u8);
            for u in &units[i..i + literal] {
                out.extend_from_slice(u);
            }
            i += literal;
        }
    }
    out
}

/// A colour table: entries of (`value`, 16-bit red, green, blue). A device
/// table is looked up by entry position instead of `value`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ctab {
    /// Sets the device flag (bit 15 of `ctFlags`).
    pub device: bool,
    /// (`value`, [red, green, blue]) in table order.
    pub entries: Vec<(u16, [u16; 3])>,
}

impl Ctab {
    /// The `ColorTable` record: `ctSeed` 0, `ctFlags`, `ctSize` (entries
    /// minus one), then the entries.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = vec![0; 4];
        out.extend(u16::from(self.device).wrapping_shl(15).to_be_bytes());
        out.extend((self.entries.len() as u16).wrapping_sub(1).to_be_bytes());
        for (value, rgb) in &self.entries {
            out.extend(value.to_be_bytes());
            for component in rgb {
                out.extend(component.to_be_bytes());
            }
        }
        out
    }
}

/// Builds a version 2 `PICT` from opcodes. `frame` and other rectangles are
/// `[top, left, bottom, right]`.
#[derive(Clone, Debug)]
pub struct PictBuilder {
    frame: [i16; 4],
    bytes: Vec<u8>,
}

impl PictBuilder {
    /// The 14-byte header: `picSize` 0 (decoders ignore it), `picFrame`,
    /// and the version 2 opcode `0x0011 0x02FF`.
    #[must_use]
    pub fn new(frame: [i16; 4]) -> Self {
        let mut bytes = vec![0, 0];
        bytes.extend(rect_bytes(frame));
        bytes.extend([0x00, 0x11, 0x02, 0xFF]);
        Self { frame, bytes }
    }

    /// Any opcode and its data, after a pad byte if needed to reach an even
    /// offset.
    #[must_use]
    pub fn raw(mut self, opcode: u16, data: &[u8]) -> Self {
        if self.bytes.len() % 2 == 1 {
            self.bytes.push(0);
        }
        self.bytes.extend(opcode.to_be_bytes());
        self.bytes.extend_from_slice(data);
        self
    }

    /// `0C00` HeaderOp: an extended version 2 header at 72 dpi.
    #[must_use]
    pub fn header_op(self) -> Self {
        let mut data = vec![0xFF, 0xFE, 0, 0, 0, 0x48, 0, 0, 0, 0x48, 0, 0];
        data.extend(rect_bytes(self.frame));
        data.extend([0; 4]);
        self.raw(0x0C00, &data)
    }

    /// `001E` DefHilite.
    #[must_use]
    pub fn def_hilite(self) -> Self {
        self.raw(0x001E, &[])
    }

    /// `0000` NOP.
    #[must_use]
    pub fn nop(self) -> Self {
        self.raw(0x0000, &[])
    }

    /// `0001` Clip to a rectangular region.
    #[must_use]
    pub fn clip_rect(self, rect: [i16; 4]) -> Self {
        let mut data = vec![0, 10];
        data.extend(rect_bytes(rect));
        self.raw(0x0001, &data)
    }

    /// `00A0` ShortComment.
    #[must_use]
    pub fn short_comment(self, kind: u16) -> Self {
        self.raw(0x00A0, &kind.to_be_bytes())
    }

    /// `00A1` LongComment.
    #[must_use]
    pub fn long_comment(self, kind: u16, data: &[u8]) -> Self {
        let mut op = kind.to_be_bytes().to_vec();
        op.extend((data.len() as u16).to_be_bytes());
        op.extend_from_slice(data);
        self.raw(0x00A1, &op)
    }

    /// `009A` DirectBitsRect.
    #[must_use]
    pub fn direct_bits(self, bits: &DirectBits) -> Self {
        let mut data = vec![0, 0, 0, 0xFF]; // baseAddr
        data.extend(pixmap_bytes(&PixMapSpec {
            row_bytes: bits.row_bytes,
            bounds: bits.bounds,
            pack_type: bits.pack_type,
            pixel_type: 16,
            pixel_size: bits.pixel_size,
            cmp_count: bits.cmp_count,
            cmp_size: bits.cmp_size,
            pm_table: 0,
        }));
        data.extend(rect_bytes(bits.src));
        data.extend(rect_bytes(bits.dst));
        data.extend(bits.mode.to_be_bytes());
        let unit = match bits.pixels {
            DirectPixels::Rgb555(_) => 2,
            DirectPixels::Rgb888(_) => 1,
        };
        for row in bits.rows() {
            data.extend(pack_row(&row, bits.row_bytes, unit));
        }
        self.raw(0x009A, &data)
    }

    /// `0098` PackBitsRect.
    #[must_use]
    pub fn packbits_rect(self, bits: &IndexedBits) -> Self {
        let mut data = pixmap_bytes(&PixMapSpec {
            row_bytes: bits.row_bytes,
            bounds: bits.bounds,
            pack_type: 0,
            pixel_type: 0,
            pixel_size: bits.bits,
            cmp_count: 1,
            cmp_size: bits.bits,
            pm_table: 0,
        });
        data.extend(bits.ctab.bytes());
        data.extend(rect_bytes(bits.src));
        data.extend(rect_bytes(bits.dst));
        data.extend(bits.mode.to_be_bytes());
        let width = rect_width(bits.bounds);
        for row in bits.indices.chunks(width) {
            data.extend(pack_row(
                &index_row(row, bits.bits, bits.row_bytes),
                bits.row_bytes,
                1,
            ));
        }
        self.raw(0x0098, &data)
    }

    /// `00FF` EndPic.
    #[must_use]
    pub fn end(self) -> Self {
        self.raw(0x00FF, &[])
    }

    /// The picture's bytes.
    #[must_use]
    pub fn build(self) -> Vec<u8> {
        self.bytes
    }
}

/// A QuickDraw rectangle's 8 bytes.
fn rect_bytes(rect: [i16; 4]) -> Vec<u8> {
    rect.iter().flat_map(|v| v.to_be_bytes()).collect()
}

/// A `DirectBitsRect` (`009A`) pixel opcode. The constructors fill in a
/// valid `PixMap` and copy the whole of `bounds` to the same rectangle;
/// tests change fields to make variants.
#[derive(Clone, Debug)]
pub struct DirectBits {
    /// The `PixMap` bounds, `[top, left, bottom, right]`.
    pub bounds: [i16; 4],
    /// The source rectangle, within `bounds`.
    pub src: [i16; 4],
    /// The destination rectangle, within the picture frame.
    pub dst: [i16; 4],
    /// The transfer mode.
    pub mode: u16,
    /// `rowBytes`, without the `PixMap` flag (which is always written).
    pub row_bytes: u16,
    /// `packType`.
    pub pack_type: u16,
    /// `pixelSize`.
    pub pixel_size: u16,
    /// `cmpCount`.
    pub cmp_count: u16,
    /// `cmpSize`.
    pub cmp_size: u16,
    /// Row-major pixels covering `bounds`.
    pub pixels: DirectPixels,
}

/// The pixels of a [`DirectBits`], which also choose how rows are stored.
#[derive(Clone, Debug)]
pub enum DirectPixels {
    /// 16-bit `xRRRRRGGGGGBBBBB`, packed by 16-bit words into `rowBytes`.
    Rgb555(Vec<u16>),
    /// 8-bit red, green, blue: packed by bytes as planes (all red, then all
    /// green, then all blue), or chunky `xRGB` padded to `rowBytes` when
    /// `rowBytes` < 8 (rows stored unpacked).
    Rgb888(Vec<[u8; 3]>),
}

impl DirectBits {
    /// 16-bit `xRRRRRGGGGGBBBBB` pixels, packed by 16-bit words
    /// (`packType` 3) with `rowBytes` twice the width.
    #[must_use]
    pub fn rgb555(bounds: [i16; 4], pixels: &[u16]) -> Self {
        Self {
            bounds,
            src: bounds,
            dst: bounds,
            mode: 0,
            row_bytes: 2 * rect_width(bounds) as u16,
            pack_type: 3,
            pixel_size: 16,
            cmp_count: 3,
            cmp_size: 5,
            pixels: DirectPixels::Rgb555(pixels.to_vec()),
        }
    }

    /// 32-bit pixels with three 8-bit components, packed by component
    /// (`packType` 4) with `rowBytes` four times the width.
    #[must_use]
    pub fn rgb888(bounds: [i16; 4], pixels: &[[u8; 3]]) -> Self {
        Self {
            bounds,
            src: bounds,
            dst: bounds,
            mode: 0,
            row_bytes: 4 * rect_width(bounds) as u16,
            pack_type: 4,
            pixel_size: 32,
            cmp_count: 3,
            cmp_size: 8,
            pixels: DirectPixels::Rgb888(pixels.to_vec()),
        }
    }

    /// Each row's unpacked bytes.
    fn rows(&self) -> Vec<Vec<u8>> {
        let width = rect_width(self.bounds);
        let row_bytes = usize::from(self.row_bytes);
        match &self.pixels {
            DirectPixels::Rgb555(pixels) => pixels
                .chunks(width)
                .map(|row| {
                    let mut bytes: Vec<u8> = row.iter().flat_map(|p| p.to_be_bytes()).collect();
                    bytes.resize(row_bytes, 0);
                    bytes
                })
                .collect(),
            DirectPixels::Rgb888(pixels) if row_bytes < 8 => pixels
                .chunks(width)
                .map(|row| {
                    let mut bytes: Vec<u8> =
                        row.iter().flat_map(|&[r, g, b]| [0, r, g, b]).collect();
                    bytes.resize(row_bytes, 0);
                    bytes
                })
                .collect(),
            DirectPixels::Rgb888(pixels) => pixels
                .chunks(width)
                .map(|row| (0..3).flat_map(|c| row.iter().map(move |p| p[c])).collect())
                .collect(),
        }
    }
}

/// The `PixMap` fields a builder writes.
struct PixMapSpec {
    row_bytes: u16,
    bounds: [i16; 4],
    pack_type: u16,
    pixel_type: u16,
    pixel_size: u16,
    cmp_count: u16,
    cmp_size: u16,
    pm_table: u32,
}

/// A `PixMap` record from `rowBytes` to `pmReserved` (46 bytes). The
/// `PixMap` flag is added to `rowBytes`, which must not already carry it.
fn pixmap_bytes(pm: &PixMapSpec) -> Vec<u8> {
    let mut out = (pm.row_bytes + 0x8000).to_be_bytes().to_vec();
    out.extend(rect_bytes(pm.bounds));
    out.extend([0, 0]); // pmVersion
    out.extend(pm.pack_type.to_be_bytes());
    out.extend([0; 4]); // packSize
    out.extend([0, 0x48, 0, 0, 0, 0x48, 0, 0]); // hRes, vRes: 72 dpi
    out.extend(pm.pixel_type.to_be_bytes());
    out.extend(pm.pixel_size.to_be_bytes());
    out.extend(pm.cmp_count.to_be_bytes());
    out.extend(pm.cmp_size.to_be_bytes());
    out.extend([0; 4]); // planeBytes
    out.extend(pm.pm_table.to_be_bytes());
    out.extend([0; 4]); // pmReserved
    out
}

/// One stored row: raw when `rowBytes` < 8, otherwise a byte count (a
/// `u16` when `rowBytes` > 250, else a `u8`) and the PackBits data.
fn pack_row(row: &[u8], row_bytes: u16, unit: usize) -> Vec<u8> {
    if row_bytes < 8 {
        return row.to_vec();
    }
    let packed = pack_bits(row, unit);
    let mut out = if row_bytes > 250 {
        (packed.len() as u16).to_be_bytes().to_vec()
    } else {
        vec![packed.len() as u8]
    };
    out.extend(packed);
    out
}

fn rect_width(rect: [i16; 4]) -> usize {
    (i32::from(rect[3]) - i32::from(rect[1])) as usize
}

/// A `PackBitsRect` (`0098`) pixel opcode: indexed pixels with a colour
/// table, copying the whole of `bounds` to the same rectangle.
#[derive(Clone, Debug)]
pub struct IndexedBits {
    /// The `PixMap` bounds, `[top, left, bottom, right]`.
    pub bounds: [i16; 4],
    /// The source rectangle, within `bounds`.
    pub src: [i16; 4],
    /// The destination rectangle, within the picture frame.
    pub dst: [i16; 4],
    /// The transfer mode.
    pub mode: u16,
    /// `rowBytes`: by default the fewest bytes that hold a row.
    pub row_bytes: u16,
    /// Bits per pixel (`pixelSize`).
    pub bits: u16,
    /// The colour table.
    pub ctab: Ctab,
    /// Row-major pixel values covering `bounds`.
    pub indices: Vec<u8>,
}

impl IndexedBits {
    /// `bits`-per-pixel `indices` covering `bounds`, looked up in `ctab`.
    #[must_use]
    pub fn new(bounds: [i16; 4], bits: u16, ctab: &Ctab, indices: &[u8]) -> Self {
        Self {
            bounds,
            src: bounds,
            dst: bounds,
            mode: 0,
            row_bytes: min_row_bytes(rect_width(bounds), bits),
            bits,
            ctab: ctab.clone(),
            indices: indices.to_vec(),
        }
    }
}

/// The fewest bytes that hold `width` pixels of `bits` bits.
fn min_row_bytes(width: usize, bits: u16) -> u16 {
    (width * usize::from(bits)).div_ceil(8) as u16
}

/// Packs pixel values `bits` bits each, leftmost pixel in the high bits,
/// into `row_bytes` bytes.
fn index_row(indices: &[u8], bits: u16, row_bytes: u16) -> Vec<u8> {
    let bits = usize::from(bits);
    let mut row = vec![0; usize::from(row_bytes)];
    for (x, &index) in indices.iter().enumerate() {
        let bit = x * bits;
        // Depths over 8 bits write nothing: decoders reject them unread.
        let shift = 8usize.checked_sub(bits + bit % 8);
        if let (Some(byte), Some(shift)) = (row.get_mut(bit / 8), shift) {
            *byte |= (u16::from(index) << shift) as u8;
        }
    }
    row
}

/// A `cicn` colour icon: an indexed `PixMap`, a mask and a 1-bit icon.
/// `new` fills in the fewest row bytes; tests change fields to make
/// variants.
#[derive(Clone, Debug)]
pub struct Cicn {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// Bits per pixel (`pixelSize`).
    pub bits: u16,
    /// The `PixMap`'s `rowBytes`.
    pub row_bytes: u16,
    /// The mask's `rowBytes`.
    pub mask_row_bytes: u16,
    /// The 1-bit icon's `rowBytes`.
    pub icon_row_bytes: u16,
    /// The colour table.
    pub ctab: Ctab,
    /// Row-major pixel values.
    pub indices: Vec<u8>,
    /// Row-major mask: `true` is opaque.
    pub mask: Vec<bool>,
}

impl Cicn {
    /// A `width` x `height` icon of `bits`-per-pixel `indices`.
    #[must_use]
    pub fn new(
        width: u16,
        height: u16,
        bits: u16,
        ctab: &Ctab,
        indices: &[u8],
        mask: &[bool],
    ) -> Self {
        let mask_row_bytes = min_row_bytes(usize::from(width), 1);
        Self {
            width,
            height,
            bits,
            row_bytes: min_row_bytes(usize::from(width), bits),
            mask_row_bytes,
            icon_row_bytes: mask_row_bytes,
            ctab: ctab.clone(),
            indices: indices.to_vec(),
            mask: mask.to_vec(),
        }
    }

    /// The resource bytes: `PixMap`, mask and icon `BitMap`s, `iconData`
    /// handle, then mask bits, icon bits (the inverse of the mask), colour
    /// table and pixel rows.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let bounds = [0, 0, self.height as i16, self.width as i16];
        let mut out = vec![0; 4]; // baseAddr
        out.extend(pixmap_bytes(&PixMapSpec {
            row_bytes: self.row_bytes,
            bounds,
            pack_type: 0,
            pixel_type: 0,
            pixel_size: self.bits,
            cmp_count: 1,
            cmp_size: self.bits,
            pm_table: 0,
        }));
        for row_bytes in [self.mask_row_bytes, self.icon_row_bytes] {
            out.extend([0; 4]); // baseAddr
            out.extend(row_bytes.to_be_bytes());
            out.extend(rect_bytes(bounds));
        }
        out.extend([0; 4]); // iconData
        let width = usize::from(self.width);
        let mask: Vec<u8> = self.mask.iter().map(|&m| u8::from(m)).collect();
        for row in mask.chunks(width) {
            out.extend(index_row(row, 1, self.mask_row_bytes));
        }
        for row in mask.chunks(width) {
            let inverse: Vec<u8> = row.iter().map(|m| 1 - m).collect();
            out.extend(index_row(&inverse, 1, self.icon_row_bytes));
        }
        out.extend(self.ctab.bytes());
        for row in self.indices.chunks(width) {
            out.extend(index_row(row, self.bits, self.row_bytes));
        }
        out
    }
}

/// A `ppat` full-colour pixel pattern (`patType` 1): header, `PixMap`,
/// pixel rows, then the colour table, as in the stock resources.
#[derive(Clone, Debug)]
pub struct Ppat {
    /// `patType`.
    pub pat_type: u16,
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// Bits per pixel (`pixelSize`).
    pub bits: u16,
    /// The `PixMap`'s `rowBytes`.
    pub row_bytes: u16,
    /// The colour table.
    pub ctab: Ctab,
    /// Row-major pixel values.
    pub indices: Vec<u8>,
}

impl Ppat {
    /// A `width` x `height` pattern of `bits`-per-pixel `indices`.
    #[must_use]
    pub fn new(width: u16, height: u16, bits: u16, ctab: &Ctab, indices: &[u8]) -> Self {
        Self {
            pat_type: 1,
            width,
            height,
            bits,
            row_bytes: min_row_bytes(usize::from(width), bits),
            ctab: ctab.clone(),
            indices: indices.to_vec(),
        }
    }

    /// The resource bytes.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        const PAT_MAP: u32 = 28;
        const PAT_DATA: u32 = PAT_MAP + 50;
        let pm_table = PAT_DATA + u32::from(self.row_bytes) * u32::from(self.height);
        let mut out = self.pat_type.to_be_bytes().to_vec();
        out.extend(PAT_MAP.to_be_bytes());
        out.extend(PAT_DATA.to_be_bytes());
        out.extend([0; 4]); // patXData
        out.extend([0xFF, 0xFF]); // patXValid
        out.extend([0; 4]); // patXMap
        out.extend([0xAA; 8]); // pat1Data
        out.extend([0; 4]); // baseAddr
        out.extend(pixmap_bytes(&PixMapSpec {
            row_bytes: self.row_bytes,
            bounds: [0, 0, self.height as i16, self.width as i16],
            pack_type: 0,
            pixel_type: 0,
            pixel_size: self.bits,
            cmp_count: 1,
            cmp_size: self.bits,
            pm_table,
        }));
        // A zero width still writes its (empty) rows.
        for row in self.indices.chunks(usize::from(self.width).max(1)) {
            out.extend(index_row(row, self.bits, self.row_bytes));
        }
        out.extend(self.ctab.bytes());
        out
    }
}

/// Builds an `rlëD` sprite sheet from frames of tokens.
#[derive(Clone, Debug)]
pub struct RledBuilder {
    width: u16,
    height: u16,
    depth: u16,
    frame_count: Option<u16>,
    frames: u16,
    tokens: Vec<u8>,
}

impl RledBuilder {
    /// A 16-bit sheet of `width` x `height` frames.
    #[must_use]
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            depth: 16,
            frame_count: None,
            frames: 0,
            tokens: Vec::new(),
        }
    }

    /// Overrides the header's depth.
    #[must_use]
    pub fn depth(mut self, depth: u16) -> Self {
        self.depth = depth;
        self
    }

    /// Overrides the header's frame count (by default, the frames added).
    #[must_use]
    pub fn frame_count(mut self, count: u16) -> Self {
        self.frame_count = Some(count);
        self
    }

    /// Adds a frame built by `f`, then its frame-end token.
    #[must_use]
    pub fn frame(mut self, f: impl FnOnce(RledFrame) -> RledFrame) -> Self {
        self.tokens.extend(f(RledFrame::default()).bytes);
        self.tokens.extend([0; 4]);
        self.frames += 1;
        self
    }

    /// Adds one raw token.
    #[must_use]
    pub fn token(mut self, op: u8, count: u32) -> Self {
        self.tokens.extend(token(op, count));
        self
    }

    /// The sheet's bytes: the 16-byte header, then the tokens.
    #[must_use]
    pub fn build(self) -> Vec<u8> {
        let count = self.frame_count.unwrap_or(self.frames);
        let mut out = Vec::new();
        for field in [self.width, self.height, self.depth, 0, count, 0, 0, 0] {
            out.extend(field.to_be_bytes());
        }
        out.extend(self.tokens);
        out
    }
}

/// The tokens of one `rlëD` frame.
#[derive(Clone, Debug, Default)]
pub struct RledFrame {
    bytes: Vec<u8>,
}

impl RledFrame {
    /// Starts the next line. The count is left 0: decoders ignore it.
    #[must_use]
    pub fn line(self) -> Self {
        self.token(1, 0)
    }

    /// Opaque 16-bit pixels, padded to a 4-byte boundary.
    #[must_use]
    pub fn pixels(mut self, pixels: &[u16]) -> Self {
        self.bytes.extend(token(2, 2 * pixels.len() as u32));
        for pixel in pixels {
            self.bytes.extend(pixel.to_be_bytes());
        }
        if pixels.len() % 2 == 1 {
            self.bytes.extend([0, 0]);
        }
        self
    }

    /// `n` transparent pixels.
    #[must_use]
    pub fn skip(self, n: u32) -> Self {
        self.token(3, 2 * n)
    }

    /// A run of `n` pixels with the colour pair `a`, `b`.
    #[must_use]
    pub fn run(mut self, n: u32, a: u16, b: u16) -> Self {
        self.bytes.extend(token(4, 2 * n));
        self.bytes.extend(a.to_be_bytes());
        self.bytes.extend(b.to_be_bytes());
        self
    }

    /// One raw token.
    #[must_use]
    pub fn token(mut self, op: u8, count: u32) -> Self {
        self.bytes.extend(token(op, count));
        self
    }

    /// Raw bytes.
    #[must_use]
    pub fn raw(mut self, bytes: &[u8]) -> Self {
        self.bytes.extend_from_slice(bytes);
        self
    }
}

/// An `rlëD` token: the opcode in the top byte, a 24-bit count below.
fn token(op: u8, count: u32) -> [u8; 4] {
    // The count fits in 24 bits, so `+` is the same as `|`.
    ((u32::from(op) << 24) + count).to_be_bytes()
}
