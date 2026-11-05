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

/// Where content comes from.
pub trait Source {
    /// The files directly inside the folder `dir` of the data folder whose
    /// names end in `.ext`, sorted by path. A missing folder has none.
    fn files(&self, dir: &str, ext: &str) -> io::Result<Vec<SourceFile>>;

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
        let prefix = format!("{dir}/");
        let suffix = format!(".{ext}");
        self.files
            .iter()
            .filter(|(path, _)| {
                path.strip_prefix(&prefix)
                    .is_some_and(|name| !name.contains('/') && name.ends_with(&suffix))
            })
            .map(|(path, bytes)| {
                let text = String::from_utf8(bytes.clone())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Ok(SourceFile {
                    path: path.clone(),
                    text,
                })
            })
            .collect()
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
        let entries = match std::fs::read_dir(self.root.join(dir)) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut files = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_file() && path.extension().is_some_and(|e| e == ext) {
                let name = entry.file_name().to_string_lossy().into_owned();
                files.push(SourceFile {
                    path: format!("{dir}/{name}"),
                    text: std::fs::read_to_string(&path)?,
                });
            }
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
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

#[cfg(test)]
mod tests {
    use super::*;

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
