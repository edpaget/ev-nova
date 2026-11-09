//! The port the loader reads content through, with an in-memory fake for
//! tests and a filesystem adapter for the CLI.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

/// A content file: its path relative to the data folder, always written
/// with `/`, and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    /// e.g. `systems/marrow.md`.
    pub path: String,
    /// The file's contents.
    pub text: String,
}

/// A file [`Source::listing`] found: its path, as [`SourceFile`] writes
/// it, and its text or why it couldn't be read.
#[derive(Debug)]
pub struct Listed {
    /// e.g. `systems/marrow.md`, always written with `/`.
    pub path: String,
    /// The file's contents, or the error reading them.
    pub text: io::Result<String>,
}

/// Where content comes from.
pub trait Source {
    /// The files directly inside the folder `dir` of the data folder whose
    /// names end in `.ext`, sorted by path. A missing folder has none.
    fn files(&self, dir: &str, ext: &str) -> io::Result<Vec<SourceFile>>;

    /// Like [`files`](Self::files), but a file that can't be read is
    /// listed with its error instead of failing the folder. `Err` only
    /// when the folder itself can't be listed.
    ///
    /// The default is built from `files`, so a source that doesn't
    /// override it fails the folder on its first unreadable file.
    fn listing(&self, dir: &str, ext: &str) -> io::Result<Vec<Listed>> {
        Ok(self
            .files(dir, ext)?
            .into_iter()
            .map(|f| Listed {
                path: f.path,
                text: Ok(f.text),
            })
            .collect())
    }

    /// The bytes of the file at `path` (relative to the data folder,
    /// written with `/`), or `None` if there is no such file. Media files
    /// are read this way.
    fn bytes(&self, path: &str) -> io::Result<Option<Vec<u8>>>;
}

/// Content held in memory, keyed by relative path.
#[derive(Clone, Debug, Default)]
pub struct MemSource {
    files: BTreeMap<String, Vec<u8>>,
}

impl MemSource {
    /// Adds (or replaces) the text file at `path`.
    #[must_use]
    pub fn with(self, path: &str, text: &str) -> Self {
        self.with_bytes(path, text.as_bytes())
    }

    /// Adds (or replaces) the file at `path`, holding `bytes`.
    #[must_use]
    pub fn with_bytes(mut self, path: &str, bytes: &[u8]) -> Self {
        self.files.insert(path.to_string(), bytes.to_vec());
        self
    }

    /// Removes the file at `path`.
    #[must_use]
    pub fn without(mut self, path: &str) -> Self {
        self.files.remove(path);
        self
    }

    /// Every file as `(path, bytes)`, sorted by path.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
    }
}

impl Source for MemSource {
    fn files(&self, dir: &str, ext: &str) -> io::Result<Vec<SourceFile>> {
        texts(self.listing(dir, ext)?)
    }

    fn listing(&self, dir: &str, ext: &str) -> io::Result<Vec<Listed>> {
        let prefix = format!("{dir}/");
        let suffix = format!(".{ext}");
        Ok(self
            .files
            .iter()
            .filter(|(path, _)| {
                path.strip_prefix(&prefix)
                    .is_some_and(|name| !name.contains('/') && name.ends_with(&suffix))
            })
            .map(|(path, bytes)| Listed {
                path: path.clone(),
                text: String::from_utf8(bytes.clone())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            })
            .collect())
    }

    fn bytes(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
        Ok(self.files.get(path).cloned())
    }
}

/// Content in a folder on disk.
#[derive(Clone, Debug)]
pub struct DirSource {
    root: PathBuf,
}

impl DirSource {
    /// Reads content from the data folder at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The data folder.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Source for DirSource {
    fn files(&self, dir: &str, ext: &str) -> io::Result<Vec<SourceFile>> {
        texts(self.listing(dir, ext)?)
    }

    /// An entry that can't be read, or whose type can't be read, is listed
    /// with its error once its name has the extension. Only a folder that
    /// can't be listed, or an entry without a name, fails the folder.
    fn listing(&self, dir: &str, ext: &str) -> io::Result<Vec<Listed>> {
        let entries = match std::fs::read_dir(self.root.join(dir)) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut listed = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_none_or(|e| e != ext) {
                continue;
            }
            let text = match entry.file_type() {
                Ok(kind) if !kind.is_file() => continue,
                Ok(_) => std::fs::read_to_string(&path),
                Err(e) => Err(e),
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            listed.push(Listed {
                path: format!("{dir}/{name}"),
                text,
            });
        }
        listed.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(listed)
    }

    fn bytes(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
        let mut file = self.root.clone();
        file.extend(path.split('/'));
        match std::fs::read(file) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// `listed`'s files with their texts, or the first one's error.
fn texts(listed: Vec<Listed>) -> io::Result<Vec<SourceFile>> {
    listed
        .into_iter()
        .map(|f| {
            Ok(SourceFile {
                path: f.path,
                text: f.text?,
            })
        })
        .collect()
}

#[cfg(any(test, feature = "fixture"))]
/// The tree every adapter is checked against: `(path, text)` pairs.
pub const TREE: &[(&str, &str)] = &[
    ("systems/marrow.md", "m"),
    ("systems/brightwater.md", "b"),
    ("systems/notes.txt", "ignored: wrong extension"),
    ("systems/old/ignored.md", "ignored: nested"),
    ("dialog/holding-the-point.ink", "ink"),
    ("media/export/pack/hull.glb", "glTF\u{2}"),
];

/// Checks that `source` returns what every adapter must for [`TREE`]
/// (written into it by the caller), panicking if not. Test support for
/// this crate's adapters and for adapters in other crates.
#[cfg(any(test, feature = "fixture"))]
pub fn check_contract(source: &impl Source) {
    let systems = source.files("systems", "md").expect("lists systems/");
    let paths: Vec<_> = systems.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["systems/brightwater.md", "systems/marrow.md"]);
    assert_eq!(systems[1].text, "m");
    assert_eq!(
        source.files("dialog", "ink").expect("lists dialog/")[0].path,
        "dialog/holding-the-point.ink"
    );
    assert!(
        source
            .files("missions", "md")
            .expect("lists a missing folder")
            .is_empty(),
        "a missing folder is empty"
    );
    assert_eq!(
        source
            .bytes("media/export/pack/hull.glb")
            .expect("reads a file"),
        Some(b"glTF\x02".to_vec()),
        "a nested file reads as bytes"
    );
    assert_eq!(
        source
            .bytes("media/export/pack/gone.glb")
            .expect("reads a missing file"),
        None,
        "a missing file is None"
    );
}

/// A file every adapter must list with its error rather than fail its
/// folder over: `(path, bytes)` pairs, written into a source alongside
/// [`TREE`]. Its bytes aren't UTF-8.
#[cfg(any(test, feature = "fixture"))]
pub const UNREADABLE: &[(&str, &[u8])] = &[("systems/garbled.md", b"---\xff\xfe\n")];

/// Checks that `source`, holding [`TREE`] and [`UNREADABLE`] (written into
/// it by the caller), lists the unreadable file with its error and the
/// rest with their texts, panicking if not. Test support for this crate's
/// adapters and for adapters in other crates.
#[cfg(any(test, feature = "fixture"))]
pub fn check_unreadable_contract(source: &impl Source) {
    let listed = source.listing("systems", "md").expect("lists systems/");
    let paths: Vec<_> = listed.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "systems/brightwater.md",
            "systems/garbled.md",
            "systems/marrow.md"
        ]
    );
    assert_eq!(listed[0].text.as_deref().ok(), Some("b"));
    assert_eq!(listed[2].text.as_deref().ok(), Some("m"));
    assert_eq!(
        listed[1].text.as_ref().map_err(io::Error::kind).err(),
        Some(io::ErrorKind::InvalidData),
        "the garbled file is listed with its error"
    );
    assert!(
        source.files("systems", "md").is_err(),
        "files still fails the folder"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_lists_its_entries_and_drops_a_file_without_the_rest() {
        let fake = MemSource::default()
            .with("b.md", "b")
            .with_bytes("a.bin", b"\x01")
            .with("c.md", "c")
            .without("c.md");
        let entries: Vec<_> = fake.entries().collect();
        assert_eq!(entries, [("a.bin", &b"\x01"[..]), ("b.md", &b"b"[..])]);
    }

    #[test]
    fn the_directory_adapter_reports_errors_other_than_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("systems"), "a file, not a folder").unwrap();
        std::fs::create_dir(dir.path().join("media")).unwrap();
        let source = DirSource::new(dir.path());
        assert!(source.files("systems", "md").is_err(), "not a folder");
        assert!(source.bytes("media").is_err(), "a folder, not a file");
    }

    #[test]
    fn the_fake_meets_the_contract() {
        let mut fake = MemSource::default();
        for (path, text) in TREE {
            fake = fake.with(path, text);
        }
        check_contract(&fake);
    }

    #[test]
    #[should_panic(expected = "assertion")]
    fn the_contract_fails_a_source_that_breaks_it() {
        check_contract(&MemSource::default());
    }

    /// Writes `bytes` at the relative `path` under `root`.
    fn write(root: &Path, path: &str, bytes: &[u8]) {
        let mut file = root.to_path_buf();
        file.extend(path.split('/'));
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, bytes).unwrap();
    }

    #[test]
    fn the_fake_meets_the_unreadable_contract() {
        let mut fake = MemSource::default();
        for (path, text) in TREE {
            fake = fake.with(path, text);
        }
        for (path, bytes) in UNREADABLE {
            fake = fake.with_bytes(path, bytes);
        }
        check_unreadable_contract(&fake);
    }

    #[test]
    fn the_directory_adapter_meets_the_unreadable_contract() {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in TREE {
            write(dir.path(), path, text.as_bytes());
        }
        for (path, bytes) in UNREADABLE {
            write(dir.path(), path, bytes);
        }
        check_unreadable_contract(&DirSource::new(dir.path()));
    }

    /// A source keeping only the default `listing`, which fails the
    /// folder over one unreadable file.
    struct FilesOnly(MemSource);
    impl Source for FilesOnly {
        fn files(&self, dir: &str, ext: &str) -> io::Result<Vec<SourceFile>> {
            self.0.files(dir, ext)
        }
        fn bytes(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
            self.0.bytes(path)
        }
    }

    #[test]
    #[should_panic(expected = "lists systems/")]
    fn the_unreadable_contract_fails_a_source_that_breaks_it() {
        let mut fake = MemSource::default();
        for (path, text) in TREE {
            fake = fake.with(path, text);
        }
        for (path, bytes) in UNREADABLE {
            fake = fake.with_bytes(path, bytes);
        }
        check_unreadable_contract(&FilesOnly(fake));
    }

    #[test]
    fn the_directory_adapter_meets_the_contract() {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in TREE {
            let mut file = dir.path().to_path_buf();
            file.extend(path.split('/'));
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        }
        check_contract(&DirSource::new(dir.path()));
    }
}
