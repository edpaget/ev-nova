//! [`MemoryPilots`], a [`PilotStore`] in memory that records each write
//! and delete, the fake of the pilot store port.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::io;
use std::rc::Rc;

use crate::saves::PilotStore;

/// A [`PilotStore`] in memory that records each write and delete. Clones
/// share the saves, the records and the failure switches, so a test can seed the
/// store before the app takes a clone, read back what was saved and when,
/// or "restart" over another clone. Its keys are told apart by case, unless
/// it is made [`ignoring_case`](Self::ignoring_case).
#[derive(Clone, Debug, Default)]
pub struct MemoryPilots {
    /// How keys match, if ignoring case: `Some(keeps_spelling)`.
    ignores_case: Option<bool>,
    saves: Rc<RefCell<BTreeMap<String, String>>>,
    written: Rc<RefCell<Vec<(String, String)>>>,
    fail_reads: Rc<Cell<bool>>,
    fail_writes: Rc<Cell<bool>>,
    deleted: Rc<RefCell<Vec<String>>>,
    fail_deletes: Rc<Cell<bool>>,
    last: Rc<RefCell<Option<String>>>,
    fail_last_reads: Rc<Cell<bool>>,
    fail_last_writes: Rc<Cell<bool>>,
}

impl MemoryPilots {
    /// A store with nothing saved.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A store with nothing saved whose keys ignore case, as the Mac's file
    /// names do: one save for "shane" and "Shane", which each key reads,
    /// writes and deletes. A write over a save in another spelling keeps
    /// the spelling saved when `keeps_spelling`, or renames it to the key
    /// written.
    #[must_use]
    pub fn ignoring_case(keeps_spelling: bool) -> Self {
        Self {
            ignores_case: Some(keeps_spelling),
            ..Self::default()
        }
    }

    /// The key `key` is saved under, if anything is: itself, or when the
    /// store ignores case, the saved key that matches it ignoring case.
    fn saved_as(&self, key: &str) -> Option<String> {
        let saves = self.saves.borrow();
        if self.ignores_case.is_none() {
            return saves.contains_key(key).then(|| key.to_owned());
        }
        let key = key.to_lowercase();
        saves
            .keys()
            .find(|saved| saved.to_lowercase() == key)
            .cloned()
    }

    /// Saves `text` under `key`, as if it had been written; it does not
    /// count as a write.
    pub fn put(&self, key: &str, text: &str) {
        let mut at = key.to_owned();
        if let Some(saved) = self.saved_as(key) {
            self.saves.borrow_mut().remove(&saved);
            if self.ignores_case == Some(true) {
                at = saved;
            }
        }
        self.saves.borrow_mut().insert(at, text.to_owned());
    }

    /// The text saved under `key`, if any.
    #[must_use]
    pub fn text(&self, key: &str) -> Option<String> {
        let saved = self.saved_as(key)?;
        self.saves.borrow().get(&saved).cloned()
    }

    /// Every key saved, in order.
    #[must_use]
    pub fn keys(&self) -> Vec<String> {
        self.saves.borrow().keys().cloned().collect()
    }

    /// How many writes have succeeded.
    #[must_use]
    pub fn writes(&self) -> usize {
        self.written.borrow().len()
    }

    /// Each write that succeeded, in order: its key and its text.
    #[must_use]
    pub fn written(&self) -> Vec<(String, String)> {
        self.written.borrow().clone()
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

    /// Each key a delete succeeded on, in order, whether or not anything
    /// was saved under it.
    #[must_use]
    pub fn deleted(&self) -> Vec<String> {
        self.deleted.borrow().clone()
    }

    /// Makes every delete fail (`true`), keeping what was saved, or
    /// succeed again.
    pub fn fail_deletes(&self, fail: bool) {
        self.fail_deletes.set(fail);
    }

    /// The key the last-pilot record names, if any.
    #[must_use]
    pub fn last(&self) -> Option<String> {
        self.last.borrow().clone()
    }

    /// Sets the last-pilot record to name `key`, as if it had been written.
    pub fn put_last(&self, key: &str) {
        *self.last.borrow_mut() = Some(key.to_owned());
    }

    /// Makes every read of the last-pilot record fail (`true`), or succeed
    /// again. Reads of pilots are untouched.
    pub fn fail_last_reads(&self, fail: bool) {
        self.fail_last_reads.set(fail);
    }

    /// Makes every write of the last-pilot record fail (`true`), keeping
    /// the record as it was, or succeed again. Writes of pilots are
    /// untouched.
    pub fn fail_last_writes(&self, fail: bool) {
        self.fail_last_writes.set(fail);
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
        self.written
            .borrow_mut()
            .push((key.to_owned(), text.to_owned()));
        Ok(())
    }

    fn delete(&self, key: &str) -> io::Result<()> {
        if self.fail_deletes.get() {
            return Err(io::Error::other("the disk is locked"));
        }
        if let Some(saved) = self.saved_as(key) {
            self.saves.borrow_mut().remove(&saved);
        }
        self.deleted.borrow_mut().push(key.to_owned());
        Ok(())
    }

    fn last_pilot(&self) -> io::Result<Option<String>> {
        if self.fail_last_reads.get() {
            return Err(io::Error::other("the record is unreadable"));
        }
        Ok(self.last())
    }

    fn set_last_pilot(&self, key: &str) -> io::Result<()> {
        if self.fail_last_writes.get() {
            return Err(io::Error::other("the record is locked"));
        }
        self.put_last(key);
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
    fn a_store_ignoring_case_keeps_one_save_for_both_spellings() {
        for keeps_spelling in [false, true] {
            let store = MemoryPilots::ignoring_case(keeps_spelling);
            store.write("shane", "old").expect("writes");
            store.write("Shane", "new").expect("writes");
            let spelling = if keeps_spelling { "shane" } else { "Shane" };
            assert_eq!(store.list().expect("lists"), [spelling], "{keeps_spelling}");
            for key in ["shane", "Shane", "SHANE"] {
                assert_eq!(store.read(key).expect("reads").as_deref(), Some("new"));
            }
            store.delete("SHANE").expect("deletes");
            assert_eq!(store.keys(), Vec::<String>::new(), "{keeps_spelling}");
        }
        let store = MemoryPilots::new();
        store.write("shane", "old").expect("writes");
        store.write("Shane", "new").expect("writes");
        assert_eq!(store.keys(), ["Shane", "shane"], "case kept apart");
        assert_eq!(store.read("SHANE").expect("reads"), None);
        store.delete("SHANE").expect("deletes");
        assert_eq!(store.keys(), ["Shane", "shane"]);
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
    fn it_logs_each_successful_write_across_clones() {
        let store = MemoryPilots::new();
        let other = store.clone();
        store.write("a", "one").expect("writes");
        other.write("b", "two").expect("writes");
        store.write("a", "three").expect("writes");
        store.put("c", "not a write");
        store.fail_writes(true);
        assert!(store.write("a", "lost").is_err());
        assert_eq!(
            other.written(),
            [
                ("a".to_owned(), "one".to_owned()),
                ("b".to_owned(), "two".to_owned()),
                ("a".to_owned(), "three".to_owned()),
            ]
        );
    }

    #[test]
    fn a_delete_removes_the_text_and_is_recorded() {
        let store = MemoryPilots::new();
        let other = store.clone();
        store.put("a", "one");
        store.put("b", "two");
        store.delete("a").expect("deletes");
        other.delete("c").expect("an absent key deletes");
        assert_eq!(store.keys(), ["b"]);
        assert_eq!(store.text("a"), None);
        assert_eq!(other.deleted(), ["a", "c"]);
        assert_eq!(store.writes(), 0, "a delete is not a write");
        store.fail_deletes(true);
        assert!(store.delete("b").is_err());
        assert_eq!(
            (store.keys(), store.deleted().len()),
            (vec!["b".to_owned()], 2)
        );
        store.fail_deletes(false);
        store.delete("b").expect("deletes again");
        assert_eq!(store.keys(), Vec::<String>::new());
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

    #[test]
    fn clones_share_the_last_pilot_record_apart_from_the_pilots() {
        let store = MemoryPilots::new();
        let other = store.clone();
        assert_eq!(store.last_pilot().expect("reads"), None);
        store.set_last_pilot("Ada").expect("writes");
        assert_eq!(other.last_pilot().expect("reads").as_deref(), Some("Ada"));
        assert_eq!(other.last().as_deref(), Some("Ada"));
        assert_eq!(
            (store.keys(), store.writes()),
            (Vec::<String>::new(), 0),
            "the record is no pilot and no write"
        );
        other.put_last("Bea");
        assert_eq!(store.last().as_deref(), Some("Bea"));
    }

    #[test]
    fn the_last_pilot_record_fails_when_told_to_apart_from_the_pilots() {
        let store = MemoryPilots::new();
        store.put_last("Ada");
        store.fail_last_writes(true);
        assert!(store.set_last_pilot("Bea").is_err());
        assert_eq!(store.last().as_deref(), Some("Ada"));
        store.write("Cy", "saved").expect("pilot writes still work");
        store.fail_last_writes(false);
        store.set_last_pilot("Bea").expect("writes");
        assert_eq!(store.last().as_deref(), Some("Bea"));
        store.fail_last_reads(true);
        assert!(store.last_pilot().is_err());
        assert!(store.read("Cy").is_ok(), "pilot reads still work");
        store.fail_last_reads(false);
        assert_eq!(store.last_pilot().expect("reads").as_deref(), Some("Bea"));
        store.fail_reads(true);
        store.fail_writes(true);
        assert!(
            store.last_pilot().is_ok(),
            "pilot failures leave the record"
        );
        assert!(store.set_last_pilot("Dee").is_ok());
    }
}
