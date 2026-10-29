//! A throwaway GPG key, made in a gpg home the test owns, for the opt-in
//! tests of the Flatpak repo's key checks (`gpg_repo_key`) and of the
//! repo itself (`linux_flatpak_repo`).
//!
//! The key has no passphrase and lives only in a temporary directory. Its
//! secret half leaves gpg only as the base64 text [`ThrowawayKey::secret_base64`]
//! returns, which a test puts in an in-memory environment and never
//! prints. Dropping it stops the home's gpg-agent and removes the home.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use nova_bundle::linux::flatpak::repo::key::{Primary, fingerprint};

/// A throwaway Ed25519 signing key.
pub struct ThrowawayKey {
    tmp: tempfile::TempDir,
    /// Its primary key's fingerprint.
    pub fingerprint: String,
}

impl ThrowawayKey {
    /// Makes one in a new temporary gpg home.
    pub fn new() -> Self {
        let tmp = tempfile::tempdir().expect("temp dir");
        let mut key = Self {
            tmp,
            fingerprint: String::new(),
        };
        std::fs::create_dir_all(key.home()).expect("the gpg home");
        key.gpg(&[
            "--pinentry-mode",
            "loopback",
            "--passphrase",
            "",
            "--quick-gen-key",
            "ev-nova test key",
            "ed25519",
            "sign",
            "never",
        ]);
        let listing = key.gpg(&["--with-colons", "--list-secret-keys"]);
        key.fingerprint = fingerprint(&String::from_utf8_lossy(&listing), Primary::Secret)
            .expect("the new key's fingerprint");
        key
    }

    /// The gpg home.
    pub fn home(&self) -> PathBuf {
        self.tmp.path().join("keygen")
    }

    /// A directory beside the home, for the test's own files.
    pub fn dir(&self) -> &Path {
        self.tmp.path()
    }

    /// Runs gpg in the home, in batch mode, with these arguments, and
    /// returns its standard output. Its standard error, which never holds
    /// key material, is shown on failure; its output never is.
    pub fn gpg(&self, args: &[&str]) -> Vec<u8> {
        let output = Command::new("gpg")
            .arg("--homedir")
            .arg(self.home())
            .args(["--batch", "--no-permission-warning"])
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("gpg runs");
        assert!(
            output.status.success(),
            "gpg {:?}: {}\n{}",
            args.first(),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    /// The public key, ASCII-armored, as the operator commits it.
    pub fn write_public(&self, path: &Path) {
        let armored = self.gpg(&[
            "--armor",
            "--export-options",
            "export-minimal",
            "--export",
            &self.fingerprint,
        ]);
        std::fs::write(path, armored).expect("the public key file");
    }

    /// The public key, binary.
    pub fn public_binary(&self) -> Vec<u8> {
        self.gpg(&[
            "--export-options",
            "export-minimal",
            "--export",
            &self.fingerprint,
        ])
    }

    /// The secret key, ASCII-armored, as an operator might commit it by
    /// mistake.
    pub fn write_secret_armored(&self, path: &Path) {
        let armored = self.gpg(&[
            "--pinentry-mode",
            "loopback",
            "--passphrase",
            "",
            "--armor",
            "--export-options",
            "export-minimal",
            "--export-secret-keys",
            &self.fingerprint,
        ]);
        std::fs::write(path, armored).expect("the secret key file");
    }

    /// The secret key as `FLATPAK_SIGNING_KEY` holds it: base64 of the
    /// binary export, on one line. Never print it.
    pub fn secret_base64(&self) -> String {
        STANDARD.encode(self.gpg(&[
            "--pinentry-mode",
            "loopback",
            "--passphrase",
            "",
            "--export-options",
            "export-minimal",
            "--export-secret-keys",
            &self.fingerprint,
        ]))
    }
}

impl Drop for ThrowawayKey {
    fn drop(&mut self) {
        kill_agent(&self.home());
    }
}

/// Stops the gpg-agent of the home `home`, if one runs.
pub fn kill_agent(home: &Path) {
    let status = Command::new("gpgconf")
        .arg("--homedir")
        .arg(home)
        .args([OsStr::new("--kill"), OsStr::new("gpg-agent")])
        .stdin(Stdio::null())
        .status();
    if !status.as_ref().is_ok_and(std::process::ExitStatus::success) {
        eprintln!("cleanup: gpgconf --kill gpg-agent failed: {status:?}");
    }
}
