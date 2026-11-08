//! The story state of each Ink file a pilot has played.
//!
//! The new game keeps one story state per Ink file, so a file's read counts
//! and spent once-only choices carry from one of its scenes to the next,
//! across missions and stellars, and across a save and load. A file is
//! keyed as a ref names it (`dialog/dock-job.ink`). A conversation reads
//! and writes its file's state
//! ([`Conversation::open`](super::conversation::Conversation::open)); the
//! pilot holds them ([`Pilot::dialogs`](crate::pilot::Pilot::dialogs)).
//!
//! A file the data no longer has keeps its state as saved, as the save
//! keeps other records whose data is gone. It is never read again.

use std::collections::BTreeMap;

use super::SavedDialog;

/// One [`SavedDialog`] per Ink file. A new pilot has none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DialogStates(BTreeMap<String, SavedDialog>);

impl DialogStates {
    /// The state kept for `file`, or `None` if none is.
    #[must_use]
    pub fn get(&self, file: &str) -> Option<&SavedDialog> {
        self.0.get(file)
    }

    /// Keeps `saved` as `file`'s state, replacing any kept before.
    pub fn keep(&mut self, file: &str, saved: SavedDialog) {
        self.0.insert(file.to_owned(), saved);
    }

    /// Each file with its state, by file ascending.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &SavedDialog)> {
        self.0.iter().map(|(file, saved)| (file.as_str(), saved))
    }
}

impl FromIterator<(String, SavedDialog)> for DialogStates {
    fn from_iter<I: IntoIterator<Item = (String, SavedDialog)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(text: &str) -> SavedDialog {
        SavedDialog(text.to_owned())
    }

    #[test]
    fn an_unknown_file_has_no_state() {
        let mut states = DialogStates::default();
        states.keep("dialog/a.ink", saved("a"));
        assert_eq!(states.get("dialog/b.ink"), None);
    }

    #[test]
    fn a_kept_state_reads_back() {
        let mut states = DialogStates::default();
        states.keep("dialog/a.ink", saved("a"));
        assert_eq!(states.get("dialog/a.ink"), Some(&saved("a")));
    }

    #[test]
    fn keeping_replaces_a_file_s_state_and_leaves_the_others() {
        let mut states = DialogStates::default();
        states.keep("dialog/a.ink", saved("a1"));
        states.keep("dialog/b.ink", saved("b"));
        states.keep("dialog/a.ink", saved("a2"));
        assert_eq!(states.get("dialog/a.ink"), Some(&saved("a2")));
        assert_eq!(states.get("dialog/b.ink"), Some(&saved("b")));
    }

    #[test]
    fn states_are_listed_by_file_ascending() {
        let mut states = DialogStates::default();
        states.keep("dialog/b.ink", saved("b"));
        states.keep("dialog/a.ink", saved("a"));
        assert_eq!(
            states.iter().collect::<Vec<_>>(),
            [("dialog/a.ink", &saved("a")), ("dialog/b.ink", &saved("b"))]
        );
    }

    #[test]
    fn states_collect_from_files() {
        let states: DialogStates = [
            ("dialog/b.ink".to_owned(), saved("b")),
            ("dialog/a.ink".to_owned(), saved("a")),
        ]
        .into_iter()
        .collect();
        assert_eq!(states.get("dialog/a.ink"), Some(&saved("a")));
        assert_eq!(states.iter().count(), 2);
    }
}
