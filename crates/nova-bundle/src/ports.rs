//! The ports the bundler's core reaches the outside world through: running
//! a program, and the files of the staging tree.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
use std::path::Path;

/// A program to run, with every argument the core chose for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// The program, found on `PATH` unless it is a path.
    pub program: OsString,
    /// Its arguments, in order.
    pub args: Vec<OsString>,
    /// Whether the core reads the program's standard output. Otherwise the
    /// output goes to the bundler's own.
    pub capture: bool,
}

impl Invocation {
    /// A program whose output goes to the bundler's own.
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            capture: false,
        }
    }

    /// Adds one argument.
    #[must_use]
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    /// Adds several arguments.
    #[must_use]
    pub fn args<I, A>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|a| a.as_ref().to_owned()));
        self
    }

    /// Captures the program's standard output for the core to read.
    #[must_use]
    pub fn captured(mut self) -> Self {
        self.capture = true;
        self
    }
}

/// The command line, as an error message or a log shows it.
impl fmt::Display for Invocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.program.to_string_lossy())?;
        for arg in &self.args {
            write!(f, " {}", arg.to_string_lossy())?;
        }
        Ok(())
    }
}

/// Why a program could not be run to success.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// It could not be started (not found, not executable).
    #[error("could not start it: {0}")]
    Spawn(#[source] io::Error),
    /// It ran and exited unsuccessfully: with this exit code, or with none
    /// when a signal ended it.
    #[error("{}", exit_text(*.0))]
    Failed(Option<i32>),
}

fn exit_text(code: Option<i32>) -> String {
    match code {
        Some(code) => format!("it exited with code {code}"),
        None => "a signal ended it".to_owned(),
    }
}

/// Port: runs a program to completion.
pub trait Process {
    /// Runs `invocation`, waiting for it to exit. Returns its standard
    /// output when `invocation.capture` is set, and nothing otherwise. A
    /// program that exits unsuccessfully is an error.
    fn run(&mut self, invocation: &Invocation) -> Result<Vec<u8>, RunError>;
}

/// Port: the files of the staging tree and the output directory.
pub trait Files {
    /// Removes the directory and everything in it, if it exists.
    fn remove_dir_all(&mut self, path: &Path) -> io::Result<()>;

    /// Creates the directory and any missing parents.
    fn create_dir_all(&mut self, path: &Path) -> io::Result<()>;

    /// Writes `bytes` to the file, replacing it if it exists.
    fn write(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()>;

    /// Copies the file `from` to `to`.
    fn copy(&mut self, from: &Path, to: &Path) -> io::Result<()>;

    /// Makes a symbolic link at `link` pointing to `target`.
    fn symlink(&mut self, target: &Path, link: &Path) -> io::Result<()>;
}
