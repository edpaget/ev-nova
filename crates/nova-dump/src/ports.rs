//! The ports the dump's core uses to reach the outside world.

use std::io;
use std::path::Path;

use nova_data::store::{GameData, OpenError};

/// Port: where exported files go.
pub trait Sink {
    /// Writes a new file at `rel`, relative to the output directory,
    /// creating its parent directories. Never overwrites an existing file.
    fn write(&mut self, rel: &Path, bytes: &[u8]) -> io::Result<()>;
}

/// Port: opens the game data.
pub trait DataSource {
    /// Opens the data directory and, if given, the plug-ins directory.
    fn open(&self, data: &Path, plugins: Option<&Path>) -> Result<GameData, OpenError>;
}

/// Port: the output directory.
pub trait OutputRoot {
    /// Where files under the directory are written.
    type Sink: Sink;

    /// Creates the directory, and any missing parents, unless it exists.
    fn create(&self, root: &Path) -> io::Result<()>;

    /// Whether the existing directory has no entries.
    fn is_empty(&self, root: &Path) -> io::Result<bool>;

    /// A sink writing files under the directory.
    fn sink(&self, root: &Path) -> Self::Sink;
}
