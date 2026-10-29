//! An in-memory [`ForkReader`] and the port's contract.
//!
//! [`MemForks`] is the one shared fake of the port: forks held in memory,
//! keyed by path, with every read recorded. [`check_contract`] states what
//! every reader must do, over the small layout [`write_contract_files`]
//! writes to disk and [`MemForks::contract`] builds in memory. It runs
//! against [`MemForks`] here and against [`StdForkReader`] in this crate's
//! `tests/it/std_fs.rs`.
//!
//! [`StdForkReader`]: crate::StdForkReader

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::{Fork, ForkReader};

/// What one fork of a fake file answers.
#[derive(Clone, Debug)]
enum Canned {
    Bytes(Vec<u8>),
    Absent,
    Fails,
}

/// The two forks of one fake file; `None` is the default answer.
#[derive(Clone, Debug, Default)]
struct Forks {
    data: Option<Canned>,
    resource: Option<Canned>,
}

/// In-memory forks, keyed by path.
///
/// - [`file`](Self::file) and [`resource`](Self::resource) give a path a
///   data or a resource fork.
/// - [`unreadable`](Self::unreadable) makes both of a path's forks fail.
/// - [`no_data_fork`](Self::no_data_fork) makes its data fork absent.
/// - A path never added has no file: its data fork is an error of kind
///   [`io::ErrorKind::NotFound`], and its resource fork is absent, as
///   [`StdForkReader`](crate::StdForkReader) reports a missing file.
/// - A path given only a data fork has no resource fork, and one given only
///   a resource fork has an empty data fork, as a classic resource file on
///   disk does.
///
/// Every read is recorded, in order, for [`reads`](Self::reads).
#[derive(Debug, Default)]
pub struct MemForks {
    files: HashMap<PathBuf, Forks>,
    reads: Mutex<Vec<(PathBuf, Fork)>>,
}

impl MemForks {
    /// No files at all.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Gives `path` the data fork `data`.
    #[must_use]
    pub fn file(mut self, path: impl AsRef<Path>, data: impl Into<Vec<u8>>) -> Self {
        self.entry(path).data = Some(Canned::Bytes(data.into()));
        self
    }

    /// Gives `path` the resource fork `bytes`.
    #[must_use]
    pub fn resource(mut self, path: impl AsRef<Path>, bytes: impl Into<Vec<u8>>) -> Self {
        self.entry(path).resource = Some(Canned::Bytes(bytes.into()));
        self
    }

    /// Makes reading either of `path`'s forks fail with an error of kind
    /// [`io::ErrorKind::Other`] whose message is [`UNREADABLE`]. A later [`file`](Self::file),
    /// [`resource`](Self::resource) or [`no_data_fork`](Self::no_data_fork)
    /// replaces the failure on that fork.
    #[must_use]
    pub fn unreadable(mut self, path: impl AsRef<Path>) -> Self {
        let forks = self.entry(path);
        forks.data = Some(Canned::Fails);
        forks.resource = Some(Canned::Fails);
        self
    }

    /// Makes `path`'s data fork absent (`Ok(None)`).
    ///
    /// [`StdForkReader`](crate::StdForkReader) never reports this: a file
    /// always has a data fork, if an empty one. It exists for callers'
    /// `None` arms.
    #[must_use]
    pub fn no_data_fork(mut self, path: impl AsRef<Path>) -> Self {
        self.entry(path).data = Some(Canned::Absent);
        self
    }

    /// Every `(path, fork)` read so far, in order.
    ///
    /// # Panics
    ///
    /// If an earlier read panicked while recording.
    #[must_use]
    pub fn reads(&self) -> Vec<(PathBuf, Fork)> {
        self.reads.lock().expect("reads").clone()
    }

    /// The contract's layout under `root`, in memory: what
    /// [`write_contract_files`] writes to disk.
    #[must_use]
    pub fn contract(root: &Path) -> Self {
        Self::new()
            .file(root.join(DATA), CONTRACT_BYTES)
            .file(root.join(EMPTY), Vec::new())
    }

    fn entry(&mut self, path: impl AsRef<Path>) -> &mut Forks {
        self.files.entry(path.as_ref().to_path_buf()).or_default()
    }
}

impl ForkReader for MemForks {
    fn read_fork(&self, path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        self.reads
            .lock()
            .expect("reads")
            .push((path.to_path_buf(), fork));
        let Some(forks) = self.files.get(path) else {
            return match fork {
                Fork::Data => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("no fake file at {}", path.display()),
                )),
                Fork::Resource => Ok(None),
            };
        };
        let canned = match fork {
            Fork::Data => forks.data.as_ref(),
            Fork::Resource => forks.resource.as_ref(),
        };
        match (canned, fork) {
            (Some(Canned::Bytes(bytes)), _) => Ok(Some(bytes.clone())),
            (Some(Canned::Absent), _) | (None, Fork::Resource) => Ok(None),
            (Some(Canned::Fails), _) => Err(io::Error::other(UNREADABLE)),
            (None, Fork::Data) => Ok(Some(Vec::new())),
        }
    }
}

/// The message of the error an [`unreadable`](MemForks::unreadable) fork
/// fails with.
pub const UNREADABLE: &str = "the fake fork is unreadable";

/// The data fork of the contract's [`DATA`] file.
pub const CONTRACT_BYTES: &[u8] = b"contract fork \x00\x01\xFF";

/// The contract's file holding [`CONTRACT_BYTES`].
pub const DATA: &str = "Data";

/// The contract's empty file.
pub const EMPTY: &str = "Empty";

/// A name the contract never creates.
pub const MISSING: &str = "Missing";

/// Writes the contract's layout under `root`: [`DATA`] holding
/// [`CONTRACT_BYTES`] and [`EMPTY`] holding nothing. It writes nothing
/// outside `root`.
///
/// # Errors
///
/// When a file can't be written.
pub fn write_contract_files(root: &Path) -> io::Result<()> {
    std::fs::write(root.join(DATA), CONTRACT_BYTES)?;
    std::fs::write(root.join(EMPTY), [])
}

/// Checks the port's contract against `reader`, over the layout
/// [`write_contract_files`] (or [`MemForks::contract`]) put under `root`:
///
/// 1. [`DATA`]'s data fork is [`CONTRACT_BYTES`], and the same on a second
///    read.
/// 2. A plain file such as [`DATA`] has no resource fork.
/// 3. [`EMPTY`]'s data fork is empty, not absent: an empty file is an empty
///    fork.
/// 4. [`MISSING`]'s data fork is an error of kind
///    [`io::ErrorKind::NotFound`].
/// 5. [`MISSING`]'s resource fork is absent, not an error.
///
/// It doesn't cover a real resource fork, which can't be written portably:
/// only macOS has them. `StdForkReader`'s own macOS tests read one.
///
/// # Errors
///
/// A message naming the first clause `reader` breaks.
pub fn check_contract(reader: &impl ForkReader, root: &Path) -> Result<(), String> {
    let data = root.join(DATA);
    let first = reader.read_fork(&data, Fork::Data);
    if !matches!(&first, Ok(Some(bytes)) if bytes == CONTRACT_BYTES) {
        return Err(format!(
            "{DATA}'s data fork is {first:?}, not the contract bytes"
        ));
    }
    let second = reader.read_fork(&data, Fork::Data);
    if !matches!(&second, Ok(Some(bytes)) if bytes == CONTRACT_BYTES) {
        return Err(format!(
            "{DATA}'s data fork is {second:?} on a second read, not the contract bytes"
        ));
    }
    let resource = reader.read_fork(&data, Fork::Resource);
    if !matches!(resource, Ok(None)) {
        return Err(format!(
            "a plain file's resource fork is {resource:?}, not absent"
        ));
    }
    let empty = reader.read_fork(&root.join(EMPTY), Fork::Data);
    if !matches!(&empty, Ok(Some(bytes)) if bytes.is_empty()) {
        return Err(format!(
            "an empty file's data fork is {empty:?}, not an empty fork"
        ));
    }
    let missing = root.join(MISSING);
    match reader.read_fork(&missing, Fork::Data) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        other => {
            return Err(format!(
                "a missing file's data fork is {other:?}, not an error of kind NotFound"
            ));
        }
    }
    let resource = reader.read_fork(&missing, Fork::Resource);
    if !matches!(resource, Ok(None)) {
        return Err(format!(
            "a missing file's resource fork is {resource:?}, not absent"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new("contract").join("root")
    }

    #[test]
    fn mem_forks_meet_the_contract() {
        assert_eq!(
            check_contract(&MemForks::contract(&root()), &root()),
            Ok(())
        );
    }

    /// [`MemForks::contract`] with one answer changed.
    struct Broken<F> {
        inner: MemForks,
        change: F,
    }

    impl<F> ForkReader for Broken<F>
    where
        F: Fn(&Path, Fork, io::Result<Option<Vec<u8>>>) -> io::Result<Option<Vec<u8>>>,
    {
        fn read_fork(&self, path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            (self.change)(path, fork, self.inner.read_fork(path, fork))
        }
    }

    /// The error `check_contract` gives when `change` rewrites every answer.
    fn broken(
        change: impl Fn(&Path, Fork, io::Result<Option<Vec<u8>>>) -> io::Result<Option<Vec<u8>>>,
    ) -> String {
        let reader = Broken {
            inner: MemForks::contract(&root()),
            change,
        };
        check_contract(&reader, &root()).expect_err("the broken reader fails the contract")
    }

    fn named(path: &Path, name: &str) -> bool {
        path.file_name().is_some_and(|file| file == name)
    }

    #[test]
    fn rejects_wrong_data_fork_bytes() {
        let error = broken(|path, fork, answer| match fork {
            Fork::Data if named(path, DATA) => Ok(Some(b"other".to_vec())),
            _ => answer,
        });
        assert!(error.contains("contract bytes"), "{error}");
    }

    #[test]
    fn rejects_a_data_fork_that_changes_between_reads() {
        let reads = Mutex::new(0);
        let error = broken(move |path, fork, answer| match fork {
            Fork::Data if named(path, DATA) => {
                let mut count = reads.lock().expect("count");
                *count += 1;
                if *count > 1 {
                    Ok(Some(Vec::new()))
                } else {
                    answer
                }
            }
            _ => answer,
        });
        assert!(error.contains("second read"), "{error}");
    }

    #[test]
    fn rejects_a_plain_file_with_a_resource_fork() {
        let error = broken(|path, fork, answer| match fork {
            Fork::Resource if named(path, DATA) => Ok(Some(b"rsrc".to_vec())),
            _ => answer,
        });
        assert!(error.contains("plain file"), "{error}");
    }

    #[test]
    fn rejects_an_empty_file_read_as_absent() {
        let error = broken(|path, fork, answer| match fork {
            Fork::Data if named(path, EMPTY) => Ok(None),
            _ => answer,
        });
        assert!(error.contains("empty"), "{error}");
    }

    #[test]
    fn rejects_a_missing_file_with_a_data_fork() {
        let error = broken(|path, fork, answer| match fork {
            Fork::Data if named(path, MISSING) => Ok(Some(Vec::new())),
            _ => answer,
        });
        assert!(error.contains("NotFound"), "{error}");
    }

    #[test]
    fn rejects_a_missing_file_failing_with_another_kind() {
        let error = broken(|path, fork, answer| match fork {
            Fork::Data if named(path, MISSING) => Err(io::Error::other("disk on fire")),
            _ => answer,
        });
        assert!(error.contains("NotFound"), "{error}");
    }

    #[test]
    fn rejects_a_missing_file_whose_resource_fork_fails() {
        let error = broken(|path, fork, answer| match fork {
            Fork::Resource if named(path, MISSING) => {
                Err(io::Error::new(io::ErrorKind::NotFound, "gone"))
            }
            _ => answer,
        });
        assert!(error.contains("missing file's resource fork"), "{error}");
    }

    #[test]
    fn rejects_a_missing_file_with_a_resource_fork() {
        let error = broken(|path, fork, answer| match fork {
            Fork::Resource if named(path, MISSING) => Ok(Some(Vec::new())),
            _ => answer,
        });
        assert!(error.contains("missing file's resource fork"), "{error}");
    }

    #[test]
    fn a_path_never_added_is_a_missing_file() {
        let forks = MemForks::new();
        let error = forks
            .read_fork(Path::new("nowhere"), Fork::Data)
            .expect_err("no file");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert_eq!(
            forks.read_fork(Path::new("nowhere"), Fork::Resource).ok(),
            Some(None)
        );
    }

    #[test]
    fn a_file_holds_its_data_fork_and_no_resource_fork() {
        let forks = MemForks::new().file("a", b"bytes".to_vec());
        assert_eq!(
            forks.read_fork(Path::new("a"), Fork::Data).ok(),
            Some(Some(b"bytes".to_vec()))
        );
        assert_eq!(
            forks.read_fork(Path::new("a"), Fork::Resource).ok(),
            Some(None)
        );
    }

    #[test]
    fn a_file_with_only_a_resource_fork_has_an_empty_data_fork() {
        let forks = MemForks::new().resource("r", b"rsrc".to_vec());
        assert_eq!(
            forks.read_fork(Path::new("r"), Fork::Resource).ok(),
            Some(Some(b"rsrc".to_vec()))
        );
        assert_eq!(
            forks.read_fork(Path::new("r"), Fork::Data).ok(),
            Some(Some(Vec::new()))
        );
    }

    #[test]
    fn an_unreadable_file_fails_on_both_forks() {
        let forks = MemForks::new().file("u", b"x".to_vec()).unreadable("u");
        for fork in [Fork::Data, Fork::Resource] {
            let error = forks.read_fork(Path::new("u"), fork).expect_err("fails");
            assert_eq!(error.kind(), io::ErrorKind::Other, "{fork:?}");
            assert_eq!(error.to_string(), UNREADABLE, "{fork:?}");
        }
    }

    #[test]
    fn a_later_fork_replaces_the_failure_on_that_fork_only() {
        let forks = MemForks::new().unreadable("u").no_data_fork("u");
        assert_eq!(forks.read_fork(Path::new("u"), Fork::Data).ok(), Some(None));
        assert!(forks.read_fork(Path::new("u"), Fork::Resource).is_err());
    }

    #[test]
    fn no_data_fork_is_absent_not_an_error() {
        let forks = MemForks::new().no_data_fork("n");
        assert_eq!(forks.read_fork(Path::new("n"), Fork::Data).ok(), Some(None));
        assert_eq!(
            forks.read_fork(Path::new("n"), Fork::Resource).ok(),
            Some(None)
        );
    }

    #[test]
    fn reads_records_every_read_in_order() {
        let forks = MemForks::new().file("a", Vec::new());
        assert!(forks.reads().is_empty());
        let _ = forks.read_fork(Path::new("a"), Fork::Data);
        let _ = forks.read_fork(Path::new("b"), Fork::Resource);
        let _ = forks.read_fork(Path::new("a"), Fork::Resource);
        assert_eq!(
            forks.reads(),
            vec![
                (PathBuf::from("a"), Fork::Data),
                (PathBuf::from("b"), Fork::Resource),
                (PathBuf::from("a"), Fork::Resource),
            ]
        );
    }

    #[test]
    fn the_contract_layout_holds_data_and_empty_under_the_root() {
        let forks = MemForks::contract(&root());
        assert_eq!(
            forks.read_fork(&root().join(DATA), Fork::Data).ok(),
            Some(Some(CONTRACT_BYTES.to_vec()))
        );
        assert_eq!(
            forks.read_fork(&root().join(EMPTY), Fork::Data).ok(),
            Some(Some(Vec::new()))
        );
        assert!(forks.read_fork(Path::new(DATA), Fork::Data).is_err());
    }
}
