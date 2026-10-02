//! Mac Roman text in its three wire forms.
//!
//! Nova stores text as Mac Roman bytes in three shapes: Pascal strings (a
//! length byte, then the text), NUL-terminated C strings, and fixed-width C
//! string fields that always occupy the same number of bytes. All three
//! decode to [`MacString`]. Mac Roman maps every byte value, so decoding
//! never fails; line endings (CR) are kept as stored.

use std::fmt;
use std::io::{Read, Seek};
use std::ops::Deref;

use binrw::{BinResult, Endian};
use encoding_rs::MACINTOSH;
use serde::{Deserialize, Serialize};

/// Text decoded from Mac Roman. Serializes as a plain JSON string.
#[derive(Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MacString(String);

impl MacString {
    /// Decodes Mac Roman bytes.
    #[must_use]
    pub fn from_mac_roman(bytes: &[u8]) -> Self {
        Self(MACINTOSH.decode_without_bom_handling(bytes).0.into_owned())
    }

    /// The decoded text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for MacString {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MacString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for MacString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl From<&str> for MacString {
    fn from(text: &str) -> Self {
        Self(text.to_owned())
    }
}

/// Reads a Pascal string: a length byte followed by that many bytes.
///
/// For `#[br(parse_with = pascal)]`.
pub fn pascal<R: Read + Seek>(reader: &mut R, _: Endian, (): ()) -> BinResult<MacString> {
    let mut len = [0];
    reader.read_exact(&mut len)?;
    let mut text = vec![0; usize::from(len[0])];
    reader.read_exact(&mut text)?;
    Ok(MacString::from_mac_roman(&text))
}

/// Reads a C string: bytes up to a NUL, which is consumed.
///
/// A missing NUL is an error at the string's first byte. For
/// `#[br(parse_with = c_string)]`.
pub fn c_string<R: Read + Seek>(reader: &mut R, _: Endian, (): ()) -> BinResult<MacString> {
    let start = reader.stream_position()?;
    let mut text = Vec::new();
    let mut byte = [0];
    loop {
        if reader.read(&mut byte)? == 0 {
            return Err(binrw::Error::AssertFail {
                pos: start,
                message: "unterminated string".to_owned(),
            });
        }
        if byte[0] == 0 {
            return Ok(MacString::from_mac_roman(&text));
        }
        text.push(byte[0]);
    }
}

/// Reads a fixed-width C string field of exactly `N` bytes.
///
/// The text runs to the first NUL, or all `N` bytes if there is none. Bytes
/// after the NUL are ignored (editors often leave stale text there). For
/// `#[br(parse_with = fixed_c_string::<_, N>)]`.
pub fn fixed_c_string<R: Read + Seek, const N: usize>(
    reader: &mut R,
    _: Endian,
    (): (),
) -> BinResult<MacString> {
    let mut field = [0; N];
    reader.read_exact(&mut field)?;
    let end = field.iter().position(|&b| b == 0).unwrap_or(N);
    Ok(MacString::from_mac_roman(&field[..end]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Every special character the plan checks, as Mac Roman bytes and the
    /// exact Unicode they stand for.
    const SPECIALS: &[u8] = b"\xD2q\xD3 \xD4s\xD5 caf\x8E K\x8Ase \xC9 a\xD1b";
    const SPECIALS_TEXT: &str =
        "\u{201C}q\u{201D} \u{2018}s\u{2019} caf\u{E9} K\u{E4}se \u{2026} a\u{2014}b";

    fn read<'a, T>(
        bytes: &'a [u8],
        parse: fn(&mut Cursor<&'a [u8]>, Endian, ()) -> BinResult<T>,
    ) -> (BinResult<T>, u64) {
        let mut cursor = Cursor::new(bytes);
        let result = parse(&mut cursor, Endian::Big, ());
        (result, cursor.position())
    }

    #[test]
    fn mac_roman_specials_decode_to_exact_unicode() {
        assert_eq!(MacString::from_mac_roman(SPECIALS).as_str(), SPECIALS_TEXT);
    }

    #[test]
    fn carriage_returns_are_kept() {
        assert_eq!(&*MacString::from_mac_roman(b"a\rb"), "a\rb");
    }

    #[test]
    fn display_debug_and_from_str() {
        let s = MacString::from("caf\u{E9}");
        assert_eq!(s.to_string(), "caf\u{E9}");
        assert_eq!(format!("{s:?}"), "\"caf\u{E9}\"");
        assert_eq!(MacString::default().as_str(), "");
    }

    #[test]
    fn serializes_as_a_plain_json_string() {
        let s = MacString::from_mac_roman(SPECIALS);
        let json = serde_json::to_string(&s).expect("serializes");
        assert_eq!(json, serde_json::to_string(SPECIALS_TEXT).expect("str"));
        let back: MacString = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, s);
    }

    #[test]
    fn pascal_reads_length_then_text() {
        let mut bytes = vec![u8::try_from(SPECIALS.len()).expect("short")];
        bytes.extend(SPECIALS);
        bytes.extend(b"rest");
        let (result, pos) = read(&bytes, pascal);
        assert_eq!(result.expect("decodes").as_str(), SPECIALS_TEXT);
        assert_eq!(pos, 1 + SPECIALS.len() as u64);
    }

    #[test]
    fn pascal_empty() {
        let (result, pos) = read(b"\0x", pascal);
        assert_eq!(result.expect("decodes").as_str(), "");
        assert_eq!(pos, 1);
    }

    #[test]
    fn pascal_length_past_end_is_unexpected_eof() {
        let (result, _) = read(b"\x05abc", pascal);
        match result {
            Err(binrw::Error::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof),
            other => panic!("expected EOF, got {other:?}"),
        }
    }

    #[test]
    fn c_string_reads_to_nul_and_consumes_it() {
        let mut bytes = SPECIALS.to_vec();
        bytes.extend(b"\0after");
        let (result, pos) = read(&bytes, c_string);
        assert_eq!(result.expect("decodes").as_str(), SPECIALS_TEXT);
        assert_eq!(pos, SPECIALS.len() as u64 + 1);
    }

    #[test]
    fn c_string_empty() {
        let (result, pos) = read(b"\0\0", c_string);
        assert_eq!(result.expect("decodes").as_str(), "");
        assert_eq!(pos, 1);
    }

    #[test]
    fn unterminated_c_string_is_an_error_at_its_start() {
        let mut cursor = Cursor::new(&b"xxabc"[..]);
        cursor.set_position(2);
        match c_string(&mut cursor, Endian::Big, ()) {
            Err(binrw::Error::AssertFail { pos, message }) => {
                assert_eq!(pos, 2);
                assert_eq!(message, "unterminated string");
            }
            other => panic!("expected AssertFail, got {other:?}"),
        }
    }

    #[test]
    fn fixed_c_string_stops_at_nul_but_consumes_the_whole_field() {
        let (result, pos) = read(b"ab\0zz\x8E\0\0rest", fixed_c_string::<_, 8>);
        assert_eq!(result.expect("decodes").as_str(), "ab");
        assert_eq!(pos, 8);
    }

    #[test]
    fn fixed_c_string_without_nul_uses_every_byte() {
        let (result, pos) = read(b"caf\x8Erest", fixed_c_string::<_, 4>);
        assert_eq!(result.expect("decodes").as_str(), "caf\u{E9}");
        assert_eq!(pos, 4);
    }

    #[test]
    fn fixed_c_string_decodes_specials_and_empty() {
        let mut bytes = SPECIALS.to_vec();
        bytes.resize(64, 0);
        let (result, _) = read(&bytes, fixed_c_string::<_, 64>);
        assert_eq!(result.expect("decodes").as_str(), SPECIALS_TEXT);
        let (result, _) = read(&[0; 4], fixed_c_string::<_, 4>);
        assert_eq!(result.expect("decodes").as_str(), "");
    }

    #[test]
    fn fixed_c_string_short_field_is_unexpected_eof() {
        let (result, _) = read(b"abc", fixed_c_string::<_, 4>);
        match result {
            Err(binrw::Error::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof),
            other => panic!("expected EOF, got {other:?}"),
        }
    }
}
