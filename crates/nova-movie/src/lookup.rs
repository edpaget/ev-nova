//! Finding a movie by its bare file name, as `_PlayQTMovie` (@0x55957 in
//! the `EV Nova` executable) does: directly inside the Plug-Ins folder,
//! then the Nova Files folder (@0x559d2, @0x559ff), and `_StartLoopingMovie`
//! the same (@0x602f5, @0x60322). The folders come from the caller, in
//! order.
//!
//! The rules are here, over the [`Folders`] port, so they are tested on a
//! hand-written fake on every OS; [`StdFolders`] reads the real
//! filesystem.
//!
//! The original looks names up on HFS+, which ignores case, so a name
//! differing in case from the file still finds it here on every OS. HFS+
//! also matches names whatever their Unicode normalisation (it stores them
//! decomposed); this lookup compares them as they are spelled.

use std::fs;
use std::path::{Path, PathBuf};

/// What the lookup reads of a folder.
pub trait Folders {
    /// The names of the files directly inside `dir` (following links),
    /// or `None` when it is missing or can't be listed. Names that aren't
    /// Unicode are left out: no movie name can match them.
    fn files(&self, dir: &Path) -> Option<Vec<String>>;
}

/// The real filesystem.
#[derive(Clone, Copy, Debug, Default)]
pub struct StdFolders;

impl Folders for StdFolders {
    fn files(&self, dir: &Path) -> Option<Vec<String>> {
        let entries = fs::read_dir(dir).ok()?;
        Some(
            entries
                .filter_map(Result::ok)
                .filter(|entry| fs::metadata(entry.path()).is_ok_and(|meta| meta.is_file()))
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect(),
        )
    }
}

/// The file `name` names in the first of `dirs` that holds it: a file
/// whose name is `name` exactly, else one whose name equals it ignoring
/// case. A folder that is missing or can't be listed is skipped. A name
/// that is empty, `.` or `..`, or holds a `/` or `\`, isn't a bare file
/// name, and finds nothing.
pub fn find(folders: &impl Folders, dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    if !is_bare(name) {
        return None;
    }
    let folded = name.to_lowercase();
    dirs.iter().find_map(|dir| {
        let files = folders.files(dir)?;
        let found = files
            .iter()
            .find(|file| *file == name)
            .or_else(|| files.iter().find(|file| file.to_lowercase() == folded))?;
        Some(dir.join(found))
    })
}

/// Whether `name` is a bare file name.
fn is_bare(name: &str) -> bool {
    !(name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Folders holding the files named, and no others.
    #[derive(Default)]
    struct FakeFolders(HashMap<PathBuf, Vec<String>>);

    impl FakeFolders {
        fn with(mut self, dir: &str, files: &[&str]) -> Self {
            self.0.insert(
                PathBuf::from(dir),
                files.iter().map(|f| (*f).to_owned()).collect(),
            );
            self
        }
    }

    impl Folders for FakeFolders {
        fn files(&self, dir: &Path) -> Option<Vec<String>> {
            self.0.get(dir).cloned()
        }
    }

    fn dirs() -> Vec<PathBuf> {
        vec![PathBuf::from("plug-ins"), PathBuf::from("nova-files")]
    }

    #[test]
    fn a_movie_in_plug_ins_wins_over_nova_files() {
        let folders = FakeFolders::default()
            .with("plug-ins", &["Race 1.mov"])
            .with("nova-files", &["Race 1.mov"]);
        assert_eq!(
            find(&folders, &dirs(), "Race 1.mov"),
            Some(Path::new("plug-ins").join("Race 1.mov"))
        );
    }

    #[test]
    fn a_movie_only_in_nova_files_is_found() {
        let folders = FakeFolders::default()
            .with("plug-ins", &["Other.mov"])
            .with("nova-files", &["Race 1.mov"]);
        assert_eq!(
            find(&folders, &dirs(), "Race 1.mov"),
            Some(Path::new("nova-files").join("Race 1.mov"))
        );
    }

    #[test]
    fn a_name_in_neither_folder_is_not_found() {
        let folders = FakeFolders::default()
            .with("plug-ins", &["Other.mov"])
            .with("nova-files", &["Race 1.mov"]);
        assert_eq!(find(&folders, &dirs(), "Race 2.mov"), None);
    }

    #[test]
    fn a_missing_folder_is_skipped() {
        let folders = FakeFolders::default().with("nova-files", &["Race 1.mov"]);
        assert_eq!(
            find(&folders, &dirs(), "Race 1.mov"),
            Some(Path::new("nova-files").join("Race 1.mov"))
        );
    }

    #[test]
    fn a_name_differing_in_case_is_found() {
        let folders = FakeFolders::default().with("nova-files", &["Race 1.mov"]);
        assert_eq!(
            find(&folders, &dirs(), "RACE 1.MOV"),
            Some(Path::new("nova-files").join("Race 1.mov"))
        );
    }

    #[test]
    fn an_exact_name_wins_over_one_differing_in_case() {
        let folders = FakeFolders::default().with("nova-files", &["race 1.mov", "Race 1.mov"]);
        assert_eq!(
            find(&folders, &dirs(), "Race 1.mov"),
            Some(Path::new("nova-files").join("Race 1.mov"))
        );
    }

    #[test]
    fn an_earlier_folder_wins_even_by_case_alone() {
        let folders = FakeFolders::default()
            .with("plug-ins", &["race 1.mov"])
            .with("nova-files", &["Race 1.mov"]);
        assert_eq!(
            find(&folders, &dirs(), "Race 1.mov"),
            Some(Path::new("plug-ins").join("race 1.mov"))
        );
    }

    #[test]
    fn names_with_separators_or_dots_are_refused() {
        let folders = FakeFolders::default().with("nova-files", &["", ".", "..", "a"]);
        for name in ["", ".", "..", "../a", "sub/a", "sub\\a", "/a"] {
            assert_eq!(find(&folders, &dirs(), name), None, "{name:?}");
        }
    }

    #[test]
    fn the_std_adapter_lists_the_files_of_a_real_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(dir.path().join("Race 1.mov"), b"movie").expect("write");
        fs::create_dir(dir.path().join("Race 2.mov")).expect("a folder");
        let mut files = StdFolders.files(dir.path()).expect("listed");
        files.sort();
        assert_eq!(files, ["Race 1.mov"]);
    }

    #[test]
    fn the_std_adapter_lists_nothing_for_a_missing_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert_eq!(StdFolders.files(&dir.path().join("missing")), None);
    }

    #[test]
    fn the_std_adapter_finds_a_movie_in_a_real_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(dir.path().join("Race 1.mov"), b"movie").expect("write");
        let dirs = [dir.path().join("missing"), dir.path().to_path_buf()];
        assert_eq!(
            find(&StdFolders, &dirs, "Race 1.mov"),
            Some(dir.path().join("Race 1.mov"))
        );
    }
}
