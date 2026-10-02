//! Decoding resources into typed records.
//!
//! [`decode_bytes`] decodes one record's bytes; [`decode`] decodes a
//! [`Resource`] and attaches its identity to any error; [`decode_all`] decodes
//! every resource of one type in a file and collects every failure instead of
//! stopping at the first.

use std::fmt::Debug;

use binrw::{BinRead, Endian};
use nova_rsrc::{ResType, Resource, ResourceFile};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{Cause, DecodeError, DecodeWarning, FieldError, FieldPath, field_error};
use crate::wire::reader::TrackingReader;

/// A record type: one resource type's layout.
pub trait Record:
    for<'a> BinRead<Args<'a> = ()> + Serialize + DeserializeOwned + Debug + PartialEq
{
    /// The resource type code this record decodes, e.g. `shïp`.
    const TYPE: ResType;
    /// The fixed layout size in bytes; `None` for variable-length layouts.
    const SIZE: Option<usize>;
}

/// A decoded record and how many bytes its layout used.
#[derive(Clone, Debug, PartialEq)]
pub struct Decoded<T> {
    /// The record.
    pub record: T,
    /// Bytes read by the layout.
    pub consumed: usize,
}

/// A decoded record with its resource's ID and name.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Entry<T> {
    /// The resource ID.
    pub id: i16,
    /// The resource name, if it has one.
    pub name: Option<String>,
    /// The record.
    pub record: T,
}

impl<T> Entry<T> {
    /// Converts the record, keeping the ID and name.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Entry<U> {
        Entry {
            id: self.id,
            name: self.name,
            record: f(self.record),
        }
    }
}

/// Every resource of one type in a file: the records that decoded, warnings
/// about them, and every failure.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedReport<T> {
    /// Records that decoded, in reference-list order.
    pub entries: Vec<Entry<T>>,
    /// Warnings about records that decoded.
    pub warnings: Vec<DecodeWarning>,
    /// Resources that failed, in reference-list order.
    pub errors: Vec<DecodeError>,
}

/// The struct's own name, e.g. `Ship`, for errors that `binrw` gives no
/// field context.
pub(crate) fn struct_name<T>() -> &'static str {
    let full = std::any::type_name::<T>();
    full.rsplit("::").next().unwrap_or(full)
}

/// Decodes one record's bytes.
///
/// A record shorter than its layout is always an error, even when the
/// missing bytes were only skipped over rather than read.
pub fn decode_bytes<T: Record>(data: &[u8]) -> Result<Decoded<T>, FieldError> {
    let mut reader = TrackingReader::new(data);
    let unnamed = || FieldPath(vec![struct_name::<T>().to_owned()]);
    match T::read_options(&mut reader, Endian::Big, ()) {
        Ok(record) => {
            let consumed = reader.position();
            let len = data.len() as u64;
            if consumed > len {
                return Err(FieldError {
                    path: unnamed(),
                    offset: len,
                    cause: Cause::UnexpectedEnd,
                });
            }
            Ok(Decoded {
                record,
                consumed: consumed as usize,
            })
        }
        Err(err) => {
            let mut error = field_error(err, reader.read_start());
            if error.path.0.is_empty() {
                error.path = unnamed();
            }
            Err(error)
        }
    }
}

/// Decodes one resource, attaching its type, ID, name and lengths to any
/// error. Bytes after the layout are a warning, not an error.
pub fn decode<T: Record>(
    res: &Resource<'_>,
) -> Result<(Entry<T>, Option<DecodeWarning>), DecodeError> {
    debug_assert_eq!(res.res_type(), T::TYPE, "decoding the wrong resource type");
    let data = res.data();
    let name = res.name().map(str::to_owned);
    match decode_bytes::<T>(data) {
        Ok(Decoded { record, consumed }) => {
            let warning = (consumed < data.len()).then(|| DecodeWarning::TrailingBytes {
                res_type: res.res_type(),
                id: res.id(),
                name: name.clone(),
                consumed,
                actual_len: data.len(),
            });
            let entry = Entry {
                id: res.id(),
                name,
                record,
            };
            Ok((entry, warning))
        }
        Err(field) => Err(DecodeError {
            res_type: res.res_type(),
            id: res.id(),
            name,
            expected_len: T::SIZE,
            actual_len: data.len(),
            field,
        }),
    }
}

/// Decodes every resource of type `T::TYPE` in `file`, never stopping early.
#[must_use]
pub fn decode_all<T: Record>(file: &ResourceFile) -> TypedReport<T> {
    let mut report = TypedReport {
        entries: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    };
    for res in file.resources(T::TYPE) {
        match decode::<T>(&res) {
            Ok((entry, warning)) => {
                report.entries.push(entry);
                report.warnings.extend(warning);
            }
            Err(err) => report.errors.push(err),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::list::indexed;
    use nova_rsrc::fixture::ForkBuilder;
    use serde::Deserialize;

    const FLAT: ResType = ResType::new(*b"FLAT");

    #[derive(BinRead, Debug, PartialEq, Serialize, Deserialize)]
    #[br(big)]
    struct Flat {
        a: u16,
        b: u32,
    }

    impl Record for Flat {
        const TYPE: ResType = FLAT;
        const SIZE: Option<usize> = Some(6);
    }

    #[derive(BinRead, Debug, PartialEq, Serialize, Deserialize)]
    #[br(big)]
    struct Item {
        id: u16,
        count: u16,
    }

    #[derive(BinRead, Debug, PartialEq, Serialize, Deserialize)]
    #[br(big)]
    struct Outer {
        tag: u16,
        #[br(parse_with = indexed(3, Item::read_options))]
        items: Vec<Item>,
    }

    impl Record for Outer {
        const TYPE: ResType = ResType::new(*b"OUTR");
        const SIZE: Option<usize> = Some(14);
    }

    #[derive(BinRead, Debug, PartialEq, Serialize, Deserialize)]
    #[br(big)]
    struct Checked {
        #[br(assert(kind < 4, "kind out of range"))]
        kind: u16,
    }

    impl Record for Checked {
        const TYPE: ResType = ResType::new(*b"CHKD");
        const SIZE: Option<usize> = Some(2);
    }

    /// Skips trailing bytes with a seek instead of reading them.
    #[derive(BinRead, Debug, PartialEq, Serialize, Deserialize)]
    #[br(big)]
    struct Padded {
        a: u8,
        #[br(pad_after = 4)]
        b: u8,
    }

    impl Record for Padded {
        const TYPE: ResType = ResType::new(*b"PADD");
        const SIZE: Option<usize> = Some(6);
    }

    fn path(segments: &[&str]) -> FieldPath {
        FieldPath(segments.iter().map(|s| (*s).to_owned()).collect())
    }

    #[test]
    fn decodes_a_flat_record_and_reports_bytes_used() {
        let decoded = decode_bytes::<Flat>(&[0, 1, 0, 0, 0, 2, 0xEE]).expect("decodes");
        assert_eq!(
            decoded,
            Decoded {
                record: Flat { a: 1, b: 2 },
                consumed: 6,
            }
        );
    }

    #[test]
    fn truncation_mid_field_names_the_field_and_its_offset() {
        let err = decode_bytes::<Flat>(&[0, 1, 0, 0]).expect_err("short");
        assert_eq!(
            err,
            FieldError {
                path: path(&["Flat", "b"]),
                offset: 2,
                cause: Cause::UnexpectedEnd,
            }
        );
    }

    #[test]
    fn truncation_inside_an_array_element_names_the_index() {
        let mut bytes = vec![0, 9, 0, 1, 0, 1, 0, 2, 0, 2, 0, 3];
        bytes.push(0);
        let err = decode_bytes::<Outer>(&bytes).expect_err("short");
        assert_eq!(
            err,
            FieldError {
                path: path(&["Outer", "items[2]", "count"]),
                offset: 12,
                cause: Cause::UnexpectedEnd,
            }
        );
        assert_eq!(
            err.to_string(),
            "field Outer → items[2] → count at byte 0xc: unexpected end of data"
        );
    }

    #[test]
    fn nested_record_decodes() {
        let bytes = [0, 9, 0, 1, 0, 10, 0, 2, 0, 20, 0, 3, 0, 30];
        let decoded = decode_bytes::<Outer>(&bytes).expect("decodes");
        assert_eq!(decoded.consumed, 14);
        assert_eq!(decoded.record.items[2], Item { id: 3, count: 30 });
    }

    #[test]
    fn assert_failure_is_invalid_and_names_the_struct() {
        let err = decode_bytes::<Checked>(&[0, 7]).expect_err("asserts");
        assert_eq!(
            err,
            FieldError {
                path: path(&["Checked"]),
                offset: 0,
                cause: Cause::Invalid("kind out of range".to_owned()),
            }
        );
    }

    #[test]
    fn a_record_ending_inside_skipped_bytes_is_an_error() {
        for len in 2..6 {
            let err = decode_bytes::<Padded>(&vec![1; len]).expect_err("short");
            assert_eq!(
                err,
                FieldError {
                    path: path(&["Padded"]),
                    offset: len as u64,
                    cause: Cause::UnexpectedEnd,
                },
                "{len} bytes"
            );
        }
        let decoded = decode_bytes::<Padded>(&[1, 2, 0, 0, 0, 0]).expect("exact size");
        assert_eq!(decoded.consumed, 6);
    }

    fn file(builder: &ForkBuilder) -> ResourceFile {
        ResourceFile::from_bytes(builder.build().bytes).expect("valid fork")
    }

    #[test]
    fn decode_attaches_id_and_name() {
        let file =
            file(&ForkBuilder::new().resource(FLAT, 130, Some(b"K\x8Ase"), &[0, 1, 0, 0, 0, 2]));
        let res = file.get(FLAT, 130).expect("present");
        let (entry, warning) = decode::<Flat>(&res).expect("decodes");
        assert_eq!(
            entry,
            Entry {
                id: 130,
                name: Some("Käse".to_owned()),
                record: Flat { a: 1, b: 2 },
            }
        );
        assert_eq!(warning, None, "exact size gives no warning");
    }

    #[test]
    fn trailing_bytes_are_a_warning_not_an_error() {
        let file = file(&ForkBuilder::new().resource(FLAT, 128, None, &[0, 1, 0, 0, 0, 2, 9, 9]));
        let res = file.get(FLAT, 128).expect("present");
        let (entry, warning) = decode::<Flat>(&res).expect("decodes");
        assert_eq!(entry.record, Flat { a: 1, b: 2 });
        assert_eq!(
            warning,
            Some(DecodeWarning::TrailingBytes {
                res_type: FLAT,
                id: 128,
                name: None,
                consumed: 6,
                actual_len: 8,
            })
        );
    }

    #[test]
    fn decode_error_carries_identity_and_lengths() {
        let file = file(&ForkBuilder::new().resource(FLAT, -3, Some(b"short"), &[0, 1, 0]));
        let res = file.get(FLAT, -3).expect("present");
        let err = decode::<Flat>(&res).expect_err("short");
        assert_eq!(
            err,
            DecodeError {
                res_type: FLAT,
                id: -3,
                name: Some("short".to_owned()),
                expected_len: Some(6),
                actual_len: 3,
                field: FieldError {
                    path: path(&["Flat", "b"]),
                    offset: 2,
                    cause: Cause::UnexpectedEnd,
                },
            }
        );
    }

    #[test]
    fn decode_all_keeps_going_and_reports_every_failure_in_order() {
        let other = ResType::new(*b"othr");
        let file = file(
            &ForkBuilder::new()
                .resource(FLAT, 128, Some(b"bad one"), &[0])
                .resource(FLAT, 129, Some(b"good one"), &[0, 1, 0, 0, 0, 1])
                .resource(other, 128, None, &[])
                .resource(FLAT, 130, None, &[0, 1, 0, 0, 0])
                .resource(FLAT, 131, None, &[0, 2, 0, 0, 0, 2, 0])
                .resource(FLAT, 132, Some(b"bad three"), &[]),
        );
        let report = decode_all::<Flat>(&file);

        assert_eq!(
            report.entries,
            vec![
                Entry {
                    id: 129,
                    name: Some("good one".to_owned()),
                    record: Flat { a: 1, b: 1 },
                },
                Entry {
                    id: 131,
                    name: None,
                    record: Flat { a: 2, b: 2 },
                },
            ]
        );
        assert_eq!(report.warnings.len(), 1);
        let errors: Vec<_> = report
            .errors
            .iter()
            .map(|e| {
                (
                    e.res_type,
                    e.id,
                    e.name.as_deref(),
                    e.field.path.to_string(),
                    e.field.offset,
                )
            })
            .collect();
        assert_eq!(
            errors,
            vec![
                (FLAT, 128, Some("bad one"), "Flat → a".to_owned(), 0),
                (FLAT, 130, None, "Flat → b".to_owned(), 2),
                (FLAT, 132, Some("bad three"), "Flat → a".to_owned(), 0),
            ]
        );
    }

    #[test]
    fn decode_all_of_an_absent_type_is_empty() {
        let report = decode_all::<Flat>(&file(&ForkBuilder::new()));
        assert!(report.entries.is_empty());
        assert!(report.warnings.is_empty());
        assert!(report.errors.is_empty());
    }

    #[test]
    fn struct_name_drops_the_module_path() {
        assert_eq!(struct_name::<Flat>(), "Flat");
        assert_eq!(struct_name::<u8>(), "u8");
    }
}
