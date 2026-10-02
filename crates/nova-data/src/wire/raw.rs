//! Raw bytes, for content with no documented layout.
//!
//! [`RawBytes`] holds the rest of a record (whole resources such as `csüm`);
//! [`RawArray`] holds a fixed run of bytes inside a record whose meaning is
//! undocumented. Both serialize as a lowercase hex string, so JSON output
//! keeps every byte.

use std::fmt;
use std::io::{Read, Seek, SeekFrom};

use binrw::{BinRead, BinResult, Endian};
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};

/// The rest of a record, as raw bytes. Hex in JSON.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct RawBytes(pub Vec<u8>);

/// A fixed run of `N` raw bytes. Hex in JSON.
#[derive(BinRead, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawArray<const N: usize>(pub [u8; N]);

impl BinRead for RawBytes {
    type Args<'a> = ();

    /// Reads everything after the current position in one bounded read, so
    /// a misbehaving reader can never make it loop.
    fn read_options<R: Read + Seek>(reader: &mut R, _: Endian, (): ()) -> BinResult<Self> {
        let start = reader.stream_position()?;
        let end = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(start))?;
        let len = usize::try_from(end.saturating_sub(start)).unwrap_or(usize::MAX);
        let mut bytes = vec![0; len];
        reader.read_exact(&mut bytes)?;
        Ok(Self(bytes))
    }
}

impl<const N: usize> Default for RawArray<N> {
    fn default() -> Self {
        Self([0; N])
    }
}

fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|&b| [DIGITS[usize::from(b >> 4)], DIGITS[usize::from(b & 0xF)]])
        .map(char::from)
        .collect()
}

fn from_hex(text: &str) -> Result<Vec<u8>, String> {
    if !text.len().is_multiple_of(2) {
        return Err(format!("odd-length hex string ({} digits)", text.len()));
    }
    text.as_bytes()
        .chunks(2)
        .map(|pair| {
            let digits = std::str::from_utf8(pair).map_err(|e| e.to_string())?;
            if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(format!("invalid hex digits {digits:?}"));
            }
            u8::from_str_radix(digits, 16).map_err(|e| e.to_string())
        })
        .collect()
}

impl fmt::Debug for RawBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RawBytes({})", to_hex(&self.0))
    }
}

impl<const N: usize> fmt::Debug for RawArray<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RawArray({})", to_hex(&self.0))
    }
}

impl Serialize for RawBytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&to_hex(&self.0))
    }
}

impl<const N: usize> Serialize for RawArray<N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&to_hex(&self.0))
    }
}

/// Visits a hex string.
struct HexVisitor;

impl Visitor<'_> for HexVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a lowercase hex string")
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Vec<u8>, E> {
        from_hex(text).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for RawBytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(HexVisitor).map(Self)
    }
}

impl<'de, const N: usize> Deserialize<'de> for RawArray<N> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = deserializer.deserialize_str(HexVisitor)?;
        let len = bytes.len();
        <[u8; N]>::try_from(bytes)
            .map(Self)
            .map_err(|_| de::Error::invalid_length(len, &format!("{N} bytes").as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[derive(BinRead, Debug, PartialEq)]
    #[br(big)]
    struct Tail {
        head: u8,
        rest: RawBytes,
    }

    #[test]
    fn raw_bytes_reads_the_rest_of_the_record() {
        let mut cursor = Cursor::new([1, 0x91, 0x84, 0xEE, 0xB0]);
        let tail = Tail::read_be(&mut cursor).expect("decodes");
        assert_eq!(tail.head, 1);
        assert_eq!(tail.rest, RawBytes(vec![0x91, 0x84, 0xEE, 0xB0]));
        assert_eq!(cursor.position(), 5);
    }

    #[test]
    fn raw_bytes_past_the_end_is_empty() {
        let mut cursor = Cursor::new([1, 2]);
        cursor.set_position(5);
        assert_eq!(
            RawBytes::read_be(&mut cursor).expect("decodes"),
            RawBytes::default()
        );
    }

    #[test]
    fn raw_bytes_mid_stream_reads_only_the_tail_and_ends_at_the_end() {
        let mut cursor = Cursor::new([1, 2, 3, 4, 5]);
        cursor.set_position(2);
        assert_eq!(
            RawBytes::read_be(&mut cursor).expect("decodes"),
            RawBytes(vec![3, 4, 5])
        );
        assert_eq!(cursor.position(), 5);
    }

    /// Calls to `read` past this many fail, so a reader that loops errors
    /// out instead of hanging.
    const READ_LIMIT: usize = 8;

    /// A reader whose `read` reports the bytes after its position but never
    /// advances, so it never reaches end-of-file. Seeking works normally.
    struct StuckReader {
        data: Vec<u8>,
        pos: usize,
        reads: usize,
    }

    impl Read for StuckReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.reads += 1;
            if self.reads > READ_LIMIT {
                return Err(std::io::Error::other("read called too many times"));
            }
            let rest = &self.data[self.pos..];
            let n = buf.len().min(rest.len());
            buf[..n].copy_from_slice(&rest[..n]);
            Ok(n)
        }
    }

    impl Seek for StuckReader {
        fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
            self.pos = match to {
                SeekFrom::Start(n) => usize::try_from(n).expect("small offset"),
                SeekFrom::End(0) => self.data.len(),
                SeekFrom::Current(0) => self.pos,
                other => unimplemented!("{other:?}"),
            };
            Ok(u64::try_from(self.pos).expect("small position"))
        }
    }

    #[test]
    fn raw_bytes_reads_a_reader_that_never_advances_in_one_bounded_read() {
        let mut reader = StuckReader {
            data: vec![1, 2, 3, 4, 5],
            pos: 2,
            reads: 0,
        };
        assert_eq!(
            RawBytes::read_be(&mut reader).expect("decodes"),
            RawBytes(vec![3, 4, 5])
        );
        assert_eq!(reader.reads, 1);
    }

    #[test]
    fn raw_bytes_may_be_empty() {
        let tail = Tail::read_be(&mut Cursor::new([7])).expect("decodes");
        assert_eq!(tail.rest, RawBytes::default());
    }

    #[test]
    fn raw_array_reads_exactly_n_bytes() {
        let mut cursor = Cursor::new([1, 2, 3, 4]);
        let raw = RawArray::<3>::read_be(&mut cursor).expect("decodes");
        assert_eq!(raw, RawArray([1, 2, 3]));
        assert_eq!(cursor.position(), 3);
        assert_eq!(RawArray::<2>::default(), RawArray([0, 0]));
    }

    #[test]
    fn short_raw_array_is_an_error() {
        let result = RawArray::<3>::read_be(&mut Cursor::new([1, 2]));
        assert!(result.is_err());
    }

    #[test]
    fn json_is_lowercase_hex_and_round_trips() {
        let raw = RawBytes(vec![0x00, 0x0A, 0xFF, 0x91]);
        let json = serde_json::to_string(&raw).expect("serializes");
        assert_eq!(json, "\"000aff91\"");
        assert_eq!(
            serde_json::from_str::<RawBytes>(&json).expect("deserializes"),
            raw
        );

        let array = RawArray([0xAB, 0x01]);
        let json = serde_json::to_string(&array).expect("serializes");
        assert_eq!(json, "\"ab01\"");
        assert_eq!(
            serde_json::from_str::<RawArray<2>>(&json).expect("deserializes"),
            array
        );
        assert_eq!(
            serde_json::from_str::<RawBytes>("\"\"").expect("empty"),
            RawBytes::default()
        );
    }

    #[test]
    fn uppercase_hex_is_accepted() {
        assert_eq!(from_hex("AbCd"), Ok(vec![0xAB, 0xCD]));
    }

    #[test]
    fn malformed_hex_is_rejected() {
        assert!(
            serde_json::from_str::<RawBytes>("\"abc\"").is_err(),
            "odd length"
        );
        assert!(
            serde_json::from_str::<RawBytes>("\"zz\"").is_err(),
            "not hex"
        );
        assert!(serde_json::from_str::<RawBytes>("\"+1\"").is_err(), "sign");
        let err = serde_json::from_str::<RawBytes>("12").expect_err("not a string");
        assert!(
            err.to_string().contains("expected a lowercase hex string"),
            "{err}"
        );
        assert!(
            serde_json::from_str::<RawArray<2>>("\"ab\"").is_err(),
            "wrong length"
        );
    }

    #[test]
    fn debug_shows_hex() {
        assert_eq!(format!("{:?}", RawBytes(vec![1, 0xFE])), "RawBytes(01fe)");
        assert_eq!(format!("{:?}", RawArray([0x10])), "RawArray(10)");
    }
}
