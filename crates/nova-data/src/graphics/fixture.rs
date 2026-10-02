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
