//! The pure core: validates a flattened resource fork and indexes it.
//!
//! Nothing here does I/O. Every offset is bounds-checked and every declared
//! entry is read, so a fork either parses completely or is rejected.

use std::collections::HashSet;
use std::ops::Range;
use std::sync::OnceLock;

use encoding_rs::MACINTOSH;

use crate::{ParseError, ResType, Section};

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
    /// The decoded name, filled on first use: decoding while parsing would
    /// let many references to one long name multiply its size.
    pub(crate) name: OnceLock<String>,
    pub(crate) name_bytes: Option<Range<usize>>,
    pub(crate) attributes: u8,
    pub(crate) data: Range<usize>,
}

/// Name offset meaning "unnamed".
const NO_NAME: u16 = 0xFFFF;
/// Attribute bit for a compressed (`dcmp`) resource.
const COMPRESSED: u8 = 0x01;
/// The map header, up to and including the name-list offset.
const MAP_HEADER_LEN: usize = 28;
const TYPE_LIST_OFFSET_FIELD: usize = 24;
const NAME_LIST_OFFSET_FIELD: usize = 26;
const TYPE_ENTRY_LEN: usize = 8;
const REF_ENTRY_LEN: usize = 12;

/// Parses and validates a flattened resource fork.
pub(crate) fn parse(bytes: &[u8]) -> Result<Index, ParseError> {
    let [
        d0,
        d1,
        d2,
        d3,
        m0,
        m1,
        m2,
        m3,
        l0,
        l1,
        l2,
        l3,
        n0,
        n1,
        n2,
        n3,
    ] = read::<16>(bytes, 0).ok_or(ParseError::HeaderTruncated)?;
    let data = section(
        Section::Data,
        u32::from_be_bytes([d0, d1, d2, d3]),
        u32::from_be_bytes([l0, l1, l2, l3]),
        bytes.len(),
    )?;
    let map = section(
        Section::Map,
        u32::from_be_bytes([m0, m1, m2, m3]),
        u32::from_be_bytes([n0, n1, n2, n3]),
        bytes.len(),
    )?;
    let fork = Fork {
        bytes,
        data,
        map: map.clone(),
    };
    fork.parse_map(&bytes[map])
}

/// Checks that a section named by the header lies within the file.
fn section(
    section: Section,
    offset: u32,
    len: u32,
    file_len: usize,
) -> Result<Range<usize>, ParseError> {
    let start = offset as usize;
    match start.checked_add(len as usize) {
        Some(end) if end <= file_len => Ok(start..end),
        _ => Err(ParseError::SectionOutOfBounds {
            section,
            offset,
            len,
            file_len,
        }),
    }
}

/// A fork whose header sections are known to lie within `bytes`.
struct Fork<'a> {
    bytes: &'a [u8],
    data: Range<usize>,
    map: Range<usize>,
}

impl Fork<'_> {
    fn parse_map(&self, map: &[u8]) -> Result<Index, ParseError> {
        if map.len() < MAP_HEADER_LEN + 2 {
            return Err(ParseError::MapTruncated);
        }
        let type_list =
            usize::from(u16_at(map, TYPE_LIST_OFFSET_FIELD).ok_or(ParseError::MapTruncated)?);
        let name_list =
            usize::from(u16_at(map, NAME_LIST_OFFSET_FIELD).ok_or(ParseError::MapTruncated)?);
        let names = map
            .get(name_list..)
            .ok_or(ParseError::NameListOutOfBounds)?;
        let names_start = self.map.start + name_list;

        // Stored minus one, so 0xFFFF means no types at all.
        let type_count = usize::from(
            u16_at(map, type_list)
                .ok_or(ParseError::TypeListOutOfBounds)?
                .wrapping_add(1),
        );
        let entries_start = type_list + 2;
        let type_entries = map
            .get(entries_start..entries_start + TYPE_ENTRY_LEN * type_count)
            .ok_or(ParseError::TypeListOutOfBounds)?;

        // First every type's reference list, so that lists sharing bytes are
        // rejected before any reference is read: disjoint lists bound the
        // total number of references by the map's size.
        let mut seen = HashSet::with_capacity(type_count);
        let mut lists: Vec<(ResType, Range<usize>)> = Vec::with_capacity(type_count);
        for &[t0, t1, t2, t3, c0, c1, r0, r1] in type_entries.as_chunks::<TYPE_ENTRY_LEN>().0 {
            let ty = ResType([t0, t1, t2, t3]);
            if !seen.insert(ty) {
                return Err(ParseError::DuplicateType { ty });
            }
            // Also stored minus one.
            let count = usize::from(u16::from_be_bytes([c0, c1])) + 1;
            let refs_start = type_list + usize::from(u16::from_be_bytes([r0, r1]));
            let refs = refs_start..refs_start + REF_ENTRY_LEN * count;
            if map.get(refs.clone()).is_none() {
                return Err(ParseError::ReferenceListOutOfBounds { ty });
            }
            lists.push((ty, refs));
        }
        reject_overlaps(&lists)?;

        let mut types: Vec<TypeEntry> = Vec::with_capacity(type_count);
        for (ty, refs) in lists {
            let mut ids = HashSet::with_capacity(refs.len() / REF_ENTRY_LEN);
            let mut entries: Vec<Entry> = Vec::with_capacity(refs.len() / REF_ENTRY_LEN);
            for reference in map[refs].as_chunks::<REF_ENTRY_LEN>().0 {
                let entry = self.parse_reference(ty, reference, names, names_start)?;
                if !ids.insert(entry.id) {
                    return Err(ParseError::DuplicateId { ty, id: entry.id });
                }
                entries.push(entry);
            }
            types.push(TypeEntry { ty, entries });
        }
        Ok(Index { types })
    }

    fn parse_reference(
        &self,
        ty: ResType,
        reference: &[u8; REF_ENTRY_LEN],
        names: &[u8],
        names_start: usize,
    ) -> Result<Entry, ParseError> {
        let [i0, i1, n0, n1, attributes, o0, o1, o2, ..] = read::<REF_ENTRY_LEN>(reference, 0)
            .ok_or(ParseError::ReferenceListOutOfBounds { ty })?;
        let id = i16::from_be_bytes([i0, i1]);
        if attributes & COMPRESSED != 0 {
            return Err(ParseError::CompressedResource { ty, id });
        }
        let data = self.data_range(ty, id, u32::from_be_bytes([0, o0, o1, o2]))?;
        let name_bytes = name_range(ty, id, u16::from_be_bytes([n0, n1]), names)?
            .map(|r| names_start + r.start..names_start + r.end);
        Ok(Entry {
            id,
            name: OnceLock::new(),
            name_bytes,
            attributes,
            data,
        })
    }

    /// The absolute range of a resource's data, from its offset in the data
    /// section.
    fn data_range(&self, ty: ResType, id: i16, offset: u32) -> Result<Range<usize>, ParseError> {
        let section = &self.bytes[self.data.clone()];
        let start = offset as usize;
        let len =
            u32_at(section, start).ok_or(ParseError::DataOffsetOutOfBounds { ty, id, offset })?;
        let body = start + 4..start + 4 + len as usize;
        if section.get(body.clone()).is_none() {
            return Err(ParseError::DataLengthOutOfBounds { ty, id, len });
        }
        Ok(self.data.start + body.start..self.data.start + body.end)
    }
}

/// Fails if any two types' reference lists share a byte.
///
/// Sorting by start (stably, so ties keep type-list order) puts any overlap
/// between neighbours; the later one is named.
fn reject_overlaps(lists: &[(ResType, Range<usize>)]) -> Result<(), ParseError> {
    let mut by_start: Vec<&(ResType, Range<usize>)> = lists.iter().collect();
    by_start.sort_by_key(|(_, refs)| refs.start);
    for pair in by_start.windows(2) {
        let [(other, earlier), (ty, later)] = pair else {
            unreachable!("windows(2) yields pairs")
        };
        if later.start < earlier.end {
            return Err(ParseError::OverlappingReferenceLists {
                ty: *ty,
                other: *other,
            });
        }
    }
    Ok(())
}

/// A name's range within the name list, or `None` if unnamed.
fn name_range(
    ty: ResType,
    id: i16,
    offset: u16,
    names: &[u8],
) -> Result<Option<Range<usize>>, ParseError> {
    if offset == NO_NAME {
        return Ok(None);
    }
    let pos = usize::from(offset);
    let len = *names
        .get(pos)
        .ok_or(ParseError::NameOffsetOutOfBounds { ty, id, offset })?;
    let body = pos + 1..pos + 1 + usize::from(len);
    if names.get(body.clone()).is_none() {
        return Err(ParseError::NameLengthOutOfBounds { ty, id });
    }
    Ok(Some(body))
}

/// Decodes a Mac Roman name.
pub(crate) fn decode_name(bytes: &[u8]) -> String {
    MACINTOSH.decode_without_bom_handling(bytes).0.into_owned()
}

/// `N` bytes at `pos`, or `None` if they run past the end.
fn read<const N: usize>(bytes: &[u8], pos: usize) -> Option<[u8; N]> {
    bytes.get(pos..pos.checked_add(N)?)?.try_into().ok()
}

fn u16_at(bytes: &[u8], pos: usize) -> Option<u16> {
    read(bytes, pos).map(u16::from_be_bytes)
}

fn u32_at(bytes: &[u8], pos: usize) -> Option<u32> {
    read(bytes, pos).map(u32::from_be_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Section;
    use crate::fixture::{
        BuiltFork, DATA_LEN_FIELD, DATA_OFFSET_FIELD, ForkBuilder, MAP_LEN_FIELD, MAP_OFFSET_FIELD,
        NAME_LIST_OFFSET_FIELD, REF_ATTRIBUTES_FIELD, REF_DATA_OFFSET_FIELD, REF_LIST_OFFSET_FIELD,
        REF_NAME_OFFSET_FIELD, TYPE_COUNT_FIELD, TYPE_LIST_OFFSET_FIELD,
    };

    const PICT: ResType = ResType(*b"PICT");
    const SHIP: ResType = ResType([b's', b'h', 0x95, b'p']);

    /// Two types; the last data entry is empty and ends exactly at the end of
    /// the data section, and the last name ends exactly at the end of the map.
    fn sample() -> BuiltFork {
        ForkBuilder::new()
            .resource(SHIP, 128, Some(b"Shuttle"), b"ship-128")
            .resource(SHIP, 129, Some(b"\x8Aa"), b"ship-129")
            .resource(PICT, -5, None, b"")
            .build()
    }

    /// One type, no names: its reference list ends exactly at the map end.
    fn unnamed() -> BuiltFork {
        ForkBuilder::new()
            .resource(PICT, 1, None, b"one")
            .resource(PICT, 2, None, b"two")
            .build()
    }

    fn err(built: &BuiltFork) -> ParseError {
        parse(&built.bytes).expect_err("corrupt fork is rejected")
    }

    fn ok(built: &BuiltFork) -> Index {
        parse(&built.bytes).expect("fork is valid")
    }

    fn u32_of(n: usize) -> u32 {
        u32::try_from(n).expect("fits")
    }

    fn u16_of(n: usize) -> u16 {
        u16::try_from(n).expect("fits")
    }

    #[test]
    fn samples_are_valid() {
        let index = ok(&sample());
        assert_eq!(index.types.len(), 2);
        assert_eq!(ok(&unnamed()).types[0].entries.len(), 2);
    }

    #[test]
    fn rejects_a_truncated_header() {
        for len in 0..16 {
            assert_eq!(
                parse(&vec![0; len]).err(),
                Some(ParseError::HeaderTruncated)
            );
        }
    }

    #[test]
    fn rejects_a_data_section_past_eof() {
        let mut built = sample();
        let file_len = built.bytes.len();
        let offset = u32_of(built.layout.data_offset);
        let fits = u32_of(file_len - built.layout.data_offset);
        built.put_u32(DATA_LEN_FIELD, fits);
        assert!(parse(&built.bytes).is_ok(), "data section ending at EOF");

        built.put_u32(DATA_LEN_FIELD, fits + 1);
        assert_eq!(
            err(&built),
            ParseError::SectionOutOfBounds {
                section: Section::Data,
                offset,
                len: fits + 1,
                file_len,
            }
        );
    }

    #[test]
    fn rejects_a_map_section_past_eof() {
        let mut built = sample();
        let file_len = built.bytes.len();
        let map_len = u32_of(built.layout.map_len);
        built.put_u32(MAP_LEN_FIELD, map_len + 1);
        assert_eq!(
            err(&built),
            ParseError::SectionOutOfBounds {
                section: Section::Map,
                offset: u32_of(built.layout.map_offset),
                len: map_len + 1,
                file_len,
            }
        );
    }

    #[test]
    fn rejects_a_map_offset_past_eof() {
        let mut built = sample();
        let file_len = built.bytes.len();
        built.put_u32(MAP_OFFSET_FIELD, u32_of(built.layout.map_offset + 1));
        assert!(matches!(
            err(&built),
            ParseError::SectionOutOfBounds { section: Section::Map, file_len: f, .. } if f == file_len
        ));
    }

    #[test]
    fn rejects_overflowing_section_bounds() {
        let mut built = sample();
        built
            .put_u32(DATA_OFFSET_FIELD, u32::MAX)
            .put_u32(DATA_LEN_FIELD, u32::MAX);
        assert!(matches!(
            err(&built),
            ParseError::SectionOutOfBounds {
                section: Section::Data,
                offset: u32::MAX,
                len: u32::MAX,
                ..
            }
        ));
    }

    #[test]
    fn rejects_a_map_shorter_than_its_header() {
        let mut built = ForkBuilder::new().build();
        assert_eq!(built.layout.map_len, 30);
        assert!(parse(&built.bytes).is_ok(), "30-byte map is the minimum");
        built.put_u32(MAP_LEN_FIELD, 29);
        assert_eq!(err(&built), ParseError::MapTruncated);
    }

    #[test]
    fn rejects_a_type_list_offset_outside_the_map() {
        // The empty fork's 2-byte type list ends exactly at the map end.
        let mut built = ForkBuilder::new().build();
        let field = built.layout.map_offset + TYPE_LIST_OFFSET_FIELD;
        built.put_u16(field, 29);
        assert_eq!(err(&built), ParseError::TypeListOutOfBounds);
        built.put_u16(field, 0xFFFF);
        assert_eq!(err(&built), ParseError::TypeListOutOfBounds);
    }

    #[test]
    fn rejects_a_name_list_offset_outside_the_map() {
        // The empty fork's (empty) name list starts exactly at the map end.
        let mut built = ForkBuilder::new().build();
        let field = built.layout.map_offset + NAME_LIST_OFFSET_FIELD;
        built.put_u16(field, 31);
        assert_eq!(err(&built), ParseError::NameListOutOfBounds);
    }

    #[test]
    fn rejects_a_type_count_larger_than_the_map() {
        let mut built = unnamed();
        built.put_u16(built.layout.type_list, 0x7FFF);
        assert_eq!(err(&built), ParseError::TypeListOutOfBounds);
    }

    #[test]
    fn type_entries_must_fit_exactly() {
        // From the type list to the map end there are 34 bytes: the count,
        // one type entry and two references. Four declared types (stored as
        // 3) need exactly 34 bytes, so the type list itself fits (the extra
        // "entries" are garbage read from the references and fail later
        // checks); five need 42 and do not.
        let mut built = unnamed();
        let to_end = built.layout.map_offset + built.layout.map_len - built.layout.type_list;
        assert_eq!(to_end, 2 + 8 + 24);
        built.put_u16(built.layout.type_list, 3);
        assert_ne!(
            parse(&built.bytes).err(),
            Some(ParseError::TypeListOutOfBounds)
        );
        built.put_u16(built.layout.type_list, 4);
        assert_eq!(err(&built), ParseError::TypeListOutOfBounds);
    }

    #[test]
    fn rejects_a_reference_list_outside_the_map() {
        let mut built = sample();
        let field = built.layout.type_entry(PICT) + REF_LIST_OFFSET_FIELD;
        built.put_u16(field, 0xFFFF);
        assert_eq!(
            err(&built),
            ParseError::ReferenceListOutOfBounds { ty: PICT }
        );
    }

    #[test]
    fn reference_list_must_end_within_the_map() {
        // The only reference list ends exactly at the map end; one more
        // declared entry does not fit, even with bytes after the map.
        let mut built = ForkBuilder::new()
            .resource(PICT, 1, None, b"one")
            .resource(PICT, 2, None, b"two")
            .trailing(64)
            .build();
        assert!(parse(&built.bytes).is_ok());
        built.put_u16(built.layout.type_entry(PICT) + TYPE_COUNT_FIELD, 2);
        assert_eq!(
            err(&built),
            ParseError::ReferenceListOutOfBounds { ty: PICT }
        );
    }

    #[test]
    fn rejects_two_types_sharing_one_reference_list() {
        let mut built = sample();
        let ship = built.layout.type_entry(SHIP) + REF_LIST_OFFSET_FIELD;
        let shared = u16_at(&built.bytes, ship).expect("in bounds");
        built.put_u16(
            built.layout.type_entry(PICT) + REF_LIST_OFFSET_FIELD,
            shared,
        );
        assert_eq!(
            err(&built),
            ParseError::OverlappingReferenceLists {
                ty: PICT,
                other: SHIP
            }
        );
    }

    #[test]
    fn rejects_reference_lists_overlapping_by_one_byte() {
        // SHIP's two references are directly followed by PICT's one. Moving
        // PICT's list back one byte overlaps SHIP's last byte; moving it
        // forward is still disjoint (and lands in garbage that fails later).
        let mut built = sample();
        let field = built.layout.type_entry(PICT) + REF_LIST_OFFSET_FIELD;
        let adjacent = u16_at(&built.bytes, field).expect("in bounds");
        built.put_u16(field, adjacent - 1);
        assert_eq!(
            err(&built),
            ParseError::OverlappingReferenceLists {
                ty: PICT,
                other: SHIP
            }
        );
        built.put_u16(field, adjacent + 1);
        assert!(!matches!(
            parse(&built.bytes),
            Err(ParseError::OverlappingReferenceLists { .. })
        ));
    }

    #[test]
    fn overlap_is_found_whatever_the_map_order() {
        // The later list in byte order is named, even when it comes first in
        // the type list.
        let mut built = sample();
        let pict = built.layout.type_entry(PICT) + REF_LIST_OFFSET_FIELD;
        let ship = built.layout.type_entry(SHIP) + REF_LIST_OFFSET_FIELD;
        let pict_list = u16_at(&built.bytes, pict).expect("in bounds");
        // SHIP's two-entry list now starts 12 bytes before PICT's and so
        // covers it.
        built.put_u16(ship, pict_list - 12);
        assert_eq!(
            err(&built),
            ParseError::OverlappingReferenceLists {
                ty: PICT,
                other: SHIP
            }
        );
    }

    /// The denial-of-service shape: `types` distinct types all sharing one
    /// reference list of 65,536 distinct IDs, each with the same empty data
    /// and the same 255-byte name.
    fn shared_list_bomb(types: u16) -> Vec<u8> {
        // One empty data entry, shared by every reference.
        let data = [0u8; 4];
        let type_list_len = 2 + TYPE_ENTRY_LEN * usize::from(types);
        // The name list starts at the map itself (offset 0), whose first
        // (reserved) byte is the length of a 255-byte name: every reference
        // names it with offset 0.
        let mut map = vec![0; TYPE_LIST_OFFSET_FIELD];
        map[0] = 0xFF;
        map.extend(u16_of(MAP_HEADER_LEN).to_be_bytes());
        map.extend(0u16.to_be_bytes());
        map.extend((types - 1).to_be_bytes());
        for t in 0..types {
            map.extend(u32::from(t).to_be_bytes());
            // 65,536 references (stored minus one), all in one list.
            map.extend(0xFFFFu16.to_be_bytes());
            map.extend(u16_of(type_list_len).to_be_bytes());
        }
        for id in 0..=u16::MAX {
            map.extend(id.to_be_bytes());
            map.extend([0, 0]); // name offset
            map.extend([0; 4]); // attributes and data offset
            map.extend([0; 4]); // reserved handle
        }
        let mut bytes = Vec::new();
        bytes.extend(16u32.to_be_bytes());
        bytes.extend(u32_of(16 + data.len()).to_be_bytes());
        bytes.extend(u32_of(data.len()).to_be_bytes());
        bytes.extend(u32_of(map.len()).to_be_bytes());
        bytes.extend(data);
        bytes.extend(map);
        bytes
    }

    #[test]
    fn a_shared_reference_list_bomb_is_rejected_cheaply() {
        let bytes = shared_list_bomb(8000);
        assert!(bytes.len() < 900_000);
        // One type alone is a legitimate 65,536-resource fork...
        let one = parse(&shared_list_bomb(1)).expect("one type is valid");
        assert_eq!(one.types[0].entries.len(), 0x1_0000);
        // ...but thousands sharing its list are rejected before any
        // reference is read.
        assert_eq!(
            parse(&bytes).err(),
            Some(ParseError::OverlappingReferenceLists {
                ty: ResType([0, 0, 0, 1]),
                other: ResType([0, 0, 0, 0]),
            })
        );
    }

    #[test]
    fn reads_every_declared_reference() {
        // Declaring one fewer entry is still structurally valid; the parsed
        // count always equals the declared count.
        let mut built = unnamed();
        built.put_u16(built.layout.type_entry(PICT) + TYPE_COUNT_FIELD, 0);
        let index = ok(&built);
        assert_eq!(index.types[0].entries.len(), 1);
        assert_eq!(index.types[0].entries[0].id, 1);
    }

    #[test]
    fn rejects_a_data_offset_outside_the_data_section() {
        let mut built = sample();
        let field = built.layout.reference(SHIP, 129) + REF_DATA_OFFSET_FIELD;
        built.put_u24(field, 0x00FF_FFFF);
        assert_eq!(
            err(&built),
            ParseError::DataOffsetOutOfBounds {
                ty: SHIP,
                id: 129,
                offset: 0x00FF_FFFF
            }
        );
    }

    #[test]
    fn data_length_prefix_must_fit_in_the_data_section() {
        // PICT -5 is empty and its prefix ends exactly at the data end.
        let mut built = sample();
        let data_len = u32_of(built.layout.data_len);
        let last = u32_of(built.layout.data_entry(PICT, -5) - built.layout.data_offset);
        assert_eq!(last + 4, data_len);
        built.put_u32(DATA_LEN_FIELD, data_len - 1);
        assert_eq!(
            err(&built),
            ParseError::DataOffsetOutOfBounds {
                ty: PICT,
                id: -5,
                offset: last
            }
        );
    }

    #[test]
    fn data_offset_at_section_end_is_out_of_bounds() {
        let mut built = sample();
        let data_len = u32_of(built.layout.data_len);
        let field = built.layout.reference(PICT, -5) + REF_DATA_OFFSET_FIELD;
        built.put_u24(field, data_len - 3);
        assert!(matches!(
            err(&built),
            ParseError::DataOffsetOutOfBounds { .. }
        ));
        built.put_u24(field, data_len);
        assert!(matches!(
            err(&built),
            ParseError::DataOffsetOutOfBounds { .. }
        ));
    }

    #[test]
    fn rejects_data_running_past_the_data_section() {
        // The data section must contain the data even though the map follows.
        let mut built = unnamed();
        let entry = built.layout.data_entry(PICT, 2);
        built.put_u32(entry, 4);
        assert_eq!(
            err(&built),
            ParseError::DataLengthOutOfBounds {
                ty: PICT,
                id: 2,
                len: 4
            }
        );
        built.put_u32(entry, u32::MAX);
        assert_eq!(
            err(&built),
            ParseError::DataLengthOutOfBounds {
                ty: PICT,
                id: 2,
                len: u32::MAX
            }
        );
    }

    #[test]
    fn unnamed_sentinel_is_not_an_error() {
        let index = ok(&sample());
        let pict = &index.types[1].entries[0];
        assert_eq!(pict.name_bytes, None);
    }

    #[test]
    fn rejects_a_name_offset_outside_the_name_list() {
        let mut built = sample();
        let field = built.layout.reference(SHIP, 128) + REF_NAME_OFFSET_FIELD;
        built.put_u16(field, 0x7FFF);
        assert_eq!(
            err(&built),
            ParseError::NameOffsetOutOfBounds {
                ty: SHIP,
                id: 128,
                offset: 0x7FFF
            }
        );
    }

    #[test]
    fn name_offset_at_map_end_is_out_of_bounds() {
        let mut built = sample();
        let names_len = built.layout.map_offset + built.layout.map_len - built.layout.name_list;
        let field = built.layout.reference(SHIP, 128) + REF_NAME_OFFSET_FIELD;
        built.put_u16(field, u16_of(names_len));
        assert_eq!(
            err(&built),
            ParseError::NameOffsetOutOfBounds {
                ty: SHIP,
                id: 128,
                offset: u16_of(names_len)
            }
        );
        // The last byte holds 'a' (0x61), read as a length: too long.
        built.put_u16(field, u16_of(names_len - 1));
        assert_eq!(
            err(&built),
            ParseError::NameLengthOutOfBounds { ty: SHIP, id: 128 }
        );
    }

    #[test]
    fn rejects_a_name_running_past_the_map() {
        // The last name ends exactly at the map end; bytes after the map do
        // not count.
        let mut built = ForkBuilder::new()
            .resource(PICT, 7, Some(b"ab"), b"")
            .trailing(16)
            .build();
        assert!(parse(&built.bytes).is_ok());
        built.put_u8(built.layout.name(PICT, 7), 3);
        assert_eq!(
            err(&built),
            ParseError::NameLengthOutOfBounds { ty: PICT, id: 7 }
        );
    }

    #[test]
    fn names_are_read_relative_to_the_name_list() {
        let index = ok(&sample());
        let ship = &index.types[0].entries;
        let built = sample();
        let pos = built.layout.name(SHIP, 128);
        assert_eq!(ship[0].name_bytes, Some(pos + 1..pos + 8));
        let pos = built.layout.name(SHIP, 129);
        assert_eq!(ship[1].name_bytes, Some(pos + 1..pos + 3));
    }

    #[test]
    fn names_are_not_decoded_while_parsing() {
        // Decoding (and allocating) every name up front would let many
        // references naming one long name multiply its size.
        let index = ok(&sample());
        for entry in index.types.iter().flat_map(|t| &t.entries) {
            assert_eq!(entry.name.get(), None);
        }
    }

    #[test]
    fn data_ranges_are_absolute() {
        let built = sample();
        let index = ok(&built);
        let entry = built.layout.data_entry(SHIP, 129);
        assert_eq!(index.types[0].entries[1].data, entry + 4..entry + 12);
        let pict = built.layout.data_entry(PICT, -5);
        assert_eq!(index.types[1].entries[0].data, pict + 4..pict + 4);
    }

    #[test]
    fn rejects_a_duplicate_type() {
        let mut built = sample();
        let pict = built.layout.type_entry(PICT);
        for (i, byte) in SHIP.0.into_iter().enumerate() {
            built.put_u8(pict + i, byte);
        }
        assert_eq!(err(&built), ParseError::DuplicateType { ty: SHIP });
    }

    #[test]
    fn rejects_a_duplicate_id() {
        let built = ForkBuilder::new()
            .resource(PICT, 1, None, b"a")
            .resource(SHIP, 1, None, b"b")
            .resource(PICT, 2, None, b"c")
            .resource(PICT, 1, None, b"d")
            .build();
        assert_eq!(err(&built), ParseError::DuplicateId { ty: PICT, id: 1 });
    }

    #[test]
    fn rejects_compressed_resources() {
        let built = ForkBuilder::new()
            .resource(PICT, 1, None, b"a")
            .resource(PICT, 2, None, b"b")
            .attributes(0x01)
            .build();
        assert_eq!(
            err(&built),
            ParseError::CompressedResource { ty: PICT, id: 2 }
        );
    }

    #[test]
    fn keeps_other_attribute_bits() {
        let mut built = unnamed();
        built.put_u8(built.layout.reference(PICT, 2) + REF_ATTRIBUTES_FIELD, 0xFE);
        let index = ok(&built);
        assert_eq!(index.types[0].entries[1].attributes, 0xFE);
        assert_eq!(index.types[0].entries[1].data, {
            let e = built.layout.data_entry(PICT, 2);
            e + 4..e + 7
        });
    }

    #[test]
    fn every_truncation_is_an_error_without_panicking() {
        let built = sample();
        for len in 0..built.bytes.len() {
            assert!(parse(&built.bytes[..len]).is_err(), "truncated to {len}");
        }
    }

    #[test]
    fn single_byte_corruption_never_panics() {
        let built = sample();
        for pos in 0..built.bytes.len() {
            for value in [0x00, 0x01, 0x7F, 0x80, 0xFF] {
                let mut bytes = built.bytes.clone();
                bytes[pos] = value;
                let _ = parse(&bytes);
            }
        }
    }
}
