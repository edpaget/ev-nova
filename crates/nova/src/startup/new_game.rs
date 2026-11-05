//! The new game's start: what opening its `GAME_DATA` folder gave
//! ([`NewGameOpened`], through [`DataSource::open_new_game`]), and the
//! warnings the startup reports for it ([`warnings`]).
//!
//! [`DataSource::open_new_game`]: super::DataSource::open_new_game

use std::path::Path;

use nova_data::GameData;
use nova_data::Record;
use nova_data::new_game::{ListError, NewGame};
use nova_data::records::system::System;

/// A new game's folder that listed, and what loading it found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewGameLoaded<D> {
    /// The data that loaded.
    pub data: D,
    /// Each file that didn't load or map as written, as `file: message`.
    pub problems: Vec<String>,
    /// The export media that are Git LFS pointers, not the files.
    pub lfs_pointers: Vec<String>,
    /// How many systems loaded.
    pub systems: usize,
}

/// What opening the new game's folder gave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewGameOpened<D> {
    /// It listed: what loaded, and what didn't.
    Loaded(NewGameLoaded<D>),
    /// It couldn't be listed (missing, or not a folder): why.
    Unreadable(String),
}

impl From<Result<NewGame, ListError>> for NewGameOpened<GameData> {
    fn from(opened: Result<NewGame, ListError>) -> Self {
        match opened {
            Ok(game) => Self::Loaded(NewGameLoaded {
                problems: game.problems.iter().map(ToString::to_string).collect(),
                lfs_pointers: game.lfs_pointers,
                systems: game.data.ids(System::TYPE).len(),
                data: game.data,
            }),
            Err(error) => Self::Unreadable(error.source.to_string()),
        }
    }
}

/// The warnings a new game started from `dir` reports for `loaded`: one
/// per problem, one for all the LFS pointers, and one when no system
/// loaded. The game starts with what loaded either way.
#[must_use]
pub fn warnings<D>(dir: &Path, loaded: &NewGameLoaded<D>) -> Vec<String> {
    let mut warnings: Vec<String> = loaded
        .problems
        .iter()
        .map(|problem| format!("nova: the new game's data: {problem}"))
        .collect();
    if let Some(first) = loaded.lfs_pointers.first() {
        let (count, are) = match loaded.lfs_pointers.len() {
            1 => (
                "1 media file".to_owned(),
                "is a Git LFS pointer, not the file",
            ),
            n => (
                format!("{n} media files"),
                "are Git LFS pointers, not the files",
            ),
        };
        warnings.push(format!(
            "nova: {count} in {} {are}, starting with {first}; run `git lfs install --local` \
             and `git lfs pull` in the game-bible checkout",
            dir.display()
        ));
    }
    if loaded.systems == 0 {
        warnings.push(format!(
            "nova: the new game's data in {} has no systems; starting with an empty galaxy",
            dir.display()
        ));
    }
    warnings
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use nova_data::new_game::Problem;

    use super::*;

    fn dir() -> PathBuf {
        Path::new("/Games").join("New Game")
    }

    fn loaded() -> NewGameLoaded<()> {
        NewGameLoaded {
            data: (),
            problems: Vec::new(),
            lfs_pointers: Vec::new(),
            systems: 2,
        }
    }

    #[test]
    fn a_clean_load_with_systems_warns_nothing() {
        assert_eq!(warnings(&dir(), &loaded()), Vec::<String>::new());
    }

    #[test]
    fn one_pointer_is_one_file() {
        let one = NewGameLoaded {
            lfs_pointers: vec!["media/export/a.glb".to_owned()],
            ..loaded()
        };
        assert_eq!(
            warnings(&dir(), &one),
            [format!(
                "nova: 1 media file in {} is a Git LFS pointer, not the file, starting with \
                 media/export/a.glb; run `git lfs install --local` and `git lfs pull` in the \
                 game-bible checkout",
                dir().display()
            )]
        );
    }

    #[test]
    fn problems_then_pointers_then_no_systems() {
        let all = NewGameLoaded {
            problems: vec!["a.md: bad".to_owned()],
            lfs_pointers: vec![
                "media/export/a.glb".to_owned(),
                "media/export/b.glb".to_owned(),
            ],
            systems: 0,
            ..loaded()
        };
        let lines = warnings(&dir(), &all);
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert_eq!(lines[0], "nova: the new game's data: a.md: bad");
        assert!(lines[1].starts_with("nova: 2 media files in "), "{lines:?}");
        assert!(lines[2].ends_with("has no systems; starting with an empty galaxy"));
    }

    fn new_game(problems: Vec<Problem>) -> NewGame {
        NewGame {
            data: GameData::default(),
            ids: nova_data::new_game::IdTable::default(),
            problems,
            lfs_pointers: vec!["media/export/a.glb".to_owned()],
        }
    }

    #[test]
    fn a_loaded_game_gives_its_problems_as_lines_its_pointers_and_its_systems() {
        let game = new_game(vec![Problem::new("systems/a.md", "bad")]);
        let NewGameOpened::Loaded(loaded) = NewGameOpened::from(Ok(game)) else {
            panic!("loaded");
        };
        assert_eq!(loaded.problems, ["systems/a.md: bad"]);
        assert_eq!(loaded.lfs_pointers, ["media/export/a.glb"]);
        assert_eq!(loaded.systems, 0);
    }

    #[test]
    fn a_folder_that_did_not_list_gives_why() {
        let error = ListError {
            path: dir(),
            source: std::io::Error::other("gone"),
        };
        let NewGameOpened::Unreadable(why) = NewGameOpened::from(Err(error)) else {
            panic!("unreadable");
        };
        assert_eq!(why, "gone");
    }
}
