//! Checking a download against its pinned SHA-256 before anything uses it:
//! Sparkle's archive on macOS, WinSparkle's archive and DLL on Windows.
//!
//! The digest comes from the host's own tool ([`DigestTool`]) through the
//! [`Process`] port, and the core reads its output: `shasum -a 256` on
//! macOS, whose digest is the first word, and `certutil -hashfile <path>
//! SHA256` on Windows, which is in every Windows (`shasum` is a Perl script
//! there that `Command` can't start). `certutil`'s digest is the first
//! line that is 64 hex digits once spaces are removed: older versions
//! print the bytes spaced.

use std::path::Path;

use crate::bundle::{BundleError, run};
use crate::ports::{Invocation, Process};

/// A digest a download must have, and what it is the digest of, as the
/// error names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pin {
    /// What is pinned, e.g. `Sparkle 2.10.1`.
    pub what: &'static str,
    /// Its SHA-256, lower-case hex.
    pub sha256: &'static str,
}

/// The tool that digests a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestTool {
    /// `shasum -a 256 <path>` (macOS).
    Shasum,
    /// `certutil -hashfile <path> SHA256` (Windows).
    Certutil,
}

impl DigestTool {
    /// The run that digests `path`, captured.
    #[must_use]
    pub fn invocation(self, path: &Path) -> Invocation {
        match self {
            Self::Shasum => Invocation::new("shasum")
                .args(["-a", "256"])
                .arg(path)
                .captured(),
            Self::Certutil => Invocation::new("certutil")
                .arg("-hashfile")
                .arg(path)
                .arg("SHA256")
                .captured(),
        }
    }

    /// The digest in the tool's `stdout`, lower-case, or empty when it
    /// printed none.
    #[must_use]
    pub fn digest(self, stdout: &str) -> String {
        let found = match self {
            Self::Shasum => stdout.split_whitespace().next().map(str::to_owned),
            Self::Certutil => stdout
                .lines()
                .map(|line| line.split_whitespace().collect::<String>())
                .find(|line| line.len() == 64 && line.chars().all(|c| c.is_ascii_hexdigit())),
        };
        found.unwrap_or_default().to_ascii_lowercase()
    }
}

/// Digests `path` with `tool` and fails unless the digest is `pin`'s,
/// ignoring case: a file that isn't the pinned one stops the run before
/// anything uses it.
pub fn check_sha256(
    process: &mut impl Process,
    path: &Path,
    pin: Pin,
    tool: DigestTool,
) -> Result<(), BundleError> {
    let stdout = run(process, &tool.invocation(path))?;
    let found = tool.digest(&String::from_utf8_lossy(&stdout));
    if found == pin.sha256.to_ascii_lowercase() {
        Ok(())
    } else {
        Err(BundleError::Checksum {
            path: path.to_owned(),
            expected: pin.sha256.to_owned(),
            found,
            pinned: pin.what,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::ports::RunError;

    const DIGEST: &str = "1a856d2a9edb8e628de88121d7be04cf0e2167f8641650b78fca00b14244cf27";
    const PIN: Pin = Pin {
        what: "Sparkle 2.10.1",
        sha256: DIGEST,
    };

    /// Answers every run with `stdout`, recording each.
    struct Prints {
        stdout: Vec<u8>,
        runs: Vec<Invocation>,
    }

    impl Process for Prints {
        fn run(&mut self, invocation: &Invocation) -> Result<Vec<u8>, RunError> {
            self.runs.push(invocation.clone());
            Ok(self.stdout.clone())
        }
    }

    fn prints(stdout: &str) -> Prints {
        Prints {
            stdout: stdout.as_bytes().to_vec(),
            runs: Vec::new(),
        }
    }

    fn file() -> PathBuf {
        Path::new("/d").join("archive")
    }

    /// `digest` as an older `certutil` prints it: each byte spaced.
    fn spaced(digest: &str) -> String {
        digest
            .as_bytes()
            .chunks(2)
            .map(|pair| String::from_utf8_lossy(pair).into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn each_tool_digests_the_file_captured() {
        assert_eq!(
            DigestTool::Shasum.invocation(&file()),
            Invocation::new("shasum")
                .args(["-a", "256"])
                .arg(file())
                .captured()
        );
        assert_eq!(
            DigestTool::Certutil.invocation(&file()),
            Invocation::new("certutil")
                .arg("-hashfile")
                .arg(file())
                .arg("SHA256")
                .captured()
        );
    }

    #[test]
    fn shasum_s_digest_is_its_first_word() {
        let upper = DIGEST.to_ascii_uppercase();
        for stdout in [
            format!("{DIGEST}  /d/archive\n"),
            format!("{DIGEST}\n"),
            format!("  {DIGEST} *x"),
            format!("{upper}  x\n"),
        ] {
            assert_eq!(DigestTool::Shasum.digest(&stdout), DIGEST, "{stdout:?}");
        }
    }

    #[test]
    fn certutil_s_digest_is_the_line_of_64_hex_digits_spaced_or_not() {
        let upper = DIGEST.to_ascii_uppercase();
        for stdout in [
            format!(
                "SHA256 hash of C:\\d\\archive:\r\n{DIGEST}\r\nCertUtil: -hashfile command \
                 completed successfully.\r\n"
            ),
            format!(
                "SHA256 hash of file C:\\d\\archive:\r\n{}\r\nCertUtil: -hashfile command \
                 completed successfully.\r\n",
                spaced(DIGEST)
            ),
            format!("{upper}\n"),
        ] {
            assert_eq!(DigestTool::Certutil.digest(&stdout), DIGEST, "{stdout:?}");
        }
        // A line that is hex but not 64 digits, or 64 characters but not
        // hex, is no digest.
        let short = &DIGEST[..62];
        let not_hex = "g".repeat(64);
        assert_eq!(
            DigestTool::Certutil.digest(&format!("{short}\n{not_hex}\n")),
            ""
        );
    }

    #[test]
    fn an_output_without_a_digest_is_an_error() {
        for (tool, stdout) in [
            (DigestTool::Shasum, ""),
            (DigestTool::Shasum, "  \n"),
            (DigestTool::Certutil, ""),
            (
                DigestTool::Certutil,
                "CertUtil: -hashfile command FAILED: 0x80070002\r\n",
            ),
        ] {
            let error =
                check_sha256(&mut prints(stdout), &file(), PIN, tool).expect_err("no digest");
            assert!(
                matches!(&error, BundleError::Checksum { found, .. } if found.is_empty()),
                "{tool:?} {stdout:?}: {error:?}"
            );
        }
    }

    #[test]
    fn a_matching_digest_passes_through_either_tool() {
        for (tool, stdout) in [
            (DigestTool::Shasum, format!("{DIGEST}  x\n")),
            (DigestTool::Certutil, format!("hash:\r\n{DIGEST}\r\nok\r\n")),
        ] {
            let mut process = prints(&stdout);
            check_sha256(&mut process, &file(), PIN, tool).expect(&stdout);
            assert_eq!(process.runs, [tool.invocation(&file())]);
        }
    }

    #[test]
    fn another_digest_is_an_error_naming_the_file_both_digests_and_the_pin() {
        let other = "0".repeat(64);
        for pin in [
            PIN,
            Pin {
                what: "WinSparkle 0.9.4 DLL",
                sha256: DIGEST,
            },
        ] {
            let mut process = prints(&format!("{other}  x\n"));
            let error =
                check_sha256(&mut process, &file(), pin, DigestTool::Shasum).expect_err("mismatch");
            let BundleError::Checksum {
                path,
                expected,
                found,
                pinned,
            } = &error
            else {
                panic!("{error:?}");
            };
            assert_eq!(path, &file());
            assert_eq!(expected, DIGEST);
            assert_eq!(found, &other);
            assert_eq!(*pinned, pin.what);
            assert_eq!(
                error.to_string(),
                format!(
                    "{} has SHA-256 `{other}`, not `{DIGEST}`, the pinned {}'s",
                    file().display(),
                    pin.what
                )
            );
        }
    }

    #[test]
    fn a_failing_tool_is_its_run_error() {
        struct Fails;
        impl Process for Fails {
            fn run(&mut self, _invocation: &Invocation) -> Result<Vec<u8>, RunError> {
                Err(RunError::Failed(Some(1)))
            }
        }
        for tool in [DigestTool::Shasum, DigestTool::Certutil] {
            let error = check_sha256(&mut Fails, &file(), PIN, tool).expect_err("fails");
            assert!(
                matches!(&error, BundleError::Run { command, .. } if *command == tool.invocation(&file()).to_string()),
                "{error:?}"
            );
        }
    }
}
