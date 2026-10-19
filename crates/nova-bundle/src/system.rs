//! The real adapters: [`StdProcess`] runs programs with
//! `std::process::Command`, and [`StdFiles`] works on the real filesystem.

use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::ports::{Files, Invocation, Process, RunError};

/// Runs programs as child processes. A captured run's standard error
/// still goes to the bundler's.
#[derive(Debug, Default)]
pub struct StdProcess;

impl Process for StdProcess {
    fn run(&mut self, invocation: &Invocation) -> Result<Vec<u8>, RunError> {
        let mut command = Command::new(&invocation.program);
        command.args(&invocation.args).stdin(Stdio::null());
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
