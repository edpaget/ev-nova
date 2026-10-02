//! `STR#`: a list of Pascal strings.
//!
//! The standard Mac OS string list: a big-endian `u16` count followed by that
//! many Pascal strings (a length byte and Mac Roman text).

use binrw::binread;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::list::indexed;
use crate::wire::string::{MacString, pascal};

/// A string list.
#[binread]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct StrList {
    /// The string count (offset 0x00, u16); implied by `strings` once read.
    #[br(temp)]
    count: u16,
    /// The strings, from offset 0x02.
    #[br(parse_with = indexed(usize::from(count), pascal))]
    pub strings: Vec<MacString>,
}

impl Record for StrList {
    const TYPE: ResType = ResType::new(*b"STR#");
    const SIZE: Option<usize> = None;
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::decode::decode_bytes;
    use crate::error::Cause;

    /// Three strings: curly quotes, accented letters and an empty one.
    pub(crate) const SAMPLE: &[u8] = b"\x00\x03\x06\xD2hi\xD3 \xC9\x00\x0bK\x8Ase \x8Eclair";

    #[test]
    fn decodes_each_string_from_mac_roman() {
        let decoded = decode_bytes::<StrList>(SAMPLE).expect("decodes");
        let texts: Vec<&str> = decoded
            .record
            .strings
            .iter()
            .map(MacString::as_str)
            .collect();
        assert_eq!(
            texts,
            ["\u{201C}hi\u{201D} \u{2026}", "", "K\u{E4}se \u{E9}clair"]
        );
        assert_eq!(decoded.consumed, SAMPLE.len());
    }

    #[test]
    fn empty_list() {
        let decoded = decode_bytes::<StrList>(&[0, 0]).expect("decodes");
        assert!(decoded.record.strings.is_empty());
        assert_eq!(decoded.consumed, 2);
    }

    #[test]
    fn a_short_string_names_its_index() {
        let err = decode_bytes::<StrList>(b"\x00\x03\x01a\x01b\x04cd").expect_err("short");
        assert_eq!(err.path.to_string(), "StrList → strings[2]");
        assert_eq!(err.offset, 7, "the failing read is the third string's text");
        assert_eq!(err.cause, Cause::UnexpectedEnd);
    }

    #[test]
    fn a_count_past_the_data_is_an_error() {
        let err = decode_bytes::<StrList>(b"\x00\x02\x01a").expect_err("short");
        assert_eq!(err.path.to_string(), "StrList → strings[1]");
        assert_eq!(err.offset, 4, "the second length byte");
    }

    #[test]
    fn a_missing_count_is_an_error() {
        let err = decode_bytes::<StrList>(b"\x00").expect_err("short");
        assert_eq!(err.path.0[0], "StrList");
        assert_eq!(err.offset, 0);
    }
}
