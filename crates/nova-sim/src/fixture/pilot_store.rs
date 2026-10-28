//! The [`PilotStore`] port's contract.
//!
//! [`check_contract`] states what every pilot store must do, starting
//! empty. It runs against [`MemoryPilots`](super::MemoryPilots) here and
//! against `nova`'s `FilePilots` over a fresh temporary directory.
//!
//! The keys never differ only in case, since a macOS file system folds
//! case, and use only characters a Windows file name may hold. The texts
//! hold a newline and non-ASCII characters, so a store that mangles either
//! fails.

use std::io;

use crate::saves::PilotStore;

/// The first pilot's key.
pub const ALPHA: &str = "Alpha";
/// The second pilot's key.
pub const BETA: &str = "Beta";
/// What the first write saves.
pub const FIRST: &str = "Kestrel at Earth\nCrédits: 1000";
/// What the second write saves in its place.
pub const SECOND: &str = "Shuttle at Sol\nZürich";
/// What the second pilot's save holds.
pub const OTHER: &str = "Beta's own save\n";

/// The error for a store call that failed while doing `what`.
fn failed(what: &'static str) -> impl Fn(io::Error) -> String {
    move |error| format!("{what} failed: {error}")
}

/// Checks the port's contract against `store`, which must start empty:
///
/// 1. An empty store lists nothing, reads nothing under [`ALPHA`] and has
///    no last pilot.
/// 2. After a write, a read gives the text back and the key lists.
/// 3. A second write under the same key replaces the first.
/// 4. A second key lists beside the first, in any order.
/// 5. The last-pilot record names the key it was set to, and never lists.
/// 6. A delete removes the save, deleting it again is `Ok`, and deleting
///    the key the last-pilot record names leaves the record alone.
/// 7. The location isn't empty.
///
/// # Errors
///
/// A message naming the first clause `store` breaks.
pub fn check_contract(store: &impl PilotStore) -> Result<(), String> {
    let sorted = |mut keys: Vec<String>| {
        keys.sort();
        keys
    };
    let listed = || store.list().map(sorted).map_err(failed("listing"));
    let read = |key: &str| store.read(key).map_err(failed("reading"));

    if !listed()?.is_empty() {
        return Err(format!("an empty store lists {:?}", listed()?));
    }
    if let Some(text) = read(ALPHA)? {
        return Err(format!("an empty store reads {text:?} under {ALPHA}"));
    }
    if let Some(key) = store
        .last_pilot()
        .map_err(failed("reading the last pilot"))?
    {
        return Err(format!("an empty store's last pilot is {key:?}"));
    }

    store.write(ALPHA, FIRST).map_err(failed("writing"))?;
    match read(ALPHA)? {
        Some(text) if text == FIRST => {}
        other => {
            return Err(format!(
                "after a write, the read gives {other:?}, not the text"
            ));
        }
    }
    if listed()? != [ALPHA] {
        return Err(format!("after a write, the store lists {:?}", listed()?));
    }

    store.write(ALPHA, SECOND).map_err(failed("writing"))?;
    match read(ALPHA)? {
        Some(text) if text == SECOND => {}
        other => {
            return Err(format!(
                "a second write doesn't replace the first: {other:?}"
            ));
        }
    }
    if listed()? != [ALPHA] {
        return Err(format!("a second write lists {:?}, not one key", listed()?));
    }

    store.write(BETA, OTHER).map_err(failed("writing"))?;
    if listed()? != [ALPHA, BETA] {
        return Err(format!("two keys list as {:?}", listed()?));
    }

    store
        .set_last_pilot(BETA)
        .map_err(failed("setting the last pilot"))?;
    match store
        .last_pilot()
        .map_err(failed("reading the last pilot"))?
    {
        Some(key) if key == BETA => {}
        other => return Err(format!("the last pilot is {other:?}, not {BETA:?}")),
    }
    if listed()? != [ALPHA, BETA] {
        return Err(format!(
            "with a last-pilot record, the store lists {:?}",
            listed()?
        ));
    }

    store.delete(ALPHA).map_err(failed("deleting"))?;
    if let Some(text) = read(ALPHA)? {
        return Err(format!("a deleted save still reads {text:?}"));
    }
    store.delete(ALPHA).map_err(failed("deleting again"))?;
    store.delete(BETA).map_err(failed("deleting"))?;
    match store
        .last_pilot()
        .map_err(failed("reading the last pilot"))?
    {
        Some(key) if key == BETA => {}
        other => {
            return Err(format!(
                "deleting a pilot changed the last-pilot record to {other:?}"
            ));
        }
    }
    if !listed()?.is_empty() {
        return Err(format!(
            "after deleting both, the store lists {:?}",
            listed()?
        ));
    }

    if store.location().is_empty() {
        return Err("the location is empty".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::MemoryPilots;

    #[test]
    fn memory_pilots_meet_the_contract() {
        assert_eq!(check_contract(&MemoryPilots::new()), Ok(()));
    }

    /// One way to break [`MemoryPilots`].
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Flaw {
        ListsAGhost,
        ReadsAGhost,
        HasALastPilot,
        ForgetsWrites,
        KeepsTheFirstWrite,
        ListsOneKey,
        ListsTheRecord,
        IgnoresTheLastPilot,
        KeepsDeletes,
        FailsASecondDelete,
        DeletesTheRecord,
        HasNoLocation,
    }

    struct Broken {
        inner: MemoryPilots,
        flaw: Flaw,
    }

    impl PilotStore for Broken {
        fn list(&self) -> io::Result<Vec<String>> {
            let mut keys = self.inner.list()?;
            match self.flaw {
                Flaw::ListsAGhost => keys.push("Ghost".to_owned()),
                Flaw::ListsOneKey => keys.truncate(1),
                Flaw::ListsTheRecord if self.inner.last().is_some() => {
                    keys.push("Last Pilot".to_owned());
                }
                _ => {}
            }
            Ok(keys)
        }

        fn read(&self, key: &str) -> io::Result<Option<String>> {
            let text = self.inner.read(key)?;
            Ok(match self.flaw {
                Flaw::ReadsAGhost if text.is_none() => Some("ghost".to_owned()),
                _ => text,
            })
        }

        fn write(&self, key: &str, text: &str) -> io::Result<()> {
            match self.flaw {
                Flaw::ForgetsWrites => Ok(()),
                Flaw::KeepsTheFirstWrite if self.inner.read(key)?.is_some() => Ok(()),
                _ => self.inner.write(key, text),
            }
        }

        fn delete(&self, key: &str) -> io::Result<()> {
            match self.flaw {
                Flaw::KeepsDeletes => Ok(()),
                Flaw::FailsASecondDelete if self.inner.read(key)?.is_none() => {
                    Err(io::Error::new(io::ErrorKind::NotFound, "no such pilot"))
                }
                Flaw::DeletesTheRecord => {
                    self.inner.delete(key)?;
                    if self.inner.last().as_deref() == Some(key) {
                        self.inner.put_last("");
                    }
                    Ok(())
                }
                _ => self.inner.delete(key),
            }
        }

        fn last_pilot(&self) -> io::Result<Option<String>> {
            match self.flaw {
                Flaw::HasALastPilot => Ok(Some("Ghost".to_owned())),
                _ => self.inner.last_pilot(),
            }
        }

        fn set_last_pilot(&self, key: &str) -> io::Result<()> {
            match self.flaw {
                Flaw::IgnoresTheLastPilot => Ok(()),
                _ => self.inner.set_last_pilot(key),
            }
        }

        fn location(&self) -> String {
            match self.flaw {
                Flaw::HasNoLocation => String::new(),
                _ => self.inner.location(),
            }
        }
    }

    fn broken(flaw: Flaw) -> String {
        check_contract(&Broken {
            inner: MemoryPilots::new(),
            flaw,
        })
        .expect_err("the broken store fails the contract")
    }

    #[test]
    fn rejects_each_flaw_naming_its_clause() {
        for (flaw, clause) in [
            (Flaw::ListsAGhost, "an empty store lists"),
            (Flaw::ReadsAGhost, "an empty store reads"),
            (Flaw::HasALastPilot, "an empty store's last pilot"),
            (Flaw::ForgetsWrites, "after a write, the read"),
            (Flaw::KeepsTheFirstWrite, "doesn't replace"),
            (Flaw::ListsOneKey, "two keys list"),
            (Flaw::ListsTheRecord, "with a last-pilot record"),
            (Flaw::IgnoresTheLastPilot, "the last pilot is"),
            (Flaw::KeepsDeletes, "a deleted save"),
            (Flaw::FailsASecondDelete, "deleting again failed"),
            (Flaw::DeletesTheRecord, "changed the last-pilot record"),
            (Flaw::HasNoLocation, "the location is empty"),
        ] {
            let error = broken(flaw);
            assert!(error.contains(clause), "{clause}: {error}");
        }
    }

    #[test]
    fn rejects_a_store_that_fails() {
        let store = MemoryPilots::new();
        store.fail_reads(true);
        assert!(
            check_contract(&store)
                .expect_err("fails")
                .contains("listing failed")
        );
        let store = MemoryPilots::new();
        store.fail_writes(true);
        assert!(
            check_contract(&store)
                .expect_err("fails")
                .contains("writing failed")
        );
        let store = MemoryPilots::new();
        store.fail_last_reads(true);
        assert!(
            check_contract(&store)
                .expect_err("fails")
                .contains("reading the last pilot failed")
        );
        let store = MemoryPilots::new();
        store.fail_last_writes(true);
        assert!(
            check_contract(&store)
                .expect_err("fails")
                .contains("setting the last pilot failed")
        );
        let store = MemoryPilots::new();
        store.fail_deletes(true);
        assert!(
            check_contract(&store)
                .expect_err("fails")
                .contains("deleting failed")
        );
    }

    #[test]
    fn rejects_a_store_that_isnt_empty() {
        let store = MemoryPilots::new();
        store.put(ALPHA, FIRST);
        assert!(
            check_contract(&store)
                .expect_err("fails")
                .contains("an empty store lists")
        );
    }
}
