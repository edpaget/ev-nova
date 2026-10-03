//! Hand-written fakes of the store's ports, for its unit tests.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use super::fs::{DirLister, EntryKind, Listing};

/// An in-memory directory tree. Each directory's listing is returned in
/// reverse of the order it was given, so tests that give names in a
/// scrambled order also see them scrambled differently; a directory that
/// was never added fails to list. Every listed directory is recorded.
#[derive(Default)]
pub struct FakeTree {
    dirs: HashMap<PathBuf, Vec<Listing>>,
    pub calls: RefCell<Vec<PathBuf>>,
}

impl FakeTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds directory `dir` holding `entries` (name, kind).
    #[must_use]
    pub fn dir(mut self, dir: &str, entries: &[(&str, EntryKind)]) -> Self {
        let listings = entries
            .iter()
            .map(|(name, kind)| Listing {
                name: (*name).into(),
                kind: *kind,
            })
            .collect();
        self.dirs.insert(PathBuf::from(dir), listings);
        self
    }

    /// The directories listed so far, in call order.
    pub fn listed(&self) -> Vec<PathBuf> {
        self.calls.borrow().clone()
    }
}

impl DirLister for FakeTree {
    fn list(&self, dir: &Path) -> io::Result<Vec<Listing>> {
        self.calls.borrow_mut().push(dir.to_path_buf());
        let mut listings = self
            .dirs
            .get(dir)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such fake directory"))?;
        listings.reverse();
        Ok(listings)
    }
}

/// A directory tree with no bottom: every directory holds one file and one
/// sub-folder. Panics past `limit` calls so a walk that never stops fails
/// instead of hanging.
pub struct EndlessTree {
    pub calls: RefCell<usize>,
    pub limit: usize,
}

impl DirLister for EndlessTree {
    fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
        let mut calls = self.calls.borrow_mut();
        *calls += 1;
        assert!(
            *calls <= self.limit,
            "listed more than {} directories",
            self.limit
        );
        Ok(vec![
            Listing {
                name: "deeper".into(),
                kind: EntryKind::Dir,
            },
            Listing {
                name: "plug".into(),
                kind: EntryKind::File,
            },
        ])
    }
}
