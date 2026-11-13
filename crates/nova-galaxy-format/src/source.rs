//! The port the loader reads content through, with an in-memory fake for
//! tests and a filesystem adapter for the CLI.
//!
//! The loader reads each kind's folder with [`Source::listing`], which
//! lists a file that can't be read with its error ([`Listed`]), so one bad
//! file drops only itself. [`Source::files`] keeps failing the whole
//! folder on its first such file, for callers that want that. `listing`
//! has a default built from `files`, so an adapter that doesn't override
//! it still compiles, but fails the folder as `files` does.
//!
//! A media file is read whole with [`Source::bytes`], or only as far as
//! its first bytes with [`Source::head`], for a caller that needs to know
//! how it starts and no more (the LFS check). `head` has a default cut
//! from `bytes`, so an adapter that doesn't override it still compiles,
//! but reads the file whole. So has [`Source::size`], a file's length in
//! bytes for a caller that holds it to a budget without reading it (a
//! texture checked from its header); the default reads it whole and
//! counts it.
//!
//! A borrowed source (`&S`) and a shared one (`Rc<S>`) are sources too,
//! answering with the source's own methods, so a caller can keep a source
//! another part of the program reads as well.
//!
//! Every adapter is checked against `TREE` with `check_contract`, and one
//! that lists per file against `TREE` plus `UNREADABLE` with
//! `check_unreadable_contract`. `CountingSource` wraps a source, borrowed
//! or its own, and counts the files asked of it whole, by their heads and
//! by their sizes, for tests that a file is read once (all behind the
//! `fixture` feature).

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

    /// The first `len` bytes of the file at `path` (the whole of a file
    /// shorter than that), or `None` if there is no such file: for a
    /// caller that only needs to know how a file starts, such as the LFS
    /// check ([`crate::media::lfs_pointers`]).
    ///
    /// The default reads the file whole with [`bytes`](Self::bytes) and
    /// cuts it, so a source that doesn't override it still compiles; an
    /// adapter over real files overrides it to read no more than `len`.
    fn head(&self, path: &str, len: usize) -> io::Result<Option<Vec<u8>>> {
        Ok(self.bytes(path)?.map(|mut bytes| {
            bytes.truncate(len);
            bytes
        }))
    }

    /// The length in bytes of the file at `path`, or `None` if there is no
    /// such file: for a caller that holds a file to a budget of bytes on
    /// disk without reading it (a texture checked from its header).
    ///
    /// The default reads the file whole with [`bytes`](Self::bytes) and
    /// counts it, so a source that doesn't override it still compiles; an
    /// adapter over real files overrides it to read none of it.
    fn size(&self, path: &str) -> io::Result<Option<u64>> {
        Ok(self.bytes(path)?.map(|bytes| bytes.len() as u64))
    }
}

/// A borrowed source is a source, answering with its own methods.
impl<S: Source + ?Sized> Source for &S {
    fn files(&self, dir: &str, ext: &str) -> io::Result<Vec<SourceFile>> {
        (**self).files(dir, ext)
    }

    fn listing(&self, dir: &str, ext: &str) -> io::Result<Vec<Listed>> {
        (**self).listing(dir, ext)
    }

    fn bytes(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
        (**self).bytes(path)
    }

    fn head(&self, path: &str, len: usize) -> io::Result<Option<Vec<u8>>> {
        (**self).head(path, len)
    }

    fn size(&self, path: &str) -> io::Result<Option<u64>> {
        (**self).size(path)
    }
}

/// A shared source is a source, answering with its own methods.
impl<S: Source + ?Sized> Source for std::rc::Rc<S> {
    fn files(&self, dir: &str, ext: &str) -> io::Result<Vec<SourceFile>> {
        (**self).files(dir, ext)
    }

    fn listing(&self, dir: &str, ext: &str) -> io::Result<Vec<Listed>> {
        (**self).listing(dir, ext)
    }

    fn bytes(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
        (**self).bytes(path)
    }

    fn head(&self, path: &str, len: usize) -> io::Result<Option<Vec<u8>>> {
        (**self).head(path, len)
    }

    fn size(&self, path: &str) -> io::Result<Option<u64>> {
        (**self).size(path)
    }
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
        absent_is_none(std::fs::read(self.file(path)))
    }

    /// Reads no more than `len` bytes of the file.
    fn head(&self, path: &str, len: usize) -> io::Result<Option<Vec<u8>>> {
        use std::io::Read;
        let read = std::fs::File::open(self.file(path)).and_then(|file| {
            let mut head = Vec::new();
            file.take(len as u64).read_to_end(&mut head)?;
            Ok(head)
        });
        absent_is_none(read)
    }

    /// Reads none of the file: its length is its metadata's, following a
    /// link as reading it would.
    fn size(&self, path: &str) -> io::Result<Option<u64>> {
        match std::fs::metadata(self.file(path)) {
            Ok(meta) if meta.is_dir() => Err(io::Error::new(
                io::ErrorKind::IsADirectory,
                format!("`{path}` is a folder, not a file"),
            )),
            Ok(meta) => Ok(Some(meta.len())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl DirSource {
    /// Where the file at the relative `path` is on disk.
    fn file(&self, path: &str) -> PathBuf {
        let mut file = self.root.clone();
        file.extend(path.split('/'));
        file
    }
}

/// `read`, with a file that isn't there as `None`.
fn absent_is_none(read: io::Result<Vec<u8>>) -> io::Result<Option<Vec<u8>>> {
    match read {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
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

/// A [`Source`] that answers as the source it wraps and counts, by path,
/// how often each file was asked for whole ([`Source::bytes`]) and by its
/// head ([`Source::head`]): for a test that a file is read once, or not
/// at all. Listings aren't counted.
#[cfg(any(test, feature = "fixture"))]
#[derive(Debug)]
pub struct CountingSource<S> {
    source: S,
    bytes: std::cell::RefCell<BTreeMap<String, u32>>,
    heads: std::cell::RefCell<BTreeMap<String, u32>>,
    sizes: std::cell::RefCell<BTreeMap<String, u32>>,
}

#[cfg(any(test, feature = "fixture"))]
impl<S: Source> CountingSource<S> {
    /// `source`, with nothing asked of it yet: a borrowed one (`&source`),
    /// or one it owns, which a test can share by wrapping the counter in
    /// an `Rc`.
    pub fn new(source: S) -> Self {
        CountingSource {
            source,
            bytes: std::cell::RefCell::default(),
            heads: std::cell::RefCell::default(),
            sizes: std::cell::RefCell::default(),
        }
    }

    /// How often the size of the file at `path` was asked for.
    pub fn sizes_asked(&self, path: &str) -> u32 {
        self.sizes.borrow().get(path).copied().unwrap_or(0)
    }

    /// How often the file at `path` was asked for whole.
    pub fn bytes_asked(&self, path: &str) -> u32 {
        self.bytes.borrow().get(path).copied().unwrap_or(0)
    }

    /// How often the head of the file at `path` was asked for.
    pub fn heads_asked(&self, path: &str) -> u32 {
        self.heads.borrow().get(path).copied().unwrap_or(0)
    }

    /// Every path asked for whole, with how often, sorted by path.
    pub fn all_bytes_asked(&self) -> Vec<(String, u32)> {
        let asked = self.bytes.borrow();
        asked.iter().map(|(path, &n)| (path.clone(), n)).collect()
    }
}

#[cfg(any(test, feature = "fixture"))]
impl<S: Source> Source for CountingSource<S> {
    fn files(&self, dir: &str, ext: &str) -> io::Result<Vec<SourceFile>> {
        self.source.files(dir, ext)
    }

    fn listing(&self, dir: &str, ext: &str) -> io::Result<Vec<Listed>> {
        self.source.listing(dir, ext)
    }

    fn bytes(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
        *self.bytes.borrow_mut().entry(path.to_owned()).or_default() += 1;
        self.source.bytes(path)
    }

    fn head(&self, path: &str, len: usize) -> io::Result<Option<Vec<u8>>> {
        *self.heads.borrow_mut().entry(path.to_owned()).or_default() += 1;
        self.source.head(path, len)
    }

    fn size(&self, path: &str) -> io::Result<Option<u64>> {
        *self.sizes.borrow_mut().entry(path.to_owned()).or_default() += 1;
        self.source.size(path)
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
    assert_eq!(
        source
            .head("media/export/pack/hull.glb", 2)
            .expect("reads a file's head"),
        Some(b"gl".to_vec()),
        "a file's head is its first bytes"
    );
    assert_eq!(
        source
            .head("media/export/pack/hull.glb", 64)
            .expect("reads a short file's head"),
        Some(b"glTF\x02".to_vec()),
        "a file shorter than asked is its head whole"
    );
    assert_eq!(
        source
            .head("media/export/pack/gone.glb", 2)
            .expect("reads a missing file's head"),
        None,
        "a missing file has no head"
    );
    assert_eq!(
        source
            .size("media/export/pack/hull.glb")
            .expect("reads a file's size"),
        Some(5),
        "a file's size is its length in bytes"
    );
    assert_eq!(
        source
            .size("media/export/pack/gone.glb")
            .expect("reads a missing file's size"),
        None,
        "a missing file has no size"
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
        assert!(source.head("media", 4).is_err(), "nor has a folder a head");
        assert!(source.size("media").is_err(), "nor a size");
        // A name no filesystem takes is an error, not a missing file.
        assert!(source.size("media/bad\0name").is_err(), "a NUL in its name");
    }

    #[test]
    fn the_directory_adapter_reads_only_the_head_it_is_asked_for() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "media/export/pack/hull.glb",
            b"glTF\x02 and more",
        );
        let source = DirSource::new(dir.path());
        let head = |len| source.head("media/export/pack/hull.glb", len).unwrap();
        assert_eq!(head(0), Some(Vec::new()));
        assert_eq!(head(4), Some(b"glTF".to_vec()));
        assert_eq!(head(5), Some(b"glTF\x02".to_vec()));
    }

    #[test]
    fn a_source_with_only_the_required_methods_meets_the_contract() {
        let mut fake = MemSource::default();
        for (path, text) in TREE {
            fake = fake.with(path, text);
        }
        check_contract(&FilesOnly(fake));
    }

    #[test]
    fn the_counting_source_answers_as_its_source_and_counts_each_path_asked() {
        let mut fake = MemSource::default();
        for (path, text) in TREE {
            fake = fake.with(path, text);
        }
        let counting = CountingSource::new(&fake);
        check_contract(&counting);
        // The contract asks `hull.glb` whole once and for its head twice,
        // and `gone.glb` once each way.
        assert_eq!(counting.bytes_asked("media/export/pack/hull.glb"), 1);
        assert_eq!(counting.heads_asked("media/export/pack/hull.glb"), 2);
        assert_eq!(counting.bytes_asked("media/export/pack/gone.glb"), 1);
        assert_eq!(counting.heads_asked("media/export/pack/gone.glb"), 1);
        assert_eq!(counting.bytes_asked("systems/marrow.md"), 0, "listed");
        assert_eq!(counting.heads_asked("systems/marrow.md"), 0);
        assert_eq!(
            counting.all_bytes_asked(),
            [
                ("media/export/pack/gone.glb".to_owned(), 1),
                ("media/export/pack/hull.glb".to_owned(), 1)
            ]
        );
    }

    #[test]
    fn the_counting_source_lists_per_file_as_its_source_does() {
        let mut fake = MemSource::default();
        for (path, text) in TREE {
            fake = fake.with(path, text);
        }
        for (path, bytes) in UNREADABLE {
            fake = fake.with_bytes(path, bytes);
        }
        check_unreadable_contract(&CountingSource::new(&fake));
    }

    /// A source whose `head` and `size` are its own, not a cut or the
    /// length of its `bytes`.
    #[derive(Debug)]
    struct OwnHead;
    impl Source for OwnHead {
        fn files(&self, _: &str, _: &str) -> io::Result<Vec<SourceFile>> {
            Ok(Vec::new())
        }
        fn bytes(&self, _: &str) -> io::Result<Option<Vec<u8>>> {
            Ok(Some(b"whole".to_vec()))
        }
        fn head(&self, _: &str, _: usize) -> io::Result<Option<Vec<u8>>> {
            Ok(Some(b"own".to_vec()))
        }
        fn size(&self, _: &str) -> io::Result<Option<u64>> {
            Ok(Some(77))
        }
    }

    #[test]
    fn the_counting_source_asks_its_source_for_a_head_not_the_whole_file() {
        let counting = CountingSource::new(&OwnHead);
        assert_eq!(counting.head("any", 9).unwrap(), Some(b"own".to_vec()));
        assert_eq!(counting.bytes_asked("any"), 0);
        assert_eq!(counting.heads_asked("any"), 1);
    }

    #[test]
    fn the_counting_source_counts_sizes_and_asks_its_source_for_its_own() {
        let counting = CountingSource::new(&OwnHead);
        assert_eq!(counting.size("any").unwrap(), Some(77));
        assert_eq!(counting.size("any").unwrap(), Some(77));
        assert_eq!(counting.sizes_asked("any"), 2);
        assert_eq!(counting.sizes_asked("other"), 0);
        assert_eq!(counting.bytes_asked("any"), 0, "not read whole");
        assert_eq!(counting.heads_asked("any"), 0);
    }

    #[test]
    fn a_borrowed_or_shared_source_answers_with_its_source_s_own_head_and_size() {
        let borrowed = &OwnHead;
        let answers = |source: &dyn Source| {
            (
                source.head("any", 9).unwrap(),
                source.size("any").unwrap(),
                source.bytes("any").unwrap(),
            )
        };
        let own = (Some(b"own".to_vec()), Some(77), Some(b"whole".to_vec()));
        assert_eq!(answers(&borrowed), own);
        assert_eq!(answers(&std::rc::Rc::new(OwnHead)), own);
        // Listings go through too.
        let mut fake = MemSource::default();
        for (path, text) in TREE {
            fake = fake.with(path, text);
        }
        for (path, bytes) in UNREADABLE {
            fake = fake.with_bytes(path, bytes);
        }
        check_unreadable_contract(&&fake);
        check_unreadable_contract(&std::rc::Rc::new(fake));
    }

    #[test]
    fn a_counting_source_may_own_its_source() {
        let mut fake = MemSource::default();
        for (path, text) in TREE {
            fake = fake.with(path, text);
        }
        let counting = std::rc::Rc::new(CountingSource::new(fake));
        let reader = std::rc::Rc::clone(&counting);
        check_contract(&reader);
        assert_eq!(counting.bytes_asked("media/export/pack/hull.glb"), 1);
        assert_eq!(counting.sizes_asked("media/export/pack/hull.glb"), 1);
        assert_eq!(counting.sizes_asked("media/export/pack/gone.glb"), 1);
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

    #[test]
    fn the_directory_adapter_lists_no_folder_named_like_a_file() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "systems/marrow.md", b"m");
        std::fs::create_dir(dir.path().join("systems").join("old.md")).unwrap();
        let source = DirSource::new(dir.path());
        let listed = source.listing("systems", "md").expect("lists systems/");
        let paths: Vec<_> = listed.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["systems/marrow.md"]);
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
