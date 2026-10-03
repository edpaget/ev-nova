//! The pure core for Windows `.rez` files: validates one and indexes it.
//!
//! Builds the same [`Index`] as the resource-fork parser, so
//! [`ResourceFile`](crate::ResourceFile) serves both formats through one
//! view. Nothing here does I/O; every count and offset is bounds-checked
//! with checked arithmetic, and every declared resource is read, so a file
//! either parses completely or is rejected.

use std::collections::HashSet;
use std::ops::Range;
use std::sync::OnceLock;

use crate::parse::{Entry, Index, TypeEntry, read, reject_overlaps};
use crate::{ParseError, ResType};

/// The magic number that opens a `.rez` file.
pub(crate) const MAGIC: &[u8; 4] = b"BRGR";

/// The root header (magic, group count, header length): the header length
/// and name offsets count from its end.
const ROOT_HEADER_LEN: usize = 12;
/// The root header and the group header, before the entry table.
const HEADER_LEN: usize = 24;
const ENTRY_LEN: usize = 12;
const MAP_HEADER_LEN: usize = 8;
const TYPE_ENTRY_LEN: usize = 12;
const RESOURCE_ENTRY_LEN: usize = 266;
/// Where the name field starts in a resource entry.
const NAME_FIELD: usize = 10;
/// The name of the last entry, which holds the resource map.
const MAP_NAME: &[u8] = b"resource.map";

/// Parses and validates a `.rez` file.
pub(crate) fn parse(bytes: &[u8]) -> Result<Index, ParseError> {
    let header = read::<HEADER_LEN>(bytes, 0).ok_or(ParseError::RezHeaderTruncated)?;
    let field = |i: usize| u32::from_le_bytes(header.as_chunks::<4>().0[i]);
    let (group_count, header_len, group_type, base, count) =
        (field(1), field(2), field(3), field(4), field(5));
    if group_count != 1 {
        return Err(ParseError::RezGroupCount { count: group_count });
    }
    if group_type != 1 {
        return Err(ParseError::RezGroupType { group_type });
    }
    if count == 0 {
        return Err(ParseError::RezNoEntries);
    }

    // The header (counted from the end of the root header) must fit in the
    // file, and the entry table in the header; the name table fills the rest.
    let file_len = bytes.len();
    let table_out = ParseError::RezEntryTableOutOfBounds { count, file_len };
    let header_end = (header_len as usize)
        .checked_add(ROOT_HEADER_LEN)
        .filter(|&end| end <= file_len)
        .ok_or_else(|| table_out.clone())?;
    let table_end = (count as usize)
        .checked_mul(ENTRY_LEN)
        .and_then(|len| len.checked_add(HEADER_LEN))
        .filter(|&end| end <= header_end)
        .ok_or(table_out)?;

    // Every row's data must lie within the file. The table fits in the
    // file, so `count` is bounded by its size.
    let rows = bytes[HEADER_LEN..table_end].as_chunks::<ENTRY_LEN>().0;
    let mut entries: Vec<Range<usize>> = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let [offset, size, _] = row.as_chunks::<4>().0 else {
            unreachable!("a row is three u32s")
        };
        let (offset, size) = (u32::from_le_bytes(*offset), u32::from_le_bytes(*size));
        let start = offset as usize;
        let end = start
            .checked_add(size as usize)
            .filter(|&end| end <= file_len)
            .ok_or(ParseError::RezEntryOutOfBounds {
                index,
                offset,
                size,
                file_len,
            })?;
        entries.push(start..end);
    }

    let map_row = rows.last().expect("count is at least 1");
    let map_name = u32::from_le_bytes(map_row.as_chunks::<4>().0[2]);
    if name_in_table(&bytes[..header_end], table_end, map_name) != Some(MAP_NAME) {
        return Err(ParseError::RezMapNotNamed);
    }
    let map_range = entries.last().expect("count is at least 1").clone();
    let rez = Rez {
        entries: &entries,
        base,
        map_start: map_range.start,
    };
    rez.parse_map(&bytes[map_range])
}

/// The NUL-terminated name at `offset` (from the end of the root header),
/// or `None` unless it starts in the name table (from `table_end` to the
/// end of `header`) and ends there with a NUL.
fn name_in_table(header: &[u8], table_end: usize, offset: u32) -> Option<&[u8]> {
    let start = (offset as usize).checked_add(ROOT_HEADER_LEN)?;
    if start < table_end {
        return None;
    }
    let rest = header.get(start..)?;
    let len = rest.iter().position(|&b| b == 0)?;
    Some(&rest[..len])
}

/// A `.rez` file whose entry table is known to lie within the file.
struct Rez<'a> {
    /// Every entry's absolute data range; the last is the map.
    entries: &'a [Range<usize>],
    base: u32,
    /// The map's absolute offset.
    map_start: usize,
}

impl Rez<'_> {
    fn parse_map(&self, map: &[u8]) -> Result<Index, ParseError> {
        let [o0, o1, o2, o3, n0, n1, n2, n3] =
            read::<MAP_HEADER_LEN>(map, 0).ok_or(ParseError::MapTruncated)?;
        let type_list = u32::from_be_bytes([o0, o1, o2, o3]) as usize;
        let type_count = u32::from_be_bytes([n0, n1, n2, n3]) as usize;
        let type_entries = type_count
            .checked_mul(TYPE_ENTRY_LEN)
            .and_then(|len| type_list.checked_add(len))
            .and_then(|end| map.get(type_list..end))
            .ok_or(ParseError::TypeListOutOfBounds)?;

        // First every type's resource list, so that lists sharing bytes are
        // rejected before any resource is read: disjoint lists bound the
        // total number of resources by the map's size.
        let type_entries = type_entries.as_chunks::<TYPE_ENTRY_LEN>().0;
        let mut seen = HashSet::with_capacity(type_entries.len());
        let mut lists: Vec<(ResType, Range<usize>)> = Vec::with_capacity(type_entries.len());
        for &[t0, t1, t2, t3, l0, l1, l2, l3, c0, c1, c2, c3] in type_entries {
            let ty = ResType([t0, t1, t2, t3]);
            if !seen.insert(ty) {
                return Err(ParseError::DuplicateType { ty });
            }
            let start = u32::from_be_bytes([l0, l1, l2, l3]) as usize;
            let len =
                (u32::from_be_bytes([c0, c1, c2, c3]) as usize).checked_mul(RESOURCE_ENTRY_LEN);
            let list = len
                .and_then(|len| start.checked_add(len))
                .filter(|&end| end <= map.len())
                .map(|end| start..end)
                .ok_or(ParseError::ReferenceListOutOfBounds { ty })?;
            lists.push((ty, list));
        }
        reject_overlaps(&lists)?;

        let mut types: Vec<TypeEntry> = Vec::with_capacity(lists.len());
        for (ty, list) in lists {
            let resources = map[list.clone()].as_chunks::<RESOURCE_ENTRY_LEN>().0;
            let mut ids = HashSet::with_capacity(resources.len());
            let mut entries: Vec<Entry> = Vec::with_capacity(resources.len());
            for (i, resource) in resources.iter().enumerate() {
                let pos = self.map_start + list.start + RESOURCE_ENTRY_LEN * i;
                let entry = self.parse_resource(ty, resource, pos)?;
                if !ids.insert(entry.id) {
                    return Err(ParseError::DuplicateId { ty, id: entry.id });
                }
                entries.push(entry);
            }
            types.push(TypeEntry { ty, entries });
        }
        Ok(Index { types })
    }

    /// Reads the resource entry that starts at absolute offset `pos`.
    fn parse_resource(
        &self,
        ty: ResType,
        resource: &[u8; RESOURCE_ENTRY_LEN],
        pos: usize,
    ) -> Result<Entry, ParseError> {
        let [i0, i1, i2, i3, t0, t1, t2, t3, id0, id1, name_field @ ..] = resource;
        let id = i16::from_be_bytes([*id0, *id1]);
        let found = ResType([*t0, *t1, *t2, *t3]);
        if found != ty {
            return Err(ParseError::RezTypeMismatch { ty, id, found });
        }
        let len = name_field
            .iter()
            .position(|&b| b == 0)
            .ok_or(ParseError::RezUnterminatedName { ty, id })?;
        let name_start = pos + NAME_FIELD;
        let name_bytes = (len > 0).then(|| name_start..name_start + len);

        let index = u32::from_be_bytes([*i0, *i1, *i2, *i3]);
        let map_slot = self.entries.len() - 1;
        let data = match index.checked_sub(self.base).map(|slot| slot as usize) {
            Some(slot) if slot < map_slot => self.entries[slot].clone(),
            Some(slot) if slot == map_slot => {
                return Err(ParseError::RezResourceIsMap { ty, id });
            }
            _ => return Err(ParseError::RezEntryIndexOutOfRange { ty, id, index }),
        };
        Ok(Entry {
            id,
            name: OnceLock::new(),
            name_bytes,
            attributes: 0,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{
        BuiltRez, REZ_BASE_INDEX_FIELD, REZ_ENTRY_COUNT_FIELD, REZ_ENTRY_NAME_FIELD,
        REZ_ENTRY_OFFSET_FIELD, REZ_ENTRY_SIZE_FIELD, REZ_GROUP_COUNT_FIELD, REZ_GROUP_TYPE_FIELD,
        REZ_HEADER_LEN_FIELD, REZ_MAP_TYPE_COUNT_FIELD, REZ_MAP_TYPE_LIST_FIELD,
        REZ_RES_INDEX_FIELD, REZ_RES_NAME_FIELD, REZ_RES_TYPE_FIELD, REZ_ROOT_HEADER_LEN,
        REZ_TYPE_LIST_FIELD, REZ_TYPE_RES_COUNT_FIELD, RezBuilder,
    };

    const PICT: ResType = ResType(*b"PICT");
    const SHIP: ResType = ResType([b's', b'h', 0x95, b'p']);

    /// Two types; PICT 7 shares SHIP 128's data, PICT -5 is empty.
    fn sample() -> BuiltRez {
        RezBuilder::new()
            .resource(SHIP, 128, Some(b"Shuttle"), b"ship-128")
            .resource(SHIP, 129, Some(b"\x8Aa"), b"ship-129!")
            .resource(PICT, -5, None, b"")
            .resource(PICT, 7, None, b"ship-128")
            .build()
    }

    fn ok(built: &BuiltRez) -> Index {
        parse(&built.bytes).expect(".rez is valid")
    }

    /// `(type, [(id, name bytes, data)])` for every type, in index order.
    type Flat<'a> = Vec<(ResType, Vec<(i16, Option<&'a [u8]>, &'a [u8])>)>;

    fn flatten<'a>(bytes: &'a [u8], index: &Index) -> Flat<'a> {
        index
            .types
            .iter()
            .map(|t| {
                let entries = t
                    .entries
                    .iter()
                    .map(|e| {
                        (
                            e.id,
                            e.name_bytes.clone().map(|r| &bytes[r]),
                            &bytes[e.data.clone()],
                        )
                    })
                    .collect();
                (t.ty, entries)
            })
            .collect()
    }

    #[test]
    fn an_empty_file_has_no_types() {
        assert!(ok(&RezBuilder::new().build()).types.is_empty());
    }

    #[test]
    fn reads_types_ids_names_and_data_in_map_order() {
        let built = sample();
        assert_eq!(
            flatten(&built.bytes, &ok(&built)),
            vec![
                (
                    SHIP,
                    vec![
                        (128, Some(&b"Shuttle"[..]), &b"ship-128"[..]),
                        (129, Some(&b"\x8Aa"[..]), &b"ship-129!"[..]),
                    ]
                ),
                (
                    PICT,
                    vec![(-5, None, &b""[..]), (7, None, &b"ship-128"[..])]
                ),
            ]
        );
    }

    #[test]
    fn an_empty_name_is_no_name() {
        let built = RezBuilder::new().resource(PICT, 1, Some(b""), b"x").build();
        assert_eq!(ok(&built).types[0].entries[0].name_bytes, None);
    }

    #[test]
    fn a_name_may_fill_all_but_the_last_byte_of_its_field() {
        let name = [b'n'; 255];
        let built = RezBuilder::new()
            .resource(PICT, 1, Some(&name), b"x")
            .build();
        let index = ok(&built);
        let range = index.types[0].entries[0].name_bytes.clone().expect("named");
        assert_eq!(&built.bytes[range], &name[..]);
    }

    #[test]
    fn ranges_are_absolute() {
        let built = sample();
        let index = ok(&built);
        let layout = &built.layout;
        let name = layout.resource(SHIP, 129) + crate::fixture::REZ_RES_NAME_FIELD;
        assert_eq!(index.types[0].entries[1].name_bytes, Some(name..name + 2));
        let blob = layout.data_offset + 8;
        assert_eq!(index.types[0].entries[1].data, blob..blob + 9);
    }

    #[test]
    fn shared_data_is_one_range() {
        let index = ok(&sample());
        assert_eq!(
            index.types[0].entries[0].data,
            index.types[1].entries[1].data
        );
    }

    #[test]
    fn attributes_are_zero_and_names_are_not_decoded() {
        let index = ok(&sample());
        for entry in index.types.iter().flat_map(|t| &t.entries) {
            assert_eq!(entry.attributes, 0);
            assert_eq!(entry.name.get(), None);
        }
    }

    fn err(built: &BuiltRez) -> ParseError {
        parse(&built.bytes).expect_err("corrupt .rez is rejected")
    }

    fn u32_of(n: usize) -> u32 {
        u32::try_from(n).expect("fits")
    }

    fn le_at(bytes: &[u8], pos: usize) -> u32 {
        u32::from_le_bytes(read(bytes, pos).expect("in bounds"))
    }

    fn be_at(bytes: &[u8], pos: usize) -> u32 {
        u32::from_be_bytes(read(bytes, pos).expect("in bounds"))
    }

    /// The position of a field of entry-table row `index`.
    fn row(built: &BuiltRez, index: usize, field: usize) -> usize {
        built.layout.entry(index) + field
    }

    /// The position of the resource-list offset field of `ty`'s type entry.
    fn list_field(built: &BuiltRez, ty: ResType) -> usize {
        built.layout.type_entry(ty) + REZ_TYPE_LIST_FIELD
    }

    fn index_field(built: &BuiltRez, ty: ResType, id: i16) -> usize {
        built.layout.resource(ty, id) + REZ_RES_INDEX_FIELD
    }

    #[test]
    fn rejects_a_truncated_header() {
        let built = sample();
        for len in 0..24 {
            assert_eq!(
                parse(&built.bytes[..len]).err(),
                Some(ParseError::RezHeaderTruncated),
                "{len} bytes"
            );
        }
    }

    #[test]
    fn rejects_a_group_count_other_than_one() {
        for count in [0, 2, u32::MAX] {
            let mut built = sample();
            built.put_u32_le(REZ_GROUP_COUNT_FIELD, count);
            assert_eq!(err(&built), ParseError::RezGroupCount { count });
        }
    }

    #[test]
    fn rejects_a_group_type_other_than_one() {
        for group_type in [0, 2] {
            let mut built = sample();
            built.put_u32_le(REZ_GROUP_TYPE_FIELD, group_type);
            assert_eq!(err(&built), ParseError::RezGroupType { group_type });
        }
    }

    #[test]
    fn rejects_a_zero_entry_count() {
        let mut built = sample();
        built.put_u32_le(REZ_ENTRY_COUNT_FIELD, 0);
        assert_eq!(err(&built), ParseError::RezNoEntries);
    }

    #[test]
    fn rejects_an_entry_table_past_eof() {
        let mut built = sample();
        let file_len = built.bytes.len();
        for count in [u32_of(file_len / 12), u32::MAX] {
            built.put_u32_le(REZ_ENTRY_COUNT_FIELD, count);
            assert_eq!(
                err(&built),
                ParseError::RezEntryTableOutOfBounds { count, file_len }
            );
        }
    }

    #[test]
    fn the_entry_table_must_end_within_the_header() {
        // The header ends 13 bytes after the table (the map's name), so one
        // more row still fits (and reads the name as a row) but two do not.
        let mut built = sample();
        let file_len = built.bytes.len();
        built.put_u32_le(REZ_ENTRY_COUNT_FIELD, 5);
        assert!(!matches!(
            err(&built),
            ParseError::RezEntryTableOutOfBounds { .. }
        ));
        built.put_u32_le(REZ_ENTRY_COUNT_FIELD, 6);
        assert_eq!(
            err(&built),
            ParseError::RezEntryTableOutOfBounds { count: 6, file_len }
        );
    }

    #[test]
    fn a_header_ending_at_the_entry_table_leaves_no_name_table() {
        let mut built = sample();
        let file_len = built.bytes.len();
        let table_end = u32_of(built.layout.name_table - REZ_ROOT_HEADER_LEN);
        built.put_u32_le(REZ_HEADER_LEN_FIELD, table_end);
        assert_eq!(err(&built), ParseError::RezMapNotNamed);
        built.put_u32_le(REZ_HEADER_LEN_FIELD, table_end - 1);
        assert_eq!(
            err(&built),
            ParseError::RezEntryTableOutOfBounds { count: 4, file_len }
        );
    }

    #[test]
    fn rejects_a_header_length_past_eof() {
        let mut built = sample();
        let file_len = built.bytes.len();
        let to_eof = u32_of(file_len - REZ_ROOT_HEADER_LEN);
        built.put_u32_le(REZ_HEADER_LEN_FIELD, to_eof);
        assert!(parse(&built.bytes).is_ok(), "header ending at EOF");
        for len in [to_eof + 1, u32::MAX] {
            built.put_u32_le(REZ_HEADER_LEN_FIELD, len);
            assert_eq!(
                err(&built),
                ParseError::RezEntryTableOutOfBounds { count: 4, file_len }
            );
        }
    }

    #[test]
    fn rejects_an_entry_past_eof() {
        let mut built = sample();
        let file_len = built.bytes.len();
        let offset = le_at(&built.bytes, row(&built, 1, REZ_ENTRY_OFFSET_FIELD));
        let to_eof = u32_of(file_len) - offset;
        built.put_u32_le(row(&built, 1, REZ_ENTRY_SIZE_FIELD), to_eof);
        assert!(parse(&built.bytes).is_ok(), "data ending at EOF");
        built.put_u32_le(row(&built, 1, REZ_ENTRY_SIZE_FIELD), to_eof + 1);
        assert_eq!(
            err(&built),
            ParseError::RezEntryOutOfBounds {
                index: 1,
                offset,
                size: to_eof + 1,
                file_len
            }
        );
    }

    #[test]
    fn rejects_an_entry_whose_end_overflows() {
        let mut built = sample();
        let file_len = built.bytes.len();
        let (offset, size) = (
            row(&built, 2, REZ_ENTRY_OFFSET_FIELD),
            row(&built, 2, REZ_ENTRY_SIZE_FIELD),
        );
        built
            .put_u32_le(offset, u32::MAX)
            .put_u32_le(size, u32::MAX);
        assert_eq!(
            err(&built),
            ParseError::RezEntryOutOfBounds {
                index: 2,
                offset: u32::MAX,
                size: u32::MAX,
                file_len
            }
        );
    }

    #[test]
    fn rejects_a_map_entry_past_eof() {
        let mut built = sample();
        let size = u32_of(built.layout.map_len + 1);
        built.put_u32_le(row(&built, 3, REZ_ENTRY_SIZE_FIELD), size);
        assert!(matches!(
            err(&built),
            ParseError::RezEntryOutOfBounds { index: 3, .. }
        ));
    }

    #[test]
    fn rejects_a_map_entry_with_another_name() {
        let mut built = sample();
        let last = built.layout.map_name + "resource.ma".len();
        built.put_u8(last, b'b');
        assert_eq!(err(&built), ParseError::RezMapNotNamed);
        // A shorter name, a prefix of the right one.
        built.put_u8(last, 0);
        assert_eq!(err(&built), ParseError::RezMapNotNamed);
    }

    #[test]
    fn rejects_a_map_name_without_a_nul() {
        let mut built = sample();
        built.put_u8(built.layout.map_name + "resource.map".len(), b'x');
        assert_eq!(err(&built), ParseError::RezMapNotNamed);
    }

    #[test]
    fn rejects_a_map_name_offset_outside_the_name_table() {
        let mut built = sample();
        let field = row(&built, 3, REZ_ENTRY_NAME_FIELD);
        let start = u32_of(built.layout.name_table - REZ_ROOT_HEADER_LEN);
        let end = start + 13;
        for offset in [0, start - 1, end, u32::MAX] {
            built.put_u32_le(field, offset);
            assert_eq!(err(&built), ParseError::RezMapNotNamed, "offset {offset}");
        }
    }

    #[test]
    fn a_name_must_start_in_the_name_table() {
        // Twelve root-header bytes, a 13-byte "entry table" holding a valid
        // name, then the name table proper.
        let header = [&[0; 12][..], b"resource.map\0", b"resource.map\0"].concat();
        let table_end = 25;
        assert_eq!(name_in_table(&header, table_end, 13), Some(MAP_NAME));
        assert_eq!(
            name_in_table(&header, table_end, 14),
            Some(&b"esource.map"[..])
        );
        assert_eq!(name_in_table(&header, table_end, 0), None);
        assert_eq!(name_in_table(&header, table_end, 12), None);
        assert_eq!(name_in_table(&header, table_end, 26), None);
        assert_eq!(name_in_table(&header, table_end, u32::MAX), None);
    }

    #[test]
    fn rejects_a_map_shorter_than_its_header() {
        let mut built = RezBuilder::new().build();
        assert_eq!(built.layout.map_len, 8);
        assert!(parse(&built.bytes).is_ok(), "8-byte map is the minimum");
        built.put_u32_le(row(&built, 0, REZ_ENTRY_SIZE_FIELD), 7);
        assert_eq!(err(&built), ParseError::MapTruncated);
    }

    #[test]
    fn rejects_a_type_list_outside_the_map() {
        // The empty file's empty type list starts exactly at the map end.
        let mut built = RezBuilder::new().build();
        let field = built.layout.map_offset + REZ_MAP_TYPE_LIST_FIELD;
        built.put_u32(field, 9);
        assert_eq!(err(&built), ParseError::TypeListOutOfBounds);
        built.put_u32(field, u32::MAX);
        assert_eq!(err(&built), ParseError::TypeListOutOfBounds);
    }

    #[test]
    fn type_entries_must_fit_in_the_map() {
        let mut built = sample();
        let count = built.layout.map_offset + REZ_MAP_TYPE_COUNT_FIELD;
        // From the type list to the map end: 2 type entries and 4 resource
        // entries, 24 + 4 * 266 = 1,088 bytes, room for 90 type entries but
        // not 91. (The extra "entries" are garbage that fails later checks.)
        assert_eq!(built.layout.map_len - 8, 1088);
        built.put_u32(count, 90);
        assert_ne!(
            parse(&built.bytes).err(),
            Some(ParseError::TypeListOutOfBounds)
        );
        built.put_u32(count, 91);
        assert_eq!(err(&built), ParseError::TypeListOutOfBounds);
        built.put_u32(count, u32::MAX);
        assert_eq!(err(&built), ParseError::TypeListOutOfBounds);
    }

    #[test]
    fn a_resource_list_must_end_within_the_map() {
        // PICT's list ends exactly at the map end.
        let mut built = sample();
        let count = built.layout.type_entry(PICT) + REZ_TYPE_RES_COUNT_FIELD;
        built.put_u32(count, 3);
        assert_eq!(
            err(&built),
            ParseError::ReferenceListOutOfBounds { ty: PICT }
        );
        built.put_u32(count, u32::MAX);
        assert_eq!(
            err(&built),
            ParseError::ReferenceListOutOfBounds { ty: PICT }
        );
    }

    #[test]
    fn rejects_a_resource_list_offset_outside_the_map() {
        let mut built = sample();
        let field = list_field(&built, PICT);
        built.put_u32(field, u32::MAX);
        assert_eq!(
            err(&built),
            ParseError::ReferenceListOutOfBounds { ty: PICT }
        );
    }

    #[test]
    fn rejects_two_types_sharing_one_resource_list() {
        let mut built = sample();
        let ship = be_at(&built.bytes, list_field(&built, SHIP));
        built.put_u32(list_field(&built, PICT), ship);
        assert_eq!(
            err(&built),
            ParseError::OverlappingReferenceLists {
                ty: PICT,
                other: SHIP
            }
        );
    }

    #[test]
    fn rejects_resource_lists_overlapping_by_one_byte() {
        let mut built = sample();
        let field = list_field(&built, PICT);
        let adjacent = be_at(&built.bytes, field);
        built.put_u32(field, adjacent - 1);
        assert_eq!(
            err(&built),
            ParseError::OverlappingReferenceLists {
                ty: PICT,
                other: SHIP
            }
        );
    }

    #[test]
    fn overlap_is_found_before_any_resource_is_read() {
        // SHIP 128 has a bad entry index; alone that is what fails, but once
        // PICT shares SHIP's list the overlap is found first.
        let mut built = sample();
        built.put_u32(index_field(&built, SHIP, 128), 0);
        assert!(matches!(
            err(&built),
            ParseError::RezEntryIndexOutOfRange { .. }
        ));
        let ship = be_at(&built.bytes, list_field(&built, SHIP));
        built.put_u32(list_field(&built, PICT), ship);
        assert!(matches!(
            err(&built),
            ParseError::OverlappingReferenceLists { .. }
        ));
    }

    #[test]
    fn rejects_an_entry_index_below_the_base() {
        let mut built = sample();
        built.put_u32(index_field(&built, SHIP, 129), 0);
        assert_eq!(
            err(&built),
            ParseError::RezEntryIndexOutOfRange {
                ty: SHIP,
                id: 129,
                index: 0
            }
        );
    }

    #[test]
    fn rejects_an_entry_index_past_the_table() {
        // Four rows, base 1: index 3 is the last blob, 4 the map, 5 past it.
        let mut built = sample();
        let field = index_field(&built, PICT, 7);
        built.put_u32(field, 3);
        assert!(parse(&built.bytes).is_ok());
        for index in [5, u32::MAX] {
            built.put_u32(field, index);
            assert_eq!(
                err(&built),
                ParseError::RezEntryIndexOutOfRange {
                    ty: PICT,
                    id: 7,
                    index
                }
            );
        }
    }

    #[test]
    fn rejects_a_resource_pointing_at_the_map() {
        let mut built = sample();
        built.put_u32(index_field(&built, PICT, 7), 4);
        assert_eq!(
            err(&built),
            ParseError::RezResourceIsMap { ty: PICT, id: 7 }
        );
    }

    #[test]
    fn entry_indices_count_from_the_base_index() {
        let mut built = sample();
        built.put_u32_le(REZ_BASE_INDEX_FIELD, 0);
        // Every index now names the row after its own; PICT 7 (index 1)
        // reads SHIP 129's data, and the last resource the map.
        assert_eq!(
            err(&built),
            ParseError::RezResourceIsMap { ty: PICT, id: -5 }
        );
        built.put_u32_le(REZ_BASE_INDEX_FIELD, 100);
        for (ty, id) in [(SHIP, 128), (SHIP, 129), (PICT, -5), (PICT, 7)] {
            let field = index_field(&built, ty, id);
            let index = be_at(&built.bytes, field);
            built.put_u32(field, index + 99);
        }
        let after = flatten(&built.bytes, &ok(&built));
        let before = sample();
        assert_eq!(after, flatten(&before.bytes, &ok(&before)));
    }

    #[test]
    fn rejects_a_mismatched_type_code() {
        let mut built = sample();
        let pos = built.layout.resource(PICT, 7) + REZ_RES_TYPE_FIELD;
        built.put_u8(pos + 3, b'S');
        assert_eq!(
            err(&built),
            ParseError::RezTypeMismatch {
                ty: PICT,
                id: 7,
                found: ResType(*b"PICS")
            }
        );
    }

    #[test]
    fn rejects_a_name_field_without_a_nul() {
        let mut built = sample();
        let name = built.layout.resource(SHIP, 128) + REZ_RES_NAME_FIELD;
        for i in 0..256 {
            built.put_u8(name + i, b'n');
        }
        assert_eq!(
            err(&built),
            ParseError::RezUnterminatedName { ty: SHIP, id: 128 }
        );
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
        let built = RezBuilder::new()
            .resource(PICT, 1, None, b"a")
            .resource(SHIP, 1, None, b"b")
            .resource(PICT, 2, None, b"c")
            .resource(PICT, 1, None, b"d")
            .build();
        assert_eq!(err(&built), ParseError::DuplicateId { ty: PICT, id: 1 });
    }

    #[test]
    fn unreferenced_entries_and_rows_sharing_bytes_are_allowed() {
        let mut built = sample();
        // PICT -5 now shares SHIP 128's row, leaving row 2 unreferenced.
        built.put_u32(index_field(&built, PICT, -5), 1);
        // SHIP 129's row now covers the same bytes as row 0.
        let (offset, size) = (
            le_at(&built.bytes, row(&built, 0, REZ_ENTRY_OFFSET_FIELD)),
            le_at(&built.bytes, row(&built, 0, REZ_ENTRY_SIZE_FIELD)),
        );
        let (offset_field, size_field) = (
            row(&built, 1, REZ_ENTRY_OFFSET_FIELD),
            row(&built, 1, REZ_ENTRY_SIZE_FIELD),
        );
        built
            .put_u32_le(offset_field, offset)
            .put_u32_le(size_field, size);
        let flat = flatten(&built.bytes, &ok(&built));
        assert!(flat.iter().flat_map(|(_, e)| e).all(|e| e.2 == b"ship-128"));
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
