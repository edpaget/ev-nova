//! The real adapters: [`StdProcess`] runs programs with
//! `std::process::Command`, [`StdFiles`] works on the real filesystem,
//! [`StdEnv`] reads the process's environment, and [`Ed25519Signer`] signs
//! with ed25519-dalek.

use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::{self, Write as _};
use std::path::Path;
use std::process::{Command, Stdio};

use ed25519_dalek::{Signer as _, SigningKey};

use crate::appcast::key::{KeyError, PublicKey, decode_32};
use crate::ports::{Env, Files, Invocation, Process, ReadFiles, RunError, Signer};

/// Runs programs as child processes, with the bundler's environment plus
/// the invocation's variables. A captured run's standard error still goes
/// to the bundler's.
#[derive(Debug, Default)]
pub struct StdProcess;

impl Process for StdProcess {
    fn run(&mut self, invocation: &Invocation) -> Result<Vec<u8>, RunError> {
        let mut command = Command::new(&invocation.program);
        command
            .args(&invocation.args)
            .envs(invocation.env.iter().map(|(name, value)| (name, value)))
            .stdin(Stdio::null());
        let (status, stdout) = if invocation.capture {
            let output = command
                .stderr(Stdio::inherit())
                .output()
                .map_err(RunError::Spawn)?;
            (output.status, output.stdout)
        } else {
            (command.status().map_err(RunError::Spawn)?, Vec::new())
        };
        if status.success() {
            Ok(stdout)
        } else {
            Err(RunError::Failed(status.code()))
        }
    }
}

/// The process's environment variables.
#[derive(Debug, Default)]
pub struct StdEnv;

impl Env for StdEnv {
    fn var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }
}

/// The real filesystem.
#[derive(Debug, Default)]
pub struct StdFiles;

impl Files for StdFiles {
    fn remove_dir_all(&mut self, path: &Path) -> io::Result<()> {
        match fs::remove_dir_all(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }

    fn create_dir_all(&mut self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn write(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        fs::write(path, bytes)
    }

    fn copy(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        fs::copy(from, to).map(drop)
    }

    /// Mode `0600` on Unix applies when the file is created; the bundler
    /// writes its secrets into a directory it has just made.
    fn write_secret(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut options = fs::OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        options.open(path)?.write_all(bytes)
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn symlink(&mut self, target: &Path, link: &Path) -> io::Result<()> {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link)
        }
        #[cfg(not(unix))]
        {
            let _ = (target, link);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "the bundle's symbolic links need a Unix host",
            ))
        }
    }
}

impl ReadFiles for StdFiles {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn file_names(&self, dir: &Path) -> io::Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        names.sort();
        Ok(names)
    }
}

/// Signs with an Ed25519 key made from its 32-byte seed. Its `Debug`
/// shows only the public key.
pub struct Ed25519Signer(SigningKey);

impl Ed25519Signer {
    /// The signer of the seed `text` holds as standard base64 (32 bytes,
    /// as `openssl rand -base64 32` prints), surrounding whitespace aside.
    ///
    /// # Errors
    ///
    /// Text that isn't base64 of 32 bytes. The error never holds the text.
    pub fn from_base64_seed(text: &str) -> Result<Self, KeyError> {
        decode_32(text).map(|seed| Self(SigningKey::from_bytes(&seed)))
    }
}

impl Signer for Ed25519Signer {
    fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.0.sign(message).to_bytes()
    }

    fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key().to_bytes())
    }
}

/// The public key alone: never the seed.
impl fmt::Debug for Ed25519Signer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ed25519Signer")
            .field("public_key", &self.public_key().to_base64())
            .finish_non_exhaustive()
    }
}
