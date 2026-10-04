//! The settings file: [`FileSettings`], the [`SettingsStore`] adapter over
//! one file on disk.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::settings::SettingsStore;

/// The settings saved in the file at a path the caller chooses.
///
/// A missing file is nothing saved. A write creates the directories the
/// file is in, writes a temporary file beside it and renames it over the
/// file, so a write that fails part way leaves the last settings whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSettings {
    path: PathBuf,
}

impl FileSettings {
    /// The settings in the file at `path`.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Where the file is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The temporary file a write goes to first, beside the file.
    fn temporary(&self) -> PathBuf {
        let mut name = self.path.file_name().unwrap_or_default().to_owned();
        name.push(".tmp");
        self.path.with_file_name(name)
    }

    /// `error`, naming the file.
    fn naming(&self, error: &io::Error) -> io::Error {
        io::Error::new(error.kind(), format!("{}: {error}", self.path.display()))
    }
}

impl SettingsStore for FileSettings {
    fn read(&mut self) -> io::Result<Option<String>> {
        match fs::read_to_string(&self.path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(self.naming(&error)),
        }
    }

    fn write(&mut self, text: &str) -> io::Result<()> {
        let temporary = self.temporary();
        let written = (|| {
            if let Some(dir) = self.path.parent() {
                fs::create_dir_all(dir)?;
            }
            fs::write(&temporary, text)?;
            fs::rename(&temporary, &self.path)
        })();
        written.map_err(|error| self.naming(&error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::Volume;
    use crate::settings::{AudioSettings, SettingsKeeper};

    #[test]
    fn a_missing_file_is_nothing_saved() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut store = FileSettings::new(dir.path().join("settings.json"));
        assert_eq!(store.read().expect("reads"), None);
    }

    #[test]
    fn a_write_creates_the_missing_directories_and_leaves_no_temporary_file() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("config").join("nova").join("settings.json");
        let mut store = FileSettings::new(&path);
        store.write("saved").expect("writes");
        assert_eq!(fs::read_to_string(&path).expect("written"), "saved");
        let names: Vec<_> = fs::read_dir(path.parent().expect("a parent"))
            .expect("lists")
            .map(|entry| entry.expect("an entry").file_name())
            .collect();
        assert_eq!(names, ["settings.json"]);
    }

    #[test]
    fn a_write_replaces_the_file_and_a_new_adapter_reads_it_back() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("settings.json");
        FileSettings::new(&path).write("first").expect("writes");
        FileSettings::new(&path).write("second").expect("writes");
        let mut reread = FileSettings::new(&path);
        assert_eq!(reread.read().expect("reads").as_deref(), Some("second"));
        assert_eq!(reread.path(), path);
    }

    #[test]
    fn other_read_errors_pass_through_naming_the_file() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        // A directory where the file should be cannot be read as text.
        let mut store = FileSettings::new(dir.path());
        let error = store.read().expect_err("a directory is not a file");
        assert!(
            error
                .to_string()
                .starts_with(&dir.path().display().to_string()),
            "{error}"
        );
    }

    #[test]
    fn a_write_that_cannot_be_made_is_an_error_naming_the_file() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let blocker = dir.path().join("blocker");
        fs::write(&blocker, "a file, not a directory").expect("writes");
        let path = blocker.join("settings.json");
        let error = FileSettings::new(&path)
            .write("lost")
            .expect_err("its directory is a file");
        assert!(
            error.to_string().starts_with(&path.display().to_string()),
            "{error}"
        );
    }

    #[test]
    fn settings_saved_to_the_file_survive_a_restart() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("nova").join("settings.json");
        let quiet = AudioSettings {
            sound: false,
            music: false,
            effects_volume: Volume::new(0.25),
            music_volume: Volume::new(0.75),
        };
        let (mut keeper, warning) = SettingsKeeper::open(FileSettings::new(&path));
        assert_eq!(
            (keeper.settings(), warning),
            (AudioSettings::default(), None)
        );
        keeper.change(quiet).expect("saves");
        drop(keeper);
        let (restarted, warning) = SettingsKeeper::open(FileSettings::new(&path));
        assert_eq!((restarted.settings(), warning), (quiet, None));
    }
}
