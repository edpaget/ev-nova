//! Test doubles for other crates' tests (the `fixture` feature):
//! [`MemoryPilots`], a [`PilotStore`] in memory.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::io;
use std::rc::Rc;

use crate::saves::PilotStore;

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
