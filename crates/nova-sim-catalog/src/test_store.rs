//! Test support shared by the crate's unit tests: game data loaded from
//! one in-memory data file, and a big-endian field writer for building
//! records.

use nova_data::GameData;
use nova_data::store::fixture::{one_file, store_of};
use nova_rsrc::ResType;
use nova_rsrc::fixture::ForkBuilder;

/// The store of these resources, none of them named.
pub(crate) fn store(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
    store_of(resources)
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
    one_file(fork).load()
}

/// Writes `values` big-endian into `bytes` from offset `at`.
pub(crate) fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
    }
}
