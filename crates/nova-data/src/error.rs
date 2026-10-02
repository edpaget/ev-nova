//! Decode errors and warnings, with the resource, field path and offset.
//!
//! [`FieldError`] says what went wrong inside one record's bytes: the field
//! path (e.g. `StrList → strings[2]`), the byte offset within the record and
//! the cause. [`DecodeError`] adds which resource it was. Both are plain owned
//! data (`Clone + PartialEq`), so callers can collect, compare and render
//! them however they like.

use std::fmt;

use binrw::error::BacktraceFrame;
use nova_rsrc::ResType;

/// Where in a record a failure happened, outermost first:
/// `["Ship", "weapons[2]", "count"]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldPath(pub Vec<String>);

impl fmt::Display for FieldPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.join(" → "))
    }
}

/// Why a record failed to decode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cause {
    /// The record ended before the layout did.
    UnexpectedEnd,
    /// The bytes are structurally invalid (e.g. an unterminated string).
    Invalid(String),
}

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEnd => f.write_str("unexpected end of data"),
            Self::Invalid(message) => f.write_str(message),
        }
    }
}

/// A failure inside one record's bytes, without the resource's identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
    /// The failing field, outermost first.
    pub path: FieldPath,
    /// Byte offset within the record where the failing read began.
    pub offset: u64,
    /// What went wrong.
    pub cause: Cause,
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.path.0.is_empty() {
            write!(f, "field {} ", self.path)?;
        }
        write!(f, "at byte {:#x}: {}", self.offset, self.cause)
    }
}

impl std::error::Error for FieldError {}

/// A resource that failed to decode.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{}: {field}{}", identity(*res_type, *id, name.as_deref()), lengths(*expected_len, *actual_len))]
pub struct DecodeError {
    /// The resource's type code.
    pub res_type: ResType,
    /// The resource's ID.
    pub id: i16,
    /// The resource's name, if it has one.
    pub name: Option<String>,
    /// The record's fixed layout size; `None` for variable layouts.
    pub expected_len: Option<usize>,
    /// The resource's data length.
    pub actual_len: usize,
    /// What failed inside the record.
    pub field: FieldError,
}

/// Something suspicious that did not stop a resource from decoding.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DecodeWarning {
    /// The record has bytes after the end of its layout; they are ignored.
    /// Plug-in editors sometimes pad records.
    #[error(
        "{}: {} trailing bytes ignored (layout needs {consumed}, record is {actual_len} bytes)",
        identity(*res_type, *id, name.as_deref()),
        actual_len - consumed
    )]
    TrailingBytes {
        /// The resource's type code.
        res_type: ResType,
        /// The resource's ID.
        id: i16,
        /// The resource's name, if it has one.
        name: Option<String>,
        /// Bytes the layout used.
        consumed: usize,
        /// The resource's data length.
        actual_len: usize,
    },
}

/// `shïp 128 "Shuttle"`, or `shïp 128` when unnamed.
fn identity(res_type: ResType, id: i16, name: Option<&str>) -> String {
    match name {
        Some(name) => format!("{res_type} {id} {name:?}"),
        None => format!("{res_type} {id}"),
    }
}

/// ` (record is 400 bytes, layout needs 1860)`, or without the layout size
/// for variable layouts.
fn lengths(expected: Option<usize>, actual: usize) -> String {
    match expected {
        Some(expected) => format!(" (record is {actual} bytes, layout needs {expected})"),
        None => format!(" (record is {actual} bytes)"),
    }
}

/// Converts a `binrw` error into a [`FieldError`].
///
/// `binrw` errors carry a backtrace of frames, innermost first, whose
/// messages read `While parsing field 'x' in Y`; they become the path
/// `Y → x`, outermost first. Index frames (`[2]`, added by
/// [`crate::wire::list::indexed`]) attach to the segment before them. Any
/// other frame message is kept verbatim. Running out of data carries no
/// position in `binrw`, so `read_start` (where the failing read began, from
/// [`crate::wire::reader::TrackingReader`]) is used as its offset.
#[must_use]
pub fn field_error(err: binrw::Error, read_start: u64) -> FieldError {
    let (root, frames) = match err {
        binrw::Error::Backtrace(bt) => (*bt.error, bt.frames),
        other => (other, Vec::new()),
    };
    let mut path: Vec<String> = Vec::new();
    let mut seen_field = false;
    for frame in frames.iter().rev() {
        let message = frame_message(frame);
        if let Some((field, owner)) = parse_field_frame(&message) {
            if !seen_field {
                path.push(owner.to_owned());
                seen_field = true;
            }
            path.push(field.to_owned());
        } else if let (true, Some(last)) = (is_index_frame(&message), path.last_mut()) {
            last.push_str(&message);
        } else {
            path.push(message);
        }
    }
    let (offset, cause) = root_cause(root, read_start);
    FieldError {
        path: FieldPath(path),
        offset,
        cause,
    }
}

fn frame_message(frame: &BacktraceFrame) -> String {
    match frame {
        BacktraceFrame::Full { message, .. } | BacktraceFrame::Message(message) => {
            message.to_string()
        }
        BacktraceFrame::Custom(custom) => custom.to_string(),
    }
}

/// Splits `While parsing field 'x' in Y` into `("x", "Y")`.
fn parse_field_frame(message: &str) -> Option<(&str, &str)> {
    let rest = message.strip_prefix("While parsing field '")?;
    rest.split_once("' in ")
}

/// Whether `message` is an index frame such as `[2]`.
fn is_index_frame(message: &str) -> bool {
    message
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// The root error's offset and cause.
fn root_cause(root: binrw::Error, read_start: u64) -> (u64, Cause) {
    use binrw::Error;
    match root {
        Error::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            (read_start, Cause::UnexpectedEnd)
        }
        Error::Io(e) => (read_start, Cause::Invalid(e.to_string())),
        Error::AssertFail { pos, message } => (pos, Cause::Invalid(message)),
        Error::Custom { pos, err } => (pos, Cause::Invalid(err.to_string())),
        Error::BadMagic { pos, found } => (pos, Cause::Invalid(format!("bad magic: {found:?}"))),
        Error::NoVariantMatch { pos } | Error::EnumErrors { pos, .. } => {
            (pos, Cause::Invalid("no variant matched".to_owned()))
        }
        other => (read_start, Cause::Invalid(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::error::{BacktraceFrame, ContextExt};
    use binrw::{BinRead, Error};
    use std::io::{self, Cursor};

    const SHIP: ResType = ResType::new([b's', b'h', 0x95, b'p']);

    fn path(segments: &[&str]) -> FieldPath {
        FieldPath(segments.iter().map(|s| (*s).to_owned()).collect())
    }

    fn eof() -> Error {
        Error::Io(io::Error::from(io::ErrorKind::UnexpectedEof))
    }

    fn field_frame(field: &str, ty: &str) -> BacktraceFrame {
        BacktraceFrame::Full {
            code: None,
            message: format!("While parsing field '{field}' in {ty}").into(),
            file: "test.rs",
            line: 1,
        }
    }

    #[derive(BinRead, Debug)]
    #[br(big)]
    #[allow(dead_code)]
    struct SpikeInner {
        a: u16,
        count: u32,
    }

    #[derive(BinRead, Debug)]
    #[br(big)]
    #[allow(dead_code)]
    struct SpikeOuter {
        x: u16,
        inner: SpikeInner,
    }

    /// Pins binrw's frame message format, which the path builder parses. A
    /// binrw upgrade that changes it fails here first.
    #[test]
    fn binrw_frames_name_the_field_and_struct_innermost_first() {
        let err = SpikeOuter::read(&mut Cursor::new([0, 1, 0, 2, 0])).expect_err("truncated");
        let Error::Backtrace(bt) = err else {
            panic!("expected a backtrace, got {err:?}")
        };
        let messages: Vec<String> = bt
            .frames
            .iter()
            .map(|frame| match frame {
                BacktraceFrame::Full { message, .. } | BacktraceFrame::Message(message) => {
                    message.to_string()
                }
                BacktraceFrame::Custom(c) => c.to_string(),
            })
            .collect();
        assert_eq!(
            messages,
            [
                "While parsing field 'count' in SpikeInner",
                "While parsing field 'inner' in SpikeOuter",
            ]
        );
        assert!(matches!(*bt.error, Error::Io(ref e) if e.kind() == io::ErrorKind::UnexpectedEof));
    }

    #[test]
    fn nested_frames_become_an_outermost_first_path() {
        let err = eof()
            .with_context(field_frame("count", "Inner"))
            .with_context(field_frame("inner", "Outer"));
        assert_eq!(
            field_error(err, 7),
            FieldError {
                path: path(&["Outer", "inner", "count"]),
                offset: 7,
                cause: Cause::UnexpectedEnd,
            }
        );
    }

    #[test]
    fn index_frames_attach_to_the_previous_segment() {
        let err = eof()
            .with_context(field_frame("count", "Item"))
            .with_context(BacktraceFrame::Message("[2]".into()))
            .with_context(field_frame("items", "Outer"));
        assert_eq!(
            field_error(err, 0).path,
            path(&["Outer", "items[2]", "count"])
        );
    }

    #[test]
    fn only_bracketed_digits_are_index_frames() {
        for message in ["[]", "[x]", "[2", "2]"] {
            let err = eof()
                .with_context(BacktraceFrame::Message(message.into()))
                .with_context(field_frame("items", "Outer"));
            assert_eq!(
                field_error(err, 0).path,
                path(&["Outer", "items", message]),
                "{message}"
            );
        }
    }

    #[test]
    fn an_index_frame_with_nothing_before_it_is_its_own_segment() {
        let err = eof().with_context(BacktraceFrame::Message("[0]".into()));
        assert_eq!(field_error(err, 0).path, path(&["[0]"]));
    }

    #[test]
    fn unrecognised_frames_are_kept_verbatim() {
        let err = eof()
            .with_context(BacktraceFrame::Message("reading the hull".into()))
            .with_context(field_frame("hull", "Ship"))
            .with_context(BacktraceFrame::Message("While parsing something".into()));
        assert_eq!(
            field_error(err, 0).path,
            path(&[
                "While parsing something",
                "Ship",
                "hull",
                "reading the hull"
            ])
        );
    }

    #[test]
    fn custom_frames_use_their_display_text() {
        let err = eof().with_context(BacktraceFrame::Custom(Box::new("custom note")));
        assert_eq!(field_error(err, 0).path, path(&["custom note"]));
    }

    #[test]
    fn eof_uses_the_read_start_offset() {
        let err = field_error(eof(), 42);
        assert_eq!(err.offset, 42);
        assert_eq!(err.cause, Cause::UnexpectedEnd);
        assert_eq!(err.path, FieldPath::default());
    }

    #[test]
    fn other_io_errors_are_invalid_at_the_read_start() {
        let err = field_error(Error::Io(io::Error::other("disk on fire")), 9);
        assert_eq!(err.offset, 9);
        assert_eq!(err.cause, Cause::Invalid("disk on fire".to_owned()));
    }

    #[test]
    fn positioned_errors_use_their_own_offset() {
        let assert = Error::AssertFail {
            pos: 0x1a2,
            message: "unterminated string".to_owned(),
        };
        let err = field_error(assert.with_context(field_frame("text", "Desc")), 0);
        assert_eq!(
            err,
            FieldError {
                path: path(&["Desc", "text"]),
                offset: 0x1a2,
                cause: Cause::Invalid("unterminated string".to_owned()),
            }
        );

        let custom = Error::Custom {
            pos: 5,
            err: Box::new("bad count"),
        };
        assert_eq!(
            field_error(custom, 0),
            FieldError {
                path: FieldPath::default(),
                offset: 5,
                cause: Cause::Invalid("bad count".to_owned()),
            }
        );

        let magic = Error::BadMagic {
            pos: 6,
            found: Box::new(0x1234_u16),
        };
        let err = field_error(magic, 0);
        assert_eq!(
            (err.offset, err.cause),
            (6, Cause::Invalid("bad magic: 4660".to_owned()))
        );

        let variant = Error::NoVariantMatch { pos: 8 };
        let err = field_error(variant, 0);
        assert_eq!(
            (err.offset, err.cause),
            (8, Cause::Invalid("no variant matched".to_owned()))
        );

        let enums = Error::EnumErrors {
            pos: 10,
            variant_errors: Vec::new(),
        };
        let err = field_error(enums, 0);
        assert_eq!(
            (err.offset, err.cause),
            (10, Cause::Invalid("no variant matched".to_owned()))
        );
    }

    fn sample_error(name: Option<&str>, expected_len: Option<usize>) -> DecodeError {
        DecodeError {
            res_type: SHIP,
            id: 128,
            name: name.map(str::to_owned),
            expected_len,
            actual_len: 400,
            field: FieldError {
                path: path(&["Ship", "weapons[2]", "count"]),
                offset: 0x1a2,
                cause: Cause::UnexpectedEnd,
            },
        }
    }

    #[test]
    fn decode_error_names_resource_field_offset_and_lengths() {
        assert_eq!(
            sample_error(Some("Shuttle"), Some(1860)).to_string(),
            "shïp 128 \"Shuttle\": field Ship → weapons[2] → count at byte 0x1a2: \
             unexpected end of data (record is 400 bytes, layout needs 1860)"
        );
    }

    #[test]
    fn decode_error_without_name_or_fixed_size() {
        assert_eq!(
            sample_error(None, None).to_string(),
            "shïp 128: field Ship → weapons[2] → count at byte 0x1a2: \
             unexpected end of data (record is 400 bytes)"
        );
    }

    #[test]
    fn field_error_without_a_path_and_invalid_cause() {
        let err = FieldError {
            path: FieldPath::default(),
            offset: 0,
            cause: Cause::Invalid("unterminated string".to_owned()),
        };
        assert_eq!(err.to_string(), "at byte 0x0: unterminated string");
    }

    #[test]
    fn trailing_bytes_warning_says_how_many() {
        let warning = DecodeWarning::TrailingBytes {
            res_type: SHIP,
            id: -5,
            name: Some("Big".to_owned()),
            consumed: 1860,
            actual_len: 1864,
        };
        assert_eq!(
            warning.to_string(),
            "shïp -5 \"Big\": 4 trailing bytes ignored (layout needs 1860, record is 1864 bytes)"
        );
    }
}
