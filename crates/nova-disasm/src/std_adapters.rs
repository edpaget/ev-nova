//! The real adapters: the filesystem, `otool` and `nm`, the environment
//! and git. They stay thin; the decisions are in the core.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ports::{Binary, Cache, Env, Listing, Listings, ToolError, Tools, Workspace};

/// Reads the binary from disk.
pub struct StdBinary;

impl Binary for StdBinary {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }
}

/// Runs `otool` and `nm` for the i386 slice.
pub struct StdTools {
    /// The `otool` program.
    pub otool: OsString,
    /// The `nm` program.
    pub nm: OsString,
}

impl Default for StdTools {
    fn default() -> StdTools {
        StdTools {
            otool: "otool".into(),
            nm: "nm".into(),
        }
    }
}

/// The arguments before the binary's path, for each listing.
fn arguments(which: Listing) -> &'static [&'static str] {
    match which {
        Listing::Text => &["-arch", "i386", "-tV"],
        Listing::CStrings => &["-arch", "i386", "-v", "-s", "__TEXT", "__cstring"],
        Listing::Pointers => &["-arch", "i386", "-s", "__IMPORT", "__pointers"],
        Listing::Indirect => &["-arch", "i386", "-Iv"],
        Listing::Symbols => &["-arch", "i386", "-n"],
        Listing::Sections => &["-arch", "i386", "-l"],
    }
}

/// The tools print the binary's strings as raw (`MacRoman`) bytes: one char
/// per byte keeps a string's length in bytes equal to its length in chars.
fn one_char_per_byte(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

impl Tools for StdTools {
    fn listing(&self, binary: &Path, which: Listing) -> Result<String, ToolError> {
        let program = match which {
            Listing::Symbols => &self.nm,
            _ => &self.otool,
        };
        let name = program.to_string_lossy().into_owned();
        let output = Command::new(program)
            .args(arguments(which))
            .arg(binary)
            .output()
            .map_err(|source| match source.kind() {
                io::ErrorKind::NotFound => ToolError::Missing {
                    program: name.clone(),
                },
                _ => ToolError::Io {
                    program: name.clone(),
                    source,
                },
            })?;
        if !output.status.success() {
            return Err(ToolError::Failed {
                program: name,
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }
        Ok(one_char_per_byte(&output.stdout))
    }
}

/// The cache on disk: one directory per key, one file per listing.
pub struct FsCache {
    dir: PathBuf,
}

impl FsCache {
    /// A cache under `dir` (created on the first store).
    #[must_use]
    pub fn new(dir: PathBuf) -> FsCache {
        FsCache { dir }
    }

    fn file(entry: &Path, which: Listing) -> PathBuf {
        entry.join(format!("{}.txt", which.name()))
    }
}

impl Cache for FsCache {
    fn get(&self, key: &str) -> Option<Listings> {
        let entry = self.dir.join(key);
        let mut listings = Listings::default();
        for which in Listing::ALL {
            *listings.get_mut(which) = fs::read_to_string(Self::file(&entry, which)).ok()?;
        }
        Some(listings)
    }

    /// Writes the entry beside its final name and renames it into place,
    /// so a reader never sees half of one.
    fn put(&self, key: &str, listings: &Listings) -> io::Result<()> {
        let entry = self.dir.join(key);
        let staging = self.dir.join(format!("{key}.tmp-{}", std::process::id()));
        fs::create_dir_all(&staging)?;
        let written = Listing::ALL
            .iter()
            .try_for_each(|&which| fs::write(Self::file(&staging, which), listings.get(which)))
            .and_then(|()| {
                fs::rename(&staging, &entry).or_else(|err| {
                    // An entry missing a listing (stored by a build that had
                    // fewer) is never read: put this one in its place.
                    if entry.is_dir() && self.get(key).is_none() {
                        fs::remove_dir_all(&entry)?;
                        fs::rename(&staging, &entry)
                    } else {
                        Err(err)
                    }
                })
            });
        if written.is_err() {
            let _ = fs::remove_dir_all(&staging);
            // Another run stored the same key first: that entry is as good.
            if self.get(key).is_some() {
                return Ok(());
            }
        }
        written
    }
}

/// The process environment.
pub struct StdEnv;

impl Env for StdEnv {
    fn var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }
}

/// The checkouts, found through git from the workspace's root.
pub struct StdWorkspace {
    root: PathBuf,
}

impl StdWorkspace {
    /// The workspace this crate was built from.
    #[must_use]
    pub fn from_build() -> StdWorkspace {
        StdWorkspace::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(".."))
    }

    /// A workspace rooted at `root`.
    #[must_use]
    pub fn at(root: PathBuf) -> StdWorkspace {
        StdWorkspace { root }
    }
}

impl Workspace for StdWorkspace {
    /// The parent of `git rev-parse --git-common-dir`: worktrees share the
    /// primary checkout's `.git`.
    fn primary_checkout(&self) -> Option<PathBuf> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
            // A hook's GIT_DIR would point git elsewhere than `root`.
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let common = String::from_utf8(output.stdout).ok()?;
        Path::new(common.trim()).parent().map(Path::to_path_buf)
    }

    fn workspace_root(&self) -> PathBuf {
        self.root.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listings() -> Listings {
        let mut listings = Listings::default();
        for which in Listing::ALL {
            *listings.get_mut(which) = format!("{} \u{e9}\n", which.name());
        }
        listings
    }

    #[test]
    fn the_fs_cache_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let cache = FsCache::new(dir.path().join("disasm"));
        cache.put("k1", &listings()).unwrap();
        assert_eq!(cache.get("k1"), Some(listings()));
        let names: Vec<_> = fs::read_dir(dir.path().join("disasm"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["k1"], "no staging directory is left behind");
    }

    #[test]
    fn a_missing_key_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(FsCache::new(dir.path().to_path_buf()).get("k1"), None);
    }

    #[test]
    fn an_entry_missing_a_file_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let cache = FsCache::new(dir.path().to_path_buf());
        cache.put("k1", &listings()).unwrap();
        fs::remove_file(dir.path().join("k1").join("nm.txt")).unwrap();
        assert_eq!(cache.get("k1"), None);
    }

    #[test]
    fn an_entry_from_before_a_listing_was_added_is_none_and_replaced() {
        // An older build cached five listings, without sections.txt: the
        // load lists them all again and stores them over it.
        let dir = tempfile::tempdir().unwrap();
        let cache = FsCache::new(dir.path().to_path_buf());
        cache.put("k1", &listings()).unwrap();
        fs::remove_file(dir.path().join("k1").join("sections.txt")).unwrap();
        assert_eq!(cache.get("k1"), None);
        cache.put("k1", &listings()).unwrap();
        assert_eq!(cache.get("k1"), Some(listings()));
        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["k1"], "no staging directory is left behind");
    }

    #[test]
    fn storing_a_key_already_stored_keeps_it() {
        let dir = tempfile::tempdir().unwrap();
        let cache = FsCache::new(dir.path().to_path_buf());
        cache.put("k1", &listings()).unwrap();
        let mut other = listings();
        other.text = "another run's text".to_owned();
        cache.put("k1", &other).unwrap();
        assert_eq!(cache.get("k1"), Some(listings()));
    }

    #[test]
    fn each_byte_becomes_one_char() {
        let text = one_char_per_byte(b"a\xa5b");
        assert_eq!(text.chars().count(), 3);
        assert!(text.starts_with('a') && text.ends_with('b'));
    }

    #[test]
    fn every_listing_reads_the_i386_slice() {
        for which in Listing::ALL {
            assert_eq!(arguments(which)[..2], ["-arch", "i386"], "{which:?}");
        }
        assert_eq!(arguments(Listing::Text)[2..], ["-tV"]);
        assert_eq!(arguments(Listing::Symbols)[2..], ["-n"]);
        assert_eq!(arguments(Listing::Sections)[2..], ["-l"]);
    }

    #[test]
    fn a_tool_that_is_not_installed_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("no-such-otool");
        let tools = StdTools {
            otool: absent.clone().into(),
            nm: absent.into(),
        };
        let err = tools
            .listing(&dir.path().join("bin"), Listing::Text)
            .unwrap_err();
        assert!(matches!(err, ToolError::Missing { .. }), "{err:?}");
    }

    #[test]
    fn the_symbols_come_from_nm_and_the_rest_from_otool() {
        let dir = tempfile::tempdir().unwrap();
        let otool = dir.path().join("no-such-otool");
        let nm = dir.path().join("no-such-nm");
        let tools = StdTools {
            otool: otool.clone().into(),
            nm: nm.clone().into(),
        };
        let program = |which| match tools.listing(&dir.path().join("bin"), which) {
            Err(ToolError::Missing { program }) => program,
            other => panic!("{other:?}"),
        };
        assert_eq!(program(Listing::Symbols), nm.to_string_lossy());
        for which in [
            Listing::Text,
            Listing::CStrings,
            Listing::Pointers,
            Listing::Indirect,
            Listing::Sections,
        ] {
            assert_eq!(program(which), otool.to_string_lossy(), "{which:?}");
        }
    }

    #[test]
    fn a_tool_that_exits_with_an_error_fails() {
        // git rejects otool's arguments (`-arch`) and exits non-zero.
        let dir = tempfile::tempdir().unwrap();
        let tools = StdTools {
            otool: "git".into(),
            nm: "git".into(),
        };
        let err = tools
            .listing(&dir.path().join("bin"), Listing::Text)
            .unwrap_err();
        assert!(
            matches!(&err, ToolError::Failed { program, .. } if program == "git"),
            "{err:?}"
        );
    }

    #[test]
    fn the_std_env_reads_the_process_environment() {
        // Cargo sets the package's manifest directory for the tests it runs.
        assert_eq!(
            StdEnv.var("CARGO_MANIFEST_DIR"),
            Some(OsString::from(env!("CARGO_MANIFEST_DIR")))
        );
    }

    #[test]
    fn the_std_binary_reports_a_missing_file_as_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let err = StdBinary.read(&dir.path().join("EV Nova")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn a_directory_outside_git_has_no_primary_checkout() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = StdWorkspace::at(dir.path().to_path_buf());
        assert_eq!(workspace.primary_checkout(), None);
        assert_eq!(workspace.workspace_root(), dir.path());
    }

    /// Runs git in `dir`, away from any hook's repository and the user's
    /// config.
    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", dir.join("no-global-config"))
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    #[test]
    fn a_worktree_s_primary_checkout_is_the_repository_it_came_from() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let worktree = dir.path().join("wt");
        fs::create_dir(&repo).unwrap();
        git(&repo, &["init", "--quiet"]);
        git(&repo, &["commit", "--quiet", "--allow-empty", "-m", "x"]);
        git(
            &repo,
            &["worktree", "add", "--quiet", worktree.to_str().unwrap()],
        );
        let primary = StdWorkspace::at(worktree.clone())
            .primary_checkout()
            .unwrap()
            .canonicalize()
            .unwrap();
        assert_eq!(primary, repo.canonicalize().unwrap());
        assert_ne!(primary, worktree.canonicalize().unwrap());
    }

    #[test]
    fn a_repository_is_its_own_primary_checkout() {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "--quiet"]);
        let primary = StdWorkspace::at(dir.path().to_path_buf())
            .primary_checkout()
            .unwrap();
        assert_eq!(
            primary.canonicalize().unwrap(),
            dir.path().canonicalize().unwrap()
        );
    }
}
