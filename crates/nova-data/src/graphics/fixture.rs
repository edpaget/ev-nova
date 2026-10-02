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
