//! Synthetic resource-file builders for tests.
//!
//! Available to this crate's tests and, through the `fixture` feature, to
//! other crates' tests. [`ForkBuilder`] writes a well-formed flattened fork;
//! [`Layout`] records where every field landed so corruption tests can poke
//! exact bytes with the [`BuiltFork`] helpers. [`RezBuilder`] takes the same
//! calls and writes a Windows `.rez` file instead, with [`RezLayout`] and
//! [`BuiltRez`] playing the same roles.

use crate::ResType;

/// Length of the fork header.
pub const HEADER_LEN: usize = 16;
/// Header field: offset of the data section.
pub const DATA_OFFSET_FIELD: usize = 0;
/// Header field: offset of the map.
pub const MAP_OFFSET_FIELD: usize = 4;
/// Header field: length of the data section.
pub const DATA_LEN_FIELD: usize = 8;
/// Header field: length of the map.
pub const MAP_LEN_FIELD: usize = 12;

/// Length of the map header, before the type list.
pub const MAP_HEADER_LEN: usize = 28;
/// Map field (from the map start): offset of the type list.
pub const TYPE_LIST_OFFSET_FIELD: usize = 24;
/// Map field (from the map start): offset of the name list.
pub const NAME_LIST_OFFSET_FIELD: usize = 26;

/// Type entry field: resource count minus one.
pub const TYPE_COUNT_FIELD: usize = 4;
/// Type entry field: offset of the reference list from the type list.
pub const REF_LIST_OFFSET_FIELD: usize = 6;

/// Reference entry field: name offset from the name list, or `-1`.
pub const REF_NAME_OFFSET_FIELD: usize = 2;
/// Reference entry field: attribute byte.
pub const REF_ATTRIBUTES_FIELD: usize = 4;
/// Reference entry field: 24-bit data offset from the data section.
pub const REF_DATA_OFFSET_FIELD: usize = 5;

const TYPE_ENTRY_LEN: usize = 8;

/// Builds a well-formed flattened resource fork.
///
/// Types appear in the map in the order they are first added; resources of a
/// type keep the order they were added in. Data and names are written in map
/// order.
#[derive(Clone, Debug, Default)]
pub struct ForkBuilder {
    gap: usize,
    trailing: usize,
    resources: Vec<Spec>,
}

#[derive(Clone, Debug)]
struct Spec {
    ty: ResType,
    id: i16,
    name: Option<Vec<u8>>,
    attributes: u8,
    data: Vec<u8>,
}

impl ForkBuilder {
    /// An empty fork: no types.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a resource. Names are raw (Mac Roman) bytes.
    #[must_use]
    pub fn resource(mut self, ty: ResType, id: i16, name: Option<&[u8]>, data: &[u8]) -> Self {
        self.resources.push(Spec {
            ty,
            id,
            name: name.map(<[u8]>::to_vec),
            attributes: 0,
            data: data.to_vec(),
        });
        self
    }

    /// Sets the attribute byte of the most recently added resource.
    #[must_use]
    pub fn attributes(mut self, attributes: u8) -> Self {
        self.resources
            .last_mut()
            .expect("attributes() follows resource()")
            .attributes = attributes;
        self
    }

    /// Puts `len` zero bytes between the header and the data section.
    #[must_use]
    pub fn gap(mut self, len: usize) -> Self {
        self.gap = len;
        self
    }

    /// Appends `len` zero bytes after the map.
    #[must_use]
    pub fn trailing(mut self, len: usize) -> Self {
        self.trailing = len;
        self
    }

    /// Writes the fork.
    #[must_use]
    pub fn build(&self) -> BuiltFork {
        let groups = groups(&self.resources);
        let data_offset = HEADER_LEN + self.gap;

        let mut layout = Layout {
            data_offset,
            ..Layout::default()
        };

        let mut data = Vec::new();
        let mut names = Vec::new();
        let mut type_list = Vec::new();
        let mut refs = Vec::new();
        let refs_start = 2 + TYPE_ENTRY_LEN * groups.len();
        type_list.extend(minus_one(groups.len()).to_be_bytes());

        let mut map_tail = Vec::new();
        for (ty, specs) in &groups {
            let ref_list_offset = refs_start + refs.len();
            type_list.extend(ty.0);
            type_list.extend(minus_one(specs.len()).to_be_bytes());
            type_list.extend(u16_of(ref_list_offset).to_be_bytes());
            for spec in specs {
                let name_offset = spec.name.as_ref().map_or(0xFFFF, |name| {
                    let offset = names.len();
                    layout.names.push((spec.ty, spec.id, offset));
                    names.push(u8::try_from(name.len()).expect("name fits a length byte"));
                    names.extend(name);
                    u16_of(offset)
                });
                layout.data.push((spec.ty, spec.id, data.len()));
                layout.refs.push((spec.ty, spec.id, refs.len()));
                refs.extend(spec.id.to_be_bytes());
                refs.extend(name_offset.to_be_bytes());
                refs.push(spec.attributes);
                refs.extend(&(u32_of(data.len())).to_be_bytes()[1..]);
                refs.extend([0; 4]);
                data.extend(u32_of(spec.data.len()).to_be_bytes());
                data.extend(&spec.data);
            }
        }
        map_tail.extend(type_list);
        map_tail.extend(refs);

        let type_list_offset = MAP_HEADER_LEN;
        let name_list_offset = MAP_HEADER_LEN + map_tail.len();
        let mut map = vec![0; TYPE_LIST_OFFSET_FIELD];
        map.extend(u16_of(type_list_offset).to_be_bytes());
        map.extend(u16_of(name_list_offset).to_be_bytes());
        map.extend(map_tail);
        map.extend(names);

        let map_offset = data_offset + data.len();
        layout.data_len = data.len();
        layout.map_offset = map_offset;
        layout.map_len = map.len();
        layout.type_list = map_offset + type_list_offset;
        layout.name_list = map_offset + name_list_offset;
        for (i, (ty, _)) in groups.iter().enumerate() {
            layout
                .types
                .push((*ty, layout.type_list + 2 + TYPE_ENTRY_LEN * i));
        }
        let refs_abs = layout.type_list + refs_start;
        for (_, _, pos) in &mut layout.refs {
            *pos += refs_abs;
        }
        for (_, _, pos) in &mut layout.data {
            *pos += data_offset;
        }
        for (_, _, pos) in &mut layout.names {
            *pos += layout.name_list;
        }

        let mut bytes = Vec::new();
        bytes.extend(u32_of(data_offset).to_be_bytes());
        bytes.extend(u32_of(map_offset).to_be_bytes());
        bytes.extend(u32_of(data.len()).to_be_bytes());
        bytes.extend(u32_of(map.len()).to_be_bytes());
        bytes.resize(data_offset, 0);
        bytes.extend(data);
        bytes.extend(map);
        bytes.resize(bytes.len() + self.trailing, 0);

        BuiltFork { bytes, layout }
    }
}

/// Resources grouped by type, in first-appearance order.
fn groups(resources: &[Spec]) -> Vec<(ResType, Vec<&Spec>)> {
    let mut groups: Vec<(ResType, Vec<&Spec>)> = Vec::new();
    for spec in resources {
        match groups.iter_mut().find(|(ty, _)| *ty == spec.ty) {
            Some((_, specs)) => specs.push(spec),
            None => groups.push((spec.ty, vec![spec])),
        }
    }
    groups
}

/// Overwrites `value.len()` bytes of `bytes` at `pos`.
fn put(bytes: &mut [u8], pos: usize, value: &[u8]) {
    bytes[pos..pos + value.len()].copy_from_slice(value);
}

/// A count stored minus one, as the map does (`0` becomes `0xFFFF`).
fn minus_one(count: usize) -> u16 {
    u16_of(count).wrapping_sub(1)
}

fn u16_of(n: usize) -> u16 {
    u16::try_from(n).expect("fixture field fits in u16")
}

fn u32_of(n: usize) -> u32 {
    u32::try_from(n).expect("fixture field fits in u32")
}

/// A built fork and the positions of its fields.
#[derive(Clone, Debug)]
pub struct BuiltFork {
    /// The flattened fork.
    pub bytes: Vec<u8>,
    /// Where each field was written.
    pub layout: Layout,
}

impl BuiltFork {
    /// Overwrites the byte at `pos`.
    pub fn put_u8(&mut self, pos: usize, value: u8) -> &mut Self {
        self.bytes[pos] = value;
        self
    }

    /// Overwrites a big-endian `u16` at `pos`.
    pub fn put_u16(&mut self, pos: usize, value: u16) -> &mut Self {
        self.put(pos, &value.to_be_bytes())
    }

    /// Overwrites a big-endian 24-bit value at `pos` (the top byte is dropped).
    pub fn put_u24(&mut self, pos: usize, value: u32) -> &mut Self {
        self.put(pos, &value.to_be_bytes()[1..])
    }

    /// Overwrites a big-endian `u32` at `pos`.
    pub fn put_u32(&mut self, pos: usize, value: u32) -> &mut Self {
        self.put(pos, &value.to_be_bytes())
    }

    /// Cuts the fork down to `len` bytes.
    pub fn truncate(&mut self, len: usize) -> &mut Self {
        self.bytes.truncate(len);
        self
    }

    fn put(&mut self, pos: usize, value: &[u8]) -> &mut Self {
        put(&mut self.bytes, pos, value);
        self
    }
}

/// Absolute byte positions of a built fork's sections and records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layout {
    /// Start of the data section.
    pub data_offset: usize,
    /// Length of the data section.
    pub data_len: usize,
    /// Start of the map.
    pub map_offset: usize,
    /// Length of the map.
    pub map_len: usize,
    /// Start of the type list (its count field).
    pub type_list: usize,
    /// Start of the name list.
    pub name_list: usize,
    types: Vec<(ResType, usize)>,
    refs: Vec<(ResType, i16, usize)>,
    data: Vec<(ResType, i16, usize)>,
    names: Vec<(ResType, i16, usize)>,
}

impl Layout {
    /// Start of the type list entry for `ty`.
    #[must_use]
    pub fn type_entry(&self, ty: ResType) -> usize {
        self.types
            .iter()
            .find(|(t, _)| *t == ty)
            .map(|&(_, pos)| pos)
            .expect("type in fixture")
    }

    /// Start of the first reference entry for `(ty, id)`.
    #[must_use]
    pub fn reference(&self, ty: ResType, id: i16) -> usize {
        find(&self.refs, ty, id)
    }

    /// Start of the data entry (its length prefix) for `(ty, id)`.
    #[must_use]
    pub fn data_entry(&self, ty: ResType, id: i16) -> usize {
        find(&self.data, ty, id)
    }

    /// Start of the name (its length byte) for `(ty, id)`.
    #[must_use]
    pub fn name(&self, ty: ResType, id: i16) -> usize {
        find(&self.names, ty, id)
    }
}

fn find(entries: &[(ResType, i16, usize)], ty: ResType, id: i16) -> usize {
    entries
        .iter()
        .find(|&&(t, i, _)| t == ty && i == id)
        .map(|&(_, _, pos)| pos)
        .expect("resource in fixture")
}

/// The magic number that opens a `.rez` file.
pub const REZ_MAGIC: &[u8; 4] = b"BRGR";
/// Length of the `.rez` root header (magic, group count, header length).
pub const REZ_ROOT_HEADER_LEN: usize = 12;
/// Length of the `.rez` header, root and group parts, before the entry table.
pub const REZ_HEADER_LEN: usize = 24;
/// `.rez` header field: group count (little-endian, must be 1).
pub const REZ_GROUP_COUNT_FIELD: usize = 4;
/// `.rez` header field: header length from the end of the root header
/// (little-endian).
pub const REZ_HEADER_LEN_FIELD: usize = 8;
/// `.rez` header field: group type (little-endian, must be 1).
pub const REZ_GROUP_TYPE_FIELD: usize = 12;
/// `.rez` header field: the index of the first entry (little-endian).
pub const REZ_BASE_INDEX_FIELD: usize = 16;
/// `.rez` header field: number of entries, the map included (little-endian).
pub const REZ_ENTRY_COUNT_FIELD: usize = 20;

/// Length of one `.rez` entry-table row.
pub const REZ_ENTRY_LEN: usize = 12;
/// Entry row field: absolute offset of the entry's data (little-endian).
pub const REZ_ENTRY_OFFSET_FIELD: usize = 0;
/// Entry row field: size of the entry's data (little-endian).
pub const REZ_ENTRY_SIZE_FIELD: usize = 4;
/// Entry row field: name offset from the end of the root header
/// (little-endian).
pub const REZ_ENTRY_NAME_FIELD: usize = 8;

/// Length of the `.rez` map header, before the type list.
pub const REZ_MAP_HEADER_LEN: usize = 8;
/// Map field (from the map start): offset of the type list (big-endian).
pub const REZ_MAP_TYPE_LIST_FIELD: usize = 0;
/// Map field (from the map start): number of types (big-endian).
pub const REZ_MAP_TYPE_COUNT_FIELD: usize = 4;

/// Length of one `.rez` type entry.
pub const REZ_TYPE_ENTRY_LEN: usize = 12;
/// Type entry field: offset of the resource list from the map (big-endian).
pub const REZ_TYPE_LIST_FIELD: usize = 4;
/// Type entry field: number of resources (big-endian).
pub const REZ_TYPE_RES_COUNT_FIELD: usize = 8;

/// Length of one `.rez` resource entry.
pub const REZ_RES_ENTRY_LEN: usize = 266;
/// Resource entry field: entry-table index, counted from the base index
/// (big-endian).
pub const REZ_RES_INDEX_FIELD: usize = 0;
/// Resource entry field: the type code, repeated.
pub const REZ_RES_TYPE_FIELD: usize = 4;
/// Resource entry field: the resource ID (big-endian).
pub const REZ_RES_ID_FIELD: usize = 8;
/// Resource entry field: the NUL-terminated name.
pub const REZ_RES_NAME_FIELD: usize = 10;
/// Length of a resource entry's name field.
pub const REZ_NAME_FIELD_LEN: usize = 256;

/// The map entry's name, as stored in the name table.
const REZ_MAP_NAME: &[u8] = b"resource.map\0";

/// Builds a well-formed Windows `.rez` file.
///
/// Takes the same calls as [`ForkBuilder`] and groups them the same way:
/// types in the order they are first added, resources of a type in the
/// order they were added. Identical data is written once and shared, as
/// ResForge's writer does. Blobs follow the header in map order; the map is
/// the last entry and ends the file.
#[derive(Clone, Debug, Default)]
pub struct RezBuilder {
    resources: Vec<Spec>,
}

impl RezBuilder {
    /// An empty file: no types, only the map entry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a resource. Names are raw (Mac Roman) bytes, at most 255 of them.
    #[must_use]
    pub fn resource(mut self, ty: ResType, id: i16, name: Option<&[u8]>, data: &[u8]) -> Self {
        self.resources.push(Spec {
            ty,
            id,
            name: name.map(<[u8]>::to_vec),
            attributes: 0,
            data: data.to_vec(),
        });
        self
    }

    /// Writes the file.
    #[must_use]
    pub fn build(&self) -> BuiltRez {
        let groups = groups(&self.resources);

        // Distinct blobs in map order; each resource names its blob's row.
        let mut blobs: Vec<&[u8]> = Vec::new();
        let mut slots = Vec::new();
        for spec in groups.iter().flat_map(|(_, specs)| specs) {
            let slot = blobs
                .iter()
                .position(|b| *b == spec.data.as_slice())
                .unwrap_or_else(|| {
                    blobs.push(&spec.data);
                    blobs.len() - 1
                });
            slots.push((spec.ty, spec.id, slot));
        }

        let entry_count = blobs.len() + 1;
        let name_table = REZ_HEADER_LEN + REZ_ENTRY_LEN * entry_count;
        let data_offset = name_table + REZ_MAP_NAME.len();
        let mut rows = Vec::new();
        let mut offset = data_offset;
        for blob in &blobs {
            rows.push((offset, blob.len(), 0));
            offset += blob.len();
        }
        let map_offset = offset;

        let mut layout = RezLayout {
            entry_table: REZ_HEADER_LEN,
            entry_count,
            name_table,
            map_name: name_table,
            data_offset,
            map_entry: blobs.len(),
            map_offset,
            type_list: map_offset + REZ_MAP_HEADER_LEN,
            ..RezLayout::default()
        };

        let mut map = Vec::new();
        map.extend(u32_of(REZ_MAP_HEADER_LEN).to_be_bytes());
        map.extend(u32_of(groups.len()).to_be_bytes());
        let mut list = REZ_MAP_HEADER_LEN + REZ_TYPE_ENTRY_LEN * groups.len();
        for (ty, specs) in &groups {
            layout.types.push((*ty, map_offset + map.len()));
            map.extend(ty.0);
            map.extend(u32_of(list).to_be_bytes());
            map.extend(u32_of(specs.len()).to_be_bytes());
            list += REZ_RES_ENTRY_LEN * specs.len();
        }
        for (spec, &(_, _, slot)) in groups.iter().flat_map(|(_, specs)| specs).zip(&slots) {
            layout
                .resources
                .push((spec.ty, spec.id, map_offset + map.len()));
            let name = spec.name.as_deref().unwrap_or_default();
            assert!(name.len() < REZ_NAME_FIELD_LEN, "name fits the name field");
            map.extend(u32_of(slot + 1).to_be_bytes());
            map.extend(spec.ty.0);
            map.extend(spec.id.to_be_bytes());
            map.extend(name);
            map.resize(map.len() + REZ_NAME_FIELD_LEN - name.len(), 0);
        }
        layout.map_len = map.len();
        layout.slots = slots;
        rows.push((map_offset, map.len(), name_table - REZ_ROOT_HEADER_LEN));

        let mut bytes = Vec::new();
        bytes.extend(REZ_MAGIC);
        bytes.extend(1u32.to_le_bytes());
        bytes.extend(u32_of(data_offset - REZ_ROOT_HEADER_LEN).to_le_bytes());
        bytes.extend(1u32.to_le_bytes());
        bytes.extend(1u32.to_le_bytes());
        bytes.extend(u32_of(entry_count).to_le_bytes());
        for (offset, size, name) in rows {
            bytes.extend(u32_of(offset).to_le_bytes());
            bytes.extend(u32_of(size).to_le_bytes());
            bytes.extend(u32_of(name).to_le_bytes());
        }
        bytes.extend(REZ_MAP_NAME);
        for blob in blobs {
            bytes.extend(blob);
        }
        bytes.extend(map);

        BuiltRez { bytes, layout }
    }
}

/// A built `.rez` file and the positions of its fields.
#[derive(Clone, Debug)]
pub struct BuiltRez {
    /// The file's bytes.
    pub bytes: Vec<u8>,
    /// Where each field was written.
    pub layout: RezLayout,
}

impl BuiltRez {
    /// Overwrites the byte at `pos`.
    pub fn put_u8(&mut self, pos: usize, value: u8) -> &mut Self {
        self.put(pos, &[value])
    }

    /// Overwrites a big-endian `u16` at `pos` (map fields).
    pub fn put_u16(&mut self, pos: usize, value: u16) -> &mut Self {
        self.put(pos, &value.to_be_bytes())
    }

    /// Overwrites a big-endian `u32` at `pos` (map fields).
    pub fn put_u32(&mut self, pos: usize, value: u32) -> &mut Self {
        self.put(pos, &value.to_be_bytes())
    }

    /// Overwrites a little-endian `u32` at `pos` (header and entry-table
    /// fields).
    pub fn put_u32_le(&mut self, pos: usize, value: u32) -> &mut Self {
        self.put(pos, &value.to_le_bytes())
    }

    /// Cuts the file down to `len` bytes.
    pub fn truncate(&mut self, len: usize) -> &mut Self {
        self.bytes.truncate(len);
        self
    }

    fn put(&mut self, pos: usize, value: &[u8]) -> &mut Self {
        put(&mut self.bytes, pos, value);
        self
    }
}

/// Absolute byte positions of a built `.rez` file's tables and records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RezLayout {
    /// Start of the entry table.
    pub entry_table: usize,
    /// Number of entry-table rows, the map's included.
    pub entry_count: usize,
    /// The map's row in the entry table (always the last).
    pub map_entry: usize,
    /// Start of the name table.
    pub name_table: usize,
    /// Start of the map entry's name, `resource.map`.
    pub map_name: usize,
    /// Start of the first data blob, just after the header.
    pub data_offset: usize,
    /// Start of the map.
    pub map_offset: usize,
    /// Length of the map.
    pub map_len: usize,
    /// Start of the type list (its first entry).
    pub type_list: usize,
    types: Vec<(ResType, usize)>,
    resources: Vec<(ResType, i16, usize)>,
    slots: Vec<(ResType, i16, usize)>,
}

impl RezLayout {
    /// Start of the entry-table row at `index` (from 0).
    #[must_use]
    pub fn entry(&self, index: usize) -> usize {
        self.entry_table + REZ_ENTRY_LEN * index
    }

    /// The entry-table row (from 0) holding the data of `(ty, id)`.
    #[must_use]
    pub fn entry_index(&self, ty: ResType, id: i16) -> usize {
        find(&self.slots, ty, id)
    }

    /// Start of the type entry for `ty`.
    #[must_use]
    pub fn type_entry(&self, ty: ResType) -> usize {
        self.types
            .iter()
            .find(|(t, _)| *t == ty)
            .map(|&(_, pos)| pos)
            .expect("type in fixture")
    }

    /// Start of the 266-byte resource entry for `(ty, id)`.
    #[must_use]
    pub fn resource(&self, ty: ResType, id: i16) -> usize {
        find(&self.resources, ty, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PICT: ResType = ResType(*b"PICT");
    const SHIP: ResType = ResType([b's', b'h', 0x95, b'p']);

    fn u8_at(bytes: &[u8], pos: usize) -> u8 {
        bytes[pos]
    }

    fn u16_at(bytes: &[u8], pos: usize) -> u16 {
        u16::from_be_bytes([bytes[pos], bytes[pos + 1]])
    }

    fn u24_at(bytes: &[u8], pos: usize) -> u32 {
        u32::from_be_bytes([0, bytes[pos], bytes[pos + 1], bytes[pos + 2]])
    }

    fn u32_at(bytes: &[u8], pos: usize) -> u32 {
        u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
    }

    /// `(id, name bytes, data)` as macbinary reads a resource.
    type MacbinaryResource = (i16, Option<Vec<u8>>, Vec<u8>);

    fn sample() -> ForkBuilder {
        ForkBuilder::new()
            .resource(SHIP, 128, Some(b"Shuttle"), b"ship-128")
            .resource(PICT, -5, None, b"")
            .attributes(0x20)
            .resource(SHIP, 129, Some(b"\x8Aa"), b"ship-129!")
    }

    #[test]
    fn header_fields_match_section_positions() {
        let built = sample().gap(240).trailing(7).build();
        let (bytes, layout) = (&built.bytes, &built.layout);

        assert_eq!(layout.data_offset, 256);
        assert_eq!(u32_at(bytes, DATA_OFFSET_FIELD), 256);
        assert_eq!(u32_at(bytes, MAP_OFFSET_FIELD) as usize, layout.map_offset);
        assert_eq!(u32_at(bytes, DATA_LEN_FIELD) as usize, layout.data_len);
        assert_eq!(u32_at(bytes, MAP_LEN_FIELD) as usize, layout.map_len);
        // Three length-prefixed blobs of 8, 9 and 0 bytes.
        assert_eq!(layout.data_len, 3 * 4 + 8 + 9);
        assert_eq!(layout.map_offset, layout.data_offset + layout.data_len);
        assert_eq!(bytes.len(), layout.map_offset + layout.map_len + 7);
        assert!(bytes[HEADER_LEN..256].iter().all(|&b| b == 0));
    }

    #[test]
    fn map_header_points_at_type_and_name_lists() {
        let built = sample().build();
        let (bytes, layout) = (&built.bytes, &built.layout);

        assert_eq!(layout.type_list, layout.map_offset + MAP_HEADER_LEN);
        assert_eq!(
            u16_at(bytes, layout.map_offset + TYPE_LIST_OFFSET_FIELD) as usize,
            layout.type_list - layout.map_offset
        );
        assert_eq!(
            u16_at(bytes, layout.map_offset + NAME_LIST_OFFSET_FIELD) as usize,
            layout.name_list - layout.map_offset
        );
        // Two types and three references sit between the lists.
        assert_eq!(layout.name_list, layout.type_list + 2 + 2 * 8 + 3 * 12);
        // Names "Shuttle" and "\x8Aa", each with a length byte.
        assert_eq!(layout.map_offset + layout.map_len, layout.name_list + 8 + 3);
    }

    #[test]
    fn empty_builder_writes_zero_type_count() {
        let built = ForkBuilder::new().build();
        assert_eq!(u16_at(&built.bytes, built.layout.type_list), 0xFFFF);
        assert_eq!(built.layout.data_len, 0);
        assert_eq!(built.layout.map_len, MAP_HEADER_LEN + 2);

        let fork = macbinary::ResourceFork::new(&built.bytes).expect("macbinary parses");
        assert_eq!(fork.resource_types().count(), 0);
    }

    #[test]
    fn layout_positions_point_at_fields() {
        let built = sample().build();
        let (bytes, layout) = (&built.bytes, &built.layout);

        assert_eq!(layout.type_entry(SHIP), layout.type_list + 2);
        assert_eq!(layout.type_entry(PICT), layout.type_list + 2 + 8);
        assert_eq!(&bytes[layout.type_entry(SHIP)..][..4], &SHIP.0);
        assert_eq!(u16_at(bytes, layout.type_entry(SHIP) + TYPE_COUNT_FIELD), 1);
        assert_eq!(u16_at(bytes, layout.type_entry(PICT) + TYPE_COUNT_FIELD), 0);
        let ship_refs = layout.type_list + 2 + 2 * 8;
        assert_eq!(
            u16_at(bytes, layout.type_entry(SHIP) + REF_LIST_OFFSET_FIELD) as usize,
            ship_refs - layout.type_list
        );
        assert_eq!(
            u16_at(bytes, layout.type_entry(PICT) + REF_LIST_OFFSET_FIELD) as usize,
            ship_refs + 2 * 12 - layout.type_list
        );

        assert_eq!(layout.reference(SHIP, 128), ship_refs);
        assert_eq!(layout.reference(SHIP, 129), ship_refs + 12);
        assert_eq!(layout.reference(PICT, -5), ship_refs + 24);
        for (ty, id) in [(SHIP, 128), (SHIP, 129), (PICT, -5)] {
            assert_eq!(u16_at(bytes, layout.reference(ty, id)) as i16, id);
        }
        let pict_ref = layout.reference(PICT, -5);
        assert_eq!(u16_at(bytes, pict_ref + REF_NAME_OFFSET_FIELD), 0xFFFF);
        assert_eq!(u8_at(bytes, pict_ref + REF_ATTRIBUTES_FIELD), 0x20);
        assert_eq!(u32_at(bytes, pict_ref + 8), 0, "reserved handle");

        // Data in map order: SHIP 128, SHIP 129, PICT -5.
        assert_eq!(layout.data_entry(SHIP, 128), layout.data_offset);
        assert_eq!(layout.data_entry(SHIP, 129), layout.data_offset + 4 + 8);
        assert_eq!(
            layout.data_entry(PICT, -5),
            layout.data_offset + 2 * 4 + 8 + 9
        );
        for (ty, id, len) in [(SHIP, 128, 8), (SHIP, 129, 9), (PICT, -5, 0)] {
            let entry = layout.data_entry(ty, id);
            assert_eq!(u32_at(bytes, entry), len);
            let rel = u24_at(bytes, layout.reference(ty, id) + REF_DATA_OFFSET_FIELD) as usize;
            assert_eq!(layout.data_offset + rel, entry);
        }

        assert_eq!(layout.name(SHIP, 128), layout.name_list);
        assert_eq!(layout.name(SHIP, 129), layout.name_list + 8);
        assert_eq!(u8_at(bytes, layout.name(SHIP, 129)), 2);
        let rel = u16_at(bytes, layout.reference(SHIP, 129) + REF_NAME_OFFSET_FIELD) as usize;
        assert_eq!(layout.name_list + rel, layout.name(SHIP, 129));
    }

    #[test]
    fn output_parses_with_macbinary() {
        let built = sample().build();
        let fork = macbinary::ResourceFork::new(&built.bytes).expect("macbinary parses");

        let read: Vec<([u8; 4], Vec<MacbinaryResource>)> = fork
            .resource_types()
            .map(|item| {
                let resources = fork
                    .resources(item)
                    .map(|r| {
                        (
                            r.id(),
                            r.name_bytes().map(<[u8]>::to_vec),
                            r.data().to_vec(),
                        )
                    })
                    .collect();
                (item.resource_type().0.to_be_bytes(), resources)
            })
            .collect();

        assert_eq!(
            read,
            vec![
                (
                    SHIP.0,
                    vec![
                        (128, Some(b"Shuttle".to_vec()), b"ship-128".to_vec()),
                        (129, Some(b"\x8Aa".to_vec()), b"ship-129!".to_vec()),
                    ]
                ),
                (PICT.0, vec![(-5, None, Vec::new())]),
            ]
        );
    }

    #[test]
    fn put_helpers_write_big_endian_and_truncate_shortens() {
        let mut built = ForkBuilder::new().build();
        built
            .put_u8(0, 0xAB)
            .put_u16(1, 0x0102)
            .put_u24(3, 0x03_0405)
            .put_u32(6, 0x0607_0809);
        assert_eq!(
            &built.bytes[..10],
            &[0xAB, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09]
        );

        built.truncate(5);
        assert_eq!(built.bytes.len(), 5);
    }

    #[test]
    fn attributes_apply_to_latest_resource_only() {
        let built = ForkBuilder::new()
            .resource(PICT, 1, None, b"a")
            .resource(PICT, 2, None, b"b")
            .attributes(0x01)
            .build();
        let attrs = |id| built.bytes[built.layout.reference(PICT, id) + REF_ATTRIBUTES_FIELD];
        assert_eq!(attrs(1), 0);
        assert_eq!(attrs(2), 0x01);
    }

    fn u32_le_at(bytes: &[u8], pos: usize) -> u32 {
        u32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
    }

    fn rez_sample() -> RezBuilder {
        RezBuilder::new()
            .resource(SHIP, 128, Some(b"Shuttle"), b"ship-128")
            .resource(PICT, -5, None, b"")
            .resource(SHIP, 129, Some(b"\x8Aa"), b"ship-129!")
            // Same bytes as SHIP 128, so it shares that blob.
            .resource(PICT, 7, None, b"ship-128")
    }

    /// The entry-table row `(offset, size, name offset)` at `index`.
    fn rez_row(built: &BuiltRez, index: usize) -> (usize, usize, usize) {
        let pos = built.layout.entry(index);
        let field = |f| u32_le_at(&built.bytes, pos + f) as usize;
        (
            field(REZ_ENTRY_OFFSET_FIELD),
            field(REZ_ENTRY_SIZE_FIELD),
            field(REZ_ENTRY_NAME_FIELD),
        )
    }

    #[test]
    fn rez_header_fields() {
        let built = rez_sample().build();
        let (bytes, layout) = (&built.bytes, &built.layout);

        assert_eq!(&bytes[..4], REZ_MAGIC);
        assert_eq!(u32_le_at(bytes, REZ_GROUP_COUNT_FIELD), 1);
        assert_eq!(u32_le_at(bytes, REZ_GROUP_TYPE_FIELD), 1);
        assert_eq!(u32_le_at(bytes, REZ_BASE_INDEX_FIELD), 1);
        // Three distinct blobs plus the map.
        assert_eq!(layout.entry_count, 4);
        assert_eq!(u32_le_at(bytes, REZ_ENTRY_COUNT_FIELD), 4);
        assert_eq!(layout.entry_table, REZ_HEADER_LEN);
        assert_eq!(layout.name_table, REZ_HEADER_LEN + 4 * REZ_ENTRY_LEN);
        // The header length counts from the end of the 12-byte root header
        // up to the end of the name table, which holds only the map's name.
        let header_end = layout.name_table + b"resource.map\0".len();
        assert_eq!(
            u32_le_at(bytes, REZ_HEADER_LEN_FIELD) as usize,
            header_end - REZ_ROOT_HEADER_LEN
        );
        assert_eq!(layout.data_offset, header_end);
    }

    #[test]
    fn rez_map_row_is_last_and_named_resource_map() {
        let built = rez_sample().build();
        let (bytes, layout) = (&built.bytes, &built.layout);

        assert_eq!(layout.map_entry, layout.entry_count - 1);
        let (offset, size, name) = rez_row(&built, layout.map_entry);
        assert_eq!((offset, size), (layout.map_offset, layout.map_len));
        assert_eq!(offset + size, bytes.len(), "the map ends the file");
        assert_eq!(name + REZ_ROOT_HEADER_LEN, layout.map_name);
        assert_eq!(layout.map_name, layout.name_table);
        assert_eq!(&bytes[layout.map_name..][..13], b"resource.map\0");
    }

    #[test]
    fn rez_blobs_follow_the_header_in_map_order_without_length_prefixes() {
        let built = rez_sample().build();
        let (bytes, layout) = (&built.bytes, &built.layout);

        // Map order: SHIP 128, SHIP 129, PICT -5, PICT 7.
        let blob = |ty, id| {
            let (offset, size, _) = rez_row(&built, layout.entry_index(ty, id));
            &bytes[offset..offset + size]
        };
        assert_eq!(blob(SHIP, 128), b"ship-128");
        assert_eq!(blob(SHIP, 129), b"ship-129!");
        assert_eq!(blob(PICT, -5), b"");
        assert_eq!(rez_row(&built, 0).0, layout.data_offset);
        assert_eq!(rez_row(&built, 1).0, layout.data_offset + 8);
        assert_eq!(rez_row(&built, 2).0, layout.data_offset + 8 + 9);
        assert_eq!(layout.map_offset, layout.data_offset + 8 + 9);
        for index in 0..layout.map_entry {
            assert_eq!(rez_row(&built, index).2, 0, "blob rows are unnamed");
        }
    }

    #[test]
    fn rez_identical_data_shares_one_row() {
        let built = rez_sample().build();
        let layout = &built.layout;
        assert_eq!(layout.entry_index(SHIP, 128), 0);
        assert_eq!(layout.entry_index(SHIP, 129), 1);
        assert_eq!(layout.entry_index(PICT, -5), 2);
        assert_eq!(layout.entry_index(PICT, 7), 0);
    }

    #[test]
    fn rez_map_header_and_type_list() {
        let built = rez_sample().build();
        let (bytes, layout) = (&built.bytes, &built.layout);
        let map = layout.map_offset;

        assert_eq!(layout.type_list, map + REZ_MAP_HEADER_LEN);
        assert_eq!(
            u32_at(bytes, map + REZ_MAP_TYPE_LIST_FIELD) as usize,
            layout.type_list - map
        );
        assert_eq!(u32_at(bytes, map + REZ_MAP_TYPE_COUNT_FIELD), 2);
        assert_eq!(layout.type_entry(SHIP), layout.type_list);
        assert_eq!(
            layout.type_entry(PICT),
            layout.type_list + REZ_TYPE_ENTRY_LEN
        );
        assert_eq!(&bytes[layout.type_entry(SHIP)..][..4], &SHIP.0);
        assert_eq!(&bytes[layout.type_entry(PICT)..][..4], &PICT.0);

        let lists = layout.type_list + 2 * REZ_TYPE_ENTRY_LEN;
        for (ty, start, count) in [(SHIP, lists, 2), (PICT, lists + 2 * REZ_RES_ENTRY_LEN, 2)] {
            let entry = layout.type_entry(ty);
            assert_eq!(
                u32_at(bytes, entry + REZ_TYPE_LIST_FIELD) as usize,
                start - map
            );
            assert_eq!(u32_at(bytes, entry + REZ_TYPE_RES_COUNT_FIELD), count);
        }
        assert_eq!(layout.map_len, lists + 4 * REZ_RES_ENTRY_LEN - map);
    }

    #[test]
    fn rez_resource_entries() {
        let built = rez_sample().build();
        let (bytes, layout) = (&built.bytes, &built.layout);
        let lists = layout.type_list + 2 * REZ_TYPE_ENTRY_LEN;

        assert_eq!(layout.resource(SHIP, 128), lists);
        assert_eq!(layout.resource(SHIP, 129), lists + REZ_RES_ENTRY_LEN);
        assert_eq!(layout.resource(PICT, -5), lists + 2 * REZ_RES_ENTRY_LEN);
        assert_eq!(layout.resource(PICT, 7), lists + 3 * REZ_RES_ENTRY_LEN);
        for (ty, id, name) in [
            (SHIP, 128, &b"Shuttle"[..]),
            (SHIP, 129, b"\x8Aa"),
            (PICT, -5, b""),
            (PICT, 7, b""),
        ] {
            let pos = layout.resource(ty, id);
            // Entry indices count from the base index, 1.
            assert_eq!(
                u32_at(bytes, pos + REZ_RES_INDEX_FIELD) as usize,
                layout.entry_index(ty, id) + 1
            );
            assert_eq!(&bytes[pos + REZ_RES_TYPE_FIELD..][..4], &ty.0);
            assert_eq!(u16_at(bytes, pos + REZ_RES_ID_FIELD) as i16, id);
            let field = &bytes[pos + REZ_RES_NAME_FIELD..pos + REZ_RES_ENTRY_LEN];
            assert_eq!(field.len(), REZ_NAME_FIELD_LEN);
            assert_eq!(&field[..name.len()], name);
            assert!(field[name.len()..].iter().all(|&b| b == 0));
        }
    }

    #[test]
    fn empty_rez_holds_only_the_map() {
        let built = RezBuilder::new().build();
        let layout = &built.layout;
        assert_eq!(layout.entry_count, 1);
        assert_eq!(layout.map_entry, 0);
        assert_eq!(layout.map_offset, layout.data_offset);
        assert_eq!(layout.map_len, REZ_MAP_HEADER_LEN);
        assert_eq!(
            u32_at(&built.bytes, layout.map_offset + REZ_MAP_TYPE_COUNT_FIELD),
            0
        );
        assert_eq!(RezBuilder::default().build().bytes, built.bytes);
    }

    #[test]
    fn rez_put_helpers_write_both_byte_orders_and_truncate_shortens() {
        let mut built = RezBuilder::new().build();
        built
            .put_u8(0, 0xAB)
            .put_u16(1, 0x0102)
            .put_u32(3, 0x0304_0506)
            .put_u32_le(7, 0x0A09_0807);
        assert_eq!(
            &built.bytes[..11],
            &[
                0xAB, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A
            ]
        );
        built.truncate(5);
        assert_eq!(built.bytes.len(), 5);
    }

    #[test]
    fn default_builder_matches_new() {
        assert_eq!(
            ForkBuilder::default().build().bytes,
            ForkBuilder::new().build().bytes
        );
    }
}
