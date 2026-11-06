//! The ports the core reaches the outside world through.
//!
//! One "otool run" is six listings of the same binary ([`Listing`]); the
//! [`Tools`] port produces each one, and the [`Cache`] port keeps the six
//! together ([`Listings`]) under the binary's hash.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

/// One listing of the binary, as one tool invocation prints it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Listing {
    /// `otool -tV`: the `__text` section disassembled, with labels.
    Text,
    /// `otool -v -s __TEXT __cstring`: each C string and its address.
    CStrings,
    /// `otool -s __IMPORT __pointers`: the non-lazy pointer slots, in hex.
    Pointers,
    /// `otool -Iv`: which pointer slots name an imported symbol.
    Indirect,
    /// `nm -n`: every symbol, sorted by address.
    Symbols,
    /// `otool -l`: the load commands, for each section's bounds.
    Sections,
}

impl Listing {
    /// Every listing, in a fixed order.
    pub const ALL: [Listing; 6] = [
        Listing::Text,
        Listing::CStrings,
        Listing::Pointers,
        Listing::Indirect,
        Listing::Symbols,
        Listing::Sections,
    ];

    /// A short stable name, used for the cache's file names.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Listing::Text => "text",
            Listing::CStrings => "cstring",
            Listing::Pointers => "pointers",
            Listing::Indirect => "indirect",
            Listing::Symbols => "nm",
            Listing::Sections => "sections",
        }
    }
}

/// The six listings of one binary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listings {
    /// [`Listing::Text`].
    pub text: String,
    /// [`Listing::CStrings`].
    pub cstrings: String,
    /// [`Listing::Pointers`].
    pub pointers: String,
    /// [`Listing::Indirect`].
    pub indirect: String,
    /// [`Listing::Symbols`].
    pub symbols: String,
    /// [`Listing::Sections`].
    pub sections: String,
}

impl Listings {
    /// The text of one listing.
    #[must_use]
    pub fn get(&self, which: Listing) -> &str {
        match which {
            Listing::Text => &self.text,
            Listing::CStrings => &self.cstrings,
            Listing::Pointers => &self.pointers,
            Listing::Indirect => &self.indirect,
            Listing::Symbols => &self.symbols,
            Listing::Sections => &self.sections,
        }
    }

    /// The slot for one listing.
    pub fn get_mut(&mut self, which: Listing) -> &mut String {
        match which {
            Listing::Text => &mut self.text,
            Listing::CStrings => &mut self.cstrings,
            Listing::Pointers => &mut self.pointers,
            Listing::Indirect => &mut self.indirect,
            Listing::Symbols => &mut self.symbols,
            Listing::Sections => &mut self.sections,
        }
    }
}

/// Why a listing could not be produced.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// The tool is not installed.
    #[error("{program} not found: install the Xcode command line tools (xcode-select --install)")]
    Missing {
        /// The program that could not be started.
        program: String,
    },
    /// The tool ran and failed.
    #[error("{program} failed ({status}): {stderr}")]
    Failed {
        /// The program that failed.
        program: String,
        /// How it exited.
        status: String,
        /// What it printed on stderr.
        stderr: String,
    },
    /// The tool could not be run for another reason.
    #[error("could not run {program}: {source}")]
    Io {
        /// The program.
        program: String,
        /// The error.
        source: io::Error,
    },
}

/// Lists one part of the binary.
pub trait Tools {
    /// Runs the tool that prints `which` for `binary` and returns its output.
    fn listing(&self, binary: &Path, which: Listing) -> Result<String, ToolError>;
}

/// Reads the binary's bytes, to hash them.
pub trait Binary {
    /// The binary's bytes; [`io::ErrorKind::NotFound`] when it is absent.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
}

/// Keeps listings under a key (the binary's hash).
pub trait Cache {
    /// The listings stored under `key`, if every one is there: an entry
    /// stored before a listing was added is missing it, and so is none.
    fn get(&self, key: &str) -> Option<Listings>;
    /// Stores the listings under `key`.
    fn put(&self, key: &str, listings: &Listings) -> io::Result<()>;
}

/// Reads environment variables.
pub trait Env {
    /// The variable's value, if set.
    fn var(&self, name: &str) -> Option<OsString>;
}

/// Where the checkouts are.
pub trait Workspace {
    /// The primary checkout (the one whose `.git` the worktrees share),
    /// where the gitignored `EV Nova.app` lives; `None` when git can't say.
    fn primary_checkout(&self) -> Option<PathBuf>;
    /// The root of the workspace this tool was built from.
    fn workspace_root(&self) -> PathBuf;
}
