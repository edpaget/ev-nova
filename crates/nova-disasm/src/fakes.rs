//! Hand-written fakes of the ports, for the core's tests.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use crate::ports::{Binary, Cache, Env, Listing, Listings, ToolError, Tools, Workspace};

/// A binary whose bytes the test sets; `None` is a missing file.
#[derive(Default)]
pub struct FakeBinary {
    pub bytes: RefCell<Option<Vec<u8>>>,
    pub reads: RefCell<Vec<PathBuf>>,
}

impl FakeBinary {
    pub fn with(bytes: &[u8]) -> FakeBinary {
        FakeBinary {
            bytes: RefCell::new(Some(bytes.to_vec())),
            reads: RefCell::default(),
        }
    }

    pub fn missing() -> FakeBinary {
        FakeBinary::default()
    }
}

impl Binary for FakeBinary {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.reads.borrow_mut().push(path.to_path_buf());
        self.bytes
            .borrow()
            .clone()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }
}

/// Tools that answer from canned listings and record each run.
pub struct FakeTools {
    pub listings: Listings,
    pub runs: RefCell<Vec<Listing>>,
    pub missing: bool,
}

impl FakeTools {
    pub fn with(listings: Listings) -> FakeTools {
        FakeTools {
            listings,
            runs: RefCell::default(),
            missing: false,
        }
    }

    /// Tools that answer `"<name> listing"` for each listing.
    pub fn named() -> FakeTools {
        let mut listings = Listings::default();
        for which in Listing::ALL {
            *listings.get_mut(which) = format!("{} listing", which.name());
        }
        FakeTools::with(listings)
    }

    /// How many times `which` was listed.
    pub fn count(&self, which: Listing) -> usize {
        self.runs.borrow().iter().filter(|&&w| w == which).count()
    }
}

impl Tools for FakeTools {
    fn listing(&self, _binary: &Path, which: Listing) -> Result<String, ToolError> {
        self.runs.borrow_mut().push(which);
        if self.missing {
            return Err(ToolError::Missing {
                program: "otool".into(),
            });
        }
        Ok(self.listings.get(which).to_owned())
    }
}

/// An in-memory cache.
#[derive(Default)]
pub struct MemCache {
    pub entries: RefCell<HashMap<String, Listings>>,
    pub fail_put: Cell<bool>,
}

impl Cache for MemCache {
    fn get(&self, key: &str) -> Option<Listings> {
        self.entries.borrow().get(key).cloned()
    }

    fn put(&self, key: &str, listings: &Listings) -> io::Result<()> {
        if self.fail_put.get() {
            return Err(io::Error::other("disk full"));
        }
        self.entries
            .borrow_mut()
            .insert(key.to_owned(), listings.clone());
        Ok(())
    }
}

/// A fixed environment.
#[derive(Default)]
pub struct FakeEnv(pub HashMap<String, OsString>);

impl FakeEnv {
    pub fn with(name: &str, value: impl Into<OsString>) -> FakeEnv {
        FakeEnv(HashMap::from([(name.to_owned(), value.into())]))
    }
}

impl Env for FakeEnv {
    fn var(&self, name: &str) -> Option<OsString> {
        self.0.get(name).cloned()
    }
}

/// Fixed checkout paths.
pub struct FakeWorkspace {
    pub primary: Option<PathBuf>,
    pub root: PathBuf,
}

impl Workspace for FakeWorkspace {
    fn primary_checkout(&self) -> Option<PathBuf> {
        self.primary.clone()
    }

    fn workspace_root(&self) -> PathBuf {
        self.root.clone()
    }
}
