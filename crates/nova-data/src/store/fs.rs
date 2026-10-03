//! Listing directories: the [`DirLister`] port and its std adapter.

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

/// Adapter: lists directories through `std::fs`, never following symbolic
/// links (it reports what `DirEntry::file_type` says, which does not follow
/// them).
#[derive(Copy, Clone, Debug, Default)]
pub struct StdDirLister;

impl DirLister for StdDirLister {
    fn list(&self, dir: &Path) -> io::Result<Vec<Listing>> {
        std::fs::read_dir(dir)?
            .map(|entry| {
                let entry = entry?;
                Ok(Listing {
                    kind: kind_of(entry.file_type()?),
                    name: entry.file_name(),
                })
            })
            .collect()
    }
}

fn kind_of(file_type: std::fs::FileType) -> EntryKind {
    if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_dir() {
        EntryKind::Dir
    } else if file_type.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listed(dir: &Path) -> Vec<(String, EntryKind)> {
        let mut entries: Vec<_> = StdDirLister
            .list(dir)
            .expect("lists")
            .into_iter()
            .map(|l| (l.name.to_string_lossy().into_owned(), l.kind))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }

    #[test]
    fn lists_files_and_folders_with_their_kinds() {
        let tmp = tempfile::tempdir().expect("temp dir");
        std::fs::write(tmp.path().join("plug.npif"), b"x").expect("write");
        std::fs::create_dir(tmp.path().join("Folder")).expect("mkdir");
        std::fs::write(tmp.path().join("Folder/inner"), b"y").expect("write");
        assert_eq!(
            listed(tmp.path()),
            [
                ("Folder".to_owned(), EntryKind::Dir),
                ("plug.npif".to_owned(), EntryKind::File),
            ]
        );
    }

    #[test]
    fn an_empty_folder_lists_nothing() {
        let tmp = tempfile::tempdir().expect("temp dir");
        assert!(listed(tmp.path()).is_empty());
    }

    #[test]
    fn a_missing_folder_is_an_error() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let err = StdDirLister
            .list(&tmp.path().join("missing"))
            .expect_err("missing");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_are_reported_not_followed() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir(tmp.path().join("real")).expect("mkdir");
        std::fs::write(tmp.path().join("file"), b"x").expect("write");
        symlink(tmp.path().join("real"), tmp.path().join("to dir")).expect("link");
        symlink(tmp.path().join("file"), tmp.path().join("to file")).expect("link");
        assert_eq!(
            listed(tmp.path()),
            [
                ("file".to_owned(), EntryKind::File),
                ("real".to_owned(), EntryKind::Dir),
                ("to dir".to_owned(), EntryKind::Symlink),
                ("to file".to_owned(), EntryKind::Symlink),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn sockets_are_other() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let _socket =
            std::os::unix::net::UnixListener::bind(tmp.path().join("sock")).expect("bind");
        assert_eq!(listed(tmp.path()), [("sock".to_owned(), EntryKind::Other)]);
    }
}
