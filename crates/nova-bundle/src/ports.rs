//! The ports the bundler's core reaches the outside world through: running
//! a program, the files of the staging tree, the environment the signing
//! inputs come from, reading files back, and signing the update files.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
use std::path::Path;

use crate::appcast::key::PublicKey;

/// A program to run, with every argument and environment variable the
/// core chose for it.
///
/// An argument added with [`Invocation::secret`] (a password) is passed to
/// the program but shown as `***` by `Display` and `Debug`, so error
/// messages and logs never hold it.
#[derive(Clone, PartialEq, Eq)]
pub struct Invocation {
    /// The program, found on `PATH` unless it is a path.
    pub program: OsString,
    /// Its arguments, in order.
    pub args: Vec<OsString>,
    /// Whether the core reads the program's standard output. Otherwise the
    /// output goes to the bundler's own.
    pub capture: bool,
    /// The indexes in `args` of the secret arguments.
    pub secret: Vec<usize>,
    /// Variables set for the program on top of the bundler's own
    /// environment, in order. `Display` leaves them out: it is the command
    /// line.
    pub env: Vec<(OsString, OsString)>,
}

impl Invocation {
    /// A program whose output goes to the bundler's own.
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            capture: false,
            secret: Vec::new(),
            env: Vec::new(),
        }
    }

    /// Sets one environment variable for the program.
    #[must_use]
    pub fn env(mut self, name: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((name.into(), value.into()));
        self
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

    /// Adds one argument that is never shown: `Display` and `Debug` print
    /// `***` in its place.
    #[must_use]
    pub fn secret(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.secret.push(self.args.len());
        self.arg(arg)
    }

    /// The arguments as they may be shown, with each secret one `***`.
    fn shown_args(&self) -> impl Iterator<Item = std::borrow::Cow<'_, str>> {
        self.args.iter().enumerate().map(|(i, arg)| {
            if self.secret.contains(&i) {
                "***".into()
            } else {
                arg.to_string_lossy()
            }
        })
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
        for arg in self.shown_args() {
            write!(f, " {arg}")?;
        }
        Ok(())
    }
}

/// As derived, but with each secret argument `***`.
impl fmt::Debug for Invocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Invocation")
            .field("program", &self.program)
            .field("args", &self.shown_args().collect::<Vec<_>>())
            .field("capture", &self.capture)
            .field("secret", &self.secret)
            .field("env", &self.env)
            .finish()
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

    /// Writes secret `bytes` (a certificate, a key) to the file, replacing
    /// it if it exists, readable by its owner alone where the host has
    /// permissions.
    fn write_secret(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()>;

    /// Whether the path is an existing file (not a directory).
    fn is_file(&self, path: &Path) -> bool;
}

/// Port: the environment variables the signing inputs come from.
pub trait Env {
    /// The variable's value, or `None` when it is unset.
    fn var(&self, name: &str) -> Option<OsString>;
}

/// Port: reads files back, as `nova-appcast` reads the installers, the
/// release's asset list and the published feeds.
pub trait ReadFiles {
    /// The file's bytes.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// The names of the files (not the directories) in the directory,
    /// sorted.
    fn file_names(&self, dir: &Path) -> io::Result<Vec<String>>;
}

/// Port: signs with the update-signing key. Its `Debug`, where it has one,
/// never shows the key.
pub trait Signer {
    /// The Ed25519 signature of `message`.
    fn sign(&self, message: &[u8]) -> [u8; 64];

    /// The public key the signatures verify under.
    fn public_key(&self) -> PublicKey;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_argument_is_passed_but_never_shown() {
        let invocation = Invocation::new("security")
            .args(["create-keychain", "-p"])
            .secret("hunter2")
            .arg("kc");
        assert_eq!(invocation.args, ["create-keychain", "-p", "hunter2", "kc"]);
        assert_eq!(invocation.to_string(), "security create-keychain -p *** kc");
        let debug = format!("{invocation:?}");
        assert!(!debug.contains("hunter2"), "{debug}");
        assert!(debug.contains("***"), "{debug}");
        assert!(debug.contains("create-keychain"), "{debug}");
    }

    #[test]
    fn secrets_are_compared_with_everything_else() {
        let plain = Invocation::new("security").arg("pw");
        let secret = Invocation::new("security").secret("pw");
        assert_ne!(plain, secret);
        assert_eq!(secret, Invocation::new("security").secret("pw"));
    }

    #[test]
    fn an_environment_variable_is_compared_but_not_shown() {
        let plain = Invocation::new("cargo").arg("build");
        let with = plain.clone().env("A", "b");
        assert_eq!(with.env, [(OsString::from("A"), OsString::from("b"))]);
        assert_ne!(plain, with);
        assert_eq!(with, plain.clone().env("A", "b"));
        assert_ne!(with, plain.clone().env("A", "c"));
        assert_eq!(with.to_string(), "cargo build");
        let debug = format!("{with:?}");
        assert!(debug.contains("env"), "{debug}");
    }
}
