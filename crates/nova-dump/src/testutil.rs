//! In-memory game data for unit tests.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::store::{GameData, OpenError};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};

use crate::ports::{DataSource, Sink};

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

impl DataSource for MemFs {
    fn open(&self, data: &Path, plugins: Option<&Path>) -> Result<GameData, OpenError> {
        GameData::load(self, self, data, plugins)
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

/// An in-memory output: every file written, by relative path. Fails the
/// `fail_at`-th write (counting from 1) when set; writing a path twice is a
/// test failure.
#[derive(Debug, Default)]
pub struct MemSink {
    pub files: BTreeMap<PathBuf, Vec<u8>>,
    pub writes: usize,
    pub fail_at: Option<usize>,
}

impl MemSink {
    /// The file at `path`.
    pub fn get(&self, path: &str) -> &[u8] {
        self.files
            .get(Path::new(path))
            .unwrap_or_else(|| panic!("{path} not written: {:?}", self.paths()))
    }

    /// Every path written, sorted.
    pub fn paths(&self) -> Vec<String> {
        self.files
            .keys()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }
}

impl Sink for MemSink {
    fn write(&mut self, rel: &Path, bytes: &[u8]) -> io::Result<()> {
        self.writes += 1;
        if self.fail_at == Some(self.writes) {
            return Err(io::Error::other("disk full"));
        }
        let old = self.files.insert(rel.to_path_buf(), bytes.to_vec());
        assert!(old.is_none(), "{} written twice", rel.display());
        Ok(())
    }
}

/// A sink shared with the test that handed it out.
impl Sink for Rc<RefCell<MemSink>> {
    fn write(&mut self, rel: &Path, bytes: &[u8]) -> io::Result<()> {
        self.borrow_mut().write(rel, bytes)
    }
}
