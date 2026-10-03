//! The game data store: every data file and plug-in, layered into one view.

pub mod fs;
pub mod order;

/// Which folder a file was loaded from.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    /// The data directory (`Nova Files`).
    Data,
    /// The plug-ins directory or one of its sub-folders.
    PlugIn,
}
