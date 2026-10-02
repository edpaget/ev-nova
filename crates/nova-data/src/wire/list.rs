//! Counted lists whose errors name the failing element.
//!
//! `binrw` adds no context per list element, so an error in the third string
//! of a `STR#` would only say "strings". [`indexed`] reads element by element
//! and tags a failure with an index frame (`[2]`), which the error path
//! builder joins to the field name (`strings[2]`).

use std::io::{Read, Seek};

use binrw::error::{BacktraceFrame, ContextExt};
use binrw::{BinResult, Endian};

/// Reads `count` elements with `read`, tagging a failure with its index.
///
/// For `#[br(parse_with = indexed(count, element_parser))]`.
pub fn indexed<R, T, Args, F>(
    count: usize,
    read: F,
) -> impl Fn(&mut R, Endian, Args) -> BinResult<Vec<T>>
where
    R: Read + Seek,
    Args: Clone,
    F: Fn(&mut R, Endian, Args) -> BinResult<T>,
{
    move |reader, endian, args| {
        let mut items = Vec::with_capacity(count);
        for index in 0..count {
            let item = read(reader, endian, args.clone())
                .with_context(BacktraceFrame::Message(format!("[{index}]").into()))?;
            items.push(item);
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{Cause, field_error};
    use crate::wire::string::{MacString, pascal};
    use std::io::Cursor;

    fn strings(count: usize, bytes: &[u8]) -> (BinResult<Vec<MacString>>, u64) {
        let mut cursor = Cursor::new(bytes);
        let result = indexed(count, pascal)(&mut cursor, Endian::Big, ());
        (result, cursor.position())
    }

    #[test]
    fn reads_count_elements_in_order() {
        let (result, pos) = strings(3, b"\x01a\x00\x02bc\x09");
        assert_eq!(
            result.expect("decodes"),
            vec![
                MacString::from("a"),
                MacString::from(""),
                MacString::from("bc")
            ]
        );
        assert_eq!(pos, 6, "stops after the last element");
    }

    #[test]
    fn zero_count_reads_nothing() {
        let (result, pos) = strings(0, b"\x01a");
        assert_eq!(result.expect("decodes"), Vec::<MacString>::new());
        assert_eq!(pos, 0);
    }

    #[test]
    fn a_failing_element_is_tagged_with_its_index() {
        let (result, _) = strings(3, b"\x01a\x01b\x05c");
        let err = field_error(result.expect_err("third string is short"), 0);
        assert_eq!(err.path.0, vec!["[2]".to_owned()]);
        assert_eq!(err.cause, Cause::UnexpectedEnd);
    }
}
