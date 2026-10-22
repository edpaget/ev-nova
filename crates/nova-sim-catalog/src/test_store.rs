//! Test support shared by the crate's unit tests: game data loaded from
//! one in-memory data file, and a big-endian field writer for building
//! records.

use std::io;
use std::path::Path;

use nova_data::GameData;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};

/// One data file, `/data/Nova Data`, holding a fork.
struct OneFile(Vec<u8>);

impl DirLister for OneFile {
    fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
        Ok(vec![Listing {
            name: "Nova Data".into(),
            kind: EntryKind::File,
        }])
    }
}

impl ForkReader for OneFile {
    fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        Ok((fork == Fork::Data).then(|| self.0.clone()))
    }
}

/// The store of these resources, none of them named.
pub(crate) fn store(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
    let named: Vec<_> = resources
        .iter()
        .map(|(ty, id, data)| (*ty, *id, None, data.clone()))
        .collect();
    store_named(&named)
}

/// The store of these resources, each with its name, if any.
pub(crate) fn store_named(resources: &[(ResType, i16, Option<&str>, Vec<u8>)]) -> GameData {
    let fork = resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, name, data)| {
            fork.resource(*ty, *id, name.map(str::as_bytes), data)
        })
        .build()
        .bytes;
    let file = OneFile(fork);
    GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
}

/// Writes `values` big-endian into `bytes` from offset `at`.
pub(crate) fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
    }
}
