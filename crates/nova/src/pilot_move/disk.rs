//! [`DiskFiles`], the [`MoveFiles`] port over the real filesystem.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::path::Path;

use super::MoveFiles;

/// The real filesystem, through `std::fs`.
///
/// A move is a rename. Where `XDG_DATA_HOME` is on another filesystem
/// than the old directory, a rename can't cross over, so a file is copied
/// and the original removed instead (`copy_then_remove`); anything
/// else (a directory, say) stays where it was, with the rename's error.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiskFiles;

impl MoveFiles for DiskFiles {
    fn entries(&self, dir: &Path) -> io::Result<Option<Vec<OsString>>> {
        let listing = match fs::read_dir(dir) {
            Ok(listing) => listing,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        listing
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<io::Result<_>>()
            .map(Some)
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        // Not following links: a broken one still takes the name.
        match fs::symlink_metadata(path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn create_dir_all(&mut self, dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir)
    }

    fn move_entry(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        move_with(|from, to| fs::rename(from, to), from, to)
    }

    fn remove_empty_dir(&mut self, dir: &Path) -> io::Result<()> {
        fs::remove_dir(dir)
    }
}

/// Moves `from` to `to` with `rename`, or, when it can't cross to
/// another filesystem and `from` is a file, with [`copy_then_remove`].
/// The rename is a parameter so the tests can fail it as only a second
/// filesystem would.
fn move_with(
    rename: impl FnOnce(&Path, &Path) -> io::Result<()>,
    from: &Path,
    to: &Path,
) -> io::Result<()> {
    match rename(from, to) {
        Err(error)
            if error.kind() == io::ErrorKind::CrossesDevices
                && fs::symlink_metadata(from).is_ok_and(|meta| meta.is_file()) =>
        {
            copy_then_remove(from, to)
        }
        moved => moved,
    }
}

/// Moves the file `from` to `to` by copying it and removing `from`, for
/// a move to another filesystem. It never replaces a file at `to`, and
/// when it fails it leaves the file at `from` only: a part-written copy
/// is removed, and so is a whole one when `from` can't be.
fn copy_then_remove(from: &Path, to: &Path) -> io::Result<()> {
    let mut source = File::open(from)?;
    if !source.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "only a file can be copied to another filesystem",
        ));
    }
    let mut target = File::options().write(true).create_new(true).open(to)?;
    let copied = io::copy(&mut source, &mut target)
        .and_then(|_| target.sync_all())
        .and_then(|()| {
            drop(source);
            fs::remove_file(from)
        });
    if copied.is_err() {
        drop(target);
        let _ = fs::remove_file(to);
    }
    copied
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn sorted(names: Option<Vec<OsString>>) -> Option<Vec<OsString>> {
        names.map(|mut names| {
            names.sort();
            names
        })
    }

    #[test]
    fn a_missing_directory_has_no_entries_and_a_real_one_lists_them() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let files = DiskFiles;
        assert_eq!(
            files.entries(&tmp.path().join("missing")).expect("lists"),
            None
        );
        assert_eq!(files.entries(tmp.path()).expect("lists"), Some(Vec::new()));
        fs::write(tmp.path().join("Last Pilot"), "Ada").expect("writes");
        fs::write(tmp.path().join("Ada.json"), "{}").expect("writes");
        fs::create_dir(tmp.path().join("odd")).expect("a directory");
        assert_eq!(
            sorted(files.entries(tmp.path()).expect("lists")),
            Some(vec![
                OsString::from("Ada.json"),
                OsString::from("Last Pilot"),
                OsString::from("odd"),
            ])
        );
    }

    #[test]
    fn listing_a_file_as_a_directory_is_an_error() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let file = tmp.path().join("Pilots");
        fs::write(&file, "not a directory").expect("writes");
        assert!(DiskFiles.entries(&file).is_err());
    }

    #[test]
    fn files_and_directories_exist_and_missing_paths_do_not() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let file = tmp.path().join("Ada.json");
        fs::write(&file, "{}").expect("writes");
        let files = DiskFiles;
        assert!(files.exists(&file).expect("tells"));
        assert!(files.exists(tmp.path()).expect("tells"));
        assert!(!files.exists(&tmp.path().join("Bob.json")).expect("tells"));
    }

    #[cfg(unix)]
    #[test]
    fn a_broken_link_exists() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let link = tmp.path().join("Ada.json");
        std::os::unix::fs::symlink(tmp.path().join("nowhere"), &link).expect("links");
        assert!(DiskFiles.exists(&link).expect("tells"));
    }

    #[test]
    fn an_entry_moves_into_a_made_directory() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let old = tmp.path().join("old");
        let new = tmp.path().join("share").join("nova").join("Pilots");
        fs::create_dir(&old).expect("a directory");
        fs::write(old.join("Ada.json"), "ada").expect("writes");
        let mut files = DiskFiles;
        files.create_dir_all(&new).expect("makes");
        files.create_dir_all(&new).expect("makes what is there");
        files
            .move_entry(&old.join("Ada.json"), &new.join("Ada.json"))
            .expect("moves");
        assert_eq!(
            fs::read_to_string(new.join("Ada.json")).expect("reads"),
            "ada"
        );
        assert!(!old.join("Ada.json").exists());
    }

    #[test]
    fn a_failed_move_is_an_error() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let from = tmp.path().join("missing.json");
        let to = tmp.path().join("there.json");
        assert!(DiskFiles.move_entry(&from, &to).is_err());
        assert!(!to.exists());
    }

    #[test]
    fn only_an_empty_directory_is_removed() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let dir = tmp.path().join("Pilots");
        fs::create_dir(&dir).expect("a directory");
        fs::write(dir.join("Ada.json"), "ada").expect("writes");
        let mut files = DiskFiles;
        assert!(files.remove_empty_dir(&dir).is_err());
        assert_eq!(
            fs::read_to_string(dir.join("Ada.json")).expect("kept"),
            "ada"
        );
        fs::remove_file(dir.join("Ada.json")).expect("removes");
        files.remove_empty_dir(&dir).expect("removes");
        assert!(!dir.exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_path_that_cannot_be_looked_at_is_an_error_not_missing() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let file = tmp.path().join("Ada.json");
        fs::write(&file, "{}").expect("writes");
        // A file can hold nothing: on Unix, "not a directory", not "not
        // found".
        assert!(DiskFiles.exists(&file.join("inside")).is_err());
    }

    /// A rename that fails as a move to another filesystem does.
    fn across_devices(_: &Path, _: &Path) -> io::Result<()> {
        Err(io::ErrorKind::CrossesDevices.into())
    }

    #[test]
    fn a_file_a_rename_cannot_move_across_filesystems_is_copied() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let from = tmp.path().join("Ada.json");
        let to = tmp.path().join("moved.json");
        fs::write(&from, "ada").expect("writes");
        move_with(across_devices, &from, &to).expect("moves");
        assert_eq!(fs::read_to_string(&to).expect("reads"), "ada");
        assert!(!from.exists());
    }

    #[test]
    fn a_directory_a_rename_cannot_move_across_filesystems_stays() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let from = tmp.path().join("odd");
        fs::create_dir(&from).expect("a directory");
        let to = tmp.path().join("moved");
        let error = move_with(across_devices, &from, &to).expect_err("stays");
        assert_eq!(error.kind(), io::ErrorKind::CrossesDevices);
        assert!(from.is_dir());
        assert!(!to.exists());
    }

    #[test]
    fn any_other_failed_rename_is_its_error_and_copies_nothing() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let from = tmp.path().join("Ada.json");
        let to = tmp.path().join("moved.json");
        fs::write(&from, "ada").expect("writes");
        let denied = |_: &Path, _: &Path| Err(io::ErrorKind::PermissionDenied.into());
        let error = move_with(denied, &from, &to).expect_err("fails");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(from.exists());
        assert!(!to.exists());
    }

    #[test]
    fn copying_then_removing_moves_a_file_byte_for_byte() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let from = tmp.path().join("Ada.json");
        let to = tmp.path().join("moved.json");
        let bytes: Vec<u8> = (0..=255).collect();
        fs::write(&from, &bytes).expect("writes");
        copy_then_remove(&from, &to).expect("moves");
        assert_eq!(fs::read(&to).expect("reads"), bytes);
        assert!(!from.exists());
    }

    #[test]
    fn copying_then_removing_never_replaces_a_file() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let from = tmp.path().join("Ada.json");
        let to = tmp.path().join("taken.json");
        fs::write(&from, "old ada").expect("writes");
        fs::write(&to, "new ada").expect("writes");
        assert!(copy_then_remove(&from, &to).is_err());
        assert_eq!(fs::read_to_string(&from).expect("kept"), "old ada");
        assert_eq!(fs::read_to_string(&to).expect("kept"), "new ada");
    }

    #[test]
    fn copying_then_removing_a_directory_is_an_error() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let from = tmp.path().join("odd");
        fs::create_dir(&from).expect("a directory");
        let to = tmp.path().join("moved");
        assert!(copy_then_remove(&from, &to).is_err());
        assert!(from.is_dir());
        assert!(!to.exists());
    }
}
