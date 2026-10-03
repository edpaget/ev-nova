//! The ports the dump's core uses to reach the outside world.

use std::io;
use std::path::Path;

/// Port: where exported files go.
pub trait Sink {
    /// Writes a new file at `rel`, relative to the output directory,
    /// creating its parent directories. Never overwrites an existing file.
    fn write(&mut self, rel: &Path, bytes: &[u8]) -> io::Result<()>;
}
