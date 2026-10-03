//! `DLOG`: a dialog's window template.
//!
//! Layout from Inside Macintosh: Macintosh Toolbox Essentials ("The Dialog
//! Resource"), checked against every stock `DLOG`: the bounds, the window
//! definition ID, two Boolean bytes each followed by a filler byte, the
//! reference constant, the item list's ID and a Pascal-string title. Since
//! System 7 a positioning word may follow, after a pad byte that brings it
//! to an even offset; 40 of the 41 stock records have one.

use std::io::{Read, Seek, SeekFrom};

use binrw::{BinRead, BinResult, Endian};
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::geometry::Rect;
use crate::wire::id::DitlId;
use crate::wire::string::{MacString, pascal};

/// A dialog's window: where it sits and which item list fills it.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Dlog {
    /// `boundsRect` (offset 0x00, rect): the window's content area in
    /// global coordinates.
    pub bounds: Rect,
    /// `procID` (offset 0x08, i16): the window definition variant, raw.
    pub proc_id: i16,
    /// `visible` (offset 0x0A, u8, then a filler byte): non-zero is true.
    #[br(map = |b: u8| b != 0, pad_after = 1)]
    pub visible: bool,
    /// `goAwayFlag` (offset 0x0C, u8, then a filler byte): whether the
    /// window has a close box; non-zero is true.
    #[br(map = |b: u8| b != 0, pad_after = 1)]
    pub go_away: bool,
    /// `refCon` (offset 0x0E, i32): the application's reference constant.
    pub ref_con: i32,
    /// `itemsID` (offset 0x12, i16): the `DITL` holding the dialog's items.
    #[br(map = DitlId)]
    pub items_id: DitlId,
    /// The window title (offset 0x14, Pascal string); empty in stock.
    #[br(parse_with = pascal)]
    pub title: MacString,
    /// The positioning word (after the title, at the next even offset,
    /// u16): the raw Window Manager positioning constant, such as `0xA80A`
    /// (centre on the main screen). `None` when the record ends after the
    /// title.
    #[br(parse_with = position)]
    pub position: Option<u16>,
}

impl Record for Dlog {
    const TYPE: ResType = ResType::new(*b"DLOG");
    const SIZE: Option<usize> = None;
}

/// Reads the optional positioning word at the next even offset. With less
/// than a whole word there, nothing more is read and any stray bytes are
/// left as trailing bytes.
fn position<R: Read + Seek>(reader: &mut R, _: Endian, (): ()) -> BinResult<Option<u16>> {
    let at = reader.stream_position()?;
    let end = reader.seek(SeekFrom::End(0))?;
    let word_at = at + at % 2;
    if end.saturating_sub(word_at) < 2 {
        reader.seek(SeekFrom::Start(at))?;
        return Ok(None);
    }
    reader.seek(SeekFrom::Start(word_at))?;
    let mut word = [0; 2];
    reader.read_exact(&mut word)?;
    Ok(Some(u16::from_be_bytes(word)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_bytes;
    use crate::wire::id::DitlId;

    /// A stock-shaped `DLOG`: bounds (40, 30, 340, 530), procID 2, visible,
    /// no close box, refCon 7, items 1000, an empty title, then the even
    /// pad and position `0xA80A`.
    const POSITIONED: &[u8] = &[
        0, 40, 0, 30, 1, 84, 2, 18, // bounds
        0, 2, // procID
        1, 0, // visible, filler
        0, 0, // goAway, filler
        0, 0, 0, 7, // refCon
        0x03, 0xE8, // itemsID
        0,    // title
        0,    // pad
        0xA8, 0x0A, // position
    ];

    #[test]
    fn decodes_every_field_of_a_positioned_record() {
        let decoded = decode_bytes::<Dlog>(POSITIONED).expect("decodes");
        assert_eq!(decoded.consumed, 24);
        assert_eq!(
            decoded.record,
            Dlog {
                bounds: Rect {
                    top: 40,
                    left: 30,
                    bottom: 340,
                    right: 530,
                },
                proc_id: 2,
                visible: true,
                go_away: false,
                ref_con: 7,
                items_id: DitlId(1000),
                title: MacString::from(""),
                position: Some(0xA80A),
            }
        );
    }

    /// [`POSITIONED`]'s fixed header with `title` and then `tail`.
    fn with_title(title: &[u8], tail: &[u8]) -> Vec<u8> {
        let mut bytes = POSITIONED[..20].to_vec();
        bytes.push(title.len() as u8);
        bytes.extend_from_slice(title);
        bytes.extend_from_slice(tail);
        bytes
    }

    #[test]
    fn a_record_ending_after_the_title_has_no_position() {
        let bytes = with_title(b"", &[]);
        let decoded = decode_bytes::<Dlog>(&bytes).expect("decodes");
        assert_eq!(decoded.consumed, 21);
        assert_eq!(decoded.record.position, None);
        assert_eq!(decoded.record.items_id, DitlId(1000));
    }

    #[test]
    fn less_than_a_word_after_the_title_is_no_position() {
        for tail in [&[0][..], &[0, 0xA8][..]] {
            let bytes = with_title(b"", tail);
            let decoded = decode_bytes::<Dlog>(&bytes).expect("decodes");
            assert_eq!(decoded.record.position, None, "{} bytes", bytes.len());
            assert_eq!(decoded.consumed, 21, "the stray bytes are left over");
        }
    }

    #[test]
    fn an_odd_end_of_title_is_padded_before_the_position() {
        let bytes = with_title(b"Hi", &[0xFF, 0x28, 0x0A]);
        let decoded = decode_bytes::<Dlog>(&bytes).expect("decodes");
        assert_eq!(decoded.record.title.as_str(), "Hi");
        assert_eq!(decoded.record.position, Some(0x280A));
        assert_eq!(decoded.consumed, 26);
    }

    #[test]
    fn an_even_end_of_title_needs_no_pad() {
        let bytes = with_title(b"K\x8As", &[0x30, 0x0A]);
        let decoded = decode_bytes::<Dlog>(&bytes).expect("decodes");
        assert_eq!(decoded.record.title.as_str(), "K\u{E4}s");
        assert_eq!(decoded.record.position, Some(0x300A));
        assert_eq!(decoded.consumed, 26);
    }

    #[test]
    fn truncation_inside_the_header_names_the_field() {
        let err = decode_bytes::<Dlog>(&POSITIONED[..15]).expect_err("short");
        assert_eq!(
            err.to_string(),
            "field Dlog → ref_con at byte 0xe: unexpected end of data"
        );
        let err = decode_bytes::<Dlog>(&POSITIONED[..20]).expect_err("short");
        assert_eq!(
            err.to_string(),
            "field Dlog → title at byte 0x14: unexpected end of data"
        );
    }

    #[test]
    fn negative_bounds_and_flags_decode() {
        let mut bytes = POSITIONED.to_vec();
        bytes[..2].copy_from_slice(&(-150_i16).to_be_bytes());
        bytes[10] = 0;
        bytes[12] = 0x02;
        let dlog = decode_bytes::<Dlog>(&bytes).expect("decodes").record;
        assert_eq!(dlog.bounds.top, -150);
        assert!(!dlog.visible);
        assert!(dlog.go_away, "any non-zero byte is true");
    }

    #[test]
    fn corrupt_records_never_panic() {
        crate::sweep::assert_never_panics(&with_title(b"", &[]), decode_bytes::<Dlog>);
        // Prefixes of a positioned record can be valid on their own (its
        // first 21 bytes are), so they are only checked not to panic.
        for len in 0..POSITIONED.len() {
            let _ = decode_bytes::<Dlog>(&POSITIONED[..len]);
        }
    }

    #[test]
    fn json_round_trips() {
        let dlog = decode_bytes::<Dlog>(POSITIONED).expect("decodes").record;
        let json = serde_json::to_string(&dlog).expect("serializes");
        let back: Dlog = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, dlog);
        assert!(json.contains(r#""items_id":1000"#), "{json}");
    }
}
