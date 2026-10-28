//! The pilot files: [`FilePilots`], the [`PilotStore`] adapter over a
//! directory on disk.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nova_sim::PilotStore;

/// The extension a pilot file has.
const EXTENSION: &str = "json";

/// The last-pilot record's file name, the original's "Last Pilot" alias
/// (`STR#` 130 #4). With no `.json` extension it is never listed as a
/// pilot, and no pilot's file can share its name.
const RECORD: &str = "Last Pilot";

/// Pilots saved as files in a directory the caller chooses: each under its
/// key, as `<key>.json`.
///
/// A missing directory holds no pilots, and only `.json` files are
/// pilots. A write creates the directory, writes a temporary file beside
/// the pilot's and renames it over it, so a write that fails part way
/// leaves the last save whole. A delete removes the pilot's file, and a
/// missing one is deleted already. Errors name the path.
///
/// The last-pilot record is a small text file in the same directory,
/// `Last Pilot`, holding the key it names. It is written as a pilot is
/// (directory, temporary file, rename), a missing one is no record, and
/// deleting a pilot leaves it alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePilots {
    dir: PathBuf,
}

impl FilePilots {
    /// Pilots in the directory `dir`.
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Where the pilots are.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The file pilot `key` is saved in.
    fn file(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.{EXTENSION}"))
    }
}

/// `error`, naming `path`.
fn naming(path: &Path, error: &io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}

impl PilotStore for FilePilots {
    fn list(&self) -> io::Result<Vec<String>> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(naming(&self.dir, &error)),
        };
        let mut keys = Vec::new();
        for entry in entries {
            let path = entry.map_err(|error| naming(&self.dir, &error))?.path();
            if path.extension().is_some_and(|ext| ext == EXTENSION)
                && path.is_file()
                && let Some(stem) = path.file_stem()
            {
                keys.push(stem.to_string_lossy().into_owned());
            }
        }
        Ok(keys)
    }

    fn read(&self, key: &str) -> io::Result<Option<String>> {
        let file = self.file(key);
        match fs::read_to_string(&file) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(naming(&file, &error)),
        }
    }

    fn write(&self, key: &str, text: &str) -> io::Result<()> {
        let file = self.file(key);
        let temporary = self.dir.join(format!("{key}.{EXTENSION}.tmp"));
        let written = (|| {
            fs::create_dir_all(&self.dir)?;
            fs::write(&temporary, text)?;
            fs::rename(&temporary, &file)
        })();
        written.map_err(|error| naming(&file, &error))
    }

    /// Removes the pilot's file, and nothing else: not the directory, nor
    /// any other file in it.
    fn delete(&self, key: &str) -> io::Result<()> {
        let file = self.file(key);
        match fs::remove_file(&file) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(naming(&file, &error)),
            _ => Ok(()),
        }
    }

    fn last_pilot(&self) -> io::Result<Option<String>> {
        let file = self.dir.join(RECORD);
        match fs::read_to_string(&file) {
            Ok(key) => Ok(Some(key)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(naming(&file, &error)),
        }
    }

    fn set_last_pilot(&self, key: &str) -> io::Result<()> {
        let file = self.dir.join(RECORD);
        let temporary = self.dir.join(format!("{RECORD}.tmp"));
        let written = (|| {
            fs::create_dir_all(&self.dir)?;
            fs::write(&temporary, key)?;
            fs::rename(&temporary, &file)
        })();
        written.map_err(|error| naming(&file, &error))
    }

    /// The directory's path.
    fn location(&self) -> String {
        self.dir.display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::PilotKeeper;
    use nova_sim::fixture::pilot_store::check_contract;

    use super::*;

    #[test]
    fn file_pilots_meet_the_pilot_store_contract() {
        // The directory doesn't exist yet: a missing directory holds no
        // pilots, and the first write makes it.
        let dir = tempfile::tempdir().expect("a temporary directory");
        assert_eq!(
            check_contract(&FilePilots::new(dir.path().join("Pilots"))),
            Ok(())
        );
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .expect("lists")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_missing_directory_holds_no_pilots() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let store = FilePilots::new(dir.path().join("nova").join("Pilots"));
        assert_eq!(store.list().expect("lists"), Vec::<String>::new());
        assert_eq!(store.read("Ada").expect("reads"), None);
    }

    #[test]
    fn a_write_creates_the_directory_and_a_json_file_and_lists_and_reads_back() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let pilots = dir.path().join("nova").join("Pilots");
        let store = FilePilots::new(&pilots);
        store.write("Ada", "saved").expect("writes");
        assert_eq!(names(&pilots), ["Ada.json"]);
        assert_eq!(
            fs::read_to_string(pilots.join("Ada.json")).expect("written"),
            "saved"
        );
        let reread = FilePilots::new(&pilots);
        assert_eq!(reread.list().expect("lists"), ["Ada"]);
        assert_eq!(reread.read("Ada").expect("reads").as_deref(), Some("saved"));
        assert_eq!(reread.read("Bob").expect("reads"), None);
        assert_eq!(reread.dir(), pilots);
    }

    #[test]
    fn a_write_replaces_the_file_leaving_no_temporary_file() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let store = FilePilots::new(dir.path());
        store.write("Ada", "first").expect("writes");
        store.write("Ada", "second").expect("writes");
        store.write("Bob", "other").expect("writes");
        assert_eq!(names(dir.path()), ["Ada.json", "Bob.json"]);
        assert_eq!(store.read("Ada").expect("reads").as_deref(), Some("second"));
    }

    #[test]
    fn only_json_files_are_pilots() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for name in ["Ada.json", "Bob.json.tmp", "notes.txt", "Cy.JSON", ".json"] {
            fs::write(dir.path().join(name), "x").expect("writes");
        }
        fs::create_dir(dir.path().join("Dee.json")).expect("a directory");
        let mut listed = FilePilots::new(dir.path()).list().expect("lists");
        listed.sort();
        assert_eq!(listed, ["Ada"]);
    }

    #[test]
    fn a_delete_removes_the_pilots_file() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let store = FilePilots::new(dir.path());
        store.write("Ada", "saved").expect("writes");
        store.write("Bob", "other").expect("writes");
        fs::write(dir.path().join("notes.txt"), "x").expect("writes");
        store.delete("Ada").expect("deletes");
        assert_eq!(names(dir.path()), ["Bob.json", "notes.txt"]);
        assert_eq!(store.list().expect("lists"), ["Bob"]);
        assert_eq!(store.read("Ada").expect("reads"), None);
    }

    #[test]
    fn deleting_a_missing_pilot_is_ok() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let store = FilePilots::new(dir.path());
        store.write("Bob", "other").expect("writes");
        store
            .delete("Ada")
            .expect("a missing file is deleted already");
        assert_eq!(names(dir.path()), ["Bob.json"]);
        let missing = dir.path().join("nova").join("Pilots");
        FilePilots::new(&missing)
            .delete("Ada")
            .expect("a missing directory holds nothing to delete");
        assert!(!missing.exists(), "a delete creates no directory");
    }

    #[test]
    fn a_failed_delete_names_the_path() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        // A directory where a pilot's file should be cannot be removed as
        // a file.
        let path = dir.path().join("Ada.json");
        fs::create_dir(&path).expect("a directory");
        let deleted = FilePilots::new(dir.path())
            .delete("Ada")
            .expect_err("a directory");
        assert!(
            deleted.to_string().starts_with(&path.display().to_string()),
            "{deleted}"
        );
        assert!(path.is_dir(), "it is left alone");
    }

    #[test]
    fn the_location_is_the_directory() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        assert_eq!(
            FilePilots::new(dir.path()).location(),
            dir.path().display().to_string()
        );
    }

    #[test]
    fn errors_name_the_path() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        // A file where the directory should be can be neither listed nor
        // written into.
        let blocked = dir.path().join("Pilots");
        fs::write(&blocked, "a file").expect("writes");
        let store = FilePilots::new(&blocked);
        let listed = store.list().expect_err("not a directory");
        assert!(
            listed
                .to_string()
                .starts_with(&blocked.display().to_string()),
            "{listed}"
        );
        let written = store.write("Ada", "saved").expect_err("not a directory");
        assert!(
            written.to_string().contains(&blocked.display().to_string()),
            "{written}"
        );
        // A directory where a pilot's file should be cannot be read.
        fs::create_dir(dir.path().join("Ada.json")).expect("a directory");
        let read = FilePilots::new(dir.path())
            .read("Ada")
            .expect_err("a directory");
        let path = dir.path().join("Ada.json");
        assert!(
            read.to_string().starts_with(&path.display().to_string()),
            "{read}"
        );
    }

    /// A version 1 save of a pilot named Ada.
    const ADA: &str = r#"{
        "version": 1, "name": "Ada", "ship": 128, "system": 130,
        "stellar": 128, "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 1000, "course": [],
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        }
    }"#;

    #[test]
    fn a_keeper_saves_lists_and_opens_pilots_in_the_directory() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let pilot = nova_sim::save::decode(ADA).expect("a pilot");
        let keeper = PilotKeeper::new(FilePilots::new(dir.path()));
        assert_eq!(keeper.save(&pilot), Ok(()));
        assert_eq!(names(dir.path()), ["Ada.json", RECORD]);
        let reopened = PilotKeeper::new(FilePilots::new(dir.path()));
        assert_eq!(reopened.list(), Ok(vec!["Ada".to_owned()]));
        assert_eq!(reopened.open("Ada"), Ok(pilot));
    }

    #[test]
    fn with_no_record_there_is_no_last_pilot() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        assert_eq!(
            FilePilots::new(dir.path()).last_pilot().expect("reads"),
            None
        );
        let missing = dir.path().join("nova").join("Pilots");
        assert_eq!(FilePilots::new(&missing).last_pilot().expect("reads"), None);
        assert!(!missing.exists(), "a read creates no directory");
    }

    #[test]
    fn a_written_record_reads_back_leaving_no_temporary_file() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let pilots = dir.path().join("nova").join("Pilots");
        let store = FilePilots::new(&pilots);
        store.set_last_pilot("Ada").expect("writes");
        store.set_last_pilot("Bea Two").expect("writes");
        assert_eq!(names(&pilots), [RECORD]);
        let reread = FilePilots::new(&pilots);
        assert_eq!(
            reread.last_pilot().expect("reads").as_deref(),
            Some("Bea Two")
        );
    }

    #[test]
    fn the_record_is_never_a_pilot() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let store = FilePilots::new(dir.path());
        store.write("Ada", "saved").expect("writes");
        store.set_last_pilot("Ada").expect("writes");
        assert_eq!(store.list().expect("lists"), ["Ada"]);
    }

    #[test]
    fn deleting_a_pilot_leaves_the_record() {
        // `_DeletePilotFile` (@0x72e6b-0x72ef6) never touches "Last Pilot".
        let dir = tempfile::tempdir().expect("a temporary directory");
        let store = FilePilots::new(dir.path());
        store.write("Ada", "saved").expect("writes");
        store.set_last_pilot("Ada").expect("writes");
        store.delete("Ada").expect("deletes");
        assert_eq!(names(dir.path()), [RECORD]);
        assert_eq!(store.last_pilot().expect("reads").as_deref(), Some("Ada"));
    }

    #[test]
    fn a_failed_record_write_or_read_names_the_path() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        // A directory where the record should be can be neither written
        // over nor read.
        let path = dir.path().join(RECORD);
        fs::create_dir(&path).expect("a directory");
        let store = FilePilots::new(dir.path());
        let written = store.set_last_pilot("Ada").expect_err("a directory");
        assert!(
            written.to_string().starts_with(&path.display().to_string()),
            "{written}"
        );
        let read = store.last_pilot().expect_err("a directory");
        assert!(
            read.to_string().starts_with(&path.display().to_string()),
            "{read}"
        );
    }

    #[test]
    fn a_keeper_records_the_last_pilot_on_save_and_open_in_the_directory() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let ada = nova_sim::save::decode(ADA).expect("a pilot");
        let bea = nova_sim::save::decode(&ADA.replace("\"Ada\"", "\"Bea\"")).expect("a pilot");
        let keeper = PilotKeeper::new(FilePilots::new(dir.path()));
        keeper.save(&ada).expect("saves");
        assert_eq!(
            FilePilots::new(dir.path())
                .last_pilot()
                .expect("reads")
                .as_deref(),
            Some("Ada"),
            "a save records"
        );
        keeper.save(&bea).expect("saves");
        keeper.open("Ada").expect("opens");
        assert_eq!(
            FilePilots::new(dir.path())
                .last_pilot()
                .expect("reads")
                .as_deref(),
            Some("Ada"),
            "an open records"
        );
        let relaunched = PilotKeeper::new(FilePilots::new(dir.path()));
        assert_eq!(relaunched.open_last(), Some(ada));
    }
}
