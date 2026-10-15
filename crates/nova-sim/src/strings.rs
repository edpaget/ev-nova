//! The string lists `_LoadStrings` (@0x71af5) builds once at start-up,
//! each entry from a plug-in's `'STR '` patch when one exists, else from
//! its `STR#`, for the lists the session reads through its
//! [`CommCatalog`]: `STR#` 4001, the cargo names (`_lcCargoName`), from
//! `'STR '` 9100-9355 (@0x71ba4-0x71bf1); 4002, the status bar's cargo
//! abbreviations, from 9200-9455 (@0x71c91-0x71cd8); and 4003, the status
//! bar's commodity labels, from 9400-9405 (@0x71b57-0x71ba2). The ranges
//! overlap as the original's do, so 9400-9405 patch 4002 #201-#206 and
//! 4003 #1-#6 alike.
//!
//! `STR#` 4000 and 4004, the commodities' names and prices, are patched
//! by the market's own [`CommodityStrings`](crate::catalog::CommodityStrings)
//! (see [`market`](crate::market)).
//!
//! A patch ([`StringPatch`]) with text stands for its entry, whatever the
//! list holds. One with no bytes, which `_LoadPluginString` (@0x71a8e)
//! finds and copies nothing of, leaves the entry as it was before the
//! one load, empty; one that cannot be read is empty too, as the market
//! reads both. An entry past the list's end with no patch is empty, and
//! empty entries after the last listed or patched one are dropped, so an
//! unpatched list reads as it is.

use crate::catalog::{CommCatalog, StringPatch};

/// The lists patched here: each `STR#`, the first `'STR '` patching it,
/// and how many entries `_LoadStrings` builds.
pub const PATCHED_LISTS: [(i16, i16, usize); 3] =
    [(4001, 9100, 256), (4002, 9200, 256), (4003, 9400, 6)];

/// `list` with its first `entries` entries patched by `patch(i)`, the
/// patch of entry `i` (see the module docs).
#[must_use]
pub fn patched_list(
    list: &[String],
    entries: usize,
    patch: impl Fn(usize) -> StringPatch,
) -> Vec<String> {
    let mut built: Vec<Option<String>> = (0..entries.max(list.len()))
        .map(|at| match (at < entries).then(|| patch(at)) {
            Some(StringPatch::Text(text)) => Some(text),
            Some(StringPatch::Empty | StringPatch::Unreadable) => Some(String::new()),
            Some(StringPatch::Absent) | None => list.get(at).cloned(),
        })
        .collect();
    while built.last().is_some_and(Option::is_none) {
        built.pop();
    }
    built.into_iter().map(Option::unwrap_or_default).collect()
}

/// A catalog whose patched lists ([`PATCHED_LISTS`]) are built once, as
/// `_LoadStrings` builds them, from `inner`'s lists and `'STR '`
/// patches; every other question goes to `inner`.
#[derive(Debug)]
pub struct PatchedStrings<C> {
    inner: C,
    lists: Vec<(i16, Vec<String>)>,
}

impl<C: CommCatalog> PatchedStrings<C> {
    /// `inner`'s lists, patched.
    pub fn new(inner: C) -> Self {
        let lists = PATCHED_LISTS
            .iter()
            .map(|&(id, first, entries)| {
                let patch = |at: usize| {
                    i16::try_from(at)
                        .ok()
                        .and_then(|at| first.checked_add(at))
                        .map_or(StringPatch::Absent, |id| inner.plugin_string(id))
                };
                (id, patched_list(&inner.string_list(id), entries, patch))
            })
            .collect();
        Self { inner, lists }
    }
}

impl<C: CommCatalog> CommCatalog for PatchedStrings<C> {
    fn string_list(&self, id: i16) -> Vec<String> {
        match self.lists.iter().find(|(list, _)| *list == id) {
            Some((_, strings)) => strings.clone(),
            None => self.inner.string_list(id),
        }
    }

    fn leet_mode(&self) -> bool {
        self.inner.leet_mode()
    }

    fn plugin_string(&self, id: i16) -> StringPatch {
        self.inner.plugin_string(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(strings: &[&str]) -> Vec<String> {
        strings.iter().map(|&string| string.to_owned()).collect()
    }

    fn text(text: &str) -> StringPatch {
        StringPatch::Text(text.to_owned())
    }

    #[test]
    fn an_unpatched_list_reads_as_it_is() {
        let stock = list(&["food", "*passengers", "industrial"]);
        assert_eq!(patched_list(&stock, 256, |_| StringPatch::Absent), stock);
        assert_eq!(
            patched_list(&[], 6, |_| StringPatch::Absent),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_patch_replaces_its_entry_whatever_the_list_holds() {
        let stock = list(&["food", "*passengers", "industrial"]);
        let patched = patched_list(&stock, 256, |at| match at {
            0 => text("grain"),
            2 => text(""),
            _ => StringPatch::Absent,
        });
        assert_eq!(patched, list(&["grain", "*passengers", ""]));
    }

    #[test]
    fn an_empty_or_unreadable_patch_reads_empty() {
        let stock = list(&["food", "*passengers", "industrial"]);
        let patched = patched_list(&stock, 256, |at| match at {
            0 => StringPatch::Empty,
            1 => StringPatch::Unreadable,
            _ => StringPatch::Absent,
        });
        assert_eq!(patched, list(&["", "", "industrial"]));
    }

    #[test]
    fn a_patch_past_the_lists_end_extends_it_with_empties_between() {
        let stock = list(&["food"]);
        let patched = patched_list(&stock, 256, |at| match at {
            3 => text("gems"),
            _ => StringPatch::Absent,
        });
        assert_eq!(patched, list(&["food", "", "", "gems"]));
    }

    #[test]
    fn only_the_first_entries_are_patched() {
        let stock = list(&["a", "b", "c", "d", "e", "f", "g", "h"]);
        let patched = patched_list(&stock, 6, |_| text("x"));
        assert_eq!(patched, list(&["x", "x", "x", "x", "x", "x", "g", "h"]));
        assert_eq!(patched_list(&[], 2, |_| text("x")), list(&["x", "x"]));
    }

    /// `STR#` 4001-4003 and 128, and `'STR '` patches by ID.
    struct Fake {
        patches: Vec<(i16, StringPatch)>,
    }

    impl CommCatalog for Fake {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                4001 => list(&["food", "*passengers"]),
                4002 => list(&["Food", "Pass"]),
                4003 => list(&["Food:", "Ind:"]),
                128 => list(&["kept"]),
                _ => Vec::new(),
            }
        }

        fn leet_mode(&self) -> bool {
            true
        }

        fn plugin_string(&self, id: i16) -> StringPatch {
            self.patches
                .iter()
                .find(|(patched, _)| *patched == id)
                .map_or(StringPatch::Absent, |(_, patch)| patch.clone())
        }
    }

    #[test]
    fn each_patched_list_reads_its_own_base() {
        let strings = PatchedStrings::new(Fake {
            patches: vec![
                (9100, text("grain")),
                (9201, text("Psg")),
                (9401, text("Mfg:")),
            ],
        });
        let cargo = strings.string_list(4001);
        assert_eq!(cargo[..2], list(&["grain", "*passengers"]));
        assert_eq!(cargo.len(), 102, "9201 is 4001's #102 too");
        assert_eq!(cargo[101], "Psg");
        let abbreviations = strings.string_list(4002);
        assert_eq!(abbreviations[..2], list(&["Food", "Psg"]));
        assert_eq!(abbreviations.len(), 202, "9401 is 4002's #202 too");
        assert_eq!(abbreviations[201], "Mfg:");
        assert_eq!(strings.string_list(4003), list(&["Food:", "Mfg:"]));
        assert_eq!(strings.string_list(128), list(&["kept"]), "not patched");
        assert!(strings.leet_mode(), "asked of the inner catalog");
        assert_eq!(strings.plugin_string(9100), text("grain"));
    }

    #[test]
    fn the_overlap_of_9400_to_9405_patches_both_4002_and_4003() {
        let strings = PatchedStrings::new(Fake {
            patches: vec![(9400, text("Shared"))],
        });
        assert_eq!(strings.string_list(4003)[0], "Shared");
        assert_eq!(strings.string_list(4002).len(), 201);
        assert_eq!(strings.string_list(4002)[200], "Shared", "#201");
        assert_eq!(strings.string_list(4002)[1], "Pass");
    }

    #[test]
    fn the_last_patch_of_4001_is_9355() {
        let strings = PatchedStrings::new(Fake {
            patches: vec![(9355, text("last")), (9356, text("past"))],
        });
        let cargo = strings.string_list(4001);
        assert_eq!(cargo.len(), 256);
        assert_eq!(cargo[255], "last");
    }
}
