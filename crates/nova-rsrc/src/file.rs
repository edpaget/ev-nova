//! An owned, validated resource file and borrowed views of its resources.

use crate::parse::{self, Entry, TypeEntry};
use crate::{ParseError, ResType};

/// A resource fork held in memory: the raw bytes plus a validated index.
///
/// Resources borrow their data straight from the owned bytes, so reading
/// them is zero-copy.
#[derive(Debug)]
pub struct ResourceFile {
    bytes: Vec<u8>,
    types: Vec<TypeEntry>,
}

impl ResourceFile {
    /// Parses and validates a flattened resource fork.
    ///
    /// Fails on any structural problem; never returns a partial file.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, ParseError> {
        let index = parse::parse(&bytes)?;
        Ok(Self {
            bytes,
            types: index.types,
        })
    }

    /// Every resource type, in map order.
    pub fn types(&self) -> impl Iterator<Item = ResType> + '_ {
        self.types.iter().map(|t| t.ty)
    }

    /// The number of resources of type `ty` (zero if absent).
    #[must_use]
    pub fn count(&self, ty: ResType) -> usize {
        self.type_entry(ty).map_or(0, |t| t.entries.len())
    }

    /// The resources of type `ty`, in reference-list order.
    pub fn resources(&self, ty: ResType) -> impl Iterator<Item = Resource<'_>> {
        self.type_entry(ty)
            .into_iter()
            .flat_map(move |t| t.entries.iter().map(move |e| self.view(t.ty, e)))
    }

    /// Every resource, type by type in map order.
    pub fn iter(&self) -> impl Iterator<Item = Resource<'_>> {
        self.types
            .iter()
            .flat_map(move |t| t.entries.iter().map(move |e| self.view(t.ty, e)))
    }

    /// The resource of type `ty` with ID `id`.
    #[must_use]
    pub fn get(&self, ty: ResType, id: i16) -> Option<Resource<'_>> {
        let t = self.type_entry(ty)?;
        let entry = t.entries.iter().find(|e| e.id == id)?;
        Some(self.view(t.ty, entry))
    }

    /// The total number of resources.
    #[must_use]
    pub fn len(&self) -> usize {
        self.types.iter().map(|t| t.entries.len()).sum()
    }

    /// Whether the fork holds no resources.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn type_entry(&self, ty: ResType) -> Option<&TypeEntry> {
        self.types.iter().find(|t| t.ty == ty)
    }

    fn view<'a>(&'a self, ty: ResType, entry: &'a Entry) -> Resource<'a> {
        Resource {
            ty,
            id: entry.id,
            name: entry.name.as_deref(),
            name_bytes: entry.name_bytes.clone().map(|r| &self.bytes[r]),
            attributes: entry.attributes,
            data: &self.bytes[entry.data.clone()],
        }
    }
}

/// One resource, borrowed from a [`ResourceFile`].
#[derive(Copy, Clone, Debug)]
pub struct Resource<'a> {
    ty: ResType,
    id: i16,
    name: Option<&'a str>,
    name_bytes: Option<&'a [u8]>,
    attributes: u8,
    data: &'a [u8],
}

impl<'a> Resource<'a> {
    /// The resource's type code.
    #[must_use]
    pub fn res_type(&self) -> ResType {
        self.ty
    }

    /// The resource's ID.
    #[must_use]
    pub fn id(&self) -> i16 {
        self.id
    }

    /// The name, decoded from Mac Roman; `None` if unnamed.
    #[must_use]
    pub fn name(&self) -> Option<&'a str> {
        self.name
    }

    /// The raw (Mac Roman) name bytes; `None` if unnamed.
    #[must_use]
    pub fn name_bytes(&self) -> Option<&'a [u8]> {
        self.name_bytes
    }

    /// The attribute byte from the reference list.
    #[must_use]
    pub fn attributes(&self) -> u8 {
        self.attributes
    }

    /// The resource data, borrowed from the file's bytes.
    #[must_use]
    pub fn data(&self) -> &'a [u8] {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::ForkBuilder;

    const PICT: ResType = ResType(*b"PICT");
    const SHIP: ResType = ResType([b's', b'h', 0x95, b'p']);
    const DESC: ResType = ResType(*b"d\xEBsc");

    fn parse(builder: &ForkBuilder) -> ResourceFile {
        ResourceFile::from_bytes(builder.build().bytes).expect("valid fork")
    }

    fn sample() -> ResourceFile {
        parse(
            &ForkBuilder::new()
                .resource(SHIP, 128, Some(b"Shuttle"), b"ship-128")
                .resource(PICT, -5, None, b"")
                .resource(SHIP, 130, None, b"ship-130")
                .resource(SHIP, -1, Some(b"neg"), b"ship-neg")
                .resource(DESC, 0, None, b"desc")
                .attributes(0x20),
        )
    }

    #[test]
    fn empty_fork_has_no_types() {
        let file = parse(&ForkBuilder::new());
        assert_eq!(file.types().count(), 0);
        assert_eq!(file.len(), 0);
        assert!(file.is_empty());
        assert_eq!(file.iter().count(), 0);
    }

    #[test]
    fn single_resource_exposes_type_id_and_data() {
        let file = parse(&ForkBuilder::new().resource(PICT, 128, None, b"pixels"));
        assert_eq!(file.len(), 1);
        assert!(!file.is_empty());
        let res = file.get(PICT, 128).expect("present");
        assert_eq!(res.res_type(), PICT);
        assert_eq!(res.id(), 128);
        assert_eq!(res.data(), b"pixels");
        assert_eq!(res.name(), None);
        assert_eq!(res.name_bytes(), None);
        assert_eq!(res.attributes(), 0);
    }

    #[test]
    fn name_is_decoded_from_mac_roman() {
        let file = parse(&ForkBuilder::new().resource(PICT, 1, Some(b"K\x8Ase"), b""));
        let res = file.get(PICT, 1).expect("present");
        assert_eq!(res.name(), Some("Käse"));
        assert_eq!(res.name_bytes(), Some(&b"K\x8Ase"[..]));
    }

    #[test]
    fn empty_name_is_some_empty_string() {
        let file = parse(&ForkBuilder::new().resource(PICT, 1, Some(b""), b""));
        let res = file.get(PICT, 1).expect("present");
        assert_eq!(res.name(), Some(""));
        assert_eq!(res.name_bytes(), Some(&b""[..]));
    }

    #[test]
    fn types_are_in_map_order() {
        assert_eq!(sample().types().collect::<Vec<_>>(), vec![SHIP, PICT, DESC]);
    }

    #[test]
    fn counts_per_type_and_in_total() {
        let file = sample();
        assert_eq!(file.count(SHIP), 3);
        assert_eq!(file.count(PICT), 1);
        assert_eq!(file.count(DESC), 1);
        assert_eq!(file.count(ResType(*b"none")), 0);
        assert_eq!(file.len(), 5);
    }

    #[test]
    fn resources_of_a_type_are_in_reference_list_order() {
        let file = sample();
        let ships: Vec<_> = file
            .resources(SHIP)
            .map(|r| (r.res_type(), r.id(), r.name(), r.data()))
            .collect();
        assert_eq!(
            ships,
            vec![
                (SHIP, 128, Some("Shuttle"), &b"ship-128"[..]),
                (SHIP, 130, None, &b"ship-130"[..]),
                (SHIP, -1, Some("neg"), &b"ship-neg"[..]),
            ]
        );
        assert_eq!(file.resources(ResType(*b"none")).count(), 0);
    }

    #[test]
    fn get_finds_by_type_and_id() {
        let file = sample();
        assert_eq!(file.get(SHIP, -1).expect("present").data(), b"ship-neg");
        assert_eq!(file.get(PICT, -5).expect("present").data(), b"");
        let desc = file.get(DESC, 0).expect("present");
        assert_eq!(desc.attributes(), 0x20);
        assert!(file.get(SHIP, 129).is_none(), "wrong id");
        assert!(file.get(ResType(*b"none"), 128).is_none(), "absent type");
        assert!(file.get(PICT, 128).is_none(), "id of another type");
    }

    #[test]
    fn iter_walks_every_resource_in_map_order() {
        let all: Vec<_> = sample().iter().map(|r| (r.res_type(), r.id())).collect();
        assert_eq!(
            all,
            vec![(SHIP, 128), (SHIP, 130), (SHIP, -1), (PICT, -5), (DESC, 0)]
        );
    }

    #[test]
    fn data_borrows_from_the_owned_buffer() {
        let file = sample();
        let buffer = file.bytes.as_ptr_range();
        for res in file.iter() {
            let data = res.data().as_ptr_range();
            assert!(buffer.start <= data.start && data.end <= buffer.end);
        }
    }

    #[test]
    fn accepts_trailing_bytes_after_the_map() {
        let file = parse(
            &ForkBuilder::new()
                .resource(PICT, 1, None, b"x")
                .trailing(926),
        );
        assert_eq!(file.get(PICT, 1).expect("present").data(), b"x");
    }

    #[test]
    fn accepts_a_gap_before_the_data_section() {
        let file = parse(&ForkBuilder::new().resource(PICT, 1, None, b"x").gap(240));
        assert_eq!(file.get(PICT, 1).expect("present").data(), b"x");
    }

    #[test]
    fn from_bytes_reports_parse_errors() {
        assert_eq!(
            ResourceFile::from_bytes(vec![0; 3]).err(),
            Some(ParseError::HeaderTruncated)
        );
    }
}
