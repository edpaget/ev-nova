//! `otool -l`: the load commands, read for each section's bounds.

/// One section of a segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// Its segment (`__TEXT`, `__DATA`, ...).
    pub segment: String,
    /// Its name (`__const`, `__cfstring`, ...).
    pub name: String,
    /// Its address.
    pub addr: u32,
    /// Its size in bytes.
    pub size: u32,
}

impl Section {
    /// One past its last byte.
    #[must_use]
    pub fn end(&self) -> u32 {
        self.addr.saturating_add(self.size)
    }
}

/// Parses an `otool -l` listing into its sections, by address. Segment
/// commands and the other load commands are skipped, and so is a section
/// missing its name, segment, address or size.
#[must_use]
pub fn parse(listing: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    // The fields of the section being read; `None` outside a section.
    let mut fields: Option<Fields> = None;
    for line in listing.lines() {
        let line = line.trim();
        if line == "Section" || line.starts_with("Load command ") {
            sections.extend(fields.take().and_then(Fields::section));
            fields = (line == "Section").then(Fields::default);
            continue;
        }
        let Some(fields) = fields.as_mut() else {
            continue;
        };
        match line.split_once(' ') {
            Some(("sectname", name)) => fields.name = Some(name.trim().to_owned()),
            Some(("segname", segment)) => fields.segment = Some(segment.trim().to_owned()),
            Some(("addr", addr)) => fields.addr = number(addr),
            Some(("size", size)) => fields.size = number(size),
            _ => {}
        }
    }
    sections.extend(fields.and_then(Fields::section));
    sections.sort_by_key(|s| s.addr);
    sections
}

/// A `0x`-prefixed hex number, as `otool -l` prints addresses and sizes.
fn number(s: &str) -> Option<u32> {
    s.trim().strip_prefix("0x").and_then(super::hex)
}

/// A section's fields as they are read.
#[derive(Default)]
struct Fields {
    segment: Option<String>,
    name: Option<String>,
    addr: Option<u32>,
    size: Option<u32>,
}

impl Fields {
    fn section(self) -> Option<Section> {
        Some(Section {
            segment: self.segment?,
            name: self.name?,
            addr: self.addr?,
            size: self.size?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::fixture::SECTIONS;

    fn section(segment: &str, name: &str, addr: u32, size: u32) -> Section {
        Section {
            segment: segment.into(),
            name: name.into(),
            addr,
            size,
        }
    }

    #[test]
    fn reads_every_section_by_address() {
        assert_eq!(
            parse(SECTIONS),
            [
                section("__TEXT", "__text", 0x3000, 0xa2),
                section("__TEXT", "__cstring", 0xd_8200, 0x60),
                section("__TEXT", "__const", 0xd_d060, 0x8),
                section("__DATA", "__cfstring", 0xd_d068, 0x8),
                section("__DATA", "__data", 0xd_d070, 0xa_97b8),
            ]
        );
    }

    #[test]
    fn a_section_ends_after_its_last_byte() {
        assert_eq!(section("__TEXT", "__const", 0xd_d060, 0x8).end(), 0xd_d068);
    }

    #[test]
    fn a_segment_s_own_fields_are_not_a_section() {
        // A segment command has a `segname` and a `vmaddr`, but no
        // `sectname`: nothing here is a section.
        let segment = "Load command 1\n      cmd LC_SEGMENT\n  segname __TEXT\n   vmaddr 0x00001000\n   vmsize 0x000dd000\n";
        assert!(parse(segment).is_empty());
    }

    #[test]
    fn a_section_missing_a_field_is_left_out() {
        let listing = "Section\n  sectname __const\n   segname __TEXT\n      addr 0x000dd040\n\
                       Section\n  sectname __data\n   segname __DATA\n      addr 0x000de920\n      size 0x00000fa8\n";
        assert_eq!(
            parse(listing),
            [section("__DATA", "__data", 0xd_e920, 0xfa8)]
        );
    }

    #[test]
    fn sections_come_out_by_address_whatever_the_listing_order() {
        let listing = "Section\n  sectname __b\n   segname __X\n      addr 0x00002000\n      size 0x10\n\
                       Section\n  sectname __a\n   segname __X\n      addr 0x00001000\n      size 0x10\n";
        let names: Vec<_> = parse(listing).into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["__a", "__b"]);
    }
}
