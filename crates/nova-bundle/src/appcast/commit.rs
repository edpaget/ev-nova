//! The commit a build was made from, as its release records it.
//!
//! When `publish` signs a build it writes `ev-nova-<id>-commit.txt`
//! ([`commit_asset`]) beside the build's update files, holding the full
//! SHA of the commit built ([`Commit`]), which CI gives it in [`VAR`].
//! The release keeps a build's commit file exactly as long as it keeps an
//! update file of that build ([`super::prune`]). `check-promotion` reads
//! it to name the commit a nightly built ([`super::promote`]): the build
//! ID alone can't, since landing's rebase gives a run of commits the same
//! committer second.

use std::fmt;

use crate::build_id::BuildId;

/// The variable holding the full SHA of the commit being published. CI
/// sets it to the commit the build legs checked out.
pub const VAR: &str = "NOVA_COMMIT";

/// A commit's full name: 40 (SHA-1) or 64 (SHA-256) lowercase hex digits,
/// as git prints it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Commit(String);

/// Text that isn't a commit's full name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a commit's full SHA (40 or 64 lowercase hex digits)")]
pub struct CommitError(pub String);

impl Commit {
    /// The commit `text` names, surrounding whitespace ignored.
    ///
    /// # Errors
    ///
    /// [`CommitError`] holding the trimmed text when it isn't 40 or 64
    /// lowercase hex digits.
    pub fn parse(text: &str) -> Result<Self, CommitError> {
        let text = text.trim();
        let hex = text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        if matches!(text.len(), 40 | 64) && hex {
            Ok(Self(text.to_owned()))
        } else {
            Err(CommitError(text.to_owned()))
        }
    }

    /// The SHA.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The commit file's contents: the SHA and a newline.
    #[must_use]
    pub fn file(&self) -> String {
        format!("{}\n", self.0)
    }
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The release asset recording `build`'s commit: `ev-nova-<id>-commit.txt`.
#[must_use]
pub fn commit_asset(build: &BuildId) -> String {
    format!("ev-nova-{build}-commit.txt")
}

/// The build a commit file's name holds, or `None` for any other name
/// ([`commit_asset`]'s inverse).
#[must_use]
pub fn parse_commit_asset(name: &str) -> Option<BuildId> {
    let id = name.strip_prefix("ev-nova-")?.strip_suffix("-commit.txt")?;
    BuildId::parse(id).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appcast::{Platform, parse_update_asset, update_asset};

    fn id(text: &str) -> BuildId {
        BuildId::parse(text).expect("an ID")
    }

    const SHA1: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn a_full_lowercase_sha_parses_trimmed() {
        let commit = Commit::parse(&format!("  {SHA1}\n")).expect("a commit");
        assert_eq!(commit.as_str(), SHA1);
        assert_eq!(commit.to_string(), SHA1);
        assert_eq!(commit.file(), format!("{SHA1}\n"));
        let sha256 = "ab".repeat(32);
        assert_eq!(Commit::parse(&sha256).expect("SHA-256").as_str(), sha256);
    }

    #[test]
    fn anything_else_is_malformed_naming_the_trimmed_text() {
        for text in [
            String::new(),
            SHA1[1..].to_owned(),
            format!("{SHA1}0"),
            SHA1.to_uppercase(),
            format!("{}g", &SHA1[1..]),
            "a".repeat(63),
            "a".repeat(65),
            format!("{SHA1} {SHA1}"),
            "main".to_owned(),
        ] {
            let error = Commit::parse(&format!("{text}\n")).expect_err("malformed");
            assert_eq!(error, CommitError(text.clone()), "{text}");
            assert!(error.to_string().contains(&format!("`{text}`")), "{error}");
        }
    }

    #[test]
    fn the_commit_asset_carries_the_build_id_and_parses_back() {
        let build = id("20261028123456");
        assert_eq!(commit_asset(&build), "ev-nova-20261028123456-commit.txt");
        assert_eq!(
            parse_commit_asset(&commit_asset(&build)),
            Some(build.clone())
        );
        // Neither kind of asset is taken for the other.
        for platform in Platform::ALL {
            assert_eq!(parse_commit_asset(&update_asset(platform, &build)), None);
        }
        assert_eq!(parse_update_asset(&commit_asset(&build)), None);
    }

    #[test]
    fn other_names_are_not_commit_assets() {
        for name in [
            "ev-nova-2026102812345-commit.txt",
            "ev-nova-20261328123456-commit.txt",
            "ev-nova-20261028123456-commit.text",
            "nova-20261028123456-commit.txt",
            "ev-nova-commit.txt",
            "appcast-macos-nightly.xml",
            "",
        ] {
            assert_eq!(parse_commit_asset(name), None, "{name}");
        }
    }
}
