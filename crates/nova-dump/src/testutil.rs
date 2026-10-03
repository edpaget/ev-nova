//! In-memory game data for unit tests.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use nova_data::store::GameData;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};

/// The data directory every [`MemFs`] store opens.
pub const DATA: &str = "/data";
/// The plug-ins directory a [`MemFs`] store opens when it has plug-ins.
pub const PLUGINS: &str = "/plugins";

/// An in-memory file tree: each file's flattened fork is its data fork.
/// Directories are implied by the files under them; `/data` always exists.
#[derive(Default)]
pub struct MemFs {
    files: BTreeMap<PathBuf, Vec<u8>>,
}

impl MemFs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the file at `path` holding `bytes`.
    #[must_use]
    pub fn file(mut self, path: &str, bytes: Vec<u8>) -> Self {
        self.files.insert(PathBuf::from(path), bytes);
        self
    }

    /// Opens `/data` and, if any file is under it, `/plugins`.
    pub fn open(&self) -> GameData {
        let plugins = Path::new(PLUGINS);
        let has_plugins = self.files.keys().any(|path| path.starts_with(plugins));
        GameData::load(self, self, Path::new(DATA), has_plugins.then_some(plugins))
            .expect("in-memory directories list")
    }
}

impl DirLister for MemFs {
    fn list(&self, dir: &Path) -> io::Result<Vec<Listing>> {
        let mut listings: Vec<Listing> = Vec::new();
        for path in self.files.keys() {
            let Ok(rest) = path.strip_prefix(dir) else {
                continue;
            };
            let mut parts = rest.components();
            let name = parts
                .next()
                .expect("a file below dir")
                .as_os_str()
                .to_owned();
            let kind = if parts.next().is_some() {
                EntryKind::Dir
            } else {
                EntryKind::File
            };
            if !listings.iter().any(|l| l.name == name) {
                listings.push(Listing { name, kind });
            }
        }
        Ok(listings)
    }
}

impl ForkReader for MemFs {
    fn read_fork(&self, path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        match fork {
            Fork::Data => self
                .files
                .get(path)
                .cloned()
                .map(Some)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such file")),
            Fork::Resource => Ok(None),
        }
    }
}

/// A flattened fork holding `resources` (type, ID, name, data).
pub fn fork(resources: &[(ResType, i16, Option<&str>, Vec<u8>)]) -> Vec<u8> {
    resources
        .iter()
        .fold(ForkBuilder::new(), |builder, (ty, id, name, data)| {
            builder.resource(*ty, *id, name.map(str::as_bytes), data)
        })
        .build()
        .bytes
}

/// A store with one data file holding `resources`.
pub fn store(resources: &[(ResType, i16, Option<&str>, Vec<u8>)]) -> GameData {
    MemFs::new().file("/data/Nova Data", fork(resources)).open()
}

/// Big-endian `i16` fields written into a zeroed record of `len` bytes.
pub fn record(len: usize, fields: &[(usize, i16)]) -> Vec<u8> {
    let mut bytes = vec![0; len];
    for &(at, value) in fields {
        bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
    }
    bytes
}
