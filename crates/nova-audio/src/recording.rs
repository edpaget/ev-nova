//! Port mocks, for tests: a recording [`Audio`] that plays nothing and
//! keeps every command it is given, and [`MemorySettings`], a
//! [`SettingsStore`] that keeps the saved text in memory.
//!
//! Available to this crate's tests and, through the `recording` feature,
//! to other crates' tests. Their state is shared, so a test keeps a handle
//! to it after moving the mock into the app.

use std::cell::{Cell, RefCell};
use std::io;
use std::rc::Rc;

use crate::port::{Audio, AudioCommand};
use crate::settings::SettingsStore;

/// The commands an [`Audio`] was given, in order.
pub type AudioLog = Rc<RefCell<Vec<AudioCommand>>>;

/// Records every command; plays nothing.
#[derive(Clone, Debug, Default)]
pub struct RecordingAudio {
    log: AudioLog,
}

impl RecordingAudio {
    /// A mock with an empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A handle to the log, which sees every command given after it too.
    #[must_use]
    pub fn log(&self) -> AudioLog {
        Rc::clone(&self.log)
    }
}

impl Audio for RecordingAudio {
    fn run(&mut self, command: AudioCommand) {
        self.log.borrow_mut().push(command);
    }
}

/// A [`SettingsStore`] in memory. Clones share the saved text and the
/// failure switches, so a test can seed the store before the app takes a
/// clone, read back what was saved, or "restart" by opening a new keeper
/// over another clone.
#[derive(Clone, Debug, Default)]
pub struct MemorySettings {
    text: Rc<RefCell<Option<String>>>,
    writes: Rc<Cell<usize>>,
    fail_reads: Rc<Cell<bool>>,
    fail_writes: Rc<Cell<bool>>,
}

impl MemorySettings {
    /// A store with nothing saved.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A store holding `text`, as if it had been saved.
    #[must_use]
    pub fn holding(text: &str) -> Self {
        let store = Self::new();
        *store.text.borrow_mut() = Some(text.to_owned());
        store
    }

    /// The saved text, if any.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        self.text.borrow().clone()
    }

    /// How many writes have succeeded.
    #[must_use]
    pub fn writes(&self) -> usize {
        self.writes.get()
    }

    /// Makes every read fail (`true`) or succeed again.
    pub fn fail_reads(&self, fail: bool) {
        self.fail_reads.set(fail);
    }

    /// Makes every write fail (`true`), keeping the text saved before, or
    /// succeed again.
    pub fn fail_writes(&self, fail: bool) {
        self.fail_writes.set(fail);
    }
}

impl SettingsStore for MemorySettings {
    fn read(&mut self) -> io::Result<Option<String>> {
        if self.fail_reads.get() {
            return Err(io::Error::other("the disk is unreadable"));
        }
        Ok(self.text())
    }

    fn write(&mut self, text: &str) -> io::Result<()> {
        if self.fail_writes.get() {
            return Err(io::Error::other("the disk is full"));
        }
        *self.text.borrow_mut() = Some(text.to_owned());
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
    fn it_records_every_command_in_order_for_every_handle() {
        let mut audio = RecordingAudio::new();
        let before = audio.log();
        audio.run(AudioCommand::StopEffects);
        let after = audio.log();
        audio.run(AudioCommand::StopMusic);
        let expected = [AudioCommand::StopEffects, AudioCommand::StopMusic];
        assert_eq!(*before.borrow(), expected);
        assert_eq!(*after.borrow(), expected);
    }

    #[test]
    fn the_memory_store_shares_its_text_across_clones() {
        let mut store = MemorySettings::new();
        let other = store.clone();
        assert_eq!(store.read().expect("reads"), None);
        store.write("saved").expect("writes");
        assert_eq!(other.text().as_deref(), Some("saved"));
        assert_eq!(other.writes(), 1);
        let mut seeded = MemorySettings::holding("seed");
        assert_eq!(seeded.read().expect("reads").as_deref(), Some("seed"));
        assert_eq!(seeded.writes(), 0);
    }

    #[test]
    fn the_memory_store_fails_when_told_to() {
        let mut store = MemorySettings::holding("kept");
        store.fail_reads(true);
        assert!(store.read().is_err());
        store.fail_reads(false);
        assert!(store.read().is_ok());
        store.fail_writes(true);
        assert!(store.write("lost").is_err());
        assert_eq!((store.text().as_deref(), store.writes()), (Some("kept"), 0));
        store.fail_writes(false);
        store.write("new").expect("writes");
        assert_eq!(store.text().as_deref(), Some("new"));
    }
}
