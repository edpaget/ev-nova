//! `DITL`: the items of a dialog.
//!
//! Layout from Inside Macintosh: Macintosh Toolbox Essentials ("The Item
//! List Resource"), checked against every stock `DITL`. The list starts
//! with its item count minus one (i16). Each item is a 4-byte placeholder
//! (filled in by the Dialog Manager at run time), its bounds (rect), a type
//! byte whose high bit marks the item disabled and whose low seven bits are
//! its kind, a length byte, that many bytes of data, and a pad byte when the
//! data is odd, so the next item starts at an even offset.
//!
//! | Code | Kind | Data | Stock |
//! |-----:|------|------|------:|
//! | 0 | [`ItemKind::User`] | none | 253 |
//! | 4 | [`ItemKind::Button`] | title | 25 |
//! | 5 | [`ItemKind::CheckBox`] | title | 16 |
//! | 6 | [`ItemKind::RadioButton`] | title | 0 |
//! | 7 | [`ItemKind::Control`] | `CNTL` ID (i16) | 5 |
//! | 8 | [`ItemKind::StaticText`] | text | 28 |
//! | 16 | [`ItemKind::EditText`] | text | 11 |
//! | 32 | [`ItemKind::Icon`] | icon ID (i16) | 2 |
//! | 64 | [`ItemKind::Picture`] | `PICT` ID (i16) | 19 |
//!
//! Any other code (such as 1, a help item) keeps its data raw in
//! [`ItemKind::Other`].

use std::io::{Read, Seek, SeekFrom};

use binrw::{BinResult, Endian, binread};
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::geometry::Rect;
use crate::wire::id::PictId;
use crate::wire::list::indexed;
use crate::wire::raw::RawBytes;
use crate::wire::string::MacString;

/// A dialog's item list.
#[binread]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Ditl {
    /// The item count minus one (offset 0x00, i16): -1 is an empty list.
    #[br(temp, assert(count >= -1, "negative item count {}", count))]
    count: i16,
    /// The items, from offset 0x02, in item-number order (item 1 first).
    #[br(parse_with = indexed(item_count(count), DialogItem::read_options))]
    pub items: Vec<DialogItem>,
}

impl Record for Ditl {
    const TYPE: ResType = ResType::new(*b"DITL");
    const SIZE: Option<usize> = None;
}

/// The number of items in a list whose stored count is `stored`.
fn item_count(stored: i16) -> usize {
    usize::try_from(i32::from(stored) + 1).unwrap_or(0)
}

/// One dialog item.
#[binread]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct DialogItem {
    /// The placeholder (item offset 0x00, 4 bytes), meaningful only at run
    /// time.
    #[br(temp)]
    placeholder: u32,
    /// The display rectangle (item offset 0x04, rect), in the dialog's
    /// local coordinates.
    pub bounds: Rect,
    /// The type byte (item offset 0x0C).
    #[br(temp)]
    type_byte: u8,
    /// Whether clicking the item reports it: the type byte's high bit
    /// clear.
    #[br(calc = type_byte & DISABLED == 0)]
    pub enabled: bool,
    /// The kind and its data (from item offset 0x0D: a length byte, the
    /// data and an optional pad byte).
    #[br(parse_with = item_kind, args(type_byte & !DISABLED))]
    pub kind: ItemKind,
}

/// The type byte's "disabled" bit.
const DISABLED: u8 = 0x80;

/// What a dialog item is, with its data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemKind {
    /// Code 0: an application-drawn area.
    User,
    /// Code 4: a push button.
    Button {
        /// The button's title.
        title: MacString,
    },
    /// Code 5: a check box.
    CheckBox {
        /// The check box's title.
        title: MacString,
    },
    /// Code 6: a radio button.
    RadioButton {
        /// The radio button's title.
        title: MacString,
    },
    /// Code 7: a control defined by a `CNTL` resource.
    Control {
        /// The `CNTL` resource's ID.
        cntl_id: i16,
    },
    /// Code 8: static text.
    StaticText {
        /// The text, with CR line breaks as stored.
        text: MacString,
    },
    /// Code 16: an editable text field.
    EditText {
        /// The field's initial text, often empty.
        text: MacString,
    },
    /// Code 32: an icon.
    Icon {
        /// The `ICON` (or `cicn`) resource's ID.
        icon_id: i16,
    },
    /// Code 64: a picture.
    Picture {
        /// The `PICT` resource's ID.
        pict: PictId,
    },
    /// Any other code, with its data kept raw.
    Other {
        /// The kind code (the type byte without its high bit).
        code: u8,
        /// The item's data.
        data: RawBytes,
    },
}

/// Reads an item's length byte, data and pad byte as kind `code`.
fn item_kind<R: Read + Seek>(reader: &mut R, _: Endian, (code,): (u8,)) -> BinResult<ItemKind> {
    let len_at = reader.stream_position()?;
    let mut len = [0];
    reader.read_exact(&mut len)?;
    let mut data = vec![0; usize::from(len[0])];
    reader.read_exact(&mut data)?;
    if data.len() % 2 == 1 {
        reader.seek(SeekFrom::Current(1))?;
    }
    let id = || match <[u8; 2]>::try_from(data.as_slice()) {
        Ok(bytes) => Ok(i16::from_be_bytes(bytes)),
        Err(_) => Err(binrw::Error::AssertFail {
            pos: len_at,
            message: format!(
                "item kind {code} needs a 2-byte ID, not {} bytes",
                data.len()
            ),
        }),
    };
    let text = || MacString::from_mac_roman(&data);
    Ok(match code {
        0 => ItemKind::User,
        4 => ItemKind::Button { title: text() },
        5 => ItemKind::CheckBox { title: text() },
        6 => ItemKind::RadioButton { title: text() },
        7 => ItemKind::Control { cntl_id: id()? },
        8 => ItemKind::StaticText { text: text() },
        16 => ItemKind::EditText { text: text() },
        32 => ItemKind::Icon { icon_id: id()? },
        64 => ItemKind::Picture {
            pict: PictId(id()?),
        },
        _ => ItemKind::Other {
            code,
            data: RawBytes(data.clone()),
        },
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::decode::decode_bytes;
    use crate::error::Cause;
    use crate::wire::id::PictId;

    const BOUNDS: Rect = Rect {
        top: 10,
        left: 20,
        bottom: 30,
        right: 140,
    };

    /// One item's bytes: a placeholder, [`BOUNDS`], the type byte, the
    /// length byte, `data` and a pad byte when `data` is odd.
    pub(crate) fn item(type_byte: u8, data: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
        for side in [BOUNDS.top, BOUNDS.left, BOUNDS.bottom, BOUNDS.right] {
            bytes.extend(side.to_be_bytes());
        }
        bytes.push(type_byte);
        bytes.push(data.len() as u8);
        bytes.extend_from_slice(data);
        if data.len() % 2 == 1 {
            bytes.push(0);
        }
        bytes
    }

    /// A `DITL` of `items`, with the count stored minus one.
    pub(crate) fn ditl(items: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = (items.len() as i16 - 1).to_be_bytes().to_vec();
        for item in items {
            bytes.extend_from_slice(item);
        }
        bytes
    }

    /// Decodes a one-item list from `type_byte` and `data`, checking that
    /// every byte is used and the bounds are read.
    fn one(type_byte: u8, data: &[u8]) -> DialogItem {
        let bytes = ditl(&[item(type_byte, data)]);
        let decoded = decode_bytes::<Ditl>(&bytes).expect("decodes");
        assert_eq!(decoded.consumed, bytes.len(), "uses every byte");
        let [item] = <[DialogItem; 1]>::try_from(decoded.record.items).expect("one item");
        assert_eq!(item.bounds, BOUNDS);
        item
    }

    /// The kind decoded from `code` (enabled) and `code | 0x80` (disabled),
    /// checking the enabled bit of each.
    fn kind(code: u8, data: &[u8]) -> ItemKind {
        let enabled = one(code, data);
        assert!(enabled.enabled, "{code:#04x} is enabled");
        let disabled = one(code | 0x80, data);
        assert!(!disabled.enabled, "{:#04x} is disabled", code | 0x80);
        assert_eq!(enabled.kind, disabled.kind, "the high bit is not the kind");
        enabled.kind
    }

    fn text(text: &str) -> MacString {
        MacString::from(text)
    }

    #[test]
    fn a_count_of_minus_one_is_no_items() {
        let decoded = decode_bytes::<Ditl>(&[0xFF, 0xFF]).expect("decodes");
        assert!(decoded.record.items.is_empty());
        assert_eq!(decoded.consumed, 2);
    }

    #[test]
    fn the_count_is_stored_minus_one() {
        let bytes = ditl(&[item(0, &[]), item(0x80, &[]), item(0, &[])]);
        let decoded = decode_bytes::<Ditl>(&bytes).expect("decodes");
        assert_eq!(decoded.record.items.len(), 3);
        assert_eq!(decoded.consumed, bytes.len());
    }

    #[test]
    fn a_count_below_minus_one_is_invalid() {
        let err = decode_bytes::<Ditl>(&[0xFF, 0xFE]).expect_err("invalid");
        assert_eq!(err.offset, 0);
        assert!(matches!(err.cause, Cause::Invalid(_)), "{err}");
    }

    #[test]
    fn a_count_past_the_data_names_the_missing_item() {
        let mut bytes = ditl(&[item(0, &[]), item(0, &[])]);
        bytes.truncate(bytes.len() - 14);
        let err = decode_bytes::<Ditl>(&bytes).expect_err("short");
        assert!(err.path.to_string().starts_with("Ditl → items[1]"), "{err}");
        assert_eq!(err.cause, Cause::UnexpectedEnd);
    }

    #[test]
    fn user_item() {
        assert_eq!(kind(0, &[]), ItemKind::User);
    }

    #[test]
    fn button() {
        assert_eq!(kind(4, b"OK"), ItemKind::Button { title: text("OK") });
    }

    #[test]
    fn check_box() {
        assert_eq!(
            kind(5, b"Sound"),
            ItemKind::CheckBox {
                title: text("Sound")
            }
        );
    }

    #[test]
    fn radio_button() {
        assert_eq!(
            kind(6, b"Easy"),
            ItemKind::RadioButton {
                title: text("Easy")
            }
        );
    }

    #[test]
    fn resource_control() {
        assert_eq!(kind(7, &[0x01, 0xF4]), ItemKind::Control { cntl_id: 500 });
    }

    #[test]
    fn static_text_is_mac_roman() {
        assert_eq!(
            kind(8, b"\xD2Pilot\xD3\r^0"),
            ItemKind::StaticText {
                text: text("\u{201C}Pilot\u{201D}\r^0")
            }
        );
    }

    #[test]
    fn edit_text_may_be_empty() {
        assert_eq!(kind(16, b""), ItemKind::EditText { text: text("") });
        assert_eq!(
            kind(16, b"Kestrel"),
            ItemKind::EditText {
                text: text("Kestrel")
            }
        );
    }

    #[test]
    fn icon() {
        assert_eq!(kind(32, &[0x00, 0x80]), ItemKind::Icon { icon_id: 128 });
    }

    #[test]
    fn picture() {
        assert_eq!(
            kind(64, &[0x05, 0x97]),
            ItemKind::Picture { pict: PictId(1431) }
        );
    }

    #[test]
    fn an_unknown_kind_keeps_its_bytes() {
        assert_eq!(
            kind(1, &[0, 1, 0, 2, 3]),
            ItemKind::Other {
                code: 1,
                data: RawBytes(vec![0, 1, 0, 2, 3]),
            }
        );
    }

    #[test]
    fn odd_length_data_is_padded_and_the_next_item_follows() {
        let bytes = ditl(&[item(8, b"abc"), item(4, b"Done")]);
        assert_eq!(bytes.len(), 2 + (14 + 3 + 1) + (14 + 4));
        let items = decode_bytes::<Ditl>(&bytes).expect("decodes").record.items;
        assert_eq!(items[0].kind, ItemKind::StaticText { text: text("abc") });
        assert_eq!(
            items[1].kind,
            ItemKind::Button {
                title: text("Done")
            }
        );
    }

    #[test]
    fn an_id_item_of_the_wrong_length_is_invalid_at_its_length_byte() {
        for (code, data) in [(7, &[1][..]), (32, &[0, 1, 2][..]), (64, &[][..])] {
            let bytes = ditl(&[item(0, &[]), item(code | 0x80, data)]);
            let err = decode_bytes::<Ditl>(&bytes).expect_err("invalid");
            assert!(err.path.to_string().starts_with("Ditl → items[1]"), "{err}");
            assert_eq!(err.offset, 2 + 14 + 13, "item {code}");
            assert_eq!(
                err.cause,
                Cause::Invalid(format!(
                    "item kind {code} needs a 2-byte ID, not {} bytes",
                    data.len()
                ))
            );
        }
    }

    #[test]
    fn text_past_the_end_is_unexpected_end() {
        let mut bytes = ditl(&[item(8, b"abcd")]);
        bytes.truncate(bytes.len() - 1);
        let err = decode_bytes::<Ditl>(&bytes).expect_err("short");
        assert_eq!(err.cause, Cause::UnexpectedEnd);
        assert!(err.path.to_string().starts_with("Ditl → items[0]"), "{err}");
    }

    /// One item of every kind the stock dialogs use, plus a radio button
    /// and a help item, mixing enabled and disabled.
    pub(crate) fn sample() -> Vec<u8> {
        ditl(&[
            item(0x80, &[]),
            item(4, b"OK"),
            item(0x85, b"Sound"),
            item(6, b"Easy"),
            item(7, &[0, 128]),
            item(0x88, b"abc"),
            item(16, b""),
            item(0xA0, &[0, 128]),
            item(64, &[0x05, 0x97]),
            item(1, &[1, 2, 3]),
        ])
    }

    #[test]
    fn a_missing_final_pad_byte_is_unexpected_end() {
        let mut bytes = ditl(&[item(8, b"abc")]);
        bytes.pop();
        let err = decode_bytes::<Ditl>(&bytes).expect_err("short");
        assert_eq!(err.cause, Cause::UnexpectedEnd);
        assert_eq!(err.offset, bytes.len() as u64);
    }

    #[test]
    fn corrupt_lists_never_panic() {
        crate::sweep::assert_never_panics(&sample(), decode_bytes::<Ditl>);
    }

    #[test]
    fn json_round_trips() {
        let ditl = decode_bytes::<Ditl>(&sample()).expect("decodes").record;
        let json = serde_json::to_string(&ditl).expect("serializes");
        let back: Ditl = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, ditl);
        assert!(
            json.contains(r#"{"Other":{"code":1,"data":"010203"}}"#),
            "{json}"
        );
    }
}
