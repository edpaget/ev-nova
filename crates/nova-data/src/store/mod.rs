//! The game data store: every data file and plug-in, layered into one view.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use nova_rsrc::{ForkReader, LoadError, ResType, Resource, ResourceFile, StdForkReader};

use self::fs::{DirLister, StdDirLister};
use self::order::IgnoreReason;
use crate::error::{DecodeError, DecodeWarning};
use crate::registry::{AnyDecoded, AnyRecord, Registered, decode_any};

#[cfg(test)]
mod fake;
pub mod fs;
pub mod order;
mod sprite;
#[cfg(test)]
mod tests;
mod walk;

pub use self::sprite::{RLED, ShipSprite, SpriteError};

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

/// One loaded resource file and where it came from.
#[derive(Debug)]
pub struct SourceFile {
    /// The file's path.
    pub path: PathBuf,
    /// Which folder it was found in.
    pub origin: Origin,
    /// Its resources.
    pub resources: ResourceFile,
}

/// A resource's raw bytes and the file that won it.
#[derive(Clone, Copy, Debug)]
pub struct StoreResource<'a> {
    /// The resource, from the winning file.
    pub resource: Resource<'a>,
    /// The file it came from.
    pub source: &'a SourceFile,
}

/// Which files define one resource.
#[derive(Clone, Debug)]
pub struct Provenance<'a> {
    /// The last file to define it, whose copy is used.
    pub winner: &'a SourceFile,
    /// Earlier files that defined it, in load order.
    pub shadowed: Vec<&'a SourceFile>,
}

/// A decoded record from the store.
#[derive(Debug)]
pub struct StoreEntry<'a, T> {
    /// The resource ID.
    pub id: i16,
    /// The resource name, if it has one.
    pub name: Option<&'a str>,
    /// The record.
    pub record: &'a T,
    /// A warning about the record, if decoding raised one.
    pub warning: Option<&'a DecodeWarning>,
    /// The file it came from.
    pub source: &'a SourceFile,
}

// Manual impls: derives would needlessly require `T: Clone`.
impl<T> Clone for StoreEntry<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for StoreEntry<'_, T> {}

/// One (type, ID): the files defining it and its cached decode.
#[derive(Debug)]
struct Slot {
    /// Indexes into `GameData::files` of every file defining it, in load
    /// order; the last one wins.
    definers: Vec<usize>,
    /// The winner's decode, once asked for; `None` for unregistered types.
    decoded: OnceLock<Option<AnyDecoded>>,
}

/// Every ID of one type, ascending, with a slot for each.
#[derive(Debug, Default)]
struct TypeIndex {
    ids: Vec<i16>,
    slots: Vec<Slot>,
}

/// The game's data files and plug-ins, layered into one read-only store.
#[derive(Debug)]
pub struct GameData {
    files: Vec<SourceFile>,
    failed: Vec<FailedFile>,
    ignored: Vec<IgnoredEntry>,
    index: BTreeMap<ResType, TypeIndex>,
}

impl GameData {
    /// Opens `data_dir` (`Nova Files`) and, if given, the `plugins` tree
    /// from disk.
    pub fn open(data_dir: &Path, plugins: Option<&Path>) -> Result<Self, OpenError> {
        Self::load(&StdDirLister, &StdForkReader, data_dir, plugins)
    }

    /// Opens `data_dir` and, if given, the `plugins` tree through the given
    /// ports.
    pub fn load(
        dirs: &impl DirLister,
        forks: &impl ForkReader,
        data_dir: &Path,
        plugins: Option<&Path>,
    ) -> Result<Self, OpenError> {
        let walk = walk::load_order(dirs, data_dir, plugins)?;
        let mut failed = walk.failed;
        let mut files = Vec::new();
        for (path, origin) in walk.candidates {
            match ResourceFile::load(forks, &path) {
                Ok(resources) => files.push(SourceFile {
                    path,
                    origin,
                    resources,
                }),
                Err(error) => failed.push(FailedFile {
                    path,
                    origin,
                    error,
                }),
            }
        }
        let index = build_index(&files);
        Ok(Self {
            files,
            failed,
            ignored: walk.ignored,
            index,
        })
    }

    /// Every loaded file, in load order.
    #[must_use]
    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    /// Files and plug-ins sub-folders that could not be loaded.
    #[must_use]
    pub fn failed(&self) -> &[FailedFile] {
        &self.failed
    }

    /// Directory entries skipped on purpose, in walk order.
    #[must_use]
    pub fn ignored(&self) -> &[IgnoredEntry] {
        &self.ignored
    }

    /// Every resource type present, in type-code byte order.
    pub fn types(&self) -> impl Iterator<Item = ResType> + '_ {
        self.index.keys().copied()
    }

    /// Every ID of type `ty`, ascending; empty if the type is absent.
    #[must_use]
    pub fn ids(&self, ty: ResType) -> &[i16] {
        self.index.get(&ty).map_or(&[], |t| t.ids.as_slice())
    }

    /// The winning copy of resource (`ty`, `id`).
    #[must_use]
    pub fn resource(&self, ty: ResType, id: i16) -> Option<StoreResource<'_>> {
        let source = self.winner(self.slot(ty, id)?);
        let resource = source.resources.get(ty, id)?;
        Some(StoreResource { resource, source })
    }

    /// Which files define resource (`ty`, `id`).
    #[must_use]
    pub fn provenance(&self, ty: ResType, id: i16) -> Option<Provenance<'_>> {
        let (&winner, shadowed) = self.slot(ty, id)?.definers.split_last()?;
        Some(Provenance {
            winner: &self.files[winner],
            shadowed: shadowed.iter().map(|&i| &self.files[i]).collect(),
        })
    }

    /// The decoded record (`ty`, `id`); `None` if it is absent or its type
    /// is not registered.
    #[must_use]
    pub fn get_any(
        &self,
        ty: ResType,
        id: i16,
    ) -> Option<Result<StoreEntry<'_, AnyRecord>, &DecodeError>> {
        let slot = self.slot(ty, id)?;
        let source = self.winner(slot);
        let decoded = slot.decoded.get_or_init(|| {
            source
                .resources
                .get(ty, id)
                .and_then(|res| decode_any(&res))
        });
        Some(match decoded.as_ref()? {
            Ok((entry, warning)) => Ok(StoreEntry {
                id: entry.id,
                name: entry.name.as_deref(),
                record: &entry.record,
                warning: warning.as_ref(),
                source,
            }),
            Err(error) => Err(error),
        })
    }

    /// The decoded record of type `T` with ID `id`.
    #[must_use]
    pub fn get<T: Registered>(&self, id: i16) -> Option<Result<StoreEntry<'_, T>, &DecodeError>> {
        Some(self.get_any(T::TYPE, id)?.map(|entry| StoreEntry {
            id: entry.id,
            name: entry.name,
            record: T::from_any(entry.record).expect("a record decoded from its own type"),
            warning: entry.warning,
            source: entry.source,
        }))
    }

    /// Every record of type `T`, by ascending ID, each decoded (or its
    /// error) as [`GameData::get`] gives it.
    pub fn records<'a, T: Registered + 'a>(
        &'a self,
    ) -> impl Iterator<Item = (i16, Result<StoreEntry<'a, T>, &'a DecodeError>)> + 'a {
        self.ids(T::TYPE)
            .iter()
            .filter_map(|&id| Some((id, self.get::<T>(id)?)))
    }

    fn slot(&self, ty: ResType, id: i16) -> Option<&Slot> {
        let index = self.index.get(&ty)?;
        let at = index.ids.binary_search(&id).ok()?;
        Some(&index.slots[at])
    }

    fn winner(&self, slot: &Slot) -> &SourceFile {
        let last = *slot.definers.last().expect("every slot has a definer");
        &self.files[last]
    }
}

/// Indexes every resource of every file by (type, ID), recording each
/// defining file in load order.
fn build_index(files: &[SourceFile]) -> BTreeMap<ResType, TypeIndex> {
    let mut definers: BTreeMap<ResType, BTreeMap<i16, Vec<usize>>> = BTreeMap::new();
    for (at, file) in files.iter().enumerate() {
        for res in file.resources.iter() {
            // A file never defines a (type, ID) twice: `ResourceFile`
            // rejects duplicate IDs.
            definers
                .entry(res.res_type())
                .or_default()
                .entry(res.id())
                .or_default()
                .push(at);
        }
    }
    definers
        .into_iter()
        .map(|(ty, by_id)| {
            let (ids, slots) = by_id
                .into_iter()
                .map(|(id, definers)| {
                    let slot = Slot {
                        definers,
                        decoded: OnceLock::new(),
                    };
                    (id, slot)
                })
                .unzip();
            (ty, TypeIndex { ids, slots })
        })
        .collect()
}
