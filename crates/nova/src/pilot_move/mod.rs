//! Moving the pilots out of the directory earlier versions saved them in
//! (`config::legacy_pilots_dir`, beside the settings under
//! `$XDG_CONFIG_HOME`) into the one they are saved in now
//! (`config::pilots_dir`, under `$XDG_DATA_HOME`), once, on the first
//! launch that finds them there.
//!
//! [`move_pilots`] is the core: it decides what moves, over the
//! [`MoveFiles`] port, and keeps every file. Each entry of the old
//! directory (pilots, the `Last Pilot` record, a stray temporary file:
//! everything) is moved to the same name in the new one, unless that name
//! is taken there, when it is left where it was. An entry that fails to
//! move stays too, and the others are still tried. The old directory is
//! removed only once it is empty, so a later launch moves whatever is
//! still in it, and after a full move finds nothing to do.
//!
//! [`report_lines`] says what happened, for the startup report: one
//! notice when anything moved, and one warning for each thing that went
//! wrong. [`DiskFiles`] is the port over the real filesystem.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

mod disk;

pub use disk::DiskFiles;

/// What [`move_pilots`] needs of the filesystem.
pub trait MoveFiles {
    /// The names of the entries in `dir`, or `None` when there is no
    /// `dir`.
    ///
    /// # Errors
    ///
    /// When `dir` is there but cannot be listed.
    fn entries(&self, dir: &Path) -> io::Result<Option<Vec<OsString>>>;

    /// Whether anything (a file, a directory, a link, even a broken one)
    /// is at `path`.
    ///
    /// # Errors
    ///
    /// When that cannot be told.
    fn exists(&self, path: &Path) -> io::Result<bool>;

    /// Makes the directory `dir`, and its parents, when they are missing.
    ///
    /// # Errors
    ///
    /// When one cannot be made.
    fn create_dir_all(&mut self, dir: &Path) -> io::Result<()>;

    /// Moves the entry at `from` to `to`, where nothing is.
    ///
    /// # Errors
    ///
    /// When it did not move; it is then still at `from`.
    fn move_entry(&mut self, from: &Path, to: &Path) -> io::Result<()>;

    /// Removes the directory `dir`, only if it is empty.
    ///
    /// # Errors
    ///
    /// When it is not empty, or cannot be removed.
    fn remove_empty_dir(&mut self, dir: &Path) -> io::Result<()>;
}

/// What [`move_pilots`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The directory the pilots were moved out of.
    pub old: PathBuf,
    /// The directory they were moved into.
    pub new: PathBuf,
    /// The names of the entries moved, in the order they were.
    pub moved: Vec<OsString>,
    /// What went wrong, in the order it did.
    pub problems: Vec<Problem>,
}

/// Something [`move_pilots`] could not do. The errors are their text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// The old directory could not be listed, so nothing moved.
    Unlisted(String),
    /// The new directory could not be made, so nothing moved.
    Uncreated(String),
    /// Whether the entry's name is taken in the new directory could not
    /// be told, so it was left in the old one.
    Unchecked(OsString, String),
    /// The entry's name is taken in the new directory, so it was left in
    /// the old one.
    Taken(OsString),
    /// The entry did not move.
    Failed(OsString, String),
    /// Everything moved, but the empty old directory could not be
    /// removed.
    Unremoved(String),
}

/// Moves every entry of `old` into `new` through `files`, keeping every
/// file, as the [module docs](self) say.
///
/// A missing `old` is nothing to move. An empty one is removed, and
/// `new` is not made.
pub fn move_pilots(files: &mut impl MoveFiles, old: &Path, new: &Path) -> Outcome {
    let mut outcome = Outcome {
        old: old.to_path_buf(),
        new: new.to_path_buf(),
        moved: Vec::new(),
        problems: Vec::new(),
    };
    let mut names = match files.entries(old) {
        Ok(Some(names)) => names,
        Ok(None) => return outcome,
        Err(error) => {
            outcome.problems.push(Problem::Unlisted(error.to_string()));
            return outcome;
        }
    };
    // Directories list in no particular order; the report's is the names'.
    names.sort();
    if !names.is_empty() {
        if let Err(error) = files.create_dir_all(new) {
            outcome.problems.push(Problem::Uncreated(error.to_string()));
            return outcome;
        }
        for name in names {
            let to = new.join(&name);
            // `rename` replaces what is at `to` on Linux, so a taken name
            // is found first.
            match files.exists(&to) {
                Ok(true) => outcome.problems.push(Problem::Taken(name)),
                Err(error) => outcome
                    .problems
                    .push(Problem::Unchecked(name, error.to_string())),
                Ok(false) => match files.move_entry(&old.join(&name), &to) {
                    Ok(()) => outcome.moved.push(name),
                    Err(error) => outcome
                        .problems
                        .push(Problem::Failed(name, error.to_string())),
                },
            }
        }
    }
    if outcome.problems.is_empty()
        && let Err(error) = files.remove_empty_dir(old)
    {
        outcome.problems.push(Problem::Unremoved(error.to_string()));
    }
    outcome
}

/// The startup report's lines for `outcome`: a notice naming both
/// directories when anything moved, then a warning for each problem,
/// naming the paths it is about.
#[must_use]
pub fn report_lines(outcome: &Outcome) -> Vec<String> {
    let old = outcome.old.display();
    let new = outcome.new.display();
    let from = |name: &OsString| outcome.old.join(name);
    let to = |name: &OsString| outcome.new.join(name);
    let mut lines = Vec::new();
    if !outcome.moved.is_empty() {
        lines.push(format!(
            "nova: moved the pilots in {old} to {new}, where they are saved now"
        ));
    }
    for problem in &outcome.problems {
        lines.push(match problem {
            Problem::Unlisted(error) => {
                format!("nova: could not list {old} for pilots to move: {error}")
            }
            Problem::Uncreated(error) => {
                format!("nova: could not make {new} to move the pilots in {old} to: {error}")
            }
            Problem::Unchecked(name, error) => format!(
                "nova: left {} where it was, as whether {} is taken could not be told: {error}",
                from(name).display(),
                to(name).display()
            ),
            Problem::Taken(name) => format!(
                "nova: left {} where it was, as {} is already taken",
                from(name).display(),
                to(name).display()
            ),
            Problem::Failed(name, error) => format!(
                "nova: could not move {} to {}: {error}",
                from(name).display(),
                to(name).display()
            ),
            Problem::Unremoved(error) => {
                format!("nova: moved everything out of {old} but could not remove it: {error}")
            }
        });
    }
    lines
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::*;

    /// A call the fake was given.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Call {
        Entries(PathBuf),
        Exists(PathBuf),
        CreateDirAll(PathBuf),
        MoveEntry(PathBuf, PathBuf),
        RemoveEmptyDir(PathBuf),
    }

    /// A filesystem in memory: directories holding named files, each
    /// file its contents. A call on a path in `failing` fails (for a
    /// move, on its `from`), and so does every directory removal with
    /// `removal_fails`. Every call is recorded.
    #[derive(Debug, Default)]
    struct Fake {
        dirs: BTreeMap<PathBuf, BTreeMap<OsString, &'static str>>,
        failing: BTreeMap<PathBuf, &'static str>,
        removal_fails: Option<&'static str>,
        calls: RefCell<Vec<Call>>,
    }

    impl Fake {
        fn with_dir(mut self, dir: &Path, files: &[(&str, &'static str)]) -> Self {
            let files = files
                .iter()
                .map(|(name, contents)| (OsString::from(name), *contents))
                .collect();
            self.dirs.insert(dir.to_path_buf(), files);
            self
        }

        fn failing(mut self, path: &Path, why: &'static str) -> Self {
            self.failing.insert(path.to_path_buf(), why);
            self
        }

        fn record(&self, call: Call) {
            self.calls.borrow_mut().push(call);
        }

        fn fails(&self, path: &Path) -> io::Result<()> {
            match self.failing.get(path) {
                Some(why) => Err(io::Error::other(*why)),
                None => Ok(()),
            }
        }

        fn take_calls(&self) -> Vec<Call> {
            std::mem::take(&mut self.calls.borrow_mut())
        }

        /// The names in `dir`, or `None` without it.
        fn names(&self, dir: &Path) -> Option<Vec<&str>> {
            let files = self.dirs.get(dir)?;
            Some(
                files
                    .keys()
                    .map(|name| name.to_str().expect("a test name"))
                    .collect(),
            )
        }

        /// Every file's contents, wherever it is, sorted.
        fn every_file(&self) -> Vec<&'static str> {
            let mut all: Vec<_> = self
                .dirs
                .values()
                .flat_map(|files| files.values().copied())
                .collect();
            all.sort_unstable();
            all
        }

        /// The directory and name of `path`.
        fn split(path: &Path) -> (PathBuf, OsString) {
            let dir = path.parent().expect("a parent").to_path_buf();
            let name = path.file_name().expect("a name").to_os_string();
            (dir, name)
        }
    }

    impl MoveFiles for Fake {
        fn entries(&self, dir: &Path) -> io::Result<Option<Vec<OsString>>> {
            self.record(Call::Entries(dir.to_path_buf()));
            self.fails(dir)?;
            // A real directory lists in no particular order; this one
            // lists its names backwards, so the move has to sort them.
            Ok(self
                .dirs
                .get(dir)
                .map(|files| files.keys().rev().cloned().collect()))
        }

        fn exists(&self, path: &Path) -> io::Result<bool> {
            self.record(Call::Exists(path.to_path_buf()));
            self.fails(path)?;
            let (dir, name) = Self::split(path);
            Ok(self.dirs.contains_key(path)
                || self
                    .dirs
                    .get(&dir)
                    .is_some_and(|files| files.contains_key(&name)))
        }

        fn create_dir_all(&mut self, dir: &Path) -> io::Result<()> {
            self.record(Call::CreateDirAll(dir.to_path_buf()));
            self.fails(dir)?;
            self.dirs.entry(dir.to_path_buf()).or_default();
            Ok(())
        }

        fn move_entry(&mut self, from: &Path, to: &Path) -> io::Result<()> {
            self.record(Call::MoveEntry(from.to_path_buf(), to.to_path_buf()));
            self.fails(from)?;
            let (from_dir, from_name) = Self::split(from);
            let (to_dir, to_name) = Self::split(to);
            if !self.dirs.contains_key(&to_dir) {
                return Err(io::ErrorKind::NotFound.into());
            }
            let contents = self
                .dirs
                .get_mut(&from_dir)
                .and_then(|files| files.remove(&from_name))
                .ok_or(io::ErrorKind::NotFound)?;
            let target = self.dirs.get_mut(&to_dir).expect("checked");
            assert!(
                target.insert(to_name, contents).is_none(),
                "a move over {} overwrote it",
                to.display()
            );
            Ok(())
        }

        fn remove_empty_dir(&mut self, dir: &Path) -> io::Result<()> {
            self.record(Call::RemoveEmptyDir(dir.to_path_buf()));
            self.fails(dir)?;
            if let Some(why) = self.removal_fails {
                return Err(io::Error::other(why));
            }
            match self.dirs.get(dir) {
                None => Err(io::ErrorKind::NotFound.into()),
                Some(files) if !files.is_empty() => Err(io::ErrorKind::DirectoryNotEmpty.into()),
                Some(_) => {
                    self.dirs.remove(dir);
                    Ok(())
                }
            }
        }
    }

    fn old() -> PathBuf {
        Path::new("/home/p")
            .join(".config")
            .join("nova")
            .join("Pilots")
    }

    fn new() -> PathBuf {
        Path::new("/home/p")
            .join(".local")
            .join("share")
            .join("nova")
            .join("Pilots")
    }

    /// Runs the move over `fake`, checking every file is still there,
    /// once, after it.
    fn run(fake: &mut Fake) -> Outcome {
        let before = fake.every_file();
        let outcome = move_pilots(fake, &old(), &new());
        assert_eq!(fake.every_file(), before, "every file is kept, once");
        outcome
    }

    fn notice() -> String {
        format!(
            "nova: moved the pilots in {} to {}, where they are saved now",
            old().display(),
            new().display()
        )
    }

    fn mv(name: &str) -> Call {
        Call::MoveEntry(old().join(name), new().join(name))
    }

    fn exists(name: &str) -> Call {
        Call::Exists(new().join(name))
    }

    #[test]
    fn without_an_old_directory_nothing_happens() {
        let mut fake = Fake::default();
        let outcome = run(&mut fake);
        assert_eq!(fake.take_calls(), [Call::Entries(old())]);
        assert_eq!(report_lines(&outcome), Vec::<String>::new());
        assert_eq!(fake.names(&new()), None);
    }

    #[test]
    fn an_empty_old_directory_is_removed_and_no_new_one_made() {
        let mut fake = Fake::default().with_dir(&old(), &[]);
        let outcome = run(&mut fake);
        assert_eq!(
            fake.take_calls(),
            [Call::Entries(old()), Call::RemoveEmptyDir(old())]
        );
        assert_eq!(fake.names(&old()), None);
        assert_eq!(fake.names(&new()), None);
        assert_eq!(report_lines(&outcome), Vec::<String>::new());
    }

    #[test]
    fn every_entry_moves_and_the_old_directory_goes() {
        let mut fake = Fake::default().with_dir(
            &old(),
            &[
                ("Ada.json", "ada"),
                ("Last Pilot", "Ada"),
                ("Bob.json.tmp", "half a bob"),
            ],
        );
        let outcome = run(&mut fake);
        assert_eq!(
            fake.take_calls(),
            [
                Call::Entries(old()),
                Call::CreateDirAll(new()),
                exists("Ada.json"),
                mv("Ada.json"),
                exists("Bob.json.tmp"),
                mv("Bob.json.tmp"),
                exists("Last Pilot"),
                mv("Last Pilot"),
                Call::RemoveEmptyDir(old()),
            ]
        );
        assert_eq!(fake.names(&old()), None);
        assert_eq!(
            fake.names(&new()),
            Some(vec!["Ada.json", "Bob.json.tmp", "Last Pilot"])
        );
        assert_eq!(report_lines(&outcome), [notice()]);
    }

    #[test]
    fn a_taken_name_leaves_the_old_entry_and_keeps_the_old_directory() {
        let mut fake = Fake::default()
            .with_dir(&old(), &[("Ada.json", "old ada"), ("Bob.json", "bob")])
            .with_dir(&new(), &[("Ada.json", "new ada")]);
        let outcome = run(&mut fake);
        assert_eq!(fake.names(&old()), Some(vec!["Ada.json"]));
        assert_eq!(fake.names(&new()), Some(vec!["Ada.json", "Bob.json"]));
        assert_eq!(fake.dirs[&new()][&OsString::from("Ada.json")], "new ada");
        assert!(!fake.take_calls().contains(&Call::RemoveEmptyDir(old())));
        let taken = format!(
            "nova: left {} where it was, as {} is already taken",
            old().join("Ada.json").display(),
            new().join("Ada.json").display()
        );
        assert_eq!(report_lines(&outcome), [notice(), taken]);
    }

    #[test]
    fn with_every_name_taken_there_is_no_notice() {
        let mut fake = Fake::default()
            .with_dir(&old(), &[("Ada.json", "old ada")])
            .with_dir(&new(), &[("Ada.json", "new ada")]);
        let outcome = run(&mut fake);
        assert_eq!(outcome.moved, Vec::<OsString>::new());
        assert_eq!(
            outcome.problems,
            [Problem::Taken(OsString::from("Ada.json"))]
        );
        assert_eq!(report_lines(&outcome).len(), 1);
    }

    #[test]
    fn a_move_failing_partway_keeps_the_rest_going_and_the_old_directory() {
        let mut fake = Fake::default()
            .with_dir(&old(), &[("a.json", "a"), ("b.json", "b"), ("c.json", "c")])
            .failing(&old().join("b.json"), "the disk said no");
        let outcome = run(&mut fake);
        let calls = fake.take_calls();
        assert!(calls.contains(&mv("c.json")), "later entries are tried");
        assert!(!calls.contains(&Call::RemoveEmptyDir(old())));
        assert_eq!(fake.names(&old()), Some(vec!["b.json"]));
        assert_eq!(fake.names(&new()), Some(vec!["a.json", "c.json"]));
        let failed = format!(
            "nova: could not move {} to {}: the disk said no",
            old().join("b.json").display(),
            new().join("b.json").display()
        );
        assert_eq!(report_lines(&outcome), [notice(), failed]);
    }

    #[test]
    fn a_name_that_cannot_be_checked_is_left_where_it_was() {
        let mut fake = Fake::default()
            .with_dir(&old(), &[("a.json", "a"), ("b.json", "b")])
            .failing(&new().join("a.json"), "no peeking");
        let outcome = run(&mut fake);
        assert!(!fake.take_calls().contains(&mv("a.json")));
        assert_eq!(fake.names(&old()), Some(vec!["a.json"]));
        assert_eq!(fake.names(&new()), Some(vec!["b.json"]));
        let unchecked = format!(
            "nova: left {} where it was, as whether {} is taken could not be told: no peeking",
            old().join("a.json").display(),
            new().join("a.json").display()
        );
        assert_eq!(report_lines(&outcome), [notice(), unchecked]);
    }

    #[test]
    fn when_the_new_directory_cannot_be_made_nothing_moves() {
        let mut fake = Fake::default()
            .with_dir(&old(), &[("a.json", "a")])
            .failing(&new(), "read-only");
        let outcome = run(&mut fake);
        assert_eq!(
            fake.take_calls(),
            [Call::Entries(old()), Call::CreateDirAll(new())]
        );
        assert_eq!(fake.names(&old()), Some(vec!["a.json"]));
        let uncreated = format!(
            "nova: could not make {} to move the pilots in {} to: read-only",
            new().display(),
            old().display()
        );
        assert_eq!(report_lines(&outcome), [uncreated]);
    }

    #[test]
    fn when_the_old_directory_cannot_be_listed_nothing_moves() {
        let mut fake = Fake::default()
            .with_dir(&old(), &[("a.json", "a")])
            .failing(&old(), "locked");
        let outcome = run(&mut fake);
        assert_eq!(fake.take_calls(), [Call::Entries(old())]);
        let unlisted = format!(
            "nova: could not list {} for pilots to move: locked",
            old().display()
        );
        assert_eq!(report_lines(&outcome), [unlisted]);
    }

    #[test]
    fn an_old_directory_that_cannot_be_removed_is_warned_about() {
        let mut fake = Fake::default().with_dir(&old(), &[("a.json", "a")]);
        fake.removal_fails = Some("busy");
        let outcome = run(&mut fake);
        assert_eq!(fake.names(&new()), Some(vec!["a.json"]));
        let unremoved = format!(
            "nova: moved everything out of {} but could not remove it: busy",
            old().display()
        );
        assert_eq!(report_lines(&outcome), [notice(), unremoved]);
    }

    #[test]
    fn a_second_run_after_a_partial_one_moves_only_what_is_left() {
        let mut fake = Fake::default()
            .with_dir(&old(), &[("a.json", "a"), ("b.json", "b")])
            .failing(&old().join("b.json"), "not now");
        run(&mut fake);
        fake.failing.clear();
        fake.take_calls();
        let outcome = run(&mut fake);
        assert_eq!(
            fake.take_calls(),
            [
                Call::Entries(old()),
                Call::CreateDirAll(new()),
                exists("b.json"),
                mv("b.json"),
                Call::RemoveEmptyDir(old()),
            ]
        );
        assert_eq!(outcome.moved, [OsString::from("b.json")]);
        assert_eq!(fake.names(&old()), None);
        assert_eq!(fake.names(&new()), Some(vec!["a.json", "b.json"]));
    }

    #[test]
    fn a_second_run_after_a_full_one_does_nothing() {
        let mut fake = Fake::default().with_dir(&old(), &[("a.json", "a")]);
        run(&mut fake);
        fake.take_calls();
        let outcome = run(&mut fake);
        assert_eq!(fake.take_calls(), [Call::Entries(old())]);
        assert_eq!(report_lines(&outcome), Vec::<String>::new());
    }
}
