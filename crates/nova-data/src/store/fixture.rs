//! Hand-written fakes of the store's ports, and the [`DirLister`] port's
//! contract.
//!
//! Available to this crate's tests and, through the `fixture` feature, to
//! other crates' tests. [`FakeTree`] is the one shared fake of both of the
//! store's ports: an in-memory directory tree that reads its files' forks
//! through an inner [`MemForks`]. [`check_contract`] states what every
//! [`DirLister`] must do, over the small tree that [`write_contract_tree`]
//! writes to disk and [`FakeTree::contract`] builds in memory. It runs
//! against [`FakeTree`] here and against
//! [`StdDirLister`](super::fs::StdDirLister) in `store::fs`'s tests.
//!
//! Most tests want one data file: [`one_file`] (or [`one_file_at`]) builds
//! that tree, [`FakeTree::load`] opens it as a [`GameData`], and
//! [`store_of`] does both for a list of resources.

use std::collections::{BTreeSet, HashMap};
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::fixture::forks::MemForks;
use nova_rsrc::{Fork, ForkReader, ResType};

use super::fs::{DirLister, EntryKind, Listing};
use super::order::finder_cmp;
use super::{GameData, OpenError};

/// Where [`one_file`] puts its file, and the data directory
/// [`FakeTree::load`] opens.
pub const DATA_DIR: &str = "/data";

/// The plug-ins directory [`FakeTree::load`] opens when a file is under it.
pub const PLUGINS_DIR: &str = "/plugins";

/// The name of [`one_file`]'s file.
pub const DATA_FILE: &str = "Nova Data";

/// A tree of one file, [`DATA_DIR`]`/`[`DATA_FILE`], holding the data fork
/// `fork`.
#[must_use]
pub fn one_file(fork: impl Into<Vec<u8>>) -> FakeTree {
    one_file_at(Path::new(DATA_DIR).join(DATA_FILE), fork)
}

/// A tree of one file at `path`, holding the data fork `fork`.
#[must_use]
pub fn one_file_at(path: impl AsRef<Path>, fork: impl Into<Vec<u8>>) -> FakeTree {
    FakeTree::new().file(path, fork)
}

/// A [`GameData`] of one file holding `resources` (type, ID, data), each
/// unnamed: [`one_file`] of their fork, [`loaded`](FakeTree::load).
///
/// # Panics
///
/// If the store doesn't open.
#[must_use]
pub fn store_of(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
    let fork = resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
            fork.resource(*ty, *id, None, data)
        })
        .build()
        .bytes;
    one_file(fork).load()
}

/// An in-memory directory tree, and the forks of its files.
///
/// - [`dir`](Self::dir) gives a directory an explicit listing: for an empty
///   directory, or an entry of a kind other than a file.
/// - [`file`](Self::file) adds a file holding a data fork. Every ancestor
///   of the file lists the next step down towards it (the file itself, or a
///   directory), merged with any explicit listing of that ancestor.
/// - [`unreadable`](Self::unreadable) and
///   [`no_data_fork`](Self::no_data_fork) add a file the same way, and
///   pass through to [`MemForks`]: its forks fail, or it has no data fork.
///
/// Each directory's listing is returned scrambled: put in Finder order and
/// then rotated left by one, so a listing of two or more entries never
/// comes back already sorted, whatever order a test gave its names in, and
/// the walk must sort every folder it lists. A directory that was never
/// added, explicitly or as a file's ancestor, fails to list with
/// [`io::ErrorKind::NotFound`], and listing a file fails, as `read_dir`
/// does. Every listed directory is recorded.
#[derive(Debug, Default)]
pub struct FakeTree {
    dirs: HashMap<PathBuf, Vec<Listing>>,
    files: BTreeSet<PathBuf>,
    forks: MemForks,
    listed: Mutex<Vec<PathBuf>>,
}

impl FakeTree {
    /// An empty tree: nothing lists.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds directory `dir` holding `entries` (name, kind).
    #[must_use]
    pub fn dir(mut self, dir: impl AsRef<Path>, entries: &[(&str, EntryKind)]) -> Self {
        let listings = entries
            .iter()
            .map(|(name, kind)| Listing {
                name: (*name).into(),
                kind: *kind,
            })
            .collect();
        self.dirs.insert(dir.as_ref().to_path_buf(), listings);
        self
    }

    /// Adds the file `path` holding the data fork `fork`.
    #[must_use]
    pub fn file(self, path: impl AsRef<Path>, fork: impl Into<Vec<u8>>) -> Self {
        let path = path.as_ref();
        self.with_file(path, |forks| forks.file(path, fork))
    }

    /// Adds the file `path`, whose forks both fail to read
    /// ([`MemForks::unreadable`]).
    #[must_use]
    pub fn unreadable(self, path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        self.with_file(path, |forks| forks.unreadable(path))
    }

    /// Adds the file `path`, which has no data fork
    /// ([`MemForks::no_data_fork`]).
    #[must_use]
    pub fn no_data_fork(self, path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        self.with_file(path, |forks| forks.no_data_fork(path))
    }

    /// The forks of the tree's files, with every read so far.
    #[must_use]
    pub fn forks(&self) -> &MemForks {
        &self.forks
    }

    /// The directories listed so far, in call order.
    ///
    /// # Panics
    ///
    /// If an earlier listing panicked while recording.
    #[must_use]
    pub fn listed(&self) -> Vec<PathBuf> {
        self.listed.lock().expect("listed").clone()
    }

    /// The contract's tree under `root`, in memory: what
    /// [`write_contract_tree`] writes to disk.
    #[must_use]
    pub fn contract(root: &Path) -> Self {
        Self::new()
            .dir(root, &[(CONTRACT_EMPTY, EntryKind::Dir)])
            .dir(root.join(CONTRACT_EMPTY), &[])
            .file(root.join(CONTRACT_FILE), CONTRACT_FILE)
            .file(
                root.join(CONTRACT_PLUGINS).join(CONTRACT_PATCH),
                CONTRACT_PATCH,
            )
    }

    /// Opens the tree as a store: [`DATA_DIR`], and [`PLUGINS_DIR`] too when
    /// a file is under it.
    ///
    /// # Panics
    ///
    /// If the store doesn't open.
    #[must_use]
    pub fn load(&self) -> GameData {
        let plugins = Path::new(PLUGINS_DIR);
        let plugins = self
            .files
            .iter()
            .any(|file| file.starts_with(plugins))
            .then_some(plugins);
        self.load_from(Path::new(DATA_DIR), plugins)
            .expect("the fake store opens")
    }

    /// Opens the tree as a store whose data directory is `data`, with the
    /// plug-ins directory `plugins`, if any.
    ///
    /// # Errors
    ///
    /// As [`GameData::load`].
    pub fn load_from(&self, data: &Path, plugins: Option<&Path>) -> Result<GameData, OpenError> {
        GameData::load(self, self, data, plugins)
    }

    fn with_file(mut self, path: &Path, add: impl FnOnce(MemForks) -> MemForks) -> Self {
        self.files.insert(path.to_path_buf());
        self.forks = add(std::mem::take(&mut self.forks));
        self
    }

    /// What `dir` lists because of the files under it.
    fn implied(&self, dir: &Path) -> Vec<Listing> {
        let mut implied: Vec<Listing> = Vec::new();
        for file in &self.files {
            let Ok(rest) = file.strip_prefix(dir) else {
                continue;
            };
            let mut steps = rest.components();
            let Some(next) = steps.next() else {
                continue;
            };
            let name = next.as_os_str().to_os_string();
            if implied.iter().any(|listing| listing.name == name) {
                continue;
            }
            let kind = if steps.next().is_some() {
                EntryKind::Dir
            } else {
                EntryKind::File
            };
            implied.push(Listing { name, kind });
        }
        implied
    }
}

impl DirLister for FakeTree {
    fn list(&self, dir: &Path) -> io::Result<Vec<Listing>> {
        self.listed.lock().expect("listed").push(dir.to_path_buf());
        if self.files.contains(dir) {
            return Err(io::Error::other(format!(
                "{} is a fake file, not a directory",
                dir.display()
            )));
        }
        let explicit = self.dirs.get(dir);
        let implied = self.implied(dir);
        if explicit.is_none() && implied.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no fake directory at {}", dir.display()),
            ));
        }
        let mut listings = explicit.cloned().unwrap_or_default();
        for listing in implied {
            if !listings.iter().any(|known| known.name == listing.name) {
                listings.push(listing);
            }
        }
        listings.sort_by(|a, b| finder_cmp(&a.name, &b.name));
        if !listings.is_empty() {
            listings.rotate_left(1);
        }
        Ok(listings)
    }
}

impl ForkReader for FakeTree {
    fn read_fork(&self, path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        self.forks.read_fork(path, fork)
    }
}

/// The contract tree's file at the root.
const CONTRACT_FILE: &str = "Nova Data";
/// The contract tree's folder holding one file, [`CONTRACT_PATCH`].
const CONTRACT_PLUGINS: &str = "Nova Plug-ins";
/// The file in [`CONTRACT_PLUGINS`].
const CONTRACT_PATCH: &str = "Patch";
/// The contract tree's empty folder.
const CONTRACT_EMPTY: &str = "Empty";
/// A name the contract tree never holds.
const CONTRACT_MISSING: &str = "Missing";

/// Writes the contract's tree under `root`: a file `Nova Data`, a folder
/// `Nova Plug-ins` holding a file `Patch`, and an empty folder `Empty`. It
/// writes nothing outside `root`.
///
/// # Errors
///
/// When a file or folder can't be created.
pub fn write_contract_tree(root: &Path) -> io::Result<()> {
    std::fs::write(root.join(CONTRACT_FILE), CONTRACT_FILE)?;
    std::fs::create_dir(root.join(CONTRACT_PLUGINS))?;
    std::fs::write(
        root.join(CONTRACT_PLUGINS).join(CONTRACT_PATCH),
        CONTRACT_PATCH,
    )?;
    std::fs::create_dir(root.join(CONTRACT_EMPTY))
}

/// Checks the [`DirLister`] port's contract against `lister`, over the
/// tree that [`write_contract_tree`] (or [`FakeTree::contract`]) put under
/// `root`:
///
/// 1. `root` lists exactly `Nova Data` (a file), `Nova Plug-ins` and
///    `Empty` (folders), in any order, without `.` or `..`.
/// 2. `Nova Plug-ins` lists exactly `Patch` (a file), and `Empty` lists
///    nothing.
/// 3. A missing folder fails with [`io::ErrorKind::NotFound`].
/// 4. Listing a file fails, with an error of any kind.
/// 5. A second listing of `root` gives the same entries.
///
/// It doesn't cover [`EntryKind::Symlink`] or [`EntryKind::Other`]: making
/// either needs privileges on Windows. `StdDirLister`'s own unix-only tests
/// cover them.
///
/// # Errors
///
/// A message naming the first clause `lister` breaks.
pub fn check_contract(lister: &impl DirLister, root: &Path) -> Result<(), String> {
    let entries_of = |dir: &Path, what: &str| {
        let mut entries: Vec<(OsString, EntryKind)> = lister
            .list(dir)
            .map_err(|error| format!("{what} fails to list: {error}"))?
            .into_iter()
            .map(|listing| (listing.name, listing.kind))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Ok::<_, String>(entries)
    };
    let expected = |entries: &[(&str, EntryKind)]| -> Vec<(OsString, EntryKind)> {
        entries
            .iter()
            .map(|(name, kind)| (OsString::from(name), *kind))
            .collect()
    };
    let first = entries_of(root, "the root")?;
    let want = expected(&[
        (CONTRACT_EMPTY, EntryKind::Dir),
        (CONTRACT_FILE, EntryKind::File),
        (CONTRACT_PLUGINS, EntryKind::Dir),
    ]);
    if first != want {
        return Err(format!("the root lists {first:?}, not exactly {want:?}"));
    }
    let plugins = entries_of(&root.join(CONTRACT_PLUGINS), "a sub-folder")?;
    let want = expected(&[(CONTRACT_PATCH, EntryKind::File)]);
    if plugins != want {
        return Err(format!(
            "a sub-folder lists {plugins:?}, not exactly {want:?}"
        ));
    }
    let empty = entries_of(&root.join(CONTRACT_EMPTY), "an empty folder")?;
    if !empty.is_empty() {
        return Err(format!("an empty folder lists {empty:?}, not nothing"));
    }
    match lister.list(&root.join(CONTRACT_MISSING)) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        other => {
            return Err(format!(
                "a missing folder lists {other:?}, not an error of kind NotFound"
            ));
        }
    }
    if let Ok(entries) = lister.list(&root.join(CONTRACT_FILE)) {
        return Err(format!("listing a file gives {entries:?}, not an error"));
    }
    let second = entries_of(root, "the root, listed again,")?;
    if second != first {
        return Err(format!(
            "a second listing of the root gives {second:?}, not {first:?}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::Record;
    use crate::store::fs::EntryKind::{Dir, File, Symlink};

    fn names(tree: &FakeTree, dir: &str) -> Vec<OsString> {
        let listed = tree.list(Path::new(dir)).expect("lists");
        listed.into_iter().map(|listing| listing.name).collect()
    }

    #[test]
    fn listings_come_back_rotated_out_of_finder_order() {
        let tree = FakeTree::new()
            .dir("/sorted", &[("a", File), ("B", File), ("c", File)])
            .dir("/rotated", &[("B", File), ("c", File), ("a", File)])
            .dir("/one", &[("only", File)])
            .dir("/none", &[]);
        assert_eq!(names(&tree, "/sorted"), ["B", "c", "a"]);
        assert_eq!(names(&tree, "/rotated"), ["B", "c", "a"]);
        assert_eq!(names(&tree, "/one"), ["only"]);
        assert!(names(&tree, "/none").is_empty());
        assert!(tree.list(Path::new("/missing")).is_err());
        assert_eq!(
            tree.listed(),
            ["/sorted", "/rotated", "/one", "/none", "/missing"].map(PathBuf::from)
        );
    }

    fn kinds(tree: &FakeTree, dir: &str) -> Vec<(OsString, EntryKind)> {
        let mut listed: Vec<_> = tree
            .list(Path::new(dir))
            .expect("lists")
            .into_iter()
            .map(|listing| (listing.name, listing.kind))
            .collect();
        listed.sort_by(|a, b| a.0.cmp(&b.0));
        listed
    }

    fn entries(entries: &[(&str, EntryKind)]) -> Vec<(OsString, EntryKind)> {
        entries
            .iter()
            .map(|(name, kind)| (OsString::from(name), *kind))
            .collect()
    }

    #[test]
    fn a_file_lists_in_every_ancestor() {
        let tree = FakeTree::new().file("/a/b/c", b"x".to_vec());
        assert_eq!(kinds(&tree, "/a/b"), entries(&[("c", File)]));
        assert_eq!(kinds(&tree, "/a"), entries(&[("b", Dir)]));
        assert_eq!(kinds(&tree, "/"), entries(&[("a", Dir)]));
    }

    #[test]
    fn files_merge_with_an_explicit_listing() {
        let tree = FakeTree::new()
            .dir("/d", &[("link", Symlink), ("sub", Dir)])
            .file("/d/one", Vec::new())
            .file("/d/sub/two", Vec::new())
            .file("/d/sub/three", Vec::new());
        assert_eq!(
            kinds(&tree, "/d"),
            entries(&[("link", Symlink), ("one", File), ("sub", Dir)])
        );
        assert_eq!(
            kinds(&tree, "/d/sub"),
            entries(&[("three", File), ("two", File)])
        );
    }

    #[test]
    fn listing_a_file_fails() {
        let tree = FakeTree::new().file("/d/f", Vec::new());
        assert!(tree.list(Path::new("/d/f")).is_err());
    }

    #[test]
    fn a_folder_never_added_is_not_found() {
        let error = FakeTree::new()
            .file("/d/f", Vec::new())
            .list(Path::new("/e"))
            .expect_err("missing");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn its_files_read_through_the_forks_and_are_recorded() {
        let tree = FakeTree::new()
            .file("/d/f", b"fork".to_vec())
            .unreadable("/d/u")
            .no_data_fork("/d/n");
        assert_eq!(
            kinds(&tree, "/d"),
            entries(&[("f", File), ("n", File), ("u", File)])
        );
        assert_eq!(
            tree.read_fork(Path::new("/d/f"), Fork::Data).ok(),
            Some(Some(b"fork".to_vec()))
        );
        assert!(tree.read_fork(Path::new("/d/u"), Fork::Data).is_err());
        assert_eq!(
            tree.read_fork(Path::new("/d/n"), Fork::Data).ok(),
            Some(None)
        );
        assert_eq!(
            tree.forks().reads(),
            ["/d/f", "/d/u", "/d/n"].map(|path| (PathBuf::from(path), Fork::Data))
        );
    }

    const MISN: ResType = crate::records::mission::Mission::TYPE;

    fn mission_bytes() -> Vec<u8> {
        vec![0; crate::records::mission::Mission::SIZE.expect("fixed")]
    }

    #[test]
    fn store_of_holds_its_resources() {
        let data = store_of(&[(MISN, 128, mission_bytes())]);
        assert_eq!(data.ids(MISN), [128]);
        let mission = data
            .get::<crate::records::mission::Mission>(128)
            .expect("present")
            .expect("decodes");
        assert_eq!(mission.source.path, Path::new(DATA_DIR).join(DATA_FILE));
    }

    fn fork_of(id: i16) -> Vec<u8> {
        ForkBuilder::new()
            .resource(MISN, id, None, &mission_bytes())
            .build()
            .bytes
    }

    #[test]
    fn load_opens_the_plugins_only_when_a_file_is_under_them() {
        let alone = one_file(fork_of(128));
        assert_eq!(alone.load().ids(MISN), [128]);
        assert_eq!(alone.listed(), [PathBuf::from(DATA_DIR)]);
        let with_plugin =
            one_file(fork_of(128)).file(Path::new(PLUGINS_DIR).join("Plug"), fork_of(129));
        assert_eq!(with_plugin.load().ids(MISN), [128, 129]);
        assert!(with_plugin.listed().contains(&PathBuf::from(PLUGINS_DIR)));
    }

    #[test]
    fn load_from_opens_the_directories_it_is_given() {
        let tree = one_file_at("/game/Nova Files/Nova Data", fork_of(128))
            .file("/game/Plug-ins/Plug", fork_of(129));
        let data = tree
            .load_from(
                Path::new("/game/Nova Files"),
                Some(Path::new("/game/Plug-ins")),
            )
            .expect("opens");
        assert_eq!(data.ids(MISN), [128, 129]);
        assert!(tree.load_from(Path::new("/elsewhere"), None).is_err());
    }

    fn root() -> PathBuf {
        Path::new("contract").join("root")
    }

    #[test]
    fn fake_tree_meets_the_contract() {
        assert_eq!(
            check_contract(&FakeTree::contract(&root()), &root()),
            Ok(())
        );
    }

    /// [`FakeTree::contract`] with one answer changed.
    struct Broken<F> {
        inner: FakeTree,
        change: F,
    }

    impl<F> DirLister for Broken<F>
    where
        F: Fn(&Path, io::Result<Vec<Listing>>) -> io::Result<Vec<Listing>>,
    {
        fn list(&self, dir: &Path) -> io::Result<Vec<Listing>> {
            (self.change)(dir, self.inner.list(dir))
        }
    }

    /// The error `check_contract` gives when `change` rewrites the listing
    /// of the directory named `name`.
    fn broken(
        name: &str,
        change: impl Fn(io::Result<Vec<Listing>>) -> io::Result<Vec<Listing>>,
    ) -> String {
        let root = root();
        let target = if name.is_empty() {
            root.clone()
        } else {
            root.join(name)
        };
        let lister = Broken {
            inner: FakeTree::contract(&root),
            change: |dir: &Path, answer| {
                if dir == target {
                    change(answer)
                } else {
                    answer
                }
            },
        };
        check_contract(&lister, &root).expect_err("the broken lister fails the contract")
    }

    fn push(
        name: &str,
        kind: EntryKind,
    ) -> impl Fn(io::Result<Vec<Listing>>) -> io::Result<Vec<Listing>> {
        move |answer| {
            let mut listings = answer?;
            listings.push(Listing {
                name: name.into(),
                kind,
            });
            Ok(listings)
        }
    }

    #[test]
    fn rejects_a_root_listing_with_dot_entries() {
        let error = broken("", push(".", Dir));
        assert!(error.contains("the root"), "{error}");
    }

    #[test]
    fn rejects_a_root_listing_missing_an_entry() {
        let error = broken("", |answer| {
            let mut listings = answer?;
            listings.retain(|listing| listing.name != CONTRACT_EMPTY);
            Ok(listings)
        });
        assert!(error.contains("the root"), "{error}");
    }

    #[test]
    fn rejects_a_root_listing_with_a_wrong_kind() {
        let error = broken("", |answer| {
            Ok(answer?
                .into_iter()
                .map(|listing| Listing {
                    kind: EntryKind::Other,
                    ..listing
                })
                .collect())
        });
        assert!(error.contains("the root"), "{error}");
    }

    #[test]
    fn rejects_a_root_that_fails_to_list() {
        let error = broken("", |_| Err(io::Error::other("disk on fire")));
        assert!(error.contains("the root fails"), "{error}");
    }

    #[test]
    fn rejects_a_wrong_sub_folder_listing() {
        let error = broken(CONTRACT_PLUGINS, push("Extra", File));
        assert!(error.contains("a sub-folder"), "{error}");
    }

    #[test]
    fn rejects_a_sub_folder_that_fails_to_list() {
        let error = broken(CONTRACT_PLUGINS, |_| Err(io::Error::other("disk on fire")));
        assert!(error.contains("a sub-folder fails"), "{error}");
    }

    #[test]
    fn rejects_an_empty_folder_that_lists_something() {
        let error = broken(CONTRACT_EMPTY, push("Ghost", File));
        assert!(error.contains("an empty folder"), "{error}");
    }

    #[test]
    fn rejects_an_empty_folder_that_fails_to_list() {
        let error = broken(CONTRACT_EMPTY, |_| Err(io::Error::other("disk on fire")));
        assert!(error.contains("an empty folder fails"), "{error}");
    }

    #[test]
    fn rejects_a_missing_folder_that_lists() {
        let error = broken(CONTRACT_MISSING, |_| Ok(Vec::new()));
        assert!(error.contains("NotFound"), "{error}");
    }

    #[test]
    fn rejects_a_missing_folder_failing_with_another_kind() {
        let error = broken(CONTRACT_MISSING, |_| Err(io::Error::other("disk on fire")));
        assert!(error.contains("NotFound"), "{error}");
    }

    #[test]
    fn rejects_a_file_that_lists() {
        let error = broken(CONTRACT_FILE, |_| Ok(Vec::new()));
        assert!(error.contains("listing a file"), "{error}");
    }

    #[test]
    fn rejects_a_root_that_changes_between_listings() {
        let root = root();
        let count = Mutex::new(0);
        let lister = Broken {
            inner: FakeTree::contract(&root),
            change: |dir: &Path, answer: io::Result<Vec<Listing>>| {
                if dir != root {
                    return answer;
                }
                let mut count = count.lock().expect("count");
                *count += 1;
                if *count > 1 {
                    push("Late", File)(answer)
                } else {
                    answer
                }
            },
        };
        let error = check_contract(&lister, &root).expect_err("fails");
        assert!(error.contains("second listing"), "{error}");
    }
}
