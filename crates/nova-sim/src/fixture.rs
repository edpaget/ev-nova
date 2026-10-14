//! Test support for tests outside the crate's own unit tests (the
//! `fixture` feature): [`MemoryPilots`], a [`PilotStore`] in memory; the
//! storyline walker, [`StoryWalker`], which drives a session through a
//! mission chain from a script of [`Step`]s (see the `walker` module); and
//! [`bits_allowing`], the control bits that make a test hold, for setting
//! a pilot up directly; [`disable`], for disabling a ship directly;
//! [`cloak`], for setting a ship's cloak directly; and
//! [`listed_stellars`], a fake catalog's listed stellars derived from its
//! landing sites.

mod bits;
mod walker;

pub use bits::bits_allowing;
pub use walker::{
    Became, Done, Fault, Looked, Step, StoryWalker, WalkFailure, Walked, candidates, expected_bits,
    expected_pay,
};

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::io;
use std::rc::Rc;

use crate::catalog::{LandingSite, ListedStellar, StellarId, SystemId};
use crate::cloak::Cloak;
use crate::combat::hull::Condition;
use crate::saves::PilotStore;
use crate::session::Session;
use crate::traffic::npc::NpcId;

/// Disables NPC `id` in `session`, if it is in the system, its armour
/// down to 1 and its shield to 0: for a test that needs a ship the
/// player cannot shoot, such as one of its escorts, disabled.
pub fn disable(session: &mut Session, id: NpcId) {
    if let Some(npc) = session.npc_mut(id) {
        npc.reserves.shield.now = 0.0;
        npc.reserves.armor.now = 1.0;
        npc.condition = Condition::Disabled;
    }
}

/// Sets NPC `id`'s cloak in `session` to `to`, if it is in the system:
/// for a test that draws a ship cloaked.
pub fn cloak(session: &mut Session, id: NpcId, to: Cloak) {
    if let Some(npc) = session.npc_mut(id) {
        npc.cloak = to;
    }
}

/// Every stellar of `sites` (each system's landing sites, as a fake
/// `PilotCatalog::landing_sites` gives them), by ascending ID, with every
/// system listing it, by ascending ID, each once, and the flags and
/// government of its first site: the listed stellars
/// (`PilotCatalog::listed_stellars`) a fake gives for those sites, as the
/// data's `NavDefs` give them. A session shows only the stellars its
/// system holds by these, so a fake that lists none shows none.
#[must_use]
pub fn listed_stellars<'a>(
    sites: impl IntoIterator<Item = (SystemId, &'a [LandingSite])>,
) -> Vec<ListedStellar> {
    let mut listed: BTreeMap<StellarId, ListedStellar> = BTreeMap::new();
    for (system, sites) in sites {
        for site in sites {
            let stellar = listed.entry(site.id).or_insert_with(|| ListedStellar {
                id: site.id,
                systems: Vec::new(),
                flags: site.flags,
                flags2: site.flags2,
                govt: site.govt,
            });
            if !stellar.systems.contains(&system) {
                stellar.systems.push(system);
                stellar.systems.sort_unstable();
            }
        }
    }
    listed.into_values().collect()
}

/// Adds to `listed` (as [`listed_stellars`] gives it) each `(system,
/// stellar)` of `listers`: a `NavDefs` entry with no landing site, such as
/// a gate in a system the ship never flies in. A stellar not yet listed is
/// added with no flags and no government.
pub fn list_also(
    listed: &mut Vec<ListedStellar>,
    listers: impl IntoIterator<Item = (SystemId, StellarId)>,
) {
    for (system, stellar) in listers {
        let at = listed
            .binary_search_by_key(&stellar, |listed| listed.id)
            .unwrap_or_else(|at| {
                listed.insert(
                    at,
                    ListedStellar {
                        id: stellar,
                        systems: Vec::new(),
                        flags: 0,
                        flags2: 0,
                        govt: None,
                    },
                );
                at
            });
        let systems = &mut listed[at].systems;
        if !systems.contains(&system) {
            systems.push(system);
            systems.sort_unstable();
        }
    }
}

/// A [`PilotStore`] in memory. Clones share the saves and the failure
/// switches, so a test can seed the store before the app takes a clone,
/// read back what was saved, or "restart" over another clone.
#[derive(Clone, Debug, Default)]
pub struct MemoryPilots {
    saves: Rc<RefCell<BTreeMap<String, String>>>,
    writes: Rc<Cell<usize>>,
    fail_reads: Rc<Cell<bool>>,
    fail_writes: Rc<Cell<bool>>,
}

impl MemoryPilots {
    /// A store with nothing saved.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Saves `text` under `key`, as if it had been written; it does not
    /// count as a write.
    pub fn put(&self, key: &str, text: &str) {
        self.saves
            .borrow_mut()
            .insert(key.to_owned(), text.to_owned());
    }

    /// The text saved under `key`, if any.
    #[must_use]
    pub fn text(&self, key: &str) -> Option<String> {
        self.saves.borrow().get(key).cloned()
    }

    /// Every key saved, in order.
    #[must_use]
    pub fn keys(&self) -> Vec<String> {
        self.saves.borrow().keys().cloned().collect()
    }

    /// How many writes have succeeded.
    #[must_use]
    pub fn writes(&self) -> usize {
        self.writes.get()
    }

    /// Makes every read and list fail (`true`) or succeed again.
    pub fn fail_reads(&self, fail: bool) {
        self.fail_reads.set(fail);
    }

    /// Makes every write fail (`true`), keeping what was saved before, or
    /// succeed again.
    pub fn fail_writes(&self, fail: bool) {
        self.fail_writes.set(fail);
    }

    fn check_reads(&self) -> io::Result<()> {
        if self.fail_reads.get() {
            return Err(io::Error::other("the disk is unreadable"));
        }
        Ok(())
    }
}

impl PilotStore for MemoryPilots {
    fn list(&self) -> io::Result<Vec<String>> {
        self.check_reads()?;
        Ok(self.keys())
    }

    fn read(&self, key: &str) -> io::Result<Option<String>> {
        self.check_reads()?;
        Ok(self.text(key))
    }

    fn write(&self, key: &str, text: &str) -> io::Result<()> {
        if self.fail_writes.get() {
            return Err(io::Error::other("the disk is full"));
        }
        self.put(key, text);
        self.writes.set(self.writes.get() + 1);
        Ok(())
    }

    /// Always `memory`.
    fn location(&self) -> String {
        "memory".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::planet;

    #[test]
    fn the_listed_stellars_are_every_site_by_id_with_every_system_listing_it() {
        let flagged = LandingSite {
            flags: 4,
            flags2: 0x2000,
            ..planet(129, 0.0, 0.0)
        };
        let sites = [
            (SystemId(131), vec![flagged, planet(128, 0.0, 0.0)]),
            (
                SystemId(130),
                vec![planet(129, 1.0, 1.0), planet(129, 1.0, 1.0)],
            ),
        ];
        let listed = listed_stellars(sites.iter().map(|(system, sites)| (*system, &sites[..])));
        assert_eq!(
            listed,
            [
                ListedStellar {
                    id: StellarId(128),
                    systems: vec![SystemId(131)],
                    flags: planet(128, 0.0, 0.0).flags,
                    flags2: 0,
                    govt: None,
                },
                ListedStellar {
                    id: StellarId(129),
                    systems: vec![SystemId(130), SystemId(131)],
                    flags: 4,
                    flags2: 0x2000,
                    govt: None,
                },
            ],
            "by ascending ID, each system once and in order, the first site's flags"
        );
        assert_eq!(listed_stellars([]), []);
    }

    #[test]
    fn listing_also_adds_each_lister_in_order_and_each_stellar_by_id() {
        let sites = [(SystemId(131), vec![planet(129, 0.0, 0.0)])];
        let mut listed = listed_stellars(sites.iter().map(|(system, sites)| (*system, &sites[..])));
        list_also(
            &mut listed,
            [
                (SystemId(130), StellarId(129)),
                (SystemId(131), StellarId(129)),
                (SystemId(132), StellarId(140)),
                (SystemId(133), StellarId(128)),
            ],
        );
        let gate = |id, systems: &[i16]| ListedStellar {
            id: StellarId(id),
            systems: systems.iter().copied().map(SystemId).collect(),
            flags: 0,
            flags2: 0,
            govt: None,
        };
        assert_eq!(
            listed,
            [
                gate(128, &[133]),
                ListedStellar {
                    systems: vec![SystemId(130), SystemId(131)],
                    ..listed_stellars([(SystemId(131), &[planet(129, 0.0, 0.0)][..])])[0].clone()
                },
                gate(140, &[132]),
            ]
        );
    }

    #[test]
    fn clones_share_the_saves_and_count_writes() {
        let store = MemoryPilots::new();
        let other = store.clone();
        assert_eq!(store.list().expect("lists"), Vec::<String>::new());
        store.write("b", "two").expect("writes");
        store.write("a", "one").expect("writes");
        assert_eq!(other.list().expect("lists"), ["a", "b"]);
        assert_eq!(other.read("a").expect("reads").as_deref(), Some("one"));
        assert_eq!(other.read("c").expect("reads"), None);
        assert_eq!(other.writes(), 2);
        other.put("c", "three");
        assert_eq!(store.text("c").as_deref(), Some("three"));
        assert_eq!(store.writes(), 2, "a put is not a write");
    }

    #[test]
    fn it_fails_when_told_to() {
        let store = MemoryPilots::new();
        store.put("a", "kept");
        store.fail_reads(true);
        assert!(store.read("a").is_err());
        assert!(store.list().is_err());
        store.fail_reads(false);
        assert!(store.read("a").is_ok());
        assert!(store.list().is_ok());
        store.fail_writes(true);
        assert!(store.write("a", "lost").is_err());
        assert_eq!(
            (store.text("a").as_deref(), store.writes()),
            (Some("kept"), 0)
        );
        store.fail_writes(false);
        store.write("a", "new").expect("writes");
        assert_eq!(store.text("a").as_deref(), Some("new"));
        assert_eq!(store.location(), "memory");
    }
}
