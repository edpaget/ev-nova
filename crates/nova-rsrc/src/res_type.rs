//! Four-byte resource type codes.

use std::fmt;

use encoding_rs::MACINTOSH;

/// A resource type code: four raw bytes.
///
/// Type codes are not UTF-8. Nova uses high-bit Mac Roman characters such as
/// `shïp` (bytes `73 68 95 70`), so the bytes are kept as they are and only
/// decoded from Mac Roman for display.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResType(pub [u8; 4]);

impl ResType {
    /// Wraps four raw type-code bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// The four raw type-code bytes.
    #[must_use]
    pub const fn bytes(self) -> [u8; 4] {
        self.0
    }

    /// Encodes a four-character code as Mac Roman, e.g. `"shïp"`.
    ///
    /// Returns `None` unless `code` is exactly four characters, all of them
    /// representable in Mac Roman.
    #[must_use]
    pub fn from_mac_roman(code: &str) -> Option<Self> {
        let (bytes, _, had_errors) = MACINTOSH.encode(code);
        if had_errors {
            return None;
        }
        <[u8; 4]>::try_from(bytes.as_ref()).ok().map(Self)
    }
}

impl fmt::Display for ResType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&MACINTOSH.decode_without_bom_handling(&self.0).0)
    }
}

impl fmt::Debug for ResType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResType('{self}')")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};

    const SHIP: [u8; 4] = [0x73, 0x68, 0x95, 0x70];

    #[test]
    fn displays_ascii_code() {
        assert_eq!(ResType(*b"PICT").to_string(), "PICT");
    }

    #[test]
    fn displays_high_bit_code_as_mac_roman() {
        assert_eq!(ResType(SHIP).to_string(), "shïp");
    }

    #[test]
    fn debug_shows_quoted_code() {
        assert_eq!(format!("{:?}", ResType(SHIP)), "ResType('shïp')");
    }

    #[test]
    fn new_and_bytes_round_trip() {
        assert_eq!(ResType::new(SHIP).bytes(), SHIP);
        assert_eq!(ResType::new(SHIP), ResType(SHIP));
    }

    #[test]
    fn from_mac_roman_encodes_high_bit_chars() {
        assert_eq!(ResType::from_mac_roman("shïp"), Some(ResType(SHIP)));
        let syst = ResType::from_mac_roman("sÿst").expect("valid code");
        assert_eq!(syst.bytes(), [b's', 0xD8, b's', b't']);
        assert_eq!(syst.to_string(), "sÿst");
    }

    #[test]
    fn from_mac_roman_rejects_wrong_length() {
        assert_eq!(ResType::from_mac_roman("PIC"), None);
        assert_eq!(ResType::from_mac_roman("PICTS"), None);
        assert_eq!(ResType::from_mac_roman(""), None);
    }

    #[test]
    fn from_mac_roman_rejects_chars_outside_mac_roman() {
        assert_eq!(ResType::from_mac_roman("ab\u{4E2D}d"), None);
    }

    #[test]
    fn orders_by_bytes_and_hashes_by_value() {
        let set: BTreeSet<ResType> = [ResType(*b"shpx"), ResType(*b"PICT"), ResType(SHIP)]
            .into_iter()
            .collect();
        let ordered: Vec<_> = set.into_iter().collect();
        // Raw byte order: 'p' (0x70) sorts before 'ï' (0x95).
        assert_eq!(
            ordered,
            vec![ResType(*b"PICT"), ResType(*b"shpx"), ResType(SHIP)]
        );

        let hashed: HashSet<ResType> = [ResType(SHIP), ResType(SHIP)].into_iter().collect();
        assert_eq!(hashed.len(), 1);
    }
}
