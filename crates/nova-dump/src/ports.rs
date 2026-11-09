//! The ports the dump's core uses to reach the outside world.

use std::io;
use std::path::Path;

use nova_data::new_game::{IdTable, ListError};
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

/// Port: a new game's data folder, whose ID table `--pin-ids` writes.
///
/// Unlike a [`Sink`], it replaces a file: the table is the folder's own,
/// and pinning rewrites it. The merge and the text are
/// `nova_data::new_game::pins`', so an adapter only moves bytes.
pub trait GameFolder {
    /// Loads the new game in the folder `dir`, for its records' IDs.
    ///
    /// # Errors
    ///
    /// [`ListError`] when `dir` can't be listed.
    fn load(&self, dir: &Path) -> Result<IdTable, ListError>;

    /// The bytes of the folder's ID table, or `None` when it has none.
    ///
    /// # Errors
    ///
    /// Any error reading a table that is there.
    fn table(&self, dir: &Path) -> io::Result<Option<Vec<u8>>>;

    /// Writes `bytes` as the folder's ID table, replacing any it has.
    ///
    /// # Errors
    ///
    /// Any error writing it.
    fn replace_table(&self, dir: &Path, bytes: &[u8]) -> io::Result<()>;
}
