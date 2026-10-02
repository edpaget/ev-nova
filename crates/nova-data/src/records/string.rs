//! `STR `: a single Pascal string.
//!
//! The Bible (Appendix III) uses `STR ` resources to patch single entries of
//! `STR#` lists. Not present in the stock data.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::string::{MacString, pascal};

/// One string: a length byte and Mac Roman text.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct StrResource {
    /// The text (offset 0x00, Pascal string).
    #[br(parse_with = pascal)]
    pub text: MacString,
}

impl Record for StrResource {
    const TYPE: ResType = ResType::new(*b"STR ");
    const SIZE: Option<usize> = None;
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::decode::decode_bytes;
    use crate::error::Cause;

    /// A sample `STR ` with curly quotes and an accented letter.
    pub(crate) const SAMPLE: &[u8] = b"\x0a\xD2Caf\x8E\xD3 \xD4n\xD5";

    #[test]
    fn decodes_mac_roman_text() {
        let decoded = decode_bytes::<StrResource>(SAMPLE).expect("decodes");
        assert_eq!(
            decoded.record.text.as_str(),
            "\u{201C}Caf\u{E9}\u{201D} \u{2018}n\u{2019}"
        );
        assert_eq!(decoded.consumed, SAMPLE.len());
    }

    #[test]
    fn short_text_names_the_field() {
        let err = decode_bytes::<StrResource>(b"\x05ab").expect_err("short");
        assert_eq!(err.path.to_string(), "StrResource → text");
        assert_eq!(err.cause, Cause::UnexpectedEnd);
    }
}
