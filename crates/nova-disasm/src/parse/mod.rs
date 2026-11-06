//! Parsers for the listings, one module each.
//!
//! Each parser is tolerant: a line it doesn't recognise (a header, a blank
//! line, a section it doesn't read) is skipped, never an error, so a
//! slightly different `otool` still yields an index.

pub mod cstrings;
pub mod indirect;
pub mod pointers;
pub mod sections;
pub mod symbols;
pub mod text;

/// Parses a bare hex number (no `0x`), as the listings print addresses.
fn hex(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(s, 16).ok()
}

/// The checked-in fixture listings the tests parse.
#[cfg(test)]
pub(crate) mod fixture {
    use crate::ports::Listings;

    pub const TEXT: &str = include_str!("../../fixtures/excerpt/text.txt");
    pub const CSTRINGS: &str = include_str!("../../fixtures/excerpt/cstring.txt");
    pub const POINTERS: &str = include_str!("../../fixtures/excerpt/pointers.txt");
    pub const INDIRECT: &str = include_str!("../../fixtures/excerpt/indirect.txt");
    pub const SYMBOLS: &str = include_str!("../../fixtures/excerpt/nm.txt");
    pub const SECTIONS: &str = include_str!("../../fixtures/excerpt/sections.txt");

    /// Every fixture listing.
    pub fn listings() -> Listings {
        Listings {
            text: TEXT.to_owned(),
            cstrings: CSTRINGS.to_owned(),
            pointers: POINTERS.to_owned(),
            indirect: INDIRECT.to_owned(),
            symbols: SYMBOLS.to_owned(),
            sections: SECTIONS.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::hex;

    #[test]
    fn hex_reads_bare_hex_digits_only() {
        assert_eq!(hex("d8200"), Some(0xd8200));
        assert_eq!(hex("+1f"), None);
        assert_eq!(hex(""), None);
    }
}
