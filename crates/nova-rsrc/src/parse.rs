//! The pure core: validates a flattened resource fork and indexes it.
//!
//! Nothing here does I/O. Every offset is bounds-checked and every declared
//! entry is read, so a fork either parses completely or is rejected.

use std::ops::Range;

use encoding_rs::MACINTOSH;

use crate::{ParseError, ResType};

/// A validated index into a fork's bytes.
#[derive(Debug, Default)]
pub(crate) struct Index {
    pub(crate) types: Vec<TypeEntry>,
}

/// One type and its resources, in reference-list order.
#[derive(Debug)]
pub(crate) struct TypeEntry {
    pub(crate) ty: ResType,
    pub(crate) entries: Vec<Entry>,
}

/// One resource: validated ranges into the fork's bytes.
#[derive(Debug)]
pub(crate) struct Entry {
    pub(crate) id: i16,
    pub(crate) name: Option<String>,
    pub(crate) name_bytes: Option<Range<usize>>,
    pub(crate) attributes: u8,
    pub(crate) data: Range<usize>,
}

const NO_NAME: u16 = 0xFFFF;

/// Parses and validates a flattened resource fork.
pub(crate) fn parse(bytes: &[u8]) -> Result<Index, ParseError> {
    let data_offset = u32_at(bytes, 0)? as usize;
    let map_offset = u32_at(bytes, 4)? as usize;
    let _data_len = u32_at(bytes, 8)? as usize;
    let map_len = u32_at(bytes, 12)? as usize;

    let map_end = map_offset + map_len;
    let map = map_offset;
    let type_list = map + usize::from(u16_at(bytes, map + 24)?);
    let name_list = map + usize::from(u16_at(bytes, map + 26)?);
    let type_count = usize::from(u16_at(bytes, type_list)?.wrapping_add(1));

    let mut types = Vec::with_capacity(type_count);
    for t in 0..type_count {
        let entry = type_list + 2 + 8 * t;
        let ty = ResType(
            get(bytes, entry..entry + 4)?
                .try_into()
                .map_err(|_| ParseError::HeaderTruncated)?,
        );
        let count = usize::from(u16_at(bytes, entry + 4)?) + 1;
        let refs = type_list + usize::from(u16_at(bytes, entry + 6)?);
        let mut entries = Vec::with_capacity(count);
        for r in 0..count {
            let reference = refs + 12 * r;
            let id = u16_at(bytes, reference)? as i16;
            let name_offset = u16_at(bytes, reference + 2)?;
            let attributes = *get(bytes, reference + 4..reference + 5)?
                .first()
                .ok_or(ParseError::HeaderTruncated)?;
            let data_rel = u32_at(bytes, reference + 4)? & 0x00FF_FFFF;
            let data_entry = data_offset + data_rel as usize;
            let len = u32_at(bytes, data_entry)? as usize;
            let data = data_entry + 4..data_entry + 4 + len;
            get(bytes, data.clone())?;
            let name_bytes = if name_offset == NO_NAME {
                None
            } else {
                let pos = name_list + usize::from(name_offset);
                let len = usize::from(
                    *get(bytes, pos..pos + 1)?
                        .first()
                        .ok_or(ParseError::HeaderTruncated)?,
                );
                let range = pos + 1..pos + 1 + len;
                get(bytes, range.clone())?;
                Some(range)
            };
            let name = name_bytes.clone().map(|range| {
                MACINTOSH
                    .decode_without_bom_handling(&bytes[range])
                    .0
                    .into_owned()
            });
            entries.push(Entry {
                id,
                name,
                name_bytes,
                attributes,
                data,
            });
        }
        types.push(TypeEntry { ty, entries });
    }
    let _ = map_end;
    Ok(Index { types })
}

fn get(bytes: &[u8], range: Range<usize>) -> Result<&[u8], ParseError> {
    bytes.get(range).ok_or(ParseError::HeaderTruncated)
}

fn u16_at(bytes: &[u8], pos: usize) -> Result<u16, ParseError> {
    let b = get(bytes, pos..pos + 2)?;
    Ok(u16::from_be_bytes([b[0], b[1]]))
}

fn u32_at(bytes: &[u8], pos: usize) -> Result<u32, ParseError> {
    let b = get(bytes, pos..pos + 4)?;
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}
