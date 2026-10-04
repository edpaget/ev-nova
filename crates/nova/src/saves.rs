//! The pilot files: [`FilePilots`], the [`PilotStore`] adapter over a
//! directory on disk.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nova_sim::PilotStore;

/// The extension a pilot file has.
const EXTENSION: &str = "json";

/// Pilots saved as files in a directory the caller chooses: each under its
/// key, as `<key>.json`.
///
/// A missing directory holds no pilots, and only `.json` files are
/// pilots. A write creates the directory, writes a temporary file beside
/// the pilot's and renames it over it, so a write that fails part way
/// leaves the last save whole. Errors name the path.
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

    /// The directory's path.
    fn location(&self) -> String {
        self.dir.display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::PilotKeeper;

    use super::*;

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
        assert_eq!(names(dir.path()), ["Ada.json"]);
        let reopened = PilotKeeper::new(FilePilots::new(dir.path()));
        assert_eq!(reopened.list(), Ok(vec!["Ada".to_owned()]));
        assert_eq!(reopened.open("Ada"), Ok(pilot));
    }
}
