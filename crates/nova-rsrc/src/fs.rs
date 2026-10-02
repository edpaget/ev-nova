//! Reading forks from disk: the [`ForkReader`] port and its std adapter.

use std::io;
use std::path::Path;

/// Which fork of a file to read.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Fork {
    /// The ordinary file contents (a flattened `.ndat` lives here).
    Data,
    /// The classic Mac OS resource fork.
    Resource,
}

/// Port: fetches the raw bytes of one fork of a file.
pub trait ForkReader {
    /// Reads `fork` of the file at `path`; `Ok(None)` when that fork does
    /// not exist.
    fn read_fork(&self, path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>>;
}

/// Adapter: reads forks through `std::fs`.
///
/// The data fork is the file itself; a missing file is an error. The
/// resource fork is read through macOS's `..namedfork/rsrc` path; on other
/// platforms it is always absent.
#[derive(Copy, Clone, Debug, Default)]
pub struct StdForkReader;

impl ForkReader for StdForkReader {
    fn read_fork(&self, path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        match fork {
            Fork::Data => std::fs::read(path).map(Some),
            Fork::Resource => read_resource_fork(path),
        }
    }
}

#[cfg(target_os = "macos")]
fn read_resource_fork(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match std::fs::read(path.join("..namedfork/rsrc")) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(not(target_os = "macos"))]
#[allow(clippy::unnecessary_wraps)]
fn read_resource_fork(_path: &Path) -> io::Result<Option<Vec<u8>>> {
    Ok(None)
}
