//! `otool -Iv`: which `__pointers` slots name an imported symbol.

use std::collections::BTreeMap;

/// Parses the `(__IMPORT,__pointers)` table into slot address -> import
/// name. `LOCAL` and `ABSOLUTE` slots are left out (their value is read
/// from the pointers listing), and so are the other tables (the
/// `__jump_table` stubs, which otool already names in the disassembly).
pub fn parse(listing: &str) -> BTreeMap<u32, String> {
    let mut slots = BTreeMap::new();
    let mut in_pointers = false;
    for line in listing.lines() {
        if let Some(header) = line.strip_prefix("Indirect symbols for ") {
            in_pointers = header.starts_with("(__IMPORT,__pointers)");
            continue;
        }
        if !in_pointers {
            continue;
        }
        let mut fields = line.split_whitespace();
        let Some(addr) = fields
            .next()
            .and_then(|a| a.strip_prefix("0x"))
            .and_then(super::hex)
        else {
            continue;
        };
        match (fields.next(), fields.next()) {
            (Some("LOCAL" | "ABSOLUTE") | None, _) | (_, None) => {}
            (Some(_), Some(name)) => {
                slots.insert(addr, name.to_owned());
            }
        }
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::fixture::INDIRECT;

    #[test]
    fn names_the_import_slots_only() {
        let slots = parse(INDIRECT);
        let got: Vec<_> = slots.iter().map(|(a, n)| (*a, n.as_str())).collect();
        assert_eq!(
            got,
            [(0x3b_5000, "_mach_init_routine"), (0x3b_5004, "_errno")]
        );
    }
}
