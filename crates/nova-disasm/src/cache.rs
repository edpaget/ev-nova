//! Loading the listings: from the cache when the binary's hash is known,
//! else from the tools, storing them for next time.
//!
//! What is cached is the tools' raw output, not a parsed index: the
//! listings are the format otool already guarantees, and parsing them
//! takes well under a second in a release build.

use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::ports::{Binary, Cache, Listing, Listings, ToolError, Tools};

/// Why the listings could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// The binary is not where it was looked for.
    #[error(
        "EV Nova binary not found at {}; put EV Nova.app in the primary checkout, set NOVA_BINARY, or pass --binary <path>",
        path.display()
    )]
    BinaryMissing {
        /// Where it was looked for.
        path: PathBuf,
    },
    /// The binary is there but could not be read.
    #[error("could not read {}: {source}", path.display())]
    BinaryRead {
        /// The binary's path.
        path: PathBuf,
        /// The error.
        source: io::Error,
    },
    /// A tool failed.
    #[error(transparent)]
    Tool(#[from] ToolError),
}

/// The cache key for a binary: the lower-case hex SHA-256 of its bytes.
#[must_use]
pub fn key(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        })
}

/// The binary's listings: the cached ones if its hash is known, else a
/// fresh run of every tool, stored under the hash. A failed store is a
/// warning on `warn`, not a failure.
pub fn load(
    path: &Path,
    binary: &dyn Binary,
    tools: &dyn Tools,
    cache: &dyn Cache,
    warn: &mut dyn Write,
) -> Result<Listings, LoadError> {
    let bytes = binary.read(path).map_err(|source| match source.kind() {
        io::ErrorKind::NotFound => LoadError::BinaryMissing {
            path: path.to_path_buf(),
        },
        _ => LoadError::BinaryRead {
            path: path.to_path_buf(),
            source,
        },
    })?;
    let key = key(&bytes);
    if let Some(listings) = cache.get(&key) {
        return Ok(listings);
    }
    let mut listings = Listings::default();
    for which in Listing::ALL {
        *listings.get_mut(which) = tools.listing(path, which)?;
    }
    if let Err(err) = cache.put(&key, &listings) {
        let _ = writeln!(warn, "warning: could not cache the listings: {err}");
    }
    Ok(listings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::{FakeBinary, FakeTools, MemCache};

    fn path() -> PathBuf {
        PathBuf::from("EV Nova")
    }

    fn run(
        binary: &FakeBinary,
        tools: &FakeTools,
        cache: &MemCache,
    ) -> (Result<Listings, LoadError>, String) {
        let mut warn = Vec::new();
        let result = load(&path(), binary, tools, cache, &mut warn);
        (result, String::from_utf8(warn).unwrap())
    }

    #[test]
    fn the_key_is_the_sha256_of_the_bytes() {
        assert_eq!(
            key(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn a_first_load_runs_each_tool_once_and_stores_the_listings() {
        let (binary, tools, cache) = (
            FakeBinary::with(b"v1"),
            FakeTools::named(),
            MemCache::default(),
        );
        let listings = run(&binary, &tools, &cache).0.unwrap();
        assert_eq!(listings, tools.listings);
        for which in Listing::ALL {
            assert_eq!(tools.count(which), 1, "{which:?}");
        }
        assert_eq!(cache.entries.borrow().get(&key(b"v1")), Some(&listings));
        assert_eq!(*binary.reads.borrow(), [path()]);
    }

    #[test]
    fn an_unchanged_binary_reuses_the_cache_without_running_a_tool() {
        let (binary, tools, cache) = (
            FakeBinary::with(b"v1"),
            FakeTools::named(),
            MemCache::default(),
        );
        let first = run(&binary, &tools, &cache).0.unwrap();
        tools.runs.borrow_mut().clear();
        let second = run(&binary, &tools, &cache).0.unwrap();
        assert_eq!(second, first);
        assert!(tools.runs.borrow().is_empty());
    }

    #[test]
    fn a_changed_binary_rebuilds_under_a_new_key() {
        let (binary, tools, cache) = (
            FakeBinary::with(b"v1"),
            FakeTools::named(),
            MemCache::default(),
        );
        run(&binary, &tools, &cache).0.unwrap();
        *binary.bytes.borrow_mut() = Some(b"v2".to_vec());
        tools.runs.borrow_mut().clear();
        run(&binary, &tools, &cache).0.unwrap();
        assert_eq!(tools.runs.borrow().len(), Listing::ALL.len());
        let entries = cache.entries.borrow();
        assert!(entries.contains_key(&key(b"v1")) && entries.contains_key(&key(b"v2")));
    }

    #[test]
    fn a_failed_store_is_a_warning() {
        let (binary, tools, cache) = (
            FakeBinary::with(b"v1"),
            FakeTools::named(),
            MemCache::default(),
        );
        cache.fail_put.set(true);
        let (result, warn) = run(&binary, &tools, &cache);
        assert_eq!(result.unwrap(), tools.listings);
        assert!(warn.contains("disk full"), "{warn}");
    }

    #[test]
    fn a_missing_binary_names_its_path_and_runs_nothing() {
        let (binary, tools, cache) = (
            FakeBinary::missing(),
            FakeTools::named(),
            MemCache::default(),
        );
        let err = run(&binary, &tools, &cache).0.unwrap_err();
        assert!(matches!(&err, LoadError::BinaryMissing { path: p } if *p == path()));
        assert!(err.to_string().starts_with(&format!(
            "EV Nova binary not found at {};",
            path().display()
        )));
        assert!(tools.runs.borrow().is_empty());
    }

    #[test]
    fn a_missing_tool_is_an_error() {
        let (binary, mut tools, cache) = (
            FakeBinary::with(b"v1"),
            FakeTools::named(),
            MemCache::default(),
        );
        tools.missing = true;
        let err = run(&binary, &tools, &cache).0.unwrap_err();
        assert!(err.to_string().contains("otool not found"), "{err}");
        assert!(cache.entries.borrow().is_empty());
    }
}
