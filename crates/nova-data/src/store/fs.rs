//! Listing directories: the [`DirLister`] port.

use std::ffi::OsString;
use std::io;
use std::path::Path;

/// What a directory entry is, without following symbolic links.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// A regular file.
    File,
    /// A directory.
    Dir,
    /// A symbolic link (to anything); never followed.
    Symlink,
    /// Anything else: a socket, FIFO, device and so on.
    Other,
}

/// One entry of a directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing {
    /// The entry's file name.
    pub name: OsString,
    /// What the entry is.
    pub kind: EntryKind,
}

/// Port: lists the entries of a directory, in any order.
pub trait DirLister {
    /// The entries of `dir`, without `.` and `..`.
    fn list(&self, dir: &Path) -> io::Result<Vec<Listing>>;
}
