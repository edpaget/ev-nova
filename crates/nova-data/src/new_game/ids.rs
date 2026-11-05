//! Nova IDs for the bible's ids: one pure allocation, per kind.
//!
//! Within a kind, records are numbered in id order (the order the loader's
//! maps already give), counting up from 128, the first ID Nova gives
//! data. Each kind has its range: [`GOVT_IDS`] for factions,
//! [`SYSTEM_IDS`] and [`STELLAR_IDS`] for systems and stellars. A record
//! past its range's end gets no ID, and the caller leaves it out.
//!
//! Known limit: an ID depends on how many ids sort before it, so adding a
//! file renumbers every record of its kind after it. Nothing keeps a new
//! game's IDs yet (it has no `chär`, so no pilot is saved), so nothing
//! breaks; phase 3 decides whether IDs need pinning. Phase 3 also adds
//! hypergate `spöb`s for the jump points after the stellars, so they take
//! the IDs past the last stellar's without renumbering any stellar.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use nova_galaxy_format::id::Id;

/// The `gövt` IDs Nova gives data: 128 to 383.
pub const GOVT_IDS: RangeInclusive<i16> = 128..=383;

/// The `sÿst` IDs Nova gives data: 128 to 2175.
pub const SYSTEM_IDS: RangeInclusive<i16> = 128..=2175;

/// The `spöb` IDs Nova gives data: 128 to 2175.
pub const STELLAR_IDS: RangeInclusive<i16> = 128..=2175;

/// The Nova ID of each record that has one, by its bible id, per kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IdTable {
    /// Each faction's `gövt`.
    pub factions: BTreeMap<Id, i16>,
    /// Each system's `sÿst`.
    pub systems: BTreeMap<Id, i16>,
    /// Each stellar's `spöb`.
    pub stellars: BTreeMap<Id, i16>,
}

/// Numbers `ids`, in the order given, from the start of `range`: the
/// table of those that fit, and those past its end, in order.
pub fn allocate<'a>(
    ids: impl IntoIterator<Item = &'a Id>,
    range: RangeInclusive<i16>,
) -> (BTreeMap<Id, i16>, Vec<&'a Id>) {
    let mut table = BTreeMap::new();
    let mut past = Vec::new();
    let mut next = range;
    for id in ids {
        match next.next() {
            Some(nova) => {
                table.insert(id.clone(), nova);
            }
            None => past.push(id),
        }
    }
    (table, past)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    #[test]
    fn counts_up_from_the_range_s_start_in_the_order_given() {
        let ids = [id("alpha"), id("beta"), id("gamma")];
        let (table, past) = allocate(&ids, SYSTEM_IDS);
        assert_eq!(
            table.into_iter().collect::<Vec<_>>(),
            [(id("alpha"), 128), (id("beta"), 129), (id("gamma"), 130)]
        );
        assert!(past.is_empty());
    }

    #[test]
    fn ids_past_the_range_s_end_get_none() {
        let ids = [id("a"), id("b"), id("c"), id("d")];
        let (table, past) = allocate(&ids, 128..=129);
        assert_eq!(
            table.into_iter().collect::<Vec<_>>(),
            [(id("a"), 128), (id("b"), 129)]
        );
        assert_eq!(past, [&id("c"), &id("d")]);
    }

    #[test]
    fn the_ranges_are_nova_s() {
        assert_eq!(GOVT_IDS, 128..=383);
        assert_eq!(SYSTEM_IDS, 128..=2175);
        assert_eq!(STELLAR_IDS, 128..=2175);
    }

    #[test]
    fn a_full_range_numbers_its_last_id() {
        let ids: Vec<_> = (0..256).map(|n| id(&format!("f{n}"))).collect();
        let (table, past) = allocate(&ids, GOVT_IDS);
        assert_eq!(table.len(), 256);
        assert_eq!(table[&id("f255")], 383);
        assert!(past.is_empty());
        let more: Vec<_> = (0..257).map(|n| id(&format!("f{n}"))).collect();
        let (_, past) = allocate(&more, GOVT_IDS);
        assert_eq!(past, [&id("f256")]);
    }
}
