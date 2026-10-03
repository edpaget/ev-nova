//! The game data store: every data file and plug-in, layered into one view.

use std::io;
use std::path::PathBuf;

use nova_rsrc::LoadError;

use self::order::IgnoreReason;

#[cfg(test)]
mod fake;
pub mod fs;
pub mod order;
#[allow(dead_code)] // Wired into the store in the next commit.
mod walk;

/// Why the store could not be opened at all.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// The data directory could not be listed.
    #[error("listing the data directory {}: {source}", path.display())]
    DataDir {
        /// The data directory.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// The plug-ins directory could not be listed.
    #[error("listing the plug-ins directory {}: {source}", path.display())]
    PlugInsDir {
        /// The plug-ins directory.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
}

/// A file or plug-ins sub-folder that could not be loaded; the store opened
/// without it.
#[derive(Debug)]
pub struct FailedFile {
    /// The file or folder.
    pub path: PathBuf,
    /// Which folder it was found in.
    pub origin: Origin,
    /// Why it failed. A folder that could not be listed is a
    /// [`LoadError::Io`].
    pub error: LoadError,
}

/// A directory entry that was skipped on purpose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IgnoredEntry {
    /// The entry.
    pub path: PathBuf,
    /// Why it was skipped.
    pub reason: IgnoreReason,
}

/// Which folder a file was loaded from.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    /// The data directory (`Nova Files`).
    Data,
    /// The plug-ins directory or one of its sub-folders.
    PlugIn,
}
