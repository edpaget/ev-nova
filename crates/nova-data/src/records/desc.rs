//! `dësc`: a description text plus briefing extras.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The dësc
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 504): a NUL-terminated text, then 36 bytes, which is what every
//! stock record carries after its text.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::Flags16;
use crate::wire::id::{PictId, id};
use crate::wire::string::{MacString, c_string, fixed_c_string};

/// A description.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Desc {
    /// Bible `Description` (offset 0x00, C string): the text, with Nova's
    /// `{b123 "a" "b"}`-style substitutions left unexpanded. Offsets of the
    /// fields below are relative to the end of this text's NUL.
    #[br(parse_with = c_string)]
    pub text: MacString,
    /// Bible `Graphic` (text end + 0x00, i16): ID of a `PICT` shown with
    /// the text, or -1 for none.
    #[br(map = id::<PictId>)]
    pub graphic: Option<PictId>,
    /// Bible `MovieFile` (text end + 0x02, 32-byte C string): QuickTime movie
    /// file name.
    #[br(parse_with = fixed_c_string::<_, 32>)]
    pub movie_file: MacString,
    /// Bible `Flags` (text end + 0x22, u16): movie display flags.
    pub flags: Flags16,
}

impl Record for Desc {
    const TYPE: ResType = ResType::new([b'd', 0x91, b's', b'c']);
    const SIZE: Option<usize> = None;
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::decode::decode_bytes;
    use crate::error::Cause;

    /// Text with curly quotes and an accented letter, then the 36-byte tail.
    pub(crate) const SAMPLE: &[u8] = b"\xD2Caf\x8E\xD3\r\xD4ok\xD5\0\
        \x1F\x40\
        Race 1.mov\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\
        \x00\x05";

    #[test]
    fn decodes_text_and_trailing_fields() {
        let decoded = decode_bytes::<Desc>(SAMPLE).expect("decodes");
        assert_eq!(
            decoded.record,
            Desc {
                text: MacString::from("\u{201C}Caf\u{E9}\u{201D}\r\u{2018}ok\u{2019}"),
                graphic: Some(PictId(8000)),
                movie_file: MacString::from("Race 1.mov"),
                flags: Flags16(5),
            }
        );
        assert_eq!(decoded.consumed, SAMPLE.len());
    }

    #[test]
    fn no_graphic_is_none() {
        let mut bytes = b"x\0\xFF\xFF".to_vec();
        bytes.extend([0; 34]);
        let decoded = decode_bytes::<Desc>(&bytes).expect("decodes");
        assert_eq!(decoded.record.graphic, None);
        assert_eq!(decoded.consumed, 38);
    }

    #[test]
    fn unterminated_text_is_an_error_at_its_start() {
        let err = decode_bytes::<Desc>(b"no terminator").expect_err("unterminated");
        assert_eq!(err.path.to_string(), "Desc → text");
        assert_eq!(err.offset, 0);
        assert_eq!(err.cause, Cause::Invalid("unterminated string".to_owned()));
    }

    #[test]
    fn a_short_tail_names_the_field() {
        let err = decode_bytes::<Desc>(b"x\0\x00\x80Race").expect_err("short");
        assert_eq!(err.path.to_string(), "Desc → movie_file");
        assert_eq!(err.offset, 4);
        assert_eq!(err.cause, Cause::UnexpectedEnd);
    }
}
