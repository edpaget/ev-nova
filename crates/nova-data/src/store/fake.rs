//! Hand-written fakes of the store's ports, for its unit tests.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use nova_rsrc::{Fork, ForkReader};

use super::fs::{DirLister, EntryKind, Listing};
use super::order::finder_cmp;

/// An in-memory directory tree. Each directory's listing is returned
/// scrambled: put in Finder order and then rotated left by one, so a
/// listing of two or more entries never comes back already sorted, whatever
/// order a test gave its names in, and the walk must sort every folder it
/// lists. A directory that was never added fails to list. Every listed
/// directory is recorded.
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
        listings.sort_by(|a, b| finder_cmp(&a.name, &b.name));
        if !listings.is_empty() {
            listings.rotate_left(1);
        }
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

/// What a fake file holds.
#[derive(Clone)]
pub enum FakeFile {
    /// These data-fork bytes; empty means no fork at all.
    Bytes(Vec<u8>),
    /// Reading fails.
    Unreadable,
}

/// In-memory forks: each file's data fork. Resource forks are always absent.
#[derive(Default)]
pub struct FakeForks {
    files: HashMap<PathBuf, FakeFile>,
}

impl FakeForks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `path` holding the flattened fork `bytes`.
    #[must_use]
    pub fn file(mut self, path: &str, bytes: Vec<u8>) -> Self {
        self.files
            .insert(PathBuf::from(path), FakeFile::Bytes(bytes));
        self
    }

    /// Adds `path`, which fails to read.
    #[must_use]
    pub fn unreadable(mut self, path: &str) -> Self {
        self.files.insert(PathBuf::from(path), FakeFile::Unreadable);
        self
    }
}

impl ForkReader for FakeForks {
    fn read_fork(&self, path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        match (fork, self.files.get(path)) {
            (Fork::Resource, _) => Ok(None),
            (Fork::Data, Some(FakeFile::Bytes(bytes))) => Ok(Some(bytes.clone())),
            (Fork::Data, Some(FakeFile::Unreadable)) => Err(io::Error::other("disk on fire")),
            (Fork::Data, None) => Err(io::Error::new(io::ErrorKind::NotFound, "no such fake file")),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;
    use crate::store::fs::EntryKind::File;

    fn names(tree: &FakeTree, dir: &str) -> Vec<OsString> {
        let listed = tree.list(Path::new(dir)).expect("lists");
        listed.into_iter().map(|listing| listing.name).collect()
    }

    #[test]
    fn listings_come_back_rotated_out_of_finder_order() {
        let tree = FakeTree::new()
            .dir("/sorted", &[("a", File), ("B", File), ("c", File)])
            .dir("/rotated", &[("B", File), ("c", File), ("a", File)])
            .dir("/one", &[("only", File)])
            .dir("/none", &[]);
        assert_eq!(names(&tree, "/sorted"), ["B", "c", "a"]);
        assert_eq!(names(&tree, "/rotated"), ["B", "c", "a"]);
        assert_eq!(names(&tree, "/one"), ["only"]);
        assert!(names(&tree, "/none").is_empty());
        assert!(tree.list(Path::new("/missing")).is_err());
        assert_eq!(
            tree.listed(),
            ["/sorted", "/rotated", "/one", "/none", "/missing"].map(PathBuf::from)
        );
    }
}
